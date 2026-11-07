//! Application de bureau FabVents (macOS, Windows, Linux).
//!
//! Une fenêtre qui affiche l'interface web (le moteur tourne en WebAssembly
//! dans la page, comme dans le navigateur). L'application ajoute ce qu'un
//! navigateur ne sait pas faire : fenêtres natives d'ouverture et
//! d'enregistrement, enregistrement sur place du projet ouvert, fichiers
//! récents (dans le dossier de données de l'utilisateur).

// Pas de console sous Windows pour l'application distribuée.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::Serialize;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::DialogExt;

/// Fichiers récents au plus.
const RECENT: usize = 10;

#[derive(Serialize)]
struct Opened {
    path: String,
    name: String,
    text: String,
}

fn recent_path(app: &AppHandle) -> Option<PathBuf> {
    app.path()
        .app_data_dir()
        .ok()
        .map(|d| d.join("recent.json"))
}

fn read_recent(app: &AppHandle) -> Vec<String> {
    recent_path(app)
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

/// Ajoute un fichier en tête des récents (projets seulement).
fn remember(app: &AppHandle, path: &Path) {
    let Some(file) = recent_path(app) else { return };
    let p = path.to_string_lossy().to_string();
    let mut list = read_recent(app);
    list.retain(|x| *x != p);
    list.insert(0, p);
    list.truncate(RECENT);
    if let Some(dir) = file.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(file, serde_json::to_string(&list).unwrap_or_default());
}

fn is_project(path: &Path) -> bool {
    let name = path.to_string_lossy().to_lowercase();
    name.ends_with(".json") || name.ends_with(".svg")
}

fn read(app: &AppHandle, path: PathBuf) -> Result<Opened, String> {
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{} : {e}", path.display()))?;
    if is_project(&path) {
        remember(app, &path);
    }
    Ok(Opened {
        name: path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default(),
        path: path.to_string_lossy().to_string(),
        text,
    })
}

/// Fenêtre d'ouverture : projet FabVents, SVG, DXF ou image.
#[tauri::command]
async fn open_file(app: AppHandle) -> Result<Option<Opened>, String> {
    let picked = app
        .dialog()
        .file()
        .add_filter("FabVents, SVG, DXF", &["json", "svg", "dxf"])
        .blocking_pick_file();
    match picked {
        Some(f) => read(&app, f.into_path().map_err(|e| e.to_string())?).map(Some),
        None => Ok(None),
    }
}

/// Ouvre un fichier récent.
#[tauri::command]
async fn open_path(app: AppHandle, path: String) -> Result<Opened, String> {
    read(&app, PathBuf::from(path))
}

/// Fenêtre d'enregistrement ; `None` si l'utilisateur annule.
#[tauri::command]
async fn save_file(
    app: AppHandle,
    name: String,
    data: Vec<u8>,
    filter: String,
    extensions: Vec<String>,
) -> Result<Option<String>, String> {
    let exts: Vec<&str> = extensions.iter().map(String::as_str).collect();
    let picked = app
        .dialog()
        .file()
        .set_file_name(&name)
        .add_filter(&filter, &exts)
        .blocking_save_file();
    let Some(f) = picked else { return Ok(None) };
    let path = f.into_path().map_err(|e| e.to_string())?;
    std::fs::write(&path, &data).map_err(|e| format!("{} : {e}", path.display()))?;
    if is_project(&path) {
        remember(&app, &path);
    }
    Ok(Some(path.to_string_lossy().to_string()))
}

/// Enregistre sur place (projet déjà ouvert ou enregistré).
#[tauri::command]
async fn write_file(app: AppHandle, path: String, data: Vec<u8>) -> Result<(), String> {
    let path = PathBuf::from(path);
    std::fs::write(&path, &data).map_err(|e| format!("{} : {e}", path.display()))?;
    remember(&app, &path);
    Ok(())
}

#[tauri::command]
fn recent_files(app: AppHandle) -> Vec<String> {
    read_recent(&app)
        .into_iter()
        .filter(|p| Path::new(p).exists())
        .collect()
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            open_file,
            open_path,
            save_file,
            write_file,
            recent_files
        ])
        .run(tauri::generate_context!())
        .expect("FabVents n'a pas pu démarrer");
}
