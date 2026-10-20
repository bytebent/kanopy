//! A species' habitat and movement traits (design §5.3–5.4). The voice and call traits come in
//! batch 03.

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
}

impl Species {
    pub fn new(name: &str, habitat: Habitat) -> Self {
        Self {
            name: name.into(),
            habitat,
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
        ),
    ]
}
