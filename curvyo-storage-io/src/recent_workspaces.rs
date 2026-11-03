//! The list of recently opened workspaces, `recent-workspaces.toml` in the app data directory
//! (`specs/0094-workspace` criterion 20, ADR 0036 decision 1): its text, its order and which of
//! its folders still exist. It is never sent anywhere.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// The file's name in the app data directory, beside `font-library.toml`.
pub const RECENT_WORKSPACES_FILE: &str = "recent-workspaces.toml";
/// The `format` this build writes and the newest it reads.
pub const RECENT_WORKSPACES_FORMAT: u32 = 1;
/// The most workspaces the list keeps.
pub const MAX_RECENT_WORKSPACES: usize = 10;

/// Why the text was not read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum RecentWorkspacesError {
    /// The file was written by a newer build; it is never written over.
    #[error("the recent workspaces file was made by a newer Curvyo")]
    Newer,
    /// The text is not a recents file; the next write replaces it **(Proposal)**: it holds no
    /// work, only shortcuts.
    #[error("the recent workspaces file is damaged")]
    Damaged,
}

/// One row of the list as the File tab shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecentWorkspace {
    /// The folder's name, which is the workspace's name.
    pub name: String,
    /// The folder's full path.
    pub path: PathBuf,
    /// Whether the folder exists now; a missing one is shown "Not found".
    pub found: bool,
}

/// The recent workspaces, most recent first, at most [`MAX_RECENT_WORKSPACES`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RecentWorkspaces {
    paths: Vec<PathBuf>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RecentsToml {
    format: u32,
    paths: Vec<String>,
}

#[derive(Deserialize)]
struct RecentsVersion {
    format: u32,
}

impl RecentWorkspaces {
    /// The list in `text`.
    ///
    /// # Errors
    /// [`RecentWorkspacesError::Newer`] or [`RecentWorkspacesError::Damaged`].
    pub fn parse(text: &str) -> Result<Self, RecentWorkspacesError> {
        let version: RecentsVersion =
            toml::from_str(text).map_err(|_| RecentWorkspacesError::Damaged)?;
        if version.format > RECENT_WORKSPACES_FORMAT {
            return Err(RecentWorkspacesError::Newer);
        }
        let file: RecentsToml = toml::from_str(text).map_err(|_| RecentWorkspacesError::Damaged)?;
        let mut list = Self::default();
        for path in file.paths.iter().rev() {
            list.opened(Path::new(path));
        }
        Ok(list)
    }

    /// The text of the file.
    #[must_use]
    pub fn to_text(&self) -> String {
        let file = RecentsToml {
            format: RECENT_WORKSPACES_FORMAT,
            paths: self
                .paths
                .iter()
                .filter_map(|path| path.to_str().map(str::to_string))
                .collect(),
        };
        // invariant: a struct of an integer and strings always serialises.
        toml::to_string(&file).unwrap_or_default()
    }

    /// Puts `path` first, dropping an older entry of it and the oldest past the limit. A path
    /// that is not UTF-8 text is not kept **(Proposal)**: the file is TOML text.
    pub fn opened(&mut self, path: &Path) {
        if path.to_str().is_none() {
            return;
        }
        self.paths.retain(|kept| kept != path);
        self.paths.insert(0, path.to_path_buf());
        self.paths.truncate(MAX_RECENT_WORKSPACES);
    }

    /// Removes the entry at `index`; no folder is touched. An index past the end changes nothing.
    pub fn remove(&mut self, index: usize) {
        if index < self.paths.len() {
            self.paths.remove(index);
        }
    }

    /// The path at `index`, the one a row opens.
    #[must_use]
    pub fn path(&self, index: usize) -> Option<&Path> {
        self.paths.get(index).map(PathBuf::as_path)
    }

    /// The rows, most recent first, each checked for existence now.
    #[must_use]
    pub fn rows(&self) -> Vec<RecentWorkspace> {
        self.paths
            .iter()
            .map(|path| RecentWorkspace {
                name: path.file_name().map_or_else(
                    || path.display().to_string(),
                    |name| name.to_string_lossy().into_owned(),
                ),
                path: path.clone(),
                found: path.is_dir(),
            })
            .collect()
    }
}
