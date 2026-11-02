//! The four canvas handles of a circle tiling (`specs/0069-tiling` criterion 14, UX notes §12.4):
//! where each one sits, which one a press picks, and the angle a start or sweep drag gives.
//!
//! Angles run counterclockwise on screen (Y down), 0 pointing right, as in the layout.

use std::f64::consts::{PI, TAU};

use curvyo_document_core::{Angle, CircleOptions, Point};

use crate::angle_snap::snap_angle;

/// The sweep knob's rest distance before the start knob at a full turn, screen pixels (the arc
/// handles' 14 px rule).
const FULL_TURN_REST_PX: f64 = 14.0;
/// A sweep drag this close to the full turn, screen pixels of arc, snaps to exactly 360°.
const FULL_TURN_SNAP_PX: f64 = 5.0;
/// A sweep this close to a full turn counts as one.
const FULL_TURN_TOLERANCE_RAD: f64 = 1e-9;

/// One handle of the circle (UX notes §12.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircleHandle {
    /// The circle's centre.
    Centre,
    /// On the circle at the middle of the arc.
    Radius,
    /// On the circle at the start angle.
    Start,
    /// On the circle at start + sweep.
    Sweep,
}

impl CircleHandle {
    /// The pick order on a tie: Sweep, Start, Radius, Centre.
    const TIE_ORDER: [Self; 4] = [Self::Sweep, Self::Start, Self::Radius, Self::Centre];
}

/// The circle as the canvas draws it: centre in document space, radius in millimetres, start and
/// sweep, and the millimetres one screen pixel spans.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CircleView {
    /// The centre in document space.
    pub centre: Point,
    /// The radius, millimetres.
    pub radius_mm: f64,
    /// The start angle.
    pub start: Angle,
    /// The swept angle, -360° to 360°.
    pub sweep: Angle,
    /// Millimetres per screen pixel at the current zoom.
    pub mm_per_px: f64,
}

impl CircleView {
    /// The view of `circle` around `centre` at `mm_per_px`.
    #[must_use]
    pub fn of(circle: &CircleOptions, centre: Point, mm_per_px: f64) -> Self {
        Self {
            centre,
            radius_mm: circle.radius.as_mm(),
            start: circle.start,
            sweep: circle.sweep,
            mm_per_px,
        }
    }

    /// The point at `angle` (radians) on the circle.
    #[must_use]
    pub fn on_circle(&self, angle: f64) -> Point {
        Point::new(
            self.radius_mm.mul_add(angle.cos(), self.centre.x),
            self.radius_mm.mul_add(-angle.sin(), self.centre.y),
        )
    }

    /// Whether the sweep is a full turn.
    #[must_use]
    pub fn full_turn(&self) -> bool {
        self.sweep.as_radians().abs() >= TAU - FULL_TURN_TOLERANCE_RAD
    }

    /// The angle (radians) at which the sweep knob is drawn: start + sweep, or at a full turn
    /// 14 px of arc before the start knob, so the two never lie on each other.
    #[must_use]
    pub fn sweep_knob_angle(&self) -> f64 {
        let (start, sweep) = (self.start.as_radians(), self.sweep.as_radians());
        if !self.full_turn() || self.radius_mm <= 0.0 {
            return start + sweep;
        }
        let back = (FULL_TURN_REST_PX * self.mm_per_px / self.radius_mm).min(PI);
        start + sweep - back.copysign(sweep)
    }

    /// Where `handle` sits (UX notes §12.4): the radius handle at the middle of the arc, which
    /// at a full turn is opposite the start.
    #[must_use]
    pub fn handle_point(&self, handle: CircleHandle) -> Point {
        let start = self.start.as_radians();
        match handle {
            CircleHandle::Centre => self.centre,
            CircleHandle::Radius => self.on_circle(start + self.sweep.as_radians() / 2.0),
            CircleHandle::Start => self.on_circle(start),
            CircleHandle::Sweep => self.on_circle(self.sweep_knob_angle()),
        }
    }

    /// The handle within `reach_px` screen pixels of `point`: the nearest, on a tie in the
    /// order Sweep, Start, Radius, Centre.
    #[must_use]
    pub fn pick(&self, point: Point, reach_px: f64) -> Option<CircleHandle> {
        let reach = reach_px * self.mm_per_px;
        CircleHandle::TIE_ORDER
            .into_iter()
            .map(|handle| (handle, self.handle_point(handle).vector_to(point).length()))
            .filter(|(_, distance)| *distance <= reach)
            .fold(
                None,
                |best: Option<(CircleHandle, f64)>, (handle, distance)| match best {
                    Some((_, nearest)) if nearest <= distance => best,
                    _ => Some((handle, distance)),
                },
            )
            .map(|(handle, _)| handle)
    }

    /// The screen angle (radians) of `point` seen from the centre.
    fn pointer_angle(&self, point: Point) -> f64 {
        let d = self.centre.vector_to(point);
        // `0.0 - y`, not `-y`: a pointer on the axis to the left is +180°, never -180°.
        (0.0 - d.y).atan2(d.x)
    }

    /// The start angle a start drag to `point` gives, normalised to -180° to 180°; with
    /// `ctrl` the nearest stop of 15° and 22.5°. The sweep is kept.
    #[must_use]
    pub fn dragged_start(&self, point: Point, ctrl: bool) -> Angle {
        let raw = Angle::from_radians(self.pointer_angle(point));
        let angle = if ctrl { snap_angle(raw) } else { raw };
        normalised_start(angle)
    }

    /// The sweep a sweep drag to `point` gives (UX notes §12.4): the pointer angle relative to
    /// the start, unwrapped continuously from the current sweep (so it never flips at a full
    /// turn), clamped to -360° to 360°; within 5 px of arc of the full turn it is exactly 360°;
    /// with `ctrl` the nearest stop of 15° and 22.5°.
    #[must_use]
    pub fn dragged_sweep(&self, point: Point, ctrl: bool) -> Angle {
        let current = self.sweep.as_radians();
        let relative = self.pointer_angle(point) - self.start.as_radians();
        let turns = ((current - relative) / TAU).round();
        let unwrapped = turns.mul_add(TAU, relative).clamp(-TAU, TAU);
        let swept = if ctrl {
            snap_angle(Angle::from_radians(unwrapped)).as_radians()
        } else {
            unwrapped
        };
        let snap = if self.radius_mm > 0.0 {
            FULL_TURN_SNAP_PX * self.mm_per_px / self.radius_mm
        } else {
            0.0
        };
        let full = if swept < 0.0 { -TAU } else { TAU };
        Angle::from_radians(if (full - swept).abs() <= snap {
            full
        } else {
            swept
        })
    }
}

/// `angle` brought into -180° to 180° (UX notes §12.4: Start is shown in that range); an angle
/// already inside keeps its bits.
#[must_use]
pub fn normalised_start(angle: Angle) -> Angle {
    let radians = angle.as_radians();
    if (-PI..=PI).contains(&radians) || !radians.is_finite() {
        return angle;
    }
    let wrapped = (radians + PI).rem_euclid(TAU) - PI;
    Angle::from_radians(if wrapped <= -PI { PI } else { wrapped })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn deg(d: f64) -> Angle {
        Angle::from_radians(d.to_radians())
    }

    fn view(start: f64, sweep: f64) -> CircleView {
        CircleView {
            centre: Point::new(0.0, 0.0),
            radius_mm: 100.0,
            start: deg(start),
            sweep: deg(sweep),
            mm_per_px: 1.0,
        }
    }

    fn close(a: Point, b: Point) -> bool {
        a.vector_to(b).length() < 1e-9
    }

    fn assert_deg(angle: Angle, expected: f64) {
        let got = angle.as_radians().to_degrees();
        assert!((got - expected).abs() < 1e-9, "{got} != {expected}");
    }

    #[test]
    fn the_radius_handle_sits_at_the_middle_of_the_arc() {
        let full = view(0.0, 360.0);
        assert!(close(
            full.handle_point(CircleHandle::Radius),
            Point::new(-100.0, 0.0)
        ));
        let quarter = view(0.0, 180.0);
        // 90° counterclockwise on screen is up (negative Y).
        assert!(close(
            quarter.handle_point(CircleHandle::Radius),
            Point::new(0.0, -100.0)
        ));
        assert!(close(
            quarter.handle_point(CircleHandle::Start),
            Point::new(100.0, 0.0)
        ));
        assert!(close(
            quarter.handle_point(CircleHandle::Sweep),
            Point::new(-100.0, 0.0)
        ));
    }

    #[test]
    fn at_a_full_turn_the_sweep_knob_rests_14_px_before_the_start() {
        let full = view(0.0, 360.0);
        let at = full.handle_point(CircleHandle::Sweep);
        let start = full.handle_point(CircleHandle::Start);
        let arc = (at.vector_to(start).length() / 200.0).asin() * 200.0;
        assert!((arc - 14.0).abs() < 1e-6, "arc {arc}");
        // Before the start: just below the x axis (clockwise of it).
        assert!(at.y > 0.0);
        let backwards = view(0.0, -360.0);
        assert!(backwards.handle_point(CircleHandle::Sweep).y < 0.0);
    }

    #[test]
    fn a_press_picks_the_nearest_handle_and_ties_go_to_sweep_first() {
        let v = view(0.0, 90.0);
        assert_eq!(
            v.pick(Point::new(105.0, 0.0), 12.0),
            Some(CircleHandle::Start)
        );
        assert_eq!(
            v.pick(Point::new(3.0, 3.0), 12.0),
            Some(CircleHandle::Centre)
        );
        assert_eq!(v.pick(Point::new(50.0, 50.0), 12.0), None);
        // A tiny circle: every handle lies on the centre, Sweep wins.
        let tiny = CircleView {
            radius_mm: 0.0,
            ..v
        };
        assert_eq!(
            tiny.pick(Point::new(0.0, 0.0), 12.0),
            Some(CircleHandle::Sweep)
        );
    }

    #[test]
    fn a_start_drag_follows_the_pointer_and_snaps_with_ctrl() {
        let v = view(0.0, 120.0);
        assert_deg(v.dragged_start(Point::new(0.0, -50.0), false), 90.0);
        let near_30 = Point::new(31f64.to_radians().cos(), -(31f64.to_radians().sin()));
        assert_deg(v.dragged_start(near_30, true), 30.0);
        assert_deg(v.dragged_start(Point::new(-10.0, 0.0), false), 180.0);
    }

    #[test]
    fn a_sweep_drag_unwraps_clamps_and_snaps_to_the_full_turn() {
        // From 350°, a pointer at -5° (just below the axis) means 355°, not -5°.
        let v = view(0.0, 350.0);
        let at = |d: f64| Point::new(100.0 * d.to_radians().cos(), -100.0 * d.to_radians().sin());
        assert_deg(v.dragged_sweep(at(-5.0), false), 355.0);
        // 2° short of the full turn is 3.5 px of arc at radius 100: snaps to 360.
        assert_deg(v.dragged_sweep(at(-2.0), false), 360.0);
        // Past the start from a full turn: clamped, never flipped.
        let full = view(0.0, 360.0);
        assert_deg(full.dragged_sweep(at(10.0), false), 360.0);
        // A small sweep crossing the start goes negative continuously.
        let small = view(0.0, 10.0);
        assert_deg(small.dragged_sweep(at(-20.0), false), -20.0);
        assert_deg(small.dragged_sweep(at(47.0), true), 45.0);
    }

    #[test]
    fn start_is_normalised_to_the_half_turn_range() {
        assert_deg(normalised_start(deg(270.0)), -90.0);
        assert_deg(normalised_start(deg(-190.0)), 170.0);
        assert_deg(normalised_start(deg(540.0)), 180.0);
        let inside = deg(-180.0);
        assert_eq!(normalised_start(inside), inside);
    }
}
