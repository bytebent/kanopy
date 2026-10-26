//! The mysterious clicks: the forest's sound path checked against a control that can't click.
//!
//! The control renders every unit on its own, the way the Designer does (one voice, from its
//! first sample to its last, nothing shared), and adds them up. The forest renders the same
//! units through its mixer (blocks, lanes, partial blocks) and output chain. Wherever the two
//! differ, the forest's path has done something to the sound.

use kanopy_core::files::ForestFile;
use kanopy_core::life::DT;
use kanopy_core::master::{Limiter, LOOK};
use kanopy_core::mixer::Hear;
use kanopy_core::propagation::pan_value;
use kanopy_core::scene::Scene;
use kanopy_core::voice::Voice;
use kanopy_core::wavetable::Wavetables;
use kanopy_core::world::Rect;
use kanopy_core::{amp_to_db, db_to_amp, FULL_SCALE_SPL};

const SR: f32 = 48_000.0;

fn example(name: &str) -> ForestFile {
    let p = format!(
        "{}/../../examples/forests/{name}.kanopy-forest.json",
        env!("CARGO_MANIFEST_DIR")
    );
    ForestFile::load(std::path::Path::new(&p)).unwrap()
}

/// The forest in a box bigger than the world: no distance, no edge fades; everything centred.
/// Mixer only, no output chain. Returns (mixer output L, the control, units).
fn forest_and_control(name: &str, seconds: f64) -> (Vec<f32>, Vec<f32>, usize) {
    let f = example(name);
    let mut scene: Scene = f.scene().unwrap();
    scene.settings = scene.settings.boxed(Rect::new(-50.0, 250.0, -50.0, 100.0));
    let mut scene = scene.with_raw_sound(SR);
    {
        let m = scene.mixer.as_mut().unwrap();
        m.width = 0.0;
        m.record = Some(Vec::new());
    }
    let n = scene.step_samples();
    let steps = (seconds / DT).round() as usize;
    let mut out = Vec::with_capacity(n * steps);
    let (mut l, mut r) = (vec![0f32; n], vec![0f32; n]);
    for _ in 0..steps {
        l.fill(0.0);
        r.fill(0.0);
        scene.step_audio(&mut l, &mut r);
        out.extend_from_slice(&l);
    }
    let m = scene.mixer.as_ref().unwrap();
    assert!(matches!(m.hear, Hear::Box(_)));
    let offset = m.medium.level_offset();
    let units = m.record.clone().unwrap();
    // the control: each unit alone, from its first sample
    let wt = Wavetables::new(SR);
    let centre = pan_value(0.0).0;
    let mut control = vec![0f32; out.len()];
    let mut buf = vec![0f32; 64];
    for (spec, emit, _) in &units {
        let amp =
            db_to_amp((spec.level_db as f64 - FULL_SCALE_SPL as f64 + offset) as f32) * centre;
        let mut v = Voice::new(*spec, SR);
        let mut i = (emit * SR as f64).ceil() as usize;
        while i < control.len() {
            // the mixer's block grid: 64-sample blocks from the start of each step
            let into = i % n;
            let next_edge = (i - into) + ((into / 64 + 1) * 64).min(n);
            let k = (next_edge - i).min(control.len() - i);
            let got = v.render(&wt, &mut buf[..k], 1.0);
            for (c, x) in control[i..i + k].iter_mut().zip(&buf[..k]) {
                *c += x * amp;
            }
            i += k;
            if got < k {
                break;
            }
        }
    }
    (out, control, units.len())
}

/// Largest difference, dB below the signal's peak, and where.
fn compare(a: &[f32], b: &[f32]) -> (f32, f64) {
    let peak = b.iter().fold(0f32, |m, x| m.max(x.abs()));
    let (i, d) = a
        .iter()
        .zip(b)
        .map(|(x, y)| (x - y).abs())
        .enumerate()
        .fold((0, 0f32), |m, (i, d)| if d > m.1 { (i, d) } else { m });
    (amp_to_db(d / peak), i as f64 / SR as f64)
}

/// Runs of 16 or more samples that don't move in `a` while `b` does: missing sound.
fn flat_runs(a: &[f32], b: &[f32]) -> usize {
    let (mut runs, mut len) = (0, 0);
    for i in 1..a.len() {
        if a[i] == a[i - 1] && (b[i] - b[i - 1]).abs() > 1e-4 {
            len += 1;
            if len == 16 {
                runs += 1;
            }
        } else {
            len = 0;
        }
    }
    runs
}

#[test]
fn the_mixer_matches_units_rendered_alone() {
    for name in ["sparse-forest", "canopy-box"] {
        let (forest, control, units) = forest_and_control(name, 20.0);
        let (db, at) = compare(&forest, &control);
        if let Some(d) = std::env::var_os("KANOPY_DUMP") {
            let raw = |v: &[f32]| v.iter().flat_map(|x| x.to_le_bytes()).collect::<Vec<u8>>();
            let d = std::path::PathBuf::from(d);
            std::fs::write(d.join(format!("{name}-forest.f32")), raw(&forest)).unwrap();
            std::fs::write(d.join(format!("{name}-control.f32")), raw(&control)).unwrap();
        }
        println!("{name}: {units} units, largest difference {db:.1} dB below peak at {at:.4} s");
        assert!(units > 100);
        // units are sampled at slightly different moments (the mixer re-times them per block),
        // so the two drift apart a little, smoothly; a gap or a jump would come close to 0 dB
        assert!(
            db < -24.0,
            "{name}: the mixer differs from the control by {db:.1} dB at {at:.4} s"
        );
        assert_eq!(
            flat_runs(&forest, &control),
            0,
            "{name}: stretches gone flat that the control has sound in"
        );
    }
}

#[test]
fn a_limiter_at_rest_only_delays() {
    // a smooth tone fed in the forest's step sizes (2400 samples: 37½ blocks), and in odd sizes
    let tone: Vec<f32> = (0..48_000)
        .map(|i| 0.3 * (i as f32 * 0.0731).sin())
        .collect();
    for sizes in [&[2400usize][..], &[64], &[32, 64, 7, 100, 1], &[2205]] {
        let mut lim = Limiter::new(-3.0, SR);
        let (mut l, mut r) = (tone.clone(), tone.clone());
        let (mut at, mut k) = (0, 0);
        while at < l.len() {
            let n = sizes[k % sizes.len()].min(l.len() - at);
            lim.process(&mut l[at..at + n], &mut r[at..at + n]);
            at += n;
            k += 1;
        }
        let worst = (LOOK..l.len())
            .map(|i| (l[i] - tone[i - LOOK]).abs())
            .fold(0f32, f32::max);
        assert!(
            worst < 1e-6,
            "blocks {sizes:?}: output differs from the delayed input by {worst}"
        );
        assert_eq!(lim.min_gain, 1.0);
    }
}

#[test]
fn a_working_limiter_holds_without_gaps() {
    // loud bursts: it must hold the threshold and never drop samples
    let x: Vec<f32> = (0..48_000)
        .map(|i| {
            let burst = if (i / 3000) % 2 == 0 { 2.0 } else { 0.2 };
            burst * (i as f32 * 0.05).sin()
        })
        .collect();
    let mut lim = Limiter::new(-3.0, SR);
    let (mut l, mut r) = (x.clone(), x.clone());
    let mut at = 0;
    while at < l.len() {
        let n = 2400.min(l.len() - at);
        lim.process(&mut l[at..at + n], &mut r[at..at + n]);
        at += n;
    }
    let thr = db_to_amp(-3.0);
    assert!(l.iter().all(|v| v.abs() <= thr * 1.0001));
    // the output follows the input's shape: a sign change wherever the delayed input changes sign
    let flips = (LOOK + 1..l.len())
        .filter(|&i| (x[i - LOOK] > 1e-3) != (l[i] > 1e-9) && x[i - LOOK].abs() > 1e-3)
        .count();
    assert_eq!(flips, 0, "{flips} samples went missing or changed sign");
}

/// The whole output chain, limiters in, against the same with limiters out. While they're idle
/// the two must be the same sound, just 2 × LOOK samples later (species limiter, then final).
#[test]
fn the_output_chain_only_delays_while_limiters_rest() {
    for (name, minutes) in [
        ("sparse-forest", 0.5),
        ("canopy-box", 0.5),
        ("sunken-kelp", 0.5),
    ] {
        let f = example(name);
        let run = |bypass: bool| {
            let mut s = f.scene().unwrap().with_sound(SR);
            s.mixer.as_mut().unwrap().master.as_mut().unwrap().bypass = bypass;
            let n = s.step_samples();
            let mut out = Vec::new();
            let (mut l, mut r) = (vec![0f32; n], vec![0f32; n]);
            for _ in 0..((minutes * 60.0 / DT) as usize) {
                l.fill(0.0);
                r.fill(0.0);
                s.step_audio(&mut l, &mut r);
                out.extend_from_slice(&l);
            }
            let m = s.mixer.as_ref().unwrap().master.as_ref().unwrap();
            let idle = m
                .limiters
                .iter()
                .chain([&m.last])
                .all(|x| x.min_gain == 1.0);
            (out, idle)
        };
        let (with, idle) = run(false);
        let (without, _) = run(true);
        let d = 2 * LOOK;
        let peak = without.iter().fold(0f32, |m, x| m.max(x.abs()));
        let worst = (d..with.len())
            .map(|i| (with[i] - without[i - d]).abs())
            .fold(0f32, f32::max);
        let db = amp_to_db(worst / peak);
        println!("{name}: limiters idle {idle}; largest difference {db:.1} dB below peak");
        if idle {
            assert!(
                db < -80.0,
                "{name}: the output chain changed the sound by {db:.1} dB"
            );
        }
        assert_eq!(
            flat_runs(&with[d..], &without[..without.len() - d]),
            0,
            "{name}: gaps"
        );
    }
}
