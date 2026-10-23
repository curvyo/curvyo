//! The faces of a font file the Curvyo block lists, with their family and style names and kind,
//! read from bytes (`specs/0075-font-library` criteria 28 and 32, `adrs.md` decision 3;
//! `specs/0076-single-line-fonts` criterion 1): the browser and Add font name a face the way the
//! desktop listing (`fontdb`, and the SVG listing of `curvyo-storage-io`) does.

use skrifa::string::StringId;
use skrifa::{FontRef, MetadataProvider, Tag};

use crate::face_style::FaceStyle;

use std::sync::Arc;

use crate::face::{Face, FaceKind};
use crate::limits::MAX_FACE_BYTES;
use crate::svg_font::{looks_like_xml, svg_faces};

/// The most faces read from one collection file.
const MAX_COLLECTION_FACES: u32 = 256;

/// One face of a file the Curvyo block lists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListableFace {
    /// The face's index in the file.
    pub index: u32,
    /// The family name, as [`family_name`] gives it.
    pub family: String,
    /// The style name, as [`style_name`] gives it.
    pub style: String,
    /// Outline (OpenType) or single-line (SVG font).
    pub kind: FaceKind,
}

/// The family name of face `index`: the typographic family (name ID 16), else the family
/// (ID 1), the English (US) record first, else the first one; `None` without either.
#[must_use]
pub fn family_name(bytes: &[u8], index: u32) -> Option<String> {
    let font = FontRef::from_index(bytes, index).ok()?;
    [StringId::TYPOGRAPHIC_FAMILY_NAME, StringId::FAMILY_NAME]
        .into_iter()
        .find_map(|id| {
            font.localized_strings(id)
                .english_or_first()
                .map(|name| name.to_string())
                .filter(|name| !name.is_empty())
        })
}

fn be_u16(bytes: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_be_bytes(bytes.get(at..at + 2)?.try_into().ok()?))
}

/// The style name of face `index` from its OS/2 weight, width and slant, in the words the
/// desktop listing uses (`curvyo-storage-io`): "Regular", "Bold Italic", "Condensed Light". A
/// face without OS/2 is "Regular" unless `post` states an italic angle ([`FaceStyle`]).
#[must_use]
pub fn style_name(bytes: &[u8], index: u32) -> Option<String> {
    FaceStyle::from_bytes(bytes, index).map(|style| style.name())
}

/// Whether face `index` has vector outlines and no colour or bitmap tables, the rule of the
/// Curvyo block (`0074` criterion 21).
fn is_outline_face(font: &FontRef<'_>) -> bool {
    let has = |tag: &[u8; 4]| font.table_data(Tag::new(tag)).is_some();
    let outlines = has(b"glyf") || has(b"CFF ") || has(b"CFF2");
    let colour = has(b"COLR") || has(b"CBDT") || has(b"sbix") || has(b"SVG ");
    outlines && !colour
}

/// The faces of an SVG font file the Curvyo block would list: each `<font>` with a drawn glyph
/// that the engine reads (`0076` criteria 1, 3 and 23). Empty for a file over 64 MiB, one that is
/// not an SVG font, is not well-formed or breaks a limit. Reads only the SVG font reader, never
/// the OpenType one (`specs/0076-single-line-fonts/adrs.md` decision 5).
#[must_use]
pub fn svg_listable_faces(bytes: &[u8]) -> Vec<ListableFace> {
    if bytes.len() > MAX_FACE_BYTES {
        return Vec::new();
    }
    svg_faces(bytes)
        .into_iter()
        .map(|(index, family, style)| ListableFace {
            index,
            family,
            style,
            kind: FaceKind::SingleLine,
        })
        .collect()
}

/// The faces of a font file the Curvyo block would list: each face parses for the engine, has
/// outlines and no colour tables, and is named; or each `<font>` of an SVG font with a drawn
/// glyph (`0076` criterion 1). Empty for a file over 64 MiB, a damaged file or
/// one that is not a font ("not a font", criteria 22 and 32).
#[must_use]
pub fn listable_faces(bytes: &[u8]) -> Vec<ListableFace> {
    if bytes.len() > MAX_FACE_BYTES {
        return Vec::new();
    }
    if looks_like_xml(bytes) {
        return svg_listable_faces(bytes);
    }
    let count = if bytes.starts_with(b"ttcf") {
        be_u16(bytes, 8)
            .zip(be_u16(bytes, 10))
            .map_or(0, |(high, low)| {
                (u32::from(high) << 16 | u32::from(low)).min(MAX_COLLECTION_FACES)
            })
    } else {
        1
    };
    let shared = Arc::new(bytes.to_vec());
    (0..count)
        .filter_map(|index| {
            let font = FontRef::from_index(bytes, index).ok()?;
            if !is_outline_face(&font) {
                return None;
            }
            Face::new(Arc::clone(&shared), index).ok()?;
            Some(ListableFace {
                index,
                family: family_name(bytes, index)?,
                style: style_name(bytes, index)?,
                kind: FaceKind::Outline,
            })
        })
        .collect()
}
