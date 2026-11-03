//! The commands that move the text of the per-user extension settings, `extensions.toml` in the
//! app data directory (ADR 0034 §6): read it and write it atomically. They parse nothing;
//! `curvyo-ui-core` does, inside the frontend's `WasmSession`.

use std::path::PathBuf;

use tauri::{AppHandle, Manager};

/// The file's name in the data directory.
const EXTENSIONS_FILE: &str = "extensions.toml";

fn extensions_path(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|dir| dir.join(EXTENSIONS_FILE))
        .map_err(|error| format!("the data folder is not available ({error})"))
}

/// The text of the file; `None` when there is none yet (every extension enabled).
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn read_extensions_file(app: AppHandle) -> Result<Option<String>, String> {
    let path = extensions_path(&app)?;
    match curvyo_storage_io::read_optional(&path).map_err(|e| e.to_string())? {
        Some(bytes) => String::from_utf8(bytes)
            .map(Some)
            .map_err(|_| "the file is not valid UTF-8 text".to_string()),
        None => Ok(None),
    }
}

/// Replaces the file with `text`, atomically; the folder is created at the first write.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn write_extensions_file(app: AppHandle, text: String) -> Result<(), String> {
    let path = extensions_path(&app)?;
    curvyo_storage_io::ensure_parent_dir(&path).map_err(|e| e.to_string())?;
    curvyo_storage_io::write_atomic(&path, text.as_bytes()).map_err(|e| e.to_string())
}
