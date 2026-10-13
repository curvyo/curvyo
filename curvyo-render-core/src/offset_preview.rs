//! What the Offset tool draws on the canvas (`specs/0038-path-offset` criteria 9, 14, 18): the
//! blue hollow outline of the result over a white casing, never stored; the selection box with
//! the four round handles; and the dashed grown box that replaces the preview of a large
//! selection.

use curvyo_document_core::{AnchorId, AnchorKind, NewAnchor, Point, Vec2, ViewTransform};

use crate::RgbaColor;
use crate::glyphs::{DrawList, circle, rounded_square};
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

/// The side of the Offset glyph in the center handle, in screen pixels.
const CENTER_GLYPH_PX: f64 = 10.0;

/// The ring width and corner radius of the center glyph's square outline, in screen pixels.
const CENTER_GLYPH_RING_PX: f64 = 1.5;
const CENTER_GLYPH_RADIUS_PX: f64 = 2.0;

/// The solid square inside the center glyph, in screen pixels.
const CENTER_GLYPH_DOT_PX: f64 = 4.0;

/// What the center handle shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct OffsetCenterState {
    /// The pointer is over it.
    pub hovered: bool,
    /// The Offset entry chip is open: solid `--accent` with a white glyph.
    pub open: bool,
}

/// The center handle of the Offset tool at `center` (`specs/0038-path-offset` criterion 53,
/// design-system row "Offset handles"): the shell of the Move center handle (16 px rounded square,
/// 1 px `--accent` outline, white ground, `--accent-hover` on hover, solid `--accent` while the
/// chip is open) around the Offset glyph in small (a 10 px square outline around a 4 px solid
/// square). Upright: it never turns with the box.
#[must_use]
pub fn build_offset_center_handle(
    center: Point,
    state: OffsetCenterState,
    view: ViewTransform,
) -> DrawList {
    let mm = |px: f64| px / view.scale();
    let size = mm(theme::TRANSFORM_MOVE_HANDLE_SIZE_PX);
    let outline = mm(theme::TRANSFORM_MOVE_HANDLE_OUTLINE_PX);
    let radius = mm(theme::TRANSFORM_MOVE_HANDLE_CORNER_RADIUS_PX);
    let inner = (size - 2.0 * outline).max(0.0);
    let inner_radius = (radius - outline).max(0.0);
    let (ground, glyph) = if state.open {
        (theme::ACCENT, RgbaColor::WHITE)
    } else if state.hovered {
        (theme::ACCENT_HOVER, theme::ACCENT)
    } else {
        (RgbaColor::WHITE, theme::ACCENT)
    };
    let mut list = rounded_square(center, size, radius, theme::ACCENT);
    list.extend(rounded_square(
        center,
        inner,
        inner_radius,
        RgbaColor::WHITE,
    ));
    if ground != RgbaColor::WHITE {
        list.extend(rounded_square(center, inner, inner_radius, ground));
    }
    let ring = mm(CENTER_GLYPH_RING_PX);
    let glyph_radius = mm(CENTER_GLYPH_RADIUS_PX);
    list.extend(rounded_square(
        center,
        mm(CENTER_GLYPH_PX),
        glyph_radius,
        glyph,
    ));
    let hole = mm(CENTER_GLYPH_PX) - 2.0 * ring;
    let hole_radius = (glyph_radius - ring).max(0.0);
    list.extend(rounded_square(center, hole, hole_radius, RgbaColor::WHITE));
    if ground != RgbaColor::WHITE {
        list.extend(rounded_square(center, hole, hole_radius, ground));
    }
    list.extend(rounded_square(center, mm(CENTER_GLYPH_DOT_PX), 0.0, glyph));
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
    fn ac53_the_center_handle_turns_solid_while_the_chip_is_open() {
        let view = ViewTransform::new(1.0, Point::new(0.0, 0.0));
        let at = Point::new(50.0, 50.0);
        let idle = build_offset_center_handle(at, OffsetCenterState::default(), view);
        assert!(idle.triangles.iter().any(|v| v.color == theme::ACCENT));
        assert!(
            idle.triangles
                .iter()
                .all(|v| (v.position.x - 50.0).abs() <= 8.01 && (v.position.y - 50.0).abs() <= 8.01),
            "16 px across"
        );
        let open = build_offset_center_handle(
            at,
            OffsetCenterState {
                hovered: false,
                open: true,
            },
            view,
        );
        assert_ne!(idle, open);
        let hovered = build_offset_center_handle(
            at,
            OffsetCenterState {
                hovered: true,
                open: false,
            },
            view,
        );
        assert!(
            hovered
                .triangles
                .iter()
                .any(|v| v.color == theme::ACCENT_HOVER)
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
