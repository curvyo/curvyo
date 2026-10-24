//! Writes a string laid out in a face as SVG path data: the name of a Fonts-tab row drawn in a
//! single-line face's own centre lines (`specs/0076-single-line-fonts` criterion 6, `adrs.md`
//! decision 6), and the one-line sample of a font preview tile (`specs/0077-font-preview`
//! criteria 13 and 15, `adrs.md` decision 2), since browsers draw no SVG fonts and would fill a
//! missing character from another font.

use std::fmt::Write as _;

use kurbo::{PathEl, Rect};

use crate::face::Face;
use crate::layout::{LayoutOptions, layout};
use crate::units::Mm;

/// `text` in `face` at 1 em as path data (em units, y down, origin on the baseline) and its line
/// box, the whole line however wide; `None` beyond the engine's bound.
#[must_use]
pub fn preview_path(face: &Face, text: &str) -> Option<(String, Rect)> {
    preview_path_within(face, text, f64::INFINITY)
}

/// As [`preview_path`], but only the glyphs whose origin lies within `width_caps` cap heights
/// of the line start are drawn, so the one straddling that edge is kept: a preview tile shows
/// one line clipped at its edge (customer decision 2026-10-23) and need not draw past it. The
/// box stays the whole line's.
#[must_use]
pub fn preview_path_within(face: &Face, text: &str, width_caps: f64) -> Option<(String, Rect)> {
    let cap = face.metrics().cap_height;
    let mut laid = layout(face, text, Mm(cap), &LayoutOptions::default()).ok()?;
    let edge = width_caps * cap;
    for line in &mut laid.lines {
        line.glyphs.retain(|placed| placed.origin.x <= edge);
    }
    let mut d = String::new();
    for path in laid.glyph_paths(face) {
        for element in path.elements() {
            let _ = match *element {
                PathEl::MoveTo(p) => write!(d, "M{:.4} {:.4}", p.x, p.y),
                PathEl::LineTo(p) => write!(d, "L{:.4} {:.4}", p.x, p.y),
                PathEl::QuadTo(a, p) => write!(d, "Q{:.4} {:.4} {:.4} {:.4}", a.x, a.y, p.x, p.y),
                PathEl::CurveTo(a, b, p) => write!(
                    d,
                    "C{:.4} {:.4} {:.4} {:.4} {:.4} {:.4}",
                    a.x, a.y, b.x, b.y, p.x, p.y
                ),
                PathEl::ClosePath => write!(d, "Z"),
            };
        }
    }
    Some((d, laid.line_box()))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::face_store::FaceStore;

    fn face(path: &str) -> Arc<Face> {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let bytes = std::fs::read(dir.join(path)).unwrap();
        let mut store = FaceStore::new();
        let key = store.add(bytes, 0).unwrap();
        store.get(&key).unwrap().clone()
    }

    fn sans() -> Arc<Face> {
        face("../frontend/public/fonts/source-sans-3/SourceSans3-Regular.otf")
    }

    fn no_umlaut() -> Arc<Face> {
        face("tests/fixtures/svg_fonts/no-umlaut.svg")
    }

    const SAMPLE: &str = "Anna & Jonas 2026";

    #[test]
    fn a_sample_is_one_line_however_long() {
        let face = sans();
        let (_, one) = preview_path(&face, SAMPLE).unwrap();
        let long = SAMPLE.repeat(10);
        let (d, wide) = preview_path(&face, &long).unwrap();
        assert!((wide.height() - one.height()).abs() < 1e-9, "one line high");
        assert!(wide.width() > 9.0 * one.width(), "not cut, not wrapped");
        assert!(!d.contains('…'));
    }

    #[test]
    fn a_width_drops_the_glyphs_past_it_but_keeps_the_one_on_the_edge() {
        let face = sans();
        let (whole, line) = preview_path(&face, "HHHH").unwrap();
        let (none, _) = preview_path_within(&face, "HHHH", 0.0).unwrap();
        // Only the first H starts at 0, so it alone is drawn.
        assert_eq!(none.matches('M').count(), whole.matches('M').count() / 4);
        let advance = line.width() / 4.0 / face.metrics().cap_height;
        let (two, cut) = preview_path_within(&face, "HHHH", 1.5 * advance).unwrap();
        assert_eq!(two.matches('M').count(), whole.matches('M').count() / 2);
        assert!(
            (cut.width() - line.width()).abs() < 1e-9,
            "the box stays the line's"
        );
        let (all, _) = preview_path_within(&face, "HHHH", 10.0 * advance).unwrap();
        assert_eq!(all, whole);
    }

    #[test]
    fn an_empty_sample_draws_nothing() {
        let (d, _) = preview_path(&sans(), "").unwrap();
        assert_eq!(d, "");
    }

    #[test]
    fn missing_characters_are_named_once_in_order() {
        assert_eq!(no_umlaut().missing_chars("Jürgen"), vec!['ü']);
        assert_eq!(sans().missing_chars("Jürgen"), Vec::<char>::new());
        assert_eq!(no_umlaut().missing_chars("ü ä\tü\n"), vec!['ü', 'ä']);
        assert_eq!(sans().missing_chars("Jürgen 漢"), vec!['漢']);
    }
}
