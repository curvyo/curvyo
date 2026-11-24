//! The Rounded value of a polygon or star (`specs/0145-shape-spiro-handles/` criteria 1 to 3
//! and 5, `ux-notes.md` sections 2 and 3): where its knob rests, a pointer displacement turned
//! into a value, and a value applied to a shape, the one rule the drag, the entry chip and the
//! bar share through [`crate::apply_param`].

use curvyo_document_core::{Curve, Point, Q_C, Rounded, Shape, effective_curve, effective_rounded};

use crate::transform_handle_layout::TransformHandleTolerances;

/// A drag within this of 0 or 1 snaps to it without a modifier (criterion 2), or within 3
/// screen pixels when that is larger.
const ROUNDED_SNAP: f64 = 0.015;

/// The step of a Ctrl-snapped drag.
const ROUNDED_CTRL_STEP: f64 = 0.25;

/// A value within this of the current one is the same: nothing is written.
const ROUNDED_EQUAL_EPSILON: f64 = 1e-9;

/// The smallest unit (millimetres per 1.0) a drag can divide by.
const MIN_UNIT_MM: f64 = 1e-6;

/// Whether `shape` takes a Rounded value now (criteria 3, 14 and 19): a polygon, a star or an
/// ellipse without an arc (the arc wins).
#[must_use]
pub fn rounded_capable(shape: &Shape) -> bool {
    matches!(
        shape,
        Shape::Polygon { .. } | Shape::Star { .. } | Shape::Ellipse { arc: None, .. }
    )
}

/// The Rounded value the knob shows and a drag starts from: the effective one, except on an
/// untouched ellipse (`q = 0`, Curve 100 %), which is drawn as `q_c` already, so the first pixel of
/// a drag continues from the circle (criterion 14).
#[must_use]
pub fn rounded_drawn(shape: &Shape) -> Option<f64> {
    let q = effective_rounded(shape)?;
    let untouched_ellipse = matches!(shape, Shape::Ellipse { arc: None, .. })
        && q == 0.0
        && effective_curve(shape) == Some(Curve::ELLIPSE_DEFAULT);
    Some(if untouched_ellipse { Q_C } else { q })
}

/// The distance from vertex 0 to the centre and the vertex-0 point's x offset basis: a
/// polygon's or star's outer radius, an ellipse's `rx` (its vertex E).
fn vertex0(shape: &Shape) -> Option<(Point, f64)> {
    match shape {
        Shape::Polygon { frame, .. } | Shape::Star { frame, .. } => {
            Some((frame.center, frame.radius.as_mm()))
        }
        Shape::Ellipse {
            frame, arc: None, ..
        } => Some((frame.center, frame.rx.as_mm())),
        _ => None,
    }
}

/// The drag's unit `U = D - 15 px` in millimetres per 1.0 of `q` (`ux-notes.md` section 2):
/// `D` is the distance from vertex 0 to the centre, the outer radius. `None` for another shape
/// or one too small to drag.
#[must_use]
pub fn rounded_unit_mm(shape: &Shape, tolerances: &TransformHandleTolerances) -> Option<f64> {
    let (_, radius) = vertex0(shape)?;
    let unit = radius - tolerances.param_inset_mm;
    (unit > MIN_UNIT_MM).then_some(unit)
}

/// The snap window in `q` of a drag with the unit `unit_mm`: the larger of 0.015 and 3 screen
/// pixels (the drag threshold) over the unit.
#[must_use]
pub fn rounded_snap_window(unit_mm: f64, tolerances: &TransformHandleTolerances) -> f64 {
    (tolerances.drag_threshold_mm / unit_mm).max(ROUNDED_SNAP)
}

/// Where the Rounded knob rests in the box-local frame (vertex 0 at angle 0 from the centre):
/// on the ray from vertex 0 to the centre, `15 px + q·U` from vertex 0, at the effective `q`
/// (criterion 1). `None` for a shape without the knob.
#[must_use]
pub fn rounded_handle_position(
    shape: &Shape,
    tolerances: &TransformHandleTolerances,
) -> Option<Point> {
    let (center, radius) = vertex0(shape)?;
    let q = rounded_drawn(shape)?;
    let unit = rounded_unit_mm(shape, tolerances)?;
    let along = tolerances.param_inset_mm + q * unit;
    Some(Point::new(center.x + radius - along, center.y))
}

/// The value a drag asks for: `press` plus the pointer's displacement `local_dx` along the
/// box-local x axis (inward, toward the centre, is negative x) over the unit, snapped to 0 and
/// 1 within `window`, or with `ctrl` to steps of 0.25, and limited to -10 to 10 (criterion 2).
#[must_use]
pub fn rounded_from_drag(press: f64, local_dx: f64, unit_mm: f64, window: f64, ctrl: bool) -> f64 {
    rounded_from_drag_to(press, local_dx, unit_mm, window, ctrl, &[0.0, 1.0])
}

/// [`rounded_from_drag`] with the snap targets `stops`: an ellipse adds `q_c` to 0 and 1
/// (criterion 15), and the snap to `q_c` writes it exactly.
#[must_use]
pub fn rounded_from_drag_to(
    press: f64,
    local_dx: f64,
    unit_mm: f64,
    window: f64,
    ctrl: bool,
    stops: &[f64],
) -> f64 {
    let raw = press - local_dx / unit_mm;
    let snapped = if ctrl {
        (raw / ROUNDED_CTRL_STEP).round() * ROUNDED_CTRL_STEP
    } else {
        stops
            .iter()
            .copied()
            .find(|stop| (raw - stop).abs() <= window)
            .unwrap_or(raw)
    };
    snapped.clamp(-Rounded::RANGE, Rounded::RANGE)
}

/// `shape` with its Rounded value set to `value`, limited to -10 to 10. A value other than 0
/// also sets the shape's curve to 0 (criterion 5). `None` when the limited value equals the
/// effective one (nothing to write), when `value` is not finite and for a shape that takes no
/// Rounded value.
#[must_use]
pub fn with_rounded(shape: &Shape, value: f64) -> Option<Shape> {
    if !value.is_finite() {
        return None;
    }
    let limited = Rounded::clamped(value);
    // An untouched ellipse is drawn at `q_c`: a release there writes nothing (criterion 14).
    let drawn = rounded_drawn(shape)?;
    if (limited.get() - drawn).abs() <= ROUNDED_EQUAL_EPSILON {
        return None;
    }
    let curve = |old: Curve| {
        if limited.is_zero() {
            old
        } else {
            Curve::POLYGON_DEFAULT
        }
    };
    Some(match *shape {
        Shape::Polygon {
            frame,
            point_count,
            curve: old,
            randomized,
            ..
        } => Shape::Polygon {
            randomized,
            frame,
            point_count,
            curve: curve(old),
            rounded: limited,
        },
        Shape::Star {
            frame,
            point_count,
            inner_ratio,
            curve: old,
            randomized,
            ..
        } => Shape::Star {
            randomized,
            frame,
            point_count,
            inner_ratio,
            curve: curve(old),
            rounded: limited,
        },
        Shape::Ellipse {
            frame,
            arc: None,
            curve: old,
            ..
        } => Shape::Ellipse {
            frame,
            arc: None,
            curve: curve(old),
            rounded: limited,
        },
        Shape::Rect { .. } | Shape::Ellipse { arc: Some(_), .. } => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use curvyo_document_core::{Angle, Length, PointCount, StarFrame};

    fn hexagon(q: f64, curve: f64) -> Shape {
        Shape::Polygon {
            randomized: curvyo_document_core::Randomized::ZERO,
            frame: StarFrame {
                center: Point::new(10.0, 20.0),
                radius: Length::from_mm(40.0),
                angle: Angle::from_radians(0.0),
            },
            point_count: PointCount::new(6).unwrap(),
            curve: Curve::new(curve).unwrap(),
            rounded: Rounded::new(q).unwrap(),
        }
    }

    /// 1 px per mm: the 15 px inset is 15 mm, the drag threshold 3 mm.
    fn tolerances() -> TransformHandleTolerances {
        TransformHandleTolerances::at_scale(1.0)
    }

    /// Criterion 1 and 2: at q = 0 the knob is 15 px inside vertex 0, at q = 1 at the centre,
    /// at q = -1 outside vertex 0 by U.
    #[test]
    fn the_knob_rests_on_the_ray_and_q_one_is_the_centre() {
        let t = tolerances();
        let at = |q: f64| rounded_handle_position(&hexagon(q, 0.0), &t).unwrap();
        assert!((at(0.0).x - (10.0 + 40.0 - 15.0)).abs() < 1e-9);
        assert!((at(1.0).x - 10.0).abs() < 1e-9, "the centre");
        assert!((at(2.0).x - (10.0 - 25.0)).abs() < 1e-9, "past the centre");
        assert!((at(-1.0).x - (10.0 + 40.0 - 15.0 + 25.0)).abs() < 1e-9);
        assert!((at(1.0).y - 20.0).abs() < 1e-9);
    }

    /// Criterion 2: relative to the press, no stop at the centre, snaps, Ctrl, limits.
    #[test]
    fn a_drag_is_relative_snaps_and_runs_through_the_centre() {
        let (u, w) = (25.0, 0.015);
        assert!((rounded_from_drag(0.5, -25.0, u, w, false) - 1.5).abs() < 1e-12);
        assert_eq!(
            rounded_from_drag(0.0, -24.8, u, w, false),
            1.0,
            "snaps to 1"
        );
        assert_eq!(rounded_from_drag(0.0, 0.2, u, w, false), 0.0, "snaps to 0");
        assert!((rounded_from_drag(0.0, -60.0, u, w, false) - 2.4).abs() < 1e-12);
        assert_eq!(rounded_from_drag(0.0, -11.0, u, w, true), 0.5);
        assert_eq!(rounded_from_drag(0.0, -9999.0, u, w, false), 10.0);
        assert_eq!(rounded_from_drag(0.0, 9999.0, u, w, false), -10.0);
        let window = rounded_snap_window(25.0, &tolerances());
        assert!((window - 0.12).abs() < 1e-12, "3 px at U = 25 px: {window}");
    }

    /// Criteria 3, 5 and 11: limited, equal values write nothing, a value clears the curve.
    #[test]
    fn a_value_is_limited_and_clears_the_curve() {
        let Some(Shape::Polygon { rounded, curve, .. }) = with_rounded(&hexagon(0.0, 0.5), 25.0)
        else {
            panic!("a polygon")
        };
        assert_eq!((rounded.get(), curve), (10.0, Curve::POLYGON_DEFAULT));
        assert!(
            with_rounded(&hexagon(10.0, 0.0), 12.0).is_none(),
            "already at the limit"
        );
        assert!(with_rounded(&hexagon(0.0, 0.0), f64::NAN).is_none());
        let Some(Shape::Polygon { rounded, curve, .. }) = with_rounded(&hexagon(2.0, 0.0), 0.0)
        else {
            panic!("a polygon")
        };
        assert_eq!((rounded, curve.get()), (Rounded::ZERO, 0.0));
    }
}
