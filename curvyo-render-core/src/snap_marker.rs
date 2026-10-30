//! Constant-screen-width geometry for snapping feedback.
use crate::{DrawList, stroke, theme};
use curvyo_document_core::{AnchorId, NewAnchor, Point, Vec2, ViewTransform};

/// One finite snapping marker segment in document coordinates.
#[derive(Debug, Clone, Copy)]
pub struct SnapMarkerSegment {
    /// Start position.
    pub start: Point,
    /// Relative outgoing handle.
    pub handle_out: Vec2,
    /// Relative incoming handle.
    pub handle_in: Vec2,
    /// End position.
    pub end: Point,
}
/// Draws target geometry with a 12px cross and white casing.
#[must_use]
pub fn build_snap_marker(
    segments: &[SnapMarkerSegment],
    cross: Option<Point>,
    view: ViewTransform,
) -> DrawList {
    let mut curves: Vec<_> = segments
        .iter()
        .map(|s| {
            let mut a = NewAnchor::corner(AnchorId::new(0, 0), s.start);
            let mut b = NewAnchor::corner(AnchorId::new(0, 1), s.end);
            a.handle_out = s.handle_out;
            b.handle_in = s.handle_in;
            [a, b]
        })
        .collect();
    if let Some(p) = cross {
        let radius = 6.0 / view.scale();
        for d in [Vec2::new(radius, 0.0), Vec2::new(0.0, radius)] {
            curves.push([
                NewAnchor::corner(AnchorId::new(0, 0), Point::new(p.x - d.x, p.y - d.y)),
                NewAnchor::corner(AnchorId::new(0, 1), p.translated(d)),
            ]);
        }
    }
    let mut result = DrawList::default();
    for (width, color) in [(3.0, theme::SELECTION_CASING), (1.0, theme::REFERENCE_MARK)] {
        for curve in &curves {
            result.extend(stroke::path_stroke(
                curve,
                false,
                width / view.scale(),
                color,
                theme::DISPLAY_TOLERANCE_PX / view.scale(),
            ));
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn marker_strokes_keep_one_pixel_and_three_pixel_casing_at_every_zoom() {
        for scale in [0.1, 1.0, 25.0] {
            let marker = build_snap_marker(
                &[SnapMarkerSegment {
                    start: Point::new(0.0, 0.0),
                    end: Point::new(100.0, 0.0),
                    handle_out: Vec2::ZERO,
                    handle_in: Vec2::ZERO,
                }],
                None,
                ViewTransform::new(scale, Point::new(0.0, 0.0)),
            );
            for (color, width) in [(theme::REFERENCE_MARK, 1.0), (theme::SELECTION_CASING, 3.0)] {
                let ys: Vec<_> = marker
                    .triangles
                    .iter()
                    .filter(|v| v.color == color)
                    .map(|v| v.position.y)
                    .collect();
                let min = ys.iter().copied().fold(f64::INFINITY, f64::min);
                let max = ys.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                assert!(((max - min) * scale - width).abs() < 1e-5);
            }
        }
    }
}
