//! The commands of a shared workspace (`specs/0094-workspace` criteria 25, 27, 28 and 33): the
//! marker's share files, the fresh id and the documents Share writes, the `index.json` cache,
//! the background writer's file writes, and the folder a joined workspace lands in. The page
//! builds every byte; paths from the page are relative to the open workspace and checked by
//! `curvyo-storage-io`.

use std::path::Path;

use curvyo_storage_io::MarkerShare;
use serde::{Deserialize, Serialize};
use tauri::ipc::Request;
use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::DialogExt;

use crate::project::{parented, refresh_window_title};
use crate::raw_ipc::call_args;
use crate::state::AppState;
use crate::workspace::{WorkspaceAnswer, error_code, open_root, recents_file};

/// The marker's share files as the page reads and writes them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct MarkerPayload {
    share_json: Vec<u8>,
    keyring_log: Vec<u8>,
    list: Vec<u8>,
}

/// One document of `index.json`: its id as hex and its path.
#[derive(Debug, Deserialize)]
pub(crate) struct IndexPayload {
    document: String,
    path: Vec<String>,
}

/// How choosing the folder of a joined workspace ended.
#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub(crate) enum JoinFolderAnswer {
    /// The folder is kept for the join; its path as the page shows it.
    Picked { path: String },
    /// The maker closed the picker.
    Cancelled,
    /// The folder cannot take the workspace: `not-empty`, `inside-workspace`, `not-writable`.
    Refused { code: String },
}

fn root(state: &AppState) -> Result<std::path::PathBuf, String> {
    state.workspace().ok_or_else(|| "not-found".to_owned())
}

pub(crate) fn unhex(text: &str) -> Option<[u8; 16]> {
    if text.len() != 32 {
        return None;
    }
    let mut out = [0_u8; 16];
    for (index, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(text.get(index * 2..index * 2 + 2)?, 16).ok()?;
    }
    Some(out)
}

/// The open workspace's marker share files, `None` when it is not shared.
pub(crate) fn marker_of(state: &AppState) -> Result<Option<MarkerPayload>, String> {
    let root = root(state)?;
    curvyo_storage_io::read_marker_share(&root)
        .map(|found| {
            found.map(|share| MarkerPayload {
                share_json: share.share_json,
                keyring_log: share.keyring_log,
                list: share.list,
            })
        })
        .map_err(|error| error_code(&error))
}

/// Writes the open workspace's marker share files.
pub(crate) fn write_marker(state: &AppState, marker: MarkerPayload) -> Result<(), String> {
    let root = root(state)?;
    curvyo_storage_io::write_marker_share(
        &root,
        &MarkerShare {
            share_json: marker.share_json,
            keyring_log: marker.keyring_log,
            list: marker.list,
        },
    )
    .map_err(|error| error_code(&error))
}

/// Share's fresh id into `workspace.toml` (ADR 0036 decision 4).
pub(crate) fn write_id(state: &AppState, id: &str) -> Result<(), String> {
    let root = root(state)?;
    let id = unhex(id).ok_or_else(|| "damaged".to_owned())?;
    curvyo_storage_io::replace_workspace_id(&root, id).map_err(|error| error_code(&error))
}

/// Replaces the existing document at `path` by `bytes`.
pub(crate) fn replace(state: &AppState, path: &[String], bytes: &[u8]) -> Result<(), String> {
    let root = root(state)?;
    curvyo_storage_io::replace_document(&root, path, bytes).map_err(|error| error_code(&error))
}

/// Writes `index.json` for the documents named.
pub(crate) fn write_index(state: &AppState, documents: &[IndexPayload]) -> Result<(), String> {
    let root = root(state)?;
    let ids: Vec<([u8; 16], Vec<String>)> = documents
        .iter()
        .filter_map(|d| Some((unhex(&d.document)?, d.path.clone())))
        .collect();
    curvyo_storage_io::write_index(&root, &ids).map_err(|error| error_code(&error))
}

/// Checks `folder` for a joined workspace and keeps it.
pub(crate) fn choose_join_folder(state: &AppState, folder: &Path) -> JoinFolderAnswer {
    match curvyo_storage_io::check_join_folder(folder) {
        Ok(()) => {
            state.set_join_folder(Some(folder.to_path_buf()));
            JoinFolderAnswer::Picked {
                path: folder.display().to_string(),
            }
        }
        Err(error) => JoinFolderAnswer::Refused {
            code: error.code().to_owned(),
        },
    }
}

/// Makes the kept join folder the workspace `id` and opens it (criterion 27).
pub(crate) fn start_joined(state: &AppState, recents: Option<&Path>, id: &str) -> WorkspaceAnswer {
    let refused = |code: &str| WorkspaceAnswer::Refused {
        code: code.to_owned(),
    };
    let (Some(folder), Some(id)) = (state.join_folder(), unhex(id)) else {
        return refused("not-found");
    };
    if let Err(error) = curvyo_storage_io::create_joined_workspace(&folder, id) {
        return refused(&error_code(&error));
    }
    state.set_join_folder(None);
    open_root(state, recents, &folder)
}

/// The marker share files of the open workspace (criterion 33), `null` when not shared.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn workspace_marker_share(app: AppHandle) -> Result<Option<MarkerPayload>, String> {
    marker_of(&app.state::<AppState>())
}

/// Writes the marker share files (criteria 25, 28, 33).
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn write_workspace_marker_share(
    app: AppHandle,
    marker: MarkerPayload,
) -> Result<(), String> {
    write_marker(&app.state::<AppState>(), marker)
}

/// Writes Share's fresh workspace id (hex).
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn write_workspace_id(app: AppHandle, id: String) -> Result<(), String> {
    write_id(&app.state::<AppState>(), &id)
}

/// The arguments of `replace_workspace_document`.
#[derive(Debug, Deserialize)]
pub(crate) struct ReplaceArgs {
    path: Vec<String>,
}

/// Replaces an existing workspace document (Share, background writer); the bytes come as the
/// raw body (`raw_ipc`).
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn replace_workspace_document(
    app: AppHandle,
    request: Request<'_>,
) -> Result<(), String> {
    let (args, bytes): (ReplaceArgs, Vec<u8>) = call_args(&request)?;
    replace(&app.state::<AppState>(), &args.path, &bytes)
}

/// Writes `index.json`.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn write_workspace_index(
    app: AppHandle,
    documents: Vec<IndexPayload>,
) -> Result<(), String> {
    write_index(&app.state::<AppState>(), &documents)
}

/// Choose folder… of a join: the OS folder picker, then the folder's check.
#[tauri::command]
#[allow(clippy::unused_async)] // async keeps the blocking dialog off the main thread
pub(crate) async fn pick_join_folder(app: AppHandle) -> JoinFolderAnswer {
    let picked = parented(&app, app.dialog().file()).blocking_pick_folder();
    let Some(folder) = picked.and_then(|picked| picked.into_path().ok()) else {
        return JoinFolderAnswer::Cancelled;
    };
    choose_join_folder(&app.state::<AppState>(), &folder)
}

/// The join was admitted: the chosen folder becomes the open workspace `id` (hex).
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn start_joined_workspace(app: AppHandle, id: String) -> WorkspaceAnswer {
    let answer = start_joined(&app.state::<AppState>(), recents_file(&app).as_deref(), &id);
    refresh_window_title(&app);
    answer
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    /// Criteria 25 and 33: the marker files and the fresh id are written into the open
    /// workspace and read back.
    #[test]
    fn the_marker_share_files_belong_to_the_open_workspace() {
        let work = tempfile::tempdir().unwrap();
        let state = AppState::default();
        assert_eq!(marker_of(&state), Err("not-found".to_owned()));
        open_root(&state, None, work.path());
        assert_eq!(marker_of(&state), Ok(None));
        let marker = MarkerPayload {
            share_json: b"{}".to_vec(),
            keyring_log: vec![1],
            list: vec![2],
        };
        write_marker(&state, marker.clone()).unwrap();
        assert_eq!(marker_of(&state), Ok(Some(marker)));
        write_id(&state, &"57".repeat(16)).unwrap();
        let text =
            fs::read_to_string(work.path().join(".curvyo-workspace/workspace.toml")).unwrap();
        assert!(text.contains(&"57".repeat(16)));
    }

    /// Criterion 27: a refused folder is not kept; an accepted one becomes the open workspace
    /// with the link's id once the join starts.
    #[test]
    fn a_joined_workspace_opens_in_the_chosen_folder() {
        let outer = tempfile::tempdir().unwrap();
        let state = AppState::default();
        fs::write(outer.path().join("note.txt"), b"x").unwrap();
        assert_eq!(
            choose_join_folder(&state, outer.path()),
            JoinFolderAnswer::Refused {
                code: "not-empty".to_owned()
            }
        );
        let folder = outer.path().join("Box job");
        fs::create_dir(&folder).unwrap();
        assert!(matches!(
            choose_join_folder(&state, &folder),
            JoinFolderAnswer::Picked { .. }
        ));
        let answer = start_joined(&state, None, &"57".repeat(16));
        assert!(
            matches!(&answer, WorkspaceAnswer::Opened { name, entries } if name == "Box job" && entries.is_empty()),
            "{answer:?}"
        );
        assert_eq!(state.workspace().as_deref(), Some(folder.as_path()));
        assert_eq!(state.join_folder(), None);
    }
}
