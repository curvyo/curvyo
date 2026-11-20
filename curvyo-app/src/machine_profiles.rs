//! The commands that move the text of the machine profile records, `machines/<id>.json` in the
//! app data directory, and of `machine-choice.json` in the config directory (ADR 0038 §1, §6, §7).
//! They parse nothing; `curvyo-laser-core` does, inside the frontend's `WasmSession`. An id that
//! is not a profile id is refused, so no command can reach outside the folder.

use std::path::{Path, PathBuf};

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
    record_path_in(&machines_dir(app)?, id)
}

/// The path of the record `id` in the folder `dir`: a profile id, or one with the conflict
/// copy's suffix (`specs/0116-library-sync`, ADR 0062 decision 5); any other id is refused.
fn record_path_in(dir: &Path, id: &str) -> Result<PathBuf, String> {
    let profile = id
        .strip_suffix(curvyo_laser_core::CONFLICT_SUFFIX)
        .unwrap_or(id);
    if !curvyo_laser_core::is_profile_id(profile) {
        return Err("not a machine profile id".to_owned());
    }
    Ok(dir.join(curvyo_laser_core::record_file_name(id)))
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

/// Removes the record file and the connector pairing kept for it (`0064` Q10a).
fn remove_profile_and_pairing(
    path: &std::path::Path,
    store: Option<&curvyo_storage_io::CredentialStore>,
    id: &str,
) -> Result<(), String> {
    curvyo_storage_io::remove_file(path).map_err(|e| e.to_string())?;
    if let Some(store) = store {
        crate::connector::forget_record(store, id).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Removes the record `id`, and with it the connector pairing kept for that profile.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn remove_machine_profile(
    app: AppHandle,
    open: tauri::State<'_, crate::device_keys::OpenStore>,
    id: String,
) -> Result<(), String> {
    let path = record_path(&app, &id)?;
    match open.with_keychain(&app, |store| {
        remove_profile_and_pairing(&path, Some(store), &id)
    }) {
        Ok(result) => result,
        // No credential store here means no pairing was ever kept.
        Err(_) => remove_profile_and_pairing(&path, None, &id),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ID: &str = "6f1c2a4e-9b3d-4c1a-8e2f-0a1b2c3d4e5f";

    #[test]
    fn a_conflict_copy_has_a_record_path_and_other_ids_are_refused() {
        let dir = Path::new("machines");
        assert_eq!(
            record_path_in(dir, ID).unwrap(),
            dir.join(curvyo_laser_core::record_file_name(ID))
        );
        let copy = curvyo_laser_core::conflict_id(ID);
        assert_eq!(
            record_path_in(dir, &copy).unwrap(),
            dir.join(curvyo_laser_core::record_file_name(&copy))
        );
        for bad in [
            "",
            ".conflict",
            "../x.conflict",
            "6f1c2a4e.conflict",
            &format!("{ID}.conflict.conflict"),
            &format!("{ID}/../x"),
        ] {
            assert!(record_path_in(dir, bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn deleting_a_profile_removes_its_pairing_too() {
        let dir = tempfile::tempdir().unwrap();
        let store = curvyo_storage_io::CredentialStore::file(dir.path(), "pass").unwrap();
        let path = dir.path().join("a.json");
        std::fs::write(&path, "{}").unwrap();
        store
            .set(&crate::connector::record_name("a"), b"pairing")
            .unwrap();
        store
            .set(&crate::connector::record_name("b"), b"other")
            .unwrap();
        remove_profile_and_pairing(&path, Some(&store), "a").unwrap();
        assert!(!path.exists());
        assert!(store.get("connector/a").unwrap().is_none());
        assert!(store.get("connector/b").unwrap().is_some());
    }
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
