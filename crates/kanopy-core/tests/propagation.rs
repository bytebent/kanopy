//! Spreading, pan, delay, Doppler, culling and repeatability through the mixer.

mod common;
use common::*;
use kanopy_core::mixer::{Listener, Mixer};
use kanopy_core::propagation::*;
use kanopy_core::rng::Rng;
use kanopy_core::voice::UnitSpec;
use kanopy_core::wavetable::Wavetables;
use kanopy_core::Vec2;
use std::sync::Arc;

fn mixer() -> Mixer {
    Mixer::new(
        Arc::new(Wavetables::new(SR)),
        Medium::earth(),
        Listener::still(Vec2::new(0.0, 1.5)),
    )
}

fn run(m: &mut Mixer, seconds: f32) -> (Vec<f32>, Vec<f32>) {
    let n = (seconds * SR) as usize;
    let mut l = vec![0f32; n];
    let mut r = vec![0f32; n];
    m.render(&mut l, &mut r);
    (l, r)
}

fn tone_spec(note: f32, dur: f32) -> UnitSpec {
    UnitSpec {
        note_start: note,
        note_end: note,
        dur,
        attack: 0.005,
        release: 0.005,
        ..Default::default()
    }
}

#[test]
fn spreading_is_6db_per_doubling() {
    let g = db(spreading(20.0) / spreading(10.0));
    assert!((g + 6.02).abs() < 0.01);
    assert_eq!(spreading(0.2), 1.0);
}

#[test]
fn pan_is_equal_power_and_centred_above() {
    let me = Vec2::new(0.0, 0.0);
    for s in [
        Vec2::new(0.0, 10.0),
        Vec2::new(10.0, 0.0),
        Vec2::new(-3.0, 4.0),
    ] {
        let (l, r) = pan(me, s);
        assert!((l * l + r * r - 1.0).abs() < 1e-5);
    }
    let (l, r) = pan(me, Vec2::new(0.0, 10.0));
    assert!((l - r).abs() < 1e-6);
    let (l, r) = pan(me, Vec2::new(10.0, 0.0));
    assert!(l < 1e-6 && (r - 1.0).abs() < 1e-6);
}

#[test]
fn arrives_after_the_travel_time() {
    let mut m = mixer();
    m.add(
        tone_spec(69.0, 0.2),
        0.0,
        Vec2::new(34.3, 1.5),
        Vec2::default(),
    );
    let (l, r) = run(&mut m, 0.3);
    let first = l
        .iter()
        .zip(&r)
        .position(|(a, b)| a.abs() + b.abs() > 0.0)
        .unwrap();
    let expect = (0.1 * SR) as usize;
    assert!(
        (first as i64 - expect as i64).abs() <= 2,
        "first sound at {first}, expected {expect}"
    );
}

#[test]
fn level_falls_6db_per_doubling_through_the_mixer() {
    let level_at = |d: f64| {
        let mut m = mixer();
        m.add(
            tone_spec(55.0, 1.0),
            0.0,
            Vec2::new(0.0, 1.5 + d),
            Vec2::default(),
        );
        let (l, r) = run(&mut m, 1.2);
        let s = (0.4 * SR) as usize;
        let mono: Vec<f32> = l[s..s + 24_000]
            .iter()
            .zip(&r[s..])
            .map(|(a, b)| a + b)
            .collect();
        db(rms(&mono))
    };
    let diff = level_at(20.0) - level_at(10.0);
    assert!((diff + 6.0).abs() < 0.3, "{diff}");
}

#[test]
fn doppler_raises_an_approaching_call() {
    let c = Medium::earth().speed;
    let v = 30.0;
    let mut m = mixer();
    // starts 300 m to the right, flies left towards the listener at 30 m/s
    m.add(
        tone_spec(69.0, 4.0),
        0.0,
        Vec2::new(300.0, 1.5),
        Vec2::new(-v, 0.0),
    );
    let (_, r) = run(&mut m, 2.0);
    let s = (1.2 * SR) as usize;
    let f = zc_freq(&r[s..s + 19_200], SR);
    let expect = 440.0 * (c / (c - v)) as f32;
    assert!((f - expect).abs() < 1.0, "heard {f}, expected {expect}");
}

#[test]
fn emission_time_for_a_still_source() {
    let tau = emission_time(
        2.0,
        Vec2::new(0.0, 0.0),
        Vec2::new(343.0, 0.0),
        Vec2::default(),
        &Medium::earth(),
    );
    assert!((tau - 1.0).abs() < 1e-9);
}

#[test]
fn inaudible_units_are_not_rendered() {
    let mut m = mixer();
    let spec = UnitSpec {
        level_db: 10.0,
        ..tone_spec(69.0, 0.5)
    };
    m.add(spec, 0.0, Vec2::new(0.0, 101.5), Vec2::default()); // 10 dB SPL at 100 m: −130 dBFS
    let (l, r) = run(&mut m, 1.0);
    assert!(l.iter().chain(&r).all(|x| *x == 0.0));
    assert_eq!(m.stats.rendered, 0);
    assert!(m.stats.culled > 0);
    assert!(m.emitters.is_empty(), "finished units are dropped");
}

fn busy_scene(seed: u64) -> (Vec<f32>, Vec<f32>) {
    busy_scene_with(seed, false)
}

fn busy_scene_with(seed: u64, single: bool) -> (Vec<f32>, Vec<f32>) {
    let mut rng = Rng::new(seed);
    let mut m = mixer();
    m.single = single;
    for _ in 0..200 {
        let spec = UnitSpec {
            note_start: rng.range(40.0, 96.0) as f32,
            note_end: rng.range(40.0, 96.0) as f32,
            dur: rng.range(0.05, 2.0) as f32,
            shape_start: rng.range(0.0, 3.0) as f32,
            shape_end: rng.range(0.0, 3.0) as f32,
            cutoff_start: rng.range(20.0, 4000.0) as f32,
            cutoff_end: rng.range(20.0, 4000.0) as f32,
            resonance: rng.f64() as f32,
            level_db: rng.range(50.0, 90.0) as f32,
            ..Default::default()
        };
        let p = Vec2::new(rng.range(-100.0, 100.0), rng.range(0.0, 50.0));
        let v = Vec2::new(rng.range(-15.0, 15.0), rng.range(-3.0, 3.0));
        m.add(spec, rng.range(0.0, 3.0), p, v);
    }
    run(&mut m, 4.0)
}

#[test]
fn same_seed_same_bits() {
    let a = busy_scene(42);
    let b = busy_scene(42);
    assert!(a
        .0
        .iter()
        .zip(&b.0)
        .all(|(x, y)| x.to_bits() == y.to_bits()));
    assert!(a
        .1
        .iter()
        .zip(&b.1)
        .all(|(x, y)| x.to_bits() == y.to_bits()));
    let c = busy_scene(43);
    assert!(a.0 != c.0);
    assert!(a.0.iter().chain(&a.1).all(|x| x.is_finite()));
}

#[test]
fn side_by_side_matches_one_by_one() {
    let (al, ar) = busy_scene_with(5, false);
    let (bl, br) = busy_scene_with(5, true);
    let peak = al.iter().fold(0f32, |a, x| a.max(x.abs()));
    let err = al
        .iter()
        .zip(&bl)
        .chain(ar.iter().zip(&br))
        .fold(0f32, |a, (x, y)| a.max((x - y).abs()));
    assert!(peak > 1e-3);
    assert!(
        err < peak * 1e-4,
        "max difference {err} against peak {peak}"
    );
}
