//! Where the Offset tool's four handles sit and what distance a handle drag means
//! (`specs/0038-path-offset` criteria 9, 10, 11): the box, the handle positions, the distance
//! projected on a handle's outward diagonal, and its snapping.

use curvyo_document_core::{Angle, ObjectSnapshot, Point, Vec2};

use crate::object_bounds::object_outline_bounds;
use crate::offset_entry::OFFSET_MAX_MM;
use crate::oriented_box::{OrientedBox, oriented_bounds};

/// How far a handle sits outside its corner, in screen pixels.
pub const HANDLE_OUTSIDE_PX: f64 = 12.0;

/// The smallest extent of the box, in screen pixels, for placing the handles.
pub const MIN_BOX_PX: f64 = 24.0;

/// A handle is hit within this radius, in screen pixels (16 px hit area).
pub const HANDLE_HIT_RADIUS_PX: f64 = 8.0;

/// The snap steps of a drag, in millimetres (criterion 11).
const SNAP_SERIES_MM: [f64; 9] = [0.01, 0.02, 0.05, 0.1, 0.2, 0.5, 1.0, 2.0, 5.0];

/// The smallest snap step on screen, in pixels.
const MIN_STEP_PX: f64 = 2.0;

/// A drag snaps to the bar's value within this distance of the pointer, in pixels.
const SNAP_TO_BAR_PX: f64 = 6.0;

/// The four corners, as their outward directions in the box frame: top-left, top-right,
/// bottom-right, bottom-left.
pub const CORNER_SIGNS: [(f64, f64); 4] = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)];

/// The box the handles sit on (criterion 9): one object's own rotated box, otherwise the
/// axis-aligned box around all of them. `None` without objects.
#[must_use]
pub fn offset_box(objects: &[&ObjectSnapshot]) -> Option<OrientedBox> {
    match objects {
        [] => None,
        [one] => Some(oriented_bounds(one)),
        many => {
            let (mut min, mut max) = object_outline_bounds(many[0]);
            for object in &many[1..] {
                let (a, b) = object_outline_bounds(object);
                min = Point::new(min.x.min(a.x), min.y.min(a.y));
                max = Point::new(max.x.max(b.x), max.y.max(b.y));
            }
            Some(OrientedBox {
                min,
                max,
                angle: Angle::from_radians(0.0),
                pivot: Point::new(f64::midpoint(min.x, max.x), f64::midpoint(min.y, max.y)),
            })
        }
    }
}

/// The four handle positions in document space, in the order of [`CORNER_SIGNS`]: each
/// `HANDLE_OUTSIDE_PX` outside its corner on the outward diagonal, with an axis narrower than
/// `MIN_BOX_PX` on screen widened to it. `scale` is screen pixels per millimetre.
#[must_use]
pub fn handle_positions(bounds: &OrientedBox, scale: f64) -> [Point; 4] {
    let center = bounds.local_center();
    let min_half = MIN_BOX_PX / 2.0 / scale;
    let half = Vec2::new(
        (bounds.width() / 2.0).max(min_half),
        (bounds.height() / 2.0).max(min_half),
    );
    let outside = HANDLE_OUTSIDE_PX / scale / std::f64::consts::SQRT_2;
    CORNER_SIGNS.map(|(sx, sy)| {
        bounds.to_document(Point::new(
            center.x + sx * (half.x + outside),
            center.y + sy * (half.y + outside),
        ))
    })
}

/// The distance a drag of the handle at corner `signs` by `delta` (document millimetres) means:
/// the movement projected on the outward diagonal, `(dx sx + dy sy) / 2` in the box frame,
/// positive outward, limited to 1000 mm (criterion 10).
#[must_use]
pub fn drag_distance(angle: Angle, signs: (f64, f64), delta: Vec2) -> f64 {
    let local = delta.rotated(Angle::from_radians(-angle.as_radians()));
    let d = f64::midpoint(local.x * signs.0, local.y * signs.1);
    d.clamp(-OFFSET_MAX_MM, OFFSET_MAX_MM)
}

/// The snap step at `scale` screen pixels per millimetre: the smallest of the series that is at
/// least 2 pixels on screen (criterion 11).
#[must_use]
pub fn snap_step(scale: f64) -> f64 {
    SNAP_SERIES_MM
        .into_iter()
        .find(|step| step * scale >= MIN_STEP_PX)
        .unwrap_or(SNAP_SERIES_MM[SNAP_SERIES_MM.len() - 1])
}

/// `d` snapped: to the bar's value `bar_mm` (either sign) when the pointer is within 6 pixels of
/// where that value puts it, otherwise to the step of [`snap_step`].
#[must_use]
pub fn snap_distance(d: f64, scale: f64, bar_mm: f64) -> f64 {
    // The pointer moves `sqrt(2)` times `d` along the diagonal.
    let off_bar_px = (d.abs() - bar_mm).abs() * std::f64::consts::SQRT_2 * scale;
    if off_bar_px <= SNAP_TO_BAR_PX && d != 0.0 {
        return bar_mm.copysign(d);
    }
    let step = snap_step(scale);
    let snapped = (d / step).round() * step;
    // Round away the float noise of the multiplication (steps are whole hundredths).
    (snapped * 100.0).round() / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 100 % zoom: 96 pixels per inch.
    const ZOOM_100: f64 = 96.0 / 25.4;

    fn square_box() -> OrientedBox {
        OrientedBox {
            min: Point::new(0.0, 0.0),
            max: Point::new(20.0, 10.0),
            angle: Angle::from_radians(0.0),
            pivot: Point::new(10.0, 5.0),
        }
    }

    #[test]
    fn ac9_handles_sit_twelve_pixels_outside_on_the_diagonal() {
        let handles = handle_positions(&square_box(), 10.0);
        let out = 1.2 / std::f64::consts::SQRT_2;
        assert!((handles[2].x - (20.0 + out)).abs() < 1e-9);
        assert!((handles[2].y - (10.0 + out)).abs() < 1e-9);
        assert!((handles[0].x + out).abs() < 1e-9);
    }

    #[test]
    fn ac9_a_thin_box_is_widened_for_the_handles() {
        let line = OrientedBox {
            min: Point::new(0.0, 5.0),
            max: Point::new(20.0, 5.0),
            angle: Angle::from_radians(0.0),
            pivot: Point::new(10.0, 5.0),
        };
        let handles = handle_positions(&line, 1.0);
        let out = 12.0 / std::f64::consts::SQRT_2;
        assert!((handles[0].y - (5.0 - 12.0 - out)).abs() < 1e-9);
    }

    #[test]
    fn ac10_the_distance_is_the_projection_on_the_diagonal() {
        let angle = Angle::from_radians(0.0);
        assert!((drag_distance(angle, (1.0, 1.0), Vec2::new(1.0, 1.0)) - 1.0).abs() < 1e-12);
        assert!((drag_distance(angle, (-1.0, -1.0), Vec2::new(-1.0, -1.0)) - 1.0).abs() < 1e-12);
        assert!((drag_distance(angle, (1.0, 1.0), Vec2::new(-0.5, -0.5)) + 0.5).abs() < 1e-12);
        assert!((drag_distance(angle, (1.0, 1.0), Vec2::new(5000.0, 0.0)) - 1000.0).abs() < 1e-12);
        let turned = Angle::from_radians(std::f64::consts::FRAC_PI_2);
        // In a box turned by 90°, the local x axis points down the screen.
        assert!((drag_distance(turned, (1.0, 1.0), Vec2::new(-1.0, 1.0)) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn ac11_the_step_is_at_least_two_pixels() {
        assert!((snap_step(ZOOM_100) - 1.0).abs() < 1e-12);
        assert!((snap_step(4.0 * ZOOM_100) - 0.2).abs() < 1e-12);
        assert!((snap_distance(2.4, ZOOM_100, 50.0) - 2.0).abs() < 1e-12);
        assert!((snap_distance(-0.33, 4.0 * ZOOM_100, 50.0) + 0.4).abs() < 1e-12);
    }

    #[test]
    fn ac11_the_bar_value_catches_a_near_pointer() {
        assert!((snap_distance(2.45, 4.0 * ZOOM_100, 2.5) - 2.5).abs() < 1e-12);
        assert!((snap_distance(-2.45, 4.0 * ZOOM_100, 2.5) + 2.5).abs() < 1e-12);
        assert!((snap_distance(1.95, 4.0 * ZOOM_100, 2.5) - 2.0).abs() < 1e-12);
    }
}
