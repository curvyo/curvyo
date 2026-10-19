//! The typed geometry of a click-to-create placement: the shape that a size, an angle and the
//! kept direction give from point A (`specs/0052-click-to-create-shapes/` criteria 16 to 19,
//! `adrs.md` decisions 1 and 2).

use curvyo_document_core::{
    Angle, CornerRadii, EllipseFrame, Length, Point, RectBounds, Shape, StarFrame,
};

/// The side of A a rectangle or ellipse grows to on each axis, kept from the pointer at the
/// freeze. An axis with no offset counts as right or down (the spec's "Direction").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Direction {
    /// The box grows to the right of A.
    pub(crate) right: bool,
    /// The box grows below A (Y grows down, ADR 0002 §4).
    pub(crate) down: bool,
}

impl Direction {
    /// The direction of `pointer` seen from `a`.
    pub(crate) fn of(a: Point, pointer: Point) -> Self {
        Self {
            right: pointer.x >= a.x,
            down: pointer.y >= a.y,
        }
    }
}

/// How a rectangle's or ellipse's box sits at A before it is turned.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct BoxAtA {
    /// Point A.
    pub(crate) a: Point,
    /// The full width of the box.
    pub(crate) width: Length,
    /// The full height of the box.
    pub(crate) height: Length,
    /// The side of A the box grows to, without Shift.
    pub(crate) direction: Direction,
    /// Shift kept: A is the box's centre.
    pub(crate) centred: bool,
}

impl BoxAtA {
    /// The centre of the box once turned by `angle` about A, clockwise on screen: the box's own
    /// centre `c` moved to `A + R(angle)(c - A)` (`adrs.md` decision 2). The stored primitive is
    /// the unturned box about that centre, with `rotation = angle`, since its register turns it
    /// about its own centre.
    fn turned_centre(self, angle: Angle) -> Point {
        if self.centred {
            return self.a;
        }
        let half_w = self.width.as_mm() / 2.0;
        let half_h = self.height.as_mm() / 2.0;
        let dx = if self.direction.right {
            half_w
        } else {
            -half_w
        };
        let dy = if self.direction.down { half_h } else { -half_h };
        let (sin, cos) = angle.as_radians().sin_cos();
        Point::new(
            self.a.x + dx * cos - dy * sin,
            self.a.y + dx * sin + dy * cos,
        )
    }
}

/// The rectangle a placement makes (criterion 16): a corner at A, or its centre with Shift,
/// extending in the kept direction, turned by `angle` about A. Zero corner radii, as a
/// create-drag makes it.
pub(crate) fn placed_rect(at: BoxAtA, angle: Angle) -> Shape {
    let centre = at.turned_centre(angle);
    Shape::Rect {
        bounds: RectBounds {
            origin: Point::new(
                centre.x - at.width.as_mm() / 2.0,
                centre.y - at.height.as_mm() / 2.0,
            ),
            width: at.width,
            height: at.height,
        },
        corner_radii: CornerRadii::uniform(Length::from_mm(0.0)),
    }
}

/// The ellipse a placement makes (criterion 18): the ellipse in the box of [`placed_rect`].
pub(crate) fn placed_ellipse(at: BoxAtA, angle: Angle) -> Shape {
    Shape::Ellipse {
        frame: EllipseFrame {
            center: at.turned_centre(angle),
            rx: Length::from_mm(at.width.as_mm() / 2.0),
            ry: Length::from_mm(at.height.as_mm() / 2.0),
        },
        arc: None,
        curve: curvyo_document_core::Curve::ELLIPSE_DEFAULT,
    }
}

/// The frame of a polygon or star a placement makes (criterion 19): centred at A, its first
/// vertex at `angle` (`StarFrame.angle`; the `rotation` register stays 0, `adrs.md` decision 2).
pub(crate) fn placed_star_frame(a: Point, radius: Length, angle: Angle) -> StarFrame {
    StarFrame {
        center: a,
        radius,
        angle: angle.normalized(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOL: f64 = 1e-9;

    fn degrees(value: f64) -> Angle {
        Angle::from_radians(value.to_radians())
    }

    fn box_at(a: Point, pointer: Point, w: f64, h: f64, centred: bool) -> BoxAtA {
        BoxAtA {
            a,
            width: Length::from_mm(w),
            height: Length::from_mm(h),
            direction: Direction::of(a, pointer),
            centred,
        }
    }

    fn assert_rect(shape: Shape, x: f64, y: f64, w: f64, h: f64) {
        let Shape::Rect { bounds, .. } = shape else {
            panic!("a rectangle");
        };
        assert!(
            (bounds.origin.x - x).abs() < TOL && (bounds.origin.y - y).abs() < TOL,
            "origin {:?}",
            bounds.origin
        );
        assert!((bounds.width.as_mm() - w).abs() < TOL);
        assert!((bounds.height.as_mm() - h).abs() < TOL);
    }

    fn ellipse_centre(shape: Shape) -> (Point, f64, f64) {
        let Shape::Ellipse { frame, .. } = shape else {
            panic!("an ellipse");
        };
        (frame.center, frame.rx.as_mm(), frame.ry.as_mm())
    }

    const A: Point = Point::new(100.0, 50.0);
    const POINTER: Point = Point::new(130.0, 40.0);

    /// Criterion 16 (a): "40", Tab, "20": origin (100, 30), width 40, height 20.
    #[test]
    fn criterion_16a_the_box_grows_in_the_kept_direction() {
        let shape = placed_rect(box_at(A, POINTER, 40.0, 20.0, false), degrees(0.0));
        assert_rect(shape, 100.0, 30.0, 40.0, 20.0);
    }

    /// Criterion 16 (b): turned 90° about A, the stored box has its centre at (110, 70).
    #[test]
    fn criterion_16b_the_angle_turns_the_box_about_a() {
        let shape = placed_rect(box_at(A, POINTER, 40.0, 20.0, false), degrees(90.0));
        assert_rect(shape, 90.0, 60.0, 40.0, 20.0);
    }

    /// Criterion 16 (c): Shift kept puts the centre at A: origin (80, 40).
    #[test]
    fn criterion_16c_shift_kept_centres_the_box_on_a() {
        let shape = placed_rect(box_at(A, POINTER, 40.0, 20.0, true), degrees(0.0));
        assert_rect(shape, 80.0, 40.0, 40.0, 20.0);
        let turned = placed_rect(box_at(A, POINTER, 40.0, 20.0, true), degrees(30.0));
        assert_rect(turned, 80.0, 40.0, 40.0, 20.0);
    }

    /// Criterion 17: a 25 x 25 square from A extending right and up: origin (100, 25).
    #[test]
    fn criterion_17_a_linked_square_extends_right_and_up() {
        let shape = placed_rect(box_at(A, POINTER, 25.0, 25.0, false), degrees(0.0));
        assert_rect(shape, 100.0, 25.0, 25.0, 25.0);
    }

    /// Criterion 18: the circle's 2r box has a corner at A, or its centre with Shift, and turns
    /// about A.
    #[test]
    fn criterion_18_the_circle_box_sits_at_a_and_turns_about_it() {
        let a = Point::new(50.0, 50.0);
        let pointer = Point::new(70.0, 80.0);
        let (c, rx, ry) = ellipse_centre(placed_ellipse(
            box_at(a, pointer, 20.0, 20.0, false),
            degrees(0.0),
        ));
        assert!((c.x - 60.0).abs() < TOL && (c.y - 60.0).abs() < TOL);
        assert!((rx - 10.0).abs() < TOL && (ry - 10.0).abs() < TOL);
        let (c, rx, _) = ellipse_centre(placed_ellipse(
            box_at(a, pointer, 10.0, 10.0, false),
            degrees(0.0),
        ));
        assert!((c.x - 55.0).abs() < TOL && (c.y - 55.0).abs() < TOL && (rx - 5.0).abs() < TOL);
        let (c, ..) = ellipse_centre(placed_ellipse(
            box_at(a, pointer, 20.0, 20.0, true),
            degrees(0.0),
        ));
        assert!((c.x - 50.0).abs() < TOL && (c.y - 50.0).abs() < TOL);
        let (c, ..) = ellipse_centre(placed_ellipse(
            box_at(a, pointer, 20.0, 20.0, false),
            degrees(90.0),
        ));
        assert!(
            (c.x - 40.0).abs() < TOL && (c.y - 60.0).abs() < TOL,
            "{c:?}"
        );
    }

    /// The spec's "Direction": an axis with no offset counts as right or down.
    #[test]
    fn an_axis_with_no_offset_counts_as_right_or_down() {
        assert_eq!(
            Direction::of(A, A),
            Direction {
                right: true,
                down: true
            }
        );
        assert_eq!(
            Direction::of(A, Point::new(90.0, 50.0)),
            Direction {
                right: false,
                down: true
            }
        );
    }

    /// Criterion 19: centre A, radius r, the first vertex at the angle.
    #[test]
    fn criterion_19_the_first_vertex_lies_at_the_angle() {
        let a = Point::new(100.0, 100.0);
        let r = Length::from_mm(25.0);
        for (angle, x, y) in [(0.0, 125.0, 100.0), (-90.0, 100.0, 75.0)] {
            let frame = placed_star_frame(a, r, degrees(angle));
            let vertex = Point::new(
                a.x + r.as_mm() * frame.angle.as_radians().cos(),
                a.y + r.as_mm() * frame.angle.as_radians().sin(),
            );
            assert!((vertex.x - x).abs() < TOL && (vertex.y - y).abs() < TOL);
        }
    }
}
