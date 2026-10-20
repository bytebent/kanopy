//! kanopy, the desktop app: the pages in `ui/`, the live engine behind them.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use kanopy_core::files::ForestFile;
use kanopy_core::files::SpeciesFile;
use kanopy_core::species::Species;
use kanopy_live::library::Library;
use kanopy_live::{Cmd, ExportState, Live, Take};
use std::path::PathBuf;
use tauri::ipc::Response;
use tauri::State;
use tauri_plugin_dialog::DialogExt;

struct App(Live, Library);

/// The example forests, built in: (file name, JSON).
const EXAMPLES: [(&str, &str); 6] = [
    (
        "Sparse forest",
        include_str!("../../../examples/forests/sparse-forest.kanopy-forest.json"),
    ),
    (
        "Demo forest",
        include_str!("../../../examples/forests/demo-forest.kanopy-forest.json"),
    ),
    (
        "Mars woodland",
        include_str!("../../../examples/forests/mars-woodland.kanopy-forest.json"),
    ),
    (
        "Sunken kelp",
        include_str!("../../../examples/forests/sunken-kelp.kanopy-forest.json"),
    ),
    (
        "Canopy box",
        include_str!("../../../examples/forests/canopy-box.kanopy-forest.json"),
    ),
    (
        "Under the floor",
        include_str!("../../../examples/forests/under-the-floor.kanopy-forest.json"),
    ),
];

#[tauri::command]
fn info(app: State<App>) -> String {
    app.0.info()
}

#[tauri::command]
fn frame(app: State<App>) -> Response {
    Response::new(app.0.frame())
}

#[tauri::command]
fn command(app: State<App>, cmd: Cmd) -> Option<String> {
    app.0.send(cmd);
    app.0.error()
}

#[tauri::command]
fn error(app: State<App>) -> Option<String> {
    app.0.error()
}

#[tauri::command]
fn examples() -> Vec<String> {
    EXAMPLES.iter().map(|(n, _)| n.to_string()).collect()
}

#[tauri::command]
fn open_example(app: State<App>, name: String) -> Result<String, String> {
    let (_, json) = EXAMPLES
        .iter()
        .find(|(n, _)| *n == name)
        .ok_or("no such example")?;
    let f = ForestFile::from_json(json)?;
    let n = f.name.clone();
    app.0.send(Cmd::Load {
        forest: Box::new(f),
    });
    Ok(n)
}

#[tauri::command]
async fn open_forest(window: tauri::Window, app: State<'_, App>) -> Result<Option<String>, String> {
    let Some(p) = window
        .dialog()
        .file()
        .add_filter("kanopy forest or render", &["json", "wav"])
        .blocking_pick_file()
    else {
        return Ok(None);
    };
    let path: PathBuf = p.into_path().map_err(|e| e.to_string())?;
    let wav = path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("wav"));
    let f = if wav {
        ForestFile::from_wav(&path)?
    } else {
        ForestFile::load(&path)?
    };
    let name = f.name.clone();
    app.0.send(Cmd::Load {
        forest: Box::new(f),
    });
    Ok(Some(name))
}

#[tauri::command]
async fn save_forest(window: tauri::Window, app: State<'_, App>) -> Result<Option<String>, String> {
    let f = app.0.forest();
    let name = format!("{}.kanopy-forest.json", f.name);
    let Some(p) = window
        .dialog()
        .file()
        .set_file_name(&name)
        .add_filter("kanopy forest", &["json"])
        .blocking_save_file()
    else {
        return Ok(None);
    };
    let path: PathBuf = p.into_path().map_err(|e| e.to_string())?;
    f.save(&path)?;
    Ok(Some(path.display().to_string()))
}

#[tauri::command]
async fn export(
    window: tauri::Window,
    app: State<'_, App>,
    minutes: f64,
) -> Result<Option<String>, String> {
    let f = app.0.forest();
    let name = format!("{}_{}_{}min.wav", f.name, f.seed, minutes);
    let Some(p) = window
        .dialog()
        .file()
        .set_file_name(&name)
        .add_filter("WAV", &["wav"])
        .blocking_save_file()
    else {
        return Ok(None);
    };
    let path: PathBuf = p.into_path().map_err(|e| e.to_string())?;
    app.0.export(path.display().to_string(), minutes);
    Ok(Some(path.display().to_string()))
}

#[tauri::command]
fn library(app: State<App>) -> Vec<Species> {
    app.1.list()
}

#[tauri::command]
fn library_save(app: State<App>, species: Vec<Species>) -> Result<(), String> {
    species.iter().try_for_each(|s| app.1.save(s))
}

#[tauri::command]
fn library_delete(app: State<App>, name: String) -> Result<(), String> {
    app.1.delete(&name)
}

#[tauri::command]
async fn library_import(
    window: tauri::Window,
    app: State<'_, App>,
) -> Result<Option<Species>, String> {
    let Some(p) = window
        .dialog()
        .file()
        .add_filter("kanopy species", &["json"])
        .blocking_pick_file()
    else {
        return Ok(None);
    };
    let path: PathBuf = p.into_path().map_err(|e| e.to_string())?;
    app.1.import(&path).map(Some)
}

#[tauri::command]
async fn species_export(window: tauri::Window, species: Species) -> Result<Option<String>, String> {
    let name = format!(
        "{}.kanopy-species.json",
        kanopy_live::library::slug(&species.name)
    );
    let Some(p) = window
        .dialog()
        .file()
        .set_file_name(&name)
        .add_filter("kanopy species", &["json"])
        .blocking_save_file()
    else {
        return Ok(None);
    };
    let path: PathBuf = p.into_path().map_err(|e| e.to_string())?;
    SpeciesFile::save(&species, &path)?;
    Ok(Some(path.display().to_string()))
}

#[tauri::command]
fn audition(app: State<App>, species: Species, take: Take) -> Result<serde_json::Value, String> {
    app.0.audition(&species, take)
}

#[tauri::command]
fn check(app: State<App>, species: Species) -> Result<serde_json::Value, String> {
    app.0.check(&species)
}

#[tauri::command]
fn export_state(app: State<App>) -> ExportState {
    app.0.export_state()
}

fn main() {
    let start = ForestFile::from_json(EXAMPLES[0].1).expect("built-in example");
    let live = Live::start(start, true).expect("kanopy engine");
    let lib = Library::open(&Library::default_dir()).expect("species library");
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(App(live, lib))
        .invoke_handler(tauri::generate_handler![
            info,
            frame,
            command,
            error,
            examples,
            open_example,
            open_forest,
            save_forest,
            export,
            export_state,
            library,
            library_save,
            library_delete,
            library_import,
            species_export,
            audition,
            check
        ])
        .run(tauri::generate_context!())
        .expect("kanopy app");
}
