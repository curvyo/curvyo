//! The commands of the maker's device group (`specs/0096-own-devices-sync` criteria 2, 10, 24,
//! 27 and 31, ADR 0040 decision 2): its files in `devices/` of the app data directory, the
//! default device name, the Synced workspaces folder, and the folder a synced workspace arrives
//! in. The page builds the keyring and list bytes; `curvyo-storage-io` checks and writes.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use curvyo_storage_io::{DEFAULT_SYNCED_FOLDER, DeviceGroupFiles, GroupRecord};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::DialogExt;

use crate::project::parented;
use crate::state::AppState;
use crate::workspace::{read_recents, recents_file, update_recents};
use crate::workspace_share::{JoinFolderAnswer, unhex};

/// The device group's files as the page reads and writes them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct GroupPayload {
    server: String,
    room: String,
    server_seq: u64,
    synced_version: String,
    synced_folder: String,
    /// Workspace id (hex) to the folder it is in on this device.
    folders: BTreeMap<String, String>,
    keyring_log: Vec<u8>,
    list: Vec<u8>,
}

/// How choosing the Synced workspaces folder ended.
#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub(crate) enum SyncedFolderAnswer {
    /// The folder exists and can take synced workspaces.
    Picked { path: String },
    /// The maker closed the picker.
    Cancelled,
    /// `inside-workspace` or `not-writable`.
    Refused { code: String },
}

fn data_dir(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map_err(|error| format!("the data folder is not available ({error})"))
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut out, b| {
        let _ = write!(out, "{b:02x}");
        out
    })
}

fn to_payload(files: DeviceGroupFiles) -> GroupPayload {
    GroupPayload {
        server: files.group.server,
        room: hex(&files.group.room),
        server_seq: files.group.server_seq,
        synced_version: files.group.synced_version,
        synced_folder: files.group.synced_folder,
        folders: files
            .group
            .folders
            .into_iter()
            .map(|(id, path)| (hex(&id), path))
            .collect(),
        keyring_log: files.keyring_log,
        list: files.list,
    }
}

fn from_payload(payload: GroupPayload) -> Result<DeviceGroupFiles, String> {
    let room = unhex(&payload.room).ok_or("not a room")?;
    let mut folders = BTreeMap::new();
    for (id, path) in payload.folders {
        folders.insert(unhex(&id).ok_or("not a workspace")?, path);
    }
    Ok(DeviceGroupFiles {
        group: GroupRecord {
            server: payload.server,
            room,
            server_seq: payload.server_seq,
            synced_version: payload.synced_version,
            synced_folder: payload.synced_folder,
            folders,
        },
        keyring_log: payload.keyring_log,
        list: payload.list,
    })
}

/// Reads the group's files under `dir`; `None` without a group.
pub(crate) fn read_group(dir: &Path) -> Result<Option<GroupPayload>, String> {
    curvyo_storage_io::read_device_group(dir)
        .map(|found| found.map(to_payload))
        .map_err(|error| error.to_string())
}

/// Writes the group's files under `dir`.
pub(crate) fn write_group(dir: &Path, payload: GroupPayload) -> Result<(), String> {
    let files = from_payload(payload)?;
    curvyo_storage_io::write_device_group(dir, &files).map_err(|error| error.to_string())
}

/// Checks and creates the Synced workspaces folder.
pub(crate) fn prepare_folder(folder: &Path) -> SyncedFolderAnswer {
    match curvyo_storage_io::prepare_synced_folder(folder) {
        Ok(()) => SyncedFolderAnswer::Picked {
            path: folder.display().to_string(),
        },
        Err(error) => SyncedFolderAnswer::Refused {
            code: error.code().to_owned(),
        },
    }
}

/// The workspace id in the marker of the folder `root`, `None` when it is no workspace this
/// build reads. Nothing is written.
pub(crate) fn marker_id(root: &Path) -> Option<[u8; 16]> {
    let marker = root
        .join(curvyo_storage_io::MARKER_DIR)
        .join(curvyo_storage_io::MARKER_FILE);
    let text = std::fs::read_to_string(marker).ok()?;
    curvyo_storage_io::parse_marker(&text).ok()
}

/// Keeps `root` (the open workspace when `None`) as the folder of the synced workspace `id`
/// when its marker names that id; the folder's path.
pub(crate) fn register(
    state: &AppState,
    id: &str,
    root: Option<PathBuf>,
) -> Result<String, String> {
    let wanted = unhex(id).ok_or("damaged")?;
    let root = root.or_else(|| state.workspace()).ok_or("not-found")?;
    if marker_id(&root) != Some(wanted) {
        return Err("not-found".to_owned());
    }
    let shown = root.display().to_string();
    state.register_synced(id, root);
    Ok(shown)
}

/// Creates the folder of the synced workspace `id` named `name` under `synced` without opening
/// it (criteria 10 and 11, `ux-notes.md` 4.4): " 2" on a clash, its marker, kept as the
/// workspace's folder and put first among the recent workspaces at `recents`.
pub(crate) fn create_synced(
    state: &AppState,
    recents: Option<&Path>,
    synced: &Path,
    name: &str,
    id: &str,
) -> JoinFolderAnswer {
    let refused = |code: &str| JoinFolderAnswer::Refused {
        code: code.to_owned(),
    };
    let Some(wanted) = unhex(id) else {
        return refused("not-found");
    };
    if let SyncedFolderAnswer::Refused { code } = prepare_folder(synced) {
        return JoinFolderAnswer::Refused { code };
    }
    let folder = curvyo_storage_io::arrival_folder(synced, name);
    if !folder.is_dir() && std::fs::create_dir(&folder).is_err() {
        return refused("not-writable");
    }
    if let Err(error) = curvyo_storage_io::create_joined_workspace(&folder, wanted) {
        return refused(&crate::workspace::error_code(&error));
    }
    if let Some(file) = recents {
        update_recents(file, |list| list.opened(&folder));
    }
    let path = folder.display().to_string();
    state.register_synced(id, folder);
    JoinFolderAnswer::Picked { path }
}

/// A folder on this device that already holds the workspace `id` (criterion 14): a recent
/// workspace or a folder directly under `synced` whose marker names it.
pub(crate) fn find_copy(recents: Option<&Path>, synced: &Path, id: &str) -> Option<String> {
    let wanted = unhex(id)?;
    let mut candidates: Vec<PathBuf> = recents
        .and_then(read_recents)
        .map(|list| list.rows().into_iter().map(|row| row.path).collect())
        .unwrap_or_default();
    if let Ok(entries) = std::fs::read_dir(synced) {
        let mut below: Vec<PathBuf> = entries.filter_map(|e| Some(e.ok()?.path())).collect();
        below.sort();
        candidates.extend(below);
    }
    candidates
        .into_iter()
        .find(|root| marker_id(root) == Some(wanted))
        .map(|root| root.display().to_string())
}

/// What choosing workspaces after Pair shows of a folder (criterion 8): its id (hex), its
/// documents, and whether it is shared (its marker holds the share files).
#[derive(Debug, PartialEq, Eq, Serialize)]
pub(crate) struct WorkspaceSummary {
    id: String,
    documents: usize,
    shared: bool,
    /// The marker's `keyring.log` as hex, empty when not shared: the page reads this device's
    /// role from it (ux-notes 3.1, "Not an admin here").
    keyring: String,
}

/// The summary of the workspace folder `root`, `None` when it is no workspace this build reads.
pub(crate) fn summary(root: &Path) -> Option<WorkspaceSummary> {
    let id = marker_id(root)?;
    let documents = curvyo_storage_io::scan_workspace(root)
        .ok()?
        .iter()
        .filter(|entry| entry.kind == curvyo_storage_io::ScannedKind::Document)
        .count();
    let share = curvyo_storage_io::read_marker_share(root).ok().flatten();
    Some(WorkspaceSummary {
        id: hex(&id),
        documents,
        shared: share.is_some(),
        keyring: share.map(|s| hex(&s.keyring_log)).unwrap_or_default(),
    })
}

/// The device group's files, `null` while this device is in no group.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn read_device_group(app: AppHandle) -> Result<Option<GroupPayload>, String> {
    read_group(&data_dir(&app)?)
}

/// Writes the device group's files.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn write_device_group(app: AppHandle, group: GroupPayload) -> Result<(), String> {
    write_group(&data_dir(&app)?, group)
}

/// Forgets the device group (Leave, or this device was removed); files of workspaces stay.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn remove_device_group(app: AppHandle) -> Result<(), String> {
    curvyo_storage_io::remove_device_group(&data_dir(&app)?).map_err(|error| error.to_string())
}

/// The default device name: the host name, or "This computer".
#[tauri::command]
pub(crate) fn device_host_name() -> String {
    let from_env = ["HOSTNAME", "COMPUTERNAME"]
        .iter()
        .find_map(|name| std::env::var(name).ok());
    let from_file = || {
        std::fs::read_to_string("/etc/hostname")
            .ok()
            .map(|text| text.trim().to_owned())
    };
    from_env
        .or_else(from_file)
        .filter(|name| !name.trim().is_empty())
        .map_or_else(|| "This computer".to_owned(), |name| name.trim().to_owned())
}

/// Prepares the Synced workspaces folder `path`, or `~/Curvyo` when empty (criterion 2).
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn prepare_synced_folder(app: AppHandle, path: String) -> SyncedFolderAnswer {
    let folder = if path.is_empty() {
        match app.path().home_dir() {
            Ok(home) => home.join(DEFAULT_SYNCED_FOLDER),
            Err(_) => {
                return SyncedFolderAnswer::Refused {
                    code: "not-writable".to_owned(),
                };
            }
        }
    } else {
        PathBuf::from(path)
    };
    prepare_folder(&folder)
}

/// Change folder… of the Synced workspaces folder: the OS picker, then the check.
#[tauri::command]
#[allow(clippy::unused_async)] // async keeps the blocking dialog off the main thread
pub(crate) async fn pick_synced_folder(app: AppHandle) -> SyncedFolderAnswer {
    let picked = parented(&app, app.dialog().file()).blocking_pick_folder();
    let Some(folder) = picked.and_then(|picked| picked.into_path().ok()) else {
        return SyncedFolderAnswer::Cancelled;
    };
    prepare_folder(&folder)
}

/// Registers the folder of the synced workspace `id` (hex): `path`, or the open workspace when
/// empty; the folder's path, or the code why not (criteria 11, 14, 20).
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn register_synced_workspace(
    app: AppHandle,
    id: String,
    path: String,
) -> Result<String, String> {
    let root = (!path.is_empty()).then(|| PathBuf::from(path));
    register(&app.state::<AppState>(), &id, root)
}

/// Creates the folder of an arriving synced workspace without opening it (criterion 10).
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn create_synced_workspace(
    app: AppHandle,
    id: String,
    synced: String,
    name: String,
) -> JoinFolderAnswer {
    create_synced(
        &app.state::<AppState>(),
        recents_file(&app).as_deref(),
        Path::new(&synced),
        &name,
        &id,
    )
}

/// The summary of the workspace folder `path`, or `null` (criterion 8).
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn workspace_summary(path: String) -> Option<WorkspaceSummary> {
    summary(Path::new(&path))
}

/// The folder that already holds a copy of the synced workspace `id`, or `null` (criterion 14).
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn find_workspace_copy(app: AppHandle, id: String, synced: String) -> Option<String> {
    find_copy(recents_file(&app).as_deref(), Path::new(&synced), &id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn payload() -> GroupPayload {
        GroupPayload {
            server: "ws://relay".to_owned(),
            room: "6e".repeat(16),
            server_seq: 3,
            synced_version: String::new(),
            synced_folder: "/home/maker/Curvyo".to_owned(),
            folders: BTreeMap::from([("77".repeat(16), "/home/maker/Curvyo/Box job".to_owned())]),
            keyring_log: vec![1],
            list: vec![2],
        }
    }

    /// Criterion 31: the files go to `devices/` of the data directory and read back.
    #[test]
    fn the_group_files_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(read_group(dir.path()), Ok(None));
        write_group(dir.path(), payload()).unwrap();
        assert_eq!(read_group(dir.path()), Ok(Some(payload())));
        let mut bad = payload();
        bad.room = "zz".to_owned();
        assert!(write_group(dir.path(), bad).is_err());
    }

    /// Criteria 10, 11 and 14: an arriving workspace's folder is made without opening it,
    /// registered by its id, listed among the recent workspaces, and found again as a copy; a
    /// folder whose marker names another id is not registered.
    #[test]
    fn a_synced_workspace_arrives_closed_and_is_found_as_a_copy() {
        let dir = tempfile::tempdir().unwrap();
        let synced = dir.path().join("Curvyo");
        let recents = dir.path().join("recent-workspaces.toml");
        let state = AppState::default();
        let id = "77".repeat(16);
        let answer = create_synced(&state, Some(&recents), &synced, "a:b", &id);
        let JoinFolderAnswer::Picked { path } = answer else {
            panic!("{answer:?}");
        };
        assert_eq!(state.workspace(), None, "nothing opened");
        assert_eq!(state.synced_root(&id), Some(PathBuf::from(&path)));
        assert_eq!(marker_id(Path::new(&path)), Some([0x77; 16]));
        assert_eq!(crate::workspace::recent_rows(&recents).len(), 1);
        assert_eq!(find_copy(Some(&recents), &synced, &id), Some(path.clone()));
        assert_eq!(find_copy(None, &synced, &id), Some(path.clone()));
        assert_eq!(find_copy(None, &synced, &"78".repeat(16)), None);

        let other = AppState::default();
        assert_eq!(
            register(&other, &"78".repeat(16), Some(PathBuf::from(&path))),
            Err("not-found".to_owned())
        );
        assert_eq!(
            register(&other, &id, Some(PathBuf::from(&path))),
            Ok(path.clone())
        );
        assert_eq!(
            summary(Path::new(&path)),
            Some(WorkspaceSummary {
                id: id.clone(),
                documents: 0,
                shared: false,
                keyring: String::new()
            })
        );
        let scanned = crate::workspace::scan_open(&other, Some(&id)).unwrap();
        assert_eq!(scanned, Vec::new());
        crate::workspace::apply_op(
            &other,
            Some(&id),
            &crate::workspace::OpPayload::CreateFolder {
                path: vec!["Sides".to_owned()],
            },
            &[],
        )
        .unwrap();
        assert!(Path::new(&path).join("Sides").is_dir());
        assert_eq!(
            crate::workspace::scan_open(&other, None),
            Err("not-found".to_owned())
        );
    }
}
