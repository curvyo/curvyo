//! Whether a primitive draws the line of an Inkscape `d` (`specs/0024-svg-import-export`
//! criterion 111): every point of one outline lies within a distance of the other, sampled at 16
//! points per segment both ways.

use curvyo_document_core::{Angle, PrimitiveOutline, Shape, drawing_outline_of_rotated};
use kurbo::{
    BezPath, ParamCurve, ParamCurveExtrema, ParamCurveNearest, PathSeg, Point, Rect, Shape as _,
};

/// How far the outline of a primitive may be from the drawn `d`, millimetres (criterion 111).
pub(crate) const MATCH_MM: f64 = 1e-3;

/// The samples per segment, ends included (criterion 111: at least 16).
const SAMPLES: u32 = 16;

/// The accuracy of the nearest-point search, millimetres.
const NEAREST_ACCURACY: f64 = 1e-9;

/// How close to the true ellipse the comparison path of a whole ellipse is, millimetres.
const ELLIPSE_ACCURACY_MM: f64 = 1e-6;

/// Whether `shape` turned by `rotation` draws `drawn` within [`MATCH_MM`]. A whole ellipse is
/// compared as the true ellipse (its four-cubic drawing is 2.7e-4 of the radius off it); an arc
/// as its drawing outline (pieces of 22.5°, far closer).
pub(crate) fn draws(shape: &Shape, rotation: Angle, drawn: &BezPath) -> bool {
    let outline = match *shape {
        Shape::Ellipse {
            frame, arc: None, ..
        } => kurbo::Ellipse::new(
            Point::new(frame.center.x, frame.center.y),
            (frame.rx.as_mm(), frame.ry.as_mm()),
            rotation.as_radians(),
        )
        .to_path(ELLIPSE_ACCURACY_MM),
        _ => outline_path(&drawing_outline_of_rotated(shape, rotation)),
    };
    covers(&outline, drawn) && covers(drawn, &outline)
}

/// Every sample of `samples_of` lies within [`MATCH_MM`] of `near`; false when either has no
/// segment.
fn covers(samples_of: &BezPath, near: &BezPath) -> bool {
    let targets: Vec<(PathSeg, Rect)> = near
        .segments()
        .map(|seg| {
            (
                seg,
                ParamCurveExtrema::bounding_box(&seg).inflate(MATCH_MM, MATCH_MM),
            )
        })
        .collect();
    if targets.is_empty() || samples_of.segments().next().is_none() {
        return false;
    }
    let close = |p: Point| {
        targets.iter().any(|(seg, room)| {
            room.contains(p) && seg.nearest(p, NEAREST_ACCURACY).distance_sq <= MATCH_MM * MATCH_MM
        })
    };
    samples_of
        .segments()
        .all(|seg| (0..=SAMPLES).all(|i| close(seg.eval(f64::from(i) / f64::from(SAMPLES)))))
}

/// A primitive's outline as a path: a cubic between two nodes with handles, else a line.
fn outline_path(outline: &PrimitiveOutline) -> BezPath {
    let mut path = BezPath::new();
    let at = |p: curvyo_document_core::Point| Point::new(p.x, p.y);
    let Some(first) = outline.anchors.first() else {
        return path;
    };
    path.move_to(at(first.point));
    let count = outline.anchors.len();
    let segments = if outline.closed { count } else { count - 1 };
    for i in 0..segments {
        let (from, to) = (&outline.anchors[i], &outline.anchors[(i + 1) % count]);
        let straight = from.handle_out.x == 0.0
            && from.handle_out.y == 0.0
            && to.handle_in.x == 0.0
            && to.handle_in.y == 0.0;
        if straight {
            path.line_to(at(to.point));
        } else {
            let c1 = at(from.point) + kurbo::Vec2::new(from.handle_out.x, from.handle_out.y);
            let c2 = at(to.point) + kurbo::Vec2::new(to.handle_in.x, to.handle_in.y);
            path.curve_to(c1, c2, at(to.point));
        }
    }
    if outline.closed {
        path.close_path();
    }
    path
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use curvyo_document_core::{Curve, EllipseFrame, Length, Point as DocPoint};

    fn circle() -> Shape {
        Shape::Ellipse {
            frame: EllipseFrame {
                center: DocPoint::new(0.0, 0.0),
                rx: Length::from_mm(10.0),
                ry: Length::from_mm(10.0),
            },
            arc: None,
            curve: Curve::ELLIPSE_DEFAULT,
        }
    }

    #[test]
    fn ac111_within_and_beyond_the_distance() {
        let drawn = |r: f64| kurbo::Circle::new((0.0, 0.0), r).to_path(1e-7);
        let rotation = Angle::from_radians(0.0);
        assert!(draws(&circle(), rotation, &drawn(10.0005)));
        assert!(!draws(&circle(), rotation, &drawn(10.002)));
        let mut half = BezPath::new();
        half.move_to((10.0, 0.0));
        half.line_to((-10.0, 0.0));
        assert!(!draws(&circle(), rotation, &half), "both ways");
        assert!(!draws(&circle(), rotation, &BezPath::new()));
    }
}
