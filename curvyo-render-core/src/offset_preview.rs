//! What the Offset tool draws on the canvas (`specs/0038-path-offset` criteria 9, 14, 18): the
//! blue hollow outline of the result over a white casing, never stored; the selection box with
//! the four round handles; and the dashed grown box that replaces the preview of a large
//! selection.

use curvyo_document_core::{AnchorId, AnchorKind, NewAnchor, Point, Vec2, ViewTransform};

use crate::glyphs::{DrawList, circle};
use crate::select_box::dashed_edge;
use crate::stroke;
use crate::theme;

/// The id the drawn anchors carry; nothing reads it.
const UNUSED_ANCHOR_ID: AnchorId = AnchorId::new(0, 0);

/// The handle's diameter, screen pixels (the "Parameter handle").
const HANDLE_PX: f64 = 10.0;

/// The handle's ring, screen pixels.
const HANDLE_RING_PX: f64 = 1.5;

/// The handle's centre dot, screen pixels.
const HANDLE_DOT_PX: f64 = 4.0;

/// One outline node: point and the two handles relative to it.
pub type PreviewAnchor = (Point, Vec2, Vec2);

/// A 1.5 px `--preview-new` outline of every closed outline in `outlines` (nodes with their
/// handles, in document millimetres), over a white casing, of constant screen width; no fill.
/// Every casing comes first, then every line, so no casing covers a line.
#[must_use]
pub fn build_offset_preview(outlines: &[Vec<PreviewAnchor>], view: ViewTransform) -> DrawList {
    let width_mm = theme::LIVE_PREVIEW_STROKE_PX / view.scale();
    let tolerance_mm = theme::DISPLAY_TOLERANCE_PX / view.scale();
    let paths: Vec<Vec<NewAnchor>> = outlines
        .iter()
        .map(|outline| {
            outline
                .iter()
                .map(|&(point, handle_in, handle_out)| NewAnchor {
                    id: UNUSED_ANCHOR_ID,
                    point,
                    handle_in,
                    handle_out,
                    kind: AnchorKind::Corner,
                })
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
        for anchors in &paths {
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

/// What the handles show.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct OffsetHandleState {
    /// The handle under the pointer, by corner index.
    pub hovered: Option<usize>,
    /// The handle being dragged, by corner index.
    pub dragged: Option<usize>,
}

/// The selection box (1 px dashed `--accent`) through `corners` and a round handle at each of
/// `handles` (10 px, 1.5 px `--accent` ring on white, 4 px `--accent` dot; hover lighter, the
/// dragged one filled), in document millimetres.
#[must_use]
pub fn build_offset_handles(
    corners: [Point; 4],
    handles: [Point; 4],
    state: OffsetHandleState,
    view: ViewTransform,
) -> DrawList {
    let mut list = dashed_box(corners, theme::ACCENT, view);
    let mm = |px: f64| px / view.scale();
    for (index, &at) in handles.iter().enumerate() {
        let ground = if state.dragged == Some(index) {
            theme::ACCENT
        } else {
            theme::SELECTION_CASING
        };
        let ring = if state.hovered == Some(index) && state.dragged.is_none() {
            theme::ACCENT_HOVER
        } else {
            theme::ACCENT
        };
        list.extend(circle(at, mm(HANDLE_PX), ring));
        list.extend(circle(at, mm(HANDLE_PX - 2.0 * HANDLE_RING_PX), ground));
        list.extend(circle(at, mm(HANDLE_DOT_PX), theme::ACCENT));
    }
    list
}

/// The dashed `--preview-new` box through `corners` that stands for the result of a selection
/// too large to preview (criterion 18).
#[must_use]
pub fn build_offset_grown_box(corners: [Point; 4], view: ViewTransform) -> DrawList {
    dashed_box(corners, theme::PREVIEW_NEW, view)
}

fn dashed_box(corners: [Point; 4], color: crate::RgbaColor, view: ViewTransform) -> DrawList {
    let mut list = DrawList::default();
    for i in 0..4 {
        let from = view.document_to_screen(corners[i]);
        let to = view.document_to_screen(corners[(i + 1) % 4]);
        list.extend(dashed_edge(
            view,
            from,
            to,
            theme::BOUNDING_BOX_OUTLINE_PX,
            &[],
            color,
            1.0,
        ));
    }
    list
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square(x0: f64, side: f64) -> Vec<PreviewAnchor> {
        [
            (x0, x0),
            (x0 + side, x0),
            (x0 + side, x0 + side),
            (x0, x0 + side),
        ]
        .into_iter()
        .map(|(x, y)| (Point::new(x, y), Vec2::ZERO, Vec2::ZERO))
        .collect()
    }

    #[test]
    fn every_outline_is_drawn_hollow_in_the_preview_colour() {
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

    #[test]
    fn four_handles_and_a_dashed_box() {
        let corners = [
            Point::new(0.0, 0.0),
            Point::new(100.0, 0.0),
            Point::new(100.0, 100.0),
            Point::new(0.0, 100.0),
        ];
        let handles = corners.map(|c| Point::new(c.x + 5.0, c.y + 5.0));
        let view = ViewTransform::new(1.0, Point::new(0.0, 0.0));
        let list = build_offset_handles(corners, handles, OffsetHandleState::default(), view);
        assert!(list.triangles.iter().any(|v| v.color == theme::ACCENT));
        let grown = build_offset_grown_box(corners, view);
        assert!(
            grown
                .triangles
                .iter()
                .all(|v| v.color == theme::PREVIEW_NEW)
        );
        assert!(grown.triangle_count() > 4, "dashed, not four solid lines");
    }
}
