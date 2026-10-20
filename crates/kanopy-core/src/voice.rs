//! One unit: a single sound from the prototype voice.
//!
//! Wavetable (Shape 0…3: sine → triangle → saw → square) through a resonant high-pass body,
//! with a pitch glide, an attack/release envelope and a source level. Everything moves in
//! *source time*: the mixer feeds it at a rate that includes Doppler, so pitch, envelope and
//! length all stretch together, as they would for a moving animal.
use crate::pm::Pm;

use crate::filter::Svf;
use crate::wavetable::{shape_pair, Wavetables, TABLE_BITS, TABLE_LEN};

/// What to play. Pitches are MIDI notes (fractional allowed), times in seconds of source time.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UnitSpec {
    pub note_start: f32,
    pub note_end: f32,
    pub dur: f32,
    pub shape_start: f32,
    pub shape_end: f32,
    pub cutoff_start: f32,
    pub cutoff_end: f32,
    pub resonance: f32,
    pub attack: f32,
    pub release: f32,
    /// dB SPL at 1 m.
    pub level_db: f32,
}

impl Default for UnitSpec {
    fn default() -> Self {
        Self {
            note_start: 69.0,
            note_end: 69.0,
            dur: 0.5,
            shape_start: 0.0,
            shape_end: 0.0,
            cutoff_start: 20.0,
            cutoff_end: 20.0,
            resonance: 0.0,
            attack: 0.01,
            release: 0.05,
            level_db: 80.0,
        }
    }
}

#[inline]
pub fn note_to_hz(n: f32) -> f32 {
    440.0 * ((n - 69.0) / 12.0).p_exp2()
}

#[derive(Clone, Debug)]
pub struct Voice {
    pub spec: UnitSpec,
    /// Source time elapsed, seconds.
    pub t: f64,
    /// Oscillator phase as a 32-bit fraction of a cycle: wraps by itself.
    phase: u32,
    body: Svf,
    inv_attack: f32,
    inv_release: f32,
}

impl Voice {
    pub fn new(spec: UnitSpec, sample_rate: f32) -> Self {
        Self {
            spec,
            t: 0.0,
            phase: 0,
            body: Svf::new(spec.cutoff_start, spec.resonance, sample_rate),
            inv_attack: 1.0 / spec.attack.max(0.001),
            inv_release: 1.0 / spec.release.max(0.001),
        }
    }

    pub fn finished(&self) -> bool {
        self.t >= self.spec.dur as f64
    }

    #[inline]
    fn progress(&self, t: f64) -> f32 {
        (t / self.spec.dur.max(1e-6) as f64).clamp(0.0, 1.0) as f32
    }

    #[inline]
    fn freq_at(&self, t: f64) -> f32 {
        let s = &self.spec;
        let n = s.note_start + (s.note_end - s.note_start) * self.progress(t);
        note_to_hz(n)
    }

    /// Advance source time without making sound (for voices too quiet to hear).
    pub fn skip(&mut self, n: usize, rate: f32, sample_rate: f32) {
        self.t += n as f64 * rate as f64 / sample_rate as f64;
    }

    /// Work out this block: how many samples carry the unit, tables, pitch ramp and envelope
    /// ramps. Also sets the body filter's coefficients. `rate` is how fast source time runs
    /// against the listener's (1 = still; above 1 = approaching, so higher and shorter).
    pub fn plan(&mut self, wt: &Wavetables, rate: f32, n: usize) -> Plan {
        let sr = wt.sample_rate;
        let s = self.spec;
        let step = rate as f64 / sr as f64; // source seconds per output sample
        let t0 = self.t;
        let remain = s.dur as f64 - t0;
        let valid = if remain <= 0.0 {
            0
        } else {
            ((remain / step).ceil() as usize).min(n)
        };
        let t1 = t0 + valid as f64 * step;
        let pm = self.progress(0.5 * (t0 + t1));
        let cutoff = s.cutoff_start * (s.cutoff_end / s.cutoff_start.max(1.0)).p_powf(pm);
        self.body.set(cutoff, s.resonance, sr);
        let shape = s.shape_start + (s.shape_end - s.shape_start) * pm;
        let (ta, tb, sf) = shape_pair(shape);
        let f0 = self.freq_at(t0) * rate;
        let f1 = self.freq_at(t1) * rate;
        const ONE: f64 = 4_294_967_296.0; // 2^32 = one cycle
        let i0 = ((f0 / sr).min(0.49) as f64 * ONE) as i64;
        let i1 = ((f1 / sr).min(0.49) as f64 * ONE) as i64;
        let inc = i0 as u32;
        let dinc = if valid > 0 {
            ((i1 - i0) / valid as i64) as i32
        } else {
            0
        };
        let st = step as f32;
        Plan {
            valid,
            level: wt.level_for(f0.max(f1)),
            ta,
            tb,
            sf,
            inc,
            dinc,
            ea: t0 as f32 * self.inv_attack,
            dea: st * self.inv_attack,
            er: (s.dur - t0 as f32) * self.inv_release,
            der: st * self.inv_release,
            t1,
        }
    }

    /// The oscillator alone for a planned block: fills `out[..plan.valid]`.
    pub fn osc(&mut self, wt: &Wavetables, p: &Plan, out: &mut [f32]) {
        const SHIFT: u32 = 32 - TABLE_BITS;
        const FRAC_MASK: u32 = (1 << SHIFT) - 1;
        const FRAC_SCALE: f32 = 1.0 / (1u32 << SHIFT) as f32;
        let a = &wt.table(p.level, p.ta)[..TABLE_LEN + 1];
        let b = &wt.table(p.level, p.tb)[..TABLE_LEN + 1];
        let out = &mut out[..p.valid];
        let mut inc = p.inc;
        let mut ph = self.phase;
        if p.sf == 0.0 {
            for o in out.iter_mut() {
                let i = (ph >> SHIFT) as usize;
                let fr = (ph & FRAC_MASK) as f32 * FRAC_SCALE;
                *o = a[i] + (a[i + 1] - a[i]) * fr;
                ph = ph.wrapping_add(inc);
                inc = inc.wrapping_add_signed(p.dinc);
            }
        } else {
            for o in out.iter_mut() {
                let i = (ph >> SHIFT) as usize;
                let fr = (ph & FRAC_MASK) as f32 * FRAC_SCALE;
                let xa = a[i] + (a[i + 1] - a[i]) * fr;
                let xb = b[i] + (b[i + 1] - b[i]) * fr;
                *o = xa + (xb - xa) * p.sf;
                ph = ph.wrapping_add(inc);
                inc = inc.wrapping_add_signed(p.dinc);
            }
        }
        self.phase = ph;
    }

    pub fn body_mut(&mut self) -> &mut Svf {
        &mut self.body
    }

    /// Render `out.len()` samples (≤ BLOCK) into `out` (overwritten), one voice on its own.
    /// Returns the number of samples written before the unit ended; the rest is zeroed.
    /// The mixer uses this for blocks where a unit starts or ends, and runs full blocks of
    /// several units side by side instead (same result).
    pub fn render(&mut self, wt: &Wavetables, out: &mut [f32], rate: f32) -> usize {
        let p = self.plan(wt, rate, out.len());
        out[p.valid..].fill(0.0);
        if p.valid == 0 {
            return 0;
        }
        self.osc(wt, &p, out);
        let out = &mut out[..p.valid];
        self.body.highpass_block(out);
        let (mut ea, mut er) = (p.ea, p.er);
        for o in out.iter_mut() {
            *o *= ea.min(1.0).min(er).max(0.0);
            ea += p.dea;
            er -= p.der;
        }
        self.t = p.t1;
        p.valid
    }
}

/// One planned block of one unit (see `Voice::plan`).
#[derive(Clone, Copy, Debug)]
pub struct Plan {
    pub valid: usize,
    level: usize,
    ta: usize,
    tb: usize,
    sf: f32,
    inc: u32,
    dinc: i32,
    /// Envelope: attack ramp, its step, release ramp, its step. Envelope = min(ea, 1, er) ≥ 0.
    pub ea: f32,
    pub dea: f32,
    pub er: f32,
    pub der: f32,
    /// Source time at the end of the block.
    pub t1: f64,
}
