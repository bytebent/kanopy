//! kanopy's files (design §11): `.kanopy-species.json` (one species) and `.kanopy-forest.json`
//! (a whole forest: its world, how it's heard, how it renders, and its species, so it opens on
//! its own). Both carry a format name and a version. Missing fields take their defaults, so a
//! file only needs what differs.

use crate::life::{Life, Zone};
use crate::master::Ear;
use crate::propagation::Medium;
use crate::scale::{Key, Scale};
use crate::scene::{Scene, SceneSettings, BOX_MARGIN};
use crate::species::{demo, Species};
use crate::world::{Forest, ForestSettings, Rect};
use crate::Vec2;
use serde::{Deserialize, Serialize};
use std::path::Path;

pub const VERSION: u32 = 1;
pub const SPECIES_FORMAT: &str = "kanopy-species";
pub const FOREST_FORMAT: &str = "kanopy-forest";

/// A medium by preset name ("earth", "mars", "titan", "venus", "water") or custom values.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MediumFile {
    Preset(String),
    Custom {
        speed: f64,
        density: f64,
        absorption: f64,
        #[serde(default)]
        liquid: bool,
    },
}

impl MediumFile {
    pub fn medium(&self) -> Result<Medium, String> {
        match self {
            MediumFile::Preset(name) => Medium::preset(name).ok_or_else(|| {
                format!("unknown medium \"{name}\": earth, mars, titan, venus or water")
            }),
            MediumFile::Custom {
                speed,
                density,
                absorption,
                liquid,
            } => Ok(Medium::custom(*speed, *density, *absorption, *liquid)),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct KeyFile {
    /// "C", "C#", "Db" …
    pub root: String,
    pub scale: Scale,
}

const NOTE_NAMES: [&str; 12] = [
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];

pub fn parse_root(s: &str) -> Result<i32, String> {
    let t = s.trim();
    let mut chars = t.chars();
    let base = match chars.next().map(|c| c.to_ascii_uppercase()) {
        Some('C') => 0,
        Some('D') => 2,
        Some('E') => 4,
        Some('F') => 5,
        Some('G') => 7,
        Some('A') => 9,
        Some('B') => 11,
        _ => return Err(format!("unknown root \"{s}\"")),
    };
    let shift: i32 = chars
        .map(|c| match c {
            '#' | '♯' => 1,
            'b' | '♭' => -1,
            _ => 99,
        })
        .sum();
    if shift.abs() > 2 {
        return Err(format!("unknown root \"{s}\""));
    }
    Ok((base + shift).rem_euclid(12))
}

impl KeyFile {
    pub fn key(&self) -> Result<Key, String> {
        Ok(Key::new(parse_root(&self.root)?, self.scale))
    }
    pub fn from_key(k: &Key) -> Self {
        Self {
            root: NOTE_NAMES[k.root.rem_euclid(12) as usize].into(),
            scale: k.scale,
        }
    }
}

/// How the forest is heard.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "lowercase")]
pub enum ListenFile {
    /// From the listener at (x, y); animals within `radius` call.
    Listener { x: f64, y: f64, radius: f64 },
    /// Everything inside the box, no distance.
    Box { x0: f64, x1: f64, y0: f64, y1: f64 },
}

/// What `kanopy render` makes of it unless told otherwise.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RenderFile {
    pub minutes: f64,
    pub fade_in: f64,
    pub fade_out: f64,
    /// 16 or 24.
    pub bits: u16,
}

impl Default for RenderFile {
    fn default() -> Self {
        Self {
            minutes: 10.0,
            fade_in: 5.0,
            fade_out: 10.0,
            bits: 24,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ForestFile {
    pub format: String,
    pub version: u32,
    pub name: String,
    pub seed: u64,
    /// 0 woodland … 0.5 forest … 1 thicket.
    pub density: f64,
    pub tree_variation: f64,
    pub medium: MediumFile,
    /// null: Free.
    pub key: Option<KeyFile>,
    pub listen: ListenFile,
    /// Most calling at once.
    pub cap: usize,
    pub ear: Ear,
    pub medium_compensation: f64,
    /// Reverb amount 0…1, or null for off.
    pub reverb: Option<f32>,
    pub bodies_follow: bool,
    pub underwater_stereo: bool,
    pub render: RenderFile,
    pub species: Vec<Species>,
}

impl Default for ForestFile {
    fn default() -> Self {
        let st = SceneSettings::default();
        let f = ForestSettings::default();
        Self {
            format: FOREST_FORMAT.into(),
            version: VERSION,
            name: "Untitled forest".into(),
            seed: f.seed,
            density: f.density,
            tree_variation: f.tree_variation,
            medium: MediumFile::Preset("earth".into()),
            key: st.key.as_ref().map(KeyFile::from_key),
            listen: ListenFile::Listener {
                x: st.listener.x,
                y: st.listener.y,
                radius: 150.0,
            },
            cap: st.cap,
            ear: st.ear,
            medium_compensation: st.medium_compensation,
            reverb: st.reverb.then_some(st.reverb_amount),
            bodies_follow: st.bodies_follow,
            underwater_stereo: st.underwater_stereo,
            render: RenderFile::default(),
            species: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SpeciesFile {
    pub format: String,
    pub version: u32,
    #[serde(flatten)]
    pub species: Species,
}

/// Pretty JSON, with short arrays of numbers (ranges) kept on one line.
pub fn pretty<T: Serialize>(v: &T) -> String {
    let text = serde_json::to_string_pretty(v).expect("to JSON");
    let mut out = String::with_capacity(text.len());
    let mut rest = text.as_str();
    while let Some(i) = rest.find('[') {
        out.push_str(&rest[..i]);
        let after = &rest[i + 1..];
        let end = after.find(']');
        let inner = end.map(|e| &after[..e]);
        match inner {
            Some(inner) if !inner.contains(['{', '[', '"']) && inner.split(',').count() <= 4 => {
                let items: Vec<&str> = inner
                    .split(',')
                    .map(|x| x.trim())
                    .filter(|x| !x.is_empty())
                    .collect();
                out.push('[');
                out.push_str(&items.join(", "));
                out.push(']');
                rest = &after[end.unwrap() + 1..];
            }
            _ => {
                out.push('[');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

fn check(format: &str, version: u32, want: &str) -> Result<(), String> {
    if format != want {
        return Err(format!("not a {want} file (format \"{format}\")"));
    }
    if version > VERSION {
        return Err(format!(
            "made by a newer kanopy (version {version}; this one reads up to {VERSION})"
        ));
    }
    Ok(())
}

impl ForestFile {
    /// The 7-species demo forest.
    pub fn demo() -> Self {
        Self {
            name: "Demo forest".into(),
            species: demo(1.0),
            ..Default::default()
        }
    }

    pub fn from_json(text: &str) -> Result<Self, String> {
        let f: ForestFile = serde_json::from_str(text).map_err(|e| format!("forest file: {e}"))?;
        check(&f.format, f.version, FOREST_FORMAT)?;
        f.settings()?; // medium and key names are valid
        Ok(f)
    }

    pub fn to_json(&self) -> String {
        pretty(self)
    }

    pub fn load(path: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        Self::from_json(&text).map_err(|e| format!("{}: {e}", path.display()))
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        std::fs::write(path, self.to_json() + "\n").map_err(|e| format!("{}: {e}", path.display()))
    }

    /// The forest file stored in a kanopy WAV.
    pub fn from_wav(path: &Path) -> Result<Self, String> {
        let f = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let body = crate::wav::read_chunk(std::io::BufReader::new(f), b"knpy")
            .map_err(|e| format!("{}: {e}", path.display()))?
            .ok_or_else(|| format!("{}: not made by kanopy (no forest inside)", path.display()))?;
        Self::from_json(&String::from_utf8_lossy(&body))
    }

    pub fn settings(&self) -> Result<SceneSettings, String> {
        let medium = self.medium.medium()?;
        let key = self.key.as_ref().map(|k| k.key()).transpose()?;
        let mut st = SceneSettings {
            key,
            cap: self.cap,
            medium,
            ear: self.ear,
            medium_compensation: self.medium_compensation,
            reverb: self.reverb.is_some(),
            reverb_amount: self.reverb.unwrap_or(0.0),
            bodies_follow: self.bodies_follow,
            underwater_stereo: self.underwater_stereo,
            ..Default::default()
        };
        match self.listen {
            ListenFile::Listener { x, y, radius } => {
                st.listener = Vec2::new(x, y);
                st.zone = Zone::Around {
                    pos: st.listener,
                    radius,
                };
            }
            ListenFile::Box { x0, x1, y0, y1 } => {
                let r = Rect::new(x0.min(x1), x0.max(x1), y0.min(y1), y0.max(y1));
                st.listener = Vec2::new(0.5 * (r.x0 + r.x1), 0.5 * (r.y0 + r.y1));
                st = st.boxed(r);
                debug_assert!(matches!(st.zone, Zone::Box { margin, .. } if margin == BOX_MARGIN));
            }
        }
        Ok(st)
    }

    /// The forest, alive and ready to play (without sound; add it with `with_sound`).
    pub fn scene(&self) -> Result<Scene, String> {
        if self.species.is_empty() {
            return Err("the forest has no species".into());
        }
        let st = self.settings()?;
        let forest = Forest::generate(ForestSettings {
            seed: self.seed,
            density: self.density,
            tree_variation: self.tree_variation,
            liquid: st.medium.liquid,
        });
        Ok(Scene::new(Life::new(forest, self.species.clone()), st))
    }
}

impl SpeciesFile {
    pub fn new(species: Species) -> Self {
        Self {
            format: SPECIES_FORMAT.into(),
            version: VERSION,
            species,
        }
    }
    pub fn from_json(text: &str) -> Result<Species, String> {
        let f: SpeciesFile =
            serde_json::from_str(text).map_err(|e| format!("species file: {e}"))?;
        check(&f.format, f.version, SPECIES_FORMAT)?;
        Ok(f.species)
    }
    pub fn to_json(species: &Species) -> String {
        pretty(&SpeciesFile::new(species.clone()))
    }
    pub fn load(path: &Path) -> Result<Species, String> {
        let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        Self::from_json(&text).map_err(|e| format!("{}: {e}", path.display()))
    }
    pub fn save(species: &Species, path: &Path) -> Result<(), String> {
        std::fs::write(path, Self::to_json(species) + "\n")
            .map_err(|e| format!("{}: {e}", path.display()))
    }
}
