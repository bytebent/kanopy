//! A species: its voice and calls (design §5.1–5.2), habitat and movement (§5.3–5.4) and
//! behaviour (§5.5). Every voice and call number is a range; each call rolls inside it, and
//! lo = hi locks it.

use crate::world::{Layer, Rect, WIDTH};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Movement {
    /// On the floor and in the shrubs (seabed in a liquid).
    Walker,
    /// Inside tree crowns, moving between trees by migrating (kelp in a liquid).
    Climber,
    /// Anywhere in its flight band, resting at its perch (swims in a liquid).
    Flyer,
    /// In the soil (sediment in a liquid).
    Burrower,
}

impl Movement {
    /// Top speed, m/s. Internal defaults for now.
    pub fn speed(self) -> f64 {
        match self {
            Movement::Walker => 1.2,
            Movement::Climber => 0.8,
            Movement::Flyer => 9.0,
            Movement::Burrower => 0.05,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Spread {
    /// Evenly spaced across the area, like territories.
    Even,
    /// Anywhere in the area.
    Random,
    /// Bunched into a few patches.
    Clumped,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Perch {
    /// Rests in this layer: a tree layer (in crowns), Shrubs, or Floor (on the ground).
    At(Layer),
    /// Always airborne.
    Never,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Flight {
    /// Heights it flies in, metres.
    pub band: (f64, f64),
    pub perch: Perch,
    /// Share of its time spent flying rather than perched, 0…1.
    pub share: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Habitat {
    pub movement: Movement,
    /// Where it lives. Every individual stays inside.
    pub area: Rect,
    pub individuals: usize,
    pub spread: Spread,
    /// Smallest distance kept between neighbouring groups, metres.
    pub spacing: f64,
    /// Group size range; 1–1 is solitary.
    pub group_size: (u32, u32),
    /// 0 = members roam up to 25 m from their group's centre, 1 = within 1 m.
    pub cohesion: f64,
    /// Movers only: the group moves together.
    pub flock: bool,
    /// Flyers only.
    pub flight: Flight,
    /// How often a group moves on, per hour, and how far, metres.
    pub migration_per_hour: f64,
    pub migration_distance: f64,
}

impl Default for Habitat {
    fn default() -> Self {
        Self {
            movement: Movement::Walker,
            area: Rect::new(0.0, WIDTH, 0.0, 3.0),
            individuals: 20,
            spread: Spread::Random,
            spacing: 5.0,
            group_size: (1, 1),
            cohesion: 0.5,
            flock: false,
            flight: Flight {
                band: (5.0, 30.0),
                perch: Perch::At(Layer::Canopy),
                share: 0.5,
            },
            migration_per_hour: 0.0,
            migration_distance: 30.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Species {
    pub name: String,
    pub habitat: Habitat,
    pub voice: Voice,
    pub calls: Calls,
    pub behaviour: Behaviour,
}

impl Species {
    pub fn new(name: &str, habitat: Habitat) -> Self {
        Self {
            name: name.into(),
            habitat,
            voice: Voice::default(),
            calls: Calls::default(),
            behaviour: Behaviour::default(),
        }
    }
    pub fn with(mut self, voice: Voice, calls: Calls, behaviour: Behaviour) -> Self {
        self.voice = voice;
        self.calls = calls;
        self.behaviour = behaviour;
        self
    }
}

/// lo…hi. lo = hi locks it.
pub type Range = (f64, f64);

/// The prototype voice (§5.1).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Voice {
    /// 0 sine · 1 triangle · 2 saw · 3 square, continuous.
    pub shape: Range,
    /// Body: resonant high-pass cutoff, Hz.
    pub cutoff: Range,
    /// 0…1.
    pub resonance: Range,
    /// Anchor notes, MIDI.
    pub pitch: Range,
    /// Glide across a unit, semitones.
    pub contour: Range,
    pub attack: Range,
    pub release: Range,
    /// dB SPL at 1 m.
    pub level: Range,
}

impl Default for Voice {
    fn default() -> Self {
        Self {
            shape: (0.0, 1.5),
            cutoff: (100.0, 800.0),
            resonance: (0.2, 0.6),
            pitch: (72.0, 84.0),
            contour: (-2.0, 2.0),
            attack: (0.005, 0.03),
            release: (0.02, 0.1),
            level: (65.0, 75.0),
        }
    }
}

/// How units change across a call.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Evolve {
    Steady,
    Rise,
    Fall,
    Arc,
    Wander,
}

/// Call grammar (§5.2) and signature.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Calls {
    /// Units per call, 1–32.
    pub units: (u32, u32),
    /// Seconds.
    pub unit_length: Range,
    /// Silence between units, seconds.
    pub gap: Range,
    /// Silence between calls, seconds.
    pub rest: Range,
    pub evolve: Evolve,
    /// How far units move across a call, as a share of the pitch range.
    pub evolve_amount: Range,
    /// How often a call is the group's signature, 0…1.
    pub signature: f64,
    /// How much each repeat of the signature changes, 0…1.
    pub variation: f64,
}

impl Default for Calls {
    fn default() -> Self {
        Self {
            units: (1, 4),
            unit_length: (0.08, 0.3),
            gap: (0.03, 0.15),
            rest: (4.0, 12.0),
            evolve: Evolve::Steady,
            evolve_amount: (0.0, 0.3),
            signature: 0.3,
            variation: 0.2,
        }
    }
}

/// Behaviour (§5.5–5.6).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Behaviour {
    /// How readily it calls when its rest is over, 0…1.
    pub talkative: f64,
    /// Odds it answers a heard call of its own species, 0…1.
    pub response: f64,
    /// −1 alternate (take turns) … 0 free … 1 synchronise.
    pub chorus: f64,
    /// Odds a call is pitched consonant with group mates it hears, 0…1.
    pub harmonise: f64,
    /// How often it copies something it heard instead of its own call, 0…1.
    pub mimicry_chance: f64,
    /// How close its copies are, 0…1.
    pub mimicry_accuracy: f64,
    /// 0 keeps its own sound, 1 aims for the caller's Shape and body.
    pub timbre_match: f64,
    /// 0 copies pitch steps in its own range, 1 aims for the caller's pitch.
    pub pitch_match: f64,
    /// Quietest arriving level it reacts to, dB SPL.
    pub hearing: f64,
}

impl Default for Behaviour {
    fn default() -> Self {
        Self {
            talkative: 0.6,
            response: 0.3,
            chorus: 0.0,
            harmonise: 0.0,
            mimicry_chance: 0.0,
            mimicry_accuracy: 0.7,
            timbre_match: 0.5,
            pitch_match: 0.5,
            hearing: 30.0,
        }
    }
}

/// A mixed forest to look at and test with: seven species, about 2,300 individuals.
/// `scale` multiplies every count.
pub fn demo(scale: f64) -> Vec<Species> {
    let n = |k: f64| ((k * scale).round() as usize).max(1);
    let w = WIDTH;
    vec![
        Species::new(
            "Floor hoppers",
            Habitat {
                movement: Movement::Walker,
                area: Rect::new(0.0, w, 0.0, 3.0),
                individuals: n(300.0),
                spread: Spread::Clumped,
                spacing: 10.0,
                group_size: (3, 8),
                cohesion: 0.6,
                migration_per_hour: 3.0,
                migration_distance: 30.0,
                ..Default::default()
            },
        )
        .with(
            Voice {
                shape: (1.5, 2.5),
                cutoff: (800.0, 2500.0),
                resonance: (0.5, 0.9),
                pitch: (72.0, 84.0),
                contour: (-1.0, 1.0),
                attack: (0.002, 0.006),
                release: (0.01, 0.03),
                level: (60.0, 70.0),
            },
            Calls {
                units: (3, 8),
                unit_length: (0.03, 0.08),
                gap: (0.04, 0.12),
                rest: (4.0, 15.0),
                evolve: Evolve::Steady,
                evolve_amount: (0.0, 0.1),
                signature: 0.4,
                variation: 0.2,
            },
            Behaviour {
                talkative: 0.6,
                response: 0.4,
                chorus: 0.6,
                ..Default::default()
            },
        ),
        Species::new(
            "Diggers",
            Habitat {
                movement: Movement::Burrower,
                area: Rect::new(0.0, 120.0, -3.0, 0.0),
                individuals: n(100.0),
                spread: Spread::Even,
                spacing: 1.0,
                group_size: (1, 1),
                cohesion: 0.9,
                ..Default::default()
            },
        )
        .with(
            Voice {
                shape: (0.0, 0.5),
                cutoff: (20.0, 60.0),
                resonance: (0.3, 0.6),
                pitch: (36.0, 48.0),
                contour: (-3.0, 0.0),
                attack: (0.01, 0.03),
                release: (0.08, 0.2),
                level: (70.0, 78.0),
            },
            Calls {
                units: (1, 3),
                unit_length: (0.15, 0.4),
                gap: (0.2, 0.5),
                rest: (8.0, 30.0),
                evolve: Evolve::Fall,
                evolve_amount: (0.0, 0.2),
                signature: 0.5,
                variation: 0.2,
            },
            Behaviour {
                talkative: 0.5,
                response: 0.2,
                chorus: -0.8,
                hearing: 40.0,
                ..Default::default()
            },
        ),
        Species::new(
            "Crown chatterers",
            Habitat {
                movement: Movement::Climber,
                area: Rect::new(0.0, w, 15.0, 30.0),
                individuals: n(600.0),
                spread: Spread::Random,
                spacing: 15.0,
                group_size: (5, 15),
                cohesion: 0.7,
                migration_per_hour: 4.0,
                migration_distance: 40.0,
                ..Default::default()
            },
        )
        .with(
            Voice {
                shape: (1.0, 3.0),
                cutoff: (600.0, 2000.0),
                resonance: (0.3, 0.7),
                pitch: (76.0, 90.0),
                contour: (-3.0, 5.0),
                attack: (0.003, 0.01),
                release: (0.02, 0.05),
                level: (65.0, 75.0),
            },
            Calls {
                units: (4, 12),
                unit_length: (0.04, 0.12),
                gap: (0.02, 0.08),
                rest: (3.0, 12.0),
                evolve: Evolve::Wander,
                evolve_amount: (0.1, 0.4),
                signature: 0.3,
                variation: 0.3,
            },
            Behaviour {
                talkative: 0.6,
                response: 0.6,
                harmonise: 0.5,
                mimicry_chance: 0.15,
                ..Default::default()
            },
        ),
        Species::new(
            "Lower climbers",
            Habitat {
                movement: Movement::Climber,
                area: Rect::new(50.0, w, 3.0, 15.0),
                individuals: n(300.0),
                spread: Spread::Even,
                spacing: 8.0,
                group_size: (2, 4),
                cohesion: 0.5,
                migration_per_hour: 2.0,
                migration_distance: 25.0,
                ..Default::default()
            },
        )
        .with(
            Voice {
                shape: (0.0, 0.8),
                cutoff: (60.0, 200.0),
                resonance: (0.2, 0.5),
                pitch: (55.0, 67.0),
                contour: (-2.0, 0.0),
                attack: (0.04, 0.1),
                release: (0.1, 0.3),
                level: (68.0, 76.0),
            },
            Calls {
                units: (1, 3),
                unit_length: (0.3, 0.8),
                gap: (0.2, 0.5),
                rest: (6.0, 20.0),
                evolve: Evolve::Fall,
                evolve_amount: (0.1, 0.3),
                signature: 0.7,
                variation: 0.2,
            },
            Behaviour {
                talkative: 0.5,
                response: 0.5,
                harmonise: 0.8,
                ..Default::default()
            },
        ),
        Species::new(
            "Swifts",
            Habitat {
                movement: Movement::Flyer,
                area: Rect::new(0.0, w, 0.0, 50.0),
                individuals: n(400.0),
                spread: Spread::Random,
                spacing: 20.0,
                group_size: (20, 40),
                cohesion: 0.4,
                flock: true,
                flight: Flight {
                    band: (34.0, 50.0),
                    perch: Perch::At(Layer::Emergent),
                    share: 0.85,
                },
                migration_per_hour: 6.0,
                migration_distance: 80.0,
                ..Default::default()
            },
        )
        .with(
            Voice {
                shape: (2.0, 3.0),
                cutoff: (1500.0, 4000.0),
                resonance: (0.4, 0.8),
                pitch: (88.0, 98.0),
                contour: (-5.0, -1.0),
                attack: (0.01, 0.03),
                release: (0.05, 0.1),
                level: (70.0, 80.0),
            },
            Calls {
                units: (1, 4),
                unit_length: (0.2, 0.5),
                gap: (0.05, 0.2),
                rest: (5.0, 20.0),
                evolve: Evolve::Steady,
                evolve_amount: (0.0, 0.1),
                signature: 0.2,
                variation: 0.3,
            },
            Behaviour {
                talkative: 0.6,
                response: 0.3,
                chorus: 0.8,
                ..Default::default()
            },
        ),
        Species::new(
            "Warblers",
            Habitat {
                movement: Movement::Flyer,
                area: Rect::new(0.0, w, 0.0, 30.0),
                individuals: n(400.0),
                spread: Spread::Even,
                spacing: 6.0,
                group_size: (1, 3),
                cohesion: 0.6,
                flight: Flight {
                    band: (5.0, 25.0),
                    perch: Perch::At(Layer::Subcanopy),
                    share: 0.3,
                },
                migration_per_hour: 1.0,
                migration_distance: 20.0,
                ..Default::default()
            },
        )
        .with(
            Voice {
                shape: (0.0, 1.2),
                cutoff: (300.0, 1200.0),
                resonance: (0.1, 0.5),
                pitch: (79.0, 96.0),
                contour: (-7.0, 7.0),
                attack: (0.003, 0.015),
                release: (0.01, 0.05),
                level: (65.0, 78.0),
            },
            Calls {
                units: (6, 16),
                unit_length: (0.06, 0.2),
                gap: (0.02, 0.1),
                rest: (2.0, 8.0),
                evolve: Evolve::Arc,
                evolve_amount: (0.1, 0.5),
                signature: 0.5,
                variation: 0.3,
            },
            Behaviour {
                talkative: 0.7,
                response: 0.3,
                mimicry_chance: 0.35,
                mimicry_accuracy: 0.7,
                timbre_match: 0.6,
                pitch_match: 0.5,
                ..Default::default()
            },
        ),
        Species::new(
            "Gliders",
            Habitat {
                movement: Movement::Flyer,
                area: Rect::new(20.0, 180.0, 0.0, 50.0),
                individuals: n(200.0),
                spread: Spread::Clumped,
                spacing: 20.0,
                group_size: (6, 12),
                cohesion: 0.5,
                flock: true,
                flight: Flight {
                    band: (15.0, 35.0),
                    perch: Perch::At(Layer::Canopy),
                    share: 0.6,
                },
                migration_per_hour: 3.0,
                migration_distance: 50.0,
                ..Default::default()
            },
        )
        .with(
            Voice {
                shape: (0.0, 0.3),
                cutoff: (100.0, 400.0),
                resonance: (0.1, 0.3),
                pitch: (74.0, 86.0),
                contour: (-2.0, 12.0),
                attack: (0.05, 0.2),
                release: (0.1, 0.4),
                level: (68.0, 76.0),
            },
            Calls {
                units: (1, 2),
                unit_length: (0.5, 1.5),
                gap: (0.2, 0.6),
                rest: (8.0, 25.0),
                evolve: Evolve::Rise,
                evolve_amount: (0.0, 0.3),
                signature: 0.4,
                variation: 0.2,
            },
            Behaviour {
                talkative: 0.5,
                response: 0.3,
                mimicry_chance: 0.1,
                ..Default::default()
            },
        ),
    ]
}
