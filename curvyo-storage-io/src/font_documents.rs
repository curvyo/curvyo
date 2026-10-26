//! Finds a document of a font folder by folder and file name for the info view
//! (`specs/0083-font-info-and-zip` criterion 4, `adrs.md` decision 7): only a document file
//! directly in a listed subfolder of the Curvyo folder is ever read or handed to a viewer.

use std::io::Read;
use std::path::{Path, PathBuf};

use curvyo_text_core::is_document_name;

use crate::font_subfolders::is_listed_folder_name;

/// The most bytes of a text document shown (1 MiB, criterion 4).
pub const MAX_SHOWN_DOCUMENT_BYTES: u64 = 1024 * 1024;

/// Whether `name` is one plain path part (no separator, not `.` or `..`).
fn is_plain_part(name: &str) -> bool {
    !name.is_empty() && name != "." && name != ".." && !name.contains(['/', '\\', '\0'])
}

/// The document `name` of the font folder `folder` in the Curvyo folder `dir`, when it is a
/// regular document file directly in a listed subfolder (canonical paths compared, so no link
/// leads out).
fn document_path(dir: &Path, folder: &str, name: &str) -> Option<PathBuf> {
    if !is_plain_part(folder)
        || !is_listed_folder_name(folder)
        || !is_plain_part(name)
        || !is_document_name(name)
    {
        return None;
    }
    let root = dir.canonicalize().ok()?;
    let real = dir.join(folder).join(name).canonicalize().ok()?;
    let parent = real.parent()?;
    let inside =
        parent.parent() == Some(root.as_path()) && parent.file_name().is_some_and(|f| f == folder);
    (inside && std::fs::metadata(&real).is_ok_and(|meta| meta.is_file())).then_some(real)
}

/// The first 1 MiB of a font folder's document and whether it was cut there; `None` when it is
/// not such a document or cannot be read.
#[must_use]
pub fn read_font_document(dir: &Path, folder: &str, name: &str) -> Option<(Vec<u8>, bool)> {
    let path = document_path(dir, folder, name)?;
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .ok()?
        .take(MAX_SHOWN_DOCUMENT_BYTES + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    let cut = bytes.len() as u64 > MAX_SHOWN_DOCUMENT_BYTES;
    bytes.truncate(usize::try_from(MAX_SHOWN_DOCUMENT_BYTES).unwrap_or(usize::MAX));
    Some((bytes, cut))
}

/// The path of a font folder's `.pdf` document for the system viewer; `None` for any other
/// file.
#[must_use]
pub fn font_document_pdf(dir: &Path, folder: &str, name: &str) -> Option<PathBuf> {
    let pdf = Path::new(name)
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("pdf"));
    pdf.then(|| document_path(dir, folder, name)).flatten()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Criterion 4: a document is read up to 1 MiB; names that leave the folder, files that are
    /// not documents and PDFs outside a font folder are refused.
    #[test]
    fn ac04_documents_are_found_only_inside_font_folders() {
        let dir = tempfile::tempdir().unwrap();
        let fonts = dir.path().join("fonts");
        std::fs::create_dir_all(fonts.join("Lobster")).unwrap();
        std::fs::write(fonts.join("Lobster/OFL.txt"), b"licence").unwrap();
        std::fs::write(fonts.join("Lobster/Big.txt"), vec![b'a'; 1024 * 1024 + 5]).unwrap();
        std::fs::write(fonts.join("Lobster/Licence.pdf"), b"%PDF").unwrap();
        std::fs::write(fonts.join("Lobster/Lobster.ttf"), b"x").unwrap();
        std::fs::write(fonts.join("top.pdf"), b"%PDF").unwrap();
        std::fs::write(dir.path().join("secret.txt"), b"secret").unwrap();
        assert_eq!(
            read_font_document(&fonts, "Lobster", "OFL.txt"),
            Some((b"licence".to_vec(), false))
        );
        let (big, cut) = read_font_document(&fonts, "Lobster", "Big.txt").unwrap();
        assert_eq!((big.len(), cut), (1024 * 1024, true));
        assert!(font_document_pdf(&fonts, "Lobster", "Licence.pdf").is_some());
        for (folder, name) in [
            ("Lobster", "Lobster.ttf"),
            ("..", "secret.txt"),
            ("Lobster", "../../secret.txt"),
            (".", "top.pdf"),
            ("Lobster", "OFL.txt/"),
        ] {
            assert_eq!(
                read_font_document(&fonts, folder, name),
                None,
                "{folder}/{name}"
            );
        }
        assert_eq!(font_document_pdf(&fonts, "Lobster", "OFL.txt"), None);
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(
                dir.path().join("secret.txt"),
                fonts.join("Lobster/link.txt"),
            )
            .unwrap();
            assert_eq!(read_font_document(&fonts, "Lobster", "link.txt"), None);
        }
    }
}
