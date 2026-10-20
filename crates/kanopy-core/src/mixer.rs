//! Renders sounding units to stereo at the listener, block by block.
//!
//! Each `Emitter` is one unit from one animal: a `Voice` plus where it was emitted and how the
//! animal moves. Per block the mixer works out which source time reaches the listener (delay
//! and Doppler), the level from spreading, the low-pass from the medium, foliage, soil and
//! height, and the pan, ramps them across the block and adds the result to its species' bus.
//! Units too quiet to hear are not rendered, only advanced.
//!
//! **Box** listening (design §4.2) has no distance: no delay, spreading or filtering; a unit
//! sounds at its source level, panned by where it is across the box, faded over the last 2 m
//! inside the edges, silent outside.
//!
//! The buses then go through the `Master` chain (Ear, limiters, reverb), or are simply summed
//! when there is none.

use crate::filter::OnePole;
use crate::master::Master;
use crate::propagation::{
    cutoff_for, emission_time, height_cutoff, pan_value, pan_width, spreading, Medium, PathLoss,
    MIN_DIST,
};
use crate::voice::{Plan, UnitSpec, Voice};
use crate::wavetable::Wavetables;
use crate::world::Rect;
use crate::{db_to_amp, Vec2, BLOCK, FULL_SCALE_SPL};
use std::sync::Arc;

/// Units rendered side by side in full blocks. Their sample-to-sample chains (filter,
/// absorption) overlap instead of waiting on each other, and vectorise.
pub const LANES: usize = 8;

/// Below this linear gain at the listener (≈ −100 dBFS) a unit is not rendered.
pub const CULL_GAIN: f32 = 1e-5;

/// Box edge fade, metres.
pub const BOX_FADE: f64 = 2.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Hear {
    /// From the listener, with distance.
    Listener,
    /// Everything inside the box at its source level; nothing outside.
    Box(Rect),
}

/// Box listening: 1 well inside, 0 outside, ramping over the last BOX_FADE metres.
#[inline]
pub fn box_fade(r: &Rect, p: Vec2) -> f32 {
    let e = (p.x - r.x0).min(r.x1 - p.x).min(p.y - r.y0).min(r.y1 - p.y);
    (e / BOX_FADE).clamp(0.0, 1.0) as f32
}

#[derive(Clone, Copy, Debug)]
pub struct Listener {
    pub pos: Vec2,
    pub vel: Vec2,
}

impl Listener {
    pub fn still(pos: Vec2) -> Self {
        Self {
            pos,
            vel: Vec2::default(),
        }
    }
    pub fn at(&self, t: f64) -> Vec2 {
        self.pos.add_scaled(self.vel, t)
    }
}

#[derive(Clone, Debug)]
pub struct Emitter {
    pub voice: Voice,
    /// World time the unit starts at the animal, seconds.
    pub emit: f64,
    /// Animal position at world time 0 and its velocity: pos(τ) = p0 + v·τ.
    pub p0: Vec2,
    pub v: Vec2,
    pub bus: u32,
    /// Extra high-frequency loss from foliage and soil, dB per Hz².
    pub extra_k: f64,
    base_amp: f32,
    lp: OnePole,
    done: bool,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Stats {
    /// Unit-blocks rendered and skipped as inaudible.
    pub rendered: u64,
    pub culled: u64,
}

pub struct Mixer {
    pub wt: Arc<Wavetables>,
    pub medium: Medium,
    pub listener: Listener,
    pub hear: Hear,
    /// Stereo width, 0…1 (Underwater stereo narrows it).
    pub width: f32,
    /// Body cutoffs follow the medium's speed of sound.
    pub bodies_follow: bool,
    /// The output chain; without it the buses are just summed.
    pub master: Option<Master>,
    buses: Vec<[[f32; BLOCK]; 2]>,
    pub emitters: Vec<Emitter>,
    /// World time at the start of the next block, seconds.
    pub time: f64,
    pub stats: Stats,
    /// Render every unit on its own (no lanes). Same result, slower; for tests.
    pub single: bool,
    sample_rate: f32,
    scratch: [f32; BLOCK],
    jobs: Vec<Job>,
    lanes: Box<[[f32; LANES]; BLOCK]>,
}

/// A full block of one unit, waiting to be rendered side by side with others.
#[derive(Clone, Copy, Debug)]
struct Job {
    idx: usize,
    bus: usize,
    plan: Plan,
    lp_a: f32,
    gl0: f32,
    dgl: f32,
    gr0: f32,
    dgr: f32,
}

impl Mixer {
    pub fn new(wt: Arc<Wavetables>, medium: Medium, listener: Listener) -> Self {
        let sample_rate = wt.sample_rate;
        Self {
            wt,
            medium,
            listener,
            hear: Hear::Listener,
            width: 1.0,
            bodies_follow: true,
            master: None,
            buses: vec![[[0.0; BLOCK]; 2]],
            emitters: Vec::new(),
            time: 0.0,
            stats: Stats::default(),
            single: false,
            sample_rate,
            scratch: [0.0; BLOCK],
            jobs: Vec::new(),
            lanes: Box::new([[0.0; LANES]; BLOCK]),
        }
    }

    pub fn sample_rate(&self) -> f32 {
        self.sample_rate
    }

    pub fn add(&mut self, spec: UnitSpec, emit: f64, p0: Vec2, v: Vec2) {
        self.add_unit(spec, emit, p0, v, 0, PathLoss::default());
    }

    /// A unit on a species bus, with what its path adds (foliage, soil).
    pub fn add_unit(
        &mut self,
        mut spec: UnitSpec,
        emit: f64,
        p0: Vec2,
        v: Vec2,
        bus: u32,
        path: PathLoss,
    ) {
        if self.bodies_follow {
            let r = self.medium.body_ratio() as f32;
            spec.cutoff_start *= r;
            spec.cutoff_end *= r;
        }
        let db =
            spec.level_db as f64 - FULL_SCALE_SPL as f64 + self.medium.level_offset() - path.db;
        while self.buses.len() <= bus as usize {
            self.buses.push([[0.0; BLOCK]; 2]);
        }
        self.emitters.push(Emitter {
            voice: Voice::new(spec, self.sample_rate),
            emit,
            p0,
            v,
            bus,
            extra_k: path.k,
            base_amp: db_to_amp(db as f32),
            lp: OnePole::default(),
            done: false,
        });
    }

    pub fn buses(&self) -> usize {
        self.buses.len()
    }

    /// Render into `left`/`right` (added to what's there). Lengths must match.
    pub fn render(&mut self, left: &mut [f32], right: &mut [f32]) {
        assert_eq!(left.len(), right.len());
        let mut done = 0;
        while done < left.len() {
            let n = (left.len() - done).min(BLOCK);
            self.block(&mut left[done..done + n], &mut right[done..done + n]);
            done += n;
        }
    }

    fn block(&mut self, left: &mut [f32], right: &mut [f32]) {
        let n = left.len();
        let sr = self.sample_rate;
        let t0 = self.time;
        let t1 = t0 + n as f64 / sr as f64;
        let l0 = self.listener.at(t0);
        let l1 = self.listener.at(t1);
        let m = self.medium;
        let hear = self.hear;
        let width = self.width;
        let wt = Arc::clone(&self.wt);
        let mut jobs = std::mem::take(&mut self.jobs);
        jobs.clear();
        for b in &mut self.buses {
            b[0][..n].fill(0.0);
            b[1][..n].fill(0.0);
        }

        for idx in 0..self.emitters.len() {
            let e = &mut self.emitters[idx];
            let (tau0, tau1) = match hear {
                Hear::Listener => (
                    emission_time(t0, l0, e.p0, e.v, &m),
                    emission_time(t1, l1, e.p0, e.v, &m),
                ),
                Hear::Box(_) => (t0, t1),
            };
            let local0 = tau0 - e.emit;
            let local1 = tau1 - e.emit;
            if local0 >= e.voice.spec.dur as f64 {
                e.done = true;
                continue;
            }
            if local1 <= 0.0 {
                continue; // not arrived (or not started) yet
            }
            let rate = ((tau1 - tau0) / (t1 - t0)) as f32;
            // first sample of this block that carries the unit
            let start = if local0 < 0.0 {
                ((-local0) * sr as f64 / rate as f64).ceil() as usize
            } else {
                0
            }
            .min(n);
            e.voice.t = local0.max(0.0);

            let s0 = e.p0.add_scaled(e.v, tau0);
            let s1 = e.p0.add_scaled(e.v, tau1);
            let (g0, g1, lp_a, (pl0, pr0), (pl1, pr1)) = match hear {
                Hear::Listener => {
                    let d1 = l1.dist(s1);
                    let g0 = e.base_amp * spreading(l0.dist(s0));
                    let g1 = e.base_amp * spreading(d1);
                    let elev = if d1 > 1e-9 { (s1.y - l1.y) / d1 } else { 0.0 };
                    let cut = cutoff_for(m.absorb_k * d1.max(MIN_DIST) + e.extra_k, sr)
                        .min(height_cutoff(elev, sr));
                    (
                        g0,
                        g1,
                        OnePole::coeff(cut, sr),
                        pan_width(l0, s0, width),
                        pan_width(l1, s1, width),
                    )
                }
                Hear::Box(r) => {
                    let p = |s: Vec2| {
                        ((((s.x - r.x0) / r.width().max(1e-9)) * 2.0 - 1.0) as f32) * width
                    };
                    (
                        e.base_amp * box_fade(&r, s0),
                        e.base_amp * box_fade(&r, s1),
                        1.0,
                        pan_value(p(s0)),
                        pan_value(p(s1)),
                    )
                }
            };
            if g0.max(g1) < CULL_GAIN {
                self.stats.culled += 1;
                continue;
            }
            self.stats.rendered += 1;

            // gain and pan ramp together across the samples that carry the unit
            let k = 1.0 / (n - start).max(1) as f32;
            let (gl0, gr0) = (g0 * pl0, g0 * pr0);
            let dgl = (g1 * pl1 - gl0) * k;
            let dgr = (g1 * pr1 - gr0) * k;
            let bus = e.bus as usize;

            if start == 0 && n == BLOCK && !self.single {
                let plan = e.voice.plan(&wt, rate, n);
                if plan.valid == n {
                    jobs.push(Job {
                        idx,
                        bus,
                        plan,
                        lp_a,
                        gl0,
                        dgl,
                        gr0,
                        dgr,
                    });
                    continue;
                }
            }

            // a unit starting or ending inside this block: render it on its own
            let buf = &mut self.scratch[start..n];
            e.voice.render(&wt, buf, rate);
            let mut gl = gl0 + 0.5 * dgl;
            let mut gr = gr0 + 0.5 * dgr;
            let mut y = e.lp.y;
            let [bl, br] = &mut self.buses[bus];
            let (ls, rs) = (&mut bl[start..n], &mut br[start..n]);
            for ((x, lo), ro) in buf.iter().zip(ls.iter_mut()).zip(rs.iter_mut()) {
                y += lp_a * (*x - y);
                *lo += y * gl;
                *ro += y * gr;
                gl += dgl;
                gr += dgr;
            }
            e.lp.y = y;
        }

        for chunk in jobs.chunks(LANES) {
            self.lanes_block(&wt, chunk);
        }
        self.jobs = jobs;
        self.emitters.retain(|e| !e.done);
        match self.master.as_mut() {
            Some(master) => master.process(&mut self.buses, left, right),
            None => {
                for b in &self.buses {
                    for i in 0..n {
                        left[i] += b[0][i];
                        right[i] += b[1][i];
                    }
                }
            }
        }
        self.time = t1;
    }

    /// Up to LANES full blocks side by side: oscillators one by one, then body, envelope and
    /// absorption in lockstep, then pan into the mix.
    fn lanes_block(&mut self, wt: &Wavetables, chunk: &[Job]) {
        let lanes = &mut *self.lanes;
        let mut tmp = [0f32; BLOCK];
        let z = [0f32; LANES];
        let (mut a1, mut a2, mut a3, mut k, mut ic1, mut ic2) = (z, z, z, z, z, z);
        let (mut ea, mut dea, mut er, mut der, mut lpa, mut lpy) = (z, z, z, z, z, z);
        for (l, job) in chunk.iter().enumerate() {
            let e = &mut self.emitters[job.idx];
            e.voice.osc(wt, &job.plan, &mut tmp);
            for (row, x) in lanes.iter_mut().zip(tmp.iter()) {
                row[l] = *x;
            }
            let f = e.voice.body_mut();
            (a1[l], a2[l], a3[l], k[l], ic1[l], ic2[l]) = (f.a1, f.a2, f.a3, f.k, f.ic1, f.ic2);
            (ea[l], dea[l], er[l], der[l]) = (job.plan.ea, job.plan.dea, job.plan.er, job.plan.der);
            (lpa[l], lpy[l]) = (job.lp_a, e.lp.y);
            e.voice.t = job.plan.t1;
        }
        for row in chunk.len()..LANES {
            for r in lanes.iter_mut() {
                r[row] = 0.0;
            }
        }
        for row in lanes.iter_mut() {
            for l in 0..LANES {
                let x = row[l];
                let v3 = x - ic2[l];
                let v1 = a1[l] * ic1[l] + a2[l] * v3;
                let v2 = ic2[l] + a2[l] * ic1[l] + a3[l] * v3;
                ic1[l] = 2.0 * v1 - ic1[l];
                ic2[l] = 2.0 * v2 - ic2[l];
                let hp = x - k[l] * v1 - v2;
                let env = ea[l].min(1.0).min(er[l]).max(0.0);
                ea[l] += dea[l];
                er[l] -= der[l];
                lpy[l] += lpa[l] * (hp * env - lpy[l]);
                row[l] = lpy[l];
            }
        }
        for (l, job) in chunk.iter().enumerate() {
            let e = &mut self.emitters[job.idx];
            let f = e.voice.body_mut();
            f.ic1 = ic1[l];
            f.ic2 = ic2[l];
            e.lp.y = lpy[l];
            let [left, right] = &mut self.buses[job.bus];
            for (i, row) in lanes.iter().enumerate() {
                let w = i as f32 + 0.5;
                left[i] += row[l] * (job.gl0 + job.dgl * w);
                right[i] += row[l] * (job.gr0 + job.dgr * w);
            }
        }
    }
}
