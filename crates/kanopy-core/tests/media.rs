//! Batch 04: media, path losses, Box listening, the Ear, limiters and reverb.

mod common;
use common::*;
use kanopy_core::life::{Life, Zone};
use kanopy_core::master::*;
use kanopy_core::mixer::{box_fade, Hear, Listener, Mixer};
use kanopy_core::propagation::*;
use kanopy_core::rng::Rng;
use kanopy_core::scene::{Scene, SceneSettings};
use kanopy_core::species::*;
use kanopy_core::voice::UnitSpec;
use kanopy_core::wavetable::Wavetables;
use kanopy_core::world::{Forest, ForestSettings, Rect, Tree};
use kanopy_core::Vec2;
use std::sync::Arc;

fn mixer(medium: Medium, at: Vec2) -> Mixer {
    Mixer::new(Arc::new(Wavetables::new(SR)), medium, Listener::still(at))
}

fn run(m: &mut Mixer, seconds: f32) -> (Vec<f32>, Vec<f32>) {
    let n = (seconds * SR) as usize;
    let (mut l, mut r) = (vec![0f32; n], vec![0f32; n]);
    m.render(&mut l, &mut r);
    (l, r)
}

fn saw(note: f32, dur: f32) -> UnitSpec {
    UnitSpec {
        note_start: note,
        note_end: note,
        dur,
        shape_start: 2.0,
        shape_end: 2.0,
        attack: 0.01,
        release: 0.01,
        level_db: 80.0,
        ..Default::default()
    }
}

fn mono(l: &[f32], r: &[f32]) -> Vec<f32> {
    l.iter().zip(r).map(|(a, b)| a + b).collect()
}

/// Power of a steady part of a render, both channels, dB
fn level(l: &[f32], r: &[f32], from: f32, to: f32) -> f32 {
    let (a, b) = ((from * SR) as usize, (to * SR) as usize);
    let p: f64 = l[a..b]
        .iter()
        .chain(&r[a..b])
        .map(|x| (*x as f64).powi(2))
        .sum::<f64>()
        / (b - a) as f64;
    (10.0 * p.max(1e-24).log10()) as f32
}

/// High (harmonic near 4 kHz) against low (fundamental) of a 220 Hz saw, dB
fn brightness(x: &[f32]) -> f32 {
    db(tone(x, 220.0 * 18.0, SR)) - db(tone(x, 220.0, SR))
}

// ---------- media

#[test]
fn media_presets() {
    let near = |a: f64, b: f64| (a - b).abs() < 1.0;
    assert!(near(Medium::earth().level_offset(), 0.0));
    assert!(near(Medium::mars().level_offset(), -19.0));
    assert!(near(Medium::titan().level_offset(), 4.0));
    assert!(near(Medium::venus().level_offset(), 18.0));
    assert!(near(Medium::water().level_offset(), 36.0));
    let st = |m: Medium| 12.0 * m.body_ratio().log2();
    assert!((st(Medium::titan()) + 9.9).abs() < 0.2 && (st(Medium::water()) - 25.3).abs() < 0.1);
    assert_eq!(Medium::preset("WATER").unwrap().name, "Water");
    assert!(Medium::water().liquid && !Medium::venus().liquid);
}

#[test]
fn water_is_louder_and_faster() {
    let at = Vec2::new(0.0, 1.0);
    let lv = |m: Medium| {
        let mut mx = mixer(m, at);
        mx.add(saw(57.0, 1.0), 0.0, Vec2::new(10.0, 1.0), Vec2::default());
        let (l, r) = run(&mut mx, 1.0);
        level(&l, &r, 0.3, 0.7)
    };
    let d = lv(Medium::water()) - lv(Medium::earth());
    assert!((d - 35.6).abs() < 0.5, "{d}");
    let mut mx = mixer(Medium::water(), at);
    mx.add(saw(69.0, 0.1), 0.0, Vec2::new(148.0, 1.0), Vec2::default());
    let (l, _) = run(&mut mx, 0.2);
    let first = l.iter().position(|x| *x != 0.0).unwrap();
    assert!(
        (first as f32 - 0.1 * SR).abs() <= 2.0,
        "arrives after {first} samples"
    );
}

#[test]
fn mars_takes_the_highs() {
    let bright = |m: Medium, d: f64| {
        let mut mx = mixer(m, Vec2::new(0.0, 1.0));
        mx.add(saw(57.0, 1.0), 0.0, Vec2::new(d, 1.0), Vec2::default());
        let (l, r) = run(&mut mx, 1.1);
        let x = mono(&l, &r);
        brightness(&x[(0.4 * SR) as usize..(0.9 * SR) as usize])
    };
    let (earth, mars) = (bright(Medium::earth(), 20.0), bright(Medium::mars(), 20.0));
    assert!(earth - mars > 10.0, "earth {earth:.1}, mars {mars:.1}");
    assert!(
        bright(Medium::mars(), 2.0) - mars > 6.0,
        "on Mars, near is much brighter than far"
    );
}

#[test]
fn bodies_follow_the_medium() {
    for (m, on) in [
        (Medium::water(), true),
        (Medium::titan(), true),
        (Medium::water(), false),
    ] {
        let mut mx = mixer(m, Vec2::default());
        mx.bodies_follow = on;
        let spec = UnitSpec {
            cutoff_start: 300.0,
            cutoff_end: 300.0,
            ..saw(60.0, 0.5)
        };
        mx.add(spec, 0.0, Vec2::new(5.0, 0.0), Vec2::default());
        let c = mx.emitters[0].voice.spec.cutoff_start as f64;
        let want = if on { 300.0 * m.body_ratio() } else { 300.0 };
        assert!((c - want).abs() < 1e-3);
    }
}

// ---------- path

#[test]
fn path_geometry() {
    let c = Vec2::new(0.0, 0.0);
    let l = length_in_ellipse(Vec2::new(-10.0, 0.0), Vec2::new(10.0, 0.0), c, 4.0, 2.0);
    assert!((l - 8.0).abs() < 1e-9);
    assert_eq!(
        length_in_ellipse(Vec2::new(-10.0, 3.0), Vec2::new(10.0, 3.0), c, 4.0, 2.0),
        0.0
    );
    let half = length_in_ellipse(Vec2::new(0.0, 0.0), Vec2::new(10.0, 0.0), c, 4.0, 2.0);
    assert!((half - 4.0).abs() < 1e-9);
    let b = length_in_band(Vec2::new(0.0, -2.0), Vec2::new(0.0, 4.0), -1.0, 0.0);
    assert!((b - 1.0).abs() < 1e-9);
}

fn one_tree() -> Forest {
    let mut f = Forest::generate(ForestSettings {
        seed: 1,
        density: 0.0,
        ..Default::default()
    });
    f.trees = vec![Tree {
        x: 50.0,
        height: 30.0,
        crown_bottom: 10.0,
        crown_half_width: 10.0,
    }];
    f.tallest = 0;
    f.shrub_fullness = 0.0;
    f
}

#[test]
fn foliage_is_subtle_and_soil_is_not() {
    let f = one_tree();
    let open = path_loss(&f, Vec2::new(30.0, 40.0), Vec2::new(70.0, 40.0));
    let through = path_loss(&f, Vec2::new(30.0, 20.0), Vec2::new(70.0, 20.0));
    assert_eq!(open, PathLoss::default());
    assert!(through.k > 0.0 && through.db == 0.0);
    let at_4k = through.k * 4000f64.powi(2);
    assert!(
        at_4k > 1.0 && at_4k < 6.0,
        "20 m of crown takes {at_4k:.1} dB off 4 kHz"
    );
    let under = path_loss(&f, Vec2::new(10.0, 5.0), Vec2::new(10.0, -2.0));
    assert!((under.db - (SURFACE_LOSS + 2.0 * SOIL_DB)).abs() < 1e-9);
    let both_under = path_loss(&f, Vec2::new(10.0, -1.0), Vec2::new(13.0, -2.0));
    assert!(both_under.db < 6.0, "burrowers near each other stay clear");
}

#[test]
fn from_underground_the_world_is_muffled() {
    let lv = |src: Vec2, lis: Vec2| {
        let f = one_tree();
        let mut mx = mixer(Medium::earth(), lis);
        mx.add_unit(
            saw(57.0, 1.0),
            0.0,
            src,
            Vec2::default(),
            0,
            path_loss(&f, src, lis),
        );
        let (l, r) = run(&mut mx, 1.1);
        (
            level(&l, &r, 0.4, 0.9),
            brightness(&mono(&l, &r)[(0.4 * SR) as usize..(0.9 * SR) as usize]),
        )
    };
    let above = lv(Vec2::new(10.0, 8.0), Vec2::new(10.0, 1.0));
    let below = lv(Vec2::new(10.0, 8.0), Vec2::new(10.0, -2.0));
    let burrower = lv(Vec2::new(13.0, -1.5), Vec2::new(10.0, -2.0));
    assert!(above.0 - below.0 > 20.0, "{above:?} vs {below:?}");
    assert!(above.1 - below.1 > 6.0, "and duller");
    assert!(burrower.0 > below.0 + 15.0);
}

#[test]
fn below_is_a_little_darker_than_above() {
    let b = |y: f64| {
        let mut mx = mixer(Medium::earth(), Vec2::new(0.0, 20.0));
        mx.add(
            saw(57.0, 1.0),
            0.0,
            Vec2::new(0.0, 20.0 + y),
            Vec2::default(),
        );
        let (l, r) = run(&mut mx, 1.1);
        brightness(&mono(&l, &r)[(0.4 * SR) as usize..(0.9 * SR) as usize])
    };
    let (up, down) = (b(10.0), b(-10.0));
    assert!(
        up - down > 1.0 && up - down < 6.0,
        "above {up:.1}, below {down:.1}"
    );
}

#[test]
fn underwater_stereo_narrows() {
    let side = |width: f32| {
        let mut mx = mixer(Medium::water(), Vec2::new(0.0, 1.0));
        mx.width = width;
        mx.add(saw(57.0, 0.5), 0.0, Vec2::new(20.0, 1.0), Vec2::default());
        let (l, r) = run(&mut mx, 0.6);
        db(rms(&r[4800..19200])) - db(rms(&l[4800..19200]))
    };
    assert!(side(1.0) > 40.0);
    let narrow = side(0.35);
    assert!(narrow > 3.0 && narrow < 10.0, "{narrow:.1} dB");
}

// ---------- box

#[test]
fn box_has_no_distance() {
    let r = Rect::new(40.0, 80.0, 0.0, 30.0);
    let render = |p: Vec2| {
        let mut mx = mixer(Medium::earth(), Vec2::new(0.0, 1.0));
        mx.hear = Hear::Box(r);
        mx.add(saw(57.0, 0.5), 0.1, p, Vec2::default());
        run(&mut mx, 0.7)
    };
    // well inside, far from the listener: its 1 m level, and no delay
    let (l, rr) = render(Vec2::new(70.0, 20.0));
    let first = l.iter().position(|x| *x != 0.0).unwrap();
    assert!((first as f32 - 0.1 * SR).abs() <= 1.0);
    let mut solo = mixer(Medium::earth(), Vec2::new(70.0, 20.0));
    solo.add(saw(57.0, 0.5), 0.1, Vec2::new(70.0, 20.0), Vec2::default());
    let (sl, sr) = run(&mut solo, 0.7);
    assert!((level(&l, &rr, 0.2, 0.5) - level(&sl, &sr, 0.2, 0.5)).abs() < 0.2);
    // panned by x across the box: 70 of 40…80 is right of centre
    assert!(rms(&rr[9600..24000]) > 2.0 * rms(&l[9600..24000]));
    // outside: nothing
    let (l, rr) = render(Vec2::new(90.0, 20.0));
    assert!(l.iter().chain(&rr).all(|x| *x == 0.0));
    // fades over the last 2 m
    assert_eq!(box_fade(&r, Vec2::new(41.0, 15.0)), 0.5);
    assert_eq!(box_fade(&r, Vec2::new(60.0, 15.0)), 1.0);
}

// ---------- limiters and master

#[test]
fn limiter_holds_the_line_and_leaves_quiet_alone() {
    let mut lim = Limiter::new(-3.0, SR);
    let mut rng = Rng::new(1);
    let thr = kanopy_core::db_to_amp(-3.0);
    for k in 0..2000 {
        let amp = if k % 300 < 40 { 8.0 } else { 0.3 };
        let mut l: Vec<f32> = (0..64).map(|_| (rng.gauss() as f32) * amp).collect();
        let mut r = l.clone();
        lim.process(&mut l, &mut r);
        assert!(
            l.iter().chain(&r).all(|x| x.abs() <= thr * 1.0001),
            "block {k}"
        );
    }
    // quiet: passes through one block late, untouched
    let mut quiet = Limiter::new(-3.0, SR);
    let a: Vec<f32> = (0..64).map(|i| (i as f32 * 0.1).sin() * 0.2).collect();
    let (mut l1, mut r1) = (a.clone(), a.clone());
    quiet.process(&mut l1, &mut r1);
    let (mut l2, mut r2) = (vec![0f32; 64], vec![0f32; 64]);
    quiet.process(&mut l2, &mut r2);
    assert!(l1.iter().all(|x| *x == 0.0));
    assert_eq!(l2, a);
    assert_eq!(quiet.min_gain, 1.0);
}

fn master_run(ear: Ear, start_db: f64, input_amp: f32, seconds: f32) -> (Master, Vec<f32>) {
    let s = MasterSettings {
        ear,
        start_db,
        reference_shift: 0.0,
        reverb: None,
    };
    let mut m = Master::new(&s, 2, SR);
    let mut rng = Rng::new(2);
    let mut out = Vec::new();
    let mut gains = Vec::new();
    for _ in 0..(seconds * SR / 64.0) as usize {
        let mut buses = vec![[[0f32; 64]; 2]; 2];
        for b in buses.iter_mut() {
            for c in b.iter_mut() {
                for x in c.iter_mut() {
                    *x = rng.gauss() as f32 * input_amp;
                }
            }
        }
        let (mut l, mut r) = (vec![0f32; 64], vec![0f32; 64]);
        m.process(&mut buses, &mut l, &mut r);
        out.extend(l);
        gains.push(m.gain_db as f32);
    }
    (m, gains.into_iter().chain(out).collect())
}

#[test]
fn ear_fixed_and_adapting() {
    let (m, _) = master_run(Ear::Fixed, 3.0, 0.001, 5.0);
    assert_eq!(m.gain_db, 3.0);
    // very quiet input: Adapting turns up, but never past +5
    let (m, _) = master_run(Ear::Adapting, 0.0, 0.0001, 120.0);
    assert!((m.gain_db - EAR_RANGE).abs() < 0.05, "{}", m.gain_db);
    // loud input: down, never past −5; the final limiter holds −1 dBFS
    let (m, rest) = master_run(Ear::Adapting, 0.0, 1.0, 60.0);
    assert!((m.gain_db + EAR_RANGE).abs() < 0.5, "{}", m.gain_db);
    let ceiling = kanopy_core::db_to_amp(FINAL_LIMIT_DB);
    let audio = &rest[rest.len() - (60.0 * SR) as usize..];
    assert!(audio.iter().all(|x| x.abs() <= ceiling * 1.0001));
}

// ---------- reverb

fn impulse_tail(rt: f32, bright: f32, seconds: f32) -> Vec<f32> {
    let mut rv = Reverb::new(rt, bright, 1.0, SR);
    let n = (seconds * SR) as usize;
    let (mut l, mut r) = (vec![0f32; n], vec![0f32; n]);
    l[0] = 1.0;
    r[0] = 1.0;
    for k in 0..n / 64 {
        rv.process(&mut l[k * 64..(k + 1) * 64], &mut r[k * 64..(k + 1) * 64]);
    }
    l[0] = 0.0;
    l
}

/// Seconds to fall 30 dB (energy over 50 ms windows), doubled: an RT60 estimate.
fn rt60(x: &[f32]) -> f32 {
    let w = (0.05 * SR) as usize;
    let e: Vec<f32> = x.chunks(w).map(|c| db(rms(c))).collect();
    let start = e.iter().cloned().fold(f32::MIN, f32::max);
    let k = e.iter().position(|v| *v < start - 30.0).unwrap_or(e.len());
    2.0 * k as f32 * 0.05
}

#[test]
fn reverb_lengths_follow_the_forest() {
    for rt in [0.8f32, 2.0] {
        let got = rt60(&impulse_tail(rt, 20_000.0, rt * 2.0));
        assert!((got - rt).abs() < rt * 0.4, "asked {rt}, got {got}");
    }
    let (wood, _) = reverb_for(0.0, &Medium::earth());
    let (thicket, _) = reverb_for(1.0, &Medium::earth());
    let (_, mars_bright) = reverb_for(0.5, &Medium::mars());
    let (_, earth_bright) = reverb_for(0.5, &Medium::earth());
    assert!(thicket > 2.0 * wood && mars_bright < earth_bright / 3.0);
    let dark = impulse_tail(1.5, 900.0, 1.0);
    let bright = impulse_tail(1.5, 7000.0, 1.0);
    let hf = |x: &[f32]| {
        x.windows(2).map(|w| (w[1] - w[0]).powi(2)).sum::<f32>()
            / x.iter().map(|v| v * v).sum::<f32>()
    };
    assert!(hf(&dark[4800..]) < 0.5 * hf(&bright[4800..]));
}

// ---------- the scene's level

#[test]
fn the_estimate_is_near_the_truth() {
    for medium in [Medium::earth(), Medium::titan()] {
        let f = Forest::generate(ForestSettings {
            seed: 3,
            ..Default::default()
        });
        let st = SceneSettings {
            medium,
            ear: Ear::Fixed,
            reverb: false,
            ..Default::default()
        };
        let mut s = Scene::new(Life::new(f, demo(0.3)), st).with_sound(SR);
        let n = s.step_samples();
        let steps = (90.0 / kanopy_core::life::DT) as usize;
        let (mut l, mut r) = (vec![0f32; n * steps], vec![0f32; n * steps]);
        for k in 0..steps {
            s.step_audio(&mut l[k * n..(k + 1) * n], &mut r[k * n..(k + 1) * n]);
        }
        let est = s.estimate_db();
        let m = s.mixer.as_ref().unwrap().master.as_ref().unwrap();
        let predicted = est + m.reference_shift + m.start_db;
        let from = (30.0 * SR) as usize; // after the first calls have started
        let measured = db(rms(&[&l[from..], &r[from..]].concat()));
        assert!(
            (measured - predicted as f32).abs() < 4.0,
            "{}: predicted {predicted:.1}, measured {measured:.1}",
            medium.name
        );
    }
}

#[test]
fn in_water_they_hear_further() {
    let copies = |medium: Medium| {
        let mut a = Species::new(
            "a",
            Habitat {
                area: Rect::new(10.0, 10.0, 0.0, 0.0),
                individuals: 1,
                ..Default::default()
            },
        );
        a.voice.level = (60.0, 60.0);
        a.behaviour.talkative = 1.0;
        a.calls.rest = (1.0, 2.0);
        let mut b = Species::new(
            "b",
            Habitat {
                area: Rect::new(160.0, 160.0, 0.0, 0.0),
                individuals: 1,
                ..Default::default()
            },
        );
        b.behaviour = Behaviour {
            talkative: 1.0,
            mimicry_chance: 1.0,
            hearing: 30.0,
            ..Default::default()
        };
        let f = Forest::generate(ForestSettings {
            seed: 7,
            ..Default::default()
        });
        let listener = Vec2::new(100.0, 1.5);
        let st = SceneSettings {
            medium,
            zone: Zone::Around {
                pos: listener,
                radius: 1000.0,
            },
            ..Default::default()
        };
        let mut s = Scene::new(Life::new(f, vec![a, b]), st);
        s.run(60.0);
        s.stats.mimics
    };
    assert_eq!(
        copies(Medium::earth()),
        0,
        "60 dB at 150 m is below 30 dB in air"
    );
    assert!(copies(Medium::water()) > 5);
}

#[test]
fn same_seed_same_sound_with_the_whole_chain() {
    let run = |seed: u64| {
        let f = Forest::generate(ForestSettings {
            seed,
            ..Default::default()
        });
        let st = SceneSettings {
            medium: Medium::venus(),
            ..Default::default()
        };
        let mut s = Scene::new(Life::new(f, demo(0.3)), st).with_sound(SR);
        let n = s.step_samples();
        let (mut l, mut r) = (vec![0f32; n * 200], vec![0f32; n * 200]);
        for k in 0..200 {
            s.step_audio(&mut l[k * n..(k + 1) * n], &mut r[k * n..(k + 1) * n]);
        }
        l.iter().chain(&r).map(|x| x.to_bits()).collect::<Vec<_>>()
    };
    assert_eq!(run(4), run(4));
}
