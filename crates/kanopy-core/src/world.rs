//! The forest slice: its size, layers, trees and shrubs.
//!
//! A 200 m wide side-on slice from −3 m (soil) to 50 m. Trees are generated from the seed:
//! Density sets how many there are and how full their crowns are, Tree variation how much they
//! differ. The tallest tree always reaches 50 m. A crown is an ellipse from its bottom to the top
//! of the tree. Shrubs are one band (0.5–3 m) whose fullness follows Density.

use crate::rng::Rng;
use crate::Vec2;

pub const WIDTH: f64 = 200.0;
pub const TOP: f64 = 50.0;
pub const BOTTOM: f64 = -3.0;
/// Top of the shrub band: walkers live from the floor up to here.
pub const SHRUB_TOP: f64 = 3.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layer {
    Soil,
    Floor,
    Shrubs,
    Understorey,
    Subcanopy,
    Canopy,
    Emergent,
}

impl Layer {
    pub const ALL: [Layer; 7] = [
        Layer::Soil,
        Layer::Floor,
        Layer::Shrubs,
        Layer::Understorey,
        Layer::Subcanopy,
        Layer::Canopy,
        Layer::Emergent,
    ];

    /// Bottom and top height, metres.
    pub fn range(self) -> (f64, f64) {
        match self {
            Layer::Soil => (BOTTOM, 0.0),
            Layer::Floor => (0.0, 0.5),
            Layer::Shrubs => (0.5, SHRUB_TOP),
            Layer::Understorey => (SHRUB_TOP, 5.0),
            Layer::Subcanopy => (5.0, 15.0),
            Layer::Canopy => (15.0, 30.0),
            Layer::Emergent => (30.0, TOP),
        }
    }

    pub fn at(y: f64) -> Layer {
        for l in Layer::ALL {
            if y < l.range().1 {
                return l;
            }
        }
        Layer::Emergent
    }

    pub fn name(self) -> &'static str {
        match self {
            Layer::Soil => "Soil",
            Layer::Floor => "Floor",
            Layer::Shrubs => "Shrubs",
            Layer::Understorey => "Understorey",
            Layer::Subcanopy => "Subcanopy",
            Layer::Canopy => "Canopy",
            Layer::Emergent => "Emergent",
        }
    }
}

/// An axis-aligned box in the slice, metres.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x0: f64,
    pub x1: f64,
    pub y0: f64,
    pub y1: f64,
}

impl Rect {
    pub const fn new(x0: f64, x1: f64, y0: f64, y1: f64) -> Self {
        Self { x0, x1, y0, y1 }
    }
    pub const fn world() -> Self {
        Self::new(0.0, WIDTH, BOTTOM, TOP)
    }
    pub fn contains(&self, p: Vec2, eps: f64) -> bool {
        p.x >= self.x0 - eps && p.x <= self.x1 + eps && p.y >= self.y0 - eps && p.y <= self.y1 + eps
    }
    pub fn clamp(&self, p: Vec2) -> Vec2 {
        Vec2::new(p.x.clamp(self.x0, self.x1), p.y.clamp(self.y0, self.y1))
    }
    pub fn intersect(&self, o: &Rect) -> Option<Rect> {
        let r = Rect::new(
            self.x0.max(o.x0),
            self.x1.min(o.x1),
            self.y0.max(o.y0),
            self.y1.min(o.y1),
        );
        (r.x0 <= r.x1 && r.y0 <= r.y1).then_some(r)
    }
    pub fn width(&self) -> f64 {
        self.x1 - self.x0
    }
    pub fn random_point(&self, rng: &mut Rng) -> Vec2 {
        Vec2::new(
            rng.range(self.x0, self.x1.max(self.x0)),
            rng.range(self.y0, self.y1.max(self.y0)),
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tree {
    pub x: f64,
    pub height: f64,
    pub crown_bottom: f64,
    pub crown_half_width: f64,
}

impl Tree {
    pub fn centre(&self) -> Vec2 {
        Vec2::new(self.x, 0.5 * (self.crown_bottom + self.height))
    }
    pub fn radii(&self) -> (f64, f64) {
        (
            self.crown_half_width,
            0.5 * (self.height - self.crown_bottom),
        )
    }
    pub fn in_crown(&self, p: Vec2, eps: f64) -> bool {
        let c = self.centre();
        let (a, b) = self.radii();
        let (a, b) = (a + eps, b + eps);
        ((p.x - c.x) / a).powi(2) + ((p.y - c.y) / b).powi(2) <= 1.0 + 1e-9
    }
    pub fn crown_box(&self) -> Rect {
        Rect::new(
            self.x - self.crown_half_width,
            self.x + self.crown_half_width,
            self.crown_bottom,
            self.height,
        )
    }
    /// Half-width of the crown at height y (0 outside it).
    pub fn crown_width_at(&self, y: f64) -> f64 {
        let c = self.centre();
        let (a, b) = self.radii();
        let u = (y - c.y) / b;
        if u.abs() >= 1.0 {
            0.0
        } else {
            a * (1.0 - u * u).sqrt()
        }
    }
    /// Does the crown reach into this box?
    pub fn crown_meets(&self, r: &Rect) -> bool {
        self.in_crown(r.clamp(self.centre()), 1e-9)
    }
    /// A random point in the crown and inside `r` (which the crown must meet).
    pub fn crown_point(&self, r: &Rect, rng: &mut Rng) -> Vec2 {
        if let Some(b) = self.crown_box().intersect(r) {
            for _ in 0..48 {
                let p = b.random_point(rng);
                if self.in_crown(p, 0.0) {
                    return p;
                }
            }
        }
        r.clamp(self.centre()) // the crown's closest point to its centre inside r
    }
    /// Do the two crowns touch, so a climber can cross without coming down?
    pub fn touches(&self, o: &Tree) -> bool {
        let lo = self.crown_bottom.max(o.crown_bottom);
        let hi = self.height.min(o.height);
        if lo > hi {
            return false;
        }
        let dx = (self.x - o.x).abs();
        (0..=8).any(|k| {
            let y = lo + (hi - lo) * k as f64 / 8.0;
            self.crown_width_at(y) + o.crown_width_at(y) >= dx
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ForestSettings {
    pub seed: u64,
    /// 0 = woodland, 0.5 = forest, 1 = thicket.
    pub density: f64,
    /// 0 = an even stand, 1 = a ragged mix.
    pub tree_variation: f64,
    /// Liquid world: same geometry, trees read as kelp, flyers swim.
    pub liquid: bool,
}

impl Default for ForestSettings {
    fn default() -> Self {
        Self {
            seed: 1,
            density: 0.5,
            tree_variation: 0.5,
            liquid: false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Forest {
    pub settings: ForestSettings,
    pub trees: Vec<Tree>,
    /// Index of the tallest tree (always TOP high).
    pub tallest: usize,
    /// 0…1, how full the shrub band is.
    pub shrub_fullness: f64,
}

/// Trees in the slice for a density: 6 (woodland) to 40 (thicket).
pub fn tree_count(density: f64) -> usize {
    (6.0 + 34.0 * density.clamp(0.0, 1.0)).round() as usize
}

/// Height of an ordinary tree in an even stand.
pub const STAND_HEIGHT: f64 = 26.0;

impl Forest {
    pub fn generate(settings: ForestSettings) -> Self {
        let d = settings.density.clamp(0.0, 1.0);
        let v = settings.tree_variation.clamp(0.0, 1.0);
        let mut rng = Rng::stream(settings.seed, 0x7EE5);
        let n = tree_count(d);
        let gap = WIDTH / n as f64;
        let tallest = rng.int(0, n as i64 - 1) as usize;
        let fullness = 0.7 + 0.6 * d;
        let trees = (0..n)
            .map(|i| {
                let x =
                    ((i as f64 + 0.5) * gap + rng.range(-0.35, 0.35) * gap).clamp(1.0, WIDTH - 1.0);
                let height = if i == tallest {
                    TOP
                } else {
                    (STAND_HEIGHT * (v * 0.55 * rng.gauss()).exp()).clamp(4.0, TOP - 4.0)
                };
                let bottom_frac = (0.5 + v * 0.12 * rng.gauss()).clamp(0.25, 0.75);
                let half =
                    (height * 0.24 * fullness * (1.0 + v * 0.25 * rng.gauss())).clamp(1.5, 12.0);
                Tree {
                    x,
                    height,
                    crown_bottom: height * bottom_frac,
                    crown_half_width: half,
                }
            })
            .collect();
        Self {
            settings,
            trees,
            tallest,
            shrub_fullness: 0.15 + 0.85 * d,
        }
    }

    /// The tree whose trunk is nearest to x, among `among` (or all trees if empty).
    pub fn nearest_tree(&self, x: f64, among: &[usize]) -> Option<usize> {
        let pick = |i: usize| (self.trees[i].x - x).abs();
        if among.is_empty() {
            (0..self.trees.len()).min_by(|&a, &b| pick(a).total_cmp(&pick(b)))
        } else {
            among
                .iter()
                .copied()
                .min_by(|&a, &b| pick(a).total_cmp(&pick(b)))
        }
    }
}
