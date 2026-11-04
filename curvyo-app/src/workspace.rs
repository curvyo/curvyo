//! The commands of a local workspace (`specs/0095-workspace` criteria 1 to 4, 6 to 9, 13 to 24):
//! pick or reopen the folder, scan it, run the file operations the page planned, read a
//! document's bytes or its thumbnail, and keep the recent workspaces. The folder lives in
//! `AppState`; every path from the page is relative to it and checked by `curvyo-storage-io`.
//! Nothing here uses the network (criterion 22).

use std::path::{Path, PathBuf};

use curvyo_storage_io::{
    DocumentProbe, RecentWorkspaces, RecentWorkspacesError, ScannedEntry, ScannedKind,
    WorkspaceError,
};
use serde::{Deserialize, Serialize};
use tauri::ipc::{Request, Response};
use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::DialogExt;

use crate::project::{parented, refresh_window_title};
use crate::raw_ipc::call_args;
use crate::state::AppState;

/// How opening a workspace ended, as the page receives it.
#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub(crate) enum WorkspaceAnswer {
    /// The folder is the open workspace now; its name and its scan.
    Opened {
        name: String,
        entries: Vec<ScannedPayload>,
    },
    /// The maker closed the folder picker.
    Cancelled,
    /// The folder was not opened; `code` names why (`not-writable`, `too-new`, `damaged`,
    /// `not-found`). Nothing was written.
    Refused { code: String },
}

/// One scanned entry as the page's `WasmWorkspace::set_scan` reads it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct ScannedPayload {
    path: Vec<String>,
    kind: &'static str,
    size: u64,
    modified: u64,
}

impl From<ScannedEntry> for ScannedPayload {
    fn from(entry: ScannedEntry) -> Self {
        Self {
            path: entry.path,
            kind: match entry.kind {
                ScannedKind::Folder => "folder",
                ScannedKind::Document => "document",
            },
            size: entry.size,
            modified: entry.modified,
        }
    }
}

/// One row of the recent workspaces.
#[derive(Debug, Serialize)]
pub(crate) struct RecentPayload {
    name: String,
    path: String,
    found: bool,
}

/// A file operation the page planned (`curvyo-editor-wasm`'s `WorkspaceGlue::plan`).
#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub(crate) enum OpPayload {
    CreateDocument { path: Vec<String> },
    CreateFolder { path: Vec<String> },
    Move { from: Vec<String>, to: Vec<String> },
    Trash { path: Vec<String> },
}

/// What a tree row shows of a document file.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ProbePayload {
    /// The PNG bytes of the container's thumbnail, or none.
    Readable { thumbnail: Option<Vec<u8>> },
    /// Why the file cannot be read: `not-curvyo` or `damaged`.
    Unreadable { code: &'static str },
}

/// The code the page maps to words for a failed operation.
pub(crate) fn error_code(error: &WorkspaceError) -> String {
    match error {
        WorkspaceError::NotAFolder | WorkspaceError::NotFound => "not-found",
        WorkspaceError::NotWritable => "not-writable",
        WorkspaceError::TooNew { .. } => "too-new",
        WorkspaceError::Damaged => "damaged",
        WorkspaceError::OutsideWorkspace => "outside",
        WorkspaceError::Exists => "exists",
        WorkspaceError::NoTrash(_) => "no-trash",
        WorkspaceError::Io { .. } => "io",
    }
    .to_owned()
}

/// Opens `root` as the workspace: the marker (created only when absent), the scan, the recents
/// file at `recents` moved to the front. A refusal leaves the open workspace as it was.
pub(crate) fn open_root(state: &AppState, recents: Option<&Path>, root: &Path) -> WorkspaceAnswer {
    let refused = |error: &WorkspaceError| WorkspaceAnswer::Refused {
        code: error_code(error),
    };
    let opened = curvyo_storage_io::fresh_workspace_id()
        .and_then(|fresh| curvyo_storage_io::open_workspace(root, fresh));
    if let Err(error) = opened {
        return refused(&error);
    }
    let entries = match curvyo_storage_io::scan_workspace(root) {
        Ok(entries) => entries,
        Err(error) => return refused(&error),
    };
    state.set_workspace(Some(root.to_path_buf()));
    if let Some(file) = recents {
        update_recents(file, |list| list.opened(root));
    }
    WorkspaceAnswer::Opened {
        name: folder_name(root),
        entries: entries.into_iter().map(ScannedPayload::from).collect(),
    }
}

/// The folder a command works in: the open workspace, or with `workspace` (an id as hex) the
/// registered synced workspace of that id, which need not be open
/// (`specs/0096-own-devices-sync`, ADR 0040 decision 7).
pub(crate) fn root_of(state: &AppState, workspace: Option<&str>) -> Result<PathBuf, String> {
    match workspace {
        None => state.workspace(),
        Some(id) => state.synced_root(id),
    }
    .ok_or_else(|| "not-found".to_owned())
}

/// The scan of the open workspace, or of the synced workspace `workspace`.
pub(crate) fn scan_open(
    state: &AppState,
    workspace: Option<&str>,
) -> Result<Vec<ScannedPayload>, String> {
    let root = root_of(state, workspace)?;
    curvyo_storage_io::scan_workspace(&root)
        .map(|entries| entries.into_iter().map(ScannedPayload::from).collect())
        .map_err(|error| error_code(&error))
}

/// Runs a planned operation in the open workspace; a new document gets `bytes`. The project's
/// path follows a move of its file or a folder above it.
pub(crate) fn apply_op(
    state: &AppState,
    workspace: Option<&str>,
    op: &OpPayload,
    bytes: &[u8],
) -> Result<(), String> {
    let root = root_of(state, workspace)?;
    let result = match op {
        OpPayload::CreateDocument { path } => {
            curvyo_storage_io::create_document(&root, path, bytes)
        }
        OpPayload::CreateFolder { path } => curvyo_storage_io::create_folder(&root, path),
        OpPayload::Move { from, to } => curvyo_storage_io::move_entry(&root, from, to),
        OpPayload::Trash { path } => curvyo_storage_io::trash_entry(&root, path),
    };
    result.map_err(|error| error_code(&error))?;
    if let OpPayload::Move { from, to } = op {
        state.follow_move(&join(&root, from), &join(&root, to));
    }
    Ok(())
}

/// The bytes of the document at `path` in the open workspace.
pub(crate) fn read_open(
    state: &AppState,
    workspace: Option<&str>,
    path: &[String],
) -> Result<Vec<u8>, String> {
    let root = root_of(state, workspace)?;
    curvyo_storage_io::read_document(&root, path).map_err(|error| error_code(&error))
}

/// What the row of the document at `path` shows.
pub(crate) fn probe_open(state: &AppState, path: &[String]) -> Result<ProbePayload, String> {
    let root = state.workspace().ok_or_else(|| "not-found".to_owned())?;
    match curvyo_storage_io::probe_document(&root, path) {
        Ok(DocumentProbe::Readable { thumbnail }) => Ok(ProbePayload::Readable { thumbnail }),
        Ok(DocumentProbe::NotCurvyo) => Ok(ProbePayload::Unreadable { code: "not-curvyo" }),
        Ok(DocumentProbe::Damaged) => Ok(ProbePayload::Unreadable { code: "damaged" }),
        Err(error) => Err(error_code(&error)),
    }
}

/// The recents file's rows; a newer or damaged file reads as no rows.
pub(crate) fn recent_rows(file: &Path) -> Vec<RecentPayload> {
    read_recents(file)
        .unwrap_or_default()
        .rows()
        .into_iter()
        .map(|row| RecentPayload {
            name: row.name,
            path: row.path.display().to_string(),
            found: row.found,
        })
        .collect()
}

/// The recents list in `file`: empty when there is none or it is damaged (the next write
/// replaces it), `None` when a newer build wrote it (never written over).
pub(crate) fn read_recents(file: &Path) -> Option<RecentWorkspaces> {
    let Ok(Some(bytes)) = curvyo_storage_io::read_optional(file) else {
        return Some(RecentWorkspaces::default());
    };
    match RecentWorkspaces::parse(&String::from_utf8_lossy(&bytes)) {
        Ok(list) => Some(list),
        Err(RecentWorkspacesError::Damaged) => Some(RecentWorkspaces::default()),
        Err(RecentWorkspacesError::Newer) => None,
    }
}

/// Changes the recents list in `file` with `change` and writes it back, unless a newer build
/// wrote it. A failed write only loses a shortcut.
pub(crate) fn update_recents(file: &Path, change: impl FnOnce(&mut RecentWorkspaces)) {
    let Some(mut list) = read_recents(file) else {
        return;
    };
    change(&mut list);
    let _ = curvyo_storage_io::ensure_parent_dir(file);
    let _ = curvyo_storage_io::write_atomic(file, list.to_text().as_bytes());
}

fn folder_name(root: &Path) -> String {
    root.file_name().map_or_else(
        || root.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    )
}

fn join(root: &Path, path: &[String]) -> PathBuf {
    path.iter()
        .fold(root.to_path_buf(), |dir, part| dir.join(part))
}

pub(crate) fn recents_file(app: &AppHandle) -> Option<PathBuf> {
    app.path()
        .app_data_dir()
        .ok()
        .map(|dir| dir.join(curvyo_storage_io::RECENT_WORKSPACES_FILE))
}

/// Open workspace…: the OS folder picker, then the folder as the workspace (criteria 1, 2, 4).
#[tauri::command]
#[allow(clippy::unused_async)] // async keeps the blocking dialog off the main thread
pub(crate) async fn pick_workspace(app: AppHandle) -> WorkspaceAnswer {
    let picked = parented(&app, app.dialog().file()).blocking_pick_folder();
    let Some(root) = picked.and_then(|picked| picked.into_path().ok()) else {
        return WorkspaceAnswer::Cancelled;
    };
    let answer = open_root(
        &app.state::<AppState>(),
        recents_file(&app).as_deref(),
        &root,
    );
    refresh_window_title(&app);
    answer
}

/// Opens the recent workspace at `index` (criterion 20).
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn open_recent_workspace(app: AppHandle, index: usize) -> WorkspaceAnswer {
    let file = recents_file(&app);
    let root = file
        .as_deref()
        .and_then(read_recents)
        .and_then(|list| list.path(index).map(Path::to_path_buf));
    let Some(root) = root else {
        return WorkspaceAnswer::Refused {
            code: "not-found".to_owned(),
        };
    };
    let answer = open_root(&app.state::<AppState>(), file.as_deref(), &root);
    refresh_window_title(&app);
    answer
}

/// The recent workspaces, most recent first (criterion 20).
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn recent_workspaces(app: AppHandle) -> Vec<RecentPayload> {
    recents_file(&app).map_or_else(Vec::new, |file| recent_rows(&file))
}

/// Removes the recent workspace at `index`; no folder is touched. The rows afterwards.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn remove_recent_workspace(app: AppHandle, index: usize) -> Vec<RecentPayload> {
    let Some(file) = recents_file(&app) else {
        return Vec::new();
    };
    update_recents(&file, |list| list.remove(index));
    recent_rows(&file)
}

/// Close workspace (criterion 21): the page has closed the document first.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn close_workspace(app: AppHandle) {
    app.state::<AppState>().set_workspace(None);
    refresh_window_title(&app);
}

/// The scan of the open workspace, polled by the page (criterion 7).
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn scan_open_workspace(
    app: AppHandle,
    workspace: Option<String>,
) -> Result<Vec<ScannedPayload>, String> {
    scan_open(&app.state::<AppState>(), workspace.as_deref())
}

/// The arguments of `apply_workspace_op`.
#[derive(Debug, Deserialize)]
pub(crate) struct ApplyArgs {
    op: OpPayload,
    /// A synced workspace that is not open, by id (hex); absent: the open workspace.
    #[serde(default)]
    workspace: Option<String>,
}

/// Runs a planned operation (criteria 13, 14, 16 to 19); a new document's bytes come as the raw
/// body (`raw_ipc`). The error is a code.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn apply_workspace_op(app: AppHandle, request: Request<'_>) -> Result<(), String> {
    let (args, bytes): (ApplyArgs, Vec<u8>) = call_args(&request)?;
    let result = apply_op(
        &app.state::<AppState>(),
        args.workspace.as_deref(),
        &args.op,
        &bytes,
    );
    refresh_window_title(&app);
    result
}

/// The bytes of a workspace document for the page to open (criterion 9), raw.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn read_workspace_document(
    app: AppHandle,
    path: Vec<String>,
    workspace: Option<String>,
) -> Result<Response, String> {
    read_open(&app.state::<AppState>(), workspace.as_deref(), &path).map(Response::new)
}

/// The page opened the workspace document at `path`: it is the project's file now, clean.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn confirm_workspace_document_opened(app: AppHandle, path: Vec<String>) {
    let state = app.state::<AppState>();
    if let Some(root) = state.workspace() {
        state.confirm_workspace_opened(join(&root, &path));
    }
    refresh_window_title(&app);
}

/// What the row of a workspace document shows (criteria 6, 8): the thumbnail's PNG bytes raw
/// (none: empty); an unreadable file is the error `not-curvyo` or `damaged`.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn probe_workspace_document(
    app: AppHandle,
    path: Vec<String>,
) -> Result<Response, String> {
    match probe_open(&app.state::<AppState>(), &path)? {
        ProbePayload::Readable { thumbnail } => Ok(Response::new(thumbnail.unwrap_or_default())),
        ProbePayload::Unreadable { code } => Err(code.to_owned()),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    fn p(text: &str) -> Vec<String> {
        text.split('/').map(str::to_owned).collect()
    }

    /// Criteria 1, 20 and 22: opening a folder makes it the workspace, lists its documents and
    /// puts it first among the recent workspaces, in the app's own file.
    #[test]
    fn opening_a_folder_makes_it_the_workspace_and_a_recent_one() {
        let data = tempfile::tempdir().unwrap();
        let work = tempfile::tempdir().unwrap();
        fs::write(work.path().join("Lid.curvyo"), b"lid").unwrap();
        let recents = data.path().join("recent-workspaces.toml");
        let state = AppState::default();

        let answer = open_root(&state, Some(&recents), work.path());
        let WorkspaceAnswer::Opened { entries, .. } = answer else {
            panic!("not opened: {answer:?}");
        };
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].path, p("Lid.curvyo"));
        assert_eq!(state.workspace().as_deref(), Some(work.path()));
        let rows = recent_rows(&recents);
        assert_eq!(rows.len(), 1);
        assert!(rows[0].found);
        assert!(
            work.path()
                .join(".curvyo-workspace/workspace.toml")
                .is_file()
        );
    }

    /// Criteria 4 and 23: a refused folder leaves the open workspace as it was.
    #[test]
    fn a_refused_folder_keeps_the_open_workspace() {
        let first = tempfile::tempdir().unwrap();
        let newer = tempfile::tempdir().unwrap();
        fs::create_dir(newer.path().join(".curvyo-workspace")).unwrap();
        fs::write(
            newer.path().join(".curvyo-workspace/workspace.toml"),
            "format_version = 9\nid = \"00\"\n",
        )
        .unwrap();
        let state = AppState::default();
        assert!(matches!(
            open_root(&state, None, first.path()),
            WorkspaceAnswer::Opened { .. }
        ));
        let answer = open_root(&state, None, newer.path());
        assert!(
            matches!(&answer, WorkspaceAnswer::Refused { code } if code == "too-new"),
            "{answer:?}"
        );
        assert_eq!(state.workspace().as_deref(), Some(first.path()));
    }

    /// Criteria 13, 14, 16, 17, 18: planned operations run inside the folder, never replace an
    /// entry, and the open document's path follows its move.
    #[test]
    fn operations_run_inside_the_workspace_and_the_open_path_follows_a_move() {
        let work = tempfile::tempdir().unwrap();
        let state = AppState::default();
        open_root(&state, None, work.path());
        apply_op(
            &state,
            None,
            &OpPayload::CreateDocument {
                path: p("Lid.curvyo"),
            },
            b"bytes",
        )
        .unwrap();
        apply_op(
            &state,
            None,
            &OpPayload::CreateFolder { path: p("Sides") },
            &[],
        )
        .unwrap();
        assert_eq!(read_open(&state, None, &p("Lid.curvyo")).unwrap(), b"bytes");
        state.confirm_opened(work.path().join("Lid.curvyo"));
        apply_op(
            &state,
            None,
            &OpPayload::Move {
                from: p("Lid.curvyo"),
                to: p("Sides/Lid.curvyo"),
            },
            &[],
        )
        .unwrap();
        assert_eq!(
            state.lock_project().path,
            Some(work.path().join("Sides/Lid.curvyo"))
        );
        assert_eq!(
            apply_op(
                &state,
                None,
                &OpPayload::CreateDocument {
                    path: p("Sides/Lid.curvyo")
                },
                b"other",
            ),
            Err("exists".to_owned())
        );
        assert_eq!(
            apply_op(
                &state,
                None,
                &OpPayload::CreateFolder {
                    path: p("../outside")
                },
                &[]
            ),
            Err("outside".to_owned())
        );
        let scanned = scan_open(&state, None).unwrap();
        assert_eq!(scanned.len(), 2);
        assert_eq!(
            probe_open(&state, &p("Sides/Lid.curvyo")).unwrap(),
            ProbePayload::Unreadable { code: "not-curvyo" }
        );
    }

    /// Criterion 21: without an open workspace, nothing is read or written.
    #[test]
    fn without_a_workspace_nothing_is_touched() {
        let state = AppState::default();
        assert_eq!(scan_open(&state, None), Err("not-found".to_owned()));
        assert_eq!(
            apply_op(&state, None, &OpPayload::CreateFolder { path: p("A") }, &[]),
            Err("not-found".to_owned())
        );
    }
}
