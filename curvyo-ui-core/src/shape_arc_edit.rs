//! The arc and curve values of an ellipse, polygon or star
//! (`specs/0021-ellipse-arcs-and-shaping/` criteria 2, 3, 5, 7, 23, 31, 38, 41, 44 and 64):
//! a pointer turned into an angle or a curve, and a value applied to a shape, the one rule
//! the drag, the entry chip and the bar share through [`crate::apply_param`].

use curvyo_document_core::{
    Angle, ArcKind, Curve, EllipseArc, Point, Shape, curve_from_position, curve_limit,
    effective_curve,
};

use crate::angle_snap::snap_angle;

/// The angle both arc handles rest at on a whole ellipse, and the angle the handle that is
/// not dragged takes when a first drag or entry makes an arc (criteria 1 to 3): 45°.
pub const ARC_SEAM: Angle = Angle::from_radians(std::f64::consts::FRAC_PI_4);

/// A drag within this (a fraction, one percentage point) of 100 %, 0 % or the shape's
/// limit snaps to it without a modifier (criteria 23 and 41).
const CURVE_SNAP: f64 = 0.01;

/// The step of a Ctrl-snapped curve drag: 25 percentage points.
const CURVE_CTRL_STEP: f64 = 0.25;

/// A curve within this of the current one is the same: nothing is written.
const CURVE_EQUAL_EPSILON: f64 = 1e-9;

/// Which end of an arc a handle or a field sets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArcEnd {
    /// The start angle.
    Start,
    /// The end angle.
    End,
}

/// The arc an ellipse draws, as the handles see it: its own arc, or none for a whole
/// ellipse. `None` for an ellipse whose curve is not 100 % and for every other shape: it
/// has no arc handles (criterion 31).
#[must_use]
pub fn arc_capable(shape: &Shape) -> Option<Option<EllipseArc>> {
    match *shape {
        Shape::Ellipse { arc, curve, .. } if arc.is_some() || curve == Curve::ELLIPSE_DEFAULT => {
            Some(arc)
        }
        _ => None,
    }
}

/// Whether `shape` takes a curve now (criteria 26, 31, 44 and 52): an ellipse without an
/// arc, a polygon, a star.
#[must_use]
pub fn curve_capable(shape: &Shape) -> bool {
    match shape {
        Shape::Ellipse { arc, .. } => arc.is_none(),
        Shape::Polygon { .. } | Shape::Star { .. } => true,
        Shape::Rect { .. } => false,
    }
}

/// The angle a pointer at `local` (the box-local frame, the ellipse's centre at
/// `frame.center`) asks for (criterion 2): `atan2(y / ry, x / rx)`, inside or outside
/// the ellipse; with `ctrl` the nearest stop of 15° and 22.5° (criterion 7). `None` for a
/// degenerate ellipse, the centre itself or another shape.
#[must_use]
pub fn arc_angle_from_pointer(shape: &Shape, local: Point, ctrl: bool) -> Option<Angle> {
    let Shape::Ellipse { frame, .. } = shape else {
        return None;
    };
    let (rx, ry) = (frame.rx.as_mm(), frame.ry.as_mm());
    if rx <= f64::EPSILON || ry <= f64::EPSILON {
        return None;
    }
    let offset = frame.center.vector_to(local);
    let (u, v) = (offset.x / rx, offset.y / ry);
    if !(u.is_finite() && v.is_finite()) || (u == 0.0 && v == 0.0) {
        return None;
    }
    let raw = Angle::from_radians(v.atan2(u));
    Some(if ctrl { snap_angle(raw) } else { raw })
}

/// The curve a pointer at `local` asks for on `shape` (criteria 23 and 41): the exact
/// inverse of the handle position, snapped to 100 %, 0 % and the limit within one
/// percentage point, with `ctrl` to the nearest multiple of 25 points or the limit, and
/// limited to `[k_min, 100 %]`.
#[must_use]
pub fn curve_from_pointer(shape: &Shape, local: Point, ctrl: bool) -> Option<f64> {
    let raw = curve_from_position(shape, local)?;
    let limit = curve_limit(shape)?.get();
    let snapped = if ctrl {
        let step = (raw / CURVE_CTRL_STEP).round() * CURVE_CTRL_STEP;
        if (raw - limit).abs() < (raw - step).abs() {
            limit
        } else {
            step
        }
    } else {
        [1.0, 0.0, limit]
            .into_iter()
            .find(|stop| (raw - stop).abs() <= CURVE_SNAP)
            .unwrap_or(raw)
    };
    Some(snapped.clamp(limit, 1.0))
}

/// `shape` with `end`'s angle set to `angle` (criteria 2, 3 and 64): on a whole ellipse
/// the other angle is [`ARC_SEAM`] and the new arc an open Arc; equal angles within 0.1°
/// make it whole. `None` when nothing changes or `shape` has no arc handles.
#[must_use]
pub fn with_arc_angle(shape: &Shape, end: ArcEnd, angle: Angle) -> Option<Shape> {
    let current = arc_capable(shape)?;
    if !angle.as_radians().is_finite() {
        return None;
    }
    let (start, stop, kind) = match current {
        Some(arc) => (arc.start(), arc.end(), arc.kind),
        None => (ARC_SEAM, ARC_SEAM, ArcKind::Arc),
    };
    let (start, stop) = match end {
        ArcEnd::Start => (angle, stop),
        ArcEnd::End => (start, angle),
    };
    with_arc(shape, EllipseArc::new(start, stop, kind))
}

/// `shape` with its arc's kind set to `kind` (criterion 3); a whole ellipse has no kind to
/// set. `None` when nothing changes.
#[must_use]
pub fn with_arc_kind(shape: &Shape, kind: ArcKind) -> Option<Shape> {
    let arc = arc_capable(shape)??;
    with_arc(shape, Some(arc.with_kind(kind)))
}

/// `shape` made whole ("Whole ellipse", criterion 3). `None` when it is whole already.
#[must_use]
pub fn made_whole(shape: &Shape) -> Option<Shape> {
    arc_capable(shape)??;
    with_arc(shape, None)
}

fn with_arc(shape: &Shape, arc: Option<EllipseArc>) -> Option<Shape> {
    let Shape::Ellipse {
        frame,
        arc: old,
        curve,
    } = *shape
    else {
        return None;
    };
    (old != arc).then_some(Shape::Ellipse { frame, arc, curve })
}

/// `shape` with its curve set to `fraction`, limited to that shape's `[k_min, 100 %]`
/// (criteria 26, 38, 42 and 44). `None` when the limited value equals the effective one
/// (nothing to write), when the value is not finite, and for a shape that takes no curve
/// now.
#[must_use]
pub fn with_curve(shape: &Shape, fraction: f64) -> Option<Shape> {
    if !fraction.is_finite() || !curve_capable(shape) {
        return None;
    }
    let limit = curve_limit(shape)?.get();
    let limited = Curve::clamped(fraction.clamp(limit, 1.0));
    let effective = effective_curve(shape)?;
    if (limited.get() - effective.get()).abs() <= CURVE_EQUAL_EPSILON {
        return None;
    }
    Some(match *shape {
        Shape::Ellipse { frame, arc, .. } => Shape::Ellipse {
            frame,
            arc,
            curve: limited,
        },
        Shape::Polygon {
            frame, point_count, ..
        } => Shape::Polygon {
            frame,
            point_count,
            curve: limited,
        },
        Shape::Star {
            frame,
            point_count,
            inner_ratio,
            ..
        } => Shape::Star {
            frame,
            point_count,
            inner_ratio,
            curve: limited,
        },
        Shape::Rect { .. } => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use curvyo_document_core::{EllipseFrame, Length, PointCount, StarFrame};

    fn ellipse(arc: Option<EllipseArc>, curve: f64) -> Shape {
        Shape::Ellipse {
            frame: EllipseFrame {
                center: Point::new(10.0, 20.0),
                rx: Length::from_mm(30.0),
                ry: Length::from_mm(10.0),
            },
            arc,
            curve: Curve::new(curve).unwrap(),
        }
    }

    fn deg(value: f64) -> Angle {
        Angle::from_radians(value.to_radians())
    }

    fn arc_of(shape: &Shape) -> Option<EllipseArc> {
        match *shape {
            Shape::Ellipse { arc, .. } => arc,
            _ => None,
        }
    }

    fn degrees(angle: Angle) -> f64 {
        angle.as_radians().to_degrees()
    }

    /// Criterion 2: the parametric angle of the pointer, inside or outside the ellipse.
    #[test]
    fn the_angle_follows_the_pointer_in_the_ellipse_frame() {
        let shape = ellipse(None, 1.0);
        let t = 30_f64.to_radians();
        for scale in [0.5, 1.0, 3.0] {
            let at = Point::new(10.0 + 30.0 * t.cos() * scale, 20.0 + 10.0 * t.sin() * scale);
            let angle = arc_angle_from_pointer(&shape, at, false).unwrap();
            assert!((degrees(angle) - 30.0).abs() < 1e-9);
        }
        let near = Point::new(10.0 + 30.0 * 0.26_f64.cos(), 20.0 + 10.0 * 0.26_f64.sin());
        let snapped = arc_angle_from_pointer(&shape, near, true).unwrap();
        assert!((degrees(snapped) - 15.0).abs() < 1e-9, "Ctrl snaps to 15°");
        assert!(arc_angle_from_pointer(&shape, Point::new(10.0, 20.0), false).is_none());
    }

    /// Criteria 2 and 3: the first drag or entry on a whole ellipse makes an open Arc with
    /// the other angle at the seam of 45°.
    #[test]
    fn a_first_angle_makes_an_open_arc_from_the_seam() {
        let shape = with_arc_angle(&ellipse(None, 1.0), ArcEnd::End, deg(300.0)).unwrap();
        let arc = arc_of(&shape).unwrap();
        assert_eq!(arc.kind, ArcKind::Arc);
        assert!((degrees(arc.start()) - 45.0).abs() < 1e-9);
        assert!((degrees(arc.end()) - 300.0).abs() < 1e-9);
        let shape = with_arc_angle(&shape, ArcEnd::Start, deg(-90.0)).unwrap();
        assert!(
            (degrees(arc_of(&shape).unwrap().start()) - 270.0).abs() < 1e-9,
            "criterion 64"
        );
    }

    /// Criteria 5 and 64: an angle within 0.1° of the other makes the ellipse whole.
    #[test]
    fn meeting_angles_make_the_ellipse_whole() {
        let arc = EllipseArc::new(deg(10.0), deg(100.0), ArcKind::Pie);
        let shape = ellipse(arc, 1.0);
        let whole = with_arc_angle(&shape, ArcEnd::End, deg(10.05)).unwrap();
        assert!(arc_of(&whole).is_none());
        assert!(
            with_arc_angle(&shape, ArcEnd::End, deg(100.0)).is_none(),
            "unchanged"
        );
    }

    /// Criteria 3 and 31: kind and Whole act only on an arc; a curved ellipse has no arc.
    #[test]
    fn kind_whole_and_the_curve_exclusion() {
        let whole = ellipse(None, 1.0);
        assert!(with_arc_kind(&whole, ArcKind::Pie).is_none());
        assert!(made_whole(&whole).is_none());
        let arc = ellipse(EllipseArc::new(deg(10.0), deg(100.0), ArcKind::Arc), 1.0);
        let pie = with_arc_kind(&arc, ArcKind::Pie).unwrap();
        assert_eq!(arc_of(&pie).unwrap().kind, ArcKind::Pie);
        assert!(arc_of(&made_whole(&pie).unwrap()).is_none());
        let curved = ellipse(None, 0.5);
        assert!(with_arc_angle(&curved, ArcEnd::Start, deg(90.0)).is_none());
        assert!(with_curve(&arc, 0.0).is_none(), "an arc takes no curve");
        assert!(
            with_curve(&curved, 1.0).is_some(),
            "typing 100 makes it an ellipse"
        );
    }

    fn triangle(curve: f64) -> Shape {
        Shape::Polygon {
            frame: StarFrame {
                center: Point::new(0.0, 0.0),
                radius: Length::from_mm(20.0),
                angle: Angle::from_radians(0.0),
            },
            point_count: PointCount::new(3).unwrap(),
            curve: Curve::new(curve).unwrap(),
        }
    }

    /// Criteria 38 and 44: a value is limited to the shape's own range.
    #[test]
    fn a_curve_is_limited_per_shape() {
        let Some(Shape::Polygon { curve, .. }) = with_curve(&triangle(0.0), -0.9) else {
            panic!("a polygon")
        };
        assert!((curve.get() + 0.5).abs() < 1e-12);
        assert!(
            with_curve(&triangle(-0.8), -0.5).is_none(),
            "already effective"
        );
    }

    /// Criteria 23 and 41: snapping to the stops, Ctrl to 25 points or the limit.
    #[test]
    fn curve_drags_snap_to_the_stops() {
        let shape = ellipse(None, 1.0);
        let at = |k: f64| {
            let mut s = shape;
            if let Shape::Ellipse { curve, .. } = &mut s {
                *curve = Curve::new(k).unwrap();
            }
            curvyo_document_core::curve_handle_position(&s).unwrap()
        };
        assert_eq!(curve_from_pointer(&shape, at(0.995), false), Some(1.0));
        assert_eq!(curve_from_pointer(&shape, at(0.006), false), Some(0.0));
        let k = curve_from_pointer(&shape, at(-0.37), false).unwrap();
        assert!((k + 0.37).abs() < 1e-9);
        assert_eq!(curve_from_pointer(&shape, at(-0.37), true), Some(-0.25));
        let t = triangle(0.0);
        let limit_at = curvyo_document_core::curve_handle_position(&triangle(-0.5)).unwrap();
        let k = curve_from_pointer(&t, limit_at, true).unwrap();
        assert!((k + 0.5).abs() < 1e-9, "{k}");
    }
}
