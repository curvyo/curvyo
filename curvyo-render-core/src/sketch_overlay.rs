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
