//! The species library: every species made so far, one `.kanopy-species.json` per species in a
//! folder shared by the app and `kanopy serve`. It starts with the demo species. Applying changes
//! on the Species page saves the forest's species here, so nothing is lost and nothing has to be
//! loaded by hand.

use kanopy_core::files::SpeciesFile;
use kanopy_core::species::{demo, Species};
use std::path::{Path, PathBuf};

pub struct Library {
    pub dir: PathBuf,
}

/// The file name for a species: its name, lowercase, dashes for the rest.
pub fn slug(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars() {
        if c.is_alphanumeric() {
            out.extend(c.to_lowercase());
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    let s = out.trim_matches('-').to_string();
    if s.is_empty() {
        "species".into()
    } else {
        s
    }
}

impl Library {
    /// Where the library lives: Application Support on macOS, ~/.local/share elsewhere.
    pub fn default_dir() -> PathBuf {
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));
        if cfg!(target_os = "macos") {
            home.join("Library/Application Support/kanopy/species")
        } else {
            home.join(".local/share/kanopy/species")
        }
    }

    /// Open (and on first use create, with the demo species) a library folder.
    pub fn open(dir: &Path) -> Result<Library, String> {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        let lib = Library {
            dir: dir.to_path_buf(),
        };
        let marker = dir.join(".started");
        if !marker.exists() {
            for sp in demo(1.0) {
                lib.save(&sp)?;
            }
            std::fs::write(&marker, "kanopy species library\n").map_err(|e| e.to_string())?;
        }
        Ok(lib)
    }

    fn path(&self, name: &str) -> PathBuf {
        self.dir.join(format!("{}.kanopy-species.json", slug(name)))
    }

    /// Every species in the library, by name. Files that don't read are skipped.
    pub fn list(&self) -> Vec<Species> {
        let mut v: Vec<Species> = std::fs::read_dir(&self.dir)
            .into_iter()
            .flatten()
            .flatten()
            .filter(|e| {
                e.file_name()
                    .to_string_lossy()
                    .ends_with(".kanopy-species.json")
            })
            .filter_map(|e| SpeciesFile::load(&e.path()).ok())
            .collect();
        v.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        v
    }

    /// Save (or replace) a species, by name.
    pub fn save(&self, sp: &Species) -> Result<(), String> {
        SpeciesFile::save(sp, &self.path(&sp.name))
    }

    pub fn delete(&self, name: &str) -> Result<(), String> {
        let p = self.path(name);
        std::fs::remove_file(&p).map_err(|e| format!("{}: {e}", p.display()))
    }

    /// Read a species file from anywhere and keep it in the library.
    pub fn import(&self, file: &Path) -> Result<Species, String> {
        let sp = SpeciesFile::load(file)?;
        self.save(&sp)?;
        Ok(sp)
    }
}
