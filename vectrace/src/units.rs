//! The units of the tracer: millimetres, radians and the map from pixels to millimetres.
//!
//! The arithmetic of [`PixelMap`] is the arithmetic of `curvyo-document-core`'s `Affine`, entry
//! for entry with `mul_add` unchanged, so a trace gives the same last bit as before (ADR 0048 §3).

use kurbo::{Point, Vec2};

/// Below this absolute determinant a map has no inverse.
const SINGULAR_DETERMINANT: f64 = 1e-9;

/// A length in millimetres.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mm(f64);

impl Mm {
    /// A length from a value already in millimetres.
    #[must_use]
    pub const fn from_mm(millimetres: f64) -> Self {
        Self(millimetres)
    }

    /// The length as a plain millimetre value.
    #[must_use]
    pub const fn as_mm(self) -> f64 {
        self.0
    }
}

/// An angle in radians.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Angle(f64);

impl Angle {
    /// An angle from a value already in radians.
    #[must_use]
    pub const fn from_radians(radians: f64) -> Self {
        Self(radians)
    }

    /// The angle as a plain radian value.
    #[must_use]
    pub const fn as_radians(self) -> f64 {
        self.0
    }
}

/// The map from pixel corners to millimetres: `x' = a·x + c·y + e`, `y' = b·x + d·y + f`, held as
/// the list `[a, b, c, d, e, f]` in SVG order.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PixelMap([f64; 6]);

#[allow(clippy::many_single_char_names)] // the SVG names of the six entries
impl PixelMap {
    /// The map with these six entries, in SVG order.
    #[must_use]
    pub const fn new(entries: [f64; 6]) -> Self {
        Self(entries)
    }

    /// The map that changes nothing.
    pub const IDENTITY: Self = Self([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);

    /// The six entries in SVG order.
    #[must_use]
    pub const fn to_array(self) -> [f64; 6] {
        self.0
    }

    /// A scale by `(sx, sy)` about the origin.
    #[must_use]
    pub const fn scale(sx: f64, sy: f64) -> Self {
        Self([sx, 0.0, 0.0, sy, 0.0, 0.0])
    }

    /// A pure translation by `offset`.
    #[must_use]
    pub const fn translation(offset: Vec2) -> Self {
        Self([1.0, 0.0, 0.0, 1.0, offset.x, offset.y])
    }

    /// `self · then`: the map that applies `then` first and `self` after it.
    #[must_use]
    pub fn compose(self, then: Self) -> Self {
        let [a, b, c, d, e, f] = self.0;
        let t = then.0;
        Self([
            a.mul_add(t[0], c * t[1]),
            b.mul_add(t[0], d * t[1]),
            a.mul_add(t[2], c * t[3]),
            b.mul_add(t[2], d * t[3]),
            a.mul_add(t[4], c.mul_add(t[5], e)),
            b.mul_add(t[4], d.mul_add(t[5], f)),
        ])
    }

    /// `p` mapped by the whole map.
    #[must_use]
    pub fn apply_point(self, p: Point) -> Point {
        let [a, b, c, d, e, f] = self.0;
        Point::new(
            a.mul_add(p.x, c.mul_add(p.y, e)),
            b.mul_add(p.x, d.mul_add(p.y, f)),
        )
    }

    /// `v` mapped by the linear part only.
    #[must_use]
    pub fn apply_vector(self, v: Vec2) -> Vec2 {
        let [a, b, c, d, ..] = self.0;
        Vec2::new(a.mul_add(v.x, c * v.y), b.mul_add(v.x, d * v.y))
    }

    /// The determinant of the linear part.
    #[must_use]
    pub fn determinant(self) -> f64 {
        let [a, b, c, d, ..] = self.0;
        a.mul_add(d, -(c * b))
    }

    /// The inverse map, or `None` when the absolute determinant is below `1e-9` or not finite.
    #[must_use]
    pub fn inverse(self) -> Option<Self> {
        let det = self.determinant();
        if !det.is_finite() || det.abs() < SINGULAR_DETERMINANT {
            return None;
        }
        let [sa, sb, sc, sd, se, sf] = self.0;
        let (a, b, c, d) = (sd / det, -sb / det, -sc / det, sa / det);
        Some(Self([
            a,
            b,
            c,
            d,
            -a.mul_add(se, c * sf),
            -b.mul_add(se, d * sf),
        ]))
    }

    /// Whether every entry is a finite number.
    #[must_use]
    pub fn is_finite(self) -> bool {
        self.0.iter().all(|v| v.is_finite())
    }

    /// The same map as a `kurbo` affine, to map outlines.
    #[must_use]
    pub fn to_kurbo(self) -> kurbo::Affine {
        kurbo::Affine::new(self.0)
    }
}

#[cfg(test)]
mod tests {
    use curvyo_document_core::{Affine, Point as DocPoint, Vec2 as DocVec2};

    use super::*;

    const MAPS: [[f64; 6]; 5] = [
        [0.1, 0.0, 0.0, 0.1, 5.0, 7.0],
        [
            0.264_583_333_333_333_3,
            0.0,
            0.0,
            -0.264_583_333_333_333_3,
            12.345_678_9,
            99.9,
        ],
        [0.7, 0.3, -0.2, 1.3, 0.1, -0.3],
        [1e-5, 3e-6, -7e-6, 2e-5, 1e3, -1e3],
        [1.0, 2.0, 2.0, 4.0, 0.0, 0.0],
    ];

    fn bits(a: [f64; 6]) -> [u64; 6] {
        a.map(f64::to_bits)
    }

    #[test]
    fn matches_document_core_bit_for_bit() {
        let pts = [(0.0, 0.0), (1.0, 1.0), (123.456, -78.9), (1e-7, 3.3)];
        for m in MAPS {
            for n in MAPS {
                let ours = PixelMap::new(m).compose(PixelMap::new(n)).to_array();
                let theirs = Affine::from_array(m)
                    .compose(Affine::from_array(n))
                    .to_array();
                assert_eq!(bits(ours), bits(theirs));
            }
            let (pm, am) = (PixelMap::new(m), Affine::from_array(m));
            assert_eq!(pm.determinant().to_bits(), am.determinant().to_bits());
            assert_eq!(pm.is_finite(), am.is_finite());
            assert_eq!(
                pm.inverse().map(|i| bits(i.to_array())),
                am.inverse().map(|i| bits(i.to_array()))
            );
            for (x, y) in pts {
                let o = pm.apply_point(Point::new(x, y));
                let t = am.apply_point(DocPoint::new(x, y));
                assert_eq!(
                    (o.x.to_bits(), o.y.to_bits()),
                    (t.x.to_bits(), t.y.to_bits())
                );
                let o = pm.apply_vector(Vec2::new(x, y));
                let t = am.apply_vector(DocVec2::new(x, y));
                assert_eq!(
                    (o.x.to_bits(), o.y.to_bits()),
                    (t.x.to_bits(), t.y.to_bits())
                );
            }
        }
    }

    #[test]
    fn scale_and_translation_match_document_core() {
        assert_eq!(
            PixelMap::scale(0.5, 0.25).to_array(),
            Affine::scale(0.5, 0.25).to_array()
        );
        assert_eq!(
            PixelMap::translation(Vec2::new(3.0, -4.0)).to_array(),
            Affine::translation(DocVec2::new(3.0, -4.0)).to_array()
        );
    }

    #[test]
    fn a_singular_or_non_finite_map_has_no_inverse() {
        assert!(
            PixelMap::new([1.0, 2.0, 2.0, 4.0, 0.0, 0.0])
                .inverse()
                .is_none()
        );
        assert!(
            PixelMap::new([f64::NAN, 0.0, 0.0, 1.0, 0.0, 0.0])
                .inverse()
                .is_none()
        );
        assert!(!PixelMap::new([f64::INFINITY, 0.0, 0.0, 1.0, 0.0, 0.0]).is_finite());
    }

    #[test]
    fn units_hold_their_value() {
        assert!((Mm::from_mm(2.5).as_mm() - 2.5).abs() < f64::EPSILON);
        assert!((Angle::from_radians(1.5).as_radians() - 1.5).abs() < f64::EPSILON);
    }
}
