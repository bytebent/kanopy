//! kanopy-core: the forest world, behaviour and audio. No UI.
//!
//! - `world`: the slice, layers, trees and shrubs. `species` and `life`: habitats, groups and
//!   how individuals move.
//! - `calls`, `scale`, `scene`: calls, keys, and the forest calling, hearing and answering.
//! - `voice`, `wavetable`, `filter`: the prototype voice. `propagation` and `mixer`: how calls
//!   reach the listener. `master`: the Ear, limiters and reverb.
//! - `files`: forest and species files. `render`: a forest file to a WAV.

pub mod calls;
pub mod files;
pub mod filter;
pub mod life;
pub mod master;
pub mod mixer;
pub mod pm;
use pm::Pm;
pub mod propagation;
pub mod render;
pub mod rng;
pub mod scale;
pub mod scene;
pub mod species;
pub mod voice;
pub mod wav;
pub mod wavetable;
pub mod world;

/// kanopy's version, written into renders.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Samples per processing block. Coefficients (cutoff, shape, gains) update once per block
/// and are ramped across it.
pub const BLOCK: usize = 64;

/// The physical scale inside the mixer: a sine of amplitude 1.0 is this many dB SPL. The output
/// chain (`master`) maps it to the file's level.
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
    10f32.p_powf(db / 20.0)
}

/// Linear amplitude to dB.
#[inline]
pub fn amp_to_db(a: f32) -> f32 {
    20.0 * a.max(1e-12).p_log10()
}
