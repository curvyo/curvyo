//! Writes a font that arrived from another device into the Curvyo font folder
//! (`specs/0116-library-sync` criteria 16 and 18): the bytes and the licence files it brought,
//! under the rules of Add font, so the next listing finds it like any added font.

use std::path::{Path, PathBuf};

use curvyo_text_core::{is_document_name, sanitised_name};

use crate::filesystem::write_atomic;
use crate::font_files::{every_font_file, free_name, is_listable, same_bytes_in};
use crate::font_subfolders::{folder_files, is_listed_folder_name};
use crate::fonts::has_font_extension;

/// One font that arrived: its file, the folder it sat in, and the licence files it brought.
pub struct SyncedFont<'a> {
    /// The file name, one plain path part.
    pub file_name: &'a str,
    /// The subfolder of the Curvyo folder the file sat in (`0083`), `None` at the top level.
    pub folder: Option<&'a str>,
    /// The file's bytes.
    pub bytes: &'a [u8],
    /// The licence and readme files, `(name, bytes)`.
    pub documents: &'a [(String, Vec<u8>)],
}

/// What writing one arrived font did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncedWrite {
    /// Written under this file name (a taken name gets `-2`, `-3`, ...).
    Written(String),
    /// A file with the same bytes is in the folder already; nothing was written.
    Present,
    /// The name is not a plain font file name or the bytes are not a font the listing reads.
    NotAFont,
}

/// A name that is one plain path part and one every system accepts as it is: what
/// `sanitised_name` would leave unchanged (no reserved Windows name, `:`, control character or
/// trailing dot).
fn is_plain_part(name: &str) -> bool {
    !name.is_empty()
        && name != "."
        && name != ".."
        && !name.contains(['/', '\\', '\0'])
        && sanitised_name(name) == name
}

/// The names of the files directly in the place the font goes.
fn taken_names(dir: &Path, folder: Option<&str>) -> Vec<String> {
    folder_files(dir)
        .files
        .into_iter()
        .filter(|(_, in_folder)| in_folder.as_deref() == folder)
        .filter_map(|(path, _)| path.file_name().map(|n| n.to_string_lossy().into_owned()))
        .collect()
}

/// Writes `font` into the Curvyo folder `dir` (created when missing): the same bytes as a file
/// already there are "present", a taken name gets a number, each file is written to a temporary
/// file and renamed; its documents are written beside it unless a file of that name is there.
#[must_use]
pub fn write_synced_font(dir: &Path, font: &SyncedFont<'_>) -> SyncedWrite {
    let folder_ok = font
        .folder
        .is_none_or(|folder| is_plain_part(folder) && is_listed_folder_name(folder));
    let path = Path::new(font.file_name);
    if !is_plain_part(font.file_name)
        || !folder_ok
        || !has_font_extension(path)
        || !is_listable(font.bytes)
    {
        return SyncedWrite::NotAFont;
    }
    let _ = std::fs::create_dir_all(dir);
    if same_bytes_in(&every_font_file(dir), font.bytes) {
        return SyncedWrite::Present;
    }
    let target_dir: PathBuf = font
        .folder
        .map_or_else(|| dir.to_path_buf(), |f| dir.join(f));
    if std::fs::create_dir_all(&target_dir).is_err() {
        return SyncedWrite::NotAFont;
    }
    let name = free_name(font.file_name, &taken_names(dir, font.folder));
    if write_atomic(&target_dir.join(&name), font.bytes).is_err() {
        return SyncedWrite::NotAFont;
    }
    for (document, bytes) in font.documents {
        let place = target_dir.join(document);
        if is_plain_part(document) && is_document_name(document) && !place.exists() {
            let _ = write_atomic(&place, bytes);
        }
    }
    SyncedWrite::Written(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn serif() -> Vec<u8> {
        std::fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../frontend/public/fonts/source-serif-4/SourceSerif4-Bold.ttf"),
        )
        .unwrap()
    }

    /// Criteria 16 and 18: a font that arrived is written with its licence file into its folder,
    /// the listing finds it, and the same bytes again are present; a taken name gets `-2`.
    #[test]
    fn an_arrived_font_is_written_with_its_documents_and_listed() {
        let dir = tempfile::tempdir().unwrap();
        let folder = dir.path().join("fonts");
        let bytes = serif();
        let documents = vec![("OFL.txt".to_owned(), b"licence".to_vec())];
        let font = SyncedFont {
            file_name: "Serif.ttf",
            folder: Some("Source Serif"),
            bytes: &bytes,
            documents: &documents,
        };
        assert_eq!(
            write_synced_font(&folder, &font),
            SyncedWrite::Written("Serif.ttf".to_owned())
        );
        assert_eq!(
            std::fs::read(folder.join("Source Serif/OFL.txt")).unwrap(),
            b"licence"
        );
        assert_eq!(write_synced_font(&folder, &font), SyncedWrite::Present);
        let listed = crate::list_font_folder(&folder);
        assert_eq!(listed.faces.len(), 1);
        assert_eq!(listed.faces[0].folder.as_deref(), Some("Source Serif"));
        let other = std::fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../frontend/public/fonts/great-vibes/GreatVibes-Regular.ttf"),
        )
        .unwrap();
        let same_name = SyncedFont {
            file_name: "Serif.ttf",
            folder: Some("Source Serif"),
            bytes: &other,
            documents: &[],
        };
        assert_eq!(
            write_synced_font(&folder, &same_name),
            SyncedWrite::Written("Serif-2.ttf".to_owned())
        );
    }

    /// A path in a name, a hidden folder and bytes that are no font write nothing.
    #[test]
    fn names_that_are_paths_and_non_fonts_are_refused() {
        let dir = tempfile::tempdir().unwrap();
        let bytes = serif();
        for (name, folder) in [
            ("../x.ttf", None),
            ("a.ttf", Some(".hidden")),
            ("a.ttf", Some("a/b")),
            ("CON.ttf", None),
            ("a:b.ttf", None),
            ("nul.otf", None),
            ("a.ttf", Some("AUX")),
        ] {
            let font = SyncedFont {
                file_name: name,
                folder,
                bytes: &bytes,
                documents: &[],
            };
            assert_eq!(write_synced_font(dir.path(), &font), SyncedWrite::NotAFont);
        }
        let junk = SyncedFont {
            file_name: "junk.ttf",
            folder: None,
            bytes: b"not a font",
            documents: &[],
        };
        assert_eq!(write_synced_font(dir.path(), &junk), SyncedWrite::NotAFont);
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
    }
}
