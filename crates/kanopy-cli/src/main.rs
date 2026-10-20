//! kanopy command line.
//!
//!   kanopy bench [--seconds S]     how many sounding units one thread renders in real time
//!   kanopy demo [--out DIR]        short WAVs to hear the voice and the distance cues

use kanopy_core::mixer::{Listener, Mixer};
use kanopy_core::propagation::Medium;
use kanopy_core::rng::Rng;
use kanopy_core::voice::UnitSpec;
use kanopy_core::wav::WavWriter;
use kanopy_core::wavetable::Wavetables;
use kanopy_core::{amp_to_db, Vec2};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

const SR: f32 = 48_000.0;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let opt = |name: &str| -> Option<String> {
        args.iter()
            .position(|a| a == name)
            .and_then(|i| args.get(i + 1).cloned())
    };
    match args.first().map(String::as_str) {
        Some("bench") => bench(
            opt("--seconds")
                .map(|s| s.parse().expect("--seconds"))
                .unwrap_or(10.0),
        ),
        Some("demo") => demo(&PathBuf::from(
            opt("--out").unwrap_or_else(|| "out/demo".into()),
        )),
        _ => {
            eprintln!("usage: kanopy bench [--seconds S] | kanopy demo [--out DIR]");
            std::process::exit(2);
        }
    }
}

fn random_unit(rng: &mut Rng) -> UnitSpec {
    let n0 = rng.range(45.0, 96.0) as f32;
    UnitSpec {
        note_start: n0,
        note_end: n0 + rng.range(-7.0, 7.0) as f32,
        dur: rng.range(0.2, 2.0) as f32,
        shape_start: rng.range(0.0, 3.0) as f32,
        shape_end: rng.range(0.0, 3.0) as f32,
        cutoff_start: rng.range(50.0, 3000.0) as f32,
        cutoff_end: rng.range(50.0, 3000.0) as f32,
        resonance: rng.f64() as f32,
        attack: rng.range(0.005, 0.1) as f32,
        release: rng.range(0.01, 0.3) as f32,
        level_db: rng.range(60.0, 85.0) as f32,
    }
}

fn random_place(rng: &mut Rng) -> (Vec2, Vec2) {
    let p = Vec2::new(rng.range(-80.0, 80.0), rng.range(0.0, 50.0));
    let v = if rng.f64() < 0.2 {
        Vec2::new(rng.range(-12.0, 12.0), rng.range(-2.0, 2.0))
    } else {
        Vec2::default()
    };
    (p, v)
}

/// Keep `n` units sounding for `seconds`, render on one thread in 512-frame chunks (as the live
/// app will), and time it.
fn bench_one(wt: &Arc<Wavetables>, n: usize, seconds: f64) -> (f64, f64) {
    let mut rng = Rng::new(7);
    let mut m = Mixer::new(
        Arc::clone(wt),
        Medium::earth(),
        Listener::still(Vec2::new(0.0, 1.5)),
    );
    let chunk = 512;
    let mut l = vec![0f32; chunk];
    let mut r = vec![0f32; chunk];
    let chunks = (seconds * SR as f64 / chunk as f64) as usize;
    let mut sounding = 0u64;
    let start = Instant::now();
    for _ in 0..chunks {
        while m.emitters.len() < n {
            let spec = random_unit(&mut rng);
            let (p, v) = random_place(&mut rng);
            // emitted so it arrives now: no units waiting in the air
            let d = Vec2::new(0.0, 1.5).dist(p);
            m.add(spec, m.time - d / 343.0, p, v);
        }
        let before = m.stats.rendered;
        l.fill(0.0);
        r.fill(0.0);
        m.render(&mut l, &mut r);
        sounding += m.stats.rendered - before;
        std::hint::black_box((&l, &r));
    }
    let cpu = start.elapsed().as_secs_f64();
    let blocks = chunks as f64 * (chunk / kanopy_core::BLOCK) as f64;
    (
        cpu / (chunks as f64 * chunk as f64 / SR as f64),
        sounding as f64 / blocks,
    )
}

fn bench(seconds: f64) {
    let wt = Arc::new(Wavetables::new(SR));
    println!(
        "kanopy bench · one thread · best of 2 runs · {} {} · 48 kHz · {seconds} s per row",
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    println!(
        "{:>8} {:>10} {:>12} {:>14}",
        "units", "sounding", "core load", "ns/unit-sample"
    );
    let mut best = (0usize, 0f64);
    for n in [50usize, 100, 200, 400, 800, 1600, 3200] {
        // best of two runs: a busy machine only ever makes things slower
        let (l1, avg) = bench_one(&wt, n, seconds);
        let (l2, _) = bench_one(&wt, n, seconds);
        let load = l1.min(l2);
        let ns = load * 1e9 / (SR as f64 * avg);
        println!("{n:>8} {avg:>10.0} {:>11.1}% {ns:>14.1}", load * 100.0);
        if load <= 0.5 {
            best = (n, load);
        }
        if load > 1.5 {
            break;
        }
    }
    let est = if best.1 > 0.0 {
        (best.0 as f64 * 0.5 / best.1) as usize
    } else {
        0
    };
    println!(
        "\nAt half of one core (room left for the forest and the app): about {est} sounding units."
    );
    println!(
        "Suggested calling cap for live play: {}",
        (est / 50).max(1) * 50
    );
}

fn write(path: &Path, l: &[f32], r: &[f32]) {
    let f = std::io::BufWriter::new(std::fs::File::create(path).expect("create wav"));
    let mut w = WavWriter::new(f, SR as u32, 24).expect("wav header");
    w.write(l, r).expect("wav write");
    w.finish().expect("wav finish");
    let peak = l.iter().chain(r).fold(0f32, |a, x| a.max(x.abs()));
    println!("{}  peak {:.1} dBFS", path.display(), amp_to_db(peak));
}

fn render(m: &mut Mixer, seconds: f32) -> (Vec<f32>, Vec<f32>) {
    let n = (seconds * SR) as usize;
    let mut l = vec![0f32; n];
    let mut r = vec![0f32; n];
    m.render(&mut l, &mut r);
    (l, r)
}

fn demo(dir: &Path) {
    std::fs::create_dir_all(dir).expect("out dir");
    let wt = Arc::new(Wavetables::new(SR));
    let me = Vec2::new(0.0, 1.5);
    let new = || Mixer::new(Arc::clone(&wt), Medium::earth(), Listener::still(me));
    let base = UnitSpec {
        attack: 0.05,
        release: 0.3,
        level_db: 80.0,
        cutoff_start: 20.0,
        cutoff_end: 20.0,
        ..Default::default()
    };

    // 1. Shape: sine → triangle → saw → square over 8 s, 2 m away
    let mut m = new();
    m.add(
        UnitSpec {
            note_start: 57.0,
            note_end: 57.0,
            dur: 8.0,
            shape_start: 0.0,
            shape_end: 3.0,
            ..base
        },
        0.0,
        Vec2::new(0.0, 3.5),
        Vec2::default(),
    );
    let (l, r) = render(&mut m, 8.5);
    write(&dir.join("01-shape-sweep.wav"), &l, &r);

    // 2. Body: a saw through the resonant high-pass, cutoff 40 Hz → 6 kHz
    let mut m = new();
    m.add(
        UnitSpec {
            note_start: 45.0,
            note_end: 45.0,
            dur: 8.0,
            shape_start: 2.0,
            shape_end: 2.0,
            cutoff_start: 40.0,
            cutoff_end: 6000.0,
            resonance: 0.8,
            ..base
        },
        0.0,
        Vec2::new(0.0, 3.5),
        Vec2::default(),
    );
    let (l, r) = render(&mut m, 8.5);
    write(&dir.join("02-body-sweep.wav"), &l, &r);

    // 3. Distance: the same three-unit call at 2, 8, 32 and 128 m, alternating sides
    let mut m = new();
    for (i, d) in [2.0f64, 8.0, 32.0, 128.0].iter().enumerate() {
        let side = if i % 2 == 0 { 1.0 } else { -1.0 };
        for u in 0..3 {
            let spec = UnitSpec {
                note_start: 84.0 - u as f32 * 2.0,
                note_end: 79.0,
                dur: 0.18,
                shape_start: 1.5,
                shape_end: 2.5,
                cutoff_start: 600.0,
                cutoff_end: 900.0,
                resonance: 0.5,
                attack: 0.005,
                release: 0.06,
                level_db: 85.0,
            };
            m.add(
                spec,
                i as f64 * 2.5 + u as f64 * 0.25,
                Vec2::new(side * d * 0.9, 1.5 + d * 0.43),
                Vec2::default(),
            );
        }
    }
    let (l, r) = render(&mut m, 11.0);
    write(&dir.join("03-distance.wav"), &l, &r);

    // 4. Fly-by: a calling flyer crosses from 150 m left to 150 m right at 25 m/s, 10 m up
    let mut m = new();
    m.add(
        UnitSpec {
            note_start: 76.0,
            note_end: 76.0,
            dur: 12.0,
            shape_start: 2.2,
            shape_end: 2.2,
            cutoff_start: 400.0,
            cutoff_end: 400.0,
            resonance: 0.3,
            attack: 0.5,
            release: 0.5,
            level_db: 90.0,
        },
        0.0,
        Vec2::new(-150.0, 11.5),
        Vec2::new(25.0, 0.0),
    );
    let (l, r) = render(&mut m, 12.5);
    write(&dir.join("04-flyby.wav"), &l, &r);

    // 5. Crowd: 1,500 random units over 20 s, 2–80 m away. No behaviour yet, just numbers.
    let mut m = new();
    let mut rng = Rng::new(11);
    for _ in 0..1500 {
        let mut spec = random_unit(&mut rng);
        spec.dur = rng.range(0.05, 0.6) as f32;
        let (p, v) = random_place(&mut rng);
        m.add(spec, rng.range(0.0, 19.0), p, v);
    }
    let (l, r) = render(&mut m, 20.0);
    write(&dir.join("05-crowd.wav"), &l, &r);
}
