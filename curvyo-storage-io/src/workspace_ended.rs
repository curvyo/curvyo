//! The marker's ended record of a workspace this device was removed from or left
//! (`specs/0097-membership-lifecycle` criteria 15, 20 and 33; ADR 0041 decision 6).
//!
//! The page builds `share.ended.json`; this module writes it in place of the marker's
//! `share.json` and `keyring.log`. The list snapshot and `index.json` stay (Sync again resumes
//! from the record's cursor, which counts the list's records), and every document keeps its
//! `share.json` v2, so the documents keep their ids on the way back.

use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use crate::filesystem::write_atomic;
use crate::workspace::{MARKER_DIR, WorkspaceError, io_error};
use crate::workspace_share_files::{MARKER_KEYRING_FILE, MARKER_LIST_FILE, MARKER_SHARE_FILE};

/// The marker's ended record.
pub const MARKER_ENDED_FILE: &str = "share.ended.json";

/// What a removed or left workspace keeps for the way back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EndedMarker {
    /// `share.ended.json`.
    pub ended_json: Vec<u8>,
    /// The marker's list snapshot, empty when there is none.
    pub list: Vec<u8>,
}

/// Whether the marker holds an ended record: then a workspace without complete share files is
/// not shared.
pub(crate) fn has_ended_record(root: &Path) -> bool {
    root.join(MARKER_DIR).join(MARKER_ENDED_FILE).is_file()
}

/// The ended record and the list of the workspace at `root`, `None` when it keeps none or its
/// complete share files win over it ([`crate::read_marker_share`]).
///
/// # Errors
/// [`WorkspaceError::Io`].
pub fn read_marker_ended(root: &Path) -> Result<Option<EndedMarker>, WorkspaceError> {
    let dir = root.join(MARKER_DIR);
    if [MARKER_SHARE_FILE, MARKER_KEYRING_FILE, MARKER_LIST_FILE]
        .iter()
        .all(|name| dir.join(name).is_file())
    {
        return Ok(None);
    }
    let Some(ended_json) = read_optional(&dir.join(MARKER_ENDED_FILE))? else {
        return Ok(None);
    };
    let list = read_optional(&dir.join(MARKER_LIST_FILE))?.unwrap_or_default();
    Ok(Some(EndedMarker { ended_json, list }))
}

/// Ends the share of the workspace at `root`: writes `ended_json` by an atomic rename, then
/// removes the marker's `share.json` and `keyring.log`. Documents, the list and `index.json`
/// stay. Calling it again does no harm.
///
/// # Errors
/// [`WorkspaceError::Io`].
pub fn end_marker_share(root: &Path, ended_json: &[u8]) -> Result<(), WorkspaceError> {
    let dir = root.join(MARKER_DIR);
    let path = dir.join(MARKER_ENDED_FILE);
    write_atomic(&path, ended_json)
        .map_err(|error| io_error(&path, std::io::Error::other(error.to_string())))?;
    for name in [MARKER_SHARE_FILE, MARKER_KEYRING_FILE] {
        remove_if_there(&dir.join(name))?;
    }
    Ok(())
}

/// Removes the ended record (Sync again wrote the share files, or the way back is gone).
pub(crate) fn remove_ended_record(root: &Path) -> Result<(), WorkspaceError> {
    remove_if_there(&root.join(MARKER_DIR).join(MARKER_ENDED_FILE))
}

fn read_optional(path: &Path) -> Result<Option<Vec<u8>>, WorkspaceError> {
    match fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(io_error(path, error)),
    }
}

fn remove_if_there(path: &Path) -> Result<(), WorkspaceError> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(io_error(path, error)),
    }
}
