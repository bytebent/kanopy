//! The scene: the forest living, calling and being heard (design §5.5–§7, §10).
//!
//! Each step (DT) the scene:
//! 1. marks who is in the active zone (only they call);
//! 2. handles every event due in the step, at its exact time: an individual whose rest is over
//!    (it calls with Talkative's odds, or answers if it chose to), and units starting (they go to
//!    the mixer with the animal's position and velocity at that moment);
//! 3. moves the forest one step.
//!
//! When a call starts it travels to everyone who could react, using only call events, never
//! audio. A hearer reacts if the call arrives above its species' Hearing level (spreading and air
//! absorption at the call's pitch). Arrival is d / c later; reactions are timed from it:
//! - **memory**: a mimic copies one of the last few calls that reached it (any species, its own
//!   included). Worked out when it decides to copy, from the recent calls, with the same
//!   reach and arrival times, rather than on every hearing;
//! - **group mates**: the note it heard, for Harmonise;
//! - **same species**: a call gets at most one answer: with the caller species' Response odds,
//!   one conspecific that heard it (and has rested for its shortest Rest) answers, louder hearers
//!   likelier. Exchanges go back and forth and die out on their own. Everyone else of the species
//!   who heard it lets Chorus move their next call: sooner (synchronise) or out of the way
//!   (alternate).
//!
//! What a call is: a copy of something remembered (Mimicry chance), else the group's signature
//! (Signature amount, changed by Variation), else a fresh call. Fresh and signature calls may
//! then be moved to a consonance with a group mate (Harmonise). In key, notes snap to the scale.
//!
//! The calling cap limits how many call at once; past three quarters of it, only those nearer
//! the listener may start (none of that in a box: there, plain cap).

use crate::calls::{self, Motif};
use crate::life::{Life, Zone, DT};
use crate::mixer::{Listener, Mixer};
use crate::propagation::Medium;
use crate::rng::Rng;
use crate::scale::{Key, Scale};
use crate::voice::{note_to_hz, UnitSpec};
use crate::wavetable::Wavetables;
use crate::world::{Rect, BOTTOM, TOP, WIDTH};
use crate::Vec2;
use std::cmp::{Ordering, Reverse};
use std::collections::BinaryHeap;
use std::rc::Rc;
use std::sync::Arc;

/// Calls each individual remembers.
pub const MEMORY: usize = 4;
/// Delay before an answer, after the call it answers has arrived in full, seconds.
pub const ANSWER_DELAY: (f64, f64) = (0.2, 1.0);
/// Calls don't travel further than this, metres (well past the 200 m slice).
const MAX_REACH: f64 = 400.0;
/// Recent calls kept for mimics to draw on.
const RECENT: usize = 16384;
/// A mimic looks this far back for calls it heard, seconds.
const MEMORY_SPAN: f64 = 60.0;

/// How far a call of this level and pitch carries before it drops below `hearing`: spreading
/// (−6 dB per doubling from 1 m) and air absorption (absorb dB per metre).
pub fn reach(level: f64, hearing: f64, absorb: f64) -> f64 {
    let lost = |r: f64| 20.0 * r.max(1.0).log10() + absorb * r;
    if lost(1.0) > level - hearing {
        return 0.0;
    }
    let (mut lo, mut hi) = (1.0, MAX_REACH);
    if lost(hi) <= level - hearing {
        return MAX_REACH;
    }
    for _ in 0..40 {
        let mid = 0.5 * (lo + hi);
        if lost(mid) <= level - hearing {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    lo
}

#[derive(Clone, Debug)]
struct Recent {
    who: u32,
    species: u32,
    start: f64,
    pos: Vec2,
    level: f64,
    absorb: f64,
    motif: Rc<Motif>,
}

#[derive(Clone, Copy, Debug)]
pub struct SceneSettings {
    /// In key, or None for Free.
    pub key: Option<Key>,
    pub listener: Vec2,
    /// Who may call.
    pub zone: Zone,
    /// The most that may call at once.
    pub cap: usize,
    pub medium: Medium,
}

impl Default for SceneSettings {
    fn default() -> Self {
        let listener = Vec2::new(100.0, 1.5);
        Self {
            key: Some(Key::new(2, Scale::Minor)),
            listener,
            zone: Zone::Around {
                pos: listener,
                radius: 150.0,
            },
            cap: 600,
            medium: Medium::earth(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    Fresh,
    Signature,
    /// A copy of a call by this species.
    Mimic {
        of: u32,
    },
}

#[derive(Clone, Debug)]
pub struct Call {
    pub who: u32,
    pub species: u32,
    pub group: u32,
    pub start: f64,
    pub pos: Vec2,
    pub motif: Rc<Motif>,
    pub source: Source,
    pub answer: bool,
    /// For an answer: the caller and start time of the call it answers.
    pub reply_to: Option<(u32, f64)>,
    pub harmonised: bool,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Stats {
    pub calls: u64,
    pub answers: u64,
    pub mimics: u64,
    pub signatures: u64,
    pub harmonised: u64,
    pub skipped_cap: u64,
    pub skipped_inactive: u64,
    pub skipped_quiet: u64,
    /// Calls arriving above someone's Hearing.
    pub heard: u64,
    pub max_calling: usize,
}

#[derive(Clone, Debug)]
struct Mind {
    gen: u32,
    due: f64,
    answer: bool,
    reply_to: Option<(u32, f64)>,
    calling_until: f64,
    last_end: f64,
    mate_note: f64,
    mate_until: f64,
    /// Chorus already moved this rest's call.
    nudged: bool,
    rng: Rng,
}

#[derive(Debug)]
enum Ev {
    Due { who: u32, gen: u32 },
    Unit { who: u32, spec: UnitSpec },
}

#[derive(Debug)]
struct Item {
    at: f64,
    seq: u64,
    ev: Ev,
}
impl PartialEq for Item {
    fn eq(&self, o: &Self) -> bool {
        self.cmp(o) == Ordering::Equal
    }
}
impl Eq for Item {}
impl PartialOrd for Item {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}
impl Ord for Item {
    // reversed: the heap pops the earliest
    fn cmp(&self, o: &Self) -> Ordering {
        o.at.total_cmp(&self.at).then(o.seq.cmp(&self.seq))
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct T(f64);
impl Eq for T {}
impl PartialOrd for T {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}
impl Ord for T {
    fn cmp(&self, o: &Self) -> Ordering {
        self.0.total_cmp(&o.0)
    }
}

/// Active individuals of one species, bucketed by 10 m cells, for finding who's in reach.
struct Grid {
    cells: Vec<Vec<u32>>,
}
const CELL: f64 = 10.0;
const NX: usize = (WIDTH / CELL) as usize + 1;
const NY: usize = ((TOP - BOTTOM) / CELL) as usize + 1;

impl Grid {
    fn new() -> Self {
        Self {
            cells: vec![Vec::new(); NX * NY],
        }
    }
    fn cell(p: Vec2) -> (usize, usize) {
        (
            ((p.x / CELL).floor().max(0.0) as usize).min(NX - 1),
            (((p.y - BOTTOM) / CELL).floor().max(0.0) as usize).min(NY - 1),
        )
    }
    fn clear(&mut self) {
        for c in &mut self.cells {
            c.clear();
        }
    }
    fn insert(&mut self, i: u32, p: Vec2) {
        let (x, y) = Self::cell(p);
        self.cells[y * NX + x].push(i);
    }
    fn near(&self, p: Vec2, r: f64, out: &mut Vec<u32>) {
        let (x0, y0) = Self::cell(Vec2::new(p.x - r, p.y - r));
        let (x1, y1) = Self::cell(Vec2::new(p.x + r, p.y + r));
        for y in y0..=y1 {
            for x in x0..=x1 {
                out.extend_from_slice(&self.cells[y * NX + x]);
            }
        }
    }
}

pub struct Scene {
    pub life: Life,
    pub settings: SceneSettings,
    pub mixer: Option<Mixer>,
    /// Every call, if `keep_score`.
    pub score: Vec<Call>,
    pub keep_score: bool,
    pub stats: Stats,
    minds: Vec<Mind>,
    signatures: Vec<Rc<Motif>>,
    heap: BinaryHeap<Item>,
    seq: u64,
    ends: BinaryHeap<Reverse<T>>,
    grids: Vec<Grid>,
    /// Per species: does it react to its own species' calls as they arrive.
    listens: Vec<bool>,
    recent: Vec<Recent>,
    recent_next: usize,
    buf: Vec<u32>,
}

impl Scene {
    /// A scene without sound (for scores and tests).
    pub fn new(life: Life, settings: SceneSettings) -> Self {
        let seed = life.forest.settings.seed;
        let key = settings.key;
        let mut minds = Vec::with_capacity(life.animals.len());
        let mut heap = BinaryHeap::new();
        let mut seq = 0;
        for (i, a) in life.animals.iter().enumerate() {
            let sp = &life.species[a.species as usize];
            let mut rng = Rng::stream(seed, 0xB000_0000_0000 + i as u64);
            let due = rng.range(0.0, sp.calls.rest.1.max(sp.calls.rest.0));
            heap.push(Item {
                at: due,
                seq,
                ev: Ev::Due {
                    who: i as u32,
                    gen: 0,
                },
            });
            seq += 1;
            minds.push(Mind {
                gen: 0,
                due,
                answer: false,
                reply_to: None,
                calling_until: f64::NEG_INFINITY,
                last_end: f64::NEG_INFINITY,
                mate_note: 0.0,
                mate_until: f64::NEG_INFINITY,
                nudged: false,
                rng,
            });
        }
        let signatures = life
            .groups
            .iter()
            .enumerate()
            .map(|(gi, g)| {
                let mut rng = Rng::stream(seed, 0xC000_0000_0000 + gi as u64);
                Rc::new(calls::fresh(
                    &life.species[g.species as usize],
                    g.species,
                    key.as_ref(),
                    &mut rng,
                ))
            })
            .collect();
        let n = life.species.len();
        let listens = (0..n)
            .map(|h| {
                let b = &life.species[h].behaviour;
                b.response > 0.0 || b.chorus != 0.0 || b.harmonise > 0.0
            })
            .collect();
        let grids = (0..n).map(|_| Grid::new()).collect();
        Self {
            life,
            settings,
            mixer: None,
            score: Vec::new(),
            keep_score: false,
            stats: Stats::default(),
            minds,
            signatures,
            heap,
            seq,
            ends: BinaryHeap::new(),
            grids,
            listens,
            recent: Vec::new(),
            recent_next: 0,
            buf: Vec::new(),
        }
    }

    /// The same with sound at this sample rate.
    pub fn with_sound(mut self, sample_rate: f32) -> Self {
        let wt = Arc::new(Wavetables::new(sample_rate));
        self.mixer = Some(Mixer::new(
            wt,
            self.settings.medium,
            Listener::still(self.settings.listener),
        ));
        self
    }

    pub fn time(&self) -> f64 {
        self.life.time
    }

    /// The group's signature call.
    pub fn signature(&self, group: usize) -> &Motif {
        &self.signatures[group]
    }

    /// One step without sound.
    pub fn step(&mut self) {
        self.think();
        self.life.step();
    }

    /// One step with sound: `left`/`right` get DT of audio added.
    pub fn step_audio(&mut self, left: &mut [f32], right: &mut [f32]) {
        self.think();
        if let Some(m) = self.mixer.as_mut() {
            m.render(left, right);
        }
        self.life.step();
    }

    /// Samples per step at the mixer's rate.
    pub fn step_samples(&self) -> usize {
        self.mixer
            .as_ref()
            .map_or(0, |m| (DT * m.sample_rate() as f64).round() as usize)
    }

    pub fn run(&mut self, seconds: f64) {
        for _ in 0..(seconds / DT).round() as usize {
            self.step();
        }
    }

    /// How many are calling at time t.
    pub fn calling(&mut self, t: f64) -> usize {
        while let Some(Reverse(T(e))) = self.ends.peek() {
            if *e <= t {
                self.ends.pop();
            } else {
                break;
            }
        }
        self.ends.len()
    }

    fn push(&mut self, at: f64, ev: Ev) {
        self.heap.push(Item {
            at,
            seq: self.seq,
            ev,
        });
        self.seq += 1;
    }

    fn rest(&mut self, who: usize, from: f64) {
        let sp = &self.life.species[self.life.animals[who].species as usize];
        let r = sp.calls.rest;
        let m = &mut self.minds[who];
        m.gen += 1;
        m.due = from + m.rng.range(r.0.min(r.1), r.0.max(r.1));
        m.answer = false;
        m.nudged = false;
        let (due, gen) = (m.due, m.gen);
        self.push(
            due,
            Ev::Due {
                who: who as u32,
                gen,
            },
        );
    }

    fn think(&mut self) {
        let t0 = self.life.time;
        let t1 = t0 + DT;
        self.life.mark_active(&self.settings.zone);
        for g in &mut self.grids {
            g.clear();
        }
        for (i, a) in self.life.animals.iter().enumerate() {
            if a.active {
                self.grids[a.species as usize].insert(i as u32, a.pos);
            }
        }
        while self.heap.peek().is_some_and(|it| it.at < t1) {
            let it = self.heap.pop().unwrap();
            match it.ev {
                Ev::Due { who, gen } => self.due(who as usize, gen, it.at),
                Ev::Unit { who, spec } => {
                    if let Some(m) = self.mixer.as_mut() {
                        let a = &self.life.animals[who as usize];
                        let p = a.pos.add_scaled(a.vel, it.at - t0);
                        m.add(spec, it.at, p.add_scaled(a.vel, -it.at), a.vel);
                    }
                }
            }
        }
    }

    fn due(&mut self, who: usize, gen: u32, t: f64) {
        if self.minds[who].gen != gen {
            return; // replaced by a later decision
        }
        let a = &self.life.animals[who];
        let sp = &self.life.species[a.species as usize];
        if !a.active {
            self.stats.skipped_inactive += 1;
            return self.rest(who, t);
        }
        let answer = self.minds[who].answer;
        if !answer && self.minds[who].rng.f64() >= sp.behaviour.talkative {
            self.stats.skipped_quiet += 1;
            return self.rest(who, t);
        }
        if !self.allowed(who, t) {
            self.stats.skipped_cap += 1;
            return self.rest(who, t);
        }
        self.start_call(who, t, answer);
    }

    fn allowed(&mut self, who: usize, t: f64) -> bool {
        let cap = self.settings.cap;
        let n = self.calling(t);
        if n >= cap {
            return false;
        }
        if let Zone::Around { pos, radius } = self.settings.zone {
            let soft = (cap as f64 * 0.75).floor() as usize;
            if n >= soft && cap > soft {
                let room = (cap - n) as f64 / (cap - soft) as f64;
                return self.life.animals[who].pos.dist(pos) <= radius * room;
            }
        }
        true
    }

    fn start_call(&mut self, who: usize, t: f64, answer: bool) {
        let key = self.settings.key;
        let (si, gi, pos) = {
            let a = &self.life.animals[who];
            (a.species as usize, a.group as usize, a.pos)
        };
        let species = &self.life.species;
        let sp = &species[si];
        let m = &mut self.minds[who];
        let mimic = sp.behaviour.mimicry_chance > 0.0 && m.rng.f64() < sp.behaviour.mimicry_chance;
        let heard = if mimic {
            heard_recently(
                &self.recent,
                self.recent_next,
                who as u32,
                pos,
                sp.behaviour.hearing,
                self.settings.medium.speed,
                t,
            )
        } else {
            Vec::new()
        };
        let (mut motif, source) = if !heard.is_empty() {
            let r = &self.recent[heard[m.rng.int(0, heard.len() as i64 - 1) as usize]];
            let from = &species[r.motif.species as usize];
            (
                calls::copy(&r.motif, from, sp, si as u32, key.as_ref(), &mut m.rng),
                Source::Mimic { of: r.species },
            )
        } else if m.rng.f64() < sp.calls.signature {
            let v = calls::vary(
                &self.signatures[gi],
                sp,
                sp.calls.variation,
                sp.calls.units,
                key.as_ref(),
                &mut m.rng,
            );
            (v, Source::Signature)
        } else {
            (
                calls::fresh(sp, si as u32, key.as_ref(), &mut m.rng),
                Source::Fresh,
            )
        };
        let mut harmonised = false;
        if !matches!(source, Source::Mimic { .. })
            && sp.behaviour.harmonise > 0.0
            && m.mate_until > t
            && m.rng.f64() < sp.behaviour.harmonise
        {
            harmonised = calls::harmonise(&mut motif, m.mate_note, sp, key.as_ref(), &mut m.rng);
        }
        let motif = Rc::new(motif);
        let dur = motif.duration();
        m.calling_until = t + dur;
        m.last_end = t + dur;
        self.ends.push(Reverse(T(t + dur)));
        for (at, spec) in motif.specs() {
            self.push(
                t + at,
                Ev::Unit {
                    who: who as u32,
                    spec,
                },
            );
        }
        self.rest(who, t + dur);

        let s = &mut self.stats;
        s.calls += 1;
        s.answers += answer as u64;
        s.harmonised += harmonised as u64;
        match source {
            Source::Mimic { .. } => s.mimics += 1,
            Source::Signature => s.signatures += 1,
            Source::Fresh => {}
        }
        s.max_calling = s.max_calling.max(self.ends.len());
        let reply_to = if answer {
            self.minds[who].reply_to
        } else {
            None
        };
        let call = Call {
            who: who as u32,
            species: si as u32,
            group: gi as u32,
            start: t,
            pos,
            motif,
            source,
            answer,
            reply_to,
            harmonised,
        };
        self.spread(&call);
        if self.keep_score {
            self.score.push(call);
        }
    }

    /// The call reaches everyone of its species in range who might react.
    fn spread(&mut self, call: &Call) {
        let level = call.motif.level();
        let f = note_to_hz(call.motif.first_note() as f32) as f64;
        let dur = call.motif.duration();
        let c = self.settings.medium.speed;
        let absorb = self.settings.medium.absorb_k * f * f;
        let entry = Recent {
            who: call.who,
            species: call.species,
            start: call.start,
            pos: call.pos,
            level,
            absorb,
            motif: Rc::clone(&call.motif),
        };
        if self.recent.len() < RECENT {
            self.recent.push(entry);
        } else {
            self.recent[self.recent_next] = entry;
        }
        self.recent_next = (self.recent_next + 1) % RECENT;

        let si = call.species as usize;
        if !self.listens[si] {
            return;
        }
        let hs = &self.life.species[si];
        let (bh, rest_lo) = (hs.behaviour, hs.calls.rest.0.min(hs.calls.rest.1));
        let rest_mean = 0.5 * (hs.calls.rest.0 + hs.calls.rest.1);
        let ch = bh.chorus.clamp(-1.0, 1.0);
        let r = reach(level, bh.hearing, absorb);
        let r2 = r * r;
        // does this call get an answer? one roll per call
        let wants_answer =
            bh.response > 0.0 && self.minds[call.who as usize].rng.f64() < bh.response;
        let mut answerers: Vec<(usize, f64, f64)> = Vec::new(); // (who, arrival, weight)
        let mut buf = std::mem::take(&mut self.buf);
        buf.clear();
        self.grids[si].near(call.pos, r, &mut buf);
        for &h in &buf {
            let h = h as usize;
            if h == call.who as usize {
                continue;
            }
            let b = &self.life.animals[h];
            let (dx, dy) = (b.pos.x - call.pos.x, b.pos.y - call.pos.y);
            let d2 = dx * dx + dy * dy;
            if d2 > r2 {
                continue;
            }
            self.stats.heard += 1;
            let d = d2.sqrt();
            let ta = call.start + d / c;
            let m = &mut self.minds[h];
            if b.group == call.group {
                m.mate_note = call.motif.first_note();
                m.mate_until = ta + dur;
            }
            if m.calling_until > ta || m.answer {
                continue;
            }
            if wants_answer && ta - m.last_end >= rest_lo {
                let arriving = level - 20.0 * d.max(1.0).log10() - absorb * d;
                answerers.push((h, ta, 10f64.powf((arriving - bh.hearing) / 20.0)));
            }
            let new_due = if ch > 0.0
                && !m.nudged
                && m.due > ta
                && m.due - ta < ch * 0.4 * rest_mean
            {
                // synchronise: if its next call was coming soon anyway, join this one now.
                // Once per rest, so it shifts when it calls, not how often.
                m.nudged = true;
                Some(ta + m.rng.range(0.05, 0.25))
            } else if ch < 0.0 && m.due > ta - dur && m.due < ta + dur + 0.1 && m.rng.f64() < -ch {
                // alternate: wait until this one has finished
                Some(ta + dur + m.rng.range(0.1, 0.4))
            } else {
                None
            };
            if let Some(due) = new_due {
                m.gen += 1;
                m.due = due;
                let gen = m.gen;
                self.heap.push(Item {
                    at: due,
                    seq: self.seq,
                    ev: Ev::Due { who: h as u32, gen },
                });
                self.seq += 1;
            }
        }
        self.buf = buf;
        if !answerers.is_empty() {
            let total: f64 = answerers.iter().map(|a| a.2).sum();
            let mut x = self.minds[call.who as usize].rng.f64() * total;
            let mut pick = answerers[answerers.len() - 1];
            for a in &answerers {
                x -= a.2;
                if x <= 0.0 {
                    pick = *a;
                    break;
                }
            }
            let (h, ta, _) = pick;
            let m = &mut self.minds[h];
            m.answer = true;
            m.reply_to = Some((call.who, call.start));
            m.gen += 1;
            m.due = ta + dur + m.rng.range(ANSWER_DELAY.0, ANSWER_DELAY.1);
            let (due, gen) = (m.due, m.gen);
            self.push(due, Ev::Due { who: h as u32, gen });
        }
    }
}

/// A rectangle for a Box zone, for convenience.
pub fn box_zone(x0: f64, x1: f64, y0: f64, y1: f64, margin: f64) -> Zone {
    Zone::Box {
        rect: Rect::new(x0, x1, y0, y1),
        margin,
    }
}

/// Indices into `recent` of the last MEMORY calls that reached `pos` above `hearing` and had
/// arrived by `t` (not its own), newest first.
fn heard_recently(
    recent: &[Recent],
    next: usize,
    who: u32,
    pos: Vec2,
    hearing: f64,
    c: f64,
    t: f64,
) -> Vec<usize> {
    let mut out = Vec::with_capacity(MEMORY);
    let n = recent.len();
    for k in 1..=n {
        let i = (next + RECENT - k) % RECENT;
        if i >= n {
            continue;
        }
        let e = &recent[i];
        if t - e.start > MEMORY_SPAN {
            break;
        }
        if e.who == who {
            continue;
        }
        let d = e.pos.dist(pos);
        if e.start + d / c > t || e.level - 20.0 * d.max(1.0).log10() - e.absorb * d < hearing {
            continue;
        }
        out.push(i);
        if out.len() == MEMORY {
            break;
        }
    }
    out
}
