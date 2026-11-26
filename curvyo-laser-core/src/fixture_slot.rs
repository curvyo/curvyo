//! A holder's slot: a rectangle or a circle in the board's frame, with its fit size and its local
//! snap points (`specs/0102-laser-fixtures` criteria 19 to 21 and 27; ADR 0049 §3).

use curvyo_document_core::{Length, Point};

use crate::fixture_kind::SlotShape;

/// How far past the grown outline a point may round and still lie in it, in mm.
const HOLDS_EPSILON_MM: f64 = 1e-9;

/// One slot of a holder. The centre is in mm from the board's top-left corner, and only the sizes
/// of the slot's shape are meaningful: width and height of a rectangle, the diameter of a circle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Slot {
    /// Rectangle or circle.
    pub shape: SlotShape,
    /// The centre in the board's frame.
    pub centre: Point,
    /// The width of a rectangle.
    pub width: Length,
    /// The height of a rectangle.
    pub height: Length,
    /// The diameter of a circle.
    pub diameter: Length,
}

impl Slot {
    /// The size a part must match to fit: width and height of a rectangle, the diameter twice
    /// for a circle.
    #[must_use]
    pub fn extent(&self) -> (Length, Length) {
        match self.shape {
            SlotShape::Rectangle => (self.width, self.height),
            SlotShape::Circle => (self.diameter, self.diameter),
        }
    }

    /// Whether `point` (in the board's frame) lies in the slot's closed outline grown by
    /// `tolerance`: a rectangle by each axis, a circle by the distance to its centre
    /// (`specs/0154-library-into-slots` Terms "Filled", ADR 0088 decision 4).
    #[must_use]
    pub fn holds(&self, point: Point, tolerance: Length) -> bool {
        let (dx, dy) = (point.x - self.centre.x, point.y - self.centre.y);
        let grow = tolerance.as_mm() + HOLDS_EPSILON_MM;
        match self.shape {
            SlotShape::Rectangle => {
                dx.abs() <= self.width.as_mm() / 2.0 + grow
                    && dy.abs() <= self.height.as_mm() / 2.0 + grow
            }
            SlotShape::Circle => dx.hypot(dy) <= self.diameter.as_mm() / 2.0 + grow,
        }
    }

    /// The slot's centre, its four corners of its bounding box and the four midpoints of its
    /// edges, in the board's frame (nine points, criterion 27 b).
    #[must_use]
    pub fn local_points(&self) -> Vec<Point> {
        let (width, height) = self.extent();
        let (half_w, half_h) = (width.as_mm() / 2.0, height.as_mm() / 2.0);
        let (cx, cy) = (self.centre.x, self.centre.y);
        vec![
            Point::new(cx, cy),
            Point::new(cx - half_w, cy - half_h),
            Point::new(cx + half_w, cy - half_h),
            Point::new(cx + half_w, cy + half_h),
            Point::new(cx - half_w, cy + half_h),
            Point::new(cx, cy - half_h),
            Point::new(cx + half_w, cy),
            Point::new(cx, cy + half_h),
            Point::new(cx - half_w, cy),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(cx: f64, cy: f64, w: f64, h: f64) -> Slot {
        Slot {
            shape: SlotShape::Rectangle,
            centre: Point::new(cx, cy),
            width: Length::from_mm(w),
            height: Length::from_mm(h),
            diameter: Length::from_mm(0.0),
        }
    }

    #[test]
    fn a_circle_fits_a_square_of_its_diameter() {
        let circle = Slot {
            shape: SlotShape::Circle,
            centre: Point::new(30.0, 30.0),
            width: Length::from_mm(0.0),
            height: Length::from_mm(0.0),
            diameter: Length::from_mm(12.0),
        };
        assert_eq!(
            circle.extent(),
            (Length::from_mm(12.0), Length::from_mm(12.0))
        );
    }

    #[test]
    fn a_rectangle_holds_a_point_up_to_its_edge_grown_by_the_tolerance() {
        let slot = rect(50.0, 30.0, 20.0, 10.0);
        let tol = Length::from_mm(0.1);
        assert!(slot.holds(Point::new(50.0, 30.0), tol));
        assert!(slot.holds(Point::new(60.1, 35.1), tol));
        assert!(!slot.holds(Point::new(60.2, 30.0), tol));
        assert!(!slot.holds(Point::new(50.0, 35.2), tol));
    }

    #[test]
    fn a_circle_holds_a_point_by_its_distance_to_the_centre() {
        let circle = Slot {
            shape: SlotShape::Circle,
            centre: Point::new(0.0, 0.0),
            width: Length::from_mm(0.0),
            height: Length::from_mm(0.0),
            diameter: Length::from_mm(20.0),
        };
        let tol = Length::from_mm(0.1);
        assert!(circle.holds(Point::new(10.1, 0.0), tol));
        // The box corner is outside the circle.
        assert!(!circle.holds(Point::new(9.0, 9.0), tol));
        assert!(!circle.holds(Point::new(0.0, -10.2), tol));
    }

    #[test]
    fn a_rectangle_has_nine_snap_points_with_the_centre_first() {
        let points = rect(50.0, 30.0, 20.0, 10.0).local_points();
        assert_eq!(points.len(), 9);
        assert_eq!(points[0], Point::new(50.0, 30.0));
        assert!(points.contains(&Point::new(40.0, 25.0)));
        assert!(points.contains(&Point::new(60.0, 35.0)));
        assert!(points.contains(&Point::new(50.0, 25.0)));
        assert!(points.contains(&Point::new(40.0, 30.0)));
    }
}
