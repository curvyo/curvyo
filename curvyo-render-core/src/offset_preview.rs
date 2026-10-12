//! The blue hollow outline of an offset result before it is applied (`specs/0038-path-offset`
//! criterion 2): every outline of the result, holes included, over a white casing, never stored.

use curvyo_document_core::{AnchorId, NewAnchor, Point, ViewTransform};

use crate::glyphs::DrawList;
use crate::stroke;
use crate::theme;

/// The id the drawn anchors carry; nothing reads it.
const UNUSED_ANCHOR_ID: AnchorId = AnchorId::new(0, 0);

/// A 1.5 px `--preview-new` outline of every closed polyline in `outlines` (corner points in
/// document millimetres), over a white casing, of constant screen width; no fill. Every casing
/// comes first, then every line, so no casing covers a line.
#[must_use]
pub fn build_offset_preview(outlines: &[Vec<Point>], view: ViewTransform) -> DrawList {
    let width_mm = theme::LIVE_PREVIEW_STROKE_PX / view.scale();
    let tolerance_mm = theme::DISPLAY_TOLERANCE_PX / view.scale();
    let polylines: Vec<Vec<NewAnchor>> = outlines
        .iter()
        .map(|points| {
            points
                .iter()
                .map(|&point| NewAnchor::corner(UNUSED_ANCHOR_ID, point))
                .collect()
        })
        .collect();
    let mut list = DrawList::default();
    for casing in [true, false] {
        let (width, color) = if casing {
            (
                width_mm * theme::CASING_WIDTH_FACTOR,
                theme::SELECTION_CASING,
            )
        } else {
            (width_mm, theme::PREVIEW_NEW)
        };
        for anchors in &polylines {
            list.extend(stroke::path_stroke(
                anchors,
                true,
                width,
                color,
                tolerance_mm,
            ));
        }
    }
    list
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_outline_is_drawn_hollow_in_the_preview_colour() {
        let square = |x0: f64, side: f64| {
            vec![
                Point::new(x0, x0),
                Point::new(x0 + side, x0),
                Point::new(x0 + side, x0 + side),
                Point::new(x0, x0 + side),
            ]
        };
        let ring = vec![square(0.0, 40.0), square(10.0, 20.0)];
        let list = build_offset_preview(&ring, ViewTransform::new(1.0, Point::new(0.0, 0.0)));
        assert!(
            list.triangles
                .iter()
                .all(|v| v.color == theme::PREVIEW_NEW || v.color == theme::SELECTION_CASING)
        );
        assert!(
            list.triangles
                .iter()
                .any(|v| v.color == theme::PREVIEW_NEW && (v.position.x - 10.0).abs() < 1.0),
            "the hole is drawn"
        );
        assert!(
            list.triangles
                .iter()
                .all(|v| (v.position.x - 20.0).abs() > 5.0 || (v.position.y - 20.0).abs() > 5.0),
            "no fill"
        );
        assert_eq!(
            build_offset_preview(&[], ViewTransform::new(1.0, Point::new(0.0, 0.0)))
                .triangle_count(),
            0
        );
    }
}
