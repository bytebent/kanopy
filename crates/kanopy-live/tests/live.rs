//! The live engine, silent (paced in real time, no sound card).

use kanopy_core::files::ForestFile;
use kanopy_core::species::demo;
use kanopy_live::{frame_header, Cmd, Live, PathKind, FRAME_ANIMAL, FRAME_HEADER};
use std::time::Duration;

fn small() -> ForestFile {
    ForestFile {
        name: "Small".into(),
        species: demo(0.05),
        ..ForestFile::demo()
    }
}

fn wait(ms: u64) {
    std::thread::sleep(Duration::from_millis(ms));
}

fn listener(live: &Live) -> (f32, f32) {
    let f = live.frame();
    let at = |i: usize| f32::from_le_bytes(f[i..i + 4].try_into().unwrap());
    (at(40), at(44))
}

#[test]
fn plays_in_real_time_and_pauses() {
    let live = Live::start(small(), false).unwrap();
    assert!(!live.audio);
    let (t0, _, playing, _, _) = frame_header(&live.frame()).unwrap();
    assert!(!playing && t0 == 0.0);
    live.send(Cmd::Play { on: true });
    wait(1500);
    let (t1, _, playing, _, active) = frame_header(&live.frame()).unwrap();
    assert!(playing && t1 > 1.0 && t1 < 1.8, "t = {t1}");
    assert!(active > 0);
    assert_eq!(
        live.frame().len(),
        FRAME_HEADER
            + FRAME_ANIMAL
                * small()
                    .species
                    .iter()
                    .map(|s| s.habitat.individuals)
                    .sum::<usize>()
    );
    live.send(Cmd::Play { on: false });
    wait(300);
    let (t2, ..) = frame_header(&live.frame()).unwrap();
    wait(500);
    let (t3, ..) = frame_header(&live.frame()).unwrap();
    assert!((t3 - t2).abs() < 1e-9, "paused: time stands still");
}

#[test]
fn the_listener_glides_and_wanders() {
    let live = Live::start(small(), false).unwrap();
    live.send(Cmd::Play { on: true });
    live.send(Cmd::Listener { x: 120.0, y: 10.0 });
    wait(300);
    let (x, _) = listener(&live);
    assert!(x > 100.0 && x < 120.0, "on its way: {x}");
    wait(1200);
    let (x, y) = listener(&live);
    assert!(
        (x - 120.0).abs() < 0.1 && (y - 10.0).abs() < 0.1,
        "arrived: {x}, {y}"
    );
    live.send(Cmd::Path {
        kind: PathKind::Wander,
        speed: 10.0,
        species: None,
    });
    wait(1500);
    let (x2, y2) = listener(&live);
    assert!((x2 - x).abs() + (y2 - y).abs() > 3.0, "wandering");
    // the forest file follows, for Save and Export
    let f = live.forest();
    assert!(matches!(
        f.listen,
        kanopy_core::files::ListenFile::Listener { .. }
    ));
}

#[test]
fn box_rebuild_load_and_live_settings() {
    let live = Live::start(small(), false).unwrap();
    live.send(Cmd::Play { on: true });
    live.send(Cmd::Hear {
        rect: Some([20.0, 60.0, 0.0, 20.0]),
    });
    wait(500);
    assert_eq!(live.frame()[17], 1, "box");
    assert!(matches!(
        live.forest().listen,
        kanopy_core::files::ListenFile::Box { .. }
    ));
    let (_, v0, ..) = frame_header(&live.frame()).unwrap();
    live.send(Cmd::Density { density: 0.9 });
    wait(500);
    let (t, v1, playing, ..) = frame_header(&live.frame()).unwrap();
    assert!(
        v1 > v0 && playing && t < 0.6,
        "a new forest, playing from its start"
    );
    assert_eq!(live.frame()[17], 1, "the box stays");
    let info: serde_json::Value = serde_json::from_str(&live.info()).unwrap();
    assert_eq!(info["forest"]["density"], 0.9);
    live.send(Cmd::Reverb { amount: None });
    live.send(Cmd::Name {
        name: "Renamed".into(),
    });
    wait(200);
    let f = live.forest();
    assert!(f.reverb.is_none() && f.name == "Renamed");
    let other = ForestFile {
        name: "Other".into(),
        medium: kanopy_core::files::MediumFile::Preset("water".into()),
        ..small()
    };
    live.send(Cmd::Load {
        forest: Box::new(other),
    });
    wait(300);
    let info: serde_json::Value = serde_json::from_str(&live.info()).unwrap();
    assert_eq!(info["forest"]["name"], "Other");
    assert_eq!(info["liquid"], true);
    live.send(Cmd::Key {
        key: Some(kanopy_core::files::KeyFile {
            root: "H".into(),
            scale: kanopy_core::scale::Scale::Major,
        }),
    });
    wait(200);
    assert!(live.error().unwrap().contains("unknown root"));
}

#[test]
fn export_renders_the_forest_as_it_is() {
    let live = Live::start(small(), false).unwrap();
    live.send(Cmd::Listener { x: 50.0, y: 5.0 });
    live.send(Cmd::Play { on: true });
    wait(1500);
    let dir = std::env::temp_dir().join("kanopy-live-test");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("export.wav");
    live.export(path.display().to_string(), 0.05);
    for _ in 0..200 {
        if !live.export_state().running {
            break;
        }
        wait(50);
    }
    let s = live.export_state();
    assert_eq!(s.result, Some(Ok(path.display().to_string())));
    let inside = ForestFile::from_wav(&path).unwrap();
    match inside.listen {
        kanopy_core::files::ListenFile::Listener { x, y, .. } => {
            assert!((x - 50.0).abs() < 0.5 && (y - 5.0).abs() < 0.5)
        }
        _ => panic!("listener expected"),
    }
}

#[test]
fn species_apply_checks_then_rebuilds() {
    let live = Live::start(small(), false).unwrap();
    let v0 = frame_header(&live.frame()).map(|_| ()).is_some();
    assert!(v0);
    let version = |l: &Live| u32::from_le_bytes(l.frame()[12..16].try_into().unwrap());
    let start = version(&live);

    live.send(Cmd::Species { species: vec![] });
    wait(300);
    assert!(live.error().unwrap().contains("at least one"));

    let mut many = demo(0.05);
    many[0].habitat.individuals = 30_000;
    live.send(Cmd::Species { species: many });
    wait(300);
    assert!(live.error().unwrap().contains("more than"));
    assert_eq!(
        version(&live),
        start,
        "a refused change leaves the forest alone"
    );

    let mut sp = demo(0.05);
    sp[0] = sp[0].clone().coloured("#963d97");
    sp[1] = sp[1].clone().coloured("#963d97");
    sp.truncate(3);
    live.send(Cmd::Species { species: sp });
    wait(600);
    assert!(live.error().is_none());
    assert_ne!(version(&live), start);
    let f = live.forest();
    assert_eq!(f.species.len(), 3);
    assert_eq!(f.species[0].colour.as_deref(), Some("#963d97"));
    assert_eq!(
        f.species[1].colour, f.species[0].colour,
        "two species may share a colour"
    );
}
