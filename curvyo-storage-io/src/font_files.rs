//! Copies font files into the Curvyo font folder and deletes a family's files from it
//! (`specs/0075-font-library` criteria 21, 22, 25 and 32, `adrs.md` decision 6). A file is
//! accepted exactly when the folder listing would list a face of it; nothing outside the folder
//! and its listed subfolders is ever written or deleted (`specs/0083-font-info-and-zip`
//! criteria 11 and 14, `adrs.md` decision 8).

use std::path::{Path, PathBuf};

use crate::filesystem::write_atomic;
use crate::font_subfolders::{folder_files, is_listed_folder_name};
use crate::fonts::{MAX_FONT_FILE_BYTES, face_tables, has_font_extension, is_svg};
use curvyo_text_core::is_document_name;

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
    /// the folder or one of its listed subfolders.
    pub kept: Vec<String>,
    /// The documents deleted with a font folder that lost its last font (`0083` criterion 14).
    pub documents: usize,
}

fn name_of(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// Whether `bytes` hold a face the folder listing lists: `fontdb` reads it with a family name
/// and it has outlines and no colour tables, or it is an SVG font with a drawn glyph
/// (`specs/0076-single-line-fonts` criterion 3).
pub(crate) fn is_listable(bytes: &[u8]) -> bool {
    if !crate::svg_fonts::svg_font_faces(bytes).is_empty() {
        return true;
    }
    let mut db = fontdb::Database::new();
    db.load_font_data(bytes.to_vec());
    db.faces().any(|face| {
        !face.families.is_empty()
            && face_tables(&mut std::io::Cursor::new(bytes), face.index).is_some_and(|t| t.outline)
    })
}

/// The bytes of the file at `path`, read through `take` so a file that grows past the 64 MiB
/// limit after a size check is never read whole; `None` for a larger or unreadable file.
pub(crate) fn read_limited(path: &Path) -> Option<Vec<u8>> {
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
fn top_level_files(dir: &Path) -> Vec<PathBuf> {
    folder_files(dir)
        .files
        .into_iter()
        .filter(|(_, folder)| folder.is_none())
        .map(|(path, _)| path)
        .collect()
}

/// Whether a file of `existing` holds exactly `bytes`.
pub(crate) fn same_bytes_in(existing: &[PathBuf], bytes: &[u8]) -> bool {
    existing.iter().any(|other| {
        std::fs::metadata(other).is_ok_and(|m| m.len() == bytes.len() as u64)
            && std::fs::read(other).is_ok_and(|other| other == bytes)
    })
}

/// Every font file of the Curvyo folder and its listed subfolders (de-duplication, `0083`
/// criterion 11).
pub(crate) fn every_font_file(dir: &Path) -> Vec<PathBuf> {
    folder_files(dir)
        .files
        .into_iter()
        .map(|(path, _)| path)
        .filter(|path| has_font_extension(path))
        .collect()
}

/// The first free name for `name` among `taken`, compared by Unicode lower case (the browser's
/// `freeFontName` uses the same rule): `name`, else
/// `<stem>-2.<ext>`, `-3`, ...
pub(crate) fn free_name(name: &str, taken: &[String]) -> String {
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
        if same_bytes_in(&every_font_file(dir), &bytes) {
            report.present.push(name);
            continue;
        }
        let taken: Vec<String> = top_level_files(dir).iter().map(|p| name_of(p)).collect();
        let target = free_name(&name, &taken);
        if write_atomic(&dir.join(&target), &bytes).is_ok() {
            report.added.push(target);
        } else {
            report.not_fonts.push(name);
        }
    }
    report
}

/// Where a file to remove lies in the Curvyo folder.
enum Place {
    /// Directly in it.
    Top,
    /// In this listed subfolder of it.
    Subfolder(PathBuf),
}

/// Where `file` lies in the canonical Curvyo folder `folder`; `None` anywhere else.
fn place_of(folder: &Path, file: &Path) -> Option<Place> {
    let real = file.canonicalize().ok()?;
    let parent = real.parent()?;
    if parent == folder {
        return Some(Place::Top);
    }
    let listed = parent
        .file_name()
        .is_some_and(|name| is_listed_folder_name(&name.to_string_lossy()));
    (listed && parent.parent() == Some(folder)).then(|| Place::Subfolder(parent.to_path_buf()))
}

/// The face indices of the font file `file`.
fn face_indices(file: &Path) -> Vec<u32> {
    if is_svg(file) {
        let bytes = std::fs::read(file).unwrap_or_default();
        return crate::svg_fonts::svg_font_faces(&bytes)
            .iter()
            .map(|face| face.index)
            .collect();
    }
    let mut db = fontdb::Database::new();
    let _ = db.load_font_file(file);
    db.faces().map(|face| face.index).collect()
}

/// After a removal from the font folder `subfolder`: when it holds no font the listing would
/// list, its documents are deleted and the folder removed if that leaves it empty (never a
/// maker's other files). Returns the documents deleted.
fn clear_emptied_folder(subfolder: &Path) -> usize {
    let files: Vec<PathBuf> = std::fs::read_dir(subfolder)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .collect();
    let has_font = files.iter().any(|path| {
        has_font_extension(path) && read_limited(path).is_some_and(|bytes| is_listable(&bytes))
    });
    if has_font {
        return 0;
    }
    let mut deleted = 0;
    for path in &files {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        if is_document_name(&name) && std::fs::remove_file(path).is_ok() {
            deleted += 1;
        }
    }
    let _ = std::fs::remove_dir(subfolder);
    deleted
}

/// Deletes the files of a family's faces `faces` (file and face index) from `dir`. A file is
/// deleted only when it lies directly in `dir` or in a listed subfolder of it and every face of
/// it is among `faces`; otherwise it is kept and named in [`RemovedFonts::kept`]. A subfolder
/// left with no font loses its documents and, when then empty, itself (`0083` criterion 14).
#[must_use]
pub fn remove_font_files(dir: &Path, faces: &[(PathBuf, u32)]) -> RemovedFonts {
    let mut report = RemovedFonts::default();
    let Ok(folder) = dir.canonicalize() else {
        return report;
    };
    let mut files: Vec<&PathBuf> = faces.iter().map(|(path, _)| path).collect();
    files.sort();
    files.dedup();
    let mut touched: Vec<PathBuf> = Vec::new();
    for file in files {
        let place = place_of(&folder, file);
        let indices = if place.is_some() {
            face_indices(file)
        } else {
            Vec::new()
        };
        let all_faces = !indices.is_empty()
            && indices
                .iter()
                .all(|index| faces.iter().any(|(p, i)| p == file && i == index));
        if all_faces && std::fs::remove_file(file).is_ok() {
            report.removed.push(name_of(file));
            if let Some(Place::Subfolder(subfolder)) = place
                && !touched.contains(&subfolder)
            {
                touched.push(subfolder);
            }
        } else {
            report.kept.push(name_of(file));
        }
    }
    report.documents = touched.iter().map(|sub| clear_emptied_folder(sub)).sum();
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

    /// `specs/0076-single-line-fonts` criterion 3: an SVG font is added like a TTF and removed
    /// with its family; an SVG that is not a font is "not a font".
    #[test]
    fn ac03_svg_fonts_are_added_and_removed() {
        let dir = tempfile::tempdir().unwrap();
        let folder = dir.path().join("fonts");
        let hershey = dir.path().join("Hershey.svg");
        std::fs::copy(included("hershey-sans-1/HersheySans1.svg"), &hershey).unwrap();
        let logo = dir.path().join("logo.svg");
        std::fs::write(
            &logo,
            br#"<svg xmlns="http://www.w3.org/2000/svg"><rect/></svg>"#,
        )
        .unwrap();
        let added = add_font_files(&folder, &[hershey.clone(), logo]);
        assert_eq!(added.added, ["Hershey.svg"]);
        assert_eq!(added.not_fonts, ["logo.svg"]);
        assert_eq!(add_font_files(&folder, &[hershey]).present, ["Hershey.svg"]);
        let removed = remove_font_files(&folder, &[(folder.join("Hershey.svg"), 0)]);
        assert_eq!(removed.removed, ["Hershey.svg"]);
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
        let mut names: Vec<String> = top_level_files(&folder)
            .iter()
            .map(|p| name_of(p))
            .collect();
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

    /// `0083` criterion 14: a family in a font folder is removed with the folder's documents
    /// when it was the last font; a folder that keeps another font keeps its documents; a
    /// maker's other file keeps the folder.
    #[test]
    fn ac14_remove_from_a_font_folder() {
        let dir = tempfile::tempdir().unwrap();
        let folder = dir.path().join("fonts");
        let lobster = folder.join("Lobster");
        std::fs::create_dir_all(&lobster).unwrap();
        let vibes = lobster.join("Vibes.ttf");
        let serif = lobster.join("Serif.ttf");
        std::fs::copy(included("great-vibes/GreatVibes-Regular.ttf"), &vibes).unwrap();
        std::fs::copy(included("source-serif-4/SourceSerif4-Bold.ttf"), &serif).unwrap();
        std::fs::write(lobster.join("OFL.txt"), b"licence").unwrap();
        std::fs::write(lobster.join("README.md"), b"readme").unwrap();
        let first = remove_font_files(&folder, &[(vibes.clone(), 0)]);
        assert_eq!((first.removed.len(), first.documents), (1, 0));
        assert!(lobster.join("OFL.txt").exists());
        let second = remove_font_files(&folder, &[(serif, 0)]);
        assert_eq!((second.removed.len(), second.documents), (1, 2));
        assert!(!lobster.exists());
        let kept = folder.join("Kept");
        std::fs::create_dir(&kept).unwrap();
        std::fs::copy(
            included("great-vibes/GreatVibes-Regular.ttf"),
            kept.join("V.ttf"),
        )
        .unwrap();
        std::fs::write(kept.join("OFL.txt"), b"licence").unwrap();
        std::fs::write(kept.join("photo.png"), b"mine").unwrap();
        let third = remove_font_files(&folder, &[(kept.join("V.ttf"), 0)]);
        assert_eq!(third.documents, 1);
        assert!(kept.join("photo.png").exists());
        let deep = kept.join("deeper");
        std::fs::create_dir(&deep).unwrap();
        std::fs::copy(
            included("great-vibes/GreatVibes-Regular.ttf"),
            deep.join("D.ttf"),
        )
        .unwrap();
        assert_eq!(
            remove_font_files(&folder, &[(deep.join("D.ttf"), 0)]).kept,
            ["D.ttf"]
        );
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
