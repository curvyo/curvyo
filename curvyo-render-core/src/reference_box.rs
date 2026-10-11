//! The true bounding box of a move and the marks of its nine reference points
//! (`specs/0047-transform-polish/` Part C, criteria 33 to 36 and 39; `docs/design-system.md`,
//! rows "Reference box" and "Reference mark"): a pale red 4 px line over a white casing,
//! pixel-aligned like the selection box, clipped to the viewport; and, in the typed move's
//! Absolute mode, a hollow square at each corner and edge midpoint and a cross at the centre,
//! the chosen one filled. Which marks there are, and which is current or hovered, is the
//! caller's: this only draws what it is given.

use curvyo_document_core::{Point, ViewTransform};

use crate::glyphs::{DrawList, thick_line};
use crate::select_box::{ScreenBox, effective_ratio, snap_to_device};
use crate::theme;

/// One mark to draw.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReferenceMark {
    /// Where it lies, in document space.
    pub at: Point,
    /// A cross (the centre) instead of a square.
    pub cross: bool,
    /// The chosen reference: filled instead of hollow.
    pub current: bool,
    /// The pointer or the keyboard focus is on it: larger by 2 px.
    pub hovered: bool,
}

/// What the reference box decorates this frame.
#[derive(Debug, Clone)]
pub struct ReferenceBoxInput {
    /// The box, `(min, max)` in document space.
    pub bounds: (Point, Point),
    /// The marks to draw: none in Relative mode, below 32 px, or outside the viewport.
    pub marks: Vec<ReferenceMark>,
    /// The canvas size in screen pixels, for clipping the box line; `(0, 0)` clips nothing.
    pub canvas_px: (f64, f64),
    /// `window.devicePixelRatio`; not a positive finite number reads as 1.
    pub device_pixel_ratio: f64,
}

type Screen = (f64, f64);

/// `from` to `to` (an axis-aligned edge in screen pixels) cut to the viewport with a line's
/// width of margin, `None` when it lies entirely outside.
fn clip(from: Screen, to: Screen, canvas_px: (f64, f64), margin: f64) -> Option<(Screen, Screen)> {
    let (width, height) = canvas_px;
    if width <= 0.0 || height <= 0.0 {
        return Some((from, to));
    }
    let (low, high) = ((-margin, -margin), (width + margin, height + margin));
    let horizontal = (from.1 - to.1).abs() <= f64::EPSILON * 8.0;
    if horizontal {
        if from.1 < low.1 || from.1 > high.1 {
            return None;
        }
        let (a, b) = (from.0.min(to.0).max(low.0), from.0.max(to.0).min(high.0));
        return (a <= b).then_some(((a, from.1), (b, from.1)));
    }
    if from.0 < low.0 || from.0 > high.0 {
        return None;
    }
    let (a, b) = (from.1.min(to.1).max(low.1), from.1.max(to.1).min(high.1));
    (a <= b).then_some(((from.0, a), (from.0, b)))
}

/// The pale red box: casing then line, below the blue outlines and the selection box.
#[must_use]
pub(crate) fn build_box(view: ViewTransform, input: &ReferenceBoxInput) -> DrawList {
    let ratio = effective_ratio(input.device_pixel_ratio);
    let (min, max) = input.bounds;
    let corners = [min, Point::new(max.x, min.y), max, Point::new(min.x, max.y)];
    let screen = ScreenBox::new(view, corners, ratio);
    let at = |(x, y): Screen| view.screen_to_document(x, y);
    // A box with no height or no width is one line: the opposite edge would draw it twice.
    let flat_y = (screen.corners[2].1 - screen.corners[0].1).abs() < 0.5;
    let flat_x = (screen.corners[2].0 - screen.corners[0].0).abs() < 0.5;
    let edges: Vec<usize> = match (flat_x, flat_y) {
        (true, true) => vec![],
        (false, true) => vec![0],
        (true, false) => vec![3],
        (false, false) => vec![0, 1, 2, 3],
    };
    // The line is `REFERENCE_BOX_LINE_PX` device pixels wide, so that it shows on both sides of
    // the blue outline that coincides with it (the move preview, the selection box); the casing
    // adds one device pixel on each side.
    let device_line = (theme::REFERENCE_BOX_LINE_PX * ratio).round().max(2.0);
    let line = device_line / ratio;
    // An axis-aligned box is snapped again for this line: its centre on a pixel boundary for an
    // even width in device pixels, in a pixel's middle for an odd one, so both edges are crisp.
    let axis_aligned = (0..4).all(|i| {
        let (a, b) = (screen.corners[i], screen.corners[(i + 1) % 4]);
        (b.0 - a.0).abs() <= 1e-6 || (b.1 - a.1).abs() <= 1e-6
    });
    let corners_px = screen.corners.map(|(x, y)| {
        if axis_aligned {
            (
                snap_to_device(x, ratio, device_line),
                snap_to_device(y, ratio, device_line),
            )
        } else {
            (x, y)
        }
    });
    let screen = ScreenBox {
        corners: corners_px,
        width_px: screen.width_px,
    };
    let mut list = DrawList::default();
    for (color, width) in [
        (theme::REFERENCE_BOX_CASING, line + 2.0 / ratio),
        (theme::REFERENCE_BOX, line),
    ] {
        // Reaches half a line past each corner, so two edges cover the corner pixel.
        let half = width / 2.0;
        for &i in &edges {
            let (from, to) = (screen.corners[i], screen.corners[(i + 1) % 4]);
            let length = (to.0 - from.0).hypot(to.1 - from.1);
            if length <= f64::EPSILON {
                continue;
            }
            let (ux, uy) = ((to.0 - from.0) / length, (to.1 - from.1) / length);
            let (a, b) = (
                (from.0 - ux * half, from.1 - uy * half),
                (to.0 + ux * half, to.1 + uy * half),
            );
            if let Some((a, b)) = clip(a, b, input.canvas_px, width) {
                list.extend(thick_line(at(a), at(b), width / view.scale(), color));
            }
        }
    }
    list
}

/// A filled axis-aligned rectangle `size_px` = (width, height) across, centred at `at`
/// (document space): a thick line along x.
fn bar(
    view: ViewTransform,
    at: Point,
    size_px: (f64, f64),
    color: crate::color::RgbaColor,
) -> DrawList {
    let half_w = size_px.0 / 2.0 / view.scale();
    thick_line(
        Point::new(at.x - half_w, at.y),
        Point::new(at.x + half_w, at.y),
        size_px.1 / view.scale(),
        color,
    )
}

/// One mark: casing, red shape, and for a hollow one a white inside.
fn mark(view: ViewTransform, mark: &ReferenceMark) -> DrawList {
    let grow = if mark.hovered {
        theme::REFERENCE_MARK_HOVER_GROWTH_PX
    } else {
        0.0
    };
    let line = theme::REFERENCE_MARK_LINE_PX;
    let casing = theme::SELECTION_CASING;
    let red = theme::REFERENCE_MARK;
    // The shapes as (across, thickness) of each bar: a square is one bar, a cross two.
    let shapes: Vec<(f64, f64)> = if mark.cross {
        let across = theme::REFERENCE_CROSS_PX + grow;
        let arm = theme::REFERENCE_CROSS_ARM_PX;
        vec![(across, arm), (arm, across)]
    } else {
        let side = theme::REFERENCE_MARK_SQUARE_PX + grow;
        vec![(side, side)]
    };
    let mut list = DrawList::default();
    for (w, h) in &shapes {
        list.extend(bar(view, mark.at, (w + 2.0 * line, h + 2.0 * line), casing));
    }
    for (w, h) in &shapes {
        list.extend(bar(view, mark.at, (*w, *h), red));
    }
    if !mark.current {
        for (w, h) in &shapes {
            // The outline is `line` wide: the inside of each bar is white. For a cross the two
            // inner bars overlap in the middle, which is white as well.
            list.extend(bar(view, mark.at, (w - 2.0 * line, h - 2.0 * line), casing));
        }
        if mark.cross {
            // The red of the vertical bar's outline would show inside the horizontal bar; cover
            // the middle square with white.
            let arm = theme::REFERENCE_CROSS_ARM_PX - 2.0 * line;
            list.extend(bar(view, mark.at, (arm, arm), casing));
        }
    }
    list
}

/// The marks, above the artwork, the box and the handles.
#[must_use]
pub(crate) fn build_marks(view: ViewTransform, input: &ReferenceBoxInput) -> DrawList {
    let ratio = effective_ratio(input.device_pixel_ratio);
    // The glyphs are an even number of pixels across, so a centre on a whole device pixel keeps
    // their 1 px outline crisp (an edge midpoint of an odd-width box would be on a half pixel).
    // The hit target keeps the exact point.
    let snapped = |at: Point| {
        let (x, y) = view.document_to_screen(at);
        view.screen_to_document((x * ratio).round() / ratio, (y * ratio).round() / ratio)
    };
    let mut list = DrawList::default();
    // The chosen mark last, so its fill is never covered by a neighbour's casing.
    for current in [false, true] {
        for item in input.marks.iter().filter(|m| m.current == current) {
            let item = ReferenceMark {
                at: snapped(item.at),
                ..*item
            };
            list.extend(mark(view, &item));
        }
    }
    list
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view() -> ViewTransform {
        ViewTransform::new(1.0, Point::new(0.0, 0.0))
    }

    fn input(bounds: (Point, Point), marks: Vec<ReferenceMark>) -> ReferenceBoxInput {
        ReferenceBoxInput {
            bounds,
            marks,
            canvas_px: (800.0, 600.0),
            device_pixel_ratio: 1.0,
        }
    }

    fn bounds_of(list: &DrawList) -> (f64, f64, f64, f64) {
        let xs = list.triangles.iter().map(|v| v.position.x);
        let ys = list.triangles.iter().map(|v| v.position.y);
        (
            xs.clone().fold(f64::INFINITY, f64::min),
            ys.clone().fold(f64::INFINITY, f64::min),
            xs.fold(f64::NEG_INFINITY, f64::max),
            ys.fold(f64::NEG_INFINITY, f64::max),
        )
    }

    /// Criterion 36: the box is drawn in the pale red over a white casing at 55 %, nothing
    /// else, and a box with area has four edges of casing and four of line.
    #[test]
    fn the_box_is_a_pale_red_line_over_a_white_casing() {
        let list = build_box(
            view(),
            &input((Point::new(100.0, 100.0), Point::new(300.0, 250.0)), vec![]),
        );
        assert_ne!(list.triangles.len(), 0);
        let reds = list
            .triangles
            .iter()
            .filter(|v| v.color == theme::REFERENCE_BOX)
            .count();
        let whites = list
            .triangles
            .iter()
            .filter(|v| v.color == theme::REFERENCE_BOX_CASING)
            .count();
        assert_eq!(reds, whites, "one casing under every line");
        assert!(reds > 0);
        assert!(
            list.triangles
                .iter()
                .all(|v| v.color == theme::REFERENCE_BOX || v.color == theme::REFERENCE_BOX_CASING)
        );
    }

    /// Criterion 36: the line is clipped to the viewport: a box far larger than the canvas has
    /// no vertex beyond a line width outside it.
    #[test]
    fn the_box_is_clipped_to_the_viewport() {
        let list = build_box(
            view(),
            &input(
                (Point::new(-1.0e6, -1.0e6), Point::new(1.0e6, 1.0e6)),
                vec![],
            ),
        );
        assert!(list.triangles.is_empty(), "every edge is outside");
        let list = build_box(
            view(),
            &input(
                (Point::new(-1.0e6, 100.0), Point::new(1.0e6, 1.0e6)),
                vec![],
            ),
        );
        let (x0, _, x1, _) = bounds_of(&list);
        assert!(x0 >= -6.0 && x1 <= 806.0, "{x0} {x1}");
    }

    /// A box with no height is one line, drawn once.
    #[test]
    fn a_flat_box_is_one_line() {
        let flat = build_box(
            view(),
            &input((Point::new(100.0, 100.0), Point::new(300.0, 100.0)), vec![]),
        );
        let full = build_box(
            view(),
            &input((Point::new(100.0, 100.0), Point::new(300.0, 250.0)), vec![]),
        );
        assert!(flat.triangles.len() * 3 < full.triangles.len());
    }

    /// Criterion 39: a hollow square, a filled square, a cross, and a hovered one that is 2 px
    /// larger.
    #[test]
    fn marks_are_hollow_filled_crossed_and_grow_when_hovered() {
        let one = |current: bool, cross: bool, hovered: bool| {
            build_marks(
                view(),
                &input(
                    (Point::new(0.0, 0.0), Point::new(100.0, 100.0)),
                    vec![ReferenceMark {
                        at: Point::new(50.0, 50.0),
                        cross,
                        current,
                        hovered,
                    }],
                ),
            )
        };
        let hollow = one(false, false, false);
        let filled = one(true, false, false);
        let (x0, y0, x1, y1) = bounds_of(&hollow);
        assert!(((x1 - x0) - 12.0).abs() < 1e-3 && ((y1 - y0) - 12.0).abs() < 1e-3);
        assert!(
            hollow.triangles.len() > filled.triangles.len(),
            "white inside"
        );
        assert!(
            filled
                .triangles
                .iter()
                .any(|v| v.color == theme::REFERENCE_MARK)
        );
        let (x0, _, x1, _) = bounds_of(&one(false, false, true));
        assert!(((x1 - x0) - 14.0).abs() < 1e-3, "2 px larger");
        let (x0, y0, x1, y1) = bounds_of(&one(true, true, false));
        assert!(((x1 - x0) - 14.0).abs() < 1e-3 && ((y1 - y0) - 14.0).abs() < 1e-3);
    }

    /// The line is 4 device pixels wide and the casing 6.
    #[test]
    fn the_line_is_four_pixels_wide() {
        let list = build_box(
            view(),
            &input((Point::new(100.0, 100.0), Point::new(300.0, 250.0)), vec![]),
        );
        let xs: Vec<f64> = list
            .triangles
            .iter()
            .filter(|v| v.color == theme::REFERENCE_BOX)
            .map(|v| v.position.x)
            .collect();
        let x0 = xs.iter().copied().fold(f64::INFINITY, f64::min);
        let x1 = xs.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        // The left and right lines reach 2 px out on each side of the box (100.5 and 300.5
        // after pixel snapping, so within half a pixel of 98 and 302).
        assert!(
            (x0 - 98.0).abs() <= 1.0 && (x1 - 302.0).abs() <= 1.0,
            "{x0} {x1}"
        );
    }

    /// A mark's centre is on a whole pixel, so its outline stays crisp on an odd-width box.
    #[test]
    fn a_mark_is_snapped_to_a_whole_pixel() {
        let list = build_marks(
            view(),
            &input(
                (Point::new(0.0, 0.0), Point::new(125.0, 100.0)),
                vec![ReferenceMark {
                    at: Point::new(62.5, 0.0),
                    cross: false,
                    current: false,
                    hovered: false,
                }],
            ),
        );
        let (x0, _, x1, _) = bounds_of(&list);
        assert!(
            (x0.fract()).abs() < 1e-6 && (x1.fract()).abs() < 1e-6,
            "{x0} {x1}"
        );
    }
}
