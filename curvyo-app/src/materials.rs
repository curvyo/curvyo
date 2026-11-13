//! The commands that move the material library's files: the records `materials/<id>.json`, their
//! photos `materials/<id>.<test id>.jpg|png`, `material-order.json` and `retired-test-ids.json`
//! in the app data directory, and `test-grid-choice.json` in the config directory (ADR 0055 §1,
//! §5, §6; ADR 0058 §1, §4, §5). They parse nothing; `curvyo-laser-core` does, inside the
//! frontend's `WasmSession`. A name that is not a record id or a photo file name is refused, so no
//! command can reach outside the folder. Nothing here uses the network (spec `0112` criterion 33).

use std::path::{Path, PathBuf};

use curvyo_laser_core::grids::TestId;
use curvyo_laser_core::library::{MAX_PHOTO_BYTES, MAX_RECORD_TEXT, PhotoKind};
use serde::{Deserialize, Serialize};
use tauri::ipc::{Request, Response};
use tauri::{AppHandle, Manager};

use crate::raw_ipc::call_args;

/// The records' folder in the data directory.
const MATERIALS_DIR: &str = "materials";
/// The list order in the data directory, synced with the records.
const ORDER_FILE: &str = "material-order.json";
/// The remembered grid values in the config directory, this computer's only.
const GRID_CHOICE_FILE: &str = "test-grid-choice.json";
/// The test ids never to be given again, next to the records' folder (ADR 0058 §4).
const RETIRED_FILE: &str = "retired-test-ids.json";

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

/// The path of the photo file `name` (`<record id>.<test id>.jpg|png`) under the data directory
/// `data`; refused for any other name.
fn photo_path(data: &Path, name: &str) -> Result<PathBuf, String> {
    let mut parts = name.split('.');
    let fits = matches!(
        (parts.next(), parts.next(), parts.next(), parts.next()),
        (Some(id), Some(test), Some(kind), None)
            if curvyo_laser_core::is_record_id(id)
                && TestId::parse(test).is_some()
                && PhotoKind::parse(kind).is_some()
    );
    if !fits {
        return Err("not a photo file name".to_owned());
    }
    Ok(data.join(MATERIALS_DIR).join(name))
}

/// The path of `retired-test-ids.json` under the data directory `data`.
fn retired_path(data: &Path) -> PathBuf {
    data.join(RETIRED_FILE)
}

fn list_in(data: &Path) -> Result<Vec<RecordText>, String> {
    let texts = curvyo_storage_io::read_dir_texts(
        &data.join(MATERIALS_DIR),
        |name| curvyo_laser_core::record_id(name).is_some(),
        curvyo_laser_core::library::MAX_MATERIALS,
        curvyo_laser_core::library::MAX_RECORD_TEXT,
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

/// Writes a record's text; one over the reader's 512 KiB is refused, so every record written here
/// lists again (ADR 0058 §6).
fn write_record_text(path: &Path, text: &str) -> Result<(), String> {
    if text.len() > MAX_RECORD_TEXT {
        return Err("it is over 512 KiB".to_owned());
    }
    write_text(path, text)
}

/// Writes a photo's bytes; more than 1 MiB is refused (ADR 0058 §5).
fn write_photo(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if bytes.len() > MAX_PHOTO_BYTES {
        return Err("it is over 1 MiB".to_owned());
    }
    curvyo_storage_io::ensure_parent_dir(path).map_err(|e| e.to_string())?;
    curvyo_storage_io::write_atomic(path, bytes).map_err(|e| e.to_string())
}

/// A photo's bytes; `not-found` when there is none, refused above 1 MiB.
fn read_photo(path: &Path) -> Result<Vec<u8>, String> {
    let bytes = curvyo_storage_io::read_optional(path)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "not-found".to_owned())?;
    if bytes.len() > MAX_PHOTO_BYTES {
        return Err("it is over 1 MiB".to_owned());
    }
    Ok(bytes)
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

/// Replaces the record `id` with `text`, atomically; the folder is made at the first write. A text
/// over 512 KiB is refused.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn write_material(app: AppHandle, id: String, text: String) -> Result<(), String> {
    write_record_text(&record_path(&data_dir(&app)?, &id)?, &text)
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

/// The bytes of the photo file `name`, raw (not JSON); `not-found` when there is none.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn read_material_photo(app: AppHandle, name: String) -> Result<Response, String> {
    read_photo(&photo_path(&data_dir(&app)?, &name)?).map(Response::new)
}

/// The arguments of [`write_material_photo`] besides its bytes.
#[derive(Deserialize)]
struct PhotoArgs {
    name: String,
}

/// Replaces the photo file `name` with the raw body's bytes (`raw_ipc`), atomically.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn write_material_photo(app: AppHandle, request: Request<'_>) -> Result<(), String> {
    let (args, bytes): (PhotoArgs, Vec<u8>) = call_args(&request)?;
    write_photo(&photo_path(&data_dir(&app)?, &args.name)?, &bytes)
}

/// Removes the photo file `name`; one already gone is not an error.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn remove_material_photo(app: AppHandle, name: String) -> Result<(), String> {
    curvyo_storage_io::remove_file(&photo_path(&data_dir(&app)?, &name)?).map_err(|e| e.to_string())
}

/// The text of `retired-test-ids.json`; `None` when there is none yet.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn read_retired_test_ids(app: AppHandle) -> Result<Option<String>, String> {
    read_text(&retired_path(&data_dir(&app)?))
}

/// Replaces `retired-test-ids.json` with `text`, atomically.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn write_retired_test_ids(app: AppHandle, text: String) -> Result<(), String> {
    write_text(&retired_path(&data_dir(&app)?), &text)
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

    #[test]
    fn a_record_over_512_kib_is_refused_before_it_is_written() {
        let dir = tempfile::tempdir().unwrap();
        let path = record_path(dir.path(), ID).unwrap();
        let big = "x".repeat(curvyo_laser_core::library::MAX_RECORD_TEXT + 1);
        assert_eq!(
            write_record_text(&path, &big),
            Err("it is over 512 KiB".to_owned())
        );
        assert_eq!(read_text(&path).unwrap(), None, "nothing written");
        let fits = "x".repeat(curvyo_laser_core::library::MAX_RECORD_TEXT);
        write_record_text(&path, &fits).unwrap();
        assert_eq!(
            list_in(dir.path()).unwrap().len(),
            1,
            "a written record lists"
        );
    }

    #[test]
    fn only_photo_names_reach_the_materials_folder() {
        let data = Path::new("/data");
        for good in [format!("{ID}.M8W1O.png"), format!("{ID}.7K2QX.jpg")] {
            assert_eq!(
                photo_path(data, &good).unwrap(),
                Path::new("/data/materials").join(&good)
            );
        }
        let upper = ID.to_uppercase();
        for bad in [
            format!("{ID}.json"),
            format!("{ID}.m8w1o.png"),
            format!("{ID}.M8W1.png"),
            format!("{ID}.M8W1O.gif"),
            format!("{ID}.M8W1O.jpeg"),
            format!("{ID}.M8W1O.png.json"),
            format!("{upper}.M8W1O.png"),
            "../machines/x.M8W1O.png".to_owned(),
            format!("../{ID}.M8W1O.png"),
            ".M8W1O.png".to_owned(),
            String::new(),
        ] {
            assert!(photo_path(data, &bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn photos_are_written_read_and_removed_as_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let name = format!("{ID}.M8W1O.png");
        let path = photo_path(dir.path(), &name).unwrap();
        assert_eq!(read_photo(&path), Err("not-found".to_owned()), "none yet");
        write_photo(&path, b"\x89PNG bytes").unwrap();
        assert_eq!(read_photo(&path).unwrap(), b"\x89PNG bytes".to_vec());
        assert_eq!(
            list_in(dir.path()).unwrap(),
            Vec::new(),
            "a photo is not a record"
        );
        curvyo_storage_io::remove_file(&path).unwrap();
        curvyo_storage_io::remove_file(&path).unwrap();
        assert_eq!(read_photo(&path), Err("not-found".to_owned()));
    }

    #[test]
    fn a_photo_over_1_mib_is_neither_written_nor_read() {
        let dir = tempfile::tempdir().unwrap();
        let path = photo_path(dir.path(), &format!("{ID}.M8W1O.jpg")).unwrap();
        let big = vec![0_u8; curvyo_laser_core::library::MAX_PHOTO_BYTES + 1];
        assert_eq!(write_photo(&path, &big), Err("it is over 1 MiB".to_owned()));
        assert_eq!(read_photo(&path), Err("not-found".to_owned()));
        curvyo_storage_io::ensure_parent_dir(&path).unwrap();
        std::fs::write(&path, &big).unwrap();
        assert_eq!(read_photo(&path), Err("it is over 1 MiB".to_owned()));
    }

    #[test]
    fn the_retired_ids_sit_next_to_the_materials_folder() {
        let dir = tempfile::tempdir().unwrap();
        let path = retired_path(dir.path());
        assert_eq!(path, dir.path().join("retired-test-ids.json"));
        assert_eq!(read_text(&path).unwrap(), None);
        let text = "{\n  \"format_version\": 1,\n  \"retired\": []\n}\n";
        write_text(&path, text).unwrap();
        assert_eq!(read_text(&path).unwrap().as_deref(), Some(text));
    }
}
