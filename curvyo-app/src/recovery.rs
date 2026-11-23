//! The commands of the recovery copies (`specs/0136-autosave-and-restore` criteria 2, 3, 6, 7,
//! 9, 11 and 12, ADR 0079): a folder per session key in the app's *local* data folder (never the
//! roaming or synced one), written by the page's recovery worker through these commands, listed
//! at start, and read back for Restore. Nothing here leaves the machine (criterion 10).

use std::collections::HashMap;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use curvyo_document_core::check_recovery_copy;
use curvyo_storage_io::{ListedCopy, RecoveryError, RecoveryFolder, SessionLock};
use curvyo_ui_core::{
    PruneCandidate, RecordError, RecoveryBase, RecoveryRecord, decode_record, prune_plan,
};
use serde::{Deserialize, Serialize};
use tauri::ipc::{Request, Response};
use tauri::{AppHandle, Manager, State};

use crate::raw_ipc::call_args;

/// The folder below the app's local data folder.
const RECOVERY_DIR: &str = "recovery";

/// The locks this running app holds, one per session that has a copy.
#[derive(Default)]
pub(crate) struct RecoveryLocks(Mutex<HashMap<String, SessionLock>>);

fn folder(app: &AppHandle) -> Result<RecoveryFolder, String> {
    let dir = app
        .path()
        .app_local_data_dir()
        .map_err(|error| format!("the data folder is not available ({error})"))?;
    Ok(RecoveryFolder::new(dir.join(RECOVERY_DIR)))
}

/// The page's word for a failed write: `disk-full`, `no-permission`, or the system's reason.
fn failure_code(error: &RecoveryError) -> String {
    let kind = match error {
        RecoveryError::Io(error) => Some(error.kind()),
        RecoveryError::Fs(error) => std::error::Error::source(error)
            .and_then(|source| source.downcast_ref::<std::io::Error>())
            .map(std::io::Error::kind),
        _ => None,
    };
    match kind {
        Some(ErrorKind::StorageFull) => "disk-full".to_owned(),
        Some(ErrorKind::PermissionDenied) => "no-permission".to_owned(),
        _ => error.to_string(),
    }
}

/// Takes the key's lock when this app does not hold it yet.
fn ensure_lock(
    locks: &RecoveryLocks,
    folder: &RecoveryFolder,
    key: &str,
) -> Result<(), RecoveryError> {
    let mut held = locks
        .0
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if !held.contains_key(key) {
        held.insert(key.to_owned(), folder.begin(key)?);
    }
    Ok(())
}

/// One row of the File tab block.
#[derive(Debug, Serialize, PartialEq, Eq)]
pub(crate) struct Row {
    key: String,
    name: Option<String>,
    pages: u32,
    last_step_ms: u64,
    /// Criterion 6: the saved file the copy was based on has changed or is gone.
    changed: bool,
    /// Criterion 7: `damaged` or `newer`.
    problem: Option<&'static str>,
}

/// The listing: where the copies are kept and the rows, newest first.
#[derive(Debug, Serialize)]
pub(crate) struct Listing {
    path: String,
    rows: Vec<Row>,
}

/// What a file looks like now, `None` when it is gone.
fn file_base(path: &Path) -> Option<RecoveryBase> {
    let meta = std::fs::metadata(path).ok()?;
    let modified_ms = meta
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|d| u64::try_from(d.as_millis()).unwrap_or(0))?;
    Some(RecoveryBase {
        path: path.to_string_lossy().into_owned(),
        modified_ms,
        length: meta.len(),
    })
}

fn row_of(folder: &RecoveryFolder, copy: &ListedCopy) -> Row {
    let decoded = copy.record.as_deref().map(decode_record);
    let record: Option<&RecoveryRecord> = decoded.as_ref().and_then(|r| r.as_ref().ok());
    let problem = match &decoded {
        Some(Err(RecordError::Newer)) => Some("newer"),
        Some(Err(RecordError::Damaged)) | None => Some("damaged"),
        Some(Ok(_)) => match folder
            .read_copy(&copy.key)
            .map(|bytes| check_recovery_copy(&bytes))
        {
            Ok(Ok(())) => None,
            Ok(Err(curvyo_document_core::OpenError::FormatTooNew { .. })) => Some("newer"),
            _ => Some("damaged"),
        },
    };
    let changed = record.and_then(|r| r.base.as_ref()).is_some_and(|base| {
        file_base(Path::new(&base.path))
            .is_none_or(|now| now.modified_ms != base.modified_ms || now.length != base.length)
    });
    Row {
        key: copy.key.clone(),
        name: record.and_then(|r| r.name.clone()),
        pages: record.map_or(0, |r| r.pages),
        last_step_ms: record.map_or(copy.modified_ms, |r| r.last_step_ms),
        changed,
        problem,
    }
}

/// The start of the app: lists the copies no running session holds, prunes (criterion 12) and
/// returns the rows.
fn listing(folder: &RecoveryFolder) -> Listing {
    let mut copies = folder.list();
    let candidates: Vec<PruneCandidate> = copies
        .iter()
        .map(|copy| PruneCandidate {
            key: copy.key.clone(),
            bytes: copy.bytes,
            last_step_ms: copy
                .record
                .as_deref()
                .and_then(|bytes| decode_record(bytes).ok())
                .map_or(copy.modified_ms, |r| r.last_step_ms),
            locked: copy.locked,
        })
        .collect();
    for key in prune_plan(&candidates) {
        let _ = folder.delete_all(&key);
        copies.retain(|copy| copy.key != key);
    }
    let mut rows: Vec<Row> = copies
        .iter()
        .filter(|copy| !copy.locked)
        .map(|copy| row_of(folder, copy))
        .collect();
    rows.sort_by(|a, b| {
        b.last_step_ms
            .cmp(&a.last_step_ms)
            .then_with(|| a.key.cmp(&b.key))
    });
    Listing {
        path: folder.path().to_string_lossy().into_owned(),
        rows,
    }
}

/// The copies waiting at start.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn recovery_list(app: AppHandle) -> Result<Listing, String> {
    Ok(listing(&folder(&app)?))
}

/// Takes the key's lock: Restore adopts the copy it opens (ADR 0079 §6); refused while a
/// running session holds it.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn recovery_begin(
    app: AppHandle,
    locks: State<'_, RecoveryLocks>,
    key: String,
) -> Result<(), String> {
    ensure_lock(&locks, &folder(&app)?, &key).map_err(|e| failure_code(&e))
}

/// The arguments of [`recovery_write_asset`] besides its bytes.
#[derive(Deserialize)]
struct AssetArgs {
    key: String,
    name: String,
}

/// Writes one asset once, before the copy that refers to it.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn recovery_write_asset(
    app: AppHandle,
    locks: State<'_, RecoveryLocks>,
    request: Request<'_>,
) -> Result<(), String> {
    let (args, bytes): (AssetArgs, Vec<u8>) = call_args(&request)?;
    let folder = folder(&app)?;
    ensure_lock(&locks, &folder, &args.key).map_err(|e| failure_code(&e))?;
    folder
        .write_asset(&args.key, &args.name, &bytes)
        .map_err(|e| failure_code(&e))
}

/// The arguments of [`recovery_write_copy`] besides the copy's bytes.
#[derive(Deserialize)]
struct CopyArgs {
    key: String,
    /// The text of `recovery.json`.
    record: String,
}

/// Writes the copy, then its record; the record is read first, so a bad one writes nothing.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn recovery_write_copy(
    app: AppHandle,
    locks: State<'_, RecoveryLocks>,
    request: Request<'_>,
) -> Result<(), String> {
    let (args, copy): (CopyArgs, Vec<u8>) = call_args(&request)?;
    if decode_record(args.record.as_bytes()).is_err() {
        return Err("not a recovery record".to_owned());
    }
    let folder = folder(&app)?;
    ensure_lock(&locks, &folder, &args.key).map_err(|e| failure_code(&e))?;
    folder
        .write_copy(&args.key, &copy, args.record.as_bytes())
        .map_err(|e| failure_code(&e))
}

/// Save succeeded, or the project is clean again: the copy goes, the session keeps its lock.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn recovery_delete_copy(app: AppHandle, key: String) -> Result<(), String> {
    folder(&app)?
        .delete_copy(&key)
        .map_err(|e| failure_code(&e))
}

/// Discard, a clean Close, or a discarded row: the lock and the whole folder go.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn recovery_end(
    app: AppHandle,
    locks: State<'_, RecoveryLocks>,
    key: String,
) -> Result<(), String> {
    locks
        .0
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .remove(&key);
    folder(&app)?.delete_all(&key).map_err(|e| failure_code(&e))
}

/// The copy's container bytes.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn recovery_read_copy(app: AppHandle, key: String) -> Result<Response, String> {
    folder(&app)?
        .read_copy(&key)
        .map(Response::new)
        .map_err(|e| failure_code(&e))
}

/// The names of the copy's assets.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn recovery_asset_names(app: AppHandle, key: String) -> Result<Vec<String>, String> {
    folder(&app)?
        .asset_names(&key)
        .map_err(|e| failure_code(&e))
}

/// One asset's bytes.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn recovery_read_asset(
    app: AppHandle,
    key: String,
    name: String,
) -> Result<Response, String> {
    folder(&app)?
        .read_asset(&key, &name)
        .map(Response::new)
        .map_err(|e| failure_code(&e))
}

/// What the saved file at `path` looks like now: the base a copy records at open or Save
/// (criterion 6); `None` when it cannot be read.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn recovery_file_base(path: String) -> Option<RecoveryBase> {
    file_base(&PathBuf::from(path))
}

#[cfg(test)]
mod tests {
    use curvyo_document_core::{Document, pack_recovery};
    use curvyo_ui_core::{RECOVERY_FORMAT, encode_record};

    use super::*;

    const KEY: &str = "00112233445566778899aabbccddeeff";

    fn record(key: &str, name: Option<&str>, base: Option<RecoveryBase>, step: u64) -> Vec<u8> {
        encode_record(&RecoveryRecord {
            format_version: RECOVERY_FORMAT,
            key: key.to_owned(),
            name: name.map(str::to_owned),
            base,
            last_step_ms: step,
            pages: 2,
        })
    }

    fn put(folder: &RecoveryFolder, key: &str, copy: &[u8], record: &[u8]) {
        drop(folder.begin(key).unwrap());
        folder.write_copy(key, copy, record).unwrap();
    }

    #[test]
    fn a_copy_of_a_crashed_session_is_a_row_and_a_running_one_is_not() {
        let dir = tempfile::tempdir().unwrap();
        let folder = RecoveryFolder::new(dir.path().join("recovery"));
        let good = pack_recovery(&Document::new(1), "0.1.0").unwrap();
        put(
            &folder,
            KEY,
            &good,
            &record(KEY, Some("Poster.curvyo"), None, 5),
        );
        let running = "ffeeddccbbaa99887766554433221100";
        let _lock = folder.begin(running).unwrap();
        folder
            .write_copy(running, &good, &record(running, None, None, 9))
            .unwrap();
        let rows = listing(&folder).rows;
        assert_eq!(
            rows.len(),
            1,
            "a copy of a project open in another window is not offered"
        );
        assert_eq!(rows[0].key, KEY);
        assert_eq!(rows[0].name.as_deref(), Some("Poster.curvyo"));
        assert_eq!(rows[0].pages, 2);
        assert_eq!(rows[0].problem, None);
        assert!(!rows[0].changed);
    }

    #[test]
    fn a_damaged_copy_and_a_damaged_record_are_rows_that_cannot_be_restored() {
        let dir = tempfile::tempdir().unwrap();
        let folder = RecoveryFolder::new(dir.path().join("recovery"));
        let good = pack_recovery(&Document::new(1), "0.1.0").unwrap();
        put(
            &folder,
            KEY,
            &good[..good.len() / 2],
            &record(KEY, None, None, 5),
        );
        let other = "ffeeddccbbaa99887766554433221100";
        put(&folder, other, &good, b"{\"format_version\":1");
        let mut rows = listing(&folder).rows;
        rows.sort_by(|a, b| a.key.cmp(&b.key));
        assert_eq!(
            rows.iter().map(|r| r.problem).collect::<Vec<_>>(),
            [Some("damaged"), Some("damaged")]
        );
    }

    #[test]
    fn a_newer_record_says_so() {
        let dir = tempfile::tempdir().unwrap();
        let folder = RecoveryFolder::new(dir.path().join("recovery"));
        let good = pack_recovery(&Document::new(1), "0.1.0").unwrap();
        put(&folder, KEY, &good, b"{\"format_version\":99}");
        assert_eq!(listing(&folder).rows[0].problem, Some("newer"));
    }

    #[test]
    fn a_changed_or_missing_saved_file_is_marked() {
        let dir = tempfile::tempdir().unwrap();
        let folder = RecoveryFolder::new(dir.path().join("recovery"));
        let file = dir.path().join("Poster.curvyo");
        std::fs::write(&file, b"v1").unwrap();
        let base = file_base(&file).unwrap();
        let good = pack_recovery(&Document::new(1), "0.1.0").unwrap();
        put(
            &folder,
            KEY,
            &good,
            &record(KEY, Some("Poster.curvyo"), Some(base), 5),
        );
        assert!(!listing(&folder).rows[0].changed);
        std::fs::write(&file, b"version two").unwrap();
        assert!(listing(&folder).rows[0].changed, "the length differs");
        std::fs::remove_file(&file).unwrap();
        assert!(listing(&folder).rows[0].changed, "the file is gone");
    }

    #[test]
    fn rows_come_newest_first_and_the_oldest_beyond_twenty_are_pruned() {
        let dir = tempfile::tempdir().unwrap();
        let folder = RecoveryFolder::new(dir.path().join("recovery"));
        let good = pack_recovery(&Document::new(1), "0.1.0").unwrap();
        for n in 1..=23_u64 {
            let key = format!("{n:032x}");
            put(&folder, &key, &good, &record(&key, None, None, n));
        }
        let rows = listing(&folder).rows;
        assert_eq!(rows.len(), 21, "the newest plus 20");
        assert_eq!(rows[0].last_step_ms, 23);
        assert_eq!(rows[20].last_step_ms, 3);
        assert!(!folder.path().join(format!("{:032x}", 1)).exists());
    }

    #[test]
    fn a_full_disk_and_a_refused_folder_get_their_codes() {
        let full = RecoveryError::Io(std::io::Error::from(ErrorKind::StorageFull));
        let denied = RecoveryError::Io(std::io::Error::from(ErrorKind::PermissionDenied));
        assert_eq!(failure_code(&full), "disk-full");
        assert_eq!(failure_code(&denied), "no-permission");
    }
}
