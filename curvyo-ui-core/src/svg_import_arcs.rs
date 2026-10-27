//! An Inkscape arc as a `0021` ellipse arc (`specs/0024-svg-import-export` criteria 112 to 114):
//! the kind, the angles and the frame after the element's transform; the caller checks it
//! against the drawn `d`.

use std::f64::consts::TAU;

use curvyo_document_core::{
    Angle, ArcKind, Curve, EllipseArc, EllipseAxes, EllipseFrame, Length, Linear, Point, Shape,
    arc_after_linear_map, ellipse_after_linear_map,
};
use curvyo_svg_core::ArcData;
use kurbo::Affine;

use crate::svg_import_shapes::{Fit, fit};

/// Angles beyond this many radians are hostile (criterion 124).
const MAX_ANGLE_RAD: f64 = 1e6;

/// Radii beyond this are out of range (criterion 30's coordinate limit).
const MAX_RADIUS_MM: f64 = 1e7;

/// Radii this close (relative) make a circle (criterion 112: 1e-9 mm).
const CIRCLE_TOLERANCE: f64 = 1e-9;

/// The ellipse or ellipse arc of `arc` under `m` (user units to document millimetres) with its
/// rotation, or `None` when the numbers are out of range or the transform stretches a rotated
/// or skewed arc (criterion 114): then it is a path.
#[allow(clippy::many_single_char_names)] // the SVG names of the matrix entries
pub(crate) fn arc_primitive(arc: &ArcData, m: Affine) -> Option<(Shape, Angle)> {
    if !(arc.rx > 0.0 && arc.ry > 0.0) {
        return None;
    }
    let ry = if (arc.rx - arc.ry).abs() <= CIRCLE_TOLERANCE * arc.rx.max(arc.ry) {
        arc.rx
    } else {
        arc.ry
    };
    let angles = [arc.start, arc.end];
    if angles.iter().flatten().any(|a| a.abs() > MAX_ANGLE_RAD) {
        return None;
    }
    let kind = match arc.arc_type.as_deref() {
        Some("slice") => ArcKind::Pie,
        Some("arc") => ArcKind::Arc,
        Some("chord") => ArcKind::Chord,
        Some(_) => return None,
        None if arc.open => ArcKind::Arc,
        None => ArcKind::Pie,
    };
    let local = match angles {
        [None, None] => None,
        [start, end] => {
            let start = start.unwrap_or(0.0);
            let mut end = end.unwrap_or(TAU);
            if end <= start {
                end += ((start - end) / TAU).floor().mul_add(TAU, TAU);
            }
            EllipseArc::new(Angle::from_radians(start), Angle::from_radians(end), kind)
        }
    };
    let [a, b, c, d, _, _] = m.as_coeffs();
    let linear: Linear = [[a, c], [b, d]];
    let before = EllipseAxes {
        rx: arc.rx,
        ry,
        rotation: 0.0,
    };
    let after = match fit(m) {
        Fit::Similar(scale, turn) => EllipseAxes {
            rx: arc.rx * scale,
            ry: ry * scale,
            rotation: turn,
        },
        Fit::Aligned(..) => ellipse_after_linear_map(before, linear),
        Fit::Other => return None,
    };
    if after.rx.max(after.ry) > MAX_RADIUS_MM {
        return None;
    }
    let center = m * arc.center;
    let shape = Shape::Ellipse {
        frame: EllipseFrame {
            center: Point::new(center.x, center.y),
            rx: Length::from_mm(after.rx),
            ry: Length::from_mm(after.ry),
        },
        arc: local.map(|local| arc_after_linear_map(local, before, after, linear)),
        curve: Curve::ELLIPSE_DEFAULT,
    };
    Some((shape, Angle::from_radians(after.rotation)))
}
