//! Batch 05: forest and species files, and rendering to WAV.

use kanopy_core::files::*;
use kanopy_core::render::{render, SAMPLE_RATE};
use kanopy_core::species::*;
use kanopy_core::wav::read_chunk;
use std::io::Cursor;
use std::path::Path;

fn small() -> ForestFile {
    ForestFile {
        name: "Small".into(),
        species: demo(0.05),
        ..ForestFile::demo()
    }
}

fn wav_of(f: &ForestFile, minutes: f64, fade: f64, bits: u16) -> Vec<u8> {
    let opts = RenderFile {
        minutes,
        fade_in: fade,
        fade_out: fade,
        bits,
    };
    render(f, &opts, Cursor::new(Vec::new()), |_| {})
        .unwrap()
        .0
        .into_inner()
}

fn u32_at(b: &[u8], i: usize) -> u32 {
    u32::from_le_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]])
}

fn data_of(b: &[u8]) -> &[u8] {
    let i = b.windows(4).position(|w| w == b"data").unwrap();
    let n = u32_at(b, i + 4) as usize;
    &b[i + 8..i + 8 + n]
}

#[test]
fn species_files_round_trip() {
    for sp in demo(1.0) {
        let back = SpeciesFile::from_json(&SpeciesFile::to_json(&sp)).unwrap();
        assert_eq!(back, sp);
    }
}

#[test]
fn forest_files_round_trip() {
    let f = ForestFile::demo();
    assert_eq!(ForestFile::from_json(&f.to_json()).unwrap(), f);
    let dir = std::env::temp_dir().join("kanopy-files-test");
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join("t.kanopy-forest.json");
    f.save(&p).unwrap();
    assert_eq!(ForestFile::load(&p).unwrap(), f);
}

#[test]
fn example_files_load() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    let mut n = 0;
    for dir in ["forests", "species"] {
        for e in std::fs::read_dir(root.join(dir)).unwrap() {
            let p = e.unwrap().path();
            if dir == "forests" {
                let f = ForestFile::load(&p).unwrap();
                f.scene().unwrap();
            } else {
                SpeciesFile::load(&p).unwrap();
            }
            n += 1;
        }
    }
    assert!(n >= 13);
}

#[test]
fn a_file_only_needs_what_differs() {
    let f = ForestFile::from_json(
        r#"{"format": "kanopy-forest", "version": 1, "species": [{"name": "Lone"}]}"#,
    )
    .unwrap();
    assert_eq!(f.species[0].name, "Lone");
    assert_eq!(f.species[0].voice, Voice::default());
    assert_eq!(f.cap, 600);
    f.scene().unwrap();
    let custom = ForestFile::from_json(
        r#"{"format": "kanopy-forest", "version": 1, "medium": {"speed": 500, "density": 3, "absorption": 1e-9},
            "key": null, "listen": {"mode": "box", "x0": 10, "x1": 50, "y0": 0, "y1": 20},
            "species": [{"name": "a", "habitat": {"movement": "flyer", "flight": {"perch": "never"}}}]}"#,
    )
    .unwrap();
    let st = custom.settings().unwrap();
    assert_eq!(st.medium.speed, 500.0);
    assert!(st.key.is_none());
    assert!(matches!(st.hear, kanopy_core::mixer::Hear::Box(_)));
    assert_eq!(custom.species[0].habitat.flight.perch, Perch::Never);
}

#[test]
fn bad_files_say_what_is_wrong() {
    let err = |j: &str| ForestFile::from_json(j).unwrap_err();
    assert!(
        err(r#"{"format": "kanopy-species", "version": 1}"#).contains("not a kanopy-forest file")
    );
    assert!(err(r#"{"format": "kanopy-forest", "version": 9}"#).contains("newer kanopy"));
    assert!(
        err(r#"{"format": "kanopy-forest", "version": 1, "medium": "jupiter"}"#)
            .contains("unknown medium")
    );
    assert!(err(
        r#"{"format": "kanopy-forest", "version": 1, "key": {"root": "H", "scale": "minor"}}"#
    )
    .contains("unknown root"));
    assert!(err(r#"{"format": "kanopy-forest", "version": 1, "species": [{"habitat": {"flight": {"perch": "roof"}}}]}"#).contains("unknown perch"));
    assert!(
        err(r#"{"format": "kanopy-forest", "version": 1, "density": "lots"}"#)
            .contains("forest file")
    );
}

#[test]
fn roots() {
    for (s, n) in [
        ("C", 0),
        ("c#", 1),
        ("Db", 1),
        ("Bb", 10),
        ("B#", 0),
        ("F##", 7),
        ("A", 9),
    ] {
        assert_eq!(parse_root(s).unwrap(), n, "{s}");
    }
    assert!(parse_root("X").is_err());
}

#[test]
fn renders_are_repeatable_and_complete() {
    let f = small();
    let a = wav_of(&f, 0.25, 1.0, 24);
    assert_eq!(
        a,
        wav_of(&f, 0.25, 1.0, 24),
        "same file, same seed: same bytes"
    );
    let mut g = f.clone();
    g.seed = 2;
    assert_ne!(data_of(&a), data_of(&wav_of(&g, 0.25, 1.0, 24)));
    // header: RIFF size, 48 kHz stereo 24-bit, exact length
    assert_eq!(&a[0..4], b"RIFF");
    assert_eq!(u32_at(&a, 4) as usize, a.len() - 8);
    assert_eq!(u32_at(&a, 24), SAMPLE_RATE);
    let data = data_of(&a);
    assert_eq!(data.len(), 15 * SAMPLE_RATE as usize * 6);
    // fades: silent at both ends
    assert!(data[..6].iter().all(|b| *b == 0) && data[data.len() - 6..].iter().all(|b| *b == 0));
    assert!(data.iter().any(|b| *b != 0));
    // 16-bit is two bytes a sample
    assert_eq!(
        data_of(&wav_of(&f, 0.25, 1.0, 16)).len(),
        15 * SAMPLE_RATE as usize * 4
    );
}

#[test]
fn a_render_carries_its_forest() {
    let f = small();
    let a = wav_of(&f, 0.1, 0.5, 24);
    let body = read_chunk(Cursor::new(&a), b"knpy").unwrap().unwrap();
    let inside = ForestFile::from_json(&String::from_utf8(body).unwrap()).unwrap();
    assert_eq!(inside.species, f.species);
    assert_eq!(
        inside.render,
        RenderFile {
            minutes: 0.1,
            fade_in: 0.5,
            fade_out: 0.5,
            bits: 24
        }
    );
    // rendering what's inside gives the same file
    let again = render(&inside, &inside.render, Cursor::new(Vec::new()), |_| {})
        .unwrap()
        .0
        .into_inner();
    assert_eq!(again, a);
    assert!(read_chunk(Cursor::new(&a), b"LIST")
        .unwrap()
        .unwrap()
        .windows(6)
        .any(|w| w == b"kanopy"));
}

#[test]
fn renders_stay_within_full_scale() {
    for medium in ["earth", "water", "venus"] {
        let f = ForestFile {
            medium: MediumFile::Preset(medium.into()),
            species: demo(0.3),
            ..ForestFile::demo()
        };
        let opts = RenderFile {
            minutes: 0.5,
            fade_in: 0.0,
            fade_out: 0.0,
            bits: 24,
        };
        let (_, rep) = render(&f, &opts, Cursor::new(Vec::new()), |_| {}).unwrap();
        assert!(rep.peak_dbfs <= -0.99, "{medium}: peak {}", rep.peak_dbfs);
        assert!(rep.rms_dbfs > -40.0 && rep.calls > 100, "{medium}: {rep:?}");
    }
}

#[test]
fn renders_over_4_gb_are_refused() {
    let opts = RenderFile {
        minutes: 300.0,
        fade_in: 0.0,
        fade_out: 0.0,
        bits: 24,
    };
    let e = render(&small(), &opts, Cursor::new(Vec::new()), |_| {}).unwrap_err();
    assert!(e.contains("4 GB") && e.contains("248 minutes"), "{e}");
}
