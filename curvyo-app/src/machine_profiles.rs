//! The commands that move the text of the machine profile records, `machines/<id>.json` in the
//! app data directory, and of `machine-choice.json` in the config directory (ADR 0036 §1, §6, §7).
//! They parse nothing; `curvyo-laser-core` does, inside the frontend's `WasmSession`. An id that
//! is not a profile id is refused, so no command can reach outside the folder.

use std::path::PathBuf;

use serde::Serialize;
use tauri::{AppHandle, Manager};

/// The records' folder in the data directory.
const MACHINES_DIR: &str = "machines";
/// The last-pick file in the config directory.
const CHOICE_FILE: &str = "machine-choice.json";

/// One listed record: its file name and text.
#[derive(Serialize)]
pub(crate) struct RecordText {
    name: String,
    text: String,
}

fn machines_dir(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|dir| dir.join(MACHINES_DIR))
        .map_err(|error| format!("the data folder is not available ({error})"))
}

fn record_path(app: &AppHandle, id: &str) -> Result<PathBuf, String> {
    if !curvyo_laser_core::is_profile_id(id) {
        return Err("not a machine profile id".to_owned());
    }
    Ok(machines_dir(app)?.join(curvyo_laser_core::record_file_name(id)))
}

fn choice_path(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_config_dir()
        .map(|dir| dir.join(CHOICE_FILE))
        .map_err(|error| format!("the config folder is not available ({error})"))
}

/// Every record file's name and text (`<uuid>.json` names only), in name order; none when the
/// folder does not exist yet.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn list_machine_profiles(app: AppHandle) -> Result<Vec<RecordText>, String> {
    let texts = curvyo_storage_io::read_dir_texts(
        &machines_dir(&app)?,
        |name| curvyo_laser_core::record_id(name).is_some(),
        curvyo_laser_core::MAX_RECORDS,
        curvyo_laser_core::MAX_RECORD_BYTES,
    )
    .map_err(|e| e.to_string())?;
    Ok(texts
        .into_iter()
        .map(|(name, text)| RecordText { name, text })
        .collect())
}

/// Replaces the record `id` with `text`, atomically; the folder is made at the first write.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn write_machine_profile(
    app: AppHandle,
    id: String,
    text: String,
) -> Result<(), String> {
    let path = record_path(&app, &id)?;
    curvyo_storage_io::ensure_parent_dir(&path).map_err(|e| e.to_string())?;
    curvyo_storage_io::write_atomic(&path, text.as_bytes()).map_err(|e| e.to_string())
}

/// Removes the record `id`.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn remove_machine_profile(app: AppHandle, id: String) -> Result<(), String> {
    curvyo_storage_io::remove_file(&record_path(&app, &id)?).map_err(|e| e.to_string())
}

/// The text of `machine-choice.json`; `None` when there is none yet.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn read_machine_choice(app: AppHandle) -> Result<Option<String>, String> {
    match curvyo_storage_io::read_optional(&choice_path(&app)?).map_err(|e| e.to_string())? {
        Some(bytes) => String::from_utf8(bytes)
            .map(Some)
            .map_err(|_| "the file is not valid UTF-8 text".to_string()),
        None => Ok(None),
    }
}

/// Replaces `machine-choice.json` with `text`, atomically.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn write_machine_choice(app: AppHandle, text: String) -> Result<(), String> {
    let path = choice_path(&app)?;
    curvyo_storage_io::ensure_parent_dir(&path).map_err(|e| e.to_string())?;
    curvyo_storage_io::write_atomic(&path, text.as_bytes()).map_err(|e| e.to_string())
}
