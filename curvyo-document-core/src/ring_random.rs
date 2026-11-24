//! The seeded outline of a randomized polygon or star, from its stored Randomized value
//! (`specs/0145-shape-spiro-handles/adrs.md` decision 8, criteria 17 and 21 to 30). Pure
//! integer mixing and `+ - * /`: the same ring on every platform.

use serde::Serialize;

use crate::path_model::AnchorKind;
use crate::primitive_model::Shape;
use crate::primitive_outline::OutlineAnchor;
use crate::ring_curve::{Ring, effective_curve, ring_outline};
use crate::ring_rounded::{anchors_bounds, effective_rounded, rounded_outline};
use crate::seed::{Seed, splitmix64};
use crate::units::{Angle, Point, Vec2};

/// A polygon's or star's Randomized value `r` and the seed its draws come from (criterion 17):
/// `r = 0` the shape as drawn without it, whatever the seed. Stored raw and finite; evaluated
/// limited to `-10..=10` (criterion 26). The seed is kept while `r` is 0, so going back to a
/// value brings the same shape.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Randomized {
    amount: f64,
    seed: Seed,
}

impl Randomized {
    /// No randomizing: the value a polygon or star has when none is stored.
    pub const ZERO: Self = Self {
        amount: 0.0,
        seed: Seed::new(0),
    };
    /// The largest magnitude a value is evaluated with (Inkscape's field range).
    pub const RANGE: f64 = 10.0;

    /// `amount` as stored beside `seed`, refused when it is not finite (a file's value is
    /// `Damaged`).
    #[must_use]
    pub fn new(amount: f64, seed: Seed) -> Option<Self> {
        amount.is_finite().then_some(Self { amount, seed })
    }

    /// This value with `amount` limited to `-10..=10` (a NaN is `0`), the seed kept.
    #[must_use]
    pub fn with_clamped_amount(self, amount: f64) -> Self {
        Self {
            amount: if amount.is_nan() {
                0.0
            } else {
                amount.clamp(-Self::RANGE, Self::RANGE)
            },
            seed: self.seed,
        }
    }

    /// This value with another seed.
    #[must_use]
    pub const fn with_seed(self, seed: Seed) -> Self {
        Self { seed, ..self }
    }

    /// The stored amount.
    #[must_use]
    pub const fn amount(self) -> f64 {
        self.amount
    }

    /// The seed.
    #[must_use]
    pub const fn seed(self) -> Seed {
        self.seed
    }

    /// The amount a shape is drawn with: the stored one limited to `-10..=10`.
    #[must_use]
    pub fn effective(self) -> f64 {
        self.amount.clamp(-Self::RANGE, Self::RANGE)
    }

    /// Whether the shape is drawn as without randomizing.
    #[must_use]
    pub fn is_zero(self) -> bool {
        Self::is_unset(&self)
    }

    /// [`Randomized::is_zero`] by reference, for serde: a shape without randomizing serializes
    /// as before.
    #[must_use]
    pub fn is_unset(&self) -> bool {
        self.amount == 0.0
    }
}

/// The Randomized value of a polygon or star that has one that is not 0, `None` otherwise.
fn active(shape: &Shape) -> Option<Randomized> {
    match *shape {
        Shape::Polygon { randomized, .. } | Shape::Star { randomized, .. }
            if !randomized.is_zero() =>
        {
            Some(randomized)
        }
        _ => None,
    }
}

/// Draw `k` (0 to 3) of vertex `index`: `splitmix64(seed ^ splitmix64(4 * index + k))`, its top 53
/// bits as a number in `[-0.5, 0.5)` (decision 8).
#[must_use]
pub fn draw(seed: Seed, index: usize, k: u64) -> f64 {
    // A ring holds at most 2048 vertices: the index fits.
    let mixed = splitmix64(u64::from(seed.get()) ^ splitmix64(4 * index as u64 + k));
    // 53 bits are exact in f64.
    #[allow(clippy::cast_precision_loss)]
    let unit = (mixed >> 11) as f64 / 9_007_199_254_740_992.0;
    unit - 0.5
}

/// The ring of `shape` with every vertex moved by `r * 2 * R_max * (d1, d2)` (criterion 24), in
/// the frame's own axes, together with `r`, the seed and the unmoved ring.
fn moved_vertices(ring: &Ring, random: Randomized) -> Vec<Vec2> {
    let r = random.effective();
    let r_max = ring
        .vertices
        .iter()
        .map(|v| v.length())
        .fold(0.0_f64, f64::max);
    ring.vertices
        .iter()
        .enumerate()
        .map(|(i, &v)| {
            v + Vec2::new(
                r * 2.0 * r_max * draw(random.seed(), i, 0),
                r * 2.0 * r_max * draw(random.seed(), i, 1),
            )
        })
        .collect()
}

/// The outline of a polygon or star with a Randomized value that is not 0, in the frame without
/// the object's `rotation` (criteria 23, 24, 27): the Rounded outline through the moved vertices
/// with the handle of each taken from the unshifted ring, turned by `r * pi * d3` and scaled by
/// `1 + r * d4`; at Rounded 0 the sides (straight or Curve arcs, cut into `piece`) run between the
/// moved vertices. `None` for any other shape.
pub(crate) fn randomized_outline(shape: &Shape, piece: f64) -> Option<Vec<OutlineAnchor>> {
    let random = active(shape)?;
    let ring = Ring::of(shape, true)?;
    let moved = moved_vertices(&ring, random);
    let q = effective_rounded(shape).unwrap_or(0.0);
    if q == 0.0 {
        let k = effective_curve(shape).map_or(0.0, crate::ring_curve::Curve::get);
        let moved_ring = Ring {
            vertices: moved,
            ..ring
        };
        return Some(ring_outline(&moved_ring, k, piece));
    }
    let r = random.effective();
    Some(
        rounded_outline(&ring, q)
            .into_iter()
            .zip(moved)
            .enumerate()
            .map(|(i, (anchor, vertex))| {
                let turned = anchor
                    .handle_out
                    .rotated(Angle::from_radians(
                        r * std::f64::consts::PI * draw(random.seed(), i, 2),
                    ))
                    .scaled(1.0 + r * draw(random.seed(), i, 3));
                OutlineAnchor {
                    point: ring.place(vertex),
                    handle_in: turned.negated(),
                    handle_out: turned,
                    kind: AnchorKind::Symmetric,
                }
            })
            .collect(),
    )
}

/// The exact box of the randomized outline as drawn (criterion 28), `None` while `shape` has no
/// Randomized value.
pub(crate) fn randomized_bounds(shape: &Shape) -> Option<(Point, Point)> {
    let anchors = randomized_outline(shape, crate::ellipse_arc::DRAWING_PIECE_RAD)?;
    anchors_bounds(&anchors)
}

#[cfg(test)]
#[allow(clippy::unreadable_literal)] // the pinned digits are copied exactly from the script
mod tests {
    use super::*;
    use crate::primitive_model::{InnerRatio, PointCount, StarFrame};
    use crate::ring_rounded::Rounded;
    use crate::units::Length;

    fn frame(angle: f64) -> StarFrame {
        StarFrame {
            center: Point::new(0.0, 0.0),
            radius: Length::from_mm(10.0),
            angle: Angle::from_radians(angle),
        }
    }

    fn star(n: u32, r: f64, seed: u32, q: f64) -> Shape {
        Shape::Star {
            frame: frame(0.0),
            point_count: PointCount::new(n).unwrap(),
            inner_ratio: InnerRatio::new(0.5).unwrap(),
            curve: crate::Curve::POLYGON_DEFAULT,
            rounded: Rounded::new(q).unwrap(),
            randomized: Randomized::new(r, Seed::new(seed)).unwrap(),
        }
    }

    fn polygon(n: u32, r: f64, seed: u32) -> Shape {
        Shape::Polygon {
            frame: frame(0.0),
            point_count: PointCount::new(n).unwrap(),
            curve: crate::Curve::POLYGON_DEFAULT,
            rounded: Rounded::ZERO,
            randomized: Randomized::new(r, Seed::new(seed)).unwrap(),
        }
    }

    fn outline(shape: &Shape) -> Vec<OutlineAnchor> {
        randomized_outline(shape, std::f64::consts::FRAC_PI_2).unwrap()
    }

    /// Criterion 23: the first three draws of seed 1 for vertex 0 (values computed with an
    /// independent implementation of the published `SplitMix64`, whose first output for state 0
    /// is `0xE220A8397B1DCDAF`), and the same on every call.
    #[test]
    fn the_first_draws_of_seed_one_are_pinned() {
        assert_eq!(splitmix64(0), 0xE220_A839_7B1D_CDAF);
        let seed = Seed::new(1);
        let expected = [
            -0.465_988_298_695_653_6,
            0.414_022_462_870_302_87,
            0.376_675_621_419_550_35,
        ];
        for (k, want) in (0..3_u64).zip(expected) {
            assert!((draw(seed, 0, k) - want).abs() < 1e-15, "draw {k}");
            assert_eq!(draw(seed, 0, k), draw(seed, 0, k));
        }
    }

    /// Criterion 23: the ring of a 5-point star at `r = 1`, seed 1 (radius 10, Ratio 0.5),
    /// computed with the independent script; a different seed gives a different ring.
    #[test]
    fn the_ring_of_a_five_point_star_is_pinned() {
        let anchors = outline(&star(5, 1.0, 1, 0.0));
        let want = [
            (0.6802340260869286, 8.280449257406058),
            (8.554930167155767, 10.554717916099197),
            (-6.43017980018533, 7.21376524236058),
            (-1.9249925623685655, 11.990636804898099),
            (-1.3540223197269494, 5.861247258577017),
            (-2.6611587395094727, -8.570167145441301),
            (-6.79513568204999, 1.6836049019061052),
            (8.252378649631899, 2.860146194348439),
            (2.0779424097567167, -6.520444970265052),
            (13.178186010088572, -2.7315620757166093),
        ];
        assert_eq!(anchors.len(), 10);
        for (i, (x, y)) in want.into_iter().enumerate() {
            assert!((anchors[i].point.x - x).abs() < 1e-9, "x of vertex {i}");
            assert!((anchors[i].point.y - y).abs() < 1e-9, "y of vertex {i}");
        }
        let other = outline(&star(5, 1.0, 2, 0.0));
        assert!(
            anchors
                .iter()
                .zip(&other)
                .any(|(a, b)| a.point.vector_to(b.point).length() > 1e-6)
        );
    }

    /// Criterion 27: the handle of vertex 0 at `q = 1`, `r = 0.5`, seed 1 is the unshifted
    /// handle (0, s) turned by `r*pi*d3` and scaled by `1 + r*d4`, and the vertex is on the
    /// outline (it is a node).
    #[test]
    fn rounded_handles_come_from_the_unshifted_ring_turned_and_scaled() {
        let anchors = outline(&star(5, 0.5, 1, 1.0));
        let a = anchors[0];
        assert!((a.handle_out.x + 4.487_313_399_461_994_5).abs() < 1e-9);
        assert!((a.handle_out.y - 6.677_623_923_381_786).abs() < 1e-9);
        assert!((a.handle_in + a.handle_out).length() < 1e-12);
        assert!((a.point.x - 5.340_117_013_043_464).abs() < 1e-9);
        assert!((a.point.y - 4.140_224_628_703_029).abs() < 1e-9);
    }

    /// Criterion 24: no vertex moves further than `|r| * R_max` along either axis, and the shift
    /// at `r = 2` is twice that at `r = 1` (within 1e-9).
    #[test]
    fn the_shift_is_bounded_and_linear_in_r() {
        for n in [3_u32, 7, 40] {
            let plain = Ring::of(&polygon(n, 0.0, 9), true).unwrap();
            let one = outline(&polygon(n, 1.0, 9));
            let two = outline(&polygon(n, 2.0, 9));
            let r_max = 10.0;
            for i in 0..plain.vertices.len() {
                let home = plain.place(plain.vertices[i]);
                let s1 = home.vector_to(one[i].point);
                let s2 = home.vector_to(two[i].point);
                assert!(s1.x.abs() <= r_max + 1e-9 && s1.y.abs() <= r_max + 1e-9);
                assert!((s2.x - 2.0 * s1.x).abs() < 1e-9 && (s2.y - 2.0 * s1.y).abs() < 1e-9);
            }
        }
    }

    /// Criterion 25: `r = 0` has no randomized outline at all (the plain shape draws), whatever
    /// the seed.
    #[test]
    fn zero_is_not_randomized() {
        assert!(randomized_outline(&polygon(5, 0.0, 12345), 1.0).is_none());
        assert!(randomized_bounds(&polygon(5, 0.0, 7)).is_none());
    }

    /// Criterion 27: with Curve the arcs run between the moved vertices: every moved vertex is a
    /// node of the outline.
    #[test]
    fn curve_arcs_run_between_the_moved_vertices() {
        let shape = Shape::Polygon {
            frame: frame(0.0),
            point_count: PointCount::new(6).unwrap(),
            curve: crate::Curve::new(0.2).unwrap(),
            rounded: Rounded::ZERO,
            randomized: Randomized::new(0.3, Seed::new(5)).unwrap(),
        };
        let plain = Ring::of(&shape, true).unwrap();
        let moved = moved_vertices(&plain, active(&shape).unwrap());
        let anchors = outline(&shape);
        for v in moved {
            let p = plain.place(v);
            assert!(
                anchors.iter().any(|a| a.point.vector_to(p).length() < 1e-9),
                "{p:?}"
            );
        }
    }
}
