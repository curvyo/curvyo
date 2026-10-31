//! Computes a glyph shape's occupancy descriptor for the shortlist.
//!
//! Even-odd occupancy of a 16 x 16 grid over the shape's box (cell-centre point in polygon), the
//! box's aspect ratio and the hole count; the distance is Hamming plus weighted aspect and hole
//! differences (`adrs.md` decision 4). Only for the shortlist; it never decides a letter.

use kurbo::{Point, Rect};

use super::sample::inside;

/// The grid's cells per side.
const GRID: usize = 16;
/// The weight of a doubled aspect ratio, in cells.
const ASPECT_WEIGHT: f64 = 24.0;
/// The weight of one hole more or less, in cells.
const HOLE_WEIGHT: f64 = 16.0;

/// A shape's descriptor.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Descriptor {
    bits: [u64; GRID * GRID / 64],
    /// Width over height of the box, both at least a tiny length.
    aspect: f64,
    holes: usize,
}

impl Descriptor {
    /// The descriptor of the even-odd area of `rings` within `bbox`.
    pub(crate) fn of(rings: &[&[Point]], bbox: Rect, holes: usize) -> Self {
        let mut bits = [0_u64; GRID * GRID / 64];
        let cells = f64::from(u32::try_from(GRID).unwrap_or(16));
        let (w, h) = (bbox.width(), bbox.height());
        for row in 0..GRID {
            for column in 0..GRID {
                let p = Point::new(
                    bbox.x0 + (f64::from(u32::try_from(column).unwrap_or(0)) + 0.5) * w / cells,
                    bbox.y0 + (f64::from(u32::try_from(row).unwrap_or(0)) + 0.5) * h / cells,
                );
                let crossings = rings.iter().filter(|ring| inside(ring, p)).count();
                if crossings % 2 == 1 {
                    let at = row * GRID + column;
                    bits[at / 64] |= 1 << (at % 64);
                }
            }
        }
        let tiny = 1e-9;
        Self {
            bits,
            aspect: w.max(tiny) / h.max(tiny),
            holes,
        }
    }

    /// The shortlist distance: differing cells plus the aspect and hole terms.
    pub(crate) fn distance(&self, other: &Self) -> f64 {
        let hamming: u32 = self
            .bits
            .iter()
            .zip(&other.bits)
            .map(|(a, b)| (a ^ b).count_ones())
            .sum();
        let ratio = if self.aspect > other.aspect {
            self.aspect / other.aspect
        } else {
            other.aspect / self.aspect
        };
        let holes = self.holes.abs_diff(other.holes);
        f64::from(hamming)
            + ASPECT_WEIGHT * (ratio - 1.0).min(4.0)
            + HOLE_WEIGHT * f64::from(u32::try_from(holes).unwrap_or(4).min(4))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_square_and_a_ring_differ_by_their_hole() {
        let outer = [
            Point::new(0.0, 0.0),
            Point::new(10.0, 0.0),
            Point::new(10.0, 10.0),
            Point::new(0.0, 10.0),
        ];
        let inner = [
            Point::new(3.0, 3.0),
            Point::new(7.0, 3.0),
            Point::new(7.0, 7.0),
            Point::new(3.0, 7.0),
        ];
        let bbox = Rect::new(0.0, 0.0, 10.0, 10.0);
        let square = Descriptor::of(&[&outer], bbox, 0);
        let ring = Descriptor::of(&[&outer, &inner], bbox, 1);
        assert!(square.distance(&square).abs() < 1e-12);
        let d = square.distance(&ring);
        assert!(d > HOLE_WEIGHT, "{d}");
    }
}
