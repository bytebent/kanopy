//! The output chain (design §3.6, §8): per-species buses → master gain (the Ear) → a limiter per
//! species → sum → reverb → final limiter at −1 dBFS.
//!
//! Levels: everything up to here is physical (dB SPL at the listener, with 100 dB SPL as 1.0).
//! The reference maps 80 dB SPL to 0 dBFS (+20 dB), so an ordinary forest around 60 dB SPL sits
//! near −20 dBFS. The Ear works on top of that:
//! - **starting gain**: an estimate of the forest's average level sets the gain so it starts
//!   near −20 dBFS RMS, moving at most ±5 dB from the reference;
//! - **Fixed** keeps that gain; **Adapting** then follows the measured level over about 20 s.
//!   Both stay within ±5 dB of the reference.
//! - **Medium compensation** (0…1) moves the reference by the medium's loudness offset, like a
//!   microphone set for each world: at 1 every world starts at a comparable level; at 0 water is
//!   36 dB louder than Earth and the limiters do the work.
//!
//! Limiters look one block (64 samples) ahead and ramp between gains that are both safe for the
//! block they cover, so nothing gets past them. In normal use they don't act.

use crate::{db_to_amp, BLOCK};

/// 80 dB SPL = 0 dBFS.
pub const REFERENCE_DB: f64 = 20.0;
/// Where the starting gain and Adapting aim, dBFS RMS.
pub const TARGET_DBFS: f64 = -20.0;
/// Most the starting gain, and Adapting, may move, dB.
pub const EAR_RANGE: f64 = 5.0;
/// Adapting's time constant, seconds.
pub const EAR_TAU: f64 = 20.0;
/// Species limiter threshold and the final limiter's, dBFS.
pub const SPECIES_LIMIT_DB: f32 = -3.0;
pub const FINAL_LIMIT_DB: f32 = -1.0;
/// Limiter release, seconds.
const RELEASE: f32 = 0.15;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ear {
    Fixed,
    Adapting,
}

/// A stereo-linked peak limiter with one block of look-ahead.
#[derive(Clone, Debug)]
pub struct Limiter {
    threshold: f32,
    release_step: f32,
    gain: f32,
    delay: [[f32; BLOCK]; 2],
    delay_len: usize,
    delay_peak: f32,
    /// Smallest gain it has applied (1 = never acted).
    pub min_gain: f32,
}

impl Limiter {
    pub fn new(threshold_db: f32, sample_rate: f32) -> Self {
        Self {
            threshold: db_to_amp(threshold_db),
            // gain may rise by this factor per block
            release_step: (BLOCK as f32 / (RELEASE * sample_rate)).exp(),
            gain: 1.0,
            delay: [[0.0; BLOCK]; 2],
            delay_len: 0,
            delay_peak: 0.0,
            min_gain: 1.0,
        }
    }

    /// Process one block in place (`n` ≤ BLOCK samples). Output is the previous block, limited.
    pub fn process(&mut self, l: &mut [f32], r: &mut [f32]) {
        let n = l.len();
        let peak_in = l.iter().chain(r.iter()).fold(0f32, |a, x| a.max(x.abs()));
        // gain safe for the delayed block and the incoming one
        let worst = self.delay_peak.max(peak_in);
        let target = if worst > self.threshold {
            self.threshold / worst
        } else {
            1.0
        };
        let g1 = target.min(self.gain * self.release_step);
        let g0 = self.gain.min(if self.delay_peak > self.threshold {
            self.threshold / self.delay_peak
        } else {
            1.0
        });
        // swap the incoming block with the delayed one, applying a ramp g0 → g1
        let m = self.delay_len.max(n);
        for i in 0..n {
            let f = (i as f32 + 1.0) / m as f32;
            let g = g0 + (g1 - g0) * f;
            let (dl, dr) = if i < self.delay_len {
                (self.delay[0][i], self.delay[1][i])
            } else {
                (0.0, 0.0)
            };
            self.delay[0][i] = l[i];
            self.delay[1][i] = r[i];
            l[i] = dl * g;
            r[i] = dr * g;
        }
        self.delay_len = n;
        self.delay_peak = peak_in;
        self.gain = g1;
        self.min_gain = self.min_gain.min(g0.min(g1));
    }
}

/// A small feedback delay network: 8 lines, Householder feedback, a low-pass in each loop.
#[derive(Clone, Debug)]
pub struct Reverb {
    lines: Vec<Vec<f32>>,
    pos: Vec<usize>,
    gains: [f32; 8],
    damp: [f32; 8],
    lp_a: f32,
    pub amount: f32,
}

/// Delay lengths, ms (mutually prime-ish).
const LINES_MS: [f32; 8] = [29.7, 37.1, 41.1, 43.7, 53.3, 59.9, 67.7, 73.1];

impl Reverb {
    /// `rt60` seconds, `brightness` Hz (loop low-pass), `amount` wet level 0…1.
    pub fn new(rt60: f32, brightness: f32, amount: f32, sample_rate: f32) -> Self {
        let lens: Vec<usize> = LINES_MS
            .iter()
            .map(|ms| (ms * 0.001 * sample_rate) as usize)
            .collect();
        let mut gains = [0f32; 8];
        for (g, &n) in gains.iter_mut().zip(&lens) {
            *g = 10f32.powf(-3.0 * n as f32 / (rt60.max(0.05) * sample_rate));
        }
        Self {
            lines: lens.iter().map(|&n| vec![0.0; n]).collect(),
            pos: vec![0; 8],
            gains,
            damp: [0.0; 8],
            lp_a: 1.0 - (-2.0 * std::f32::consts::PI * brightness / sample_rate).exp(),
            amount,
        }
    }

    /// Add the wet signal to l/r.
    pub fn process(&mut self, l: &mut [f32], r: &mut [f32]) {
        let wet = self.amount * 0.35;
        for i in 0..l.len() {
            let input = 0.5 * (l[i] + r[i]);
            let mut outs = [0f32; 8];
            for k in 0..8 {
                outs[k] = self.lines[k][self.pos[k]];
            }
            let sum: f32 = outs.iter().sum::<f32>() * (2.0 / 8.0);
            let (mut wl, mut wr) = (0.0, 0.0);
            for k in 0..8 {
                // Householder: x - 2/N·Σx, then loss and damping
                let fb = (outs[k] - sum) * self.gains[k];
                self.damp[k] += self.lp_a * (fb - self.damp[k]);
                let len = self.lines[k].len();
                self.lines[k][self.pos[k]] =
                    self.damp[k] + input * if k % 2 == 0 { 0.5 } else { -0.5 };
                self.pos[k] = (self.pos[k] + 1) % len;
                if k % 2 == 0 {
                    wl += outs[k]
                } else {
                    wr += outs[k]
                }
            }
            l[i] += wl * wet;
            r[i] += wr * wet;
        }
    }
}

/// Reverb length and brightness for a forest: denser is longer and darker; the medium changes
/// both (thin air short and dark, dense air and water longer).
pub fn reverb_for(density: f64, medium: &crate::propagation::Medium) -> (f32, f32) {
    let d = density.clamp(0.0, 1.0) as f32;
    let mut rt = 0.6 + 1.6 * d;
    let mut bright = 7000.0 * (1.0 - 0.5 * d);
    match medium.name {
        "Mars" => {
            rt *= 0.5;
            bright = bright.min(900.0);
        }
        "Titan" => rt *= 1.3,
        "Venus" => rt *= 1.4,
        "Water" => {
            rt *= 1.6;
            bright = bright.min(2500.0);
        }
        _ => {
            if medium.liquid {
                rt *= 1.6;
            }
        }
    }
    (rt, bright)
}

#[derive(Clone, Debug)]
pub struct MasterSettings {
    pub ear: Ear,
    /// Starting gain from the estimate, dB (already capped to ±EAR_RANGE).
    pub start_db: f64,
    /// Moves the reference, dB (−medium offset × compensation).
    pub reference_shift: f64,
    pub reverb: Option<(f32, f32, f32)>,
}

/// The chain after the buses.
#[derive(Clone, Debug)]
pub struct Master {
    pub ear: Ear,
    /// Gain the Ear started at, and is now at, dB (on top of REFERENCE_DB).
    pub start_db: f64,
    pub gain_db: f64,
    pub reference_shift: f64,
    power: f64,
    alpha: f64,
    pub limiters: Vec<Limiter>,
    pub last: Limiter,
    pub reverb: Option<Reverb>,
    sample_rate: f32,
    sum: [[f32; BLOCK]; 2],
}

impl Master {
    pub fn new(s: &MasterSettings, buses: usize, sample_rate: f32) -> Self {
        Self {
            ear: s.ear,
            start_db: s.start_db,
            gain_db: s.start_db,
            reference_shift: s.reference_shift,
            power: db_to_amp(TARGET_DBFS as f32).powi(2) as f64,
            alpha: 1.0 - (-(BLOCK as f64) / (EAR_TAU * sample_rate as f64)).exp(),
            limiters: (0..buses)
                .map(|_| Limiter::new(SPECIES_LIMIT_DB, sample_rate))
                .collect(),
            last: Limiter::new(FINAL_LIMIT_DB, sample_rate),
            reverb: s
                .reverb
                .map(|(rt, b, a)| Reverb::new(rt, b, a, sample_rate)),
            sample_rate,
            sum: [[0.0; BLOCK]; 2],
        }
    }

    pub fn sample_rate(&self) -> f32 {
        self.sample_rate
    }

    /// One block: buses in, adds to `left`/`right`.
    pub fn process(
        &mut self,
        buses: &mut [[[f32; BLOCK]; 2]],
        left: &mut [f32],
        right: &mut [f32],
    ) {
        let n = left.len();
        let g = db_to_amp((REFERENCE_DB + self.reference_shift + self.gain_db) as f32);
        let [sl, sr] = &mut self.sum;
        sl[..n].fill(0.0);
        sr[..n].fill(0.0);
        for (b, lim) in buses.iter_mut().zip(self.limiters.iter_mut()) {
            let [bl, br] = b;
            let (bl, br) = (&mut bl[..n], &mut br[..n]);
            if bl.iter().chain(br.iter()).all(|x| *x == 0.0) && lim.delay_peak == 0.0 {
                continue; // a silent bus with nothing in its delay
            }
            for x in bl.iter_mut().chain(br.iter_mut()) {
                *x *= g;
            }
            lim.process(bl, br);
            for i in 0..n {
                sl[i] += bl[i];
                sr[i] += br[i];
            }
        }
        // Adapting: follow the measured level (before reverb), within ±EAR_RANGE of the start
        if self.ear == Ear::Adapting {
            let p = (sl[..n]
                .iter()
                .chain(sr[..n].iter())
                .map(|x| (*x as f64).powi(2))
                .sum::<f64>())
                / (2 * n) as f64;
            self.power += (p - self.power) * self.alpha * n as f64 / BLOCK as f64;
            let measured = 10.0 * self.power.max(1e-20).log10();
            let want = self.gain_db + (TARGET_DBFS - measured);
            let to = want.clamp(-EAR_RANGE, EAR_RANGE);
            self.gain_db += (to - self.gain_db) * self.alpha * n as f64 / BLOCK as f64;
        }
        if let Some(r) = self.reverb.as_mut() {
            r.process(&mut sl[..n], &mut sr[..n]);
        }
        self.last.process(&mut sl[..n], &mut sr[..n]);
        for i in 0..n {
            left[i] += sl[i];
            right[i] += sr[i];
        }
    }
}
