//! kanopy command line.
//!
//!   kanopy bench [--seconds S]     how many sounding units one thread renders in real time
//!   kanopy demo [--out DIR]        short WAVs to hear the voice and the distance cues
//!   kanopy world [options]         simulate the demo forest, write a JSON snapshot to plot
//!       --seed N --density D --variation V --minutes M --scale K --trail S --out FILE
//!   kanopy world-bench [--scale K] how fast the world steps with many individuals
//!   kanopy score [options]         the demo forest's calls as text
//!       --seed N --density D --minutes M --scale K --free --x X --y Y --radius R --cap C
//!   kanopy listen [options]        the demo forest as a WAV (same options, plus:)
//!       --out FILE --medium earth|mars|titan|venus|water --box X0,X1,Y0,Y1 --ear fixed|adapting
//!       --no-reverb --reverb AMOUNT --no-bodies --underwater-stereo --compensation 0..1 --normalise
//!   kanopy scene-bench [--scale K] how fast calling and hearing run with many individuals

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
        Some("world") => {
            let f = |n: &str, d: f64| opt(n).map(|s| s.parse().expect(n)).unwrap_or(d);
            world(
                f("--seed", 1.0) as u64,
                f("--density", 0.5),
                f("--variation", 0.5),
                f("--minutes", 5.0),
                f("--scale", 1.0),
                f("--trail", 20.0),
                &PathBuf::from(opt("--out").unwrap_or_else(|| "out/world.json".into())),
            )
        }
        Some(cmd @ ("score" | "listen" | "scene-bench")) => {
            let f = |n: &str, d: f64| opt(n).map(|s| s.parse().expect(n)).unwrap_or(d);
            let o = SceneOpts {
                seed: f("--seed", 1.0) as u64,
                density: f("--density", 0.5),
                minutes: f("--minutes", if cmd == "score" { 1.0 } else { 2.0 }),
                scale: f("--scale", if cmd == "scene-bench" { 8.7 } else { 1.0 }),
                free: args.iter().any(|a| a == "--free"),
                x: f("--x", 100.0),
                y: f("--y", 1.5),
                radius: f("--radius", 150.0),
                cap: f("--cap", 600.0) as usize,
                medium: opt("--medium")
                    .map(|m| {
                        kanopy_core::propagation::Medium::preset(&m)
                            .expect("--medium: earth, mars, titan, venus or water")
                    })
                    .unwrap_or(kanopy_core::propagation::Medium::earth()),
                boxed: opt("--box").map(|b| {
                    let v: Vec<f64> = b
                        .split(',')
                        .map(|x| x.parse().expect("--box X0,X1,Y0,Y1"))
                        .collect();
                    kanopy_core::world::Rect::new(v[0], v[1], v[2], v[3])
                }),
                fixed_ear: opt("--ear").is_some_and(|e| e == "fixed"),
                reverb: if args.iter().any(|a| a == "--no-reverb") {
                    None
                } else {
                    Some(f("--reverb", 0.3) as f32)
                },
                bodies: !args.iter().any(|a| a == "--no-bodies"),
                underwater_stereo: args.iter().any(|a| a == "--underwater-stereo"),
                normalise: args.iter().any(|a| a == "--normalise"),
                compensation: f("--compensation", 1.0),
            };
            match cmd {
                "score" => score(&o),
                "listen" => listen(
                    &o,
                    &PathBuf::from(opt("--out").unwrap_or_else(|| "out/listen.wav".into())),
                ),
                _ => scene_bench(&o),
            }
        }
        Some("world-bench") => world_bench(
            opt("--scale")
                .map(|s| s.parse().expect("--scale"))
                .unwrap_or(8.7),
        ),
        _ => {
            eprintln!("usage: kanopy bench | demo | world | world-bench  (see the top of main.rs)");
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

fn world(
    seed: u64,
    density: f64,
    variation: f64,
    minutes: f64,
    scale: f64,
    trail: f64,
    out: &Path,
) {
    use kanopy_core::life::{Life, Mode};
    use kanopy_core::species::{demo as demo_species, Movement};
    use kanopy_core::world::{Forest, ForestSettings};
    use std::fmt::Write as _;

    let forest = Forest::generate(ForestSettings {
        seed,
        density,
        tree_variation: variation,
        liquid: false,
    });
    let mut life = Life::new(forest, demo_species(scale));
    let total = minutes * 60.0;
    let trail = trail.min(total);
    life.run(total - trail);
    // positions once a second over the last `trail` seconds
    let mut trails: Vec<Vec<(f64, f64)>> = vec![Vec::new(); life.animals.len()];
    for _ in 0..trail.round() as usize {
        for (t, a) in trails.iter_mut().zip(&life.animals) {
            t.push((a.pos.x, a.pos.y));
        }
        life.run(1.0);
    }
    let mut j = String::new();
    write!(j, "{{\"seed\":{seed},\"density\":{density},\"variation\":{variation},\"minutes\":{minutes},\"shrubs\":{:.3},\"trees\":[", life.forest.shrub_fullness).unwrap();
    for (i, t) in life.forest.trees.iter().enumerate() {
        write!(
            j,
            "{}[{:.2},{:.2},{:.2},{:.2}]",
            if i > 0 { "," } else { "" },
            t.x,
            t.height,
            t.crown_bottom,
            t.crown_half_width
        )
        .unwrap();
    }
    j.push_str("],\"species\":[");
    for (i, s) in life.species.iter().enumerate() {
        let m = match s.habitat.movement {
            Movement::Walker => "walker",
            Movement::Climber => "climber",
            Movement::Flyer => "flyer",
            Movement::Burrower => "burrower",
        };
        let a = s.habitat.area;
        write!(
            j,
            "{}{{\"name\":\"{}\",\"movement\":\"{m}\",\"area\":[{},{},{},{}]}}",
            if i > 0 { "," } else { "" },
            s.name,
            a.x0,
            a.x1,
            a.y0,
            a.y1
        )
        .unwrap();
    }
    j.push_str("],\"animals\":[");
    for (i, (a, tr)) in life.animals.iter().zip(&trails).enumerate() {
        let mode = match a.mode {
            Mode::Perched { .. } => "perched",
            Mode::Flying { .. } | Mode::Rising { .. } | Mode::ToPerch => "flying",
            Mode::Travel => "travel",
            _ => "still",
        };
        write!(
            j,
            "{}{{\"s\":{},\"g\":{},\"x\":{:.2},\"y\":{:.2},\"m\":\"{mode}\",\"t\":[",
            if i > 0 { "," } else { "" },
            a.species,
            a.group,
            a.pos.x,
            a.pos.y
        )
        .unwrap();
        for (k, (x, y)) in tr.iter().enumerate() {
            write!(j, "{}{:.1},{:.1}", if k > 0 { "," } else { "" }, x, y).unwrap();
        }
        j.push_str("]}");
    }
    j.push_str("]}");
    if let Some(dir) = out.parent() {
        std::fs::create_dir_all(dir).ok();
    }
    std::fs::write(out, j).expect("write json");
    let migrations: u32 = life.groups.iter().map(|g| g.migrations).sum();
    println!(
        "{} · {} trees · {} individuals in {} groups · {migrations} migrations · {:.0} min",
        out.display(),
        life.forest.trees.len(),
        life.animals.len(),
        life.groups.len(),
        minutes
    );
}

fn world_bench(scale: f64) {
    use kanopy_core::life::{Life, DT};
    use kanopy_core::species::demo as demo_species;
    use kanopy_core::world::{Forest, ForestSettings};
    let t0 = Instant::now();
    let mut life = Life::new(
        Forest::generate(ForestSettings::default()),
        demo_species(scale),
    );
    let setup = t0.elapsed().as_secs_f64();
    life.run(10.0); // settle
    let world_s = 60.0;
    let t1 = Instant::now();
    life.run(world_s);
    let cpu = t1.elapsed().as_secs_f64();
    println!(
        "kanopy world-bench · {} {} · {} individuals · step {} ms",
        std::env::consts::OS,
        std::env::consts::ARCH,
        life.animals.len(),
        DT * 1000.0
    );
    println!(
        "set-up {:.0} ms · {:.1} ms per second of forest time · {:.2} % of one core",
        setup * 1000.0,
        cpu / world_s * 1000.0,
        cpu / world_s * 100.0
    );
}

struct SceneOpts {
    seed: u64,
    density: f64,
    minutes: f64,
    scale: f64,
    free: bool,
    x: f64,
    y: f64,
    radius: f64,
    cap: usize,
    medium: kanopy_core::propagation::Medium,
    boxed: Option<kanopy_core::world::Rect>,
    fixed_ear: bool,
    reverb: Option<f32>,
    bodies: bool,
    underwater_stereo: bool,
    normalise: bool,
    compensation: f64,
}

fn make_scene(o: &SceneOpts) -> kanopy_core::scene::Scene {
    use kanopy_core::life::{Life, Zone};
    use kanopy_core::scene::{Scene, SceneSettings};
    use kanopy_core::species::demo as demo_species;
    use kanopy_core::world::{Forest, ForestSettings};
    let forest = Forest::generate(ForestSettings {
        seed: o.seed,
        density: o.density,
        ..Default::default()
    });
    let life = Life::new(forest, demo_species(o.scale));
    let listener = Vec2::new(o.x, o.y);
    let mut settings = SceneSettings {
        listener,
        zone: Zone::Around {
            pos: listener,
            radius: o.radius,
        },
        cap: o.cap,
        medium: o.medium,
        ear: if o.fixed_ear {
            kanopy_core::master::Ear::Fixed
        } else {
            kanopy_core::master::Ear::Adapting
        },
        reverb: o.reverb.is_some(),
        reverb_amount: o.reverb.unwrap_or(0.0),
        bodies_follow: o.bodies,
        underwater_stereo: o.underwater_stereo,
        medium_compensation: o.compensation,
        ..Default::default()
    };
    if let Some(r) = o.boxed {
        settings = settings.boxed(r);
    }
    if o.free {
        settings.key = None;
    }
    Scene::new(life, settings)
}

fn summary(sc: &kanopy_core::scene::Scene, seconds: f64) {
    let s = &sc.stats;
    println!(
        "{:.0} s · {} calls ({:.1}/s) · {} answers · {} signatures · {} mimics · {} harmonised · most at once {} · skipped: {} quiet, {} cap, {} inactive · {} hearings",
        seconds, s.calls, s.calls as f64 / seconds, s.answers, s.signatures, s.mimics, s.harmonised, s.max_calling, s.skipped_quiet, s.skipped_cap, s.skipped_inactive, s.heard
    );
}

fn score(o: &SceneOpts) {
    use kanopy_core::scale::note_name;
    use kanopy_core::scene::Source;
    let mut sc = make_scene(o);
    sc.keep_score = true;
    let seconds = o.minutes * 60.0;
    sc.run(seconds);
    let names: Vec<String> = sc.life.species.iter().map(|s| s.name.clone()).collect();
    println!(
        "kanopy score · seed {} · density {} · {} · listener ({}, {})",
        o.seed,
        o.density,
        if o.free {
            "free".to_string()
        } else {
            format!("{:?}", sc.settings.key.unwrap())
        },
        o.x,
        o.y
    );
    println!(
        "{:>8}  {:<17} {:>6} {:>5}  {:<26} {:>6}  notes",
        "time", "species", "#", "group", "kind", "length"
    );
    for c in &sc.score {
        let mut kind = match c.source {
            Source::Fresh => "fresh".to_string(),
            Source::Signature => "signature".to_string(),
            Source::Mimic { of } => format!("copy of {}", names[of as usize]),
        };
        if c.answer {
            kind = format!("answer · {kind}");
        }
        if c.harmonised {
            kind.push_str(" · harm.");
        }
        let notes: Vec<String> = c
            .motif
            .units
            .iter()
            .take(10)
            .map(|u| note_name(u.note))
            .collect();
        let more = if c.motif.units.len() > 10 { " …" } else { "" };
        println!(
            "{:>8.3}  {:<17} {:>6} {:>5}  {:<26} {:>5.2}s  {}{}",
            c.start,
            names[c.species as usize],
            c.who,
            c.group,
            kind,
            c.motif.duration(),
            notes.join(" "),
            more
        );
    }
    summary(&sc, seconds);
}

fn listen(o: &SceneOpts, out: &Path) {
    let mut sc = make_scene(o).with_sound(SR);
    let steps = (o.minutes * 60.0 / kanopy_core::life::DT).round() as usize;
    let n = sc.step_samples();
    let mut l = vec![0f32; steps * n];
    let mut r = vec![0f32; steps * n];
    let t = Instant::now();
    for k in 0..steps {
        sc.step_audio(&mut l[k * n..(k + 1) * n], &mut r[k * n..(k + 1) * n]);
    }
    let cpu = t.elapsed().as_secs_f64();
    let rms = (l.iter().chain(&r).map(|x| (*x as f64).powi(2)).sum::<f64>() / (2 * l.len()) as f64)
        .sqrt();
    let est = sc.estimate_db();
    let m = sc.mixer.as_ref().unwrap().master.as_ref().unwrap();
    let lim = m.limiters.iter().map(|x| x.min_gain).fold(1f32, f32::min);
    println!(
        "{} · estimated {:.1} dBFS · Ear started {:+.1} dB, ended {:+.1} dB · output {:.1} dBFS RMS · limiters: species {:.1} dB, final {:.1} dB",
        o.medium.name, est, m.start_db, m.gain_db, 20.0 * rms.max(1e-12).log10(), amp_to_db(lim), amp_to_db(m.last.min_gain)
    );
    if o.normalise {
        let peak = l
            .iter()
            .chain(&r)
            .fold(0f32, |a, x| a.max(x.abs()))
            .max(1e-9);
        let g = kanopy_core::db_to_amp(-1.0) / peak;
        l.iter_mut().chain(r.iter_mut()).for_each(|x| *x *= g);
        println!("normalised by {:+.1} dB", amp_to_db(g));
    }
    if let Some(dir) = out.parent() {
        std::fs::create_dir_all(dir).ok();
    }
    write(out, &l, &r);
    println!(
        "rendered {:.0} s in {:.1} s ({:.1}× real time)",
        o.minutes * 60.0,
        cpu,
        o.minutes * 60.0 / cpu
    );
    summary(&sc, o.minutes * 60.0);
}

fn scene_bench(o: &SceneOpts) {
    let mut sc = make_scene(o).with_sound(SR);
    let n = sc.step_samples();
    let mut l = vec![0f32; n];
    let mut r = vec![0f32; n];
    for _ in 0..200 {
        sc.step_audio(&mut l, &mut r); // settle 10 s
    }
    let steps = (o.minutes.min(1.0) * 60.0 / kanopy_core::life::DT) as usize;
    let t = Instant::now();
    for _ in 0..steps {
        l.fill(0.0);
        r.fill(0.0);
        sc.step_audio(&mut l, &mut r);
    }
    let cpu = t.elapsed().as_secs_f64();
    let secs = steps as f64 * kanopy_core::life::DT;
    println!(
        "kanopy scene-bench · {} {} · {} individuals · cap {} · whole forest active",
        std::env::consts::OS,
        std::env::consts::ARCH,
        sc.life.animals.len(),
        o.cap
    );
    println!(
        "{:.1} % of one core for life, calls, hearing and sound together",
        cpu / secs * 100.0
    );
    summary(&sc, sc.time());
}
