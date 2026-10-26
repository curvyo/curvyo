//! The words of the SVG import (`specs/0024-svg-import-export` criteria 3, 6 to 8 and its UX
//! notes): the refusal sentences, the one notice, the busy notices and the file name as notices
//! show it.

use curvyo_svg_core::SvgError;

/// The most characters a file name keeps in a notice.
const NOTICE_NAME_CHARS: usize = 24;

/// The characters kept before the extension at the end of a cut name.
const KEPT_BEFORE_EXTENSION: usize = 4;

/// What a file is, by its content (criterion 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileKind {
    /// An SVG drawing (or bytes the SVG reader refuses with a reason: gzip, UTF-16).
    Svg,
    /// A Curvyo project container (a zip).
    Project,
    /// A PNG, JPEG or WebP picture (`specs/0058-bitmap-import` criterion 2).
    Image,
    /// Neither.
    Other,
}

/// The kind of `bytes`: `<` as the first non-blank byte (after a UTF-8 byte order mark), gzip
/// bytes or a UTF-16 byte order mark is an SVG; the zip signature is a project.
#[must_use]
pub fn file_kind(bytes: &[u8]) -> FileKind {
    use curvyo_svg_core::xml_guard::{is_gzip, is_utf16, looks_like_xml};
    if curvyo_image_core::ImageFormat::detect(bytes).is_some() {
        FileKind::Image
    } else if looks_like_xml(bytes) || is_gzip(bytes) || is_utf16(bytes) {
        FileKind::Svg
    } else if bytes.starts_with(b"PK") {
        FileKind::Project
    } else {
        FileKind::Other
    }
}

/// The refusal of Open for a file that is neither a project nor an SVG (criterion 6 (a)).
pub const NEITHER_PROJECT_NOR_SVG: &str =
    "This file is not a Curvyo project or an SVG drawing. Nothing was changed.";

/// The error dialog's sentence for an SVG that gives no drawing (criterion 6, UX table).
#[must_use]
pub fn svg_refusal_text(error: SvgError) -> String {
    match error {
        SvgError::NotSvg(Some((line, column))) => format!(
            "This file is not an SVG drawing (line {line}, column {column}). Nothing was changed."
        ),
        SvgError::NotSvg(None) => "This file is not an SVG drawing. Nothing was changed.".into(),
        SvgError::Entities => "This file defines XML entities, which Curvyo does not read because \
                               they can be used to exhaust memory. Nothing was changed."
            .into(),
        SvgError::TooLarge => "This file is larger than 50 MiB. Nothing was changed.".into(),
        SvgError::NotUtf8 => {
            "Only UTF-8 SVG files can be opened. Save the file as plain SVG. Nothing was changed."
                .into()
        }
        SvgError::Compressed => "Compressed SVG (.svgz) is not supported. Save the file as plain \
                                 SVG. Nothing was changed."
            .into(),
    }
}

/// A file name as a notice shows it: at most 24 characters, cut in the middle with an ellipsis,
/// the extension and the four characters before it kept ("my-very-long-po…ster.svg").
#[must_use]
pub fn notice_file_name(name: &str) -> String {
    let chars: Vec<char> = name.chars().collect();
    if chars.len() <= NOTICE_NAME_CHARS {
        return name.to_owned();
    }
    let extension = name.rfind('.').map_or(0, |dot| name[dot..].chars().count());
    let tail = (extension + KEPT_BEFORE_EXTENSION).min(NOTICE_NAME_CHARS / 2);
    let head = NOTICE_NAME_CHARS - 1 - tail;
    let mut cut: String = chars[..head].iter().collect();
    cut.push('…');
    cut.extend(&chars[chars.len() - tail..]);
    cut
}

/// The one notice of an import that brought in nothing (criterion 8).
#[must_use]
pub fn nothing_imported_notice(file_name: &str) -> String {
    format!(
        "Nothing in {} could be imported.",
        notice_file_name(file_name)
    )
}

/// The notice while a file of 1 MiB or more is read (UX notes, "Large files").
#[must_use]
pub fn svg_busy_notice(opening: bool, file_name: &str) -> String {
    let verb = if opening { "Opening" } else { "Importing" };
    format!("{verb} {}...", notice_file_name(file_name))
}

/// The name of the layer Import SVG… makes: the file name without its extension (criterion 3).
#[must_use]
pub fn import_layer_name(file_name: &str) -> String {
    let stem = file_name
        .rsplit_once('.')
        .map_or(file_name, |(stem, _)| stem);
    curvyo_document_core::clean_name(stem).unwrap_or_else(|| "SVG".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_cut_in_the_middle_keeping_the_extension() {
        assert_eq!(notice_file_name("poster.svg"), "poster.svg");
        let cut = notice_file_name("my-very-long-poster-for-the-fair-poster.svg");
        assert_eq!(cut.chars().count(), 24);
        assert_eq!(cut, "my-very-long-po…ster.svg");
        assert_eq!(
            nothing_imported_notice("poster.svg"),
            "Nothing in poster.svg could be imported."
        );
        assert_eq!(
            svg_busy_notice(false, "poster.svg"),
            "Importing poster.svg..."
        );
        assert_eq!(import_layer_name("poster.svg"), "poster");
        assert_eq!(import_layer_name("  .svg"), "SVG");
    }

    #[test]
    fn the_content_decides_the_kind() {
        assert_eq!(file_kind(b"\xEF\xBB\xBF  <svg/>"), FileKind::Svg);
        assert_eq!(file_kind(&[0x1F, 0x8B]), FileKind::Svg);
        assert_eq!(file_kind(b"PK\x03\x04"), FileKind::Project);
        assert_eq!(file_kind(b"hello"), FileKind::Other);
        assert_eq!(file_kind(b"\x89PNG\r\n\x1a\n"), FileKind::Image);
        assert_eq!(file_kind(b"RIFF\0\0\0\0WEBPVP8 "), FileKind::Image);
        assert_eq!(
            svg_refusal_text(SvgError::NotSvg(Some((3, 14)))),
            "This file is not an SVG drawing (line 3, column 14). Nothing was changed."
        );
    }
}
