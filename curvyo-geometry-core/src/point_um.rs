//! A point in whole micrometres, the unit of every machine move (ADR 0064 §2), shared by the
//! machine families through `path_order` (`specs/0159-cutting-plotter` `adrs.md` PL4).

use curvyo_document_core::Point;

/// A point in whole micrometres of the page (Y grows down), the unit of every move.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct PointUm {
    /// X in micrometres from the page's left edge.
    pub x: i64,
    /// Y in micrometres from the page's top edge.
    pub y: i64,
}

impl PointUm {
    /// The point at `(x, y)` micrometres.
    #[must_use]
    pub const fn new(x: i64, y: i64) -> Self {
        Self { x, y }
    }

    /// The page point `point` (mm) rounded to whole micrometres (ADR 0064 §2).
    #[must_use]
    pub fn of_mm(point: Point) -> Self {
        Self::new(um_of_mm(point.x), um_of_mm(point.y))
    }

    /// The point in mm.
    #[must_use]
    pub fn to_mm(self) -> Point {
        Point::new(mm_of_um(self.x), mm_of_um(self.y))
    }

    /// The squared distance to `other` in µm², exact.
    #[must_use]
    pub const fn distance_squared(self, other: Self) -> i128 {
        let (dx, dy) = ((other.x - self.x) as i128, (other.y - self.y) as i128);
        dx * dx + dy * dy
    }

    /// The distance to `other` in mm. Uses `sqrt`, not `f64::hypot`: `sqrt` is correctly rounded
    /// by IEEE 754 on every target while `hypot` comes from the platform's libm, and job bytes
    /// must not differ between native and wasm32.
    #[must_use]
    pub fn distance_mm(self, other: Self) -> f64 {
        let (dx, dy) = (mm_of_um(self.x - other.x), mm_of_um(self.y - other.y));
        (dx * dx + dy * dy).sqrt()
    }
}

/// `mm` rounded to whole micrometres; a value beyond the range of `i64` saturates.
#[must_use]
pub fn um_of_mm(mm: f64) -> i64 {
    #[allow(clippy::cast_possible_truncation)]
    // invariant: `as` saturates; page coordinates are far inside the range (10 m beds)
    let um = (mm * 1000.0).round() as i64;
    um
}

/// `um` micrometres in mm.
#[must_use]
pub fn mm_of_um(um: i64) -> f64 {
    #[allow(clippy::cast_precision_loss)]
    // exact: page coordinates in µm are far below 2^53
    let mm = um as f64 / 1000.0;
    mm
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Determinism note (ADR 0064): job bytes must match on native and wasm32, so distances use
    /// `sqrt` (correctly rounded everywhere) instead of libm's `hypot`. This pins the bits of a
    /// non-Pythagorean distance; the GRBL goldens pin the written bytes on every CI target.
    #[test]
    fn the_distance_is_the_correctly_rounded_square_root() {
        let d = PointUm::new(0, 0).distance_mm(PointUm::new(1_000, 1_000));
        assert_eq!(d.to_bits(), 2_f64.sqrt().to_bits());
        let d = PointUm::new(1_234, -5_678).distance_mm(PointUm::new(-9_876, 5_432));
        let (dx, dy) = (11.11_f64, 11.11_f64);
        assert_eq!(d.to_bits(), (dx * dx + dy * dy).sqrt().to_bits());
        assert!((PointUm::new(0, 0).distance_mm(PointUm::new(3_000, 4_000)) - 5.0).abs() < 1e-12);
    }
}
