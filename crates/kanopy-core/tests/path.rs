//! The listener's path in renders: Wander and Follow, smooth, repeatable, underground or not.

use kanopy_core::files::ForestFile;
use kanopy_core::life::{Life, DT};
use kanopy_core::path::{PathFile, PathKind, Walker};
use kanopy_core::render::render;
use kanopy_core::world::{Forest, ForestSettings};
use kanopy_core::{species::demo, Vec2};
use std::io::Cursor;

fn life() -> Life {
    Life::new(
        Forest::generate(ForestSettings {
            seed: 4,
            density: 0.5,
            tree_variation: 0.5,
            liquid: false,
        }),
        demo(0.1),
    )
}

/// Walk for `secs`, returning every (pos, vel).
fn walk(mut w: Walker, secs: f64) -> Vec<(Vec2, Vec2)> {
    let mut l = life();
    let mut out = Vec::new();
    for _ in 0..(secs / DT) as usize {
        out.push(w.step(&l));
        l.step();
    }
    out
}

#[test]
fn wander_repeats_moves_smoothly_and_keeps_its_speed() {
    let w = || Walker::new(PathKind::Wander, 2.0, None, false, Vec2::new(100.0, 1.5), 7);
    let a = walk(w(), 600.0);
    assert_eq!(a, walk(w(), 600.0), "same seed, same walk");
    assert_ne!(
        a,
        walk(
            Walker::new(PathKind::Wander, 2.0, None, false, Vec2::new(100.0, 1.5), 8),
            600.0
        )
    );
    let mut fastest = 0.0f64;
    let mut far = 0.0f64;
    for k in 1..a.len() {
        let (p0, v0) = a[k - 1];
        let (p1, v1) = a[k];
        // each step starts where the last one's motion took it
        assert!(
            p1.dist(p0.add_scaled(v0, DT)) < 1e-9
                || v0 == Vec2::default()
                || p1.x <= 0.0
                || p1.x >= 200.0
        );
        // no sudden turns: velocity changes by at most a tenth of the speed limit per step... and a little
        let dv = ((v1.x - v0.x).powi(2) + (v1.y - v0.y).powi(2)).sqrt();
        assert!(
            dv <= 0.1 * 2.0 * 2.0 + 1e-9,
            "a jump of {dv} m/s at {} s",
            k as f64 * DT
        );
        fastest = fastest.max((v1.x * v1.x + v1.y * v1.y).sqrt());
        far = far.max(p1.dist(a[0].0));
        assert!(p1.y >= 0.0, "went underground at {} s", k as f64 * DT);
    }
    assert!(
        fastest <= 2.0 + 1e-9 && fastest > 1.8,
        "top speed {fastest}"
    );
    assert!(far > 40.0, "it gets around: {far} m");
}

#[test]
fn underground_only_when_allowed() {
    let below = |on: bool| {
        walk(
            Walker::new(PathKind::Wander, 3.0, None, on, Vec2::new(100.0, 5.0), 3),
            1800.0,
        )
        .iter()
        .filter(|(p, _)| p.y < 0.0)
        .count()
    };
    assert_eq!(below(false), 0);
    assert!(
        below(true) > 100,
        "with Underground on it goes below sometimes"
    );
}

#[test]
fn follow_stays_near_its_species() {
    let l = life();
    let si = l
        .species
        .iter()
        .position(|s| s.name == "Lower climbers")
        .unwrap();
    let p = PathFile {
        kind: PathKind::Follow,
        speed: 3.0,
        follow: Some("lower climbers".into()),
        underground: false,
    };
    let w = Walker::from_file(&p, &l, Vec2::new(10.0, 1.5), 1).unwrap();
    let mut l = life();
    let mut w = w;
    let mut pos = Vec2::default();
    for _ in 0..(120.0 / DT) as usize {
        pos = w.step(&l).0;
        l.step();
    }
    let nearest = l
        .animals
        .iter()
        .filter(|a| a.species as usize == si)
        .map(|a| a.pos.dist(pos))
        .fold(f64::MAX, f64::min);
    assert!(nearest < 8.0, "{nearest} m from the nearest lower climber");
    let bad = PathFile {
        follow: Some("nobody".into()),
        ..p
    };
    assert!(Walker::from_file(&bad, &l, pos, 1)
        .unwrap_err()
        .contains("nobody"));
}

#[test]
fn a_render_wanders_and_repeats() {
    let path = format!(
        "{}/../../examples/forests/sparse-forest.kanopy-forest.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let still = ForestFile::load(std::path::Path::new(&path)).unwrap();
    let mut wander = still.clone();
    wander.path = PathFile {
        kind: PathKind::Wander,
        speed: 5.0,
        follow: None,
        underground: false,
    };
    let opts = kanopy_core::files::RenderFile {
        minutes: 0.25,
        ..still.render.clone()
    };
    let go = |f: &ForestFile| {
        render(f, &opts, Cursor::new(Vec::new()), |_| {})
            .unwrap()
            .0
            .into_inner()
    };
    let (a, b, c) = (go(&wander), go(&wander), go(&still));
    assert_eq!(a, b, "same file, same walk, same bytes");
    assert_ne!(a, c);
    // the WAV carries the path
    let dir = std::env::temp_dir().join(format!("kanopy-path-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let wav = dir.join("w.wav");
    std::fs::write(&wav, &a).unwrap();
    assert_eq!(ForestFile::from_wav(&wav).unwrap().path, wander.path);
    // and a file without a path still reads (Still)
    let old = std::fs::read_to_string(&path).unwrap();
    assert!(!old.contains("\"path\""));
    assert_eq!(
        ForestFile::from_json(&old).unwrap().path.kind,
        PathKind::Still
    );
}
