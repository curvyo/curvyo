//! Copies font files into the Curvyo font folder and deletes a family's files from it
//! (`specs/0075-font-library` criteria 21, 22, 25 and 32, `adrs.md` decision 6). A file is
//! accepted exactly when the folder listing would list a face of it; nothing outside the folder
//! is ever written or deleted.

use std::path::{Path, PathBuf};

use crate::filesystem::write_atomic;
use crate::fonts::{FONT_EXTENSIONS, MAX_FONT_FILE_BYTES, face_tables};

/// What an Add did, by the chosen files' names.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AddedFonts {
    /// Copied into the folder (under the name they got there).
    pub added: Vec<String>,
    /// Already in the folder with the same bytes; not copied again.
    pub present: Vec<String>,
    /// Not a font the folder would list, over 64 MiB, unreadable, or not written.
    pub not_fonts: Vec<String>,
}

/// What a Remove did, by file name.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RemovedFonts {
    /// Deleted from the folder.
    pub removed: Vec<String>,
    /// Kept: a collection that also holds faces of another family, or a file not directly in
    /// the folder.
    pub kept: Vec<String>,
}

fn name_of(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn has_font_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| FONT_EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
}

/// Whether `bytes` hold a face the folder listing lists: `fontdb` reads it with a family name
/// and it has outlines and no colour tables.
fn is_listable(bytes: &[u8]) -> bool {
    let mut db = fontdb::Database::new();
    db.load_font_data(bytes.to_vec());
    db.faces().any(|face| {
        !face.families.is_empty()
            && face_tables(&mut std::io::Cursor::new(bytes), face.index).is_some_and(|t| t.outline)
    })
}

/// The bytes of the file at `path`, read through `take` so a file that grows past the 64 MiB
/// limit after a size check is never read whole; `None` for a larger or unreadable file.
fn read_limited(path: &Path) -> Option<Vec<u8>> {
    use std::io::Read;
    let file = std::fs::File::open(path).ok()?;
    if !file.metadata().ok()?.is_file() {
        return None;
    }
    let mut bytes = Vec::new();
    file.take(MAX_FONT_FILE_BYTES + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    (bytes.len() as u64 <= MAX_FONT_FILE_BYTES).then_some(bytes)
}

/// The files directly in `dir`.
fn folder_files(dir: &Path) -> Vec<PathBuf> {
    std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .filter(|path| path.is_file())
                .collect()
        })
        .unwrap_or_default()
}

/// The first free name for `name` among `taken`, compared by Unicode lower case (the browser's
/// `freeFontName` uses the same rule): `name`, else
/// `<stem>-2.<ext>`, `-3`, ...
fn free_name(name: &str, taken: &[String]) -> String {
    let is_taken = |candidate: &str| {
        taken
            .iter()
            .any(|t| t.to_lowercase() == candidate.to_lowercase())
    };
    if !is_taken(name) {
        return name.to_owned();
    }
    let (stem, ext) = name.rsplit_once('.').unwrap_or((name, ""));
    (2_u32..=u32::MAX)
        .map(|n| {
            if ext.is_empty() {
                format!("{stem}-{n}")
            } else {
                format!("{stem}-{n}.{ext}")
            }
        })
        .find(|candidate| !is_taken(candidate))
        .unwrap_or_else(|| name.to_owned())
}

/// Copies the readable fonts of `paths` into `dir` (created when missing). A file whose bytes
/// equal a file in the folder is "present"; a taken name gets `-2`, `-3`, ...; each copy is
/// written to a temporary file and renamed. The originals are only read.
#[must_use]
pub fn add_font_files(dir: &Path, paths: &[PathBuf]) -> AddedFonts {
    let _ = std::fs::create_dir_all(dir);
    let mut report = AddedFonts::default();
    for path in paths {
        let name = name_of(path);
        let bytes = has_font_extension(path)
            .then(|| read_limited(path))
            .flatten()
            .filter(|bytes| is_listable(bytes));
        let Some(bytes) = bytes else {
            report.not_fonts.push(name);
            continue;
        };
        let existing = folder_files(dir);
        let same = existing.iter().any(|other| {
            std::fs::metadata(other).is_ok_and(|m| m.len() == bytes.len() as u64)
                && std::fs::read(other).is_ok_and(|other| other == bytes)
        });
        if same {
            report.present.push(name);
            continue;
        }
        let taken: Vec<String> = existing.iter().map(|p| name_of(p)).collect();
        let target = free_name(&name, &taken);
        if write_atomic(&dir.join(&target), &bytes).is_ok() {
            report.added.push(target);
        } else {
            report.not_fonts.push(name);
        }
    }
    report
}

/// Deletes the files of a family's faces `faces` (file and face index) from `dir`. A file is
/// deleted only when it lies directly in `dir` and every face of it is among `faces`; otherwise
/// it is kept and named in [`RemovedFonts::kept`].
#[must_use]
pub fn remove_font_files(dir: &Path, faces: &[(PathBuf, u32)]) -> RemovedFonts {
    let mut report = RemovedFonts::default();
    let Ok(folder) = dir.canonicalize() else {
        return report;
    };
    let mut files: Vec<&PathBuf> = faces.iter().map(|(path, _)| path).collect();
    files.sort();
    files.dedup();
    for file in files {
        let inside = file
            .canonicalize()
            .ok()
            .and_then(|real| real.parent().map(|parent| parent == folder))
            .unwrap_or(false);
        let mut db = fontdb::Database::new();
        if inside {
            let _ = db.load_font_file(file);
        }
        let all_faces = !db.is_empty()
            && db
                .faces()
                .all(|face| faces.iter().any(|(p, i)| p == file && *i == face.index));
        if inside && all_faces && std::fs::remove_file(file).is_ok() {
            report.removed.push(name_of(file));
        } else {
            report.kept.push(name_of(file));
        }
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    fn included(path: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../frontend/public/fonts")
            .join(path)
    }

    /// Criteria 21, 22 and 32: a font is copied, the same bytes again are present, another font
    /// under a taken name gets `-2`, a non-font and a cut-off font are refused, the original
    /// stays, and no temporary file is left.
    #[test]
    fn ac21_ac32_add_copies_dedupes_renames_and_refuses() {
        let dir = tempfile::tempdir().unwrap();
        let folder = dir.path().join("fonts");
        let source = dir.path().join("in");
        std::fs::create_dir(&source).unwrap();
        let serif = source.join("Font.ttf");
        std::fs::copy(included("source-serif-4/SourceSerif4-Bold.ttf"), &serif).unwrap();
        let other = dir.path().join("other");
        std::fs::create_dir(&other).unwrap();
        let vibes = other.join("font.TTF");
        std::fs::copy(included("great-vibes/GreatVibes-Regular.ttf"), &vibes).unwrap();
        let notes = source.join("notes.ttf");
        std::fs::write(&notes, b"not a font").unwrap();
        let cut = source.join("cut.ttf");
        let bytes = std::fs::read(&serif).unwrap();
        std::fs::write(&cut, &bytes[..bytes.len() / 3]).unwrap();
        let first = add_font_files(&folder, &[serif.clone(), notes, cut]);
        assert_eq!(first.added, ["Font.ttf"]);
        assert_eq!(first.not_fonts, ["notes.ttf", "cut.ttf"]);
        let second = add_font_files(&folder, &[serif.clone(), vibes]);
        assert_eq!(second.present, ["Font.ttf"]);
        assert_eq!(second.added, ["font-2.TTF"]);
        assert!(serif.is_file(), "the original stays");
        let mut names: Vec<String> = folder_files(&folder).iter().map(|p| name_of(p)).collect();
        names.sort();
        assert_eq!(names, ["Font.ttf", "font-2.TTF"]);
    }

    /// Decision 7: names collide by Unicode lower case, as in the browser.
    #[test]
    fn names_collide_by_unicode_lower_case() {
        assert_eq!(
            free_name("ÉLAN.ttf", &["élan.TTF".to_owned()]),
            "ÉLAN-2.ttf"
        );
    }

    /// Criterion 25: a family's file in the folder is deleted; a file outside the folder is
    /// kept; a collection with a face that is not among the family's is kept.
    #[test]
    fn ac25_remove_deletes_only_whole_files_in_the_folder() {
        let dir = tempfile::tempdir().unwrap();
        let folder = dir.path().join("fonts");
        std::fs::create_dir(&folder).unwrap();
        let file = folder.join("Vibes.ttf");
        std::fs::copy(included("great-vibes/GreatVibes-Regular.ttf"), &file).unwrap();
        let outside = dir.path().join("Serif.ttf");
        std::fs::copy(included("source-serif-4/SourceSerif4-Bold.ttf"), &outside).unwrap();
        let report = remove_font_files(&folder, &[(file.clone(), 0), (outside.clone(), 0)]);
        assert_eq!(report.removed, ["Vibes.ttf"]);
        assert_eq!(report.kept, ["Serif.ttf"]);
        assert!(!file.exists() && outside.exists());
        let collection = folder.join("Two.ttc");
        std::fs::write(&collection, two_face_collection()).unwrap();
        let report = remove_font_files(&folder, &[(collection.clone(), 0)]);
        assert_eq!(report.kept, ["Two.ttc"]);
        assert!(collection.exists());
    }

    /// A TrueType collection of two faces, both Great Vibes' bytes (the table offsets of the
    /// second face point into the same tables).
    fn two_face_collection() -> Vec<u8> {
        let face = std::fs::read(included("great-vibes/GreatVibes-Regular.ttf")).unwrap();
        let header_len = 12 + 4 * 2;
        let shift = u32::try_from(header_len).unwrap();
        let mut font = face.clone();
        let tables = usize::from(u16::from_be_bytes([font[4], font[5]]));
        for record in 0..tables {
            let at = 12 + 16 * record + 8;
            let offset = u32::from_be_bytes(font[at..at + 4].try_into().unwrap()) + shift;
            font[at..at + 4].copy_from_slice(&offset.to_be_bytes());
        }
        let mut out = b"ttcf".to_vec();
        out.extend_from_slice(&0x0001_0000_u32.to_be_bytes());
        out.extend_from_slice(&2_u32.to_be_bytes());
        out.extend_from_slice(&shift.to_be_bytes());
        out.extend_from_slice(&shift.to_be_bytes());
        out.extend_from_slice(&font);
        out
    }
}
