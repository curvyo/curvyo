//! The letters of an imported text as Import SVG lays them out, as plain outlines for a preview
//! (`specs/0087-image-clipboard-and-drop` criterion 26): the same pieces, faces and positions as
//! the text objects of `svg_import`, laid out by the engine as Object to path does.

use curvyo_document_core::{
    FontKind, FontRef, MISSING_FACE_HASH, OnPathValues, Point, TextGlyphOrientation, TextSnapshot,
    TextWritingMode, UNCREATED_TEXT_ID, Vec2,
};
use curvyo_geometry_core::PathMeasure;
use curvyo_svg_core::{LeafStyle, SvgText};
use curvyo_text_core::{Face, FaceHash, FaceKey, TAPER_ACCURACY};
use kurbo::{Affine, BezPath, PathEl};

use crate::svg_font_match::FaceMemo;
use crate::svg_import_style::model_style;
use crate::svg_text::{TextPiece, convert_text};
use crate::svg_text_layout::ImportFaces;
use crate::text_layout::{taper_accuracy, text_layout};
use crate::text_on_path::PathPlacement;

/// The letters of one text object of an imported text.
#[derive(Debug, Clone, PartialEq)]
pub struct SvgTextGlyphs {
    /// Every contour of its glyphs, in the file's millimetres.
    pub path: BezPath,
    /// Its paint; a single-line face's letters have no fill (`0076` criterion 8).
    pub paint: LeafStyle,
}

/// The letters of `text` in the faces of `fonts`, one entry per text object Import SVG makes of
/// it; `carrier` gives the first outline (millimetres) of the leaf a `<textPath>` names. A piece
/// no face can draw, or whose carrier is not given, is left out.
#[must_use]
pub fn svg_text_glyphs(
    text: &SvgText,
    carrier: &dyn Fn(usize) -> Option<BezPath>,
    fonts: &ImportFaces<'_>,
) -> Vec<SvgTextGlyphs> {
    let memo = FaceMemo::new(fonts.faces);
    convert_text(text, fonts, &memo)
        .into_iter()
        .filter_map(|piece| {
            let face = face_of(&piece.text.font, fonts)?;
            let path = match piece.on_path {
                Some((leaf, values)) => on_path(&piece, face, &carrier(leaf)?, values)?,
                None => straight(&piece, face)?,
            };
            let mut paint = text
                .styles
                .iter()
                .find(|style| model_style(&style.paint) == piece.text.style)?
                .paint
                .clone();
            if piece.text.font.kind == FontKind::SingleLine {
                paint.fill = None;
            }
            Some(SvgTextGlyphs { path, paint })
        })
        .collect()
}

/// The loaded face `font` names: the substitute for a missing face.
fn face_of<'a>(font: &FontRef, fonts: &ImportFaces<'a>) -> Option<&'a Face> {
    let key = if font.face_hash == MISSING_FACE_HASH {
        fonts.substitute?
    } else {
        FaceKey {
            hash: FaceHash::from_hex(&font.face_hash)?,
            index: font.face_index,
        }
    };
    fonts.store.get(&key).map(AsRef::as_ref)
}

fn snapshot(piece: &TextPiece) -> TextSnapshot {
    let text = &piece.text;
    TextSnapshot {
        id: UNCREATED_TEXT_ID,
        string: text.string.clone(),
        font: text.font.clone(),
        size: text.size,
        layout: text.layout,
        chars: text.chars.clone(),
        placement: text.placement,
        style: text.style.clone(),
        outline: None,
        on_path: None,
        source: None,
    }
}

fn kurbo_affine(matrix: curvyo_document_core::Affine) -> Affine {
    Affine::new(matrix.to_array())
}

/// A straight piece's glyphs mapped by its placement.
fn straight(piece: &TextPiece, face: &Face) -> Option<BezPath> {
    let snapshot = snapshot(piece);
    let laid = text_layout(&snapshot, face)?;
    let map = kurbo_affine(snapshot.placement);
    let mut path = BezPath::new();
    for glyph in laid.glyph_paths_within(face, taper_accuracy(snapshot.placement)) {
        path.extend((map * glyph).elements().iter().copied());
    }
    Some(path)
}

/// A piece on its carrier: set horizontally along it (`0080` criterion 48), every glyph by its
/// cluster's frame.
fn on_path(
    piece: &TextPiece,
    face: &Face,
    carrier: &BezPath,
    values: OnPathValues,
) -> Option<BezPath> {
    let mut snapshot = snapshot(piece);
    snapshot.layout.writing_mode = TextWritingMode::Horizontal;
    snapshot.layout.orientation = TextGlyphOrientation::Auto;
    let laid = text_layout(&snapshot, face)?;
    let (anchors, closed) = anchors_of(carrier)?;
    let placement = PathPlacement::on_measure(PathMeasure::new(&anchors, closed), &laid, values);
    let mut path = BezPath::new();
    for (index, line) in laid.lines.iter().enumerate() {
        for glyph in &line.glyphs {
            let (Some(frame), Some(outline)) = (
                placement.frame(index, glyph.centre_x.0),
                laid.glyph_in_line(face, glyph, TAPER_ACCURACY),
            ) else {
                continue;
            };
            path.extend((kurbo_affine(frame) * outline).elements().iter().copied());
        }
    }
    Some(path)
}

/// A node and its handles toward the previous and the next node, relative to it.
type Node = (Point, Vec2, Vec2);

/// The first subpath of `path` as nodes with relative handles, and whether it is closed; `None`
/// without a segment.
fn anchors_of(path: &BezPath) -> Option<(Vec<Node>, bool)> {
    let point = |p: kurbo::Point| Point::new(p.x, p.y);
    let handle = |from: kurbo::Point, to: kurbo::Point| Vec2::new(to.x - from.x, to.y - from.y);
    let mut anchors: Vec<Node> = Vec::new();
    let mut last = kurbo::Point::ZERO;
    let mut closed = false;
    for element in path.elements() {
        match *element {
            PathEl::MoveTo(p) => {
                if !anchors.is_empty() {
                    break;
                }
                anchors.push((point(p), Vec2::ZERO, Vec2::ZERO));
                last = p;
            }
            PathEl::LineTo(p) => {
                anchors.push((point(p), Vec2::ZERO, Vec2::ZERO));
                last = p;
            }
            PathEl::QuadTo(c, p) => {
                let c1 = last + (c - last) * (2.0 / 3.0);
                let c2 = p + (c - p) * (2.0 / 3.0);
                anchors.last_mut()?.2 = handle(last, c1);
                anchors.push((point(p), handle(p, c2), Vec2::ZERO));
                last = p;
            }
            PathEl::CurveTo(c1, c2, p) => {
                anchors.last_mut()?.2 = handle(last, c1);
                anchors.push((point(p), handle(p, c2), Vec2::ZERO));
                last = p;
            }
            PathEl::ClosePath => {
                closed = true;
                break;
            }
        }
    }
    if closed && anchors.len() > 2 {
        // A closing segment that returns to the start ends on the first node itself.
        let (first, end) = (anchors[0].0, anchors[anchors.len() - 1].0);
        if (first.x - end.x).hypot(first.y - end.y) <= 1e-9 {
            // invariant: more than two anchors, checked above.
            #[allow(clippy::unwrap_used)]
            let end = anchors.pop().unwrap();
            anchors[0].1 = end.1;
        }
    }
    (anchors.len() > 1).then_some((anchors, closed))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_closed_square_has_four_nodes_and_an_open_curve_keeps_its_handles() {
        let square = BezPath::from_svg("M0 0 L10 0 L10 10 L0 10 Z").unwrap_or_default();
        let (anchors, closed) = anchors_of(&square).unwrap_or_default();
        assert!(closed);
        assert_eq!(anchors.len(), 4);
        let curve = BezPath::from_svg("M0 0 C 0 5 5 10 10 10 M 50 50 L 60 60").unwrap_or_default();
        let (anchors, closed) = anchors_of(&curve).unwrap_or_default();
        assert!(!closed);
        assert_eq!(anchors.len(), 2, "the first subpath only");
        assert_eq!(anchors[0].2, Vec2::new(0.0, 5.0));
        assert_eq!(anchors[1].1, Vec2::new(-5.0, 0.0));
    }
}
