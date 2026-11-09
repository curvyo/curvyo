//! Make a workspace for this file (`specs/0105-sharing-concept` criterion 8, ADR 0044 decision
//! 6): a fresh folder next to a file, named after it, with the file moved into it.

use std::fs;
use std::path::{Path, PathBuf};

use crate::workspace::{WorkspaceError, io_error};

/// The folder [`make_workspace_around`] would make for `file`: `<parent>/<name>/` with the
/// file's name without `.curvyo`, " 2", " 3" and so on while that name is taken. It is never the
/// file's parent.
///
/// # Errors
/// [`WorkspaceError::NotFound`] for a path without a parent or a name.
pub fn workspace_folder_for(file: &Path) -> Result<PathBuf, WorkspaceError> {
    let parent = file.parent().ok_or(WorkspaceError::NotFound)?;
    let stem = file
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .filter(|s| !s.is_empty())
        .ok_or(WorkspaceError::NotFound)?;
    let mut number = 1_u32;
    loop {
        let name = if number == 1 {
            stem.clone()
        } else {
            format!("{stem} {number}")
        };
        let folder = parent.join(name);
        if fs::symlink_metadata(&folder).is_err() {
            return Ok(folder);
        }
        number += 1;
    }
}

/// Creates the folder of [`workspace_folder_for`] and renames `file` into it (a sibling folder is
/// on the same file system). A failed rename removes the empty folder again and leaves the file
/// where it was. Returns the folder.
///
/// # Errors
/// [`WorkspaceError::NotFound`] for a missing file; [`WorkspaceError::Io`] when the folder cannot
/// be made or the file cannot be moved.
pub fn make_workspace_around(file: &Path) -> Result<PathBuf, WorkspaceError> {
    if !fs::symlink_metadata(file).is_ok_and(|m| m.is_file()) {
        return Err(WorkspaceError::NotFound);
    }
    let folder = workspace_folder_for(file)?;
    fs::create_dir(&folder).map_err(|error| io_error(&folder, error))?;
    let name = file.file_name().ok_or(WorkspaceError::NotFound)?;
    let target = folder.join(name);
    if let Err(error) = fs::rename(file, &target) {
        let _ = fs::remove_dir(&folder);
        return Err(io_error(&target, error));
    }
    Ok(folder)
}
