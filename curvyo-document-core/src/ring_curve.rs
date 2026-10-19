//! Evaluation of a curved ring of vertices (`specs/0021-ellipse-arcs-and-shaping/`
//! criteria 17 to 21 and 35 to 51, `adrs.md` decision 4): the side arcs, their middles,
//! the curve limit and the cubic control points, for the ellipse (a 4-ring scaled by its
//! radii), the polygon and the star alike, so the three agree by construction
//! (criterion 51). Pure arithmetic on the shape's parameters; no Loro type.

// The formulas of the specification name their quantities by single letters (k, c, θ,
// v, w, …); the code keeps those names so it reads against the criteria.
#![allow(clippy::many_single_char_names, clippy::similar_names)]

use serde::Serialize;

use crate::path_model::AnchorKind;
use crate::primitive_model::{InnerRatio, PointCount, Shape};
use crate::primitive_outline::OutlineAnchor;
use crate::units::{Point, Vec2};

/// A shape's curve (criteria 17 and 35): the fraction `k` from -1 to 1 by which every
/// side bends, `1` the ellipse (the polygon's circumscribed circle), `0` straight sides,
/// `-1` concave sides with cusps. Stored raw; a polygon or star draws the effective
/// value of [`effective_curve`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Curve(f64);

impl Curve {
    /// The curve an ellipse has when none is stored: the ellipse itself.
    pub const ELLIPSE_DEFAULT: Self = Self(1.0);
    /// The curve a polygon or star has when none is stored: straight sides.
    pub const POLYGON_DEFAULT: Self = Self(0.0);

    /// Validates `fraction` against `-1..=1`, finite (a file's value is refused, never
    /// clamped, criteria 33 and 50).
    #[must_use]
    pub fn new(fraction: f64) -> Option<Self> {
        (fraction.is_finite() && (-1.0..=1.0).contains(&fraction)).then_some(Self(fraction))
    }

    /// `fraction` limited to `-1..=1`; a NaN is the straight side `0`.
    #[must_use]
    pub fn clamped(fraction: f64) -> Self {
        if fraction.is_nan() {
            Self(0.0)
        } else {
            Self(fraction.clamp(-1.0, 1.0))
        }
    }

    /// The fraction, -1 to 1.
    #[must_use]
    pub const fn get(self) -> f64 {
        self.0
    }

    /// The value in percent, -100 to 100, as the bar and the readout show it.
    #[must_use]
    pub fn percent(self) -> f64 {
        self.0 * 100.0
    }
}

/// A ring of vertices around a centre, in the shape's frame without its `rotation`
/// (vertex 0 at `angle0` from the centre's +x axis, increasing angle clockwise on
/// screen), and the per-axis scale the ellipse puts on its unit ring.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Ring {
    /// The vertices relative to the centre, before `scale`.
    pub(crate) vertices: Vec<Vec2>,
    /// The centre.
    pub(crate) center: Point,
    /// `(rx, ry)` for the ellipse's unit ring, `(1, 1)` for a polygon or star.
    pub(crate) scale: (f64, f64),
}

impl Ring {
    /// The ring of `shape` with vertex 0 at `angle0` (`None` for a rectangle). An ellipse
    /// is the unit 4-ring E, S, W, N scaled by `rx`, `ry`, whatever `angle0`.
    pub(crate) fn of(shape: &Shape, with_frame_angle: bool) -> Option<Self> {
        match *shape {
            Shape::Ellipse { frame, .. } => Some(Self {
                vertices: vec![
                    Vec2::new(1.0, 0.0),
                    Vec2::new(0.0, 1.0),
                    Vec2::new(-1.0, 0.0),
                    Vec2::new(0.0, -1.0),
                ],
                center: frame.center,
                scale: (frame.rx.as_mm(), frame.ry.as_mm()),
            }),
            Shape::Polygon {
                frame, point_count, ..
            } => Some(Self {
                vertices: polygon_vertices(
                    frame.radius.as_mm(),
                    if with_frame_angle {
                        frame.angle.as_radians()
                    } else {
                        0.0
                    },
                    point_count,
                ),
                center: frame.center,
                scale: (1.0, 1.0),
            }),
            Shape::Star {
                frame,
                point_count,
                inner_ratio,
                ..
            } => Some(Self {
                vertices: star_vertices(
                    frame.radius.as_mm(),
                    if with_frame_angle {
                        frame.angle.as_radians()
                    } else {
                        0.0
                    },
                    point_count,
                    inner_ratio,
                ),
                center: frame.center,
                scale: (1.0, 1.0),
            }),
            Shape::Rect { .. } => None,
        }
    }

    /// The full angle `φ = π / M` (criterion 36's terms).
    pub(crate) fn full_angle(&self) -> f64 {
        // A ring holds at most 2048 vertices: exact in f64.
        #[allow(clippy::cast_precision_loss)]
        let m = self.vertices.len() as f64;
        std::f64::consts::PI / m
    }

    /// A ring-local point (before the scale) in the frame: scaled and placed.
    pub(crate) fn place(&self, local: Vec2) -> Point {
        self.center
            .translated(Vec2::new(local.x * self.scale.0, local.y * self.scale.1))
    }

    /// The interior angle (radians) at every vertex of the straight ring.
    fn interior_angles(&self) -> Vec<f64> {
        let n = self.vertices.len();
        let turns: Vec<f64> = (0..n)
            .map(|i| {
                let prev = self.vertices[(i + n - 1) % n];
                let here = self.vertices[i];
                let next = self.vertices[(i + 1) % n];
                let a = here - prev;
                let b = next - here;
                (a.x * b.y - a.y * b.x).atan2(a.x * b.x + a.y * b.y)
            })
            .collect();
        let sign = turns.iter().sum::<f64>().signum();
        turns
            .into_iter()
            .map(|turn| std::f64::consts::PI - sign * turn)
            .collect()
    }

    /// The lowest allowed curve `k_min = max(-1, -A / (2φ))` (criterion 38).
    pub(crate) fn curve_limit(&self) -> f64 {
        let smallest = self
            .interior_angles()
            .into_iter()
            .fold(f64::INFINITY, f64::min);
        (-smallest / (2.0 * self.full_angle())).max(-1.0)
    }

    /// The side from vertex `i` to vertex `i + 1`, before the scale.
    fn side(&self, i: usize) -> Side {
        let n = self.vertices.len();
        let v = self.vertices[i];
        let w = self.vertices[(i + 1) % n];
        let chord = w - v;
        let c = chord.length();
        let u = if c > 0.0 {
            chord.scaled(1.0 / c)
        } else {
            Vec2::ZERO
        };
        let mut normal = Vec2::new(-u.y, u.x);
        let middle = Vec2::new(f64::midpoint(v.x, w.x), f64::midpoint(v.y, w.y));
        // Outward: away from the centre (the ring is star-shaped about it).
        if normal.x * middle.x + normal.y * middle.y < 0.0 {
            normal = normal.negated();
        }
        Side {
            v,
            w,
            c,
            normal,
            middle,
        }
    }
}

/// One straight side of a ring: its ends, chord length, outward normal and middle,
/// before the scale.
#[derive(Debug, Clone, Copy)]
struct Side {
    v: Vec2,
    w: Vec2,
    c: f64,
    normal: Vec2,
    middle: Vec2,
}

/// The polygon's vertices at `radius`, vertex 0 at `angle0`.
fn polygon_vertices(radius: f64, angle0: f64, point_count: PointCount) -> Vec<Vec2> {
    let step = std::f64::consts::TAU / f64::from(point_count.get());
    (0..point_count.get())
        .map(|k| {
            let theta = angle0 + step * f64::from(k);
            Vec2::new(radius * theta.cos(), radius * theta.sin())
        })
        .collect()
}

/// The star's vertices, outer and inner alternating, vertex 0 the first outer one.
fn star_vertices(
    radius: f64,
    angle0: f64,
    point_count: PointCount,
    inner_ratio: InnerRatio,
) -> Vec<Vec2> {
    let step = std::f64::consts::TAU / f64::from(point_count.get());
    let inner = radius * inner_ratio.get();
    let mut vertices = Vec::with_capacity(2 * point_count.get() as usize);
    for k in 0..point_count.get() {
        let outer_theta = angle0 + step * f64::from(k);
        vertices.push(Vec2::new(
            radius * outer_theta.cos(),
            radius * outer_theta.sin(),
        ));
        let inner_theta = outer_theta + step / 2.0;
        vertices.push(Vec2::new(
            inner * inner_theta.cos(),
            inner * inner_theta.sin(),
        ));
    }
    vertices
}

/// The curve `shape` is drawn with: an ellipse's own value (`1` while it has an arc, the
/// arc wins, `adrs.md` decision 2), a polygon's or star's stored value raised to its
/// limit (criterion 38). `None` for a rectangle.
#[must_use]
pub fn effective_curve(shape: &Shape) -> Option<Curve> {
    match *shape {
        Shape::Ellipse { arc: Some(_), .. } => Some(Curve::ELLIPSE_DEFAULT),
        Shape::Ellipse { curve, .. } => Some(curve),
        Shape::Polygon { curve, .. } | Shape::Star { curve, .. } => {
            let limit = curve_limit(shape)?;
            Some(Curve(curve.get().max(limit.get())))
        }
        Shape::Rect { .. } => None,
    }
}

/// The lowest curve `shape` allows (criterion 38): `-1` for an ellipse and every polygon
/// from 4 sides, `-0.5` for a triangle, a star's own value. `None` for a rectangle.
#[must_use]
pub fn curve_limit(shape: &Shape) -> Option<Curve> {
    match shape {
        Shape::Ellipse { .. } => Some(Curve(-1.0)),
        Shape::Rect { .. } => None,
        Shape::Polygon { .. } | Shape::Star { .. } => {
            Ring::of(shape, false).map(|ring| Curve::clamped(ring.curve_limit()))
        }
    }
}

/// The largest distance of `shape`'s drawn outline from its centre, before an ellipse's
/// scale (`0` for a rectangle and for straight sides, whose extent is the outer radius): a vertex, or the point of a side's circular arc farthest
/// from the centre when the arc's span reaches it.
#[must_use]
pub fn ring_extent(shape: &Shape) -> f64 {
    let (Some(ring), Some(k)) = (Ring::of(shape, false), effective_curve(shape)) else {
        return 0.0;
    };
    let theta = k.get().abs() * ring.full_angle();
    // Straight sides: the vertices, whose farthest is the outer radius itself.
    if theta <= 0.0 {
        return 0.0;
    }
    let mut extent = ring.vertices.iter().map(|v| v.length()).fold(0.0, f64::max);
    for i in 0..ring.vertices.len() {
        let side = ring.side(i);
        if side.c <= 0.0 {
            continue;
        }
        let bulge = side.normal.scaled(k.get().signum());
        let radius = side.c / (2.0 * theta.sin());
        let center = side.middle - bulge.scaled(radius * theta.cos());
        let away = center.length();
        if away <= f64::EPSILON {
            extent = extent.max(radius);
            continue;
        }
        let far = center + center.scaled(radius / away);
        // On the arc when it lies on the bulging side of the chord, within the arc's span.
        let from_middle = far - side.middle;
        let along = from_middle.x * bulge.x + from_middle.y * bulge.y;
        let across = (from_middle.x * side.normal.y - from_middle.y * side.normal.x).abs();
        if along >= 0.0 && across <= side.c / 2.0 + 1e-12 {
            extent = extent.max(far.length());
        }
    }
    extent
}

/// The curve handle's position in the shape's box-local frame (no `rotation`, vertex 0
/// at angle 0 from the centre): the middle of the side counter-clockwise of vertex 0 at
/// the effective curve (criteria 19, 22, 37, 40). `None` for a rectangle.
#[must_use]
pub fn curve_handle_position(shape: &Shape) -> Option<Point> {
    let ring = Ring::of(shape, false)?;
    let k = effective_curve(shape)?.get();
    let side = ring.side(ring.vertices.len() - 1);
    let sagitta = side.c / 2.0 * (k * ring.full_angle() / 2.0).tan();
    Some(ring.place(side.middle + side.normal.scaled(sagitta)))
}

/// The curve a pointer at `local` (box-local frame, see [`curve_handle_position`]) asks
/// for on `shape` (criteria 23 and 41): the pointer is projected onto the perpendicular
/// bisector of the handle's side (in the ellipse's unit frame), and
/// `k = 2·atan(2t / c) / φ` from its signed distance `t`. Not limited; `None` for a
/// rectangle or a degenerate shape.
#[must_use]
pub fn curve_from_position(shape: &Shape, local: Point) -> Option<f64> {
    let ring = Ring::of(shape, false)?;
    let (sx, sy) = ring.scale;
    if !(sx.abs() > f64::EPSILON && sy.abs() > f64::EPSILON) {
        return None;
    }
    let side = ring.side(ring.vertices.len() - 1);
    if side.c <= f64::EPSILON {
        return None;
    }
    let offset = ring.center.vector_to(local);
    let unit = Vec2::new(offset.x / sx, offset.y / sy);
    let from_middle = unit - side.middle;
    let t = from_middle.x * side.normal.x + from_middle.y * side.normal.y;
    let k = 2.0 * (2.0 * t / side.c).atan() / ring.full_angle();
    k.is_finite().then_some(k)
}

/// The largest arc one cubic of "Object to path" covers (criterion 48: a side above
/// 90° is split).
pub(crate) const CONVERSION_PIECE_RAD: f64 = std::f64::consts::FRAC_PI_2;

/// The outline of `ring` at curve `k` (already effective), in the frame without
/// `rotation` (criteria 29 and 48): one node per vertex, in today's order and direction,
/// one cubic per side, a side whose arc subtends more than `piece` split into equal arcs
/// by Symmetric nodes ([`CONVERSION_PIECE_RAD`] for "Object to path", a finer piece for
/// drawing, criterion 67). A vertex node is Symmetric where its two tangents are
/// collinear (interior angle 180° within 0.01°), Corner elsewhere; at `k = 0` every side
/// is straight.
pub(crate) fn ring_outline(ring: &Ring, k: f64, piece: f64) -> Vec<OutlineAnchor> {
    let n = ring.vertices.len();
    let phi = ring.full_angle();
    let theta = k.abs() * phi;
    let interior = ring.interior_angles();
    let pieces: Vec<Vec<[Vec2; 4]>> = (0..n)
        .map(|i| side_cubics(ring.side(i), k.signum(), theta, piece))
        .collect();
    let scale = |v: Vec2| Vec2::new(v.x * ring.scale.0, v.y * ring.scale.1);
    let mut anchors: Vec<OutlineAnchor> = Vec::with_capacity(2 * n);
    for (i, side) in pieces.iter().enumerate() {
        let angle = interior[i] + 2.0 * k * phi;
        let smooth = (angle - std::f64::consts::PI).abs() <= 0.01_f64.to_radians();
        let previous = &pieces[(i + n - 1) % n];
        // invariant: every side has at least one piece.
        #[allow(clippy::unwrap_used)]
        let last = previous.last().unwrap();
        let first = side[0];
        anchors.push(OutlineAnchor {
            point: ring.place(first[0]),
            handle_in: scale(last[2] - last[3]),
            handle_out: scale(first[1] - first[0]),
            kind: if smooth {
                AnchorKind::Symmetric
            } else {
                AnchorKind::Corner
            },
        });
        for pair in side.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            anchors.push(OutlineAnchor {
                point: ring.place(b[0]),
                handle_in: scale(a[2] - a[3]),
                handle_out: scale(b[1] - b[0]),
                kind: AnchorKind::Symmetric,
            });
        }
    }
    anchors
}

/// The cubics of one side, before the scale: the straight side at `k = 0`, else its
/// circular arc of tangent angle `theta`, bulging by `sign` along the outward normal,
/// split into `ceil(2θ / piece)` equal arcs.
fn side_cubics(side: Side, sign: f64, theta: f64, piece: f64) -> Vec<[Vec2; 4]> {
    if sign == 0.0 || theta <= 0.0 || side.c <= 0.0 {
        return vec![[side.v, side.v, side.w, side.w]];
    }
    let bulge = side.normal.scaled(sign);
    // At most 2θ/piece ≤ π/(π/16) pieces: a small positive count.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let count = ((2.0 * theta / piece - 1e-9).ceil().max(1.0)) as usize;
    if count == 1 {
        return vec![arc_cubic(side.v, side.w, theta, bulge)];
    }
    // The arc's circle: radius c / (2 sin θ), centre behind the chord's middle.
    let radius = side.c / (2.0 * theta.sin());
    let center = side.middle - bulge.scaled(radius * theta.cos());
    let start = side.v - center;
    let end = side.w - center;
    let turn = (start.x * end.y - start.y * end.x).signum();
    #[allow(clippy::cast_precision_loss)]
    let step = 2.0 * theta / count as f64;
    let points: Vec<Vec2> = (0..=count)
        .map(|j| {
            if j == count {
                return side.w;
            }
            #[allow(clippy::cast_precision_loss)]
            let (sin, cos) = (turn * step * j as f64).sin_cos();
            center + Vec2::new(start.x * cos - start.y * sin, start.x * sin + start.y * cos)
        })
        .collect();
    points
        .windows(2)
        .map(|pair| {
            arc_cubic(
                pair[0],
                pair[1],
                step / 2.0,
                bulge_of(pair[0], pair[1], center),
            )
        })
        .collect()
}

/// The unit normal of chord `a b` that points away from `center`.
fn bulge_of(a: Vec2, b: Vec2, center: Vec2) -> Vec2 {
    let chord = b - a;
    let length = chord.length();
    let mut normal = Vec2::new(-chord.y / length, chord.x / length);
    let middle = Vec2::new(f64::midpoint(a.x, b.x), f64::midpoint(a.y, b.y)) - center;
    if normal.x * middle.x + normal.y * middle.y < 0.0 {
        normal = normal.negated();
    }
    normal
}

/// The cubic of the circular arc from `v` to `w` whose tangent at each end makes the
/// angle `theta` with the chord, bulging towards `bulge` (a unit normal of the chord):
/// control points `v + L·d` and `w + L·d'` with `L = (c/3) / cos²(θ/2)` (criterion 48).
fn arc_cubic(v: Vec2, w: Vec2, theta: f64, bulge: Vec2) -> [Vec2; 4] {
    let chord = w - v;
    let c = chord.length();
    let u = chord.scaled(1.0 / c);
    let (sin, cos) = theta.sin_cos();
    let length = c / 3.0 / (theta / 2.0).cos().powi(2);
    let d = u.scaled(cos) + bulge.scaled(sin);
    let d_end = u.scaled(-cos) + bulge.scaled(sin);
    [v, v + d.scaled(length), w + d_end.scaled(length), w]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitive_model::{EllipseFrame, StarFrame};
    use crate::units::{Angle, Length};

    fn circle(r: f64, k: f64) -> Shape {
        Shape::Ellipse {
            frame: EllipseFrame {
                center: Point::new(0.0, 0.0),
                rx: Length::from_mm(r),
                ry: Length::from_mm(r),
            },
            arc: None,
            curve: Curve::clamped(k),
        }
    }

    fn ellipse(rx: f64, ry: f64, k: f64) -> Shape {
        Shape::Ellipse {
            frame: EllipseFrame {
                center: Point::new(0.0, 0.0),
                rx: Length::from_mm(rx),
                ry: Length::from_mm(ry),
            },
            arc: None,
            curve: Curve::clamped(k),
        }
    }

    fn frame(r: f64, angle: f64) -> StarFrame {
        StarFrame {
            center: Point::new(0.0, 0.0),
            radius: Length::from_mm(r),
            angle: Angle::from_radians(angle),
        }
    }

    fn polygon(n: u32, r: f64, k: f64) -> Shape {
        Shape::Polygon {
            frame: frame(r, 0.0),
            point_count: PointCount::new(n).unwrap(),
            curve: Curve::clamped(k),
        }
    }

    fn star(n: u32, ratio: f64, r: f64, k: f64) -> Shape {
        Shape::Star {
            frame: frame(r, 0.0),
            point_count: PointCount::new(n).unwrap(),
            inner_ratio: InnerRatio::new(ratio).unwrap(),
            curve: Curve::clamped(k),
        }
    }

    /// Points of the closed outline at `per_segment` samples per cubic.
    pub(crate) fn samples(anchors: &[OutlineAnchor], per_segment: usize) -> Vec<Point> {
        let mut points = Vec::new();
        for (i, a) in anchors.iter().enumerate() {
            let b = anchors[(i + 1) % anchors.len()];
            let p0 = a.point;
            let p1 = a.point.translated(a.handle_out);
            let p2 = b.point.translated(b.handle_in);
            let p3 = b.point;
            for s in 0..per_segment {
                #[allow(clippy::cast_precision_loss)]
                let t = s as f64 / per_segment as f64;
                let mt = 1.0 - t;
                let x = mt.powi(3) * p0.x
                    + 3.0 * mt * mt * t * p1.x
                    + 3.0 * mt * t * t * p2.x
                    + t.powi(3) * p3.x;
                let y = mt.powi(3) * p0.y
                    + 3.0 * mt * mt * t * p1.y
                    + 3.0 * mt * t * t * p2.y
                    + t.powi(3) * p3.y;
                points.push(Point::new(x, y));
            }
        }
        points
    }

    fn area(points: &[Point]) -> f64 {
        let n = points.len();
        (0..n)
            .map(|i| {
                let (a, b) = (points[i], points[(i + 1) % n]);
                a.x * b.y - b.x * a.y
            })
            .sum::<f64>()
            .abs()
            / 2.0
    }

    fn outline(shape: &Shape) -> Vec<OutlineAnchor> {
        let ring = Ring::of(shape, true).unwrap();
        ring_outline(
            &ring,
            effective_curve(shape).unwrap().get(),
            CONVERSION_PIECE_RAD,
        )
    }

    /// The outline split as finely as it is drawn (criterion 67), for areas.
    fn fine_outline(shape: &Shape) -> Vec<OutlineAnchor> {
        let ring = Ring::of(shape, true).unwrap();
        ring_outline(
            &ring,
            effective_curve(shape).unwrap().get(),
            crate::ellipse_arc::DRAWING_PIECE_RAD,
        )
    }

    /// Criterion 19: the middle of the side E to N at `(rx·d, ry·d)`, `d(k) = 0.5·(1 +
    /// tan(22.5°·k))`, and the areas of the three examples.
    #[test]
    fn the_middle_and_the_area_of_the_circle_examples() {
        for (k, d, expected_area) in [
            (1.0, std::f64::consts::FRAC_1_SQRT_2, 1256.6),
            (0.0, 0.5, 800.0),
            (-1.0, 0.292_9, 343.4),
        ] {
            let shape = circle(20.0, k);
            let at = curve_handle_position(&shape).unwrap();
            assert!((at.x - 20.0 * d).abs() < 0.01, "k {k}: {at:?}");
            assert!((at.y + 20.0 * d).abs() < 0.01, "k {k}: {at:?}");
            let a = area(&samples(&fine_outline(&shape), 100));
            assert!((a - expected_area).abs() < 0.1, "k {k}: area {a}");
        }
        let at = curve_handle_position(&ellipse(30.0, 10.0, -1.0)).unwrap();
        assert!(
            (at.x - 8.79).abs() < 0.01 && (at.y + 2.93).abs() < 0.01,
            "{at:?}"
        );
    }

    /// Criterion 29's check values: `P1 = (1, 0.5523)` at 100 %, `(0.4477, 0)` at -100 %,
    /// on the chord at 0 %, for the side E to N of the unit shape (y down: N is (0, -1)).
    #[test]
    fn the_control_points_of_the_unit_ellipse() {
        let p1 = |k: f64| {
            let anchors = outline(&circle(1.0, k));
            // The side N to E ends at node 0 (E); its incoming handle is the mirror image
            // of P1 of the side E to N.
            let e = anchors[0];
            e.point.translated(e.handle_in)
        };
        let at = p1(1.0);
        assert!(
            (at.x - 1.0).abs() < 1e-4 && (at.y + 0.5523).abs() < 1e-4,
            "{at:?}"
        );
        let at = p1(-1.0);
        assert!((at.x - 0.4477).abs() < 1e-4 && at.y.abs() < 1e-9, "{at:?}");
        let anchors = outline(&circle(1.0, 0.0));
        assert!(
            anchors
                .iter()
                .all(|a| a.handle_in == Vec2::ZERO && a.handle_out == Vec2::ZERO)
        );
    }

    /// Criterion 23: the inverse of criterion 19 keeps the handle under the pointer.
    #[test]
    fn the_handle_inverse_is_exact() {
        for k in [-1.0, -0.35, 0.0, 0.5, 1.0] {
            let shape = ellipse(30.0, 10.0, k);
            let at = curve_handle_position(&shape).unwrap();
            let back = curve_from_position(&shape, at).unwrap();
            assert!((back - k).abs() < 1e-12, "{k} -> {back}");
            // criterion 23's formula directly
            let (u, v) = (at.x / 30.0, -at.y / 10.0);
            let formula = ((u + v) - 1.0).atan() / 22.5_f64.to_radians();
            assert!((formula - k).abs() < 1e-12);
        }
        for shape in [polygon(6, 20.0, -0.4), star(5, 0.382, 20.0, 0.7)] {
            let k = effective_curve(&shape).unwrap().get();
            let back = curve_from_position(&shape, curve_handle_position(&shape).unwrap());
            assert!((back.unwrap() - k).abs() < 1e-12);
        }
    }

    /// Criterion 37's examples, `R = 20`: the middle of a side at `m(k)` from the centre.
    #[test]
    fn the_middle_of_a_polygon_side() {
        let m = |n: u32, k: f64| {
            let shape = polygon(n, 20.0, k);
            curve_handle_position(&shape)
                .unwrap()
                .vector_to(Point::new(0.0, 0.0))
                .length()
        };
        for (n, k, expected) in [
            (6, 1.0, 20.0),
            (6, 0.0, 17.32),
            (6, -1.0, 14.64),
            (3, 0.0, 10.0),
            (3, -0.5, 5.36),
            (4, 0.0, 14.14),
            (4, 1.0, 20.0),
        ] {
            assert!(
                (m(n, k) - expected).abs() < 0.01,
                "N {n} k {k}: {}",
                m(n, k)
            );
        }
        let hexagon = |k| area(&samples(&fine_outline(&polygon(6, 20.0, k)), 100));
        assert!((hexagon(1.0) - 1256.6).abs() < 0.1);
        assert!((hexagon(0.0) - 1039.2).abs() < 0.1);
    }

    /// Criterion 38: the limits of the examples.
    #[test]
    fn the_curve_limits() {
        let limit = |shape: &Shape| curve_limit(shape).unwrap().get();
        assert!((limit(&polygon(3, 10.0, 0.0)) + 0.5).abs() < 1e-12);
        for n in [4, 5, 6, 12, 1024] {
            assert!((limit(&polygon(n, 10.0, 0.0)) + 1.0).abs() < 1e-12, "N {n}");
        }
        assert!((limit(&star(3, 0.5, 10.0, 0.0)) + 1.0).abs() < 1e-9);
        assert!((limit(&star(5, 0.382, 10.0, 0.0)) + 1.0).abs() < 1e-9);
        assert!((limit(&star(5, 0.2, 10.0, 0.0)) + 0.444).abs() < 0.001);
        assert!((limit(&circle(1.0, 0.0)) + 1.0).abs() < 1e-12);
        // A stored value below the limit is drawn at the limit.
        let low = star(5, 0.2, 10.0, -0.8);
        assert!((effective_curve(&low).unwrap().get() + 0.444).abs() < 0.001);
        let Shape::Star { curve, .. } = low else {
            unreachable!()
        };
        assert!(
            (curve.get() + 0.8).abs() < 1e-12,
            "the stored value is kept"
        );
    }

    /// Criterion 51: a 4-sided polygon with vertex 0 at E and a circle give the same
    /// control points at every curve value, steps of 12.5 percentage points included.
    #[test]
    fn a_square_polygon_is_the_curved_circle() {
        for step in -8..=8 {
            let k = f64::from(step) * 0.125;
            let a = outline(&polygon(4, 20.0, k));
            let b = outline(&circle(20.0, k));
            assert_eq!(a.len(), b.len());
            for (p, q) in a.iter().zip(&b) {
                for (x, y) in [
                    (p.point, q.point),
                    (
                        p.point.translated(p.handle_in),
                        q.point.translated(q.handle_in),
                    ),
                    (
                        p.point.translated(p.handle_out),
                        q.point.translated(q.handle_out),
                    ),
                ] {
                    assert!(x.vector_to(y).length() < 0.001, "k {k}: {x:?} {y:?}");
                }
            }
        }
    }

    /// Criterion 48: a triangle above 75 % splits each side, 6 nodes at 100 %; a
    /// polygon at 100 % is its circumscribed circle within 0.1 % of `R`.
    #[test]
    fn the_triangle_splits_its_sides_and_each_cubic_is_within_the_bound() {
        assert_eq!(outline(&polygon(3, 20.0, 1.0)).len(), 6);
        assert_eq!(outline(&polygon(3, 20.0, 0.75)).len(), 3);
        for n in [3, 4, 5, 6, 12, 1024] {
            let anchors = outline(&polygon(n, 20.0, 1.0));
            for p in samples(&anchors, 16) {
                let r = p.vector_to(Point::new(0.0, 0.0)).length();
                assert!((r - 20.0).abs() <= 0.02, "N {n}: r {r}");
            }
            assert!(
                anchors.iter().all(|a| a.kind == AnchorKind::Symmetric),
                "N {n}"
            );
        }
    }

    /// Criterion 49: Corner at every vertex away from 100 %, also at the limit.
    #[test]
    fn node_kinds_follow_the_tangents() {
        for shape in [
            polygon(6, 20.0, 0.5),
            polygon(6, 20.0, -1.0),
            circle(5.0, -1.0),
        ] {
            assert!(outline(&shape).iter().all(|a| a.kind == AnchorKind::Corner));
        }
    }

    /// Whether segment `a b` crosses segment `c d` in their interiors.
    fn crosses(a: Point, b: Point, c: Point, d: Point) -> bool {
        let orient =
            |p: Point, q: Point, r: Point| (q.x - p.x) * (r.y - p.y) - (q.y - p.y) * (r.x - p.x);
        let (d1, d2) = (orient(a, b, c), orient(a, b, d));
        let (d3, d4) = (orient(c, d, a), orient(c, d, b));
        let eps = 1e-9;
        ((d1 > eps && d2 < -eps) || (d1 < -eps && d2 > eps))
            && ((d3 > eps && d4 < -eps) || (d3 < -eps && d4 > eps))
    }

    fn self_crossing(points: &[Point]) -> bool {
        let n = points.len();
        for i in 0..n {
            for j in i + 2..n {
                if i == 0 && j == n - 1 {
                    continue;
                }
                if crosses(
                    points[i],
                    points[(i + 1) % n],
                    points[j],
                    points[(j + 1) % n],
                ) {
                    return true;
                }
            }
        }
        false
    }

    /// Criterion 38: for point counts, ratios and every allowed curve value the outline
    /// is one closed curve without self-crossings (a grid in place of random samples,
    /// so no new dependency).
    #[test]
    fn no_outline_crosses_itself() {
        for n in [3, 4, 5, 6, 7, 9, 12, 24] {
            for k in [-1.0, -0.75, -0.5, -0.25, 0.0, 0.25, 0.5, 0.75, 1.0] {
                let p = polygon(n, 20.0, k);
                assert!(!self_crossing(&samples(&outline(&p), 6)), "N {n} k {k}");
                for ratio in [0.01, 0.1, 0.2, 0.382, 0.5, 0.8, 0.99] {
                    let s = star(n, ratio, 20.0, k);
                    assert!(
                        !self_crossing(&samples(&outline(&s), 6)),
                        "star N {n} ratio {ratio} k {k}"
                    );
                }
            }
        }
        for k in [-1.0, -0.3, 0.6, 1.0] {
            let big = polygon(1024, 50.0, k);
            assert_eq!(outline(&big).len(), 1024);
            let s = star(1024, 0.95, 50.0, k);
            assert!(!self_crossing(&samples(&outline(&s), 1)), "1024 star k {k}");
        }
    }
}
