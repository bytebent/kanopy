//! The species library: seeded once, saves by name, deletes, imports.

use kanopy_core::files::SpeciesFile;
use kanopy_core::species::demo;
use kanopy_live::library::{slug, Library};

fn temp(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("kanopy-lib-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

#[test]
fn slugs() {
    assert_eq!(slug("Crown chatterers"), "crown-chatterers");
    assert_eq!(slug("  Swifts (fast)!! "), "swifts-fast");
    assert_eq!(slug("??"), "species");
}

#[test]
fn seeds_once_saves_deletes_imports() {
    let dir = temp("a");
    let lib = Library::open(&dir).unwrap();
    let n = demo(1.0).len();
    assert_eq!(lib.list().len(), n);

    // deleting a demo species sticks: opening again doesn't seed it back
    lib.delete("Swifts").unwrap();
    let lib = Library::open(&dir).unwrap();
    assert_eq!(lib.list().len(), n - 1);
    assert!(lib.list().iter().all(|s| s.name != "Swifts"));

    // save replaces by name
    let mut sp = demo(1.0)[0].clone();
    sp.habitat.individuals = 7;
    lib.save(&sp).unwrap();
    let got = lib.list();
    assert_eq!(got.len(), n - 1);
    assert_eq!(
        got.iter()
            .find(|s| s.name == sp.name)
            .unwrap()
            .habitat
            .individuals,
        7
    );

    // import from anywhere
    let other = temp("b");
    std::fs::create_dir_all(&other).unwrap();
    let mut new = sp.clone().coloured("#f4f4f6");
    new.name = "Night owls".into();
    let file = other.join("owls.kanopy-species.json");
    SpeciesFile::save(&new, &file).unwrap();
    let back = lib.import(&file).unwrap();
    assert_eq!(back.name, "Night owls");
    assert_eq!(
        lib.list()
            .iter()
            .find(|s| s.name == "Night owls")
            .unwrap()
            .colour
            .as_deref(),
        Some("#f4f4f6")
    );

    // the list is sorted by name and skips files that don't read
    std::fs::write(dir.join("broken.kanopy-species.json"), "{").unwrap();
    let names: Vec<String> = lib
        .list()
        .into_iter()
        .map(|s| s.name.to_lowercase())
        .collect();
    let mut sorted = names.clone();
    sorted.sort();
    assert_eq!(names, sorted);
    assert!(lib.delete("Nobody").is_err());
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&other);
}
