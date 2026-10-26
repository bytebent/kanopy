//! The listener's path (design §4.1): Still, Wander or Follow, at a Speed. Used by renders and
//! by live play, so both walk the same way.
//!
//! - **Wander** heads for spots picked from the seed, steering towards each one rather than
//!   turning on the spot, so its Doppler never jumps. Underground only when allowed.
//! - **Follow** keeps a couple of metres off the nearest group of a species.
//! - Each step the listener starts where the last step's motion left it, so the sound's delay
//!   and level never jump between steps.
use serde::{Deserialize, Serialize};

use crate::life::{Life, DT};
use crate::pm::Pm;
use crate::rng::Rng;
use crate::species::Movement;
use crate::world::{Rect, BOTTOM, TOP, WIDTH};
use crate::Vec2;
use std::f64::consts::TAU;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum PathKind {
    #[default]
    Still,
    Wander,
    Follow,
}

/// The path as kept in forest files.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PathFile {
    pub kind: PathKind,
    /// m/s.
    pub speed: f64,
    /// Follow: the species, by name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub follow: Option<String>,
    /// Wander may go below the surface.
    pub underground: bool,
}

impl Default for PathFile {
    fn default() -> Self {
        Self {
            kind: PathKind::Still,
            speed: 1.5,
            follow: None,
            underground: false,
        }
    }
}

/// Seconds to reach a new speed (most of the way).
pub const TURN: f64 = 1.5;
/// Tightest turn, metres (radius): faster walkers turn faster, so turns stay this tight.
pub const TURN_RADIUS: f64 = 3.0;
/// Edges of the world: outward speed at most this × the distance left (per second).
pub const WALL: f64 = 2.0;
/// Follow slows down over about this many seconds' worth of distance.
pub const ARRIVE: f64 = 1.0;
/// A Wander spot counts as reached this close, metres (at least two turning radii); a new one
/// is picked.
pub const REACHED: f64 = 3.0;

fn world() -> Rect {
    Rect::new(0.0, WIDTH, BOTTOM, TOP)
}

#[derive(Clone, Debug)]
pub struct Walker {
    pub kind: PathKind,
    pub speed: f64,
    /// Follow: species index.
    pub follow: Option<usize>,
    pub underground: bool,
    pub pos: Vec2,
    pub vel: Vec2,
    /// Which way it's facing, radians, and how fast it's going, m/s.
    heading: f64,
    pace: f64,
    goal: Vec2,
    since: f64,
    group: Option<usize>,
    rng: Rng,
}

impl Walker {
    pub fn new(
        kind: PathKind,
        speed: f64,
        follow: Option<usize>,
        underground: bool,
        pos: Vec2,
        seed: u64,
    ) -> Self {
        Self {
            kind,
            speed: speed.clamp(0.05, 30.0),
            follow,
            underground,
            pos,
            vel: Vec2::default(),
            heading: 0.0,
            pace: 0.0,
            goal: pos,
            since: f64::NEG_INFINITY,
            group: None,
            rng: Rng::stream(seed, 0x9A7B_0000_0000),
        }
    }

    /// From a forest file's path; Follow names a species of `life`.
    pub fn from_file(p: &PathFile, life: &Life, pos: Vec2, seed: u64) -> Result<Self, String> {
        let follow = match (&p.kind, &p.follow) {
            (PathKind::Follow, Some(name)) => Some(
                life.species
                    .iter()
                    .position(|s| s.name.eq_ignore_ascii_case(name))
                    .ok_or_else(|| {
                        format!("follow: no species called \"{name}\" in this forest")
                    })?,
            ),
            (PathKind::Follow, None) => return Err("follow: which species?".into()),
            _ => None,
        };
        Ok(Self::new(p.kind, p.speed, follow, p.underground, pos, seed))
    }

    /// Put it somewhere (a drag), standing still; the path carries on from there.
    pub fn place(&mut self, pos: Vec2, vel: Vec2) {
        self.pos = world().clamp(pos);
        self.vel = vel;
        self.pace = (vel.x * vel.x + vel.y * vel.y).sqrt();
        if self.pace > 1e-9 {
            self.heading = vel.y.p_atan2(vel.x);
        }
        self.goal = self.pos;
        self.since = f64::NEG_INFINITY;
    }

    /// Change the path, keeping where it is and how it's moving.
    pub fn set(&mut self, kind: PathKind, speed: f64, follow: Option<usize>, underground: bool) {
        self.kind = kind;
        self.speed = speed.clamp(0.05, 30.0);
        self.follow = follow;
        self.underground = underground;
        self.group = None;
        self.since = f64::NEG_INFINITY;
    }

    /// One step: move on by the last step's velocity, then steer. Returns where the listener is
    /// at the start of this step and how it moves through it.
    ///
    /// Steering keeps its speed and turns its heading at a limited rate, like a vehicle, so
    /// turns are arcs and the speed never dips at a corner. Near the edges of the world (and the
    /// ground, when it mustn't go under) the part of its motion heading out eases off with the
    /// distance left, so it slides along them rather than stopping dead.
    pub fn step(&mut self, life: &Life) -> (Vec2, Vec2) {
        let t = life.time;
        self.pos = world().clamp(self.pos.add_scaled(self.vel, DT));
        let (heading_to, want) = match self.target(life, t) {
            Some((goal, speed)) => {
                let (dx, dy) = (goal.x - self.pos.x, goal.y - self.pos.y);
                let d = (dx * dx + dy * dy).sqrt();
                // slow down over the last metres so Follow settles instead of circling
                (dy.p_atan2(dx), speed.min(d / ARRIVE))
            }
            None => (self.heading, 0.0),
        };
        if self.pace < 0.05 {
            self.heading = heading_to; // standing: just face the way
        } else {
            let most = self.turn_rate() * DT;
            let mut d = heading_to - self.heading;
            d -= TAU * (d / TAU).round();
            self.heading += d.clamp(-most, most);
            self.heading -= TAU * (self.heading / TAU).round();
        }
        self.pace += (want - self.pace) * (3.0 * DT / TURN).min(1.0);
        let mut v = Vec2::new(
            self.pace * self.heading.p_cos(),
            self.pace * self.heading.p_sin(),
        );
        // soft edges: the outward part of the motion at most WALL × the distance left
        let floor = if self.kind == PathKind::Wander && !self.underground && self.pos.y >= 0.3 {
            0.3
        } else {
            BOTTOM + 0.2
        };
        let (x0, x1, y1) = (0.2, WIDTH - 0.2, TOP - 0.2);
        v.x = v.x.clamp(
            -WALL * (self.pos.x - x0).max(0.0),
            WALL * (x1 - self.pos.x).max(0.0),
        );
        v.y = v.y.clamp(
            -WALL * (self.pos.y - floor).max(0.0),
            WALL * (y1 - self.pos.y).max(0.0),
        );
        self.vel = v;
        (self.pos, self.vel)
    }

    /// How fast it may turn, rad/s: arcs at most TURN_RADIUS across at any speed, and never
    /// slower than one radian a second.
    fn turn_rate(&self) -> f64 {
        (self.speed / TURN_RADIUS).max(1.0)
    }

    /// Where it's heading and how fast, or None to stop.
    fn target(&mut self, life: &Life, t: f64) -> Option<(Vec2, f64)> {
        match self.kind {
            PathKind::Still => None,
            PathKind::Wander => {
                // close enough that circling it isn't needed
                let reached = REACHED.max(2.0 * TURN_RADIUS);
                if self.pos.dist(self.goal) < reached || t - self.since > 60.0 {
                    let w = world();
                    let y = if self.underground && self.rng.f64() < 0.1 {
                        self.rng.range(BOTTOM + 0.3, -0.5)
                    } else {
                        self.rng.range(0.5, TOP - 2.0)
                    };
                    let x =
                        (self.pos.x + self.rng.range(-60.0, 60.0)).clamp(w.x0 + 2.0, w.x1 - 2.0);
                    self.goal = Vec2::new(x, y);
                    self.since = t;
                }
                Some((self.goal, self.speed))
            }
            PathKind::Follow => {
                let sp = self.follow?;
                let centre = |g: usize| {
                    let gr = &life.groups[g];
                    let mut c = Vec2::default();
                    for a in &life.animals[gr.first..gr.first + gr.len] {
                        c = c.add_scaled(a.pos, 1.0 / gr.len as f64);
                    }
                    c
                };
                let valid = self
                    .group
                    .filter(|&g| g < life.groups.len() && life.groups[g].species as usize == sp);
                let g = match valid {
                    Some(g) => g,
                    None => (0..life.groups.len())
                        .filter(|&g| life.groups[g].species as usize == sp)
                        .min_by(|&a, &b| {
                            self.pos
                                .dist(centre(a))
                                .total_cmp(&self.pos.dist(centre(b)))
                        })?,
                };
                self.group = Some(g);
                let c = centre(g);
                let off = if life.species[sp].habitat.movement == Movement::Burrower {
                    Vec2::new(-2.0, 0.0)
                } else {
                    Vec2::new(-2.0, -1.5)
                };
                Some((world().clamp(c.add_scaled(off, 1.0)), self.speed))
            }
        }
    }
}
