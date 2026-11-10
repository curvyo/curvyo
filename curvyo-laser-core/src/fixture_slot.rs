//! A holder's slot: a rectangle or a circle in the board's frame, with its fit size and its local
//! snap points (`specs/0102-laser-fixtures` criteria 19 to 21 and 27; ADR 0049 §3).

use curvyo_document_core::{Length, Point};

use crate::fixture_kind::SlotShape;

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
