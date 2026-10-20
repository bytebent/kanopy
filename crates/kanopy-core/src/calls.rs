//! Calls and motifs (design §5.2, §5.6).
//!
//! A call is kept as a motif: per unit its start, length, note, contour and sound. Values sit in
//! the ranges of the species that made it, so a value's position inside that range (0–1) is its
//! gesture.
//!
//! - `fresh`: a new call rolled from a species' ranges, units moving by its Evolve.
//! - `vary`: the same call a little changed (signature Variation, or mimicry errors).
//! - `copy`: another animal's call in the copier's voice: gesture positions mapped into the
//!   copier's ranges, pitch as steps (Pitch match 0) or aimed at the actual pitch (1), sound kept
//!   (Timbre match 0) or aimed at the caller's Shape and body (1), then errors from Accuracy.
//! - `harmonise`: move a call so it starts on a consonance with a heard note.

use crate::rng::Rng;
use crate::scale::{consonances, Key};
use crate::species::{Evolve, Range, Species};
use crate::voice::UnitSpec;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Unit {
    /// Start, seconds from the call's start.
    pub at: f64,
    pub dur: f64,
    /// Anchor note, MIDI.
    pub note: f64,
    /// Glide across the unit, semitones.
    pub contour: f64,
    pub shape: f64,
    pub cutoff: f64,
    pub resonance: f64,
    pub attack: f64,
    pub release: f64,
    /// dB SPL at 1 m.
    pub level: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Motif {
    /// The species whose ranges these values sit in.
    pub species: u32,
    pub units: Vec<Unit>,
}

impl Motif {
    pub fn duration(&self) -> f64 {
        self.units.iter().map(|u| u.at + u.dur).fold(0.0, f64::max)
    }
    pub fn level(&self) -> f64 {
        self.units
            .iter()
            .map(|u| u.level)
            .fold(f64::NEG_INFINITY, f64::max)
    }
    pub fn first_note(&self) -> f64 {
        self.units[0].note
    }
    /// The units as voice specs, with their start offsets.
    pub fn specs(&self) -> impl Iterator<Item = (f64, UnitSpec)> + '_ {
        self.units.iter().map(|u| {
            (
                u.at,
                UnitSpec {
                    note_start: u.note as f32,
                    note_end: (u.note + u.contour) as f32,
                    dur: u.dur as f32,
                    shape_start: u.shape as f32,
                    shape_end: u.shape as f32,
                    cutoff_start: u.cutoff as f32,
                    cutoff_end: u.cutoff as f32,
                    resonance: u.resonance as f32,
                    attack: u.attack as f32,
                    release: u.release as f32,
                    level_db: u.level as f32,
                },
            )
        })
    }
}

fn ord(r: Range) -> Range {
    (r.0.min(r.1), r.0.max(r.1))
}
/// Value at position x (0…1) in a range.
pub fn lin(x: f64, r: Range) -> f64 {
    let r = ord(r);
    r.0 + (r.1 - r.0) * x.clamp(0.0, 1.0)
}
/// Position (0…1) of a value in a range; 0.5 for a locked range.
pub fn pos(v: f64, r: Range) -> f64 {
    let r = ord(r);
    if r.1 - r.0 < 1e-12 {
        0.5
    } else {
        ((v - r.0) / (r.1 - r.0)).clamp(0.0, 1.0)
    }
}
/// The same on a log scale (for cutoff).
pub fn lin_log(x: f64, r: Range) -> f64 {
    let r = ord(r);
    lin(x, (r.0.max(1e-3).ln(), r.1.max(1e-3).ln())).exp()
}
pub fn pos_log(v: f64, r: Range) -> f64 {
    let r = ord(r);
    pos(v.max(1e-3).ln(), (r.0.max(1e-3).ln(), r.1.max(1e-3).ln()))
}
fn roll(r: Range, rng: &mut Rng) -> f64 {
    let r = ord(r);
    rng.range(r.0, r.1)
}
fn jit(x: f64, amount: f64, rng: &mut Rng) -> f64 {
    (x + rng.gauss() * amount).clamp(0.0, 1.0)
}

/// Clamp every note into the species' pitch range; in key, onto the scale.
fn fit(m: &mut Motif, sp: &Species, key: Option<&Key>) {
    let (lo, hi) = ord(sp.voice.pitch);
    for u in &mut m.units {
        u.note = match key {
            Some(k) => k.snap_within(u.note, lo, hi),
            None => u.note.clamp(lo, hi),
        };
    }
}

/// Starts from lengths and gaps.
fn lay_out(units: &mut [Unit], gaps: &[f64]) {
    let mut t = 0.0;
    for (u, g) in units.iter_mut().zip(gaps) {
        u.at = t;
        t += u.dur + g;
    }
}

fn gaps_of(units: &[Unit]) -> Vec<f64> {
    (0..units.len())
        .map(|k| {
            if k + 1 < units.len() {
                (units[k + 1].at - units[k].at - units[k].dur).max(0.0)
            } else {
                0.0
            }
        })
        .collect()
}

/// A new call rolled from the species' ranges.
pub fn fresh(sp: &Species, si: u32, key: Option<&Key>, rng: &mut Rng) -> Motif {
    let (v, c) = (&sp.voice, &sp.calls);
    let (ulo, uhi) = (
        c.units.0.clamp(1, 32) as i64,
        (c.units.1.clamp(1, 32) as i64).max(c.units.0.clamp(1, 32) as i64),
    );
    let n = rng.int(ulo, uhi) as usize;
    let (plo, phi) = ord(v.pitch);
    let span = phi - plo;
    let amount = roll(c.evolve_amount, rng);
    // pitch movement across the call, before choosing where it sits
    let mut walk = 0.0;
    let d: Vec<f64> = (0..n)
        .map(|k| {
            let t = if n > 1 {
                k as f64 / (n - 1) as f64
            } else {
                0.0
            };
            let f = match c.evolve {
                Evolve::Steady => 0.03 * rng.gauss(),
                Evolve::Rise => t,
                Evolve::Fall => -t,
                Evolve::Arc => (std::f64::consts::PI * t).sin(),
                Evolve::Wander => {
                    if k > 0 {
                        walk += 0.4 * rng.gauss();
                    }
                    walk
                }
            };
            amount * span * f
        })
        .collect();
    let (dmin, dmax) = d
        .iter()
        .fold((0.0f64, 0.0f64), |(a, b), &x| (a.min(x), b.max(x)));
    let (blo, bhi) = (plo - dmin, phi - dmax);
    let base = if blo <= bhi {
        rng.range(blo, bhi)
    } else {
        0.5 * (plo + phi) - 0.5 * (dmin + dmax)
    };
    // one sound per call, each unit a little different
    let xs: Vec<f64> = (0..8).map(|_| rng.f64()).collect();
    let mut units = Vec::with_capacity(n);
    let mut gaps = Vec::with_capacity(n);
    for dk in d {
        units.push(Unit {
            at: 0.0,
            dur: lin(jit(xs[0], 0.15, rng), c.unit_length),
            note: base + dk,
            contour: roll(v.contour, rng),
            shape: lin(jit(xs[1], 0.1, rng), v.shape),
            cutoff: lin_log(jit(xs[2], 0.1, rng), v.cutoff),
            resonance: lin(jit(xs[3], 0.1, rng), v.resonance),
            attack: lin(jit(xs[4], 0.1, rng), v.attack),
            release: lin(jit(xs[5], 0.1, rng), v.release),
            level: lin(jit(xs[6], 0.1, rng), v.level),
        });
        gaps.push(lin(jit(xs[7], 0.2, rng), c.gap));
    }
    lay_out(&mut units, &gaps);
    let mut m = Motif { species: si, units };
    fit(&mut m, sp, key);
    m
}

/// Move a note by whole steps: scale degrees in key, semitones when Free.
fn step(note: f64, by: i64, key: Option<&Key>) -> f64 {
    match key {
        Some(k) => k.note(k.degree(note) + by),
        None => note + by as f64,
    }
}

/// The call a little changed. `amount` 0 = unchanged, 1 = barely recognisable. Unit count stays
/// within `units` (lo, hi).
pub fn vary(
    m: &Motif,
    sp: &Species,
    amount: f64,
    units: (u32, u32),
    key: Option<&Key>,
    rng: &mut Rng,
) -> Motif {
    let a = amount.clamp(0.0, 1.0);
    if a <= 0.0 {
        return m.clone();
    }
    let (v, c) = (&sp.voice, &sp.calls);
    let mut us = m.units.clone();
    let mut gaps = gaps_of(&us);
    if rng.f64() < a * 0.3 {
        let (lo, hi) = (units.0.max(1) as usize, (units.1.max(1) as usize).min(32));
        let k = rng.int(0, us.len() as i64 - 1) as usize;
        if us.len() > lo && (rng.f64() < 0.5 || us.len() >= hi) {
            us.remove(k);
            gaps.remove(k);
        } else if us.len() < hi {
            us.insert(k + 1, us[k]);
            gaps.insert(k + 1, gaps[k]);
        }
    }
    for (u, g) in us.iter_mut().zip(gaps.iter_mut()) {
        if rng.f64() < a * 0.5 {
            let by = if rng.f64() < 0.7 { 1 } else { 2 } * if rng.f64() < 0.5 { 1 } else { -1 };
            u.note = step(u.note, by, key);
        }
        let ul = ord(c.unit_length);
        u.dur = (u.dur * (rng.gauss() * a * 0.35).exp()).clamp(ul.0, ul.1);
        let gl = ord(c.gap);
        *g = (*g * (rng.gauss() * a * 0.35).exp()).clamp(gl.0, gl.1);
        let j = a * 0.15;
        u.shape = lin(jit(pos(u.shape, v.shape), j, rng), v.shape);
        u.cutoff = lin_log(jit(pos_log(u.cutoff, v.cutoff), j, rng), v.cutoff);
        u.resonance = lin(jit(pos(u.resonance, v.resonance), j, rng), v.resonance);
        u.level = lin(jit(pos(u.level, v.level), j, rng), v.level);
        u.contour = lin(jit(pos(u.contour, v.contour), j, rng), v.contour);
    }
    lay_out(&mut us, &gaps);
    let mut out = Motif {
        species: m.species,
        units: us,
    };
    fit(&mut out, sp, key);
    out
}

/// Another animal's call in the copier's voice (§5.6).
#[allow(clippy::too_many_arguments)]
pub fn copy(
    m: &Motif,
    from: &Species,
    to: &Species,
    to_si: u32,
    key: Option<&Key>,
    rng: &mut Rng,
) -> Motif {
    let b = &to.behaviour;
    let (fv, fc, tv, tc) = (&from.voice, &from.calls, &to.voice, &to.calls);
    let n0 = m.first_note();
    // pitch: steps from the first unit, and where the copy starts
    let steps: Vec<f64> = match key {
        Some(k) => {
            let d0 = k.degree(n0);
            m.units
                .iter()
                .map(|u| (k.degree(u.note) - d0) as f64)
                .collect()
        }
        None => m.units.iter().map(|u| u.note - n0).collect(),
    };
    let gesture = lin(pos(n0, fv.pitch), tv.pitch);
    let base = gesture + (n0 - gesture) * b.pitch_match.clamp(0.0, 1.0);
    let tm = b.timbre_match.clamp(0.0, 1.0);
    let toward = |g: f64, actual: f64, r: Range| g + (actual.clamp(ord(r).0, ord(r).1) - g) * tm;
    let mut gaps = Vec::with_capacity(m.units.len());
    let src_gaps = gaps_of(&m.units);
    let units: Vec<Unit> = m
        .units
        .iter()
        .zip(&steps)
        .zip(&src_gaps)
        .map(|((u, &s), &g)| {
            gaps.push(lin(pos(g, fc.gap), tc.gap));
            let note = match key {
                Some(k) => k.note(k.degree(base) + s as i64),
                None => base + s,
            };
            let shape_g = lin(pos(u.shape, fv.shape), tv.shape);
            let cut_g = lin_log(pos_log(u.cutoff, fv.cutoff), tv.cutoff);
            let res_g = lin(pos(u.resonance, fv.resonance), tv.resonance);
            let tc_ = ord(tv.cutoff);
            Unit {
                at: 0.0,
                dur: lin(pos(u.dur, fc.unit_length), tc.unit_length),
                note,
                contour: lin(pos(u.contour, fv.contour), tv.contour),
                shape: toward(shape_g, u.shape, tv.shape),
                cutoff: (cut_g.ln() + (u.cutoff.clamp(tc_.0, tc_.1).ln() - cut_g.ln()) * tm).exp(),
                resonance: toward(res_g, u.resonance, tv.resonance),
                attack: lin(pos(u.attack, fv.attack), tv.attack),
                release: lin(pos(u.release, fv.release), tv.release),
                level: lin(pos(u.level, fv.level), tv.level),
            }
        })
        .collect();
    let mut out = Motif {
        species: to_si,
        units,
    };
    lay_out(&mut out.units, &gaps);
    fit(&mut out, to, key);
    // errors: the copy keeps the caller's unit count (that's the gesture), within 1–32
    vary(&out, to, 1.0 - b.mimicry_accuracy, (1, 32), key, rng)
}

/// Move the call so its first note is consonant with `heard`, as near as possible to where it
/// was. Returns false (and leaves it) if no consonance fits the pitch range.
pub fn harmonise(
    m: &mut Motif,
    heard: f64,
    sp: &Species,
    key: Option<&Key>,
    rng: &mut Rng,
) -> bool {
    let (lo, hi) = ord(sp.voice.pitch);
    let first = m.first_note();
    let mut cands: Vec<f64> = consonances(key)
        .into_iter()
        .map(|c| heard + c)
        .filter(|&n| n >= lo - 1e-9 && n <= hi + 1e-9 && key.is_none_or(|k| k.contains(n)))
        .collect();
    if cands.is_empty() {
        return false;
    }
    cands.sort_by(|a, b| (a - first).abs().total_cmp(&(b - first).abs()));
    let target = cands[rng.int(0, cands.len().min(3) as i64 - 1) as usize];
    match key {
        Some(k) => {
            let d = k.degree(target) - k.degree(first);
            for u in &mut m.units {
                u.note = k.note(k.degree(u.note) + d);
            }
        }
        None => {
            let d = target - first;
            for u in &mut m.units {
                u.note += d;
            }
        }
    }
    let keep = m.units[0].note;
    fit(m, sp, key);
    m.units[0].note = keep.clamp(lo, hi);
    true
}
