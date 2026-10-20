//! The faces of a font file the Curvyo block lists, with their family and style names, read from
//! bytes (`specs/0075-font-library` criteria 28 and 32, `adrs.md` decision 3): the browser and
//! Add font name a face the way the desktop listing (`fontdb`) does.

use skrifa::string::StringId;
use skrifa::{FontRef, MetadataProvider, Tag};

use std::sync::Arc;

use crate::face::Face;
use crate::limits::MAX_FACE_BYTES;

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

/// The style word of an OS/2 weight class, as the desktop listing words it.
const fn weight_word(weight: u16) -> &'static str {
    match weight {
        0..=149 => "Thin",
        150..=249 => "Extra Light",
        250..=349 => "Light",
        350..=449 => "",
        450..=549 => "Medium",
        550..=649 => "Semibold",
        650..=749 => "Bold",
        750..=849 => "Extra Bold",
        _ => "Black",
    }
}

/// The style word of an OS/2 width class (1 to 9; anything else is normal).
const fn width_word(width: u16) -> &'static str {
    match width {
        1 => "Ultra Condensed",
        2 => "Extra Condensed",
        3 => "Condensed",
        4 => "Semi Condensed",
        6 => "Semi Expanded",
        7 => "Expanded",
        8 => "Extra Expanded",
        9 => "Ultra Expanded",
        _ => "",
    }
}

/// The style name of face `index` from its OS/2 weight, width and slant, in the words the
/// desktop listing uses (`curvyo-storage-io`): "Regular", "Bold Italic", "Condensed Light". A
/// face without OS/2 is "Regular" unless `post` states an italic angle.
#[must_use]
pub fn style_name(bytes: &[u8], index: u32) -> Option<String> {
    let font = FontRef::from_index(bytes, index).ok()?;
    let os2 = font.table_data(Tag::new(b"OS/2"));
    let os2 = os2.as_ref().map(skrifa::raw::FontData::as_bytes);
    let weight = os2.and_then(|t| be_u16(t, 4)).unwrap_or(400);
    let width = os2.and_then(|t| be_u16(t, 6)).unwrap_or(5);
    let version = os2.and_then(|t| be_u16(t, 0)).unwrap_or(0);
    let selection = os2.and_then(|t| be_u16(t, 62)).unwrap_or(0);
    let post_italic = font
        .table_data(Tag::new(b"post"))
        .and_then(|post| post.as_bytes().get(4..8).map(|angle| angle != [0, 0, 0, 0]))
        .unwrap_or(false);
    let slant = if selection & 1 != 0 {
        "Italic"
    } else if version >= 4 && selection & (1 << 9) != 0 {
        "Oblique"
    } else if post_italic {
        "Italic"
    } else {
        ""
    };
    let words: Vec<&str> = [width_word(width), weight_word(weight), slant]
        .into_iter()
        .filter(|w| !w.is_empty())
        .collect();
    Some(if words.is_empty() {
        "Regular".to_owned()
    } else {
        words.join(" ")
    })
}

/// Whether face `index` has vector outlines and no colour or bitmap tables, the rule of the
/// Curvyo block (`0074` criterion 21).
fn is_outline_face(font: &FontRef<'_>) -> bool {
    let has = |tag: &[u8; 4]| font.table_data(Tag::new(tag)).is_some();
    let outlines = has(b"glyf") || has(b"CFF ") || has(b"CFF2");
    let colour = has(b"COLR") || has(b"CBDT") || has(b"sbix") || has(b"SVG ");
    outlines && !colour
}

/// The faces of a font file the Curvyo block would list: each face parses for the engine, has
/// outlines and no colour tables, and is named. Empty for a file over 64 MiB, a damaged file or
/// one that is not a font ("not a font", criteria 22 and 32).
#[must_use]
pub fn listable_faces(bytes: &[u8]) -> Vec<ListableFace> {
    if bytes.len() > MAX_FACE_BYTES {
        return Vec::new();
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
            })
        })
        .collect()
}
