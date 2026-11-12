//! The commands that move the text of the material records, `materials/<id>.json` and
//! `material-order.json` in the app data directory, and of `test-grid-choice.json` in the config
//! directory (ADR 0055 §1, §5, §6, §7). They parse nothing; `curvyo-laser-core` does, inside the
//! frontend's `WasmSession`. An id that is not a record id is refused, so no command can reach
//! outside the folder. Nothing here uses the network (spec `0028` criterion 31).

use std::path::{Path, PathBuf};

use serde::Serialize;
use tauri::{AppHandle, Manager};

/// The records' folder in the data directory.
const MATERIALS_DIR: &str = "materials";
/// The list order in the data directory, synced with the records.
const ORDER_FILE: &str = "material-order.json";
/// The remembered grid values in the config directory, this computer's only.
const GRID_CHOICE_FILE: &str = "test-grid-choice.json";

/// One listed record: its file name and text.
#[derive(Serialize, Debug, PartialEq, Eq)]
pub(crate) struct RecordText {
    name: String,
    text: String,
}

fn data_dir(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map_err(|error| format!("the data folder is not available ({error})"))
}

fn config_dir(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_config_dir()
        .map_err(|error| format!("the config folder is not available ({error})"))
}

/// The path of the record `id` under the data directory `data`; refused for any other name.
fn record_path(data: &Path, id: &str) -> Result<PathBuf, String> {
    if !curvyo_laser_core::is_record_id(id) {
        return Err("not a material id".to_owned());
    }
    Ok(data
        .join(MATERIALS_DIR)
        .join(curvyo_laser_core::record_file_name(id)))
}

fn list_in(data: &Path) -> Result<Vec<RecordText>, String> {
    let texts = curvyo_storage_io::read_dir_texts(
        &data.join(MATERIALS_DIR),
        |name| curvyo_laser_core::record_id(name).is_some(),
        curvyo_laser_core::material_library::MAX_MATERIALS,
        curvyo_laser_core::material_library::MAX_MATERIAL_BYTES,
    )
    .map_err(|e| e.to_string())?;
    Ok(texts
        .into_iter()
        .map(|(name, text)| RecordText { name, text })
        .collect())
}

fn write_text(path: &Path, text: &str) -> Result<(), String> {
    curvyo_storage_io::ensure_parent_dir(path).map_err(|e| e.to_string())?;
    curvyo_storage_io::write_atomic(path, text.as_bytes()).map_err(|e| e.to_string())
}

fn read_text(path: &Path) -> Result<Option<String>, String> {
    match curvyo_storage_io::read_optional(path).map_err(|e| e.to_string())? {
        Some(bytes) => String::from_utf8(bytes)
            .map(Some)
            .map_err(|_| "the file is not valid UTF-8 text".to_string()),
        None => Ok(None),
    }
}

/// Every record file's name and text (`<uuid>.json` names only), in name order; none when the
/// folder does not exist yet.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn list_materials(app: AppHandle) -> Result<Vec<RecordText>, String> {
    list_in(&data_dir(&app)?)
}

/// Replaces the record `id` with `text`, atomically; the folder is made at the first write.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn write_material(app: AppHandle, id: String, text: String) -> Result<(), String> {
    write_text(&record_path(&data_dir(&app)?, &id)?, &text)
}

/// Removes the record `id`; one already gone is not an error.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn remove_material(app: AppHandle, id: String) -> Result<(), String> {
    curvyo_storage_io::remove_file(&record_path(&data_dir(&app)?, &id)?).map_err(|e| e.to_string())
}

/// The text of `material-order.json`; `None` when there is none yet.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn read_material_order(app: AppHandle) -> Result<Option<String>, String> {
    read_text(&data_dir(&app)?.join(ORDER_FILE))
}

/// Replaces `material-order.json` with `text`, atomically.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn write_material_order(app: AppHandle, text: String) -> Result<(), String> {
    write_text(&data_dir(&app)?.join(ORDER_FILE), &text)
}

/// The text of `test-grid-choice.json`; `None` when there is none yet.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn read_test_grid_choice(app: AppHandle) -> Result<Option<String>, String> {
    read_text(&config_dir(&app)?.join(GRID_CHOICE_FILE))
}

/// Replaces `test-grid-choice.json` with `text`, atomically.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn write_test_grid_choice(app: AppHandle, text: String) -> Result<(), String> {
    write_text(&config_dir(&app)?.join(GRID_CHOICE_FILE), &text)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ID: &str = "0b6c3f1e-2a4d-4b8e-9c7f-1d2e3f4a5b6c";

    #[test]
    fn only_record_ids_reach_the_materials_folder() {
        let data = Path::new("/data");
        assert_eq!(
            record_path(data, ID).unwrap(),
            Path::new("/data/materials").join(format!("{ID}.json"))
        );
        for bad in ["../machines/x", "generic-laser", "", &ID.to_uppercase()] {
            assert!(record_path(data, bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn records_are_listed_written_and_removed_as_text() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(list_in(dir.path()).unwrap(), Vec::new(), "no folder yet");
        let path = record_path(dir.path(), ID).unwrap();
        write_text(&path, "{}\n").unwrap();
        std::fs::write(dir.path().join(MATERIALS_DIR).join("notes.txt"), "x").unwrap();
        assert_eq!(
            list_in(dir.path()).unwrap(),
            vec![RecordText {
                name: format!("{ID}.json"),
                text: "{}\n".to_owned()
            }]
        );
        curvyo_storage_io::remove_file(&path).unwrap();
        curvyo_storage_io::remove_file(&path).unwrap();
        assert_eq!(list_in(dir.path()).unwrap(), Vec::new());
        let order = dir.path().join(ORDER_FILE);
        assert_eq!(read_text(&order).unwrap(), None);
        write_text(&order, "order").unwrap();
        assert_eq!(read_text(&order).unwrap().as_deref(), Some("order"));
    }
}
