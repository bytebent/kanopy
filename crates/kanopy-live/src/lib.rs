//! kanopy live: plays a forest in real time (design §10: one render thread).
//!
//! - A **render thread** owns the scene. It steps it (50 ms at a time) a little ahead of the
//!   sound card, into a ring buffer about 125 ms long, and handles commands between steps.
//! - The **sound card** (CoreAudio on macOS, through cpal) reads the ring. Without the `audio`
//!   feature, or when there's no output device, the render thread paces itself to real time and
//!   the sound goes nowhere (for testing the page in a browser).
//! - **Frames** for the screen are taken after each step and published when the sound they belong
//!   to is actually playing, so the dots light up with what you hear.
//! - The **listener** glides to where it's dragged (so dragging makes real Doppler), or follows
//!   its path: Still, Wander (drifts anywhere at Speed) or Follow (the nearest group of a
//!   species).
//! - **Auditions** (the Designer's Call and Signature): one creature, dry, from 1 m, played on top.
//!   While the forest plays it ducks under the creature; when it's stopped only the creature plays.
//! - Changing the seed, density, tree variation, world or species rebuilds the forest; the old
//!   one fades out over one step. Listener, box, key, reverb, Ear and volume change live.

use kanopy_core::files::{ForestFile, KeyFile, ListenFile, MediumFile, RenderFile};
use kanopy_core::life::DT;
use kanopy_core::master::Ear;
use kanopy_core::mixer::Hear;
use kanopy_core::rng::Rng;
use kanopy_core::scene::Scene;
use kanopy_core::species::{Movement, Species};
use kanopy_core::world::{Rect, BOTTOM, TOP, WIDTH};
use kanopy_core::{db_to_amp, Vec2};
use ringbuf::traits::{Consumer, Observer, Producer, Split};
use ringbuf::HeapRb;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub mod library;

/// How far ahead of the sound card the render thread stays, seconds.
pub const AHEAD: f64 = 0.125;
/// Most individuals a forest may have in the app (the tested ceiling, design §10).
pub const MAX_INDIVIDUALS: usize = 20_000;
/// Fastest the listener glides to where it's dragged, m/s.
pub const DRAG_SPEED: f64 = 40.0;
/// Frame layout version (first 4 bytes of every frame).
pub const FRAME_MAGIC: &[u8; 4] = b"KNF1";
/// Bytes before the animals in a frame, and per animal.
pub const FRAME_HEADER: usize = 68;
pub const FRAME_ANIMAL: usize = 12;
/// How far the forest ducks under an audition, dB, and how fast: down, back up (seconds).
pub const DUCK_DB: f32 = -15.0;
pub const DUCK_DOWN: f32 = 0.04;
pub const DUCK_UP: f32 = 0.5;

/// What an audition plays.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Take {
    /// A fresh call.
    Call,
    /// Its signature, as a group repeats it.
    Signature,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum PathKind {
    #[default]
    Still,
    Wander,
    Follow,
}

/// What the page can ask for.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Cmd {
    Play {
        on: bool,
    },
    /// Drag the listener here.
    Listener {
        x: f64,
        y: f64,
    },
    /// Listen to a box [x0, x1, y0, y1], or from the listener (null).
    Hear {
        rect: Option<[f64; 4]>,
    },
    Path {
        kind: PathKind,
        speed: f64,
        species: Option<usize>,
    },
    Radius {
        radius: f64,
    },
    Seed {
        seed: u64,
    },
    Density {
        density: f64,
    },
    Variation {
        variation: f64,
    },
    Medium {
        medium: String,
    },
    UnderwaterStereo {
        on: bool,
    },
    Compensation {
        amount: f64,
    },
    Key {
        key: Option<KeyFile>,
    },
    Reverb {
        amount: Option<f32>,
    },
    Ear {
        ear: Ear,
    },
    /// Playback volume, dB (not part of renders).
    Volume {
        db: f32,
    },
    /// Replace the whole forest.
    Load {
        forest: Box<ForestFile>,
    },
    /// Rename it.
    Name {
        name: String,
    },
    /// Replace the forest's species (the Species page's Apply): grows a new forest.
    Species {
        species: Vec<kanopy_core::species::Species>,
    },
    /// Play these samples (mono, from `Live::audition`) on top, ducking the forest.
    #[serde(skip)]
    Sound(Vec<f32>),
    /// Stop an audition.
    Hush,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct ExportState {
    pub running: bool,
    pub done_s: f64,
    pub total_s: f64,
    /// Path written, or the error.
    pub result: Option<Result<String, String>>,
}

#[derive(Default)]
struct Shared {
    /// The newest frame whose sound is playing.
    frame: Mutex<Vec<u8>>,
    /// About the forest (trees, species, settings): JSON, changes on every rebuild.
    info: Mutex<String>,
    /// The forest as it is now (listener and box included), for Save and Export.
    forest: Mutex<ForestFile>,
    error: Mutex<Option<String>>,
    export: Mutex<ExportState>,
    underruns: AtomicU64,
    stop: AtomicBool,
    /// Auditions so far (each press is a new call).
    takes: AtomicU64,
}

pub struct Live {
    tx: Sender<Cmd>,
    shared: Arc<Shared>,
    pub sample_rate: u32,
    /// Sound goes to a real device.
    pub audio: bool,
}

impl Live {
    /// Start playing engine for this forest (paused). `audio`: try the sound card.
    pub fn start(forest: ForestFile, audio: bool) -> Result<Live, String> {
        forest.scene()?; // fail early on a bad forest
        let shared = Arc::new(Shared::default());
        *shared.forest.lock().unwrap() = forest.clone();
        let (tx, rx) = channel();
        fn ring_len(sr: u32) -> usize {
            ((AHEAD + 2.0 * DT) * sr as f64) as usize * 2
        }
        // the sound card, if any: it owns the stream on its own thread (streams aren't Send)
        let (sr, prod, has_audio) = match if audio {
            output::open(Arc::clone(&shared), ring_len)
        } else {
            Err("no audio".into())
        } {
            Ok((sr, prod)) => (sr, Some(prod), true),
            Err(_) => (48_000, None, false),
        };
        let sh = Arc::clone(&shared);
        std::thread::Builder::new()
            .name("kanopy-render".into())
            .spawn(move || render_thread(forest, sr, rx, prod, sh))
            .map_err(|e| e.to_string())?;
        // wait for the first info
        for _ in 0..500 {
            if !shared.info.lock().unwrap().is_empty() {
                break;
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        Ok(Live {
            tx,
            shared,
            sample_rate: sr,
            audio: has_audio,
        })
    }

    pub fn send(&self, cmd: Cmd) {
        let _ = self.tx.send(cmd);
    }

    /// The latest frame for the screen (see `encode_frame` for the layout).
    pub fn frame(&self) -> Vec<u8> {
        self.shared.frame.lock().unwrap().clone()
    }

    pub fn info(&self) -> String {
        self.shared.info.lock().unwrap().clone()
    }

    pub fn forest(&self) -> ForestFile {
        self.shared.forest.lock().unwrap().clone()
    }

    pub fn error(&self) -> Option<String> {
        self.shared.error.lock().unwrap().take()
    }

    pub fn underruns(&self) -> u64 {
        self.shared.underruns.load(Ordering::Relaxed)
    }

    /// Render the forest as it is now to a WAV, on its own thread. Follow it with `export_state`.
    pub fn export(&self, path: String, minutes: f64) {
        let forest = self.forest();
        let sh = Arc::clone(&self.shared);
        {
            let mut e = sh.export.lock().unwrap();
            if e.running {
                return;
            }
            *e = ExportState {
                running: true,
                done_s: 0.0,
                total_s: minutes * 60.0,
                result: None,
            };
        }
        std::thread::spawn(move || {
            let opts = RenderFile {
                minutes,
                ..forest.render.clone()
            };
            let r = std::fs::File::create(&path)
                .map_err(|e| format!("{path}: {e}"))
                .and_then(|f| {
                    kanopy_core::render::render(
                        &forest,
                        &opts,
                        std::io::BufWriter::new(f),
                        |done| {
                            sh.export.lock().unwrap().done_s = done;
                        },
                    )
                    .map(|_| path.clone())
                });
            let mut e = sh.export.lock().unwrap();
            e.running = false;
            e.result = Some(r);
        });
    }

    pub fn export_state(&self) -> ExportState {
        self.shared.export.lock().unwrap().clone()
    }

    /// The forest's key, seed and how its world moves bodies.
    fn world_of(&self) -> Result<(Option<kanopy_core::scale::Key>, u64, f64, String), String> {
        let f = self.forest();
        let key = f.key.as_ref().map(|k| k.key()).transpose()?;
        let m = f.medium.medium()?;
        let ratio = if f.bodies_follow { m.body_ratio() } else { 1.0 };
        let name = match &f.medium {
            MediumFile::Preset(n) => n.clone(),
            _ => "custom".into(),
        };
        Ok((key, f.seed, ratio, name))
    }

    /// Audition one species: a fresh call or its signature, dry from 1 m, played on top of the
    /// forest (ducked while it plays). Returns the call for the page to draw.
    pub fn audition(&self, species: &Species, take: Take) -> Result<serde_json::Value, String> {
        use kanopy_core::audition;
        let (key, seed, ratio, _) = self.world_of()?;
        let n = self.shared.takes.fetch_add(1, Ordering::Relaxed);
        let m = match take {
            Take::Call => audition::call(species, key.as_ref(), seed, n),
            Take::Signature => audition::signature(species, key.as_ref(), seed, n),
        };
        let samples = audition::render(&m, ratio as f32, self.sample_rate as f32);
        self.send(Cmd::Sound(samples));
        let units: Vec<serde_json::Value> = m
            .units
            .iter()
            .map(|u| {
                serde_json::json!({ "at": u.at, "dur": u.dur, "note": u.note, "contour": u.contour,
                    "level": u.level, "shape": u.shape, "cutoff": u.cutoff * ratio })
            })
            .collect();
        Ok(serde_json::json!({
            "take": take,
            "duration": m.duration() + audition::TAIL,
            "units": units,
            "ahead": AHEAD,
        }))
    }

    /// What the forest's world does to this species' body (the Designer's below-hearing flag).
    pub fn check(&self, species: &Species) -> Result<serde_json::Value, String> {
        let (key, seed, ratio, medium) = self.world_of()?;
        let c = kanopy_core::audition::check(species, ratio, key.as_ref(), seed);
        Ok(serde_json::json!({
            "medium": medium,
            "ratio": c.ratio,
            "cutoff": [c.cutoff.0, c.cutoff.1],
            "loss_db": c.loss_db,
            "pinned": c.pinned,
        }))
    }
}

impl Drop for Live {
    fn drop(&mut self) {
        self.shared.stop.store(true, Ordering::Relaxed);
    }
}

type Prod = ringbuf::HeapProd<f32>;

#[cfg(feature = "audio")]
mod output {
    use super::*;
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

    /// Open the default output; returns its sample rate and the ring's producer side.
    pub fn open(shared: Arc<Shared>, ring_len: fn(u32) -> usize) -> Result<(u32, Prod), String> {
        let (tx, rx) = channel::<Result<(u32, Prod), String>>();
        std::thread::Builder::new()
            .name("kanopy-audio".into())
            .spawn(move || {
                let r = (|| {
                    let host = cpal::default_host();
                    let dev = host.default_output_device().ok_or("no sound output")?;
                    let cfg = dev.default_output_config().map_err(|e| e.to_string())?;
                    let sr = cfg.sample_rate().0;
                    let ch = cfg.channels() as usize;
                    let (prod, mut cons) = HeapRb::<f32>::new(ring_len(sr)).split();
                    let sh = Arc::clone(&shared);
                    let mut tmp = Vec::new();
                    let mut fill = move |out: &mut [f32]| {
                        let frames = out.len() / ch;
                        tmp.resize(frames * 2, 0.0);
                        let got = cons.pop_slice(&mut tmp) / 2;
                        if got < frames {
                            sh.underruns.fetch_add(1, Ordering::Relaxed);
                        }
                        for (f, o) in out.chunks_mut(ch).enumerate() {
                            let (l, r) = if f < got {
                                (tmp[2 * f], tmp[2 * f + 1])
                            } else {
                                (0.0, 0.0)
                            };
                            o[0] = l;
                            if ch > 1 {
                                o[1] = r;
                            }
                            for x in o.iter_mut().skip(2) {
                                *x = 0.0;
                            }
                        }
                    };
                    let err = |e| eprintln!("kanopy audio: {e}");
                    let stream = match cfg.sample_format() {
                        cpal::SampleFormat::F32 => dev.build_output_stream(
                            &cfg.into(),
                            move |d: &mut [f32], _| fill(d),
                            err,
                            None,
                        ),
                        cpal::SampleFormat::I16 => {
                            let mut buf = Vec::new();
                            dev.build_output_stream(
                                &cfg.into(),
                                move |d: &mut [i16], _| {
                                    buf.resize(d.len(), 0.0);
                                    fill(&mut buf);
                                    for (o, x) in d.iter_mut().zip(&buf) {
                                        *o = (x.clamp(-1.0, 1.0) * 32767.0) as i16;
                                    }
                                },
                                err,
                                None,
                            )
                        }
                        f => return Err(format!("unsupported sample format {f:?}")),
                    }
                    .map_err(|e| e.to_string())?;
                    stream.play().map_err(|e| e.to_string())?;
                    Ok((sr, prod, stream))
                })();
                match r {
                    Ok((sr, prod, stream)) => {
                        let _ = tx.send(Ok((sr, prod)));
                        while !shared.stop.load(Ordering::Relaxed) {
                            std::thread::sleep(Duration::from_millis(200));
                        }
                        drop(stream);
                    }
                    Err(e) => {
                        let _ = tx.send(Err(e));
                    }
                }
            })
            .map_err(|e| e.to_string())?;
        rx.recv().map_err(|e| e.to_string())?
    }
}

#[cfg(not(feature = "audio"))]
mod output {
    use super::*;
    pub fn open(_: Arc<Shared>, _: fn(u32) -> usize) -> Result<(u32, Prod), String> {
        Err("built without audio".into())
    }
}

struct Player {
    forest: ForestFile,
    scene: Scene,
    sr: u32,
    playing: bool,
    pos: Vec2,
    drag: Option<(Vec2, Instant)>,
    path: PathKind,
    speed: f64,
    follow: Option<usize>,
    follow_group: Option<usize>,
    wander_goal: Vec2,
    wander_since: f64,
    rng: Rng,
    volume: f32,
    version: u64,
    cpu: f64,
    peak: [f32; 2],
    /// An audition playing: its samples and how far it's got.
    audition: Option<(Vec<f32>, usize)>,
    /// The forest's gain under an audition (1 = not ducked).
    duck: f32,
    /// Safety limiter on what goes to the sound card (gain now).
    safety: f32,
}

fn world() -> Rect {
    Rect::new(0.0, WIDTH, BOTTOM, TOP)
}

impl Player {
    fn new(forest: ForestFile, sr: u32) -> Result<Self, String> {
        let scene = forest.scene()?.with_sound(sr as f32);
        let pos = scene.settings.listener;
        let mut p = Player {
            scene,
            sr,
            playing: false,
            pos,
            drag: None,
            path: PathKind::Still,
            speed: 1.5,
            follow: None,
            follow_group: None,
            wander_goal: pos,
            wander_since: 0.0,
            rng: Rng::new(forest.seed ^ 0x11FE),
            volume: 0.0,
            version: 0,
            cpu: 0.0,
            peak: [0.0; 2],
            audition: None,
            duck: 1.0,
            safety: 1.0,
            forest,
        };
        p.scene.set_listener(pos, Vec2::default());
        Ok(p)
    }

    /// Rebuild the scene from `self.forest`, keeping the listener, box and path.
    fn rebuild(&mut self) -> Result<(), String> {
        self.forest.listen = self.listen_file();
        let scene = self.forest.scene()?.with_sound(self.sr as f32);
        self.scene = scene;
        self.scene.set_listener(self.pos, Vec2::default());
        self.follow_group = None;
        self.version += 1;
        Ok(())
    }

    fn radius(&self) -> f64 {
        match self.forest.listen {
            ListenFile::Listener { radius, .. } => radius,
            _ => 150.0,
        }
    }

    fn listen_file(&self) -> ListenFile {
        match self.scene.settings.hear {
            Hear::Box(r) => ListenFile::Box {
                x0: r.x0,
                x1: r.x1,
                y0: r.y0,
                y1: r.y1,
            },
            Hear::Listener => ListenFile::Listener {
                x: self.pos.x,
                y: self.pos.y,
                radius: self.radius(),
            },
        }
    }

    fn apply(&mut self, cmd: Cmd) -> Result<bool, String> {
        // returns whether the forest has to be rebuilt
        let f = &mut self.forest;
        match cmd {
            Cmd::Play { on } => self.playing = on,
            Cmd::Listener { x, y } => {
                self.drag = Some((world().clamp(Vec2::new(x, y)), Instant::now()))
            }
            Cmd::Hear { rect } => {
                let hear = match rect {
                    Some([x0, x1, y0, y1]) => {
                        Hear::Box(Rect::new(x0.min(x1), x0.max(x1), y0.min(y1), y0.max(y1)))
                    }
                    None => Hear::Listener,
                };
                let r = self.radius();
                self.scene.set_hear(hear, r);
            }
            Cmd::Path {
                kind,
                speed,
                species,
            } => {
                self.path = kind;
                self.speed = speed.clamp(0.05, 30.0);
                self.follow = species;
                self.follow_group = None;
                self.wander_since = f64::NEG_INFINITY;
            }
            Cmd::Radius { radius } => {
                if let ListenFile::Listener { x, y, .. } = f.listen {
                    f.listen = ListenFile::Listener { x, y, radius };
                }
                if let Hear::Listener = self.scene.settings.hear {
                    self.scene.set_hear(Hear::Listener, radius);
                }
            }
            Cmd::Seed { seed } => {
                f.seed = seed;
                return Ok(true);
            }
            Cmd::Density { density } => {
                f.density = density.clamp(0.0, 1.0);
                return Ok(true);
            }
            Cmd::Variation { variation } => {
                f.tree_variation = variation.clamp(0.0, 1.0);
                return Ok(true);
            }
            Cmd::Medium { medium } => {
                f.medium = MediumFile::Preset(medium);
                return Ok(true);
            }
            Cmd::UnderwaterStereo { on } => {
                f.underwater_stereo = on;
                return Ok(true);
            }
            Cmd::Compensation { amount } => {
                f.medium_compensation = amount.clamp(0.0, 1.0);
                return Ok(true);
            }
            Cmd::Key { key } => {
                let k = key.as_ref().map(|k| k.key()).transpose()?;
                f.key = key;
                self.scene.set_key(k);
            }
            Cmd::Reverb { amount } => {
                f.reverb = amount;
                self.scene.set_reverb(amount);
            }
            Cmd::Ear { ear } => {
                f.ear = ear;
                self.scene.set_ear(ear);
            }
            Cmd::Volume { db } => self.volume = db.clamp(-60.0, 12.0),
            Cmd::Sound(samples) => self.audition = Some((samples, 0)),
            Cmd::Hush => self.audition = None,
            Cmd::Name { name } => f.name = name,
            Cmd::Species { species } => {
                if species.is_empty() {
                    return Err("a forest needs at least one species".into());
                }
                let n: usize = species.iter().map(|s| s.habitat.individuals).sum();
                if n > MAX_INDIVIDUALS {
                    return Err(format!(
                        "{n} individuals is more than this build plays ({MAX_INDIVIDUALS})"
                    ));
                }
                let trial = ForestFile {
                    species: species.clone(),
                    ..f.clone()
                };
                trial.scene()?;
                f.species = species;
                return Ok(true);
            }
            Cmd::Load { forest } => {
                forest.scene()?;
                *f = *forest;
                self.pos = match f.listen {
                    ListenFile::Listener { x, y, .. } => Vec2::new(x, y),
                    ListenFile::Box { x0, x1, y0, y1 } => {
                        Vec2::new(0.5 * (x0 + x1), 0.5 * (y0 + y1))
                    }
                };
                self.drag = None;
                let scene = f.scene()?.with_sound(self.sr as f32);
                self.scene = scene;
                self.scene.set_listener(self.pos, Vec2::default());
                self.version += 1;
                self.follow_group = None;
            }
        }
        Ok(false)
    }

    /// Where the listener heads this step, and how fast it may go.
    fn goal(&mut self) -> (Vec2, f64) {
        if let Some((to, at)) = self.drag {
            if at.elapsed().as_secs_f64() < 0.4 || self.pos.dist(to) > 0.05 {
                return (to, DRAG_SPEED);
            }
            self.drag = None;
            self.wander_goal = self.pos;
        }
        let t = self.scene.time();
        match self.path {
            PathKind::Still => (self.pos, 0.0),
            PathKind::Wander => {
                if self.pos.dist(self.wander_goal) < 1.0 || t - self.wander_since > 40.0 {
                    let w = world();
                    // mostly above ground, sometimes under it
                    let y = if self.rng.f64() < 0.1 {
                        self.rng.range(BOTTOM + 0.3, -0.5)
                    } else {
                        self.rng.range(0.5, TOP - 2.0)
                    };
                    let x =
                        (self.pos.x + self.rng.range(-60.0, 60.0)).clamp(w.x0 + 2.0, w.x1 - 2.0);
                    self.wander_goal = Vec2::new(x, y);
                    self.wander_since = t;
                }
                (self.wander_goal, self.speed)
            }
            PathKind::Follow => {
                let Some(sp) = self.follow else {
                    return (self.pos, 0.0);
                };
                let life = &self.scene.life;
                let centre = |g: usize| {
                    let gr = &life.groups[g];
                    let mut c = Vec2::default();
                    for a in &life.animals[gr.first..gr.first + gr.len] {
                        c = c.add_scaled(a.pos, 1.0 / gr.len as f64);
                    }
                    c
                };
                let valid = self
                    .follow_group
                    .filter(|&g| g < life.groups.len() && life.groups[g].species as usize == sp);
                let g = match valid {
                    Some(g) => g,
                    None => {
                        let best = (0..life.groups.len())
                            .filter(|&g| life.groups[g].species as usize == sp)
                            .min_by(|&a, &b| {
                                self.pos
                                    .dist(centre(a))
                                    .total_cmp(&self.pos.dist(centre(b)))
                            });
                        match best {
                            Some(g) => g,
                            None => return (self.pos, 0.0),
                        }
                    }
                };
                self.follow_group = Some(g);
                // stay a couple of metres off the group, not inside it
                let c = centre(g);
                let mover = life.species[sp].habitat.movement;
                let off = if mover == Movement::Burrower {
                    Vec2::new(-2.0, 0.0)
                } else {
                    Vec2::new(-2.0, -1.5)
                };
                (world().clamp(c.add_scaled(off, 1.0)), self.speed)
            }
        }
    }

    fn move_listener(&mut self) {
        let (goal, max) = self.goal();
        let (dx, dy) = (goal.x - self.pos.x, goal.y - self.pos.y);
        let d = (dx * dx + dy * dy).sqrt();
        let step = (max * DT).min(d);
        let vel = if d > 1e-9 && step > 0.0 {
            Vec2::new(dx / d * step / DT, dy / d * step / DT)
        } else {
            Vec2::default()
        };
        self.pos = world().clamp(self.pos.add_scaled(vel, DT));
        self.scene.set_listener(self.pos, vel);
    }

    /// Duck the forest in `l`/`r` under the audition, add the audition on top, apply the volume
    /// and a safety limit at −1 dBFS. Returns the peaks.
    fn finish(&mut self, l: &mut [f32], r: &mut [f32], out: &mut [f32]) -> [f32; 2] {
        let sr = self.sr as f32;
        let (down, up) = (1.0 / (DUCK_DOWN * sr), 1.0 / (DUCK_UP * sr));
        let floor = db_to_amp(DUCK_DB);
        let vol = db_to_amp(self.volume);
        let thr = db_to_amp(-1.0);
        let release = (-1.0 / (0.2 * sr)).exp();
        let centre = std::f32::consts::FRAC_1_SQRT_2;
        let mut pk = [0f32; 2];
        for i in 0..l.len() {
            let (a, more) = match self.audition.as_mut() {
                Some((x, at)) if *at < x.len() => {
                    *at += 1;
                    (x[*at - 1], true)
                }
                _ => (0.0, false),
            };
            if !more && self.audition.is_some() {
                self.audition = None;
            }
            // linear glide in dB-ish terms is plenty here
            let target = if more { floor } else { 1.0 };
            self.duck += if target < self.duck {
                -(self.duck - target).min((1.0 - floor) * down)
            } else {
                (target - self.duck).min((1.0 - floor) * up)
            };
            let mut x = [
                (l[i] * self.duck + a * centre) * vol,
                (r[i] * self.duck + a * centre) * vol,
            ];
            let peak = x[0].abs().max(x[1].abs());
            self.safety = 1.0 - (1.0 - self.safety) * release;
            if peak * self.safety > thr {
                self.safety = thr / peak;
            }
            for (c, v) in x.iter_mut().enumerate() {
                *v *= self.safety;
                pk[c] = pk[c].max(v.abs());
            }
            out[2 * i] = x[0];
            out[2 * i + 1] = x[1];
        }
        pk
    }

    fn info(&self) -> String {
        let f = &self.scene.life.forest;
        let trees: Vec<[f64; 4]> = f
            .trees
            .iter()
            .map(|t| [t.x, t.height, t.crown_bottom, t.crown_half_width])
            .collect();
        let mut counts = vec![0usize; self.scene.life.species.len()];
        for a in &self.scene.life.animals {
            counts[a.species as usize] += 1;
        }
        let species: Vec<serde_json::Value> = self
            .scene
            .life
            .species
            .iter()
            .zip(&counts)
            .map(|(s, n)| serde_json::json!({ "name": s.name, "movement": s.habitat.movement, "count": n }))
            .collect();
        serde_json::json!({
            "version": self.version,
            "forest": self.forest,
            "trees": trees,
            "shrubs": f.shrub_fullness,
            "liquid": self.scene.settings.medium.liquid,
            "sample_rate": self.sr,
            "species": species,
            "path": { "kind": self.path, "speed": self.speed, "species": self.follow },
            "volume": self.volume,
            "default_species": kanopy_core::species::Species::new("New species", Default::default()),
            "max_individuals": MAX_INDIVIDUALS,
            "speeds": {
                "walker": Movement::Walker.speed(),
                "climber": Movement::Climber.speed(),
                "flyer": Movement::Flyer.speed(),
                "burrower": Movement::Burrower.speed(),
            },
        })
        .to_string()
    }
}

/// A frame: what the screen draws.
///
/// Header (68 bytes, little-endian): magic "KNF1"; time f64 (s); version u32 (changes when the
/// forest is rebuilt); playing u8, hear u8 (0 listener, 1 box), auditioning u8, 1 byte pad; calling u32;
/// active u32; cpu f32 (share of real time); peak L, R f32 (dBFS); listener x, y f32; box x0,
/// x1, y0, y1 f32; Ear gain f32 (dB). Then per animal (12 bytes): x f32, y f32, species u16,
/// flags u8 (1 active, 2 calling, 4 flying), pad.
fn encode_frame(p: &Player) -> Vec<u8> {
    let s = &p.scene;
    let n = s.life.animals.len();
    let mut b = Vec::with_capacity(FRAME_HEADER + n * FRAME_ANIMAL);
    b.extend_from_slice(FRAME_MAGIC);
    b.extend_from_slice(&s.time().to_le_bytes());
    b.extend_from_slice(&(p.version as u32).to_le_bytes());
    let boxed = matches!(s.settings.hear, Hear::Box(_));
    b.extend_from_slice(&[p.playing as u8, boxed as u8, p.audition.is_some() as u8, 0]);
    let mut calling = 0u32;
    let mut active = 0u32;
    for (i, a) in s.life.animals.iter().enumerate() {
        calling += s.calling_now(i) as u32;
        active += a.active as u32;
    }
    b.extend_from_slice(&calling.to_le_bytes());
    b.extend_from_slice(&active.to_le_bytes());
    let master = s.mixer.as_ref().and_then(|m| m.master.as_ref());
    let r = match s.settings.hear {
        Hear::Box(r) => r,
        Hear::Listener => Rect::new(0.0, 0.0, 0.0, 0.0),
    };
    for v in [
        p.cpu as f32,
        kanopy_core::amp_to_db(p.peak[0]),
        kanopy_core::amp_to_db(p.peak[1]),
        p.pos.x as f32,
        p.pos.y as f32,
        r.x0 as f32,
        r.x1 as f32,
        r.y0 as f32,
        r.y1 as f32,
        master.map_or(0.0, |m| m.gain_db as f32),
    ] {
        b.extend_from_slice(&v.to_le_bytes());
    }
    debug_assert_eq!(b.len(), FRAME_HEADER);
    for (i, a) in s.life.animals.iter().enumerate() {
        b.extend_from_slice(&(a.pos.x as f32).to_le_bytes());
        b.extend_from_slice(&(a.pos.y as f32).to_le_bytes());
        b.extend_from_slice(&(a.species as u16).to_le_bytes());
        let flags = a.active as u8 | (s.calling_now(i) as u8) << 1 | (a.flying() as u8) << 2;
        b.extend_from_slice(&[flags, 0]);
    }
    b
}

fn render_thread(
    forest: ForestFile,
    sr: u32,
    rx: Receiver<Cmd>,
    mut prod: Option<Prod>,
    sh: Arc<Shared>,
) {
    let mut p = match Player::new(forest, sr) {
        Ok(p) => p,
        Err(e) => {
            *sh.error.lock().unwrap() = Some(e);
            return;
        }
    };
    *sh.info.lock().unwrap() = p.info();
    *sh.frame.lock().unwrap() = encode_frame(&p);
    let n = p.scene.step_samples();
    let (mut l, mut r) = (vec![0f32; n], vec![0f32; n]);
    let mut inter = vec![0f32; 2 * n];
    // frames waiting for their sound to play: (end of their audio in samples sent, frame)
    let mut pending: VecDeque<(u64, Vec<u8>)> = VecDeque::new();
    let mut sent: u64 = 0; // samples (per channel) handed to the sound card
    let clock = Instant::now();
    let mut paused_at = 0.0f64; // silent mode: sound time when last paused
    let mut silent_start = clock;
    let mut was_playing = false;

    while !sh.stop.load(Ordering::Relaxed) {
        // commands
        let mut rebuild = false;
        let mut changed = false;
        while let Ok(cmd) = rx.try_recv() {
            changed = true;
            match p.apply(cmd) {
                Ok(rb) => rebuild |= rb,
                Err(e) => *sh.error.lock().unwrap() = Some(e),
            }
        }
        if changed && !rebuild {
            // settings changed live (or a new forest was loaded): the page and Save see them
            *sh.info.lock().unwrap() = p.info();
            let mut f = sh.forest.lock().unwrap();
            *f = p.forest.clone();
            f.listen = p.listen_file();
        }
        if rebuild {
            // fade the old forest out over one step, then swap
            if p.playing {
                if let Some(pr) = prod.as_mut() {
                    l.fill(0.0);
                    r.fill(0.0);
                    p.scene.step_audio(&mut l, &mut r);
                    for i in 0..n {
                        let g = 1.0 - i as f32 / n as f32;
                        inter[2 * i] = l[i] * g * db_to_amp(p.volume);
                        inter[2 * i + 1] = r[i] * g * db_to_amp(p.volume);
                    }
                    if pr.vacant_len() >= 2 * n {
                        pr.push_slice(&inter);
                        sent += n as u64;
                    }
                }
            }
            if let Err(e) = p.rebuild() {
                *sh.error.lock().unwrap() = Some(e);
            }
            pending.clear();
            *sh.info.lock().unwrap() = p.info();
            *sh.forest.lock().unwrap() = p.forest.clone();
            *sh.frame.lock().unwrap() = encode_frame(&p);
        }
        // sounding: the forest playing, or an audition on its own
        let on = p.playing || p.audition.is_some();
        if on != was_playing {
            was_playing = on;
            silent_start = Instant::now();
            paused_at = sent as f64 / sr as f64;
            *sh.info.lock().unwrap() = p.info();
        }
        if !on {
            p.peak = [0.0; 2];
            p.duck = 1.0;
            *sh.frame.lock().unwrap() = encode_frame(&p);
            std::thread::sleep(Duration::from_millis(15));
            continue;
        }
        // where the sound card is: samples played so far
        let played = match prod.as_ref() {
            Some(pr) => sent.saturating_sub((pr.occupied_len() / 2) as u64),
            None => ((paused_at + silent_start.elapsed().as_secs_f64()) * sr as f64) as u64,
        };
        let room = match prod.as_ref() {
            Some(pr) => pr.vacant_len() >= 2 * n && (sent - played) < (AHEAD * sr as f64) as u64,
            None => (sent as i64 - played as i64) < (AHEAD * sr as f64) as i64,
        };
        if room {
            let t0 = Instant::now();
            l.fill(0.0);
            r.fill(0.0);
            if p.playing {
                p.move_listener();
                p.scene.step_audio(&mut l, &mut r);
            }
            let pk = p.finish(&mut l, &mut r, &mut inter);
            if let Some(pr) = prod.as_mut() {
                pr.push_slice(&inter);
            }
            sent += n as u64;
            let spent = t0.elapsed().as_secs_f64() / DT;
            p.cpu += (spent - p.cpu) * 0.1;
            p.peak = pk;
            pending.push_back((sent, encode_frame(&p)));
            continue;
        }
        // publish the newest frame whose sound has started playing
        let mut latest = None;
        while pending
            .front()
            .is_some_and(|(end, _)| *end <= played + n as u64)
        {
            latest = pending.pop_front().map(|x| x.1);
        }
        if let Some(fr) = latest {
            *sh.frame.lock().unwrap() = fr;
            // keep Save and Export up to date with where the listener is
            if let Ok(mut f) = sh.forest.try_lock() {
                *f = p.forest.clone();
                f.listen = p.listen_file();
            }
        }
        std::thread::sleep(Duration::from_millis(3));
    }
}

/// Decode a frame's header (for tests and tools): (time, version, playing, calling, active).
pub fn frame_header(b: &[u8]) -> Option<(f64, u32, bool, u32, u32)> {
    if b.len() < FRAME_HEADER || &b[0..4] != FRAME_MAGIC {
        return None;
    }
    let f64_at = |i: usize| f64::from_le_bytes(b[i..i + 8].try_into().unwrap());
    let u32_at = |i: usize| u32::from_le_bytes(b[i..i + 4].try_into().unwrap());
    Some((f64_at(4), u32_at(12), b[16] != 0, u32_at(20), u32_at(24)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_forest_ducks_under_an_audition_and_comes_back() {
        let mut p = Player::new(ForestFile::demo(), 48_000).unwrap();
        let n = 2400;
        let mut out = vec![0f32; 2 * n];
        let forest = 0.1f32;
        let run = |p: &mut Player, out: &mut Vec<f32>| {
            let (mut l, mut r) = (vec![forest; n], vec![forest; n]);
            p.finish(&mut l, &mut r, out)
        };
        run(&mut p, &mut out);
        assert!(
            (out[2 * n - 2] - forest).abs() < 1e-6,
            "untouched without an audition"
        );
        // half a second of silence as the audition: the forest goes down to −15 dB within 40 ms
        p.audition = Some((vec![0.0; 24_000], 0));
        run(&mut p, &mut out);
        let ducked = out[2 * 2000];
        assert!(
            (kanopy_core::amp_to_db(ducked / forest) - DUCK_DB).abs() < 0.1,
            "{ducked}"
        );
        for _ in 0..10 {
            run(&mut p, &mut out);
        }
        assert!(p.audition.is_none(), "it ends");
        // and back up over half a second
        for _ in 0..12 {
            run(&mut p, &mut out);
        }
        assert!((out[2 * n - 2] - forest).abs() < 1e-4);
        // a loud audition on top never passes −1 dBFS
        p.audition = Some((vec![3.0; 4800], 0));
        let pk = run(&mut p, &mut out);
        assert!(pk[0] <= db_to_amp(-1.0) + 1e-6 && pk[1] <= db_to_amp(-1.0) + 1e-6);
    }
}
