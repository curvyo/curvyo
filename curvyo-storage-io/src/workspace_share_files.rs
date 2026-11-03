//! The disk side of a shared workspace (`specs/0094-workspace` criteria 25, 27, 28 and 33; ADR
//! 0036 decisions 1, 4 and 9): the marker's share files, the fresh id Share writes, a document
//! file replaced by its shared bytes, the `index.json` cache, and the folder a joined workspace
//! lands in.
//!
//! The page builds every byte that is written here (`share.json`, `keyring.log`, `list.loro`,
//! the documents); this module only checks the places and writes them atomically.

use std::collections::BTreeMap;
use std::fs;
use std::io::ErrorKind;
use std::path::Path;
use std::time::UNIX_EPOCH;

use serde::{Deserialize, Serialize};

use crate::filesystem::write_atomic;
use crate::workspace::{
    MARKER_DIR, MARKER_FILE, ScannedKind, WorkspaceError, create_new_file, hex, io_error,
    marker_text, open_workspace, parse_marker, probe_writable, resolve, scan_workspace,
};
use crate::workspace_documents::SHARE_MEMBER;
use crate::{DocumentProbe, probe_document, share_members, with_thumbnail};

/// The marker's `share.json`.
pub const MARKER_SHARE_FILE: &str = "share.json";
/// The marker's `keyring.log`.
pub const MARKER_KEYRING_FILE: &str = "keyring.log";
/// The marker's list snapshot.
pub const MARKER_LIST_FILE: &str = "list.loro";
/// The marker's cache of document id to file.
pub const MARKER_INDEX_FILE: &str = "index.json";
/// The `format_version` of `index.json` this build writes.
pub const INDEX_FORMAT_VERSION: u32 = 1;

/// The marker's files of a shared workspace, as the page writes and reads them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkerShare {
    /// `share.json`.
    pub share_json: Vec<u8>,
    /// `keyring.log`.
    pub keyring_log: Vec<u8>,
    /// `list.loro`.
    pub list: Vec<u8>,
}

/// Why a folder cannot take a joined workspace (`ux-notes.md` section 8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum JoinFolderError {
    /// It holds entries.
    #[error("choose an empty folder")]
    NotEmpty,
    /// It is a workspace or lies inside one.
    #[error("choose a folder outside any workspace")]
    InsideWorkspace,
    /// It cannot be written, or is no folder.
    #[error("cannot write to this folder")]
    NotWritable,
}

impl JoinFolderError {
    /// The code the page maps to words.
    #[must_use]
    pub fn code(self) -> &'static str {
        match self {
            Self::NotEmpty => "not-empty",
            Self::InsideWorkspace => "inside-workspace",
            Self::NotWritable => "not-writable",
        }
    }
}

/// The marker's share files, `None` for a workspace that is not shared (all three absent).
///
/// # Errors
/// [`WorkspaceError::Damaged`] when only some of them exist; [`WorkspaceError::Io`].
pub fn read_marker_share(root: &Path) -> Result<Option<MarkerShare>, WorkspaceError> {
    let dir = root.join(MARKER_DIR);
    let read = |name: &str| match fs::read(dir.join(name)) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(source) => Err(io_error(&dir.join(name), source)),
    };
    match (
        read(MARKER_SHARE_FILE)?,
        read(MARKER_KEYRING_FILE)?,
        read(MARKER_LIST_FILE)?,
    ) {
        (None, None, None) => Ok(None),
        (Some(share_json), Some(keyring_log), Some(list)) => Ok(Some(MarkerShare {
            share_json,
            keyring_log,
            list,
        })),
        _ => Err(WorkspaceError::Damaged),
    }
}

/// Writes the marker's share files, each by an atomic rename (ADR 0036 decision 1).
///
/// # Errors
/// [`WorkspaceError::Io`].
pub fn write_marker_share(root: &Path, share: &MarkerShare) -> Result<(), WorkspaceError> {
    let dir = root.join(MARKER_DIR);
    for (name, bytes) in [
        (MARKER_KEYRING_FILE, &share.keyring_log),
        (MARKER_LIST_FILE, &share.list),
        (MARKER_SHARE_FILE, &share.share_json),
    ] {
        let path = dir.join(name);
        write_atomic(&path, bytes)
            .map_err(|error| io_error(&path, std::io::Error::other(error.to_string())))?;
    }
    Ok(())
}

/// Writes `id` into `workspace.toml`: Share gives the workspace a fresh id (ADR 0036 decision
/// 4), so a folder copied before sharing never meets its copy on the relay.
///
/// # Errors
/// [`WorkspaceError::Io`].
pub fn replace_workspace_id(root: &Path, id: [u8; 16]) -> Result<(), WorkspaceError> {
    let path = root.join(MARKER_DIR).join(MARKER_FILE);
    write_atomic(&path, marker_text(id).as_bytes())
        .map_err(|error| io_error(&path, std::io::Error::other(error.to_string())))
}

/// Replaces the existing document at `path` by `bytes` (a shared copy or a background write),
/// atomically; the file's thumbnail is kept when the new bytes have none.
///
/// # Errors
/// [`WorkspaceError::OutsideWorkspace`], [`WorkspaceError::NotFound`], [`WorkspaceError::Io`].
pub fn replace_document(root: &Path, path: &[String], bytes: &[u8]) -> Result<(), WorkspaceError> {
    let target = resolve(root, path)?;
    let old = match probe_document(root, path)? {
        DocumentProbe::Readable { thumbnail } => thumbnail,
        DocumentProbe::NotCurvyo | DocumentProbe::Damaged => None,
    };
    let kept = old
        .and_then(|png| with_thumbnail_if_absent(bytes, &png))
        .unwrap_or_else(|| bytes.to_vec());
    write_atomic(&target, &kept)
        .map_err(|error| io_error(&target, std::io::Error::other(error.to_string())))
}

/// `bytes` with `png` as their thumbnail, `None` when they carry one already or are no
/// container.
fn with_thumbnail_if_absent(bytes: &[u8], png: &[u8]) -> Option<Vec<u8>> {
    let has_one = zip::ZipArchive::new(std::io::Cursor::new(bytes))
        .ok()?
        .by_name(crate::workspace_documents::THUMBNAIL_MEMBER)
        .is_ok();
    if has_one {
        return None;
    }
    with_thumbnail(bytes, png).ok()
}

/// One entry of `index.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexEntry {
    /// The file's path in the workspace.
    pub path: Vec<String>,
    /// Its size in bytes.
    pub size: u64,
    /// Its last change, unix seconds.
    pub modified: u64,
}

#[derive(Serialize, Deserialize)]
struct IndexFile {
    format_version: u32,
    documents: BTreeMap<String, IndexEntry>,
}

/// The `index.json` text of `documents` (document id to file), a cache rebuilt from the
/// documents' `share.json` when missing, damaged or newer (ADR 0036 decision 1).
///
/// # Panics
/// Does not panic in practice: the map holds only strings and numbers.
#[must_use]
pub fn index_text(documents: &BTreeMap<[u8; 16], IndexEntry>) -> String {
    let file = IndexFile {
        format_version: INDEX_FORMAT_VERSION,
        documents: documents
            .iter()
            .map(|(id, entry)| (hex(id), entry.clone()))
            .collect(),
    };
    // invariant: strings and numbers always serialise.
    #[allow(clippy::expect_used)]
    serde_json::to_string_pretty(&file).expect("index.json serialises")
}

/// Writes `index.json` for the documents `ids` (id, path), with each file's size and time.
///
/// # Errors
/// [`WorkspaceError::OutsideWorkspace`], [`WorkspaceError::Io`].
pub fn write_index(root: &Path, ids: &[([u8; 16], Vec<String>)]) -> Result<(), WorkspaceError> {
    let mut documents = BTreeMap::new();
    for (id, path) in ids {
        let target = resolve(root, path)?;
        let Ok(meta) = fs::metadata(&target) else {
            continue;
        };
        let modified = meta
            .modified()
            .ok()
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .map_or(0, |since| since.as_secs());
        documents.insert(
            *id,
            IndexEntry {
                path: path.clone(),
                size: meta.len(),
                modified,
            },
        );
    }
    let path = root.join(MARKER_DIR).join(MARKER_INDEX_FILE);
    write_atomic(&path, index_text(&documents).as_bytes())
        .map_err(|error| io_error(&path, std::io::Error::other(error.to_string())))
}

/// A removed member's workspace becomes local (`specs/0094-workspace` criterion 44, ADR 0036
/// decision 9): every document whose `share.json` names this workspace loses it (a local file),
/// then the marker's share files go; `workspace.toml`, the documents' content and the kept
/// copies of deleted documents stay.
///
/// # Errors
/// [`WorkspaceError::Io`].
pub fn remove_marker_share(root: &Path) -> Result<(), WorkspaceError> {
    drop_document_shares(root)?;
    for name in [
        MARKER_SHARE_FILE,
        MARKER_KEYRING_FILE,
        MARKER_LIST_FILE,
        MARKER_INDEX_FILE,
    ] {
        let path = root.join(MARKER_DIR).join(name);
        match fs::remove_file(&path) {
            Ok(()) => {}
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(error) => return Err(io_error(&path, error)),
        }
    }
    Ok(())
}

/// The documents `index.json` names, id to path: where this computer last saw each document of
/// the shared workspace (ADR 0036 decisions 1 and 6). A missing, damaged or newer file is an
/// empty cache, rebuilt by the next write.
#[must_use]
pub fn read_index(root: &Path) -> BTreeMap<[u8; 16], Vec<String>> {
    let path = root.join(MARKER_DIR).join(MARKER_INDEX_FILE);
    let Ok(bytes) = fs::read(&path) else {
        return BTreeMap::new();
    };
    let Ok(file) = serde_json::from_slice::<IndexFile>(&bytes) else {
        return BTreeMap::new();
    };
    if file.format_version != INDEX_FORMAT_VERSION {
        return BTreeMap::new();
    }
    file.documents
        .into_iter()
        .filter_map(|(id, entry)| Some((crate::workspace::unhex(&id)?, entry.path)))
        .collect()
}

/// Whether `folder` can take a joined workspace: an existing, empty, writable folder that is no
/// workspace and lies in none (`ux-notes.md` section 8). Hidden entries count as entries.
///
/// # Errors
/// The reason it cannot.
pub fn check_join_folder(folder: &Path) -> Result<(), JoinFolderError> {
    let mut entries = fs::read_dir(folder).map_err(|_| JoinFolderError::NotWritable)?;
    if folder.ancestors().any(|dir| dir.join(MARKER_DIR).exists()) {
        return Err(JoinFolderError::InsideWorkspace);
    }
    if entries.next().is_some() {
        return Err(JoinFolderError::NotEmpty);
    }
    probe_writable(folder).map_err(|_| JoinFolderError::NotWritable)
}

/// Makes the checked, empty `folder` the joined workspace `id`: its marker with that id
/// (criterion 27); nothing else is written.
///
/// # Errors
/// [`WorkspaceError::Exists`] when the folder no longer passes [`check_join_folder`];
/// [`WorkspaceError::NotWritable`], [`WorkspaceError::Io`].
pub fn create_joined_workspace(folder: &Path, id: [u8; 16]) -> Result<(), WorkspaceError> {
    check_join_folder(folder).map_err(|_| WorkspaceError::Exists)?;
    fs::create_dir(folder.join(MARKER_DIR)).map_err(|_| WorkspaceError::NotWritable)?;
    create_new_file(
        &folder.join(MARKER_DIR).join(MARKER_FILE),
        marker_text(id).as_bytes(),
    )?;
    open_workspace(folder, id).map(|_| ())
}

/// Removes `share.json` from every document of the workspace whose record names the
/// workspace's id; every other member is copied raw. A file that is no container is left alone.
fn drop_document_shares(root: &Path) -> Result<(), WorkspaceError> {
    let marker = root.join(MARKER_DIR).join(MARKER_FILE);
    let text = fs::read_to_string(&marker).map_err(|error| io_error(&marker, error))?;
    let id = parse_marker(&text)?;
    for entry in scan_workspace(root)? {
        if entry.kind != ScannedKind::Document {
            continue;
        }
        let members = share_members(root, &entry.path)?;
        let ours = members
            .share_json
            .as_deref()
            .is_some_and(|json| names_workspace(json, &hex(&id)));
        if !ours {
            continue;
        }
        let target = resolve(root, &entry.path)?;
        let bytes = fs::read(&target).map_err(|error| io_error(&target, error))?;
        if let Some(local) = without_member(&bytes, SHARE_MEMBER) {
            write_atomic(&target, &local)
                .map_err(|error| io_error(&target, std::io::Error::other(error.to_string())))?;
        }
    }
    Ok(())
}

/// Whether a document's `share.json` is version 2 (ADR 0036 decision 3) naming the workspace
/// `id` (lowercase hex).
fn names_workspace(share_json: &[u8], id: &str) -> bool {
    let Ok(json) = serde_json::from_slice::<serde_json::Value>(share_json) else {
        return false;
    };
    json["format_version"] == 2 && json["workspace"].as_str() == Some(id)
}

/// `container` without its member `name`, every other member copied raw; `None` when the bytes
/// are no readable zip.
fn without_member(container: &[u8], name: &str) -> Option<Vec<u8>> {
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(container)).ok()?;
    let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    for index in 0..archive.len() {
        let member = archive.by_index_raw(index).ok()?;
        if member.name().ok()? == name {
            continue;
        }
        writer.raw_copy_file(member).ok()?;
    }
    Some(writer.finish().ok()?.into_inner())
}
