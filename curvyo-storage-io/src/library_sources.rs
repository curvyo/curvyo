//! Reads a library source without opening its documents (`specs/0137-document-library` criteria
//! 1, 2, 10 and 12, ADR 0084 §2, §3): the `.curvyo` files of a folder, recursively, and of one
//! file only the members the library shows (`library.json`, the names and bytes of
//! `pages/<id>.png`, the manifest's format). Files are opened read-only and nothing is written.

use std::fs::{self, File};
use std::io::{ErrorKind, Read};
use std::path::Path;
use std::time::SystemTime;

use crate::workspace::{WorkspaceError, io_error, resolve};

/// The most documents one source lists **(Proposal)**.
pub const MAX_SOURCE_DOCUMENTS: usize = 10_000;
pub use curvyo_document_core::LibraryMembers;

/// Why a whole source cannot be read (UX notes section 10).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SourceError {
    /// The folder is not there (moved, or its drive is not connected).
    #[error("folder not found")]
    NotFound,
    /// The folder may not be read.
    #[error("no access")]
    NoAccess,
    /// Another read error.
    #[error("the folder cannot be read")]
    Io,
}

/// One document file of a source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScannedDocument {
    /// The path inside the source, one entry per folder and the file name last.
    pub path: Vec<String>,
    /// The file's size in bytes.
    pub size: u64,
    /// The file's modification time, when the system gives one.
    pub modified: Option<SystemTime>,
}

/// Every `.curvyo` file under `root`, at most [`MAX_SOURCE_DOCUMENTS`], hidden entries and links
/// to folders skipped, sorted by path. A subfolder that cannot be read is left out.
///
/// # Errors
/// [`SourceError`] when `root` itself cannot be read.
pub fn scan_library_source(root: &Path) -> Result<Vec<ScannedDocument>, SourceError> {
    match fs::metadata(root) {
        Ok(meta) if meta.is_dir() => {}
        Ok(_) => return Err(SourceError::NotFound),
        Err(error) => return Err(source_error(&error)),
    }
    fs::read_dir(root).map_err(|error| source_error(&error))?;
    let mut found = Vec::new();
    walk(root, &mut Vec::new(), &mut found);
    found.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(found)
}

fn source_error(error: &std::io::Error) -> SourceError {
    match error.kind() {
        ErrorKind::NotFound => SourceError::NotFound,
        ErrorKind::PermissionDenied => SourceError::NoAccess,
        _ => SourceError::Io,
    }
}

fn walk(folder: &Path, at: &mut Vec<String>, found: &mut Vec<ScannedDocument>) {
    let Ok(entries) = fs::read_dir(folder) else {
        return;
    };
    for entry in entries.flatten() {
        if found.len() >= MAX_SOURCE_DOCUMENTS {
            return;
        }
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        if name.starts_with('.') {
            continue;
        }
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        let path = entry.path();
        if kind.is_dir() {
            at.push(name);
            walk(&path, at, found);
            at.pop();
            continue;
        }
        // A link is followed to a file and never to a folder (ADR 0084 §3).
        let Ok(meta) = fs::metadata(&path) else {
            continue;
        };
        if !meta.is_file() || !is_document_name(&name) {
            continue;
        }
        let mut file_path = at.clone();
        file_path.push(name);
        found.push(ScannedDocument {
            path: file_path,
            size: meta.len(),
            modified: meta.modified().ok(),
        });
    }
}

fn is_document_name(name: &str) -> bool {
    Path::new(name)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("curvyo"))
}

fn open(root: &Path, path: &[String]) -> Result<File, WorkspaceError> {
    let target = resolve(root, path)?;
    File::open(&target).map_err(|source| match source.kind() {
        ErrorKind::NotFound => WorkspaceError::NotFound,
        _ => io_error(&target, source),
    })
}

/// What the library shows of the document at `path` under `root`: the manifest's format, the
/// page list and which pages have a picture. The file is opened read-only and only the zip
/// directory and these members are read (`curvyo_document_core::read_library_members`).
///
/// # Errors
/// [`WorkspaceError::OutsideWorkspace`], [`WorkspaceError::NotFound`], [`WorkspaceError::Io`].
pub fn read_library_members(
    root: &Path,
    path: &[String],
) -> Result<LibraryMembers, WorkspaceError> {
    Ok(curvyo_document_core::read_library_members(open(
        root, path,
    )?))
}

/// The PNG bytes of the picture of page `page_id` of the document at `path`; `None` when it has
/// none, it is too large, or the file is no readable container.
///
/// # Errors
/// [`WorkspaceError::OutsideWorkspace`], [`WorkspaceError::NotFound`], [`WorkspaceError::Io`].
pub fn read_page_picture(
    root: &Path,
    path: &[String],
    page_id: &str,
) -> Result<Option<Vec<u8>>, WorkspaceError> {
    Ok(curvyo_document_core::read_page_picture(
        open(root, path)?,
        page_id,
    ))
}

/// The bytes of the document at `path` under the source `root`, for the read-only preview and
/// the import; the path is resolved by the workspace rule, never outside the root.
///
/// # Errors
/// [`WorkspaceError::OutsideWorkspace`], [`WorkspaceError::NotFound`], [`WorkspaceError::Io`].
pub fn library_read(root: &Path, path: &[String]) -> Result<Vec<u8>, WorkspaceError> {
    let mut file = open(root, path)?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)
        .map_err(|source| io_error(&root.join(path.join("/")), source))?;
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Write as _};

    fn container(members: &[(&str, &[u8])]) -> Vec<u8> {
        let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        for (name, bytes) in members {
            writer
                .start_file(*name, zip::write::SimpleFileOptions::default())
                .unwrap();
            writer.write_all(bytes).unwrap();
        }
        writer.finish().unwrap().into_inner()
    }

    fn path(parts: &[&str]) -> Vec<String> {
        parts.iter().map(|p| (*p).to_owned()).collect()
    }

    #[test]
    fn a_folder_is_read_recursively_skipping_hidden_entries() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("Sides/Deep")).unwrap();
        fs::create_dir_all(dir.path().join(".hidden")).unwrap();
        for file in [
            "Lid.curvyo",
            "Sides/Front.CURVYO",
            "Sides/Deep/Back.curvyo",
            "notes.txt",
            ".secret.curvyo",
            ".hidden/In.curvyo",
        ] {
            fs::write(dir.path().join(file), b"x").unwrap();
        }
        let found = scan_library_source(dir.path()).unwrap();
        let paths: Vec<_> = found.iter().map(|d| d.path.join("/")).collect();
        assert_eq!(
            paths,
            ["Lid.curvyo", "Sides/Deep/Back.curvyo", "Sides/Front.CURVYO"]
        );
        assert_eq!(found[0].size, 1);
        assert!(found[0].modified.is_some());
    }

    #[cfg(unix)]
    #[test]
    fn a_link_to_a_folder_is_not_followed() {
        let dir = tempfile::tempdir().unwrap();
        let other = tempfile::tempdir().unwrap();
        fs::write(other.path().join("Far.curvyo"), b"x").unwrap();
        std::os::unix::fs::symlink(other.path(), dir.path().join("link")).unwrap();
        std::os::unix::fs::symlink(
            other.path().join("Far.curvyo"),
            dir.path().join("Near.curvyo"),
        )
        .unwrap();
        let found = scan_library_source(dir.path()).unwrap();
        let paths: Vec<_> = found.iter().map(|d| d.path.join("/")).collect();
        assert_eq!(paths, ["Near.curvyo"]);
    }

    #[test]
    fn a_missing_source_says_why() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(
            scan_library_source(&dir.path().join("gone")),
            Err(SourceError::NotFound)
        );
    }

    #[test]
    fn the_members_are_read_without_writing() {
        let dir = tempfile::tempdir().unwrap();
        let bytes = container(&[
            ("manifest.json", br#"{"format_version": 57}"#),
            ("library.json", br#"{"format":1,"pages":[]}"#),
            ("pages/first.png", b"\x89PNG\r\n\x1a\nfirst"),
            ("pages/7-3.png", b"\x89PNG\r\n\x1a\nsecond"),
        ]);
        let file = dir.path().join("Lid.curvyo");
        fs::write(&file, &bytes).unwrap();
        let before = fs::metadata(&file).unwrap().modified().unwrap();
        let read = read_library_members(dir.path(), &path(&["Lid.curvyo"])).unwrap();
        let LibraryMembers::Readable {
            format_version,
            summary,
            mut pictures,
        } = read
        else {
            panic!("readable")
        };
        pictures.sort();
        assert_eq!(format_version, 57);
        assert_eq!(summary.as_deref(), Some(&br#"{"format":1,"pages":[]}"#[..]));
        assert_eq!(pictures, ["7-3", "first"]);
        assert_eq!(
            read_page_picture(dir.path(), &path(&["Lid.curvyo"]), "7-3").unwrap(),
            Some(b"\x89PNG\r\n\x1a\nsecond".to_vec())
        );
        assert_eq!(
            read_page_picture(dir.path(), &path(&["Lid.curvyo"]), "none").unwrap(),
            None
        );
        assert_eq!(
            library_read(dir.path(), &path(&["Lid.curvyo"])).unwrap(),
            bytes
        );
        assert_eq!(fs::read(&file).unwrap(), bytes);
        assert_eq!(fs::metadata(&file).unwrap().modified().unwrap(), before);
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1, "no sidecar");
    }

    #[test]
    fn an_unreadable_document_says_why_and_nothing_outside_is_read() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("text.curvyo"), b"plain text").unwrap();
        fs::write(dir.path().join("broken.curvyo"), b"PK\x03\x04broken").unwrap();
        let no_manifest = container(&[("other.txt", b"x")]);
        fs::write(dir.path().join("empty.curvyo"), no_manifest).unwrap();
        let read = |name: &str| read_library_members(dir.path(), &path(&[name])).unwrap();
        assert_eq!(read("text.curvyo"), LibraryMembers::NotCurvyo);
        assert_eq!(read("broken.curvyo"), LibraryMembers::Damaged);
        assert_eq!(read("empty.curvyo"), LibraryMembers::NotCurvyo);
        assert!(matches!(
            library_read(dir.path(), &path(&["..", "x.curvyo"])),
            Err(WorkspaceError::OutsideWorkspace)
        ));
    }
}
