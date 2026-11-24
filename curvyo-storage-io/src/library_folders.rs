//! The folders the maker added to the library by hand, `library-folders.toml` in the app data
//! directory (`specs/0137-document-library` criterion 1, ADR 0084 §3): their text and their
//! order. Removing one deletes only the entry; nothing on disk is touched.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// The file's name in the app data directory, beside `recent-workspaces.toml`.
pub const LIBRARY_FOLDERS_FILE: &str = "library-folders.toml";
/// The `format` this build writes and the newest it reads.
pub const LIBRARY_FOLDERS_FORMAT: u32 = 1;

/// Why the text was not read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum LibraryFoldersError {
    /// The file was written by a newer build; it is never written over.
    #[error("the library folders file was made by a newer Curvyo")]
    Newer,
    /// The text is not a library folders file; the next write replaces it.
    #[error("the library folders file is damaged")]
    Damaged,
}

/// Why a folder was not added.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum AddFolderRefusal {
    /// The folder is in the list already.
    #[error("the folder is already in the library")]
    AlreadyIn,
    /// The path is not UTF-8 text, which the file cannot hold.
    #[error("the folder's path cannot be kept")]
    NotText,
}

/// The added folders, in the order they were added (P7).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LibraryFolders {
    paths: Vec<PathBuf>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct FoldersToml {
    format: u32,
    paths: Vec<String>,
}

#[derive(Deserialize)]
struct FoldersVersion {
    format: u32,
}

impl LibraryFolders {
    /// The list in `text`.
    ///
    /// # Errors
    /// [`LibraryFoldersError::Newer`] or [`LibraryFoldersError::Damaged`].
    pub fn parse(text: &str) -> Result<Self, LibraryFoldersError> {
        let version: FoldersVersion =
            toml::from_str(text).map_err(|_| LibraryFoldersError::Damaged)?;
        if version.format > LIBRARY_FOLDERS_FORMAT {
            return Err(LibraryFoldersError::Newer);
        }
        let file: FoldersToml = toml::from_str(text).map_err(|_| LibraryFoldersError::Damaged)?;
        let mut list = Self::default();
        for path in &file.paths {
            let _ = list.add(Path::new(path));
        }
        Ok(list)
    }

    /// The text of the file.
    #[must_use]
    pub fn to_text(&self) -> String {
        let file = FoldersToml {
            format: LIBRARY_FOLDERS_FORMAT,
            paths: self
                .paths
                .iter()
                .filter_map(|path| path.to_str().map(str::to_string))
                .collect(),
        };
        // invariant: a struct of an integer and strings always serialises.
        toml::to_string(&file).unwrap_or_default()
    }

    /// Appends `path`. A folder inside an added folder, or one that holds one, is added as asked
    /// (P12).
    ///
    /// # Errors
    /// [`AddFolderRefusal`]; the list is unchanged then.
    pub fn add(&mut self, path: &Path) -> Result<(), AddFolderRefusal> {
        if path.to_str().is_none() {
            return Err(AddFolderRefusal::NotText);
        }
        if self.paths.iter().any(|kept| kept == path) {
            return Err(AddFolderRefusal::AlreadyIn);
        }
        self.paths.push(path.to_path_buf());
        Ok(())
    }

    /// Removes `path` from the list; no folder is touched. Returns whether it was listed.
    pub fn remove(&mut self, path: &Path) -> bool {
        let before = self.paths.len();
        self.paths.retain(|kept| kept != path);
        self.paths.len() != before
    }

    /// The folders, in the order they were added.
    #[must_use]
    pub fn paths(&self) -> &[PathBuf] {
        &self.paths
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_text_reads_back_in_the_order_added() {
        let mut list = LibraryFolders::default();
        list.add(Path::new("/b/logos")).unwrap();
        list.add(Path::new("/a/jobs")).unwrap();
        let read = LibraryFolders::parse(&list.to_text()).unwrap();
        assert_eq!(
            read.paths(),
            [PathBuf::from("/b/logos"), PathBuf::from("/a/jobs")]
        );
    }

    #[test]
    fn a_folder_is_added_once_and_removed_from_the_list_only() {
        let dir = tempfile::tempdir().unwrap();
        let mut list = LibraryFolders::default();
        list.add(dir.path()).unwrap();
        assert_eq!(list.add(dir.path()), Err(AddFolderRefusal::AlreadyIn));
        list.add(&dir.path().join("inner")).unwrap();
        assert!(list.remove(dir.path()));
        assert!(!list.remove(dir.path()));
        assert!(dir.path().is_dir(), "nothing on disk is touched");
        assert_eq!(list.paths().len(), 1);
    }

    #[test]
    fn a_newer_file_is_refused_and_a_damaged_one_is_reported() {
        assert_eq!(
            LibraryFolders::parse("format = 2\npaths = []\n"),
            Err(LibraryFoldersError::Newer)
        );
        assert_eq!(
            LibraryFolders::parse("not toml ["),
            Err(LibraryFoldersError::Damaged)
        );
        assert_eq!(
            LibraryFolders::parse("format = 1\nfolders = []\n"),
            Err(LibraryFoldersError::Damaged)
        );
    }
}
