//! Sketch decorations draw analytic editing cues at constant screen size.
use crate::{DrawList, glyphs, stroke, theme};
use curvyo_document_core::{PathSnapshot, Point, ViewTransform};

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
