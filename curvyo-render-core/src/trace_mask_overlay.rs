//! Draws the regions of a trace mask over the picture in the Trace tool
//! (`specs/0114-image-trace-mask` Behaviour 2 and 4): each region's outline in black at 0.25 mm
//! and at least 1 px, a subtracted one dashed, the selected one in the accent colour with its
//! handles, and the rectangle being drawn. Widths are laid out in screen pixels and converted to
//! document millimetres, like every other decoration.

use curvyo_document_core::{Point, ViewTransform};

use crate::color::RgbaColor;
use crate::glyphs::{DrawList, dashed_line, square, thick_line};
use crate::theme;

/// The outline width of a region in document millimetres, before the 1 px floor.
const OUTLINE_MM: f64 = 0.25;
/// A subtracted region's dashes and gaps, screen pixels.
const DASH_PX: (f64, f64) = (4.0, 3.0);
/// Above this many dashes on one side the side is drawn solid (a frame must stay bounded).
const MAX_DASHES: f64 = 4096.0;
/// The selected region's outline, screen pixels.
const SELECTED_PX: f64 = 1.5;
/// The outline colour.
const INK: RgbaColor = RgbaColor::opaque(0, 0, 0);

/// One region's closed outline in document millimetres.
#[derive(Debug, Clone, PartialEq)]
pub struct TraceMaskOutline {
    /// The corners, closed from the last to the first.
    pub corners: Vec<Point>,
    /// Drawn with Subtract: dashed.
    pub subtract: bool,
    /// The selected region: accent colour.
    pub selected: bool,
}

/// What the mask overlay draws this frame.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TraceMaskOverlay {
    /// The active part's regions in order.
    pub outlines: Vec<TraceMaskOutline>,
    /// The selected region's handles, document millimetres.
    pub handles: Vec<Point>,
}

/// The overlay as a draw list, to be drawn above the preview outline.
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
        let (ink, side_width) = if outline.selected {
            (theme::ACCENT, (SELECTED_PX * px).max(width))
        } else {
            (INK, width)
        };
        let corners = &outline.corners;
        for (i, &a) in corners.iter().enumerate() {
            let b = corners[(i + 1) % corners.len()];
            let dashes = a.vector_to(b).length() * scale / (DASH_PX.0 + DASH_PX.1);
            if outline.subtract && dashes <= MAX_DASHES {
                list.extend(dashed_line(
                    a,
                    b,
                    side_width,
                    (DASH_PX.0 * px, DASH_PX.1 * px),
                    ink,
                ));
            } else {
                list.extend(thick_line(a, b, side_width, ink));
            }
        }
    }
    let size = theme::SHAPE_HANDLE_SIZE_PX * px;
    for &handle in &overlay.handles {
        list.extend(square(handle, size, theme::ACCENT));
        list.extend(square(
            handle,
            size - 2.0 * px,
            RgbaColor::opaque(255, 255, 255),
        ));
    }
    list
}

#[cfg(test)]
mod tests {
    use super::*;

    fn outline(subtract: bool, selected: bool) -> TraceMaskOutline {
        TraceMaskOutline {
            corners: vec![
                Point::new(0.0, 0.0),
                Point::new(10.0, 0.0),
                Point::new(10.0, 10.0),
                Point::new(0.0, 10.0),
            ],
            subtract,
            selected,
        }
    }

    #[test]
    fn a_subtracted_outline_is_dashed_and_a_solid_one_is_four_sides() {
        let view = ViewTransform::identity();
        let solid = build_trace_mask_overlay(
            view,
            &TraceMaskOverlay {
                outlines: vec![outline(false, false)],
                handles: Vec::new(),
            },
        );
        assert_eq!(solid.triangle_count(), 8);
        let dashed = build_trace_mask_overlay(
            view,
            &TraceMaskOverlay {
                outlines: vec![outline(true, false)],
                handles: Vec::new(),
            },
        );
        assert!(dashed.triangle_count() > 8);
    }

    #[test]
    fn a_selected_region_draws_its_handles() {
        let view = ViewTransform::identity();
        let list = build_trace_mask_overlay(
            view,
            &TraceMaskOverlay {
                outlines: vec![outline(false, true)],
                handles: vec![Point::new(0.0, 0.0), Point::new(10.0, 10.0)],
            },
        );
        assert_eq!(list.triangle_count(), 8 + 2 * 4);
    }

    #[test]
    fn a_far_zoom_draws_a_bounded_number_of_dashes() {
        let view = ViewTransform::new(1.0e6, Point::new(0.0, 0.0));
        let list = build_trace_mask_overlay(
            view,
            &TraceMaskOverlay {
                outlines: vec![outline(true, false)],
                handles: Vec::new(),
            },
        );
        assert_eq!(list.triangle_count(), 8, "past the cap the sides are solid");
    }
}
