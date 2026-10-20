//! How a call travels from an animal to the listener (design §3.4–3.5, §4.1, §7).
//!
//! - **Medium**: speed of sound, density and an absorption curve. A denser medium takes more
//!   power from the same vibrating body: its level offset is 10·log10 of its impedance (density ×
//!   speed) against Earth air. Bodies (cavities) resonate higher or lower with the speed of sound.
//! - **Spreading** −6 dB per doubling from 1 m; **absorption** k·f²·d dB, applied as a low-pass
//!   that closes with distance; **delay** and **Doppler** from the speed of sound.
//! - **Foliage**: the part of the path inside crowns or the shrub band adds a little more
//!   high-frequency loss. **Soil**: a path crossing the ground surface loses 20 dB, and each metre
//!   of soil 1.5 dB plus a strong high-frequency loss, so from underground the world above is
//!   heavily muffled while nearby burrowers stay clear.
//! - **Panning**: equal-power from the horizontal offset relative to distance (straight above is
//!   centred), narrowed in a liquid with Underwater stereo. **Height**: sources below the listener
//!   sound a little darker than ones above.
use crate::pm::Pm;

use crate::world::{Forest, BOTTOM, SHRUB_TOP, TOP, WIDTH};
use crate::Vec2;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Medium {
    pub name: &'static str,
    /// Speed of sound, m/s.
    pub speed: f64,
    /// Density, kg/m³.
    pub density: f64,
    /// Absorption: dB lost per metre per Hz². A loss A(f) = k·f²·d.
    pub absorb_k: f64,
    /// A liquid: flyers swim, trees are kelp.
    pub liquid: bool,
}

/// Earth air's impedance (density × speed), the reference.
const EARTH_Z: f64 = 1.204 * 343.0;
/// Earth air's speed of sound, the reference for bodies.
pub const EARTH_SPEED: f64 = 343.0;

impl Medium {
    /// Earth air, about 20 °C and 50 % humidity: about 0.1 dB/m at 8 kHz.
    pub const fn earth() -> Self {
        Self {
            name: "Earth",
            speed: 343.0,
            density: 1.204,
            absorb_k: 1.5e-9,
            liquid: false,
        }
    }
    /// Thin, cold CO₂: about −19 dB, highs gone within metres (≈0.2 dB/m at 1 kHz).
    pub const fn mars() -> Self {
        Self {
            name: "Mars",
            speed: 240.0,
            density: 0.020,
            absorb_k: 2.0e-7,
            liquid: false,
        }
    }
    /// Dense, cold nitrogen: about +4 dB, sound carries.
    pub const fn titan() -> Self {
        Self {
            name: "Titan",
            speed: 194.0,
            density: 5.3,
            absorb_k: 0.5e-9,
            liquid: false,
        }
    }
    /// Crushing hot CO₂: about +18 dB.
    pub const fn venus() -> Self {
        Self {
            name: "Venus",
            speed: 410.0,
            density: 65.0,
            absorb_k: 1.0e-9,
            liquid: false,
        }
    }
    /// Water: about +36 dB, almost no absorption.
    pub const fn water() -> Self {
        Self {
            name: "Water",
            speed: 1480.0,
            density: 1000.0,
            absorb_k: 1.0e-11,
            liquid: true,
        }
    }
    pub const PRESETS: [Medium; 5] = [
        Self::earth(),
        Self::mars(),
        Self::titan(),
        Self::venus(),
        Self::water(),
    ];

    pub fn custom(speed: f64, density: f64, absorb_k: f64, liquid: bool) -> Self {
        Self {
            name: "Custom",
            speed,
            density,
            absorb_k,
            liquid,
        }
    }

    pub fn preset(name: &str) -> Option<Medium> {
        Self::PRESETS
            .iter()
            .copied()
            .find(|m| m.name.eq_ignore_ascii_case(name))
    }

    /// Source level against Earth air for the same body, dB.
    pub fn level_offset(&self) -> f64 {
        10.0 * (self.density * self.speed / EARTH_Z).p_log10()
    }

    /// How far bodies move, as a frequency ratio (cutoff × this).
    pub fn body_ratio(&self) -> f64 {
        self.speed / EARTH_SPEED
    }
}

/// Distances closer than this count as this (no infinite level at the source).
pub const MIN_DIST: f64 = 1.0;

/// Foliage: dB per metre per Hz² inside crowns or full shrubs. Subtle: 20 m of crown takes
/// about 3 dB off 4 kHz.
pub const FOLIAGE_K: f64 = 1.0e-8;
/// Crossing the ground surface, dB.
pub const SURFACE_LOSS: f64 = 20.0;
/// Each metre of soil: broadband dB, and dB per Hz².
pub const SOIL_DB: f64 = 1.5;
pub const SOIL_K: f64 = 1.0e-6;

/// Gain at the listener (linear, relative to 1 m) from distance.
#[inline]
pub fn spreading(d: f64) -> f32 {
    (MIN_DIST / d.max(MIN_DIST)) as f32
}

/// Absorption low-pass cutoff (Hz): where k·f² reaches 3 dB, for a total k (dB/Hz²) along the
/// path.
#[inline]
pub fn cutoff_for(k_total: f64, sample_rate: f32) -> f32 {
    if k_total <= 0.0 {
        return 0.45 * sample_rate;
    }
    ((3.0 / k_total).sqrt() as f32).min(0.45 * sample_rate)
}

/// Absorption cutoff for a distance in a medium (no foliage or soil).
#[inline]
pub fn absorption_cutoff(m: &Medium, d: f64, sample_rate: f32) -> f32 {
    cutoff_for(m.absorb_k * d.max(MIN_DIST), sample_rate)
}

/// Height cue: the darkest a source straight below gets (Hz); straight above is open.
pub const BELOW_CUTOFF: f32 = 6_000.0;

/// Low-pass cutoff for the height cue, from elevation (−1 straight below … 1 straight above).
#[inline]
pub fn height_cutoff(elevation: f64, sample_rate: f32) -> f32 {
    let top = 0.45 * sample_rate;
    let e = ((elevation + 1.0) * 0.5).clamp(0.0, 1.0) as f32; // 0 below … 1 above
    (BELOW_CUTOFF.p_ln() + (top.p_ln() - BELOW_CUTOFF.p_ln()) * e).p_exp()
}

/// Equal-power pan gains (left, right) for a source relative to the listener, `width` 0…1.
#[inline]
pub fn pan_width(listener: Vec2, source: Vec2, width: f32) -> (f32, f32) {
    let d = listener.dist(source);
    let p = if d < 1e-9 {
        0.0
    } else {
        ((source.x - listener.x) / d) as f32
    };
    pan_value(p * width)
}

/// Equal-power gains for a position −1 (left) … 1 (right).
#[inline]
pub fn pan_value(p: f32) -> (f32, f32) {
    let a = (p.clamp(-1.0, 1.0) + 1.0) * std::f32::consts::FRAC_PI_4;
    (a.p_cos(), a.p_sin())
}

#[inline]
pub fn pan(listener: Vec2, source: Vec2) -> (f32, f32) {
    pan_width(listener, source, 1.0)
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

/// Length of the segment a→b inside an axis-aligned ellipse (centre c, radii rx, ry).
pub fn length_in_ellipse(a: Vec2, b: Vec2, c: Vec2, rx: f64, ry: f64) -> f64 {
    // in the ellipse's unit-circle space the segment is p(t) = p + t·q, t in [0, 1]
    let (px, py) = ((a.x - c.x) / rx, (a.y - c.y) / ry);
    let (qx, qy) = ((b.x - a.x) / rx, (b.y - a.y) / ry);
    let qa = qx * qx + qy * qy;
    if qa < 1e-18 {
        return 0.0;
    }
    let qb = 2.0 * (px * qx + py * qy);
    let qc = px * px + py * py - 1.0;
    let disc = qb * qb - 4.0 * qa * qc;
    if disc <= 0.0 {
        return 0.0;
    }
    let s = disc.sqrt();
    let t0 = ((-qb - s) / (2.0 * qa)).max(0.0);
    let t1 = ((-qb + s) / (2.0 * qa)).min(1.0);
    (t1 - t0).max(0.0) * a.dist(b)
}

/// Length of the segment a→b between two heights.
pub fn length_in_band(a: Vec2, b: Vec2, lo: f64, hi: f64) -> f64 {
    let dy = b.y - a.y;
    let len = a.dist(b);
    if dy.abs() < 1e-12 {
        return if a.y >= lo && a.y <= hi { len } else { 0.0 };
    }
    let (t0, t1) = ((lo - a.y) / dy, (hi - a.y) / dy);
    let (t0, t1) = (t0.min(t1).max(0.0), t0.max(t1).min(1.0));
    (t1 - t0).max(0.0) * len
}

/// What a path adds on top of spreading and the medium: extra broadband loss (dB) and extra
/// high-frequency loss (dB per Hz²).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PathLoss {
    pub db: f64,
    pub k: f64,
}

/// Foliage and soil along the straight path from a source to the listener.
pub fn path_loss(forest: &Forest, src: Vec2, lis: Vec2) -> PathLoss {
    let mut foliage = length_in_band(src, lis, 0.5, SHRUB_TOP) * forest.shrub_fullness;
    let (x0, x1) = (src.x.min(lis.x), src.x.max(lis.x));
    for t in &forest.trees {
        if t.x + t.crown_half_width < x0 || t.x - t.crown_half_width > x1 {
            continue;
        }
        let (rx, ry) = t.radii();
        foliage += length_in_ellipse(src, lis, t.centre(), rx, ry);
    }
    let soil = length_in_band(src, lis, BOTTOM, 0.0);
    let crosses = (src.y < 0.0) != (lis.y < 0.0);
    PathLoss {
        db: if crosses { SURFACE_LOSS } else { 0.0 } + SOIL_DB * soil,
        k: FOLIAGE_K * foliage + SOIL_K * soil,
    }
}

/// Share of the slice above ground taken by crowns and shrubs (for animals' hearing, which uses
/// an average instead of tracing every path).
pub fn foliage_share(forest: &Forest) -> f64 {
    let crowns: f64 = forest
        .trees
        .iter()
        .map(|t| {
            let (rx, ry) = t.radii();
            std::f64::consts::PI * rx * ry
        })
        .sum();
    let shrubs = WIDTH * (SHRUB_TOP - 0.5) * forest.shrub_fullness;
    ((crowns + shrubs) / (WIDTH * TOP)).min(1.0)
}
