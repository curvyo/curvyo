//! The outline of a ring of vertices rounded by Inkscape's tangent rule
//! (`specs/0145-shape-spiro-handles/adrs.md` decisions 1, 2 and 5): the closed chain of cubics
//! whose handles lie along the tangent at every vertex, and the exact box of that chain.
//! Pure arithmetic on a shape's parameters; no Loro type.

#![allow(clippy::many_single_char_names, clippy::similar_names)]

use kurbo::{CubicBez, ParamCurveExtrema};
use serde::Serialize;

use crate::path_model::AnchorKind;
use crate::primitive_model::Shape;
use crate::primitive_outline::OutlineAnchor;
use crate::ring_curve::Ring;
use crate::units::{Point, Vec2};

/// A polygon's or star's Rounded value `q` (criterion 4): `0` the straight shape, positive
/// values pull a soft curve out of every corner, negative values flip the tangent, and large
/// magnitudes loop. Stored raw and finite; evaluated limited to `-10..=10` (criterion 11).
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Rounded(f64);

impl Rounded {
    /// The straight shape: the value a polygon or star has when none is stored.
    pub const ZERO: Self = Self(0.0);
    /// The largest magnitude a value is evaluated with (Inkscape's field range).
    pub const RANGE: f64 = 10.0;

    /// `value` as stored, refused when it is not finite (a file's value is `Damaged`).
    #[must_use]
    pub fn new(value: f64) -> Option<Self> {
        value.is_finite().then_some(Self(value))
    }

    /// `value` limited to `-10..=10`; a NaN is `0`.
    #[must_use]
    pub fn clamped(value: f64) -> Self {
        if value.is_nan() {
            Self::ZERO
        } else {
            Self(value.clamp(-Self::RANGE, Self::RANGE))
        }
    }

    /// The stored value.
    #[must_use]
    pub const fn get(self) -> f64 {
        self.0
    }

    /// The value a shape is drawn with: the stored one limited to `-10..=10`.
    #[must_use]
    pub fn effective(self) -> f64 {
        self.0.clamp(-Self::RANGE, Self::RANGE)
    }

    /// Whether the shape is drawn with straight sides.
    #[must_use]
    pub fn is_zero(self) -> bool {
        self.0 == 0.0
    }
}

/// The value of `q` at which the ellipse's Rounded outline is exactly today's ellipse:
/// `KAPPA / sqrt(2)` (`adrs.md` decision 9, the side of the unit 4-ring is `sqrt(2)`).
pub const Q_C: f64 = crate::primitive_outline::KAPPA * std::f64::consts::FRAC_1_SQRT_2;

/// The Rounded value `shape` is drawn with: 0 for an ellipse with an arc (the arc wins), `None`
/// for a rectangle.
#[must_use]
pub fn effective_rounded(shape: &Shape) -> Option<f64> {
    match *shape {
        Shape::Polygon { rounded, .. }
        | Shape::Star { rounded, .. }
        | Shape::Ellipse {
            rounded, arc: None, ..
        } => Some(rounded.effective()),
        Shape::Ellipse { arc: Some(_), .. } => Some(0.0),
        Shape::Rect { .. } => None,
    }
}

/// The outline of `ring` rounded by `q` (already effective), in the frame without
/// `rotation`: one Symmetric node per vertex in the ring's order and one cubic per side. The
/// handles lie along the unit tangent `t_i` perpendicular to the radius at `V_i`, pointing
/// toward increasing index, with length `|q|·s` (`s` the side length): the out handle is
/// `+q·s·t_i`, the in handle `-q·s·t_i` (decision 1).
pub(crate) fn rounded_outline(ring: &Ring, q: f64) -> Vec<OutlineAnchor> {
    let n = ring.vertices.len();
    let side = if n >= 2 {
        (ring.vertices[1] - ring.vertices[0]).length()
    } else {
        0.0
    };
    let reach = q * side;
    ring.vertices
        .iter()
        .map(|&vertex| {
            let radius = vertex.length();
            let handle = if radius > 0.0 {
                // The ellipse's unit ring is scaled by (rx, ry) afterwards, handles included.
                Vec2::new(
                    -vertex.y / radius * reach * ring.scale.0,
                    vertex.x / radius * reach * ring.scale.1,
                )
            } else {
                Vec2::ZERO
            };
            OutlineAnchor {
                point: ring.place(vertex),
                handle_in: handle.negated(),
                handle_out: handle,
                kind: AnchorKind::Symmetric,
            }
        })
        .collect()
}

/// The exact box of the closed chain of cubics `anchors` (extrema, not control points).
pub(crate) fn anchors_bounds(anchors: &[OutlineAnchor]) -> Option<(Point, Point)> {
    let n = anchors.len();
    if n == 0 {
        return None;
    }
    let (mut min, mut max) = (
        Point::new(f64::INFINITY, f64::INFINITY),
        Point::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
    );
    for i in 0..n {
        let (a, b) = (anchors[i], anchors[(i + 1) % n]);
        let to_kurbo = |p: Point| kurbo::Point::new(p.x, p.y);
        let c = CubicBez::new(
            to_kurbo(a.point),
            to_kurbo(a.point.translated(a.handle_out)),
            to_kurbo(b.point.translated(b.handle_in)),
            to_kurbo(b.point),
        );
        let r = c.bounding_box();
        min = Point::new(min.x.min(r.x0), min.y.min(r.y0));
        max = Point::new(max.x.max(r.x1), max.y.max(r.y1));
    }
    Some((min, max))
}

/// The exact box of `shape`'s rounded or randomized outline as drawn (`Ring::of(shape, true)`: the frame's
/// angle included, the object's `rotation` excluded), `None` for a shape without a Rounded
/// value or with Rounded 0 (criterion 7, decision 5).
#[must_use]
pub fn rounded_bounds(shape: &Shape) -> Option<(Point, Point)> {
    // A randomized polygon or star draws moved vertices (`0145` slice 3, criterion 28).
    if let Some(bounds) = crate::ring_random::randomized_bounds(shape) {
        return Some(bounds);
    }
    let q = effective_rounded(shape).filter(|q| *q != 0.0)?;
    let ring = Ring::of(shape, true)?;
    anchors_bounds(&rounded_outline(&ring, q))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitive_model::{InnerRatio, PointCount, StarFrame};
    use crate::units::{Angle, Length};

    fn frame(angle: f64) -> StarFrame {
        StarFrame {
            center: Point::new(0.0, 0.0),
            radius: Length::from_mm(10.0),
            angle: Angle::from_radians(angle),
        }
    }

    fn polygon(n: u32, q: f64, angle: f64) -> Shape {
        Shape::Polygon {
            randomized: crate::Randomized::ZERO,
            frame: frame(angle),
            point_count: PointCount::new(n).unwrap(),
            curve: crate::Curve::POLYGON_DEFAULT,
            rounded: Rounded::new(q).unwrap(),
        }
    }

    fn star(n: u32, ratio: f64, q: f64) -> Shape {
        Shape::Star {
            randomized: crate::Randomized::ZERO,
            frame: frame(0.0),
            point_count: PointCount::new(n).unwrap(),
            inner_ratio: InnerRatio::new(ratio).unwrap(),
            curve: crate::Curve::POLYGON_DEFAULT,
            rounded: Rounded::new(q).unwrap(),
        }
    }

    fn outline(shape: &Shape) -> Vec<OutlineAnchor> {
        rounded_outline(
            &Ring::of(shape, true).unwrap(),
            effective_rounded(shape).unwrap(),
        )
    }

    /// Provisional golden (criterion 4, `adrs.md` decision 7): hand-computed from decision 1,
    /// not yet exported from Inkscape 1.4. TODO(ISSUE-NEEDED): replace by the fixture of
    /// `curvyo-ui-core/tests/fixtures/svg/inkscape_1_4_rounded.svg`.
    /// Hexagon R = 10 (side 10), q = 0.3: every handle is 3 mm long along the tangent.
    #[test]
    fn hexagon_handles_are_q_times_the_side_along_the_tangent() {
        let anchors = outline(&polygon(6, 0.3, 0.0));
        assert_eq!(anchors.len(), 6);
        let a0 = anchors[0];
        assert!((a0.point.x - 10.0).abs() < 1e-9 && a0.point.y.abs() < 1e-9);
        assert!(a0.handle_out.x.abs() < 1e-9 && (a0.handle_out.y - 3.0).abs() < 1e-9);
        assert!(a0.handle_in.x.abs() < 1e-9 && (a0.handle_in.y + 3.0).abs() < 1e-9);
        // Vertex 1 at 60°: tangent (-sin60, cos60).
        let a1 = anchors[1];
        let h = a1.handle_out;
        assert!((h.x + 3.0 * 60_f64.to_radians().sin()).abs() < 1e-9);
        assert!((h.y - 3.0 * 60_f64.to_radians().cos()).abs() < 1e-9);
        assert!(anchors.iter().all(|a| a.kind == AnchorKind::Symmetric));
    }

    /// Provisional golden (see above): the 5-point star at Ratio 0.5, q = -1: both kinds of
    /// vertex get the same length `|q|·s`, `s` the outer-to-inner distance, the sign flipped.
    #[test]
    fn star_vertices_share_one_handle_length_and_a_negative_q_flips() {
        let shape = star(5, 0.5, -1.0);
        let anchors = outline(&shape);
        assert_eq!(anchors.len(), 10);
        let s = (anchors[0].point.vector_to(anchors[1].point)).length();
        for a in &anchors {
            assert!((a.handle_out.length() - s).abs() < 1e-9);
        }
        // q < 0: the out handle of vertex 0 points toward decreasing angle (-y at angle 0).
        assert!(anchors[0].handle_out.y < 0.0);
    }

    /// Criterion 4: q = 0 gives handles of length 0 (the straight shape's cubics are lines).
    #[test]
    fn zero_has_no_handles() {
        for a in outline(&polygon(5, 0.0, 0.3)) {
            assert_eq!(a.handle_out, Vec2::ZERO);
        }
    }

    /// Criterion 4: turning the shape by 360/N degrees gives the same outline up to a cyclic
    /// shift of the start node; every vertex lies on the outline (it is a node).
    #[test]
    fn a_turn_by_one_step_is_a_cyclic_shift() {
        for (n, q) in [(5_u32, 1.0), (6, -0.3), (7, 3.0)] {
            let step = std::f64::consts::TAU / f64::from(n);
            let a = outline(&polygon(n, q, 0.0));
            let b = outline(&polygon(n, q, step));
            for i in 0..a.len() {
                let (x, y) = (&a[(i + 1) % a.len()], &b[i]);
                assert!(x.point.vector_to(y.point).length() < 1e-9);
                assert!((x.handle_out - y.handle_out).length() < 1e-9);
                assert!((x.handle_in - y.handle_in).length() < 1e-9);
            }
        }
    }

    /// Criterion 7: the exact box contains 4096 sampled outline points and touches the extreme
    /// ones.
    #[test]
    fn the_bounds_are_tight_around_sampled_points() {
        for q in [-3.0, 0.3, 2.5] {
            let shape = star(5, 0.4, q);
            let anchors = outline(&shape);
            let (min, max) = rounded_bounds(&shape).unwrap();
            let n = anchors.len();
            let (mut lo, mut hi) = (
                Point::new(f64::INFINITY, f64::INFINITY),
                Point::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
            );
            for i in 0..n {
                let (a, b) = (anchors[i], anchors[(i + 1) % n]);
                let (p0, p1, p2, p3) = (
                    a.point,
                    a.point.translated(a.handle_out),
                    b.point.translated(b.handle_in),
                    b.point,
                );
                for k in 0..=409 {
                    let t = f64::from(k) / 409.0;
                    let u = 1.0 - t;
                    let at = |f: fn(Point) -> f64| {
                        u * u * u * f(p0)
                            + 3.0 * u * u * t * f(p1)
                            + 3.0 * u * t * t * f(p2)
                            + t * t * t * f(p3)
                    };
                    let (x, y) = (at(|p| p.x), at(|p| p.y));
                    lo = Point::new(lo.x.min(x), lo.y.min(y));
                    hi = Point::new(hi.x.max(x), hi.y.max(y));
                    assert!(x >= min.x - 1e-9 && x <= max.x + 1e-9);
                    assert!(y >= min.y - 1e-9 && y <= max.y + 1e-9);
                }
            }
            assert!(
                (lo.x - min.x).abs() < 1e-3 && (hi.y - max.y).abs() < 1e-3,
                "q {q}"
            );
        }
        assert!(rounded_bounds(&polygon(5, 0.0, 0.0)).is_none());
    }

    /// Criterion 13: 1000 seeded combinations are finite and closed.
    #[test]
    fn a_thousand_seeded_combinations_are_finite() {
        let mut state = 0x9E37_79B9_7F4A_7C15_u64;
        let mut next = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            // 53 random bits to [0, 1): exact in f64.
            #[allow(clippy::cast_precision_loss)]
            {
                (state >> 11) as f64 / (1_u64 << 53) as f64
            }
        };
        for _ in 0..1000 {
            // In 0..=1020 by construction.
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let n = 3 + (next() * 1021.0) as u32;
            let ratio = 0.01 + next() * 0.98;
            let q = -10.0 + next() * 20.0;
            for shape in [polygon(n, q, next()), star(n, ratio, q)] {
                let anchors = outline(&shape);
                assert!(anchors.len() >= 3);
                for a in &anchors {
                    assert!(a.point.x.is_finite() && a.point.y.is_finite());
                    assert!(a.handle_in.x.is_finite() && a.handle_out.y.is_finite());
                }
                let (min, max) = rounded_bounds(&shape).unwrap();
                assert!(min.x.is_finite() && max.y.is_finite() && min.x <= max.x);
            }
        }
    }

    #[test]
    fn the_value_is_stored_raw_and_evaluated_limited() {
        assert!(Rounded::new(f64::NAN).is_none() && Rounded::new(f64::INFINITY).is_none());
        let big = Rounded::new(12.4).unwrap();
        assert_eq!((big.get(), big.effective()), (12.4, 10.0));
        assert_eq!(Rounded::clamped(-99.0).get(), -10.0);
        assert_eq!(Rounded::clamped(f64::NAN), Rounded::ZERO);
    }
}
