//! Life in the forest: species placed as groups of individuals, and how they move.
//!
//! - Each species lives inside its area. Its groups are laid out there by Spread and Spacing;
//!   each group has an anchor (and, for climbers, a home tree).
//! - Members stay near their anchor: within 25 m at Cohesion 0, within 1 m at Cohesion 1.
//! - Walkers and burrowers wander between spots and pause. Climbers do the same inside their
//!   tree's crown. Flyers alternate flying (in their flight band) and resting at their perch, so
//!   that on average they fly for their Flight share of the time; a flocking group flies together.
//! - Groups migrate at random times (per hour) up to their migration distance. Climbers cross to
//!   a touching crown directly, or climb down, walk and climb up.
//! - The active zone marks which individuals may call (design §10): those near the listener, or
//!   inside the box plus a margin. Everyone keeps living either way.
//!
//! Time moves in fixed steps of DT, so the same seed gives the same forest, bit for bit.
use crate::pm::Pm;

use crate::rng::Rng;
use crate::species::{Movement, Perch, Species, Spread};
use crate::world::{Forest, Layer, Rect, BOTTOM, SHRUB_TOP, TOP};
use crate::Vec2;

/// Simulation step, seconds.
pub const DT: f64 = 0.05;
/// Mean time a flyer rests at its perch, seconds.
pub const PERCH_MEAN: f64 = 40.0;
/// Pause between moves for walkers, burrowers and climbers, seconds.
const PAUSE: (f64, f64) = (1.0, 15.0);

fn exp(rng: &mut Rng, mean: f64) -> f64 {
    -mean * (1.0 - rng.f64()).p_ln()
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Mode {
    /// Standing still until then.
    Idle {
        until: f64,
    },
    /// Going to `target`.
    Moving,
    /// Following a path (a climber changing trees). Not bound to a crown meanwhile.
    Travel,
    /// In the air until then, then it goes to perch.
    Flying {
        until: f64,
    },
    /// Taking off: on the way from its perch up into its flight band, then flies until then.
    Rising {
        until: f64,
    },
    /// On the way to its perch at `target`.
    ToPerch,
    Perched {
        until: f64,
    },
}

#[derive(Clone, Debug)]
pub struct Individual {
    pub species: u32,
    pub group: u32,
    pub pos: Vec2,
    pub vel: Vec2,
    pub mode: Mode,
    pub target: Vec2,
    pub active: bool,
    path: [Vec2; 3],
    path_len: u8,
    path_i: u8,
    /// Its own share of the top speed, 0.6…1.
    pace: f64,
    rng: Rng,
}

impl Individual {
    pub fn flying(&self) -> bool {
        matches!(
            self.mode,
            Mode::Flying { .. } | Mode::Rising { .. } | Mode::ToPerch
        )
    }
}

#[derive(Clone, Debug)]
pub struct Group {
    pub species: u32,
    pub anchor: Vec2,
    /// Climbers: the tree they live in.
    pub tree: Option<usize>,
    pub first: usize,
    pub len: usize,
    pub migrations: u32,
    next_migration: f64,
    rng: Rng,
}

/// Where a species can be, worked out once from its habitat and the forest.
#[derive(Clone, Debug)]
pub struct Places {
    /// Its area, clamped to what its movement allows (walkers 0–3 m, burrowers soil).
    pub live: Rect,
    /// Flyers: flight band inside the area.
    pub fly: Rect,
    /// Flyers: perch heights inside the area.
    pub perch: Option<Rect>,
    /// Climbers: trees whose crowns reach into the area. Flyers: trees reaching the perch heights.
    pub homes: Vec<usize>,
}

/// The area cut to heights lo…hi. If they don't overlap, the nearest height.
fn band(area: &Rect, lo: f64, hi: f64) -> Rect {
    let (y0, y1) = (area.y0.max(lo), area.y1.min(hi));
    if y0 <= y1 {
        Rect::new(area.x0, area.x1, y0, y1)
    } else {
        let y = if area.y1 < lo { lo } else { hi };
        Rect::new(area.x0, area.x1, y, y)
    }
}

fn places(forest: &Forest, sp: &Species) -> Places {
    let h = &sp.habitat;
    let area = h.area.intersect(&Rect::world()).unwrap_or(Rect::world());
    let live = match h.movement {
        Movement::Walker => band(&area, 0.0, SHRUB_TOP),
        Movement::Burrower => band(&area, BOTTOM, 0.0),
        Movement::Climber | Movement::Flyer => area,
    };
    let fly = if h.movement == Movement::Flyer {
        band(&area, h.flight.band.0, h.flight.band.1)
    } else {
        live
    };
    let perch = match (h.movement, h.flight.perch) {
        (Movement::Flyer, Perch::At(Layer::Soil | Layer::Floor)) => Some(band(&area, 0.0, 0.0)),
        (Movement::Flyer, Perch::At(l)) => {
            let (lo, hi) = l.range();
            Some(band(&area, lo, hi))
        }
        _ => None,
    };
    let reach = match (h.movement, perch) {
        (Movement::Climber, _) => Some(live),
        (Movement::Flyer, Some(p)) if p.y0 >= SHRUB_TOP => Some(p),
        _ => None,
    };
    let mut homes = Vec::new();
    if let Some(r) = reach {
        // prefer trees that reach well into the area, not ones it only clips
        let depth = |i: usize| {
            let t = &forest.trees[i];
            t.crown_box().intersect(&r).map_or(0.0, |b| b.y1 - b.y0)
        };
        let enough = (0.3 * (r.y1 - r.y0)).min(2.0);
        homes = (0..forest.trees.len())
            .filter(|&i| forest.trees[i].crown_meets(&r) && depth(i) >= enough)
            .collect();
        if homes.is_empty() {
            homes = (0..forest.trees.len())
                .filter(|&i| forest.trees[i].crown_meets(&r))
                .collect();
        }
        if homes.is_empty() {
            // nothing reaches the area: fall back to the tree nearest its middle
            homes.extend(forest.nearest_tree(0.5 * (r.x0 + r.x1), &[]));
        }
    }
    Places {
        live,
        fly,
        perch,
        homes,
    }
}

/// How far members roam from their anchor.
fn roam(cohesion: f64) -> f64 {
    1.0 + 24.0 * (1.0 - cohesion.clamp(0.0, 1.0))
}

/// A box around a point, cut to `r` (or `r` itself if they don't meet).
fn near(r: &Rect, p: Vec2, radius: f64) -> Rect {
    Rect::new(p.x - radius, p.x + radius, p.y - radius, p.y + radius)
        .intersect(r)
        .unwrap_or(*r)
}

/// A spot for a member of a group to go to (not flyers' perches).
fn spot(
    forest: &Forest,
    sp: &Species,
    pl: &Places,
    anchor: Vec2,
    tree: Option<usize>,
    rng: &mut Rng,
) -> Vec2 {
    let h = &sp.habitat;
    let r = roam(h.cohesion);
    match h.movement {
        Movement::Walker | Movement::Burrower => {
            let x = (anchor.x + rng.range(-r, r)).clamp(pl.live.x0, pl.live.x1);
            // walkers spend most of their time on the floor itself
            let y = if h.movement == Movement::Walker && rng.f64() < 0.7 {
                pl.live.y0
            } else {
                rng.range(pl.live.y0, pl.live.y1)
            };
            Vec2::new(x, y)
        }
        Movement::Climber => {
            let t = &forest.trees[tree.expect("climber without a tree")];
            let mut box_ = near(&pl.live, anchor, r);
            if !t.crown_meets(&box_) {
                box_ = pl.live;
            }
            if !t.crown_meets(&box_) {
                box_ = Rect::world();
            }
            t.crown_point(&box_, rng)
        }
        Movement::Flyer => {
            let b = near(
                &pl.fly,
                Vec2::new(anchor.x, 0.5 * (pl.fly.y0 + pl.fly.y1)),
                r.max(5.0),
            );
            let b = Rect::new(b.x0, b.x1, pl.fly.y0, pl.fly.y1);
            b.random_point(rng)
        }
    }
}

/// A perch for a flyer near x.
fn perch_spot(forest: &Forest, sp: &Species, pl: &Places, x: f64, rng: &mut Rng) -> Vec2 {
    let p = pl.perch.expect("perching flyer without a perch");
    let r = roam(sp.habitat.cohesion).max(10.0);
    if p.y1 <= SHRUB_TOP || pl.homes.is_empty() {
        let x = (x + rng.range(-r, r)).clamp(p.x0, p.x1);
        return Vec2::new(x, rng.range(p.y0, p.y1));
    }
    // a tree with crown in the perch layer: one of the nearer ones
    let mut near_trees: Vec<usize> = pl
        .homes
        .iter()
        .copied()
        .filter(|&i| (forest.trees[i].x - x).abs() <= r + 15.0)
        .collect();
    if near_trees.is_empty() {
        near_trees.extend(forest.nearest_tree(x, &pl.homes));
    }
    let t = &forest.trees[near_trees[rng.int(0, near_trees.len() as i64 - 1) as usize]];
    t.crown_point(&p, rng)
}

/// Mean length of a flight bout for a flight share, seconds (∞ = never perches).
fn flight_mean(sp: &Species) -> f64 {
    let f = &sp.habitat.flight;
    if f.perch == Perch::Never || f.share >= 0.999 {
        f64::INFINITY
    } else {
        PERCH_MEAN * f.share.max(0.0) / (1.0 - f.share)
    }
}

/// Group anchors along the area for a spread: x positions and, for climbers, trees.
fn place_groups(
    forest: &Forest,
    sp: &Species,
    pl: &Places,
    n: usize,
    rng: &mut Rng,
) -> Vec<(Vec2, Option<usize>)> {
    let h = &sp.habitat;
    let r = if h.movement == Movement::Flyer {
        pl.fly
    } else {
        pl.live
    };
    let w = r.width();
    let spacing = h.spacing.max(0.0);
    let mut xs: Vec<f64> = Vec::with_capacity(n);
    let far_enough = |xs: &[f64], x: f64| {
        xs.iter()
            .map(|o| (o - x).abs())
            .fold(f64::INFINITY, f64::min)
    };
    // best of several candidates: the first that keeps Spacing, else the one farthest from the rest
    let pick = |xs: &mut Vec<f64>, mut cand: Box<dyn FnMut(&mut Rng) -> f64>, rng: &mut Rng| {
        let mut best = (f64::NEG_INFINITY, r.x0);
        for _ in 0..30 {
            let x = cand(rng).clamp(r.x0, r.x1);
            let d = far_enough(xs, x);
            if d >= spacing {
                best = (d, x);
                break;
            }
            if d > best.0 {
                best = (d, x);
            }
        }
        xs.push(best.1);
    };
    match h.spread {
        Spread::Even => {
            for k in 0..n {
                xs.push(r.x0 + (k as f64 + 0.5 + rng.range(-0.15, 0.15)) * w / n as f64);
            }
        }
        Spread::Random => {
            for _ in 0..n {
                let (x0, x1) = (r.x0, r.x1);
                pick(&mut xs, Box::new(move |g: &mut Rng| g.range(x0, x1)), rng);
            }
        }
        Spread::Clumped => {
            let patches = ((n as f64) / 4.0).ceil().max(1.0) as usize;
            let centres: Vec<f64> = (0..patches).map(|_| rng.range(r.x0, r.x1)).collect();
            let size = spacing.max(2.0) * 1.5;
            for k in 0..n {
                let c = centres[k % patches];
                pick(
                    &mut xs,
                    Box::new(move |g: &mut Rng| c + g.gauss() * size),
                    rng,
                );
            }
        }
    }
    xs.into_iter()
        .map(|x| match h.movement {
            Movement::Climber => {
                let t = forest.nearest_tree(x, &pl.homes).expect("no trees");
                let tree = &forest.trees[t];
                let b = if tree.crown_meets(&pl.live) {
                    pl.live
                } else {
                    Rect::world()
                };
                (tree.crown_point(&b, rng), Some(t))
            }
            Movement::Flyer => (Vec2::new(x, rng.range(pl.fly.y0, pl.fly.y1)), None),
            _ => (Vec2::new(x, rng.range(pl.live.y0, pl.live.y1)), None),
        })
        .collect()
}

/// Where the listener hears from, for the active zone.
#[derive(Clone, Copy, Debug)]
pub enum Zone {
    /// Within `radius` of the listener.
    Around { pos: Vec2, radius: f64 },
    /// Inside the box grown by `margin` on every side.
    Box { rect: Rect, margin: f64 },
}

impl Zone {
    pub fn contains(&self, p: Vec2) -> bool {
        match *self {
            Zone::Around { pos, radius } => pos.dist(p) <= radius,
            Zone::Box { rect, margin } => rect.contains(p, margin),
        }
    }
}

pub struct Life {
    pub forest: Forest,
    pub species: Vec<Species>,
    pub places: Vec<Places>,
    pub groups: Vec<Group>,
    pub animals: Vec<Individual>,
    /// World time, seconds.
    pub time: f64,
    prev: Vec<Vec2>,
    flock: Vec<(Vec2, Vec2, u32)>,
}

impl Life {
    pub fn new(forest: Forest, species: Vec<Species>) -> Self {
        let seed = forest.settings.seed;
        let places: Vec<Places> = species.iter().map(|s| places(&forest, s)).collect();
        let mut groups = Vec::new();
        let mut animals = Vec::new();
        for (si, sp) in species.iter().enumerate() {
            let h = &sp.habitat;
            let pl = &places[si];
            let mut rng = Rng::stream(seed, 0x5_0000 + si as u64);
            let lo = h.group_size.0.max(1) as i64;
            let hi = (h.group_size.1 as i64).max(lo);
            let mut sizes = Vec::new();
            let mut left = h.individuals;
            while left > 0 {
                let g = (rng.int(lo, hi) as usize).min(left);
                sizes.push(g);
                left -= g;
            }
            let anchors = place_groups(&forest, sp, pl, sizes.len(), &mut rng);
            let fmean = flight_mean(sp);
            for (k, (&size, (anchor, tree))) in sizes.iter().zip(anchors).enumerate() {
                let mut grng =
                    Rng::stream(seed, ((si as u64) << 32) | (k as u64) | 0x1_0000_0000_0000);
                let next_migration = if h.migration_per_hour > 0.0 {
                    exp(&mut grng, 3600.0 / h.migration_per_hour)
                } else {
                    f64::INFINITY
                };
                let gi = groups.len() as u32;
                groups.push(Group {
                    species: si as u32,
                    anchor,
                    tree,
                    first: animals.len(),
                    len: size,
                    migrations: 0,
                    next_migration,
                    rng: grng,
                });
                for _ in 0..size {
                    let id = animals.len() as u64;
                    let mut arng = Rng::stream(seed, 0xA000_0000_0000 + id);
                    let (pos, mode) = if h.movement == Movement::Flyer {
                        let flies = fmean.is_infinite() || arng.f64() < h.flight.share;
                        if flies {
                            let until = exp(&mut arng, fmean);
                            (
                                spot(&forest, sp, pl, anchor, tree, &mut arng),
                                Mode::Flying { until },
                            )
                        } else {
                            let until = exp(&mut arng, PERCH_MEAN);
                            (
                                perch_spot(&forest, sp, pl, anchor.x, &mut arng),
                                Mode::Perched { until },
                            )
                        }
                    } else {
                        let until = arng.range(0.0, PAUSE.1);
                        (
                            spot(&forest, sp, pl, anchor, tree, &mut arng),
                            Mode::Idle { until },
                        )
                    };
                    let a = arng.f64() * std::f64::consts::TAU;
                    let vel = if matches!(mode, Mode::Flying { .. }) {
                        let v = h.top_speed() * 0.6;
                        Vec2::new(a.p_cos() * v, a.p_sin() * 0.3 * v)
                    } else {
                        Vec2::default()
                    };
                    animals.push(Individual {
                        species: si as u32,
                        group: gi,
                        pos,
                        vel,
                        mode,
                        target: pos,
                        active: true,
                        path: [Vec2::default(); 3],
                        path_len: 0,
                        path_i: 0,
                        pace: arng.range(0.6, 1.0),
                        rng: arng,
                    });
                }
            }
        }
        let prev = animals.iter().map(|a| a.pos).collect();
        let flock = vec![(Vec2::default(), Vec2::default(), 0); groups.len()];
        Self {
            forest,
            species,
            places,
            groups,
            animals,
            time: 0.0,
            prev,
            flock,
        }
    }

    /// Run for this many seconds of world time.
    pub fn run(&mut self, seconds: f64) {
        let steps = (seconds / DT).round() as usize;
        for _ in 0..steps {
            self.step();
        }
    }

    pub fn step(&mut self) {
        let t = self.time;
        for gi in 0..self.groups.len() {
            if t >= self.groups[gi].next_migration {
                self.migrate(gi);
            }
        }

        // flocking: per group, centre and mean velocity of the members in the air
        for (p, a) in self.prev.iter_mut().zip(&self.animals) {
            *p = a.pos;
        }
        for f in self.flock.iter_mut() {
            *f = (Vec2::default(), Vec2::default(), 0);
        }
        for a in &self.animals {
            if matches!(a.mode, Mode::Flying { .. })
                && self.species[a.species as usize].habitat.flock
            {
                let f = &mut self.flock[a.group as usize];
                f.0 = f.0.add_scaled(a.pos, 1.0);
                f.1 = f.1.add_scaled(a.vel, 1.0);
                f.2 += 1;
            }
        }

        let Life {
            forest,
            species,
            places,
            groups,
            animals,
            prev,
            flock,
            ..
        } = self;
        for (i, a) in animals.iter_mut().enumerate() {
            let sp = &species[a.species as usize];
            let pl = &places[a.species as usize];
            let g = &groups[a.group as usize];
            match sp.habitat.movement {
                Movement::Flyer => fly_step(
                    i,
                    a,
                    sp,
                    pl,
                    g,
                    forest,
                    prev,
                    &flock[a.group as usize],
                    animals_flying_mates(g),
                    t,
                ),
                _ => ground_step(a, sp, pl, g, forest, t),
            }
        }
        self.time += DT;
    }

    fn migrate(&mut self, gi: usize) {
        let t = self.time;
        let Life {
            forest,
            species,
            places,
            groups,
            animals,
            ..
        } = self;
        let g = &mut groups[gi];
        let sp = &species[g.species as usize];
        let pl = &places[g.species as usize];
        let h = &sp.habitat;
        g.next_migration = t + exp(&mut g.rng, 3600.0 / h.migration_per_hour);
        let d = h.migration_distance.max(0.0);
        let old_tree = g.tree;
        match h.movement {
            Movement::Climber => {
                let cur = old_tree.expect("climber without a tree");
                let cx = forest.trees[cur].x;
                let options: Vec<usize> = pl
                    .homes
                    .iter()
                    .copied()
                    .filter(|&k| k != cur && (forest.trees[k].x - cx).abs() <= d)
                    .collect();
                if options.is_empty() {
                    return;
                }
                let k = options[g.rng.int(0, options.len() as i64 - 1) as usize];
                let tree = &forest.trees[k];
                let b = if tree.crown_meets(&pl.live) {
                    pl.live
                } else {
                    Rect::world()
                };
                g.anchor = tree.crown_point(&b, &mut g.rng);
                g.tree = Some(k);
            }
            Movement::Flyer => {
                let x = (g.anchor.x + g.rng.range(-d, d)).clamp(pl.fly.x0, pl.fly.x1);
                g.anchor = Vec2::new(x, g.rng.range(pl.fly.y0, pl.fly.y1));
            }
            _ => {
                let x = (g.anchor.x + g.rng.range(-d, d)).clamp(pl.live.x0, pl.live.x1);
                g.anchor = Vec2::new(x, g.rng.range(pl.live.y0, pl.live.y1));
            }
        }
        g.migrations += 1;
        let (anchor, tree) = (g.anchor, g.tree);
        for a in &mut animals[g.first..g.first + g.len] {
            match h.movement {
                Movement::Climber => {
                    let target = spot(forest, sp, pl, anchor, tree, &mut a.rng);
                    let (from, to) = (
                        &forest.trees[old_tree.unwrap()],
                        &forest.trees[tree.unwrap()],
                    );
                    if from.touches(to) {
                        a.path[0] = target;
                        a.path_len = 1;
                    } else {
                        // down the trunk, along the floor, up the next one
                        a.path = [Vec2::new(from.x, 0.0), Vec2::new(to.x, 0.0), target];
                        a.path_len = 3;
                    }
                    a.path_i = 0;
                    a.target = target;
                    a.mode = Mode::Travel;
                }
                Movement::Flyer => match a.mode {
                    Mode::Perched { .. } | Mode::ToPerch => {
                        a.target = perch_spot(forest, sp, pl, anchor.x, &mut a.rng);
                        a.mode = Mode::ToPerch;
                    }
                    _ => a.target = spot(forest, sp, pl, anchor, tree, &mut a.rng),
                },
                _ => {
                    a.target = spot(forest, sp, pl, anchor, tree, &mut a.rng);
                    a.mode = Mode::Moving;
                }
            }
        }
    }

    /// Mark who may call: inside the zone. Returns how many.
    pub fn mark_active(&mut self, zone: &Zone) -> usize {
        let mut n = 0;
        for a in &mut self.animals {
            a.active = zone.contains(a.pos);
            n += a.active as usize;
        }
        n
    }

    /// Is this individual where its species is allowed to be? (For tests.) Climbers changing
    /// trees and flyers on their way to a perch are in between and always pass.
    pub fn check(&self, i: usize) -> Result<(), String> {
        let a = &self.animals[i];
        let sp = &self.species[a.species as usize];
        let pl = &self.places[a.species as usize];
        let g = &self.groups[a.group as usize];
        let eps = 1e-6;
        let fail = |what: &str| {
            Err(format!(
                "{} #{i} ({:?}) at ({:.3}, {:.3}): {what}",
                sp.name, a.mode, a.pos.x, a.pos.y
            ))
        };
        match (sp.habitat.movement, a.mode) {
            (_, Mode::Travel | Mode::ToPerch | Mode::Rising { .. }) => Ok(()),
            (Movement::Climber, _) => {
                let t = &self.forest.trees[g.tree.unwrap()];
                if !t.in_crown(a.pos, eps) {
                    return fail("outside its tree's crown");
                }
                if t.crown_meets(&pl.live) && !pl.live.contains(a.pos, eps) {
                    return fail("outside its area");
                }
                Ok(())
            }
            (Movement::Flyer, Mode::Flying { .. }) => {
                if pl.fly.contains(a.pos, eps) {
                    Ok(())
                } else {
                    fail("outside its flight band")
                }
            }
            (Movement::Flyer, Mode::Perched { .. }) => {
                let p = pl.perch.unwrap();
                if !p.contains(a.pos, eps) {
                    return fail("perched outside its perch layer");
                }
                if p.y1 > SHRUB_TOP
                    && !pl.homes.is_empty()
                    && !self.forest.trees.iter().any(|t| t.in_crown(a.pos, eps))
                {
                    return fail("perched in mid-air");
                }
                Ok(())
            }
            _ => {
                if pl.live.contains(a.pos, eps) {
                    Ok(())
                } else {
                    fail("outside its area")
                }
            }
        }
    }
}

/// Members of a group (index range) for the flocking separation pass.
fn animals_flying_mates(g: &Group) -> std::ops::Range<usize> {
    g.first..g.first + g.len
}

/// Move towards a point at a speed. True when it got there this step.
fn move_toward(a: &mut Individual, to: Vec2, speed: f64) -> bool {
    let dx = to.x - a.pos.x;
    let dy = to.y - a.pos.y;
    let d = (dx * dx + dy * dy).sqrt();
    let step = speed * DT;
    if d <= step {
        a.vel = Vec2::new(dx / DT, dy / DT);
        a.pos = to;
        true
    } else {
        a.vel = Vec2::new(dx / d * speed, dy / d * speed);
        a.pos = Vec2::new(a.pos.x + dx / d * step, a.pos.y + dy / d * step);
        false
    }
}

fn ground_step(a: &mut Individual, sp: &Species, pl: &Places, g: &Group, forest: &Forest, t: f64) {
    let speed = sp.habitat.top_speed() * a.pace;
    match a.mode {
        Mode::Idle { until } => {
            a.vel = Vec2::default();
            if t >= until {
                a.target = spot(forest, sp, pl, g.anchor, g.tree, &mut a.rng);
                a.mode = Mode::Moving;
            }
        }
        Mode::Moving => {
            if move_toward(a, a.target, speed) {
                a.mode = Mode::Idle {
                    until: t + a.rng.range(PAUSE.0, PAUSE.1),
                };
            }
        }
        Mode::Travel => {
            let wp = a.path[a.path_i as usize];
            if move_toward(a, wp, speed) {
                a.path_i += 1;
                if a.path_i >= a.path_len {
                    a.mode = Mode::Idle {
                        until: t + a.rng.range(PAUSE.0, PAUSE.1),
                    };
                }
            }
        }
        _ => a.mode = Mode::Idle { until: t },
    }
}

#[allow(clippy::too_many_arguments)]
fn fly_step(
    i: usize,
    a: &mut Individual,
    sp: &Species,
    pl: &Places,
    g: &Group,
    forest: &Forest,
    prev: &[Vec2],
    flock: &(Vec2, Vec2, u32),
    mates: std::ops::Range<usize>,
    t: f64,
) {
    let h = &sp.habitat;
    let v = h.top_speed() * a.pace;
    match a.mode {
        Mode::Perched { until } => {
            a.vel = Vec2::default();
            if t >= until {
                a.mode = Mode::Rising {
                    until: t + exp(&mut a.rng, flight_mean(sp)),
                };
                a.target = spot(forest, sp, pl, g.anchor, g.tree, &mut a.rng);
            }
        }
        Mode::Rising { until } => {
            if move_toward(a, a.target, v) {
                a.mode = Mode::Flying { until };
            }
        }
        Mode::ToPerch => {
            if move_toward(a, a.target, v) {
                a.vel = Vec2::default();
                a.mode = Mode::Perched {
                    until: t + exp(&mut a.rng, PERCH_MEAN),
                };
            }
        }
        Mode::Flying { until } => {
            if t >= until && pl.perch.is_some() {
                a.target = perch_spot(forest, sp, pl, a.pos.x, &mut a.rng);
                a.mode = Mode::ToPerch;
                return;
            }
            let p = a.pos;
            let mut desired;
            if h.flock && flock.2 > 1 {
                let n = flock.2 as f64;
                let centre = Vec2::new(flock.0.x / n, flock.0.y / n);
                let mean_v = Vec2::new(flock.1.x / n, flock.1.y / n);
                let mut sep = Vec2::default();
                for j in mates {
                    if j == i {
                        continue;
                    }
                    let q = prev[j];
                    let (dx, dy) = (p.x - q.x, p.y - q.y);
                    let d2 = dx * dx + dy * dy;
                    if d2 < 9.0 && d2 > 1e-12 {
                        sep = Vec2::new(sep.x + dx / d2, sep.y + dy / d2);
                    }
                }
                let to_c = Vec2::new(centre.x - p.x, centre.y - p.y);
                let to_home = Vec2::new(g.anchor.x - p.x, g.anchor.y - p.y);
                // past its roaming distance the pull home grows, so a flock wheels around home
                let far = (to_home.x.p_hypot(to_home.y) / roam(h.cohesion).max(10.0) - 1.0)
                    .clamp(0.0, 3.0);
                let home = norm(to_home);
                desired = Vec2::new(
                    norm(mean_v).x + 0.08 * to_c.x + 1.5 * sep.x + 0.8 * far * home.x,
                    norm(mean_v).y + 0.08 * to_c.y + 1.5 * sep.y + 0.8 * far * home.y,
                );
            } else {
                if p.dist(a.target) < 2.0 {
                    a.target = spot(forest, sp, pl, g.anchor, g.tree, &mut a.rng);
                }
                desired = Vec2::new(a.target.x - p.x, a.target.y - p.y);
            }
            // a little wander, and stay off the band's edges
            desired = Vec2::new(
                desired.x + 0.3 * a.rng.gauss(),
                desired.y + 0.15 * a.rng.gauss(),
            );
            let edge = 2.0;
            if p.y < pl.fly.y0 + edge {
                desired.y += 2.0;
            }
            if p.y > pl.fly.y1 - edge {
                desired.y -= 2.0;
            }
            if p.x < pl.fly.x0 + edge * 3.0 {
                desired.x += 3.0;
            }
            if p.x > pl.fly.x1 - edge * 3.0 {
                desired.x -= 3.0;
            }
            let d = norm(desired);
            let want = Vec2::new(d.x * v, d.y * v);
            // turn at most this fast
            let max_dv = 3.0 * v * DT;
            let dv = Vec2::new(want.x - a.vel.x, want.y - a.vel.y);
            let dl = dv.x.p_hypot(dv.y);
            let s = if dl > max_dv { max_dv / dl } else { 1.0 };
            let mut vel = Vec2::new(a.vel.x + dv.x * s, a.vel.y + dv.y * s);
            let sp_now = vel.x.p_hypot(vel.y);
            if sp_now < 0.3 * v {
                vel = if sp_now > 1e-9 {
                    Vec2::new(vel.x / sp_now * 0.3 * v, vel.y / sp_now * 0.3 * v)
                } else {
                    Vec2::new(0.3 * v, 0.0)
                };
            }
            let mut np = Vec2::new(p.x + vel.x * DT, p.y + vel.y * DT);
            if np.x < pl.fly.x0 || np.x > pl.fly.x1 {
                vel.x = -vel.x;
            }
            if np.y < pl.fly.y0 || np.y > pl.fly.y1 {
                vel.y = -vel.y;
            }
            np = pl.fly.clamp(np);
            a.pos = np;
            a.vel = vel;
        }
        _ => a.mode = Mode::Flying { until: t },
    }
}

fn norm(v: Vec2) -> Vec2 {
    let l = v.x.p_hypot(v.y);
    if l > 1e-12 {
        Vec2::new(v.x / l, v.y / l)
    } else {
        Vec2::new(1.0, 0.0)
    }
}

/// Fraction of a species' individuals in the air right now.
pub fn flying_share(life: &Life, species: usize) -> f64 {
    let (mut n, mut f) = (0usize, 0usize);
    for a in life
        .animals
        .iter()
        .filter(|a| a.species as usize == species)
    {
        n += 1;
        f += a.flying() as usize;
    }
    f as f64 / n.max(1) as f64
}

/// Highest point any tree reaches (for tests).
pub fn canopy_top(forest: &Forest) -> f64 {
    forest
        .trees
        .iter()
        .map(|t| t.height)
        .fold(0.0, f64::max)
        .min(TOP)
}
