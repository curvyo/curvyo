//! The commands of a shared workspace's live tree (`specs/0095-workspace` criteria 35 to 41,
//! ADR 0037 decision 6): the scan with each file's sharing members, the kept copies of deleted
//! documents (keep and trash, read, write, restore). The page plans; paths are relative to the
//! open workspace and checked by `curvyo-storage-io`.

use std::collections::HashMap;
use std::sync::Mutex;

use curvyo_storage_io::ShareMembers;
use serde::{Deserialize, Serialize};
use tauri::ipc::{Request, Response};
use tauri::{AppHandle, Manager, State};

use crate::raw_ipc::call_args;
use crate::state::AppState;
use crate::workspace::{ScannedPayload, error_code};
use crate::workspace_share::unhex;

/// The sharing members read per file, kept while the file's size and time stay the same, so a
/// scan every 2 s opens only the files that changed.
#[derive(Default)]
pub(crate) struct ShareCache(Mutex<HashMap<Vec<String>, (u64, u64, ShareMembers)>>);

/// One document file of a shared scan: its `share.json` text, whether it has a keyring.
#[derive(Debug, PartialEq, Eq, Serialize)]
pub(crate) struct SharedFilePayload {
    path: Vec<String>,
    share: Option<String>,
    keyring: bool,
}

/// One document `index.json` names.
#[derive(Debug, PartialEq, Eq, Serialize)]
pub(crate) struct KnownPayload {
    document: String,
    path: Vec<String>,
}

/// The scan of a shared workspace: the rows' entries, the folders, every document file with
/// its sharing members, and where this computer last saw each document.
#[derive(Debug, Serialize)]
pub(crate) struct SharedScanPayload {
    entries: Vec<ScannedPayload>,
    folders: Vec<Vec<String>>,
    documents: Vec<SharedFilePayload>,
    known: Vec<KnownPayload>,
}

/// The scan of the open shared workspace.
pub(crate) fn shared_scan(
    state: &AppState,
    cache: &ShareCache,
) -> Result<SharedScanPayload, String> {
    let root = state.workspace().ok_or_else(|| "not-found".to_owned())?;
    let entries = curvyo_storage_io::scan_workspace(&root).map_err(|error| error_code(&error))?;
    let mut kept = cache
        .0
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut fresh = HashMap::new();
    let mut folders = Vec::new();
    let mut documents = Vec::new();
    for entry in &entries {
        if entry.kind == curvyo_storage_io::ScannedKind::Folder {
            folders.push(entry.path.clone());
            continue;
        }
        let members = match kept.get(&entry.path) {
            Some((size, modified, members))
                if *size == entry.size && *modified == entry.modified =>
            {
                members.clone()
            }
            _ => curvyo_storage_io::share_members(&root, &entry.path).unwrap_or_default(),
        };
        documents.push(SharedFilePayload {
            path: entry.path.clone(),
            share: members
                .share_json
                .as_ref()
                .map(|bytes| String::from_utf8_lossy(bytes).into_owned()),
            keyring: members.keyring,
        });
        fresh.insert(entry.path.clone(), (entry.size, entry.modified, members));
    }
    *kept = fresh;
    let known = curvyo_storage_io::read_index(&root)
        .into_iter()
        .map(|(id, path)| KnownPayload {
            document: hex(&id),
            path,
        })
        .collect();
    Ok(SharedScanPayload {
        entries: entries.into_iter().map(ScannedPayload::from).collect(),
        folders,
        documents,
        known,
    })
}

fn hex(id: &[u8; 16]) -> String {
    use std::fmt::Write as _;
    id.iter().fold(String::new(), |mut text, byte| {
        // invariant: writing into a String cannot fail.
        let _ = write!(text, "{byte:02x}");
        text
    })
}

fn root(state: &AppState) -> Result<std::path::PathBuf, String> {
    state.workspace().ok_or_else(|| "not-found".to_owned())
}

fn id(document: &str) -> Result<[u8; 16], String> {
    unhex(document).ok_or_else(|| "damaged".to_owned())
}

/// The scan of the open shared workspace, polled by the page (criteria 36, 41).
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn scan_shared_workspace(
    app: AppHandle,
    cache: State<'_, ShareCache>,
) -> Result<SharedScanPayload, String> {
    shared_scan(&app.state::<AppState>(), &cache)
}

/// Keeps a copy of the deleted document `document` at `path`; with `trash`, its file then goes
/// to the OS trash (criterion 40).
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn keep_workspace_document(
    app: AppHandle,
    path: Vec<String>,
    document: String,
    trash: bool,
) -> Result<(), String> {
    let root = root(&app.state::<AppState>())?;
    let id = id(&document)?;
    let result = if trash {
        curvyo_storage_io::keep_deleted(&root, &path, id)
    } else {
        curvyo_storage_io::read_document(&root, &path)
            .and_then(|bytes| curvyo_storage_io::write_deleted(&root, id, &bytes))
    };
    result.map_err(|error| error_code(&error))
}

/// Restore: writes the kept copy of `document` at `path` again (criterion 40); `not-found` when
/// none is kept.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn restore_workspace_document(
    app: AppHandle,
    document: String,
    path: Vec<String>,
) -> Result<(), String> {
    let root = root(&app.state::<AppState>())?;
    curvyo_storage_io::restore_deleted(&root, id(&document)?, &path)
        .map_err(|error| error_code(&error))
}

/// The kept copy of the deleted document `document`, raw.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn read_deleted_document(app: AppHandle, document: String) -> Result<Response, String> {
    let root = root(&app.state::<AppState>())?;
    curvyo_storage_io::read_deleted(&root, id(&document)?)
        .map(Response::new)
        .map_err(|error| error_code(&error))
}

/// This device was removed from the shared workspace (criterion 44): the marker's share files
/// go and the workspace is a local one; every document stays.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn clear_workspace_marker_share(app: AppHandle) -> Result<(), String> {
    let root = root(&app.state::<AppState>())?;
    curvyo_storage_io::remove_marker_share(&root).map_err(|error| error_code(&error))
}

/// The arguments of `write_deleted_document`.
#[derive(Debug, Deserialize)]
pub(crate) struct DeletedArgs {
    document: String,
}

/// Writes the kept copy of a deleted document (a background write, a join); the bytes come as
/// the raw body.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn write_deleted_document(app: AppHandle, request: Request<'_>) -> Result<(), String> {
    let (args, bytes): (DeletedArgs, Vec<u8>) = call_args(&request)?;
    let root = root(&app.state::<AppState>())?;
    curvyo_storage_io::write_deleted(&root, id(&args.document)?, &bytes)
        .map_err(|error| error_code(&error))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use curvyo_document_core::{Document, WorkspaceShareRecord, pack};

    use super::*;

    /// Criteria 36 and 41: the shared scan names each file's `share.json` and the index; an
    /// unchanged file is not opened again.
    #[test]
    fn the_shared_scan_reads_each_files_share_record_once() {
        let work = tempfile::tempdir().unwrap();
        let root = work.path().canonicalize().unwrap();
        curvyo_storage_io::open_workspace(&root, [7; 16]).unwrap();
        let document = Document::new(3);
        document.set_workspace_share(Some(WorkspaceShareRecord {
            workspace: [0x57; 16],
            document: [0x11; 16],
            synced_version: document.version(),
        }));
        fs::create_dir(root.join("Sides")).unwrap();
        fs::write(root.join("Lid.curvyo"), pack(&document, "t").unwrap()).unwrap();
        fs::write(root.join("Sides").join("Plain.curvyo"), b"plain").unwrap();
        curvyo_storage_io::write_index(&root, &[([0x11; 16], vec!["Lid.curvyo".to_owned()])])
            .unwrap();
        let state = AppState::default();
        state.set_workspace(Some(root));
        let cache = ShareCache::default();
        let scan = shared_scan(&state, &cache).unwrap();
        assert_eq!(scan.folders, vec![vec!["Sides".to_owned()]]);
        let lid = &scan.documents[0];
        assert!(
            lid.share
                .as_deref()
                .unwrap()
                .contains("\"format_version\":2")
        );
        assert!(!lid.keyring);
        assert_eq!(scan.documents[1].share, None);
        assert_eq!(
            scan.known,
            vec![KnownPayload {
                document: "11".repeat(16),
                path: vec!["Lid.curvyo".to_owned()]
            }]
        );
        assert_eq!(cache.0.lock().unwrap().len(), 2);
        let again = shared_scan(&state, &cache).unwrap();
        assert_eq!(again.documents, scan.documents);
    }
}
