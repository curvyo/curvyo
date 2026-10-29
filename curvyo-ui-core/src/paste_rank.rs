//! Which file of the clipboard a paste takes (`specs/0087-image-clipboard-and-drop` criteria 4, 5
//! and 7, `adrs.md` decision 1): content wins over the MIME label.

use crate::svg_import_text::{FileKind, file_kind};

/// How a clipboard file serves a paste; the highest rank is taken, the first of equals.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PasteRank {
    /// Not an image: text, SVG, a project, an unknown file without an image type. With nothing
    /// better the paste says the clipboard has no image (criterion 5).
    None,
    /// Labelled as a raster image (a BMP, GIF or TIFF type) but not PNG, JPEG or WebP by content:
    /// imported so that the import's refusal names it (criterion 7).
    Refused,
    /// PNG, JPEG or WebP by its content, whatever its label (criterion 1).
    Image,
}

/// The rank of a clipboard file of `bytes` with the MIME type `mime`.
#[must_use]
pub fn paste_rank(bytes: &[u8], mime: &str) -> PasteRank {
    match file_kind(bytes) {
        FileKind::Image => PasteRank::Image,
        FileKind::Svg | FileKind::Project => PasteRank::None,
        FileKind::Other => {
            let mime = mime.trim().to_ascii_lowercase();
            if mime.starts_with("image/") && !mime.starts_with("image/svg") {
                PasteRank::Refused
            } else {
                PasteRank::None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PNG: &[u8] =
        include_bytes!("../../curvyo-image-core/tests/fixtures/images/png_rgba_8.png");
    const SVG: &[u8] = b"<svg xmlns=\"http://www.w3.org/2000/svg\"/>";

    #[test]
    fn content_decides_over_the_label() {
        assert_eq!(paste_rank(PNG, "text/plain"), PasteRank::Image);
        assert_eq!(paste_rank(PNG, ""), PasteRank::Image);
        assert_eq!(paste_rank(SVG, "image/png"), PasteRank::None);
        assert_eq!(paste_rank(b"PK\x03\x04", "image/png"), PasteRank::None);
    }

    #[test]
    fn an_unsupported_raster_is_refused_by_the_import_and_text_is_nothing() {
        assert_eq!(paste_rank(b"GIF89a....", "image/gif"), PasteRank::Refused);
        assert_eq!(paste_rank(b"BM....", "image/bmp"), PasteRank::Refused);
        assert_eq!(paste_rank(b"hello", "text/plain"), PasteRank::None);
        assert_eq!(paste_rank(SVG, "image/svg+xml"), PasteRank::None);
        assert_eq!(paste_rank(b"<html>", "image/svg+xml"), PasteRank::None);
        assert!(PasteRank::Image > PasteRank::Refused && PasteRank::Refused > PasteRank::None);
    }
}
