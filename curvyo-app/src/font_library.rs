//! The commands that move the text of the font library file and of the check cache
//! (`specs/0075-font-library` criteria 12 and 18, `adrs.md` decisions 1, 5 and 8): read, write
//! atomically, set a damaged library file aside. They parse nothing; `curvyo-ui-core` does,
//! inside the page's wasm. The library file lives in the data directory beside
//! `document-formats.toml` (synced with it); the cache in the cache directory (this computer
//! only, deletable).

use std::path::{Path, PathBuf};

use tauri::{AppHandle, Manager};

const LIBRARY_FILE: &str = "font-library.toml";
const BROKEN_SUFFIX: &str = ".broken";
const CHECKS_FILE: &str = "font-checks.txt";

fn library_path(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|dir| dir.join(LIBRARY_FILE))
        .map_err(|error| format!("the data folder is not available ({error})"))
}

fn checks_path(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_cache_dir()
        .map(|dir| dir.join(CHECKS_FILE))
        .map_err(|error| format!("the cache folder is not available ({error})"))
}

fn read_text(path: &Path) -> Result<Option<String>, String> {
    match curvyo_storage_io::read_optional(path).map_err(|e| e.to_string())? {
        Some(bytes) => String::from_utf8(bytes)
            .map(Some)
            .map_err(|_| "the file is not valid UTF-8 text".to_string()),
        None => Ok(None),
    }
}

fn write_text(path: &Path, text: &str) -> Result<(), String> {
    curvyo_storage_io::ensure_parent_dir(path).map_err(|e| e.to_string())?;
    curvyo_storage_io::write_atomic(path, text.as_bytes()).map_err(|e| e.to_string())
}

/// The text of the font library file; `None` when there is none yet.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn read_font_library(app: AppHandle) -> Result<Option<String>, String> {
    read_text(&library_path(&app)?)
}

/// Replaces the font library file with `text`, atomically.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn write_font_library(app: AppHandle, text: String) -> Result<(), String> {
    write_text(&library_path(&app)?, &text)
}

/// Renames a damaged font library file to `font-library.toml.broken`, replacing an older one.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn set_font_library_aside(app: AppHandle) -> Result<(), String> {
    let path = library_path(&app)?;
    let mut aside = path.clone().into_os_string();
    aside.push(BROKEN_SUFFIX);
    curvyo_storage_io::rename_replacing(&path, &PathBuf::from(aside)).map_err(|e| e.to_string())
}

/// The text of the check cache; `None` when there is none.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn read_font_checks(app: AppHandle) -> Result<Option<String>, String> {
    read_text(&checks_path(&app)?)
}

/// Replaces the check cache with `text`, atomically.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn write_font_checks(app: AppHandle, text: String) -> Result<(), String> {
    write_text(&checks_path(&app)?, &text)
}
