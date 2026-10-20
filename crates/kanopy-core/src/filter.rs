//! Filters.
//!
//! `Svf`: a 2-pole state-variable filter (trapezoidal, after Zavalishin and Simper), used for the
//! body as a resonant high-pass at 12 dB/oct. Stable under fast cutoff changes.
//!
//! `OnePole`: a 6 dB/oct low-pass, used for absorption with distance.

use std::f32::consts::PI;

/// Lowest and highest body cutoff. The top is the filter's own working limit: a body pushed
/// further (e.g. by the medium) just stays there.
pub const CUTOFF_MIN: f32 = 10.0;
pub fn cutoff_max(sample_rate: f32) -> f32 {
    (0.45 * sample_rate).min(20_000.0)
}

/// Resonance 0…1 to Q: 0.707 (no peak) up to about 20.
#[inline]
pub fn resonance_to_q(res: f32) -> f32 {
    0.707 * 28f32.powf(res.clamp(0.0, 1.0))
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Svf {
    pub ic1: f32,
    pub ic2: f32,
    pub a1: f32,
    pub a2: f32,
    pub a3: f32,
    pub k: f32,
}

impl Svf {
    pub fn new(cutoff: f32, res: f32, sample_rate: f32) -> Self {
        let mut f = Self::default();
        f.set(cutoff, res, sample_rate);
        f
    }

    #[inline]
    pub fn set(&mut self, cutoff: f32, res: f32, sample_rate: f32) {
        let fc = cutoff.clamp(CUTOFF_MIN, cutoff_max(sample_rate));
        let g = (PI * fc / sample_rate).tan();
        let k = 1.0 / resonance_to_q(res);
        self.a1 = 1.0 / (1.0 + g * (g + k));
        self.a2 = g * self.a1;
        self.a3 = g * self.a2;
        self.k = k;
    }

    /// One sample through, high-pass output.
    #[inline]
    pub fn highpass(&mut self, x: f32) -> f32 {
        let v3 = x - self.ic2;
        let v1 = self.a1 * self.ic1 + self.a2 * v3;
        let v2 = self.ic2 + self.a2 * self.ic1 + self.a3 * v3;
        self.ic1 = 2.0 * v1 - self.ic1;
        self.ic2 = 2.0 * v2 - self.ic2;
        x - self.k * v1 - v2
    }

    /// A block through the high-pass, in place.
    #[inline]
    pub fn highpass_block(&mut self, buf: &mut [f32]) {
        let (a1, a2, a3, k) = (self.a1, self.a2, self.a3, self.k);
        let (mut ic1, mut ic2) = (self.ic1, self.ic2);
        for x in buf.iter_mut() {
            let v3 = *x - ic2;
            let v1 = a1 * ic1 + a2 * v3;
            let v2 = ic2 + a2 * ic1 + a3 * v3;
            ic1 = 2.0 * v1 - ic1;
            ic2 = 2.0 * v2 - ic2;
            *x = *x - k * v1 - v2;
        }
        self.ic1 = ic1;
        self.ic2 = ic2;
    }

    pub fn reset(&mut self) {
        self.ic1 = 0.0;
        self.ic2 = 0.0;
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct OnePole {
    pub y: f32,
}

impl OnePole {
    /// Coefficient for a -3 dB point at `cutoff` Hz.
    #[inline]
    pub fn coeff(cutoff: f32, sample_rate: f32) -> f32 {
        1.0 - (-2.0 * PI * cutoff / sample_rate).exp()
    }
    #[inline]
    pub fn lowpass(&mut self, x: f32, a: f32) -> f32 {
        self.y += a * (x - self.y);
        self.y
    }
}
