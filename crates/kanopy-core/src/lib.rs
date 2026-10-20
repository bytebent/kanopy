//! kanopy-core: the forest world, behaviour and audio. No UI.
//!
//! Batch 01 holds the sound side only: one voice (wavetable + resonant high-pass body),
//! propagation from a source to the listener, and a mixer that renders voices in blocks.

pub mod filter;
pub mod mixer;
pub mod propagation;
pub mod rng;
pub mod voice;
pub mod wav;
pub mod wavetable;

/// Samples per processing block. Coefficients (cutoff, shape, gains) update once per block
/// and are ramped across it.
pub const BLOCK: usize = 64;

/// Provisional: the source level (dB SPL) that a sine of amplitude 1.0 represents.
/// Batch 04 replaces this with the Ear and the starting gain.
pub const FULL_SCALE_SPL: f32 = 100.0;

/// A point or velocity in the forest slice: x left to right, y height, in metres.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec2 {
    pub x: f64,
    pub y: f64,
}

impl Vec2 {
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
    pub fn add_scaled(self, v: Vec2, s: f64) -> Vec2 {
        Vec2::new(self.x + v.x * s, self.y + v.y * s)
    }
    pub fn dist(self, o: Vec2) -> f64 {
        ((self.x - o.x).powi(2) + (self.y - o.y).powi(2)).sqrt()
    }
}

/// dB to linear amplitude.
#[inline]
pub fn db_to_amp(db: f32) -> f32 {
    10f32.powf(db / 20.0)
}

/// Linear amplitude to dB.
#[inline]
pub fn amp_to_db(a: f32) -> f32 {
    20.0 * a.max(1e-12).log10()
}
