//! How a call travels from an animal to the listener.
//!
//! Batch 01: spreading (−6 dB per doubling from 1 m), air absorption as a low-pass that closes
//! with distance, travel delay and Doppler (the source's time runs slow or fast as the distance
//! grows or shrinks), and equal-power panning from the horizontal offset relative to distance
//! (straight above is centred). Media other than Earth, foliage, soil and the height brightness
//! cue come in batch 04.

use crate::Vec2;

#[derive(Clone, Copy, Debug)]
pub struct Medium {
    /// Speed of sound, m/s.
    pub speed: f64,
    /// Absorption: dB lost per metre per Hz². A loss A(f) = k·f²·d.
    pub absorb_k: f64,
}

impl Medium {
    /// Earth air, roughly 20 °C and 50 % humidity. About 0.1 dB/m at 8 kHz.
    pub const fn earth() -> Self {
        Self {
            speed: 343.0,
            absorb_k: 1.5e-9,
        }
    }
}

/// Distances closer than this count as this (no infinite level at the source).
pub const MIN_DIST: f64 = 1.0;

/// Gain at the listener (linear, relative to 1 m) from distance.
#[inline]
pub fn spreading(d: f64) -> f32 {
    (MIN_DIST / d.max(MIN_DIST)) as f32
}

/// Absorption low-pass cutoff (Hz) for a distance: where k·f²·d reaches 3 dB.
#[inline]
pub fn absorption_cutoff(m: &Medium, d: f64, sample_rate: f32) -> f32 {
    let f = (3.0 / (m.absorb_k * d.max(MIN_DIST))).sqrt();
    (f as f32).min(0.45 * sample_rate)
}

/// Equal-power pan gains (left, right) for a source relative to the listener.
#[inline]
pub fn pan(listener: Vec2, source: Vec2) -> (f32, f32) {
    let d = listener.dist(source);
    let p = if d < 1e-9 {
        0.0
    } else {
        ((source.x - listener.x) / d) as f32
    };
    let a = (p.clamp(-1.0, 1.0) + 1.0) * std::f32::consts::FRAC_PI_4;
    (a.cos(), a.sin())
}

/// Source time τ heard at listener time t: τ + d(τ)/c = t, for a source moving as
/// `pos(τ) = p0 + v·τ`. Solved by fixed-point iteration (converges while |v| < c).
pub fn emission_time(t: f64, listener: Vec2, p0: Vec2, v: Vec2, m: &Medium) -> f64 {
    let mut tau = t - listener.dist(p0.add_scaled(v, t)) / m.speed;
    for _ in 0..6 {
        tau = t - listener.dist(p0.add_scaled(v, tau)) / m.speed;
    }
    tau
}
