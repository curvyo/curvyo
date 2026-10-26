//! Finds the files of the Curvyo font folder and of its subfolders one level deep, with each
//! subfolder's documents (`specs/0083-font-info-and-zip` criteria 4 and 10, `adrs.md`
//! decision 4). Folders whose name starts with `.` and symlinked folders are not read.

use std::path::{Path, PathBuf};

use curvyo_text_core::is_document_name;

/// The name prefix of the folder an archive is built in before its rename (criterion 12).
pub(crate) const TEMP_PREFIX: &str = ".curvyo-add-";

/// One document of a font folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FolderDocument {
    /// The file name.
    pub name: String,
    /// Its size in bytes.
    pub size: u64,
}

/// A subfolder of the Curvyo font folder and its documents, A to Z ignoring case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontFolder {
    /// The folder's name.
    pub name: String,
    /// The documents directly in it.
    pub documents: Vec<FolderDocument>,
}

/// The files the listing reads: each with the subfolder it is in (`None` at the top level).
#[derive(Debug, Default)]
pub(crate) struct FolderFiles {
    pub(crate) files: Vec<(PathBuf, Option<String>)>,
    pub(crate) folders: Vec<FontFolder>,
}

/// Whether `name` is a subfolder the listing reads: not empty, not starting with `.`.
pub(crate) fn is_listed_folder_name(name: &str) -> bool {
    !name.is_empty() && !name.starts_with('.')
}

fn name_of(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// The regular files directly in `dir` (a symlink to a file counts, as before `0083`) and the
/// real subfolders directly in it (a symlink to a folder does not).
fn entries_of(dir: &Path) -> (Vec<PathBuf>, Vec<PathBuf>) {
    let mut files = Vec::new();
    let mut folders = Vec::new();
    for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let path = entry.path();
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        if kind.is_dir() {
            folders.push(path);
        } else if path.is_file() {
            files.push(path);
        }
    }
    files.sort();
    folders.sort();
    (files, folders)
}

/// The files of the Curvyo folder `dir` and of its listed subfolders, and each subfolder's
/// documents.
pub(crate) fn folder_files(dir: &Path) -> FolderFiles {
    let (top, subfolders) = entries_of(dir);
    let mut out = FolderFiles {
        files: top.into_iter().map(|path| (path, None)).collect(),
        folders: Vec::new(),
    };
    for folder in subfolders {
        let name = name_of(&folder);
        if !is_listed_folder_name(&name) {
            continue;
        }
        let (files, _) = entries_of(&folder);
        let mut documents: Vec<FolderDocument> = files
            .iter()
            .filter(|path| is_document_name(&name_of(path)))
            .map(|path| FolderDocument {
                name: name_of(path),
                size: std::fs::metadata(path).map_or(0, |meta| meta.len()),
            })
            .collect();
        documents.sort_by_key(|doc| doc.name.to_lowercase());
        out.files
            .extend(files.into_iter().map(|path| (path, Some(name.clone()))));
        out.folders.push(FontFolder { name, documents });
    }
    out
}

/// Removes the folders an interrupted archive add left in `dir` (named `.curvyo-add-*`; no other
/// folder), at start (criterion 12).
pub fn remove_add_leftovers(dir: &Path) {
    let (_, folders) = entries_of(dir);
    for folder in folders {
        if name_of(&folder).starts_with(TEMP_PREFIX) {
            let _ = std::fs::remove_dir_all(&folder);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Criteria 10 and 12: one level deep, no dot-folders, no symlinked folder; only
    /// `.curvyo-add-*` leftovers are removed.
    #[test]
    fn ac10_ac12_one_level_and_leftovers() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::write(root.join("top.ttf"), b"x").unwrap();
        std::fs::create_dir_all(root.join("Lobster/deeper")).unwrap();
        std::fs::write(root.join("Lobster/Lobster.otf"), b"x").unwrap();
        std::fs::write(root.join("Lobster/OFL.txt"), b"licence").unwrap();
        std::fs::write(root.join("Lobster/README"), b"r").unwrap();
        std::fs::write(root.join("Lobster/deeper/Deep.ttf"), b"x").unwrap();
        std::fs::create_dir(root.join(".hidden")).unwrap();
        std::fs::write(root.join(".hidden/Hidden.ttf"), b"x").unwrap();
        std::fs::create_dir(root.join(".curvyo-add-7")).unwrap();
        std::fs::write(root.join(".curvyo-add-7/Half.ttf"), b"x").unwrap();
        #[cfg(unix)]
        {
            let outside = tempfile::tempdir().unwrap();
            std::fs::write(outside.path().join("Out.ttf"), b"x").unwrap();
            std::os::unix::fs::symlink(outside.path(), root.join("Linked")).unwrap();
            let listed = folder_files(root);
            assert!(listed.files.iter().all(|(p, _)| !p.ends_with("Out.ttf")));
        }
        let listed = folder_files(root);
        let names: Vec<(String, Option<String>)> = listed
            .files
            .iter()
            .map(|(p, f)| (name_of(p), f.clone()))
            .collect();
        assert_eq!(
            names,
            [
                ("top.ttf".to_owned(), None),
                ("Lobster.otf".to_owned(), Some("Lobster".to_owned())),
                ("OFL.txt".to_owned(), Some("Lobster".to_owned())),
                ("README".to_owned(), Some("Lobster".to_owned())),
            ]
        );
        let docs: Vec<&str> = listed.folders[0]
            .documents
            .iter()
            .map(|d| d.name.as_str())
            .collect();
        assert_eq!(docs, ["OFL.txt", "README"]);
        remove_add_leftovers(root);
        assert!(!root.join(".curvyo-add-7").exists());
        assert!(root.join(".hidden/Hidden.ttf").exists());
    }
}
