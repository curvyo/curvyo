//! The disk side of the maker's device group (ADR 0040 decision 2, `specs/0096-own-devices-sync`
//! criteria 2, 10 and 31): `devices/` in the app data directory with `group.json`,
//! `keyring.log` and `devices.loro`, and the Synced workspaces folder a synced workspace arrives
//! in.
//!
//! The page builds `keyring.log` and `devices.loro`; `group.json` is encoded here. Every file is
//! written temp-then-rename. The group's epoch keys are not here: they live in the credential
//! store like any room's keys (criterion 31).

use std::collections::BTreeMap;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::filesystem::write_atomic;
use crate::workspace::{MARKER_DIR, hex, probe_writable, unhex};

/// The folder in the app data directory.
pub const DEVICES_DIR: &str = "devices";
/// The group's record.
pub const GROUP_FILE: &str = "group.json";
/// The group room's keyring.
pub const GROUP_KEYRING_FILE: &str = "keyring.log";
/// The devices list's snapshot.
pub const DEVICES_LIST_FILE: &str = "devices.loro";
/// The `format_version` of `group.json` this build writes.
pub const GROUP_FORMAT_VERSION: u32 = 1;
/// The default Synced workspaces folder, under the home folder (Proposal).
pub const DEFAULT_SYNCED_FOLDER: &str = "Curvyo";

/// Why the device group's files cannot be read or written.
#[derive(Debug, thiserror::Error)]
pub enum DevicesError {
    /// `group.json` has a higher `format_version` than this build reads.
    #[error("the device group was written by a newer Curvyo (format {found})")]
    TooNew {
        /// The version found.
        found: u32,
    },
    /// A file cannot be read as what it should be.
    #[error("the device group's files are damaged")]
    Damaged,
    /// Any I/O failure.
    #[error("failed to access {path}: {message}")]
    Io {
        /// The path.
        path: String,
        /// What failed.
        message: String,
    },
}

/// Why a folder cannot be the Synced workspaces folder (`ux-notes.md` section 4.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SyncedFolderError {
    /// It is a workspace or lies inside one.
    #[error("choose a folder outside any workspace")]
    InsideWorkspace,
    /// It cannot be created or written, or is no folder.
    #[error("cannot write to this folder")]
    NotWritable,
}

impl SyncedFolderError {
    /// The code the page turns into words.
    #[must_use]
    pub fn code(self) -> &'static str {
        match self {
            Self::InsideWorkspace => "inside-workspace",
            Self::NotWritable => "not-writable",
        }
    }
}

/// `group.json`: where the group room is and how far this device read it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupRecord {
    /// The relay address.
    pub server: String,
    /// The group room.
    pub room: [u8; 16],
    /// The relay sequence up to which the devices list is in `devices.loro`.
    pub server_seq: u64,
    /// The devices list's version the room acknowledged last, as text.
    pub synced_version: String,
    /// The Synced workspaces folder (empty in a browser).
    pub synced_folder: String,
    /// The folder each synced workspace is in on this device.
    pub folders: BTreeMap<[u8; 16], String>,
}

#[derive(Serialize, Deserialize)]
struct GroupJson {
    format_version: u32,
    server: String,
    room: String,
    server_seq: u64,
    synced_version: String,
    synced_folder: String,
    folders: BTreeMap<String, String>,
}

#[derive(Deserialize)]
struct VersionOnly {
    format_version: u32,
}

impl GroupRecord {
    /// The bytes of `group.json`.
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let json = GroupJson {
            format_version: GROUP_FORMAT_VERSION,
            server: self.server.clone(),
            room: hex(&self.room),
            server_seq: self.server_seq,
            synced_version: self.synced_version.clone(),
            synced_folder: self.synced_folder.clone(),
            folders: self
                .folders
                .iter()
                .map(|(id, path)| (hex(id), path.clone()))
                .collect(),
        };
        // invariant: a struct of strings, numbers and a string map always serialises.
        serde_json::to_vec_pretty(&json).unwrap_or_default()
    }

    /// Reads `group.json`.
    ///
    /// # Errors
    /// `TooNew` for a higher `format_version`; `Damaged` otherwise.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, DevicesError> {
        let version: VersionOnly =
            serde_json::from_slice(bytes).map_err(|_| DevicesError::Damaged)?;
        if version.format_version > GROUP_FORMAT_VERSION {
            return Err(DevicesError::TooNew {
                found: version.format_version,
            });
        }
        if version.format_version != GROUP_FORMAT_VERSION {
            return Err(DevicesError::Damaged);
        }
        let json: GroupJson = serde_json::from_slice(bytes).map_err(|_| DevicesError::Damaged)?;
        let room = unhex(&json.room).ok_or(DevicesError::Damaged)?;
        let mut folders = BTreeMap::new();
        for (id, path) in json.folders {
            folders.insert(unhex(&id).ok_or(DevicesError::Damaged)?, path);
        }
        Ok(Self {
            server: json.server,
            room,
            server_seq: json.server_seq,
            synced_version: json.synced_version,
            synced_folder: json.synced_folder,
            folders,
        })
    }
}

/// The three files of the device group as the page reads and writes them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceGroupFiles {
    /// `group.json`.
    pub group: GroupRecord,
    /// `keyring.log`.
    pub keyring_log: Vec<u8>,
    /// `devices.loro`.
    pub list: Vec<u8>,
}

fn io(path: &Path, error: &impl ToString) -> DevicesError {
    DevicesError::Io {
        path: path.display().to_string(),
        message: error.to_string(),
    }
}

/// The device group's files under `app_data`, `None` while this device is in no group.
///
/// # Errors
/// `TooNew`, `Damaged` (a file missing next to `group.json` included), `Io`.
pub fn read_device_group(app_data: &Path) -> Result<Option<DeviceGroupFiles>, DevicesError> {
    let dir = app_data.join(DEVICES_DIR);
    let group_path = dir.join(GROUP_FILE);
    let group = match fs::read(&group_path) {
        Ok(bytes) => GroupRecord::from_bytes(&bytes)?,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(io(&group_path, &error)),
    };
    let read = |name: &str| {
        let path = dir.join(name);
        fs::read(&path).map_err(|error| match error.kind() {
            ErrorKind::NotFound => DevicesError::Damaged,
            _ => io(&path, &error),
        })
    };
    Ok(Some(DeviceGroupFiles {
        group,
        keyring_log: read(GROUP_KEYRING_FILE)?,
        list: read(DEVICES_LIST_FILE)?,
    }))
}

/// Writes the device group's files under `app_data`, each temp-then-rename; `group.json` last,
/// so a crash in between leaves the previous record over newer data, never the reverse.
///
/// # Errors
/// `Io`.
pub fn write_device_group(app_data: &Path, files: &DeviceGroupFiles) -> Result<(), DevicesError> {
    let dir = app_data.join(DEVICES_DIR);
    fs::create_dir_all(&dir).map_err(|error| io(&dir, &error))?;
    for (name, bytes) in [
        (GROUP_KEYRING_FILE, files.keyring_log.as_slice()),
        (DEVICES_LIST_FILE, files.list.as_slice()),
        (GROUP_FILE, files.group.to_bytes().as_slice()),
    ] {
        let path = dir.join(name);
        write_atomic(&path, bytes).map_err(|error| io(&path, &error))?;
    }
    Ok(())
}

/// Forgets the device group (Leave my devices, or this device was removed): `devices/` is
/// deleted; the workspaces' files stay (criteria 24 and 27).
///
/// # Errors
/// `Io`.
pub fn remove_device_group(app_data: &Path) -> Result<(), DevicesError> {
    let dir = app_data.join(DEVICES_DIR);
    match fs::remove_dir_all(&dir) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(io(&dir, &error)),
    }
}

/// Makes `folder` the Synced workspaces folder: created when absent, never a workspace or inside
/// one, writable (criterion 2).
///
/// # Errors
/// The reason it cannot be.
pub fn prepare_synced_folder(folder: &Path) -> Result<(), SyncedFolderError> {
    if folder.ancestors().any(|dir| dir.join(MARKER_DIR).exists()) {
        return Err(SyncedFolderError::InsideWorkspace);
    }
    fs::create_dir_all(folder).map_err(|_| SyncedFolderError::NotWritable)?;
    if !folder.is_dir() {
        return Err(SyncedFolderError::NotWritable);
    }
    probe_writable(folder).map_err(|_| SyncedFolderError::NotWritable)
}

/// The folder a synced workspace named `name` arrives in under `synced`: the name made safe for
/// every desktop OS, and " 2", " 3", ... when it is taken (criteria 10 and 15). An empty folder
/// of that name is not taken: an arrival that did not get in leaves one behind, and the next
/// try uses it again.
#[must_use]
pub fn arrival_folder(synced: &Path, name: &str) -> PathBuf {
    let taken = |path: &Path| {
        path.exists() && fs::read_dir(path).map_or(true, |mut entries| entries.next().is_some())
    };
    let base = safe_folder_name(name);
    let mut candidate = synced.join(&base);
    let mut number = 2_u32;
    while taken(&candidate) {
        candidate = synced.join(format!("{base} {number}"));
        number += 1;
    }
    candidate
}

/// The longest name [`safe_folder_name`] gives, in UTF-8 bytes: Linux takes 255 bytes per name
/// (Windows and macOS 255 UTF-16 units, never more than the bytes), less room for the longest
/// " N" suffix of [`arrival_folder`] (" 4294967295").
const MAX_NAME_BYTES: usize = 255 - 11;

/// `name` with the characters Windows, macOS or Linux refuse in a folder name replaced by `-`,
/// cut to 244 bytes on a character boundary (room for a " N" suffix within 255), trailing dots
/// and spaces dropped, and a reserved Windows name prefixed; "Workspace" when nothing is left.
#[must_use]
pub fn safe_folder_name(name: &str) -> String {
    let mut replaced: String = name
        .chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '-',
            c if c.is_control() => '-',
            c => c,
        })
        .collect();
    let mut end = MAX_NAME_BYTES.min(replaced.len());
    while !replaced.is_char_boundary(end) {
        end -= 1;
    }
    replaced.truncate(end);
    let trimmed = replaced.trim().trim_end_matches(['.', ' ']).to_owned();
    if trimmed.is_empty() {
        return "Workspace".to_owned();
    }
    let stem = trimmed
        .split('.')
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    let reserved = matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (stem.len() == 4
            && (stem.starts_with("COM") || stem.starts_with("LPT"))
            && stem.as_bytes()[3].is_ascii_digit());
    if reserved {
        format!("_{trimmed}")
    } else {
        trimmed
    }
}
