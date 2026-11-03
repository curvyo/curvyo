//! Keeps the files of documents deleted from a shared workspace (`specs/0095-workspace`
//! criterion 40, ADR 0037 decision 6) in the marker directory, under `deleted/<id>.curvyo`, so
//! that Restore writes the file again from the last synced state and later workspace snapshots
//! still carry the document. **(Proposal)** The file at its place goes to the OS trash after the
//! copy is kept, never erased.

use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use crate::filesystem::write_atomic;
use crate::workspace::{MARKER_DIR, WorkspaceError, create_document, hex, io_error, trash_entry};

/// The folder of kept deleted documents inside the marker directory.
pub const DELETED_DIR: &str = "deleted";

fn kept_path(root: &Path, document: [u8; 16]) -> PathBuf {
    root.join(MARKER_DIR)
        .join(DELETED_DIR)
        .join(format!("{}.curvyo", hex(&document)))
}

/// Keeps a copy of the document `document` at `path`, then moves the file to the OS trash.
///
/// # Errors
/// [`WorkspaceError::OutsideWorkspace`], [`WorkspaceError::NotFound`], [`WorkspaceError::NoTrash`]
/// (the copy is kept, the file stays), [`WorkspaceError::Io`].
pub fn keep_deleted(
    root: &Path,
    path: &[String],
    document: [u8; 16],
) -> Result<(), WorkspaceError> {
    let bytes = crate::read_document(root, path)?;
    write_deleted(root, document, &bytes)?;
    trash_entry(root, path)
}

/// The kept bytes of the deleted document `document`.
///
/// # Errors
/// [`WorkspaceError::NotFound`] when none is kept, [`WorkspaceError::Io`].
pub fn read_deleted(root: &Path, document: [u8; 16]) -> Result<Vec<u8>, WorkspaceError> {
    let path = kept_path(root, document);
    fs::read(&path).map_err(|source| match source.kind() {
        ErrorKind::NotFound => WorkspaceError::NotFound,
        _ => io_error(&path, source),
    })
}

/// Writes the kept copy of the deleted document `document` (a background write, a join),
/// replacing an older one atomically.
///
/// # Errors
/// [`WorkspaceError::Io`].
pub fn write_deleted(root: &Path, document: [u8; 16], bytes: &[u8]) -> Result<(), WorkspaceError> {
    let path = kept_path(root, document);
    let folder = root.join(MARKER_DIR).join(DELETED_DIR);
    fs::create_dir_all(&folder).map_err(|source| io_error(&folder, source))?;
    write_atomic(&path, bytes)
        .map_err(|error| io_error(&path, std::io::Error::other(error.to_string())))
}

/// Restore: writes the kept copy of `document` at `path` (never replacing an entry) and drops
/// the copy.
///
/// # Errors
/// [`WorkspaceError::NotFound`] when none is kept, [`WorkspaceError::OutsideWorkspace`],
/// [`WorkspaceError::Exists`], [`WorkspaceError::Io`].
pub fn restore_deleted(
    root: &Path,
    document: [u8; 16],
    path: &[String],
) -> Result<(), WorkspaceError> {
    let bytes = read_deleted(root, document)?;
    create_document(root, path, &bytes)?;
    let _ = fs::remove_file(kept_path(root, document));
    Ok(())
}
