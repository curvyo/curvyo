//! Reads the info fields of one face (`specs/0083-font-info-and-zip` criteria 2 and 5,
//! `adrs.md` decision 6) from the bytes the face already holds: name IDs, format, `fsType`, the
//! vendor and the glyph count. The words shown for them are the frontend's.

use skrifa::raw::TableProvider;
use skrifa::string::StringId;
use skrifa::{FontRef, MetadataProvider, Tag};

use crate::face::{Face, Source};

/// How a face's file is built (criterion 2, Format).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontFormat {
    /// TrueType outlines (`glyf`) in a single-face file.
    TrueType,
    /// CFF outlines in a single-face file.
    OpenTypeCff,
    /// CFF2 outlines in a single-face file.
    OpenTypeCff2,
    /// A face of a collection: its number (from 1) and the collection's face count.
    Collection {
        /// The face's number, from 1.
        face: u32,
        /// How many faces the collection holds.
        faces: u32,
    },
    /// An SVG font (single-line).
    SvgFont,
}

/// The info fields of one face; `None` where the font states nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontInfo {
    /// How the file is built.
    pub format: FontFormat,
    /// Name ID 4.
    pub full_name: Option<String>,
    /// Name ID 5.
    pub version: Option<String>,
    /// Name ID 9.
    pub designer: Option<String>,
    /// Name ID 8, else the OS/2 `achVendID` code.
    pub vendor: Option<String>,
    /// Name ID 0.
    pub copyright: Option<String>,
    /// Name ID 7.
    pub trademark: Option<String>,
    /// Name ID 13.
    pub licence: Option<String>,
    /// Name ID 14.
    pub licence_url: Option<String>,
    /// OS/2 `fsType`, the embedding permission bits.
    pub fs_type: Option<u16>,
    /// `maxp` glyph count; an SVG font's `<glyph>` count.
    pub glyphs: u32,
}

/// The faces a collection declares (bytes 8 to 11 of a `ttcf` header).
fn collection_faces(bytes: &[u8]) -> Option<u32> {
    bytes
        .starts_with(b"ttcf")
        .then(|| Some(u32::from_be_bytes(bytes.get(8..12)?.try_into().ok()?)))
        .flatten()
}

fn name(font: &FontRef<'_>, id: u16) -> Option<String> {
    font.localized_strings(StringId::new(id))
        .english_or_first()
        .map(|text| text.to_string().trim().to_owned())
        .filter(|text| !text.is_empty())
}

/// The fields of an OpenType face `index` of `bytes`.
fn opentype_info(bytes: &[u8], index: u32) -> Option<FontInfo> {
    let font = FontRef::from_index(bytes, index).ok()?;
    let has = |tag: &[u8; 4]| font.table_data(Tag::new(tag)).is_some();
    let format = match collection_faces(bytes) {
        Some(faces) => FontFormat::Collection {
            face: index + 1,
            faces,
        },
        None if has(b"CFF2") => FontFormat::OpenTypeCff2,
        None if has(b"CFF ") => FontFormat::OpenTypeCff,
        None => FontFormat::TrueType,
    };
    let os2 = font.os2().ok();
    let vendor_id = os2
        .as_ref()
        .map(|os2| {
            String::from_utf8_lossy(&os2.ach_vend_id().to_be_bytes())
                .trim()
                .to_owned()
        })
        .filter(|id| !id.is_empty() && id.chars().all(|c| c.is_ascii_graphic() || c == ' '));
    Some(FontInfo {
        format,
        full_name: name(&font, 4),
        version: name(&font, 5),
        designer: name(&font, 9),
        vendor: name(&font, 8).or(vendor_id),
        copyright: name(&font, 0),
        trademark: name(&font, 7),
        licence: name(&font, 13),
        licence_url: name(&font, 14),
        fs_type: os2.map(|os2| os2.fs_type()),
        glyphs: font.maxp().map_or(0, |maxp| u32::from(maxp.num_glyphs())),
    })
}

/// A face that states nothing but its format and glyph count.
const fn stated_nothing(format: FontFormat, glyphs: u32) -> FontInfo {
    FontInfo {
        format,
        full_name: None,
        version: None,
        designer: None,
        vendor: None,
        copyright: None,
        trademark: None,
        licence: None,
        licence_url: None,
        fs_type: None,
        glyphs,
    }
}

/// The info fields of `face` (criterion 2). An SVG font states only its glyph count here;
/// its family and style are those the listing shows.
#[must_use]
pub fn font_info(face: &Face) -> FontInfo {
    match face.source() {
        Source::OpenType { bytes, .. } => {
            opentype_info(bytes, face.index()).unwrap_or(stated_nothing(FontFormat::TrueType, 0))
        }
        // Glyph 0 is `<missing-glyph>`.
        Source::Svg(font) => stated_nothing(
            FontFormat::SvgFont,
            u32::try_from(font.glyphs.len().saturating_sub(1)).unwrap_or(u32::MAX),
        ),
    }
}
