//! Sketch decorations draw analytic editing cues at constant screen size.
use crate::{DrawList, glyphs, stroke, theme};
use curvyo_document_core::{PathSnapshot, Point, ViewTransform};

/// Width of the solved-geometry stroke (0066 AC32).
const SOLVED_PX: f64 = 1.5;

/// One analytic point or axis handle in document coordinates.
pub struct SketchPointGlyph {
    /// Visible position after sketch placement.
    pub position: Point,
    /// Selected geometry uses the accent fill.
    pub selected: bool,
    /// The fixed/local origin uses a cross as an additional non-colour cue.
    pub origin: bool,
}

/// Draws construction geometry, selected geometry and analytic control points.
#[must_use]
pub fn build_sketch_overlay(
    construction: Option<&PathSnapshot>,
    selected: Option<&PathSnapshot>,
    points: &[SketchPointGlyph],
    view: ViewTransform,
) -> DrawList {
    let mut list = DrawList::default();
    for (path, color) in [(construction, theme::AXIS_GUIDE), (selected, theme::ACCENT)] {
        if let Some(path) = path {
            for outline in path.subpaths() {
                list.extend(stroke::path_stroke(
                    outline.anchors,
                    outline.closed,
                    1.5 / view.scale(),
                    color,
                    0.1 / view.scale(),
                ));
            }
        }
    }
    for point in points {
        let diameter = 8.0 / view.scale();
        list.extend(glyphs::circle(point.position, diameter, theme::NODE_STROKE));
        list.extend(glyphs::circle(
            point.position,
            diameter - 2.0 / view.scale(),
            if point.selected {
                theme::ACCENT
            } else {
                crate::RgbaColor::WHITE
            },
        ));
        if point.origin {
            let extent = 9.0 / view.scale();
            for (dx, dy) in [(extent, 0.0), (0.0, extent)] {
                list.extend(glyphs::thick_line(
                    Point::new(point.position.x - dx, point.position.y - dy),
                    Point::new(point.position.x + dx, point.position.y + dy),
                    1.0 / view.scale(),
                    theme::ACCENT,
                ));
            }
        }
    }
    list
}

/// Draws transient analytic geometry without inventing a document object identity.
#[must_use]
pub fn build_sketch_preview(
    paths: &[curvyo_document_core::SubpathSnapshot],
    view: ViewTransform,
) -> DrawList {
    let mut list = DrawList::default();
    for path in paths {
        list.extend(stroke::path_stroke(
            &path.anchors,
            path.closed,
            1.5 / view.scale(),
            theme::ACCENT,
            0.1 / view.scale(),
        ));
    }
    list
}

/// Draws the endpoint capture ring: an 18px accent ring, 1.5px, on a 1px white casing.
#[must_use]
pub fn build_sketch_capture_marker(center: Point, view: ViewTransform) -> DrawList {
    let px = 1.0 / view.scale();
    let mut list = glyphs::ring(center, 20.0 * px, 3.5 * px, theme::SELECTION_CASING);
    list.extend(glyphs::ring(center, 19.0 * px, 1.5 * px, theme::ACCENT));
    list
}

/// Draws the Ctrl lock guide from `from` to `to`: 1px, dashed 2px on and 2px off.
#[must_use]
pub fn build_sketch_lock_guide(from: Point, to: Point, view: ViewTransform) -> DrawList {
    let px = 1.0 / view.scale();
    glyphs::dashed_line(from, to, px, (2.0 * px, 2.0 * px), theme::SKETCH_LOCK_GUIDE)
}

/// Draws fully constrained sketch geometry in `--sketch-solved` over its artwork stroke.
#[must_use]
pub fn build_sketch_solved(path: &PathSnapshot, view: ViewTransform) -> DrawList {
    let mut list = DrawList::default();
    for outline in path.subpaths() {
        list.extend(stroke::path_stroke(
            outline.anchors,
            outline.closed,
            SOLVED_PX / view.scale(),
            theme::SKETCH_SOLVED,
            0.1 / view.scale(),
        ));
    }
    list
}

/// Draws the immutable local frame with distinct inferred-axis emphasis.
#[must_use]
pub fn build_sketch_axes(
    placement: curvyo_document_core::Affine,
    half_size: curvyo_document_core::Vec2,
    hover: Option<Point>,
    view: ViewTransform,
) -> DrawList {
    let mut list = DrawList::default();
    for (a, b, active) in [
        (
            Point::new(-half_size.x, 0.0),
            Point::new(half_size.x, 0.0),
            hover.is_some_and(|p| p.y.abs() <= 1e-9),
        ),
        (
            Point::new(0.0, -half_size.y),
            Point::new(0.0, half_size.y),
            hover.is_some_and(|p| p.x.abs() <= 1e-9),
        ),
    ] {
        list.extend(glyphs::thick_line(
            placement.apply_point(a),
            placement.apply_point(b),
            if active {
                1.5 / view.scale()
            } else {
                0.5 / view.scale()
            },
            if active {
                theme::ACCENT
            } else {
                theme::AXIS_GUIDE
            },
        ));
    }
    list
}

#[cfg(test)]
mod tests {
    use super::*;

    fn extent(list: &DrawList) -> f64 {
        list.triangles
            .iter()
            .map(|v| v.position.x.abs().max(v.position.y.abs()))
            .fold(0.0, f64::max)
    }

    #[test]
    fn the_capture_ring_keeps_its_screen_size_at_every_zoom() {
        for scale in [0.25, 1.0, 8.0] {
            let view = ViewTransform::new(scale, Point::new(0.0, 0.0));
            let ring = build_sketch_capture_marker(Point::new(0.0, 0.0), view);
            assert!((extent(&ring) * scale - 10.0).abs() < 0.5, "{scale}");
        }
    }

    #[test]
    fn the_lock_guide_is_dashed_two_on_two_off() {
        let view = ViewTransform::new(1.0, Point::new(0.0, 0.0));
        let guide = build_sketch_lock_guide(Point::new(0.0, 0.0), Point::new(40.0, 0.0), view);
        let solid = build_sketch_lock_guide(Point::new(0.0, 0.0), Point::new(2.0, 0.0), view);
        assert_eq!(guide.triangles.len(), 10 * solid.triangles.len());
        let empty = build_sketch_lock_guide(Point::new(1.0, 1.0), Point::new(1.0, 1.0), view);
        assert_eq!(empty.triangles.len(), 0);
        // Darker than the faint move axis guide: the dashes read beyond the rubber band
        // (UX notes revision 2 review, change 4).
        assert!(
            guide
                .triangles
                .iter()
                .all(|v| v.color == theme::SKETCH_LOCK_GUIDE)
        );
    }
}
