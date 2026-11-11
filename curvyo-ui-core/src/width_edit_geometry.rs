//! Maps mouse width handles to normalised centre-line positions and multipliers.

use curvyo_document_core::{Length, Point, Vec2};
use curvyo_geometry_core::PathMeasure;

/// Minimum measurable centre-line length in millimetres, matching `PathMeasure`.
const MIN_LENGTH_MM: f64 = 1e-9;

fn valid_path(path: &PathMeasure) -> bool {
    let length = path.length().as_mm();
    length.is_finite() && length > MIN_LENGTH_MM
}

fn sample(path: &PathMeasure, position: f64) -> Option<curvyo_geometry_core::PathSample> {
    if !valid_path(path) || !position.is_finite() || !(0.0..=1.0).contains(&position) {
        return None;
    }
    Some(path.at(Length::from_mm(path.length().as_mm() * position)))
}

/// Finds the normalised arc-length position nearest the mouse.
#[must_use]
pub fn width_position_at(path: &PathMeasure, pointer: Point) -> Option<f64> {
    if !valid_path(path) || !pointer.x.is_finite() || !pointer.y.is_finite() {
        return None;
    }
    let result = path.nearest(pointer).as_mm() / path.length().as_mm();
    if path.is_closed() && result >= 1.0 {
        Some(0.0)
    } else {
        Some(result)
    }
}

/// Finds the positive-normal handle for the given full-width multiplier.
#[must_use]
pub fn width_handle(
    path: &PathMeasure,
    position: f64,
    base: Length,
    multiplier: f64,
) -> Option<Point> {
    let width = base.as_mm() * multiplier;
    if !width.is_finite() || base.as_mm() < 0.0 || multiplier < 0.0 {
        return None;
    }
    let at = sample(path, position)?;
    let handle = at
        .point
        .translated(Vec2::new(-at.tangent.y, at.tangent.x).scaled(width / 2.0));
    (handle.x.is_finite() && handle.y.is_finite()).then_some(handle)
}

/// Computes the full-width multiplier from the mouse's distance across the centre line.
#[must_use]
pub fn width_multiplier_at_handle(
    path: &PathMeasure,
    position: f64,
    base: Length,
    pointer: Point,
) -> Option<f64> {
    let base = base.as_mm();
    if !base.is_finite() || base <= 0.0 || !pointer.x.is_finite() || !pointer.y.is_finite() {
        return None;
    }
    let at = sample(path, position)?;
    let delta = at.point.vector_to(pointer);
    let distance = delta.x.mul_add(-at.tangent.y, delta.y * at.tangent.x).abs();
    let multiplier = 2.0 * distance / base;
    multiplier.is_finite().then_some(multiplier)
}
