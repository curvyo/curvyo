//! The disk side of a local workspace (`specs/0094-workspace` criteria 1, 2, 4, 5, 7, 13 to 18,
//! 23, 24; ADR 0036 decisions 1, 2 and 6): the marker directory, the folder scan and the file
//! operations, each confined to the workspace folder.
//!
//! What to do is planned in `curvyo-document-core` (`workspace_plan`); this module does it.
//! Paths from callers are relative, one component per entry; a component that could leave the
//! folder or reach the marker directory is refused before anything is touched.

use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Write as _};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use serde::{Deserialize, Serialize};

use crate::filesystem::{unique_suffix, write_atomic};

/// The marker directory inside a workspace folder.
pub const MARKER_DIR: &str = ".curvyo-workspace";
/// The marker file inside [`MARKER_DIR`].
pub const MARKER_FILE: &str = "workspace.toml";
/// The `format_version` of `workspace.toml` this build writes and the newest it reads.
pub const WORKSPACE_FORMAT_VERSION: u32 = 1;
/// The extension of the files the scan lists, without the dot (any case).
const DOCUMENT_EXTENSION: &str = "curvyo";

/// Why a workspace operation did not happen; nothing was changed.
#[derive(Debug, thiserror::Error)]
pub enum WorkspaceError {
    /// The path is not a folder.
    #[error("this is not a folder")]
    NotAFolder,
    /// The folder cannot be written (criterion 4).
    #[error("cannot write to this folder")]
    NotWritable,
    /// `workspace.toml` has a higher `format_version` than this build reads (criterion 23).
    #[error("this workspace was made by a newer Curvyo (format {found})")]
    TooNew {
        /// The version found in the file.
        found: u32,
    },
    /// `workspace.toml` cannot be read as a marker.
    #[error("the workspace marker is damaged")]
    Damaged,
    /// A relative path is empty, leaves the folder or names the marker directory.
    #[error("the path is not inside the workspace")]
    OutsideWorkspace,
    /// The target of a creation or a move exists.
    #[error("an entry with this name exists")]
    Exists,
    /// The entry to move or trash does not exist.
    #[error("the entry does not exist")]
    NotFound,
    /// The OS trash refused the entry; nothing was deleted (criterion 17).
    #[error("the system has no trash for this item: {0}")]
    NoTrash(String),
    /// Any other I/O failure.
    #[error("failed to access {path}: {source}")]
    Io {
        /// The path that failed.
        path: String,
        /// The underlying error.
        #[source]
        source: std::io::Error,
    },
}

/// What a scanned entry is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScannedKind {
    /// A folder.
    Folder,
    /// A `.curvyo` file.
    Document,
}

/// One entry of the scan: no file is opened, only its directory entry is read (criterion 5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScannedEntry {
    /// The path inside the workspace folder, one component per entry.
    pub path: Vec<String>,
    /// Folder or document.
    pub kind: ScannedKind,
    /// The file's size in bytes, 0 for a folder.
    pub size: u64,
    /// The last change, in seconds since the Unix epoch, 0 when the OS gives none.
    pub modified: u64,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct MarkerToml {
    format_version: u32,
    id: String,
}

#[derive(Deserialize)]
struct MarkerVersion {
    format_version: u32,
}

/// The text of `workspace.toml` for workspace `id`.
#[must_use]
pub fn marker_text(id: [u8; 16]) -> String {
    let marker = MarkerToml {
        format_version: WORKSPACE_FORMAT_VERSION,
        id: hex(&id),
    };
    // invariant: a struct of an integer and a string always serialises.
    toml::to_string(&marker).unwrap_or_default()
}

/// The workspace id in the text of `workspace.toml`.
///
/// # Errors
/// [`WorkspaceError::TooNew`] for a higher `format_version`, [`WorkspaceError::Damaged`] for
/// anything else that is not a version 1 marker.
pub fn parse_marker(text: &str) -> Result<[u8; 16], WorkspaceError> {
    let version: MarkerVersion = toml::from_str(text).map_err(|_| WorkspaceError::Damaged)?;
    if version.format_version > WORKSPACE_FORMAT_VERSION {
        return Err(WorkspaceError::TooNew {
            found: version.format_version,
        });
    }
    let marker: MarkerToml = toml::from_str(text).map_err(|_| WorkspaceError::Damaged)?;
    if marker.format_version != WORKSPACE_FORMAT_VERSION {
        return Err(WorkspaceError::Damaged);
    }
    unhex(&marker.id).ok_or(WorkspaceError::Damaged)
}

/// A random workspace id from the OS random source, for [`open_workspace`] (criteria 1, 24).
///
/// # Errors
/// [`WorkspaceError::Io`] when the OS gives no random bytes.
pub fn fresh_workspace_id() -> Result<[u8; 16], WorkspaceError> {
    let mut id = [0_u8; 16];
    getrandom::fill(&mut id).map_err(|error| WorkspaceError::Io {
        path: String::new(),
        source: std::io::Error::other(error.to_string()),
    })?;
    Ok(id)
}

/// Opens `root` as a workspace and returns its id (criteria 1, 2, 4, 23, 24).
///
/// An existing marker is read and kept; a higher version or a damaged marker is refused before
/// anything is written. A folder that cannot be written is refused. A missing marker is created
/// with `fresh_id`, which the caller draws from the OS random source. No other file is touched.
///
/// # Errors
/// [`WorkspaceError::NotAFolder`], [`WorkspaceError::TooNew`], [`WorkspaceError::Damaged`],
/// [`WorkspaceError::NotWritable`], or [`WorkspaceError::Io`] when the marker cannot be read.
pub fn open_workspace(root: &Path, fresh_id: [u8; 16]) -> Result<[u8; 16], WorkspaceError> {
    if !fs::metadata(root).is_ok_and(|meta| meta.is_dir()) {
        return Err(WorkspaceError::NotAFolder);
    }
    let marker_path = root.join(MARKER_DIR).join(MARKER_FILE);
    let existing = match fs::read(&marker_path) {
        Ok(bytes) => {
            let text = String::from_utf8(bytes).map_err(|_| WorkspaceError::Damaged)?;
            Some(parse_marker(&text)?)
        }
        Err(error) if error.kind() == ErrorKind::NotFound => None,
        Err(source) => return Err(io_error(&marker_path, source)),
    };
    probe_writable(root)?;
    if let Some(id) = existing {
        return Ok(id);
    }
    fs::create_dir_all(root.join(MARKER_DIR)).map_err(|_| WorkspaceError::NotWritable)?;
    match create_new_file(&marker_path, marker_text(fresh_id).as_bytes()) {
        Ok(()) => Ok(fresh_id),
        // Another instance created it a moment ago: its id wins.
        Err(WorkspaceError::Exists) => open_workspace(root, fresh_id),
        Err(_) => Err(WorkspaceError::NotWritable),
    }
}

/// Every `.curvyo` file and folder below `root`, sorted by path, without opening a file
/// (criteria 1, 5, 7). The marker directory, hidden entries (a leading dot), symbolic links and
/// names that are not UTF-8 are left out, and so is a subfolder that cannot be read
/// **(Proposal)**.
///
/// # Errors
/// [`WorkspaceError::Io`] when `root` itself cannot be read.
pub fn scan_workspace(root: &Path) -> Result<Vec<ScannedEntry>, WorkspaceError> {
    let mut found = Vec::new();
    let mut folders = vec![Vec::<String>::new()];
    while let Some(folder) = folders.pop() {
        let dir = join(root, &folder);
        let entries = match fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(source) if folder.is_empty() => return Err(io_error(&dir, source)),
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let Ok(name) = entry.file_name().into_string() else {
                continue;
            };
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if name.starts_with('.') || file_type.is_symlink() {
                continue;
            }
            let kind = if file_type.is_dir() {
                ScannedKind::Folder
            } else if file_type.is_file() && is_document_name(&name) {
                ScannedKind::Document
            } else {
                continue;
            };
            let meta = entry.metadata().ok();
            let mut path = folder.clone();
            path.push(name);
            if kind == ScannedKind::Folder {
                folders.push(path.clone());
            }
            found.push(ScannedEntry {
                path,
                kind,
                size: match kind {
                    ScannedKind::Folder => 0,
                    ScannedKind::Document => meta.as_ref().map_or(0, fs::Metadata::len),
                },
                modified: meta
                    .and_then(|meta| meta.modified().ok())
                    .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                    .map_or(0, |since| since.as_secs()),
            });
        }
    }
    found.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(found)
}

/// Writes a new document at `path` with `bytes`; never replaces an entry (criteria 13, 19). The
/// name is reserved first, then the bytes arrive by an atomic rename, so no half-written file
/// ever has the name.
///
/// # Errors
/// [`WorkspaceError::OutsideWorkspace`], [`WorkspaceError::Exists`], [`WorkspaceError::Io`].
pub fn create_document(root: &Path, path: &[String], bytes: &[u8]) -> Result<(), WorkspaceError> {
    let target = resolve(root, path)?;
    create_new_file(&target, &[])?;
    write_atomic(&target, bytes).map_err(|error| {
        let _ = fs::remove_file(&target);
        io_error(&target, std::io::Error::other(error.to_string()))
    })
}

/// Creates the empty folder `path` (criterion 18); its parent must exist.
///
/// # Errors
/// [`WorkspaceError::OutsideWorkspace`], [`WorkspaceError::Exists`], [`WorkspaceError::Io`].
pub fn create_folder(root: &Path, path: &[String]) -> Result<(), WorkspaceError> {
    let target = resolve(root, path)?;
    fs::create_dir(&target).map_err(|source| match source.kind() {
        ErrorKind::AlreadyExists => WorkspaceError::Exists,
        _ => io_error(&target, source),
    })
}

/// Renames or moves the entry `from` to `to` (criteria 14, 16); never replaces an entry. A change
/// of case alone is allowed on a file system that ignores case.
///
/// # Errors
/// [`WorkspaceError::OutsideWorkspace`], [`WorkspaceError::NotFound`], [`WorkspaceError::Exists`],
/// [`WorkspaceError::Io`].
pub fn move_entry(root: &Path, from: &[String], to: &[String]) -> Result<(), WorkspaceError> {
    let source = resolve(root, from)?;
    let target = resolve(root, to)?;
    if fs::symlink_metadata(&source).is_err() {
        return Err(WorkspaceError::NotFound);
    }
    let case_only = from.len() == to.len()
        && from
            .iter()
            .zip(to)
            .all(|(a, b)| a.to_lowercase() == b.to_lowercase());
    if !case_only && fs::symlink_metadata(&target).is_ok() {
        return Err(WorkspaceError::Exists);
    }
    fs::rename(&source, &target).map_err(|error| io_error(&target, error))
}

/// Moves the entry `path` to the OS trash, never erasing it (criterion 17).
///
/// # Errors
/// [`WorkspaceError::OutsideWorkspace`], [`WorkspaceError::NotFound`], or
/// [`WorkspaceError::NoTrash`] with the system's reason when it has no trash for the entry.
pub fn trash_entry(root: &Path, path: &[String]) -> Result<(), WorkspaceError> {
    let target = resolve(root, path)?;
    if fs::symlink_metadata(&target).is_err() {
        return Err(WorkspaceError::NotFound);
    }
    trash::delete(&target).map_err(|error| WorkspaceError::NoTrash(error.to_string()))
}

/// The absolute path of the relative `path`, refused when a component is empty, `.` or `..`,
/// holds a separator or NUL, or starts with a dot (which also keeps the marker directory out).
pub(crate) fn resolve(root: &Path, path: &[String]) -> Result<PathBuf, WorkspaceError> {
    let valid = !path.is_empty()
        && path.iter().all(|part| {
            !part.is_empty() && !part.starts_with('.') && !part.contains(['/', '\\', '\0'])
        });
    if !valid {
        return Err(WorkspaceError::OutsideWorkspace);
    }
    Ok(join(root, path))
}

fn join(root: &Path, path: &[String]) -> PathBuf {
    path.iter()
        .fold(root.to_path_buf(), |dir, part| dir.join(part))
}

/// Creates `path` holding `bytes`, failing with [`WorkspaceError::Exists`] when it exists.
pub(crate) fn create_new_file(path: &Path, bytes: &[u8]) -> Result<(), WorkspaceError> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|source| match source.kind() {
            ErrorKind::AlreadyExists => WorkspaceError::Exists,
            _ => io_error(path, source),
        })?;
    file.write_all(bytes)
        .map_err(|source| io_error(path, source))
}

/// Creates and removes a probe file in `root` (criterion 4).
pub(crate) fn probe_writable(root: &Path) -> Result<(), WorkspaceError> {
    let probe = root.join(format!(".curvyo-probe-{}", unique_suffix()));
    create_new_file(&probe, &[]).map_err(|_| WorkspaceError::NotWritable)?;
    fs::remove_file(&probe).map_err(|_| WorkspaceError::NotWritable)
}

fn is_document_name(name: &str) -> bool {
    Path::new(name)
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case(DOCUMENT_EXTENSION))
        && name.len() > DOCUMENT_EXTENSION.len() + 1
}

pub(crate) fn io_error(path: &Path, source: std::io::Error) -> WorkspaceError {
    WorkspaceError::Io {
        path: path.display().to_string(),
        source,
    }
}

pub(crate) fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut text, byte| {
        // invariant: writing into a String cannot fail.
        let _ = write!(text, "{byte:02x}");
        text
    })
}

pub(crate) fn unhex(text: &str) -> Option<[u8; 16]> {
    if text.len() != 32 || !text.bytes().all(|c| matches!(c, b'0'..=b'9' | b'a'..=b'f')) {
        return None;
    }
    let mut id = [0_u8; 16];
    for (index, byte) in id.iter_mut().enumerate() {
        *byte = u8::from_str_radix(text.get(index * 2..index * 2 + 2)?, 16).ok()?;
    }
    Some(id)
}
