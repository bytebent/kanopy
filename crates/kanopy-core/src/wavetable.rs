//! Band-limited wavetables for the prototype voice.
//!
//! Four shapes (sine, triangle, saw, square), one table per octave so no harmonic passes
//! Nyquist. Shape is continuous 0…3 and crossfades between neighbouring tables. All shapes
//! share sine phase (their fundamentals line up, so a crossfade never cancels) and the same
//! RMS (so Shape changes colour, not loudness).

pub const TABLE_BITS: u32 = 11;
pub const TABLE_LEN: usize = 1 << TABLE_BITS;
const STRIDE: usize = TABLE_LEN + 1; // one guard sample for interpolation

pub const SINE: usize = 0;
pub const TRIANGLE: usize = 1;
pub const SAW: usize = 2;
pub const SQUARE: usize = 3;
pub const SHAPES: usize = 4;

/// Lowest top frequency of a table. Level k serves fundamentals up to LOW_FMAX · 2^k.
const LOW_FMAX: f32 = 20.0;

pub struct Wavetables {
    pub sample_rate: f32,
    levels: usize,
    data: Vec<f32>,
}

/// Fourier coefficient of harmonic h (sine series) for a shape, before normalising.
fn coeff(shape: usize, h: usize) -> f64 {
    let hf = h as f64;
    match shape {
        SINE => (h == 1) as u8 as f64,
        TRIANGLE => {
            if h % 2 == 0 {
                0.0
            } else {
                let sign = if (h / 2) % 2 == 0 { 1.0 } else { -1.0 };
                sign / (hf * hf)
            }
        }
        SAW => {
            let sign = if h % 2 == 1 { 1.0 } else { -1.0 };
            sign / hf
        }
        SQUARE => {
            if h % 2 == 0 {
                0.0
            } else {
                1.0 / hf
            }
        }
        _ => unreachable!(),
    }
}

impl Wavetables {
    pub fn new(sample_rate: f32) -> Self {
        let nyq = sample_rate as f64 / 2.0;
        let mut levels = 0;
        while (LOW_FMAX as f64) * 2f64.powi(levels as i32) < nyq {
            levels += 1;
        }
        levels += 1;
        let sintab: Vec<f64> = (0..TABLE_LEN)
            .map(|i| (std::f64::consts::TAU * i as f64 / TABLE_LEN as f64).sin())
            .collect();
        let mut data = vec![0f32; levels * SHAPES * STRIDE];
        let mut acc = vec![0f64; TABLE_LEN];
        for lvl in 0..levels {
            let fmax = LOW_FMAX as f64 * 2f64.powi(lvl as i32);
            let hmax = ((nyq / fmax).floor() as usize).clamp(1, TABLE_LEN / 2 - 1);
            for shape in 0..SHAPES {
                acc.iter_mut().for_each(|a| *a = 0.0);
                let mut power = 0.0;
                for h in 1..=hmax {
                    let c = coeff(shape, h);
                    if c == 0.0 {
                        continue;
                    }
                    power += c * c / 2.0;
                    for (i, a) in acc.iter_mut().enumerate() {
                        *a += c * sintab[(h * i) & (TABLE_LEN - 1)];
                    }
                }
                // same RMS as a unit sine (1/√2)
                let scale = (0.5 / power).sqrt();
                let base = (lvl * SHAPES + shape) * STRIDE;
                for i in 0..TABLE_LEN {
                    data[base + i] = (acc[i] * scale) as f32;
                }
                data[base + TABLE_LEN] = data[base];
            }
        }
        Self {
            sample_rate,
            levels,
            data,
        }
    }

    pub fn levels(&self) -> usize {
        self.levels
    }

    /// The table level for a fundamental frequency in Hz (the highest it will reach in the block).
    #[inline]
    pub fn level_for(&self, freq: f32) -> usize {
        let mut lvl = 0;
        let mut fmax = LOW_FMAX;
        while fmax < freq && lvl + 1 < self.levels {
            fmax *= 2.0;
            lvl += 1;
        }
        lvl
    }

    /// The table for (level, shape) as a slice of TABLE_LEN + 1 samples.
    #[inline]
    pub fn table(&self, level: usize, shape: usize) -> &[f32] {
        let base = (level * SHAPES + shape) * STRIDE;
        &self.data[base..base + STRIDE]
    }

    /// One sample at a phase in [0, 1) and a continuous shape in [0, 3]. For tests and tools;
    /// the voice reads tables directly.
    pub fn sample(&self, level: usize, shape: f32, phase: f32) -> f32 {
        let (a, b, f) = shape_pair(shape);
        let x = read(self.table(level, a), phase);
        if f == 0.0 {
            x
        } else {
            x + (read(self.table(level, b), phase) - x) * f
        }
    }
}

/// Split a continuous shape into two neighbouring tables and a crossfade amount.
#[inline]
pub fn shape_pair(shape: f32) -> (usize, usize, f32) {
    let s = shape.clamp(0.0, 3.0);
    let a = (s.floor() as usize).min(2);
    (a, a + 1, s - a as f32)
}

/// Linear-interpolated table read at phase [0, 1).
#[inline]
pub fn read(t: &[f32], phase: f32) -> f32 {
    // phase is in [0, 1), so truncation is floor (no libm call)
    let p = phase * TABLE_LEN as f32;
    let ip = p as usize;
    let fr = p - ip as f32;
    let i = ip & (TABLE_LEN - 1);
    let a = t[i];
    a + (t[i + 1] - a) * fr
}
