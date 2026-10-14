//! The 2x3 affine map a linked clone applies on top of its original
//! (`specs/0049-linked-clone/adrs.md` decision 4).

use serde::Serialize;

use crate::units::{Angle, Point, Vec2};

/// The largest magnitude an entry of a clone matrix may have in a file or after an edit
/// (`specs/0049-linked-clone` criteria 17 and 38).
pub const MATRIX_ENTRY_LIMIT: f64 = 1e7;

/// Below this absolute determinant a matrix has no inverse (`0049` criterion 16).
pub const SINGULAR_DETERMINANT: f64 = 1e-9;

/// An affine map `x' = a·x + c·y + e`, `y' = b·x + d·y + f`, in SVG order, document
/// millimetres, X right and Y down. The linear part is `(a, c; b, d)`; `e` and `f` are
/// millimetres. Written to `document.json` as the list `[a, b, c, d, e, f]`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(into = "[f64; 6]")]
pub struct Affine {
    /// Row 1, column 1.
    pub a: f64,
    /// Row 2, column 1.
    pub b: f64,
    /// Row 1, column 2.
    pub c: f64,
    /// Row 2, column 2.
    pub d: f64,
    /// The x translation in mm.
    pub e: f64,
    /// The y translation in mm.
    pub f: f64,
}

impl Affine {
    /// The map that changes nothing.
    pub const IDENTITY: Self = Self::new(1.0, 0.0, 0.0, 1.0, 0.0, 0.0);

    /// The map with these six entries, in SVG order.
    #[must_use]
    #[allow(clippy::many_single_char_names)] // the SVG names of the six entries
    pub const fn new(a: f64, b: f64, c: f64, d: f64, e: f64, f: f64) -> Self {
        Self { a, b, c, d, e, f }
    }

    /// The map from the list `[a, b, c, d, e, f]`.
    #[must_use]
    pub const fn from_array(m: [f64; 6]) -> Self {
        Self::new(m[0], m[1], m[2], m[3], m[4], m[5])
    }

    /// The six entries in SVG order.
    #[must_use]
    pub const fn to_array(self) -> [f64; 6] {
        [self.a, self.b, self.c, self.d, self.e, self.f]
    }

    /// A pure translation by `offset`.
    #[must_use]
    pub const fn translation(offset: Vec2) -> Self {
        Self::new(1.0, 0.0, 0.0, 1.0, offset.x, offset.y)
    }

    /// A scale by `(sx, sy)` about the document origin.
    #[must_use]
    pub const fn scale(sx: f64, sy: f64) -> Self {
        Self::new(sx, 0.0, 0.0, sy, 0.0, 0.0)
    }

    /// A rotation by `angle` about `pivot`, the sense of [`Point::rotated_around`].
    #[must_use]
    pub fn rotation_about(pivot: Point, angle: Angle) -> Self {
        let (sin, cos) = angle.as_radians().sin_cos();
        let to_origin = Self::translation(Vec2::new(-pivot.x, -pivot.y));
        let back = Self::translation(Vec2::new(pivot.x, pivot.y));
        back.compose(Self::new(cos, sin, -sin, cos, 0.0, 0.0))
            .compose(to_origin)
    }

    /// `self · then`: the map that applies `then` first and `self` after it. For a chain
    /// B → A → O the composed matrix is `m_b.compose(m_a)`.
    #[must_use]
    pub fn compose(self, then: Self) -> Self {
        Self::new(
            self.a.mul_add(then.a, self.c * then.b),
            self.b.mul_add(then.a, self.d * then.b),
            self.a.mul_add(then.c, self.c * then.d),
            self.b.mul_add(then.c, self.d * then.d),
            self.a.mul_add(then.e, self.c.mul_add(then.f, self.e)),
            self.b.mul_add(then.e, self.d.mul_add(then.f, self.f)),
        )
    }

    /// `p` mapped by the whole map.
    #[must_use]
    pub fn apply_point(self, p: Point) -> Point {
        Point::new(
            self.a.mul_add(p.x, self.c.mul_add(p.y, self.e)),
            self.b.mul_add(p.x, self.d.mul_add(p.y, self.f)),
        )
    }

    /// `v` mapped by the linear part only (a handle vector, a translation).
    #[must_use]
    pub fn apply_vector(self, v: Vec2) -> Vec2 {
        Vec2::new(
            self.a.mul_add(v.x, self.c * v.y),
            self.b.mul_add(v.x, self.d * v.y),
        )
    }

    /// The determinant of the linear part.
    #[must_use]
    pub fn determinant(self) -> f64 {
        self.a.mul_add(self.d, -(self.c * self.b))
    }

    /// The inverse map, or `None` when the absolute determinant is below
    /// [`SINGULAR_DETERMINANT`] or not finite.
    #[must_use]
    pub fn inverse(self) -> Option<Self> {
        let det = self.determinant();
        if !det.is_finite() || det.abs() < SINGULAR_DETERMINANT {
            return None;
        }
        let (a, b, c, d) = (self.d / det, -self.b / det, -self.c / det, self.a / det);
        Some(Self::new(
            a,
            b,
            c,
            d,
            -a.mul_add(self.e, c * self.f),
            -b.mul_add(self.e, d * self.f),
        ))
    }

    /// Whether every entry is a finite number.
    #[must_use]
    pub fn is_finite(self) -> bool {
        self.to_array().iter().all(|v| v.is_finite())
    }

    /// Whether every entry is finite and within ±[`MATRIX_ENTRY_LIMIT`].
    #[must_use]
    pub fn is_within_limit(self) -> bool {
        self.to_array()
            .iter()
            .all(|v| v.is_finite() && v.abs() <= MATRIX_ENTRY_LIMIT)
    }
}

impl From<Affine> for [f64; 6] {
    fn from(m: Affine) -> Self {
        m.to_array()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f64 = 1e-12;

    fn close(a: Affine, b: Affine) -> bool {
        a.to_array()
            .iter()
            .zip(b.to_array())
            .all(|(x, y)| (x - y).abs() < EPS)
    }

    #[test]
    fn compose_applies_the_right_operand_first() {
        // Criterion 50: M_B = scale 2, M_A = translate (30, 0): M_B·M_A = (2, 0, 0, 2, 60, 0).
        let m_a = Affine::translation(Vec2::new(30.0, 0.0));
        let m_b = Affine::scale(2.0, 2.0);
        assert_eq!(m_b.compose(m_a), Affine::new(2.0, 0.0, 0.0, 2.0, 60.0, 0.0));
        let p = Point::new(10.0, 10.0);
        assert_eq!(
            m_b.compose(m_a).apply_point(p),
            m_b.apply_point(m_a.apply_point(p))
        );
    }

    #[test]
    fn a_quarter_turn_maps_right_to_down() {
        // Criterion 12: (a, b, c, d) = (0, 1, -1, 0) maps a move right to a move down.
        let turn = Affine::new(0.0, 1.0, -1.0, 0.0, 0.0, 0.0);
        assert_eq!(turn.apply_vector(Vec2::new(5.0, 0.0)), Vec2::new(0.0, 5.0));
    }

    #[test]
    fn a_rotation_about_a_pivot_matches_the_point_rule() {
        let pivot = Point::new(3.0, -2.0);
        let angle = Angle::from_radians(0.7);
        let p = Point::new(10.0, 4.0);
        let by_matrix = Affine::rotation_about(pivot, angle).apply_point(p);
        let by_point = p.rotated_around(pivot, angle);
        assert!((by_matrix.x - by_point.x).abs() < 1e-12);
        assert!((by_matrix.y - by_point.y).abs() < 1e-12);
    }

    #[test]
    fn a_vector_ignores_the_translation() {
        let m = Affine::new(2.0, 0.0, 0.0, 3.0, 100.0, 50.0);
        assert_eq!(m.apply_vector(Vec2::new(1.0, 1.0)), Vec2::new(2.0, 3.0));
        assert_eq!(m.apply_point(Point::new(1.0, 1.0)), Point::new(102.0, 53.0));
    }

    #[test]
    fn the_inverse_undoes_the_map() {
        let m = Affine::new(2.0, 0.5, -1.0, 3.0, 7.0, -4.0);
        let inverse = m.inverse().unwrap();
        assert!(close(m.compose(inverse), Affine::IDENTITY));
        assert!(close(inverse.compose(m), Affine::IDENTITY));
    }

    #[test]
    fn a_singular_map_has_no_inverse() {
        assert_eq!(Affine::scale(0.0, 1.0).inverse(), None);
        assert_eq!(Affine::scale(1e-10, 1.0).inverse(), None);
        assert_eq!(
            Affine::new(f64::NAN, 0.0, 0.0, 1.0, 0.0, 0.0).inverse(),
            None
        );
    }

    #[test]
    fn the_limit_holds_every_entry() {
        assert!(Affine::IDENTITY.is_within_limit());
        assert!(Affine::new(1.0, 0.0, 0.0, 1.0, 1e7, -1e7).is_within_limit());
        assert!(!Affine::new(1.0, 0.0, 0.0, 1.0, 1.000_001e7, 0.0).is_within_limit());
        assert!(!Affine::new(f64::INFINITY, 0.0, 0.0, 1.0, 0.0, 0.0).is_within_limit());
        assert!(Affine::new(1e8, 0.0, 0.0, 1.0, 0.0, 0.0).is_finite());
    }

    #[test]
    fn the_json_form_is_the_list_of_six() {
        let json = serde_json::to_string(&Affine::new(1.0, 2.0, 3.0, 4.0, 5.0, 6.0)).unwrap();
        assert_eq!(json, "[1.0,2.0,3.0,4.0,5.0,6.0]");
    }
}
