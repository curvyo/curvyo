//! Draws the regions of a trace mask over the picture in the Trace tool
//! (`specs/0114-image-trace-mask` Behaviour 2 to 4, UX notes §5): every part's region outlines in
//! black at 0.25 mm and at least 1 px (subtracted ones dashed), the hover box, the selected
//! region's dashed box with a rectangle's handles or a lasso's corner squares, the drag ghost, the
//! rectangle being drawn and the lasso in progress with its close cue. Widths are laid out in
//! screen pixels and converted to document millimetres, like every other decoration.

use curvyo_document_core::{Point, ViewTransform};

use crate::color::RgbaColor;
use crate::glyphs::{DrawList, ring, rounded_square, square, thick_line};
use crate::theme;

/// The outline width of a region in document millimetres, before the 1 px floor.
const OUTLINE_MM: f64 = 0.25;
/// A subtracted region's dashes and gaps, screen pixels.
const DASH_PX: (f64, f64) = (4.0, 3.0);
/// The lasso's closing side on the close target, screen pixels.
const CLOSE_DASH_PX: (f64, f64) = (6.0, 4.0);
/// Above this many dashes on one outline it is drawn solid (a frame must stay bounded).
const MAX_DASHES: f64 = 4096.0;
/// The outline colour.
const INK: RgbaColor = RgbaColor::opaque(0, 0, 0);

/// One region's closed outline in document millimetres.
#[derive(Debug, Clone, PartialEq)]
pub struct TraceMaskOutline {
    /// The corners, closed from the last to the first.
    pub corners: Vec<Point>,
    /// Drawn with Subtract: dashed.
    pub subtract: bool,
}

/// The selected region's decoration.
#[derive(Debug, Clone, PartialEq)]
pub struct TraceMaskSelection {
    /// The dashed box in the picture's frame, corners in order.
    pub frame: [Point; 4],
    /// A rectangle's eight resize handles, with the one being dragged.
    pub handles: Vec<Point>,
    /// The dragged handle's index into `handles`.
    pub dragged_handle: Option<usize>,
    /// A lasso's corners.
    pub corners: Vec<Point>,
    /// The lasso corner under the pointer (the hover ring).
    pub hovered_corner: Option<usize>,
    /// The lasso corner pressed last (filled).
    pub selected_corner: Option<usize>,
}

/// A lasso being drawn.
#[derive(Debug, Clone, PartialEq)]
pub struct TraceLassoDraft {
    /// The corners set so far.
    pub corners: Vec<Point>,
    /// Its sides dashed (Subtract is pressed).
    pub subtract: bool,
    /// The pointer, for the rubber band.
    pub pointer: Option<Point>,
    /// The pointer is on the close target: the closing side replaces the rubber band.
    pub closing: bool,
}

/// What the mask overlay draws this frame.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TraceMaskOverlay {
    /// Every part's regions, in order; a dragged region at its new geometry.
    pub outlines: Vec<TraceMaskOutline>,
    /// The hover box of the region under the pointer.
    pub hover: Option<[Point; 4]>,
    /// The old outline of a region being dragged.
    pub ghost: Option<Vec<Point>>,
    /// The rectangle being drawn (the live preview outline), dashed with Subtract.
    pub drawing: Option<TraceMaskOutline>,
    /// The selected region.
    pub selection: Option<TraceMaskSelection>,
    /// The lasso in progress.
    pub lasso: Option<TraceLassoDraft>,
}

/// The overlay as a draw list, to be drawn above every object.
#[must_use]
pub fn build_trace_mask_overlay(view: ViewTransform, overlay: &TraceMaskOverlay) -> DrawList {
    let mut list = DrawList::default();
    let scale = view.scale();
    if !(scale.is_finite() && scale > 0.0) {
        return list;
    }
    let px = 1.0 / scale;
    let width = OUTLINE_MM.max(px);
    for outline in &overlay.outlines {
        let dash = outline.subtract.then_some(DASH_PX);
        list.extend(polyline(&outline.corners, true, width, dash, px, INK));
    }
    if let Some(ghost) = &overlay.ghost {
        list.extend(cased(
            ghost,
            true,
            theme::LIVE_PREVIEW_STROKE_PX * px,
            None,
            px,
            theme::DRAG_GHOST,
        ));
    }
    if let Some(drawing) = &overlay.drawing {
        let dash = drawing.subtract.then_some(DASH_PX);
        let w = theme::LIVE_PREVIEW_STROKE_PX * px;
        list.extend(cased(
            &drawing.corners,
            true,
            w,
            dash,
            px,
            theme::PREVIEW_NEW,
        ));
    }
    if let Some(hover) = overlay.hover {
        list.extend(cased(&hover, true, px, None, px, theme::HOVER_BOX));
    }
    if let Some(selection) = &overlay.selection {
        list.extend(build_selection(selection, px));
    }
    if let Some(lasso) = &overlay.lasso {
        list.extend(build_lasso(lasso, width, px));
    }
    list
}

/// The dashed box, the handles or the corner squares of the selected region.
fn build_selection(selection: &TraceMaskSelection, px: f64) -> DrawList {
    let mut list = DrawList::default();
    let dash = (theme::SELECTION_BOX_DASH_PX, theme::SELECTION_BOX_GAP_PX);
    list.extend(polyline(
        &selection.frame,
        true,
        px,
        Some(dash),
        px,
        theme::ACCENT,
    ));
    let size = theme::TRANSFORM_RESIZE_HANDLE_SIZE_PX * px;
    let radius = theme::TRANSFORM_RESIZE_HANDLE_CORNER_RADIUS_PX * px;
    for (i, &handle) in selection.handles.iter().enumerate() {
        list.extend(rounded_square(handle, size, radius, theme::ACCENT));
        if selection.dragged_handle != Some(i) {
            let inner = size - 2.0 * theme::TRANSFORM_RESIZE_HANDLE_OUTLINE_PX * px;
            list.extend(rounded_square(
                handle,
                inner,
                (radius - px).max(0.0),
                RgbaColor::WHITE,
            ));
        }
    }
    let node = theme::NODE_SIZE_PX * px;
    let edge = theme::NODE_OUTLINE_PX * px;
    for (i, &corner) in selection.corners.iter().enumerate() {
        if selection.hovered_corner == Some(i) {
            list.extend(ring(
                corner,
                theme::HOVER_RING_DIAMETER_PX * px,
                2.0 * px,
                theme::ACCENT_HOVER,
            ));
        }
        if selection.selected_corner == Some(i) {
            list.extend(square(corner, node, theme::ACCENT));
        } else {
            list.extend(square(corner, node, theme::NODE_STROKE));
            list.extend(square(corner, node - 2.0 * edge, RgbaColor::WHITE));
        }
    }
    list
}

/// The lasso in progress: the Pen in progress look (UX notes §5).
fn build_lasso(lasso: &TraceLassoDraft, width: f64, px: f64) -> DrawList {
    let mut list = DrawList::default();
    let dash = lasso.subtract.then_some(DASH_PX);
    list.extend(polyline(
        &lasso.corners,
        false,
        width,
        dash,
        px,
        theme::ACCENT,
    ));
    if let (Some(&first), Some(&last)) = (lasso.corners.first(), lasso.corners.last()) {
        if lasso.closing {
            list.extend(polyline(
                &[last, first],
                false,
                px,
                Some(CLOSE_DASH_PX),
                px,
                theme::ACCENT,
            ));
            list.extend(ring(
                first,
                theme::HOVER_RING_DIAMETER_PX * px,
                2.0 * px,
                theme::ACCENT_HOVER,
            ));
        } else if let Some(pointer) = lasso.pointer {
            list.extend(thick_line(last, pointer, px, theme::ACCENT));
        }
        list.extend(ring(
            last,
            theme::HOVER_RING_DIAMETER_PX * px,
            2.0 * px,
            theme::ACCENT_HOVER,
        ));
    }
    let node = theme::NODE_SIZE_PX * px;
    for &corner in &lasso.corners {
        list.extend(square(corner, node, theme::ACCENT));
        list.extend(square(
            corner,
            node - 2.0 * theme::NODE_OUTLINE_PX * px,
            RgbaColor::WHITE,
        ));
    }
    list
}

/// `points` drawn as a line over its white casing (three times its width).
fn cased(
    points: &[Point],
    closed: bool,
    width: f64,
    dash: Option<(f64, f64)>,
    px: f64,
    color: RgbaColor,
) -> DrawList {
    let mut list = polyline(
        points,
        closed,
        width * theme::CASING_WIDTH_FACTOR,
        None,
        px,
        theme::SELECTION_CASING,
    );
    list.extend(polyline(points, closed, width, dash, px, color));
    list
}

/// The polyline through `points` (closed back to the first with `closed`), `width` thick, dashed
/// by `dash` (screen pixels, the phase running on from side to side from the first corner); drawn
/// solid when it would need more than [`MAX_DASHES`] dashes.
fn polyline(
    points: &[Point],
    closed: bool,
    width: f64,
    dash: Option<(f64, f64)>,
    px: f64,
    color: RgbaColor,
) -> DrawList {
    let mut list = DrawList::default();
    let count = if closed {
        points.len()
    } else {
        points.len().saturating_sub(1)
    };
    let sides: Vec<(Point, Point)> = (0..count)
        .map(|i| (points[i], points[(i + 1) % points.len()]))
        .collect();
    let length: f64 = sides.iter().map(|(a, b)| a.vector_to(*b).length()).sum();
    let Some((on, off)) = dash.map(|(on, off)| (on * px, off * px)) else {
        for (a, b) in sides {
            list.extend(thick_line(a, b, width, color));
        }
        return list;
    };
    if !(length.is_finite() && on + off > 0.0) || length / (on + off) > MAX_DASHES {
        for (a, b) in sides {
            list.extend(thick_line(a, b, width, color));
        }
        return list;
    }
    let mut phase = 0.0;
    for (a, b) in sides {
        let side = a.vector_to(b).length();
        if side <= 0.0 {
            continue;
        }
        let step = a.vector_to(b).scaled(1.0 / side);
        let mut at = 0.0;
        while at < side {
            let in_dash = phase < on;
            let left = if in_dash {
                on - phase
            } else {
                on + off - phase
            };
            // The sliver left of a dash can be smaller than the float step at `at`; `used` moves
            // the phase on exactly, so the walk always reaches the next dash or gap.
            let used = left.min(side - at);
            let end = (at + used).min(side);
            if in_dash {
                list.extend(thick_line(
                    a.translated(step.scaled(at)),
                    a.translated(step.scaled(end)),
                    width,
                    color,
                ));
            }
            phase = (phase + used) % (on + off);
            at = end;
        }
    }
    list
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square_outline(subtract: bool) -> TraceMaskOutline {
        TraceMaskOutline {
            corners: vec![
                Point::new(0.0, 0.0),
                Point::new(10.0, 0.0),
                Point::new(10.0, 10.0),
                Point::new(0.0, 10.0),
            ],
            subtract,
        }
    }

    fn overlay(outlines: Vec<TraceMaskOutline>) -> TraceMaskOverlay {
        TraceMaskOverlay {
            outlines,
            ..TraceMaskOverlay::default()
        }
    }

    #[test]
    fn a_subtracted_outline_is_dashed_and_a_solid_one_is_four_sides() {
        let view = ViewTransform::identity();
        let solid = build_trace_mask_overlay(view, &overlay(vec![square_outline(false)]));
        assert_eq!(solid.triangle_count(), 8);
        let dashed = build_trace_mask_overlay(view, &overlay(vec![square_outline(true)]));
        assert!(dashed.triangle_count() > 8);
    }

    #[test]
    fn a_selected_rectangle_draws_its_dashed_box_and_rounded_handles() {
        let view = ViewTransform::identity();
        let plain = build_trace_mask_overlay(view, &overlay(vec![square_outline(false)]));
        let mut with = overlay(vec![square_outline(false)]);
        with.selection = Some(TraceMaskSelection {
            frame: [
                Point::new(0.0, 0.0),
                Point::new(10.0, 0.0),
                Point::new(10.0, 10.0),
                Point::new(0.0, 10.0),
            ],
            handles: vec![Point::new(0.0, 0.0), Point::new(10.0, 10.0)],
            dragged_handle: Some(1),
            corners: Vec::new(),
            hovered_corner: None,
            selected_corner: None,
        });
        let list = build_trace_mask_overlay(view, &with);
        assert!(list.triangle_count() > plain.triangle_count() + 2 * 4);
    }

    #[test]
    fn a_lasso_in_progress_draws_its_sides_corners_and_rubber_band() {
        let view = ViewTransform::identity();
        let mut lasso = TraceLassoDraft {
            corners: vec![
                Point::new(0.0, 0.0),
                Point::new(40.0, 0.0),
                Point::new(0.0, 40.0),
            ],
            subtract: false,
            pointer: Some(Point::new(30.0, 30.0)),
            closing: false,
        };
        let open = build_trace_mask_overlay(
            view,
            &TraceMaskOverlay {
                lasso: Some(lasso.clone()),
                ..TraceMaskOverlay::default()
            },
        );
        lasso.closing = true;
        let closing = build_trace_mask_overlay(
            view,
            &TraceMaskOverlay {
                lasso: Some(lasso),
                ..TraceMaskOverlay::default()
            },
        );
        assert!(open.triangle_count() > 0);
        assert!(
            closing.triangle_count() > open.triangle_count(),
            "the dashed side and the ring"
        );
    }

    #[test]
    fn a_far_zoom_draws_a_bounded_number_of_dashes() {
        let view = ViewTransform::new(1.0e6, Point::new(0.0, 0.0));
        let list = build_trace_mask_overlay(view, &overlay(vec![square_outline(true)]));
        assert_eq!(list.triangle_count(), 8, "past the cap the sides are solid");
    }

    /// A rectangle whose sides leave a sliver of a dash smaller than the float step at the walk's
    /// position made the dash walk stand still (the page hung when such a rectangle was
    /// selected); every such outline ends.
    #[test]
    fn a_dashed_walk_over_odd_sides_ends() {
        let (w, h) = (272.741_026_798_831_2, 25.684_467_339_110_974);
        let corners = vec![
            Point::new(0.0, 0.0),
            Point::new(w, 0.0),
            Point::new(w, h),
            Point::new(0.0, h),
        ];
        let list = polyline(&corners, true, 0.5, Some((4.0, 3.0)), 0.5, INK);
        assert!(list.triangle_count() > 8, "dashes were drawn");
    }
}
