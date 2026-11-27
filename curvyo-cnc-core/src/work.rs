//! The one place page coordinates meet G-code work coordinates (`specs/0161-cnc-output`
//! `adrs.md` CN10): work zero at the stock's front-left corner on its top face, Y up, Z negative
//! into the stock.

use curvyo_document_core::Point;

/// A point in work coordinates, in mm.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WorkPoint {
    x: f64,
    y: f64,
    z: f64,
}

impl WorkPoint {
    /// The point `(x, y, z)` in mm.
    #[must_use]
    pub const fn mm(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    /// X in mm.
    #[must_use]
    pub const fn x(self) -> f64 {
        self.x
    }

    /// Y in mm.
    #[must_use]
    pub const fn y(self) -> f64 {
        self.y
    }

    /// Z in mm (0 on the stock top).
    #[must_use]
    pub const fn z(self) -> f64 {
        self.z
    }

    /// The same point at height `z`.
    #[must_use]
    pub const fn at_z(self, z: f64) -> Self {
        Self { z, ..self }
    }

    /// The distance to `other` in mm.
    #[must_use]
    pub fn distance(self, other: Self) -> f64 {
        let (dx, dy, dz) = (other.x - self.x, other.y - self.y, other.z - self.z);
        (dx * dx + dy * dy + dz * dz).sqrt()
    }
}

/// The page point `p` (mm, Y down, the stock's top-left corner at the page origin) at height `z`
/// in work coordinates, for a stock `stock_height_mm` deep in Y: `X = x`, `Y = height - y`.
#[must_use]
pub fn page_to_work(p: Point, z: f64, stock_height_mm: f64) -> WorkPoint {
    WorkPoint::mm(p.x, stock_height_mm - p.y, z)
}

/// The page point of the work point `w`: the inverse of [`page_to_work`].
#[must_use]
pub fn work_to_page(w: WorkPoint, stock_height_mm: f64) -> Point {
    Point::new(w.x, stock_height_mm - w.y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_front_left_stock_corner_is_work_zero_and_the_maps_invert() {
        let front_left = page_to_work(Point::new(0.0, 150.0), 0.0, 150.0);
        assert_eq!(front_left, WorkPoint::mm(0.0, 0.0, 0.0));
        let back_left = page_to_work(Point::new(0.0, 0.0), -2.0, 150.0);
        assert_eq!(back_left, WorkPoint::mm(0.0, 150.0, -2.0));
        let p = Point::new(12.5, 40.0);
        assert_eq!(work_to_page(page_to_work(p, 1.0, 150.0), 150.0), p);
    }
}
