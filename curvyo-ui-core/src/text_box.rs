//! The line box of a laid-out text in the document (`specs/0074-text-objects` criterion 25 and
//! the UX notes' "Text selection box"): its corners, bounds, oriented box and hit.

use curvyo_document_core::{Angle, Point, TextSnapshot};

use crate::oriented_box::OrientedBox;

/// The four corners of the text's line box in the document; a text not laid out yet is the
/// point of its anchor.
#[must_use]
pub fn line_box_corners(text: &TextSnapshot) -> [Point; 4] {
    text.outline.as_ref().map_or_else(
        || [Point::new(text.placement.e, text.placement.f); 4],
        |outline| outline.line_box,
    )
}

/// The axis-aligned bounds of the line box.
#[must_use]
pub fn line_box_bounds(text: &TextSnapshot) -> (Point, Point) {
    let corners = line_box_corners(text);
    corners.iter().fold(
        (
            Point::new(f64::INFINITY, f64::INFINITY),
            Point::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
        ),
        |(min, max), p| {
            (
                Point::new(min.x.min(p.x), min.y.min(p.y)),
                Point::new(max.x.max(p.x), max.y.max(p.y)),
            )
        },
    )
}

/// The line box as an oriented box turned by the baseline's angle about its first corner; a text
/// on path has the axis-parallel box of its letters (`specs/0080-text-on-path` criterion 28).
#[must_use]
pub fn line_box_oriented(text: &TextSnapshot) -> OrientedBox {
    let corners = line_box_corners(text);
    let angle = if on_path_boxes(text).is_some() {
        Angle::from_radians(0.0)
    } else {
        text.rotation()
    };
    let pivot = corners[0];
    let back = Angle::from_radians(-angle.as_radians());
    let local: Vec<Point> = corners
        .iter()
        .map(|p| p.rotated_around(pivot, back))
        .collect();
    let min = local
        .iter()
        .fold(Point::new(f64::INFINITY, f64::INFINITY), |m, p| {
            Point::new(m.x.min(p.x), m.y.min(p.y))
        });
    let max = local
        .iter()
        .fold(Point::new(f64::NEG_INFINITY, f64::NEG_INFINITY), |m, p| {
            Point::new(m.x.max(p.x), m.y.max(p.y))
        });
    OrientedBox {
        min,
        max,
        angle,
        pivot,
    }
}

/// The turned letter boxes of a text on path, or `None` for a straight text.
#[must_use]
pub fn on_path_boxes(text: &TextSnapshot) -> Option<&[[Point; 4]]> {
    text.outline
        .as_ref()
        .map(|outline| outline.letter_boxes.as_slice())
        .filter(|boxes| !boxes.is_empty())
}

/// Whether `point` lies inside the line box (a parallelogram once skewed), or within `margin` of
/// it: a text is hit anywhere inside the box of its lines (criterion 25); a text on path inside
/// any of its turned letter boxes (`specs/0080-text-on-path` criterion 28).
#[must_use]
pub fn line_box_contains(text: &TextSnapshot, point: Point, margin: f64) -> bool {
    if let Some(boxes) = on_path_boxes(text) {
        return boxes
            .iter()
            .any(|corners| quad_contains(corners, point, margin));
    }
    quad_contains(&line_box_corners(text), point, margin)
}

/// Whether `point` lies inside the quad `corners` or within `margin` of it.
pub(crate) fn quad_contains(corners: &[Point; 4], point: Point, margin: f64) -> bool {
    let corners = *corners;
    let mut sign = 0.0_f64;
    let mut inside = true;
    for i in 0..4 {
        let (a, b) = (corners[i], corners[(i + 1) % 4]);
        let edge = a.vector_to(b);
        let to_point = a.vector_to(point);
        let cross = edge.x.mul_add(to_point.y, -(edge.y * to_point.x));
        let length = edge.length();
        let distance = if length > 0.0 { cross / length } else { 0.0 };
        if sign == 0.0 && distance.abs() > margin {
            sign = distance.signum();
        }
        if sign != 0.0 && distance * sign < -margin {
            inside = false;
        }
    }
    inside
}
