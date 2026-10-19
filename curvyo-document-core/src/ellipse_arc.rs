//! The outline, box and cubic form of an elliptical arc
//! (`specs/0021-ellipse-arcs-and-shaping/` criteria 4, 5, 9, 13, 54, 56 and 64,
//! `adrs.md` decisions 4, 7 and 9): angles are the ellipse's own parametric angles from
//! its right-hand axis, increasing clockwise on screen; the arc under a linear map (a
//! mirror, a stretch) is again an ellipse arc with shifted angles. Pure arithmetic.

// The formulas of the specification name their quantities by single letters (k, c, θ,
// v, w, …); the code keeps those names so it reads against the criteria.
#![allow(clippy::many_single_char_names, clippy::similar_names)]

use serde::Serialize;

use crate::path_model::AnchorKind;
use crate::primitive_model::EllipseFrame;
use crate::primitive_outline::OutlineAnchor;
use crate::units::{Angle, Point, Vec2};

/// Start and end closer than this (degrees, around the circle) make the ellipse whole
/// (criterion 5).
pub const WHOLE_ARC_TOLERANCE_DEG: f64 = 0.1;

/// The largest sweep one cubic of the drawn outline covers: its deviation from the true
/// arc stays below 1e-7 of the radius, far under the 0.01 mm kernel tolerance for every
/// size the document allows (criterion 67).
pub(crate) const DRAWING_PIECE_RAD: f64 = std::f64::consts::FRAC_PI_8;

/// How an arc is closed (criterion 3's three kinds).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ArcKind {
    /// The arc and a straight line from each end to the centre: a sector.
    Pie,
    /// The arc only, an open path.
    Arc,
    /// The arc closed by the straight line from its end to its start: a segment.
    Chord,
}

impl ArcKind {
    /// The stored name of this kind (`adrs.md` decision 1).
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Pie => "pie",
            Self::Arc => "arc",
            Self::Chord => "chord",
        }
    }

    /// The kind of a stored name; `None` for any other string (damaged).
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "pie" => Some(Self::Pie),
            "arc" => Some(Self::Arc),
            "chord" => Some(Self::Chord),
            _ => None,
        }
    }
}

/// An ellipse's arc: start and end angle, both in `[0, 2π)`, never closer than
/// [`WHOLE_ARC_TOLERANCE_DEG`] around the circle, and the kind.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct EllipseArc {
    start: Angle,
    end: Angle,
    /// How the arc is closed.
    pub kind: ArcKind,
}

/// `angle` wrapped into `[0, 2π)` (criterion 64: −90° is 270°, 360° is 0°).
#[must_use]
pub fn wrapped_angle(angle: Angle) -> Angle {
    let wrapped = angle.as_radians().rem_euclid(std::f64::consts::TAU);
    // `rem_euclid` can round up to exactly τ for a tiny negative input.
    Angle::from_radians(if wrapped >= std::f64::consts::TAU {
        0.0
    } else {
        wrapped
    })
}

impl EllipseArc {
    /// The arc from `start` to `end` of `kind`, the angles wrapped into `[0, 2π)`;
    /// `None` (the whole ellipse) when they are within 0.1° of each other around the
    /// circle or not finite (criteria 5 and 64).
    #[must_use]
    pub fn new(start: Angle, end: Angle, kind: ArcKind) -> Option<Self> {
        if !(start.as_radians().is_finite() && end.as_radians().is_finite()) {
            return None;
        }
        let (start, end) = (wrapped_angle(start), wrapped_angle(end));
        let gap = (end.as_radians() - start.as_radians()).abs();
        let around = gap.min(std::f64::consts::TAU - gap);
        (around > WHOLE_ARC_TOLERANCE_DEG.to_radians()).then_some(Self { start, end, kind })
    }

    /// The start angle, in `[0, 2π)`.
    #[must_use]
    pub const fn start(&self) -> Angle {
        self.start
    }

    /// The end angle, in `[0, 2π)`.
    #[must_use]
    pub const fn end(&self) -> Angle {
        self.end
    }

    /// The sweep from start to end in the direction of increasing angle, in `(0, 2π)`
    /// (criterion 5).
    #[must_use]
    pub fn sweep(&self) -> Angle {
        wrapped_angle(Angle::from_radians(
            self.end.as_radians() - self.start.as_radians(),
        ))
    }

    /// This arc closed as `kind`, the angles kept (criterion 3).
    #[must_use]
    pub const fn with_kind(self, kind: ArcKind) -> Self {
        Self { kind, ..self }
    }

    /// This arc with its angles moved by `delta` (a rotation of the ellipse's frame
    /// against its points, `adrs.md` decision 9).
    #[must_use]
    pub fn shifted(self, delta: f64) -> Self {
        Self {
            start: wrapped_angle(Angle::from_radians(self.start.as_radians() + delta)),
            end: wrapped_angle(Angle::from_radians(self.end.as_radians() + delta)),
            kind: self.kind,
        }
    }

    /// This arc under the reflection `t → beta − t`: the direction of increasing angle
    /// reverses, so the new start is the image of the old end (criterion 54).
    #[must_use]
    pub fn reflected(self, beta: f64) -> Self {
        Self {
            start: wrapped_angle(Angle::from_radians(beta - self.end.as_radians())),
            end: wrapped_angle(Angle::from_radians(beta - self.start.as_radians())),
            kind: self.kind,
        }
    }
}

/// The point at parametric angle `t` of `frame`, without rotation.
#[must_use]
pub fn ellipse_point(frame: EllipseFrame, t: f64) -> Point {
    let (sin, cos) = t.sin_cos();
    frame
        .center
        .translated(Vec2::new(frame.rx.as_mm() * cos, frame.ry.as_mm() * sin))
}

fn ellipse_tangent(frame: EllipseFrame, t: f64) -> Vec2 {
    let (sin, cos) = t.sin_cos();
    Vec2::new(-frame.rx.as_mm() * sin, frame.ry.as_mm() * cos)
}

/// The outline of `arc` on `frame` without rotation, and whether it is closed
/// (criteria 4 and 13): the arc split into `ceil(S / piece)` cubics of equal sweep
/// (the standard arc-to-cubic construction), Symmetric nodes between them, Corner nodes
/// at the ends; a Pie adds the centre as one Corner node, a Chord and a Pie are closed.
pub(crate) fn arc_outline(
    frame: EllipseFrame,
    arc: EllipseArc,
    piece: f64,
) -> (Vec<OutlineAnchor>, bool) {
    let sweep = arc.sweep().as_radians();
    // A sweep below 2π over a piece of at least π/8 is at most 16 pieces.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let pieces = ((sweep / piece - 1e-9).ceil().max(1.0)) as usize;
    #[allow(clippy::cast_precision_loss)]
    let delta = sweep / pieces as f64;
    let handle = 4.0 / 3.0 * (delta / 4.0).tan();
    let start = arc.start.as_radians();
    let mut anchors = Vec::with_capacity(pieces + 2);
    for i in 0..=pieces {
        #[allow(clippy::cast_precision_loss)]
        let t = start + delta * i as f64;
        let tangent = ellipse_tangent(frame, t).scaled(handle);
        let first = i == 0;
        let last = i == pieces;
        anchors.push(OutlineAnchor {
            point: ellipse_point(frame, t),
            handle_in: if first { Vec2::ZERO } else { tangent.negated() },
            handle_out: if last { Vec2::ZERO } else { tangent },
            kind: if first || last {
                AnchorKind::Corner
            } else {
                AnchorKind::Symmetric
            },
        });
    }
    match arc.kind {
        ArcKind::Arc => (anchors, false),
        ArcKind::Chord => (anchors, true),
        ArcKind::Pie => {
            anchors.push(OutlineAnchor {
                point: frame.center,
                handle_in: Vec2::ZERO,
                handle_out: Vec2::ZERO,
                kind: AnchorKind::Corner,
            });
            (anchors, true)
        }
    }
}

/// The tight box of what `arc` draws on `frame`, without rotation (criterion 9): the
/// ends, every axis extreme the sweep passes and, for a Pie, the centre.
#[must_use]
pub fn arc_bounds(frame: EllipseFrame, arc: EllipseArc) -> (Point, Point) {
    let start = arc.start.as_radians();
    let sweep = arc.sweep().as_radians();
    let mut points = vec![
        ellipse_point(frame, start),
        ellipse_point(frame, start + sweep),
    ];
    for quarter in 0..4 {
        let t = f64::from(quarter) * std::f64::consts::FRAC_PI_2;
        let along = (t - start).rem_euclid(std::f64::consts::TAU);
        if along <= sweep {
            points.push(ellipse_point(frame, t));
        }
    }
    if arc.kind == ArcKind::Pie {
        points.push(frame.center);
    }
    let mut min = Point::new(f64::INFINITY, f64::INFINITY);
    let mut max = Point::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
    for p in points {
        min = Point::new(min.x.min(p.x), min.y.min(p.y));
        max = Point::new(max.x.max(p.x), max.y.max(p.y));
    }
    (min, max)
}

/// The exact axis-aligned box of what `arc` draws on `frame` turned by `rotation` about
/// the frame's centre (criteria 9 and 54): the ends, the parameters where the turned
/// ellipse's x or y is extreme and the sweep passes them, and a Pie's centre.
#[must_use]
pub fn rotated_arc_bounds(frame: EllipseFrame, arc: EllipseArc, rotation: Angle) -> (Point, Point) {
    let (rx, ry) = (frame.rx.as_mm(), frame.ry.as_mm());
    let (sin, cos) = rotation.as_radians().sin_cos();
    let point = |t: f64| {
        let (st, ct) = t.sin_cos();
        let (x, y) = (rx * ct, ry * st);
        Point::new(
            frame.center.x + x * cos - y * sin,
            frame.center.y + x * sin + y * cos,
        )
    };
    let start = arc.start.as_radians();
    let sweep = arc.sweep().as_radians();
    let mut points = vec![point(start), point(start + sweep)];
    // x(t) = rx cos t cos θ − ry sin t sin θ is extreme at atan2(−ry sin θ, rx cos θ);
    // y(t) = rx cos t sin θ + ry sin t cos θ at atan2(ry cos θ, rx sin θ); each also + π.
    for base in [(-ry * sin).atan2(rx * cos), (ry * cos).atan2(rx * sin)] {
        for t in [base, base + std::f64::consts::PI] {
            if (t - start).rem_euclid(std::f64::consts::TAU) <= sweep {
                points.push(point(t));
            }
        }
    }
    if arc.kind == ArcKind::Pie {
        points.push(frame.center);
    }
    let mut min = Point::new(f64::INFINITY, f64::INFINITY);
    let mut max = Point::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
    for p in points {
        min = Point::new(min.x.min(p.x), min.y.min(p.y));
        max = Point::new(max.x.max(p.x), max.y.max(p.y));
    }
    (min, max)
}

/// A 2×2 linear map, rows first: `(x, y) → (a x + b y, c x + d y)`.
pub type Linear = [[f64; 2]; 2];

fn multiply(p: Linear, q: Linear) -> Linear {
    [
        [
            p[0][0] * q[0][0] + p[0][1] * q[1][0],
            p[0][0] * q[0][1] + p[0][1] * q[1][1],
        ],
        [
            p[1][0] * q[0][0] + p[1][1] * q[1][0],
            p[1][0] * q[0][1] + p[1][1] * q[1][1],
        ],
    ]
}

fn rotation_matrix(angle: f64) -> Linear {
    let (sin, cos) = angle.sin_cos();
    [[cos, -sin], [sin, cos]]
}

/// An ellipse's shape in the document: radii and rotation (the centre is mapped by the
/// caller).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EllipseAxes {
    /// The radius along the ellipse's own x axis.
    pub rx: f64,
    /// The radius along its own y axis.
    pub ry: f64,
    /// The angle of its own x axis, radians.
    pub rotation: f64,
}

/// A shift or reflection of angles within this (radians) of none is none, so an aligned
/// scale leaves the stored angles exactly as they were.
const ANGLE_IDENTITY_EPSILON: f64 = 1e-12;

/// The arc `arc` of the ellipse `before` after the linear map `m`, for the ellipse
/// `after` that is its image (`adrs.md` decisions 7 and 9): with `Q = S'⁻¹ R(−θ') M R(θ)
/// S` orthogonal, a point at parameter `t` goes to `t + β` (a rotation by β) or to
/// `β − t` (a reflection, start and end swap). An `after` with a radius of 0 keeps the
/// angles.
#[must_use]
pub fn arc_after_linear_map(
    arc: EllipseArc,
    before: EllipseAxes,
    after: EllipseAxes,
    m: Linear,
) -> EllipseArc {
    if after.rx.abs() <= f64::EPSILON || after.ry.abs() <= f64::EPSILON {
        return arc;
    }
    let scale = [[before.rx, 0.0], [0.0, before.ry]];
    let unscale = [[1.0 / after.rx, 0.0], [0.0, 1.0 / after.ry]];
    let q = multiply(
        unscale,
        multiply(
            rotation_matrix(-after.rotation),
            multiply(m, multiply(rotation_matrix(before.rotation), scale)),
        ),
    );
    let beta = q[1][0].atan2(q[0][0]);
    let determinant = q[0][0] * q[1][1] - q[0][1] * q[1][0];
    if determinant >= 0.0 {
        if beta.abs() <= ANGLE_IDENTITY_EPSILON {
            arc
        } else {
            arc.shifted(beta)
        }
    } else {
        arc.reflected(beta)
    }
}

/// Singular values closer than this fraction of the larger are equal: the image is a
/// circle, written with rotation 0 (criterion 56).
const CIRCLE_EPSILON: f64 = 1e-9;

/// The ellipse `before` after the linear map `m` (criterion 56, `adrs.md` decision 9):
/// the image of an ellipse under a linear map is an ellipse whose axes are the singular
/// vectors of `M · R(θ) · diag(rx, ry)`. The rotation is the one of the four axis
/// choices nearest to `before.rotation` (so a stretch and its inverse restore it), `0`
/// for a circle.
#[must_use]
pub fn ellipse_after_linear_map(before: EllipseAxes, m: Linear) -> EllipseAxes {
    let a = multiply(
        m,
        multiply(
            rotation_matrix(before.rotation),
            [[before.rx, 0.0], [0.0, before.ry]],
        ),
    );
    // A Aᵀ, symmetric: its eigenvectors are the axes of the image.
    let p = a[0][0] * a[0][0] + a[0][1] * a[0][1];
    let q = a[0][0] * a[1][0] + a[0][1] * a[1][1];
    let r = a[1][0] * a[1][0] + a[1][1] * a[1][1];
    let principal = 0.5 * (2.0 * q).atan2(p - r);
    let radius_along = |angle: f64| {
        let (sin, cos) = angle.sin_cos();
        // |Aᵀ u| for the unit vector u at `angle`.
        (a[0][0] * cos + a[1][0] * sin).hypot(a[0][1] * cos + a[1][1] * sin)
    };
    let (major, minor) = (
        radius_along(principal),
        radius_along(principal + std::f64::consts::FRAC_PI_2),
    );
    if (major - minor).abs() <= CIRCLE_EPSILON * major.max(minor) {
        return EllipseAxes {
            rx: major,
            ry: major,
            rotation: 0.0,
        };
    }
    let half_turn = std::f64::consts::PI;
    let quarter = std::f64::consts::FRAC_PI_2;
    let mut best = (f64::INFINITY, principal, major, minor);
    for k in -4..=4 {
        let rotation = principal + f64::from(k) * quarter;
        let off = (rotation - before.rotation + half_turn).rem_euclid(2.0 * half_turn) - half_turn;
        if off.abs() < best.0 {
            let (rx, ry) = if k % 2 == 0 {
                (major, minor)
            } else {
                (minor, major)
            };
            best = (off.abs(), before.rotation + off, rx, ry);
        }
    }
    EllipseAxes {
        rx: best.2,
        ry: best.3,
        rotation: best.1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::units::Length;

    const PI2: f64 = std::f64::consts::FRAC_PI_2;

    fn frame(rx: f64, ry: f64) -> EllipseFrame {
        EllipseFrame {
            center: Point::new(3.0, -2.0),
            rx: Length::from_mm(rx),
            ry: Length::from_mm(ry),
        }
    }

    fn deg(value: f64) -> Angle {
        Angle::from_radians(value.to_radians())
    }

    fn arc(start: f64, end: f64, kind: ArcKind) -> EllipseArc {
        EllipseArc::new(deg(start), deg(end), kind).unwrap()
    }

    /// Criteria 5 and 64: angles in `[0°, 360°)`, a sweep never negative, equal angles
    /// within 0.1° the whole ellipse.
    #[test]
    fn angles_wrap_and_close_angles_are_whole() {
        let a = arc(-90.0, 450.0, ArcKind::Arc);
        assert!((a.start().as_radians().to_degrees() - 270.0).abs() < 1e-9);
        assert!((a.end().as_radians().to_degrees() - 90.0).abs() < 1e-9);
        assert!((a.sweep().as_radians().to_degrees() - 180.0).abs() < 1e-9);
        assert!(EllipseArc::new(deg(360.0), deg(0.05), ArcKind::Pie).is_none());
        assert!(EllipseArc::new(deg(359.95), deg(0.0), ArcKind::Pie).is_none());
        assert!(EllipseArc::new(deg(10.0), deg(10.2), ArcKind::Pie).is_some());
        // An end that crosses the start makes a sliver, never a negative sweep.
        let sliver = arc(45.0, 40.0, ArcKind::Arc);
        assert!((sliver.sweep().as_radians().to_degrees() - 355.0).abs() < 1e-9);
        assert!(EllipseArc::new(Angle::from_radians(f64::NAN), deg(1.0), ArcKind::Arc).is_none());
    }

    /// Criterion 13: `ceil(S/90°)` cubics within 0.1 % of the larger radius, Symmetric
    /// between them, Corner at the ends, a Pie adds the centre, an Arc is open.
    #[test]
    fn the_converted_arc_has_the_specified_nodes() {
        let f = frame(20.0, 12.0);
        let (anchors, closed) = arc_outline(f, arc(30.0, 300.0, ArcKind::Arc), PI2);
        assert!(!closed);
        assert_eq!(anchors.len(), 4, "270° is three cubics");
        assert_eq!(anchors[0].kind, AnchorKind::Corner);
        assert_eq!(anchors[3].kind, AnchorKind::Corner);
        assert!(
            anchors[1..3]
                .iter()
                .all(|a| a.kind == AnchorKind::Symmetric)
        );
        let (pie, closed) = arc_outline(f, arc(0.0, 100.0, ArcKind::Pie), PI2);
        assert!(closed);
        assert_eq!(pie.len(), 4, "two cubics and the centre");
        assert_eq!(pie[3].point, f.center);
        let (chord, closed) = arc_outline(f, arc(0.0, 90.0, ArcKind::Chord), PI2);
        assert!(closed && chord.len() == 2);
        // The deviation of every cubic from the true ellipse.
        for (a, b) in anchors.iter().zip(&anchors[1..]) {
            for s in 1..32 {
                let t = f64::from(s) / 32.0;
                let mt = 1.0 - t;
                let c1 = a.point.translated(a.handle_out);
                let c2 = b.point.translated(b.handle_in);
                let x = mt.powi(3) * a.point.x
                    + 3.0 * mt * mt * t * c1.x
                    + 3.0 * mt * t * t * c2.x
                    + t.powi(3) * b.point.x;
                let y = mt.powi(3) * a.point.y
                    + 3.0 * mt * mt * t * c1.y
                    + 3.0 * mt * t * t * c2.y
                    + t.powi(3) * b.point.y;
                let (u, v) = ((x - 3.0) / 20.0, (y + 2.0) / 12.0);
                let off = (u.hypot(v) - 1.0).abs() * 20.0;
                assert!(off < 0.001 * 20.0, "deviation {off}");
            }
        }
    }

    /// Criteria 9 and 54: the turned box is exact, also at a rotation.
    #[test]
    fn the_turned_box_is_exact() {
        let f = frame(20.0, 10.0);
        let a = arc(0.0, 180.0, ArcKind::Arc);
        let (min, max) = rotated_arc_bounds(f, a, Angle::from_radians(0.0));
        assert_eq!((min, max), arc_bounds(f, a));
        let turned = Angle::from_radians(30_f64.to_radians());
        let (min, max) = rotated_arc_bounds(f, a, turned);
        let mut lo = Point::new(f64::INFINITY, f64::INFINITY);
        let mut hi = Point::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
        for i in 0..=20_000 {
            let t = std::f64::consts::PI * f64::from(i) / 20_000.0;
            let p = ellipse_point(f, t).rotated_around(f.center, turned);
            lo = Point::new(lo.x.min(p.x), lo.y.min(p.y));
            hi = Point::new(hi.x.max(p.x), hi.y.max(p.y));
        }
        assert!(min.vector_to(lo).length() < 1e-6 && max.vector_to(hi).length() < 1e-6);
    }

    /// Criterion 9: a half circle of radius 20 has a box of 40 × 20; a pie includes its
    /// centre.
    #[test]
    fn the_box_is_the_box_of_the_drawn_part() {
        let f = EllipseFrame {
            center: Point::new(0.0, 0.0),
            rx: Length::from_mm(20.0),
            ry: Length::from_mm(20.0),
        };
        let (min, max) = arc_bounds(f, arc(0.0, 180.0, ArcKind::Arc));
        assert!((max.x - min.x - 40.0).abs() < 1e-9 && (max.y - min.y - 20.0).abs() < 1e-9);
        let (min, max) = arc_bounds(f, arc(10.0, 80.0, ArcKind::Pie));
        assert!(min.x.abs() < 1e-12 && min.y.abs() < 1e-12, "{min:?}");
        assert!(max.x > 19.0 && max.y > 19.0);
        let (min, _) = arc_bounds(f, arc(10.0, 80.0, ArcKind::Chord));
        assert!(min.x > 3.0, "a chord does not reach the centre: {min:?}");
    }

    fn sample(f: EllipseFrame, axes: EllipseAxes, a: EllipseArc, i: usize) -> Point {
        #[allow(clippy::cast_precision_loss)]
        let t = a.start().as_radians() + a.sweep().as_radians() * i as f64 / 63.0;
        let local = ellipse_point(
            EllipseFrame {
                rx: Length::from_mm(axes.rx),
                ry: Length::from_mm(axes.ry),
                ..f
            },
            t,
        );
        local.rotated_around(f.center, Angle::from_radians(axes.rotation))
    }

    fn apply(m: Linear, p: Point) -> Point {
        Point::new(m[0][0] * p.x + m[0][1] * p.y, m[1][0] * p.x + m[1][1] * p.y)
    }

    /// Criterion 56: a 30° rotated half circle stretched by (2, 0.5) in the document axes
    /// is the image of the old arc at 64 samples within 1e-9; a stretch of an ellipse and
    /// its inverse restore the angles.
    #[test]
    fn a_stretch_keeps_the_arc_and_its_inverse_restores_it() {
        let f = EllipseFrame {
            center: Point::new(0.0, 0.0),
            rx: Length::from_mm(20.0),
            ry: Length::from_mm(20.0),
        };
        let before = EllipseAxes {
            rx: 20.0,
            ry: 20.0,
            rotation: 30_f64.to_radians(),
        };
        let half = arc(0.0, 180.0, ArcKind::Arc);
        let m = [[2.0, 0.0], [0.0, 0.5]];
        let after = ellipse_after_linear_map(before, m);
        let mapped = arc_after_linear_map(half, before, after, m);
        assert!((mapped.sweep().as_radians() - half.sweep().as_radians()).abs() < 1e-9);
        for i in 0..64 {
            let expected = apply(m, sample(f, before, half, i));
            let actual = sample(f, after, mapped, i);
            assert!(expected.vector_to(actual).length() < 1e-9, "{i}");
        }
        let ellipse = EllipseAxes {
            rx: 20.0,
            ry: 8.0,
            rotation: 30_f64.to_radians(),
        };
        let a = arc(15.0, 200.0, ArcKind::Pie);
        let once = ellipse_after_linear_map(ellipse, m);
        let a1 = arc_after_linear_map(a, ellipse, once, m);
        let back_m = [[0.5, 0.0], [0.0, 2.0]];
        let back = ellipse_after_linear_map(once, back_m);
        let a2 = arc_after_linear_map(a1, once, back, back_m);
        assert!((back.rotation - ellipse.rotation).abs() < 1e-9);
        assert!((back.rx - 20.0).abs() < 1e-9 && (back.ry - 8.0).abs() < 1e-9);
        for (x, y) in [(a2.start(), a.start()), (a2.end(), a.end())] {
            let off = (x.as_radians() - y.as_radians() + std::f64::consts::PI)
                .rem_euclid(std::f64::consts::TAU)
                - std::f64::consts::PI;
            assert!(off.to_degrees().abs() < 1e-9, "{off}");
        }
    }

    /// Criterion 54: a mirror along the ellipse's own horizontal axis gives
    /// `start' = −end`, along its vertical axis `start' = 180° − end`; a flip twice is
    /// the identity; the mirrored arc is the mirror image at 64 samples.
    #[test]
    fn a_mirror_reverses_the_angles() {
        let f = frame(20.0, 10.0);
        let a = arc(20.0, 130.0, ArcKind::Chord);
        let axes = EllipseAxes {
            rx: 20.0,
            ry: 10.0,
            rotation: 0.0,
        };
        let flip_y = [[1.0, 0.0], [0.0, -1.0]];
        let m = arc_after_linear_map(a, axes, axes, flip_y);
        assert!((m.start().as_radians().to_degrees() - (360.0 - 130.0)).abs() < 1e-9);
        assert!((m.end().as_radians().to_degrees() - (360.0 - 20.0)).abs() < 1e-9);
        let flip_x = [[-1.0, 0.0], [0.0, 1.0]];
        let m = arc_after_linear_map(a, axes, axes, flip_x);
        assert!((m.start().as_radians().to_degrees() - 50.0).abs() < 1e-9);
        assert!((m.end().as_radians().to_degrees() - 160.0).abs() < 1e-9);
        let twice = arc_after_linear_map(m, axes, axes, flip_x);
        assert!((twice.start().as_radians() - a.start().as_radians()).abs() < 1e-9);
        for rotation in [0.0_f64, 30.0, 90.0] {
            let turned = EllipseAxes {
                rotation: rotation.to_radians(),
                ..axes
            };
            // A document-axes horizontal flip: rotation becomes −θ.
            let image = EllipseAxes {
                rotation: -rotation.to_radians(),
                ..axes
            };
            let mirrored = arc_after_linear_map(a, turned, image, flip_x);
            let origin = EllipseFrame {
                center: Point::new(0.0, 0.0),
                ..f
            };
            for i in 0..64 {
                let expected = apply(flip_x, sample(origin, turned, a, 63 - i));
                let actual = sample(origin, image, mirrored, i);
                assert!(expected.vector_to(actual).length() < 1e-9, "{rotation} {i}");
            }
        }
    }
}
