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

/// How one local axis is drawn: as the faint frame, hovered, or selected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SketchAxisState {
    /// The faint frame line.
    Idle,
    /// Under the pointer as a pick target: the geometry hover band behind the frame line.
    Hovered,
    /// Picked, selected or inferred: the accent line selected geometry uses.
    Selected,
}

/// Draws the immutable local frame, X axis then Y axis, each in its `states` emphasis.
#[must_use]
pub fn build_sketch_axes(
    placement: curvyo_document_core::Affine,
    half_size: curvyo_document_core::Vec2,
    states: [SketchAxisState; 2],
    view: ViewTransform,
) -> DrawList {
    let mut list = DrawList::default();
    let ends = [
        (Point::new(-half_size.x, 0.0), Point::new(half_size.x, 0.0)),
        (Point::new(0.0, -half_size.y), Point::new(0.0, half_size.y)),
    ];
    for ((a, b), state) in ends.into_iter().zip(states) {
        let (a, b) = (placement.apply_point(a), placement.apply_point(b));
        let px = 1.0 / view.scale();
        let lines: &[(f64, crate::color::RgbaColor)] = match state {
            SketchAxisState::Idle => &[(0.5, theme::AXIS_GUIDE)],
            SketchAxisState::Hovered => &[
                (theme::SEGMENT_HOVER_WIDTH_PX, theme::SEGMENT_HOVER),
                (0.5, theme::AXIS_GUIDE),
            ],
            SketchAxisState::Selected => &[(1.5, theme::ACCENT)],
        };
        for (width, color) in lines {
            list.extend(glyphs::thick_line(a, b, width * px, *color));
        }
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

    /// Revision 5 polish: a hovered axis gets the geometry hover band, a picked or selected
    /// axis the selection accent; the other axis stays the faint frame line.
    #[test]
    fn a_hovered_or_selected_axis_draws_in_the_geometry_highlight() {
        let view = ViewTransform::new(2.0, Point::new(0.0, 0.0));
        let half = curvyo_document_core::Vec2::new(50.0, 40.0);
        let draw =
            |states| build_sketch_axes(curvyo_document_core::Affine::IDENTITY, half, states, view);
        let colors = |list: &DrawList| {
            let mut colors = Vec::new();
            for v in &list.triangles {
                if !colors.contains(&v.color) {
                    colors.push(v.color);
                }
            }
            colors
        };
        let idle = draw([SketchAxisState::Idle, SketchAxisState::Idle]);
        assert_eq!(colors(&idle), [theme::AXIS_GUIDE]);
        // The hover band is the 4px segment hover band (`--segment-hover`), X axis only.
        let hovered = draw([SketchAxisState::Hovered, SketchAxisState::Idle]);
        let reach = |list: &DrawList, color, across: fn(&Point) -> f64| {
            list.triangles
                .iter()
                .filter(|v| v.color == color && across(&v.position) < 10.0)
                .map(|v| across(&v.position) * 2.0)
                .fold(0.0, f64::max)
        };
        let across_x: fn(&Point) -> f64 = |p| p.y.abs();
        let across_y: fn(&Point) -> f64 = |p| p.x.abs();
        assert!((reach(&hovered, theme::SEGMENT_HOVER, across_x) - 2.0).abs() < 1e-9);
        assert!((reach(&idle, theme::AXIS_GUIDE, across_x) - 0.25).abs() < 1e-9);
        assert!((reach(&hovered, theme::AXIS_GUIDE, across_y) - 0.25).abs() < 1e-9);
        let selected = draw([SketchAxisState::Idle, SketchAxisState::Selected]);
        let accent = selected
            .triangles
            .iter()
            .filter(|v| v.color == theme::ACCENT)
            .collect::<Vec<_>>();
        assert_eq!(accent.len(), 6, "one accent quad along the Y axis");
        assert!(
            accent
                .iter()
                .all(|v| (v.position.x.abs() * 2.0 - 0.75).abs() < 1e-9)
        );
        assert!(
            colors(&selected).contains(&theme::AXIS_GUIDE),
            "X stays the frame line"
        );
    }
}
