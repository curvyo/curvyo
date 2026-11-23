//! The drawn lines of a registration mark (ADR 0081 §1): canvas, SVG, PNG, hit and bounds read
//! these, so the print and the screen agree.
//!
//! A circle of the diameter, a crosshair of the diameter plus [`MARK_CROSS_EXTRA_MM`], and the
//! number as a stroke digit half the diameter high from a built-in table (no font), its box's
//! lower-left corner half a millimetre outside the circle's square, up and right of the centre
//! (UX notes P5: 3.5 mm at a 6 mm mark). Always upright, never scaled by a transform.

use crate::path_model::{AnchorId, AnchorKind, NewAnchor, PathSnapshot, SubpathSnapshot};
use crate::registration_mark_model::{MARK_CROSS_EXTRA_MM, RegistrationMarkSnapshot};
use crate::units::{Angle, Length, Point, Vec2};

/// The gap between the circle's square and the digit's box, in mm.
const DIGIT_GAP_MM: f64 = 0.5;
/// The digit's width as a share of its height.
const DIGIT_ASPECT: f64 = 0.6;

/// The strokes of each digit in a unit box (x right, y down, 0 at the top-left).
const DIGITS: [&[(f64, f64)]; 4] = [
    &[(0.2, 0.2), (0.55, 0.0), (0.55, 1.0)],
    &[
        (0.0, 0.25),
        (0.25, 0.0),
        (0.75, 0.0),
        (1.0, 0.25),
        (1.0, 0.45),
        (0.0, 1.0),
        (1.0, 1.0),
    ],
    &[
        (0.0, 0.0),
        (1.0, 0.0),
        (0.4, 0.42),
        (0.75, 0.42),
        (1.0, 0.7),
        (0.75, 1.0),
        (0.0, 1.0),
    ],
    &[(0.75, 1.0), (0.75, 0.0), (0.0, 0.7), (1.0, 0.7)],
];

/// A mark's lines in document mm, all stroked at [`MarkLines::line`], black, no fill.
#[derive(Debug, Clone, PartialEq)]
pub struct MarkLines {
    /// The crosshair centre.
    pub centre: Point,
    /// The circle's radius.
    pub radius: Length,
    /// The two crosshair arms, horizontal then vertical.
    pub cross: [[Point; 2]; 2],
    /// The digit's polylines (one for each digit of the table).
    pub digit: Vec<Vec<Point>>,
    /// The line width.
    pub line: Length,
}

impl MarkLines {
    /// The box of every line with half the line width around it: `(min, max)`.
    #[must_use]
    pub fn bounds(&self) -> (Point, Point) {
        let half = self.line.as_mm() / 2.0;
        let r = self.radius.as_mm().max(self.cross_half());
        let (mut min, mut max) = (
            Point::new(self.centre.x - r, self.centre.y - r),
            Point::new(self.centre.x + r, self.centre.y + r),
        );
        for p in self.digit.iter().flatten() {
            min = Point::new(min.x.min(p.x), min.y.min(p.y));
            max = Point::new(max.x.max(p.x), max.y.max(p.y));
        }
        (
            Point::new(min.x - half, min.y - half),
            Point::new(max.x + half, max.y + half),
        )
    }

    fn cross_half(&self) -> f64 {
        self.radius.as_mm() + MARK_CROSS_EXTRA_MM / 2.0
    }
}

/// The lines `mark` draws.
#[must_use]
pub fn mark_lines(mark: &RegistrationMarkSnapshot) -> MarkLines {
    let (c, r) = (mark.centre, mark.diameter.as_mm() / 2.0);
    let arm = r + MARK_CROSS_EXTRA_MM / 2.0;
    let height = r;
    let width = height * DIGIT_ASPECT;
    let (left, bottom) = (c.x + r + DIGIT_GAP_MM, c.y - r - DIGIT_GAP_MM);
    let top = bottom - height;
    let digit = DIGITS
        .get(usize::from(mark.number.saturating_sub(1)))
        .map(|strokes| {
            vec![
                strokes
                    .iter()
                    .map(|&(x, y)| Point::new(left + x * width, top + y * height))
                    .collect(),
            ]
        })
        .unwrap_or_default();
    MarkLines {
        centre: c,
        radius: Length::from_mm(r),
        cross: [
            [Point::new(c.x - arm, c.y), Point::new(c.x + arm, c.y)],
            [Point::new(c.x, c.y - arm), Point::new(c.x, c.y + arm)],
        ],
        digit,
        line: mark.line,
    }
}

/// The circle's handle length as a share of its radius (four cubic quarters).
const KAPPA: f64 = 0.552_284_749_830_793_4;

/// `mark`'s lines as one path (circle closed, cross arms and digit open) in its fixed style, so
/// the canvas, PNG, SVG, hit and bounds draw it with the path machinery. The anchor ids are
/// positions, not stored ids.
#[must_use]
pub fn mark_path(mark: &RegistrationMarkSnapshot) -> PathSnapshot {
    let lines = mark_lines(mark);
    let mut next = 0_u64;
    let mut id = || {
        next += 1;
        AnchorId::new(0, next)
    };
    let (c, r) = (lines.centre, lines.radius.as_mm());
    let k = r * KAPPA;
    let smooth = |id, point: Point, handle: Vec2| NewAnchor {
        id,
        point,
        handle_in: Vec2::new(-handle.x, -handle.y),
        handle_out: handle,
        kind: AnchorKind::Symmetric,
    };
    let circle = vec![
        smooth(id(), Point::new(c.x + r, c.y), Vec2::new(0.0, k)),
        smooth(id(), Point::new(c.x, c.y + r), Vec2::new(-k, 0.0)),
        smooth(id(), Point::new(c.x - r, c.y), Vec2::new(0.0, -k)),
        smooth(id(), Point::new(c.x, c.y - r), Vec2::new(k, 0.0)),
    ];
    let mut open = |points: &[Point]| SubpathSnapshot {
        closed: false,
        anchors: points.iter().map(|&p| NewAnchor::corner(id(), p)).collect(),
    };
    let mut extra: Vec<SubpathSnapshot> = lines.cross.iter().map(|arm| open(arm)).collect();
    extra.extend(lines.digit.iter().map(|digit| open(digit)));
    PathSnapshot {
        id: mark.id,
        closed: true,
        style: mark.style.clone(),
        anchors: circle,
        extra_subpaths: extra,
        curve_kind: crate::CurveKind::Bezier,
        spiro_transform: None,
        width_points: Vec::new(),
        rotation: Angle::from_radians(0.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::path_model::NodeId;

    fn mark(number: u8) -> RegistrationMarkSnapshot {
        RegistrationMarkSnapshot::new(
            NodeId::from_parts(1, 1),
            number,
            Point::new(50.0, 40.0),
            Length::from_mm(6.0),
            Length::from_mm(0.25),
        )
    }

    #[test]
    fn a_default_mark_has_a_6_mm_circle_an_8_mm_cross_and_a_3_mm_digit() {
        let lines = mark_lines(&mark(1));
        assert!((lines.radius.as_mm() - 3.0).abs() < 1e-12);
        let [h, v] = lines.cross;
        assert!((h[1].x - h[0].x - 8.0).abs() < 1e-12);
        assert!((v[1].y - v[0].y - 8.0).abs() < 1e-12);
        let ys: Vec<f64> = lines.digit[0].iter().map(|p| p.y).collect();
        let span = ys.iter().copied().fold(f64::MIN, f64::max)
            - ys.iter().copied().fold(f64::MAX, f64::min);
        assert!((span - 3.0).abs() < 1e-12);
        let min_x = lines.digit[0].iter().map(|p| p.x).fold(f64::MAX, f64::min);
        assert!(min_x >= 50.0 + 3.5 - 1e-12, "beside the circle");
    }

    #[test]
    fn the_path_holds_a_closed_circle_two_arms_and_the_digit() {
        let path = mark_path(&mark(2));
        assert!(path.closed);
        assert_eq!(path.anchors.len(), 4);
        assert_eq!(path.extra_subpaths.len(), 3);
        assert!(path.extra_subpaths.iter().all(|s| !s.closed));
        assert!(path.style.stroke.enabled && !path.style.fill.enabled);
        assert!((path.style.stroke.width.as_mm() - 0.25).abs() < 1e-12);
    }

    #[test]
    fn every_number_has_a_digit_and_the_bounds_hold_it() {
        for n in 1..=4 {
            let lines = mark_lines(&mark(n));
            assert_eq!(lines.digit.len(), 1);
            let (min, max) = lines.bounds();
            for p in lines.digit.iter().flatten() {
                assert!(p.x <= max.x && p.y >= min.y);
            }
            assert!(min.x <= 46.0 - 0.125 + 1e-12);
        }
    }
}
