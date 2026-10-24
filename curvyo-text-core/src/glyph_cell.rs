//! Draws one character of a face as a glyph table cell (`specs/0082-glyph-table` criteria 7, 8
//! and 21; `adrs.md` decision 3): the face's own glyph, unshaped, in cap-height units, so one
//! drawing serves every Size.

use skrifa::instance::{LocationRef, Size};
use skrifa::{FontRef, GlyphId, MetadataProvider};

use crate::face::{Face, Source};
use crate::preview::push_path_data;

/// One cell's drawing: 1.0 is the face's cap height, y down, origin at the glyph origin on the
/// baseline.
#[derive(Debug, Clone, PartialEq)]
pub struct GlyphCell {
    /// The outline as SVG path data; empty for an empty glyph.
    pub d: String,
    /// The glyph's advance.
    pub advance: f64,
    /// The glyph draws nothing (a space, a zero-width character).
    pub empty: bool,
}

/// `c` in `face` as a cell; `None` when the face has no glyph for it.
#[must_use]
pub fn glyph_cell(face: &Face, c: char) -> Option<GlyphCell> {
    let glyph = face.glyph_for(c);
    if glyph == 0 {
        return None;
    }
    let cap = face.metrics().cap_height;
    let outline = face.outline(glyph);
    let mut d = String::new();
    push_path_data(&mut d, &outline.path, 1.0 / cap);
    let advance_em = match face.source() {
        Source::OpenType { .. } => face
            .open_type_bytes()
            .and_then(|bytes| FontRef::from_index(bytes, face.index()).ok())
            .and_then(|font| {
                font.glyph_metrics(Size::unscaled(), LocationRef::default())
                    .advance_width(GlyphId::new(glyph))
            })
            .map_or(0.0, |units| f64::from(units) / face.units_per_em()),
        Source::Svg(font) => font
            .glyphs
            .get(usize::try_from(glyph).unwrap_or(0))
            .map_or(0.0, |(advance, _)| advance / font.units_per_em),
    };
    Some(GlyphCell {
        empty: outline.path.elements().is_empty(),
        d,
        advance: if advance_em.is_finite() {
            advance_em / cap
        } else {
            0.0
        },
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::face_store::FaceStore;

    fn face(path: &str) -> Arc<Face> {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let bytes = std::fs::read(dir.join("../frontend/public/fonts").join(path)).unwrap();
        let mut store = FaceStore::new();
        let key = store.add(bytes, 0).unwrap();
        store.get(&key).unwrap().clone()
    }

    #[test]
    fn a_cell_is_drawn_in_cap_height_units() {
        let sans = face("source-sans-3/SourceSans3-Regular.otf");
        let h = glyph_cell(&sans, 'H').unwrap();
        assert!(!h.empty);
        assert!(h.advance > 0.5 && h.advance < 1.5, "{}", h.advance);
        // The top of the H is the cap height: -1.0 in cap units, y down.
        let top =
            h.d.split(|c: char| c.is_ascii_alphabetic())
                .flat_map(|part| {
                    part.split_whitespace()
                        .skip(1)
                        .step_by(2)
                        .map(str::to_owned)
                        .collect::<Vec<_>>()
                })
                .filter_map(|y| y.parse::<f64>().ok())
                .fold(f64::INFINITY, f64::min);
        assert!((top + 1.0).abs() < 1e-3, "{top}");
    }

    #[test]
    fn a_space_is_an_empty_cell_and_an_unmapped_character_none() {
        let sans = face("source-sans-3/SourceSans3-Regular.otf");
        let space = glyph_cell(&sans, ' ').unwrap();
        assert!(space.empty);
        assert_eq!(space.d, "");
        assert!(space.advance > 0.0);
        assert_eq!(glyph_cell(&sans, '漢'), None);
    }

    #[test]
    fn a_single_line_glyph_is_drawn_as_its_centre_lines() {
        let script = face("hershey-script-1/HersheyScript1.svg");
        let a = glyph_cell(&script, 'a').unwrap();
        assert!(!a.empty);
        assert!(!a.d.contains('Z'), "open centre lines, not closed areas");
        assert!(a.advance > 0.0);
    }
}
