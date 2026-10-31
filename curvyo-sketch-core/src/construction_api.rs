//! Public construction helpers return solved geometry for noninteractive callers.

use crate::{EntityId, Length, Position, Sketch, SketchError};

/// Creates connected segments whose endpoints are structurally shared.
///
/// # Errors
/// Rejects fewer than two vertices, degenerate edges or colliding IDs.
pub fn create_polyline(
    seed: &Sketch,
    prefix: &str,
    points: &[Position],
    closed: bool,
) -> Result<Sketch, SketchError> {
    crate::construction::finish(&crate::primitives::create_polyline(
        seed, prefix, points, closed,
    )?)
}

/// Creates a circle from centre and radius.
///
/// # Errors
/// Rejects invalid geometry, colliding IDs or an already inconsistent sketch.
pub fn create_circle(
    seed: &Sketch,
    prefix: &str,
    center: Position,
    radius: Length,
) -> Result<Sketch, SketchError> {
    crate::construction::finish(&crate::primitives::create_circle(
        seed, prefix, center, radius,
    )?)
}

/// Creates the unique circle through three non-collinear construction picks.
///
/// # Errors
/// Rejects coincident or collinear picks and invalid construction output.
pub fn create_circle_three_points(
    seed: &Sketch,
    prefix: &str,
    points: [Position; 3],
) -> Result<Sketch, SketchError> {
    crate::construction::finish(&crate::primitives::create_circle_three_points(
        seed, prefix, points,
    )?)
}

/// Creates an ellipse from its centre, first axis endpoint and second semi-axis.
///
/// # Errors
/// Rejects zero/non-finite axes or colliding IDs.
pub fn create_ellipse(
    seed: &Sketch,
    prefix: &str,
    center: Position,
    axis_end: Position,
    second_axis: Length,
) -> Result<Sketch, SketchError> {
    crate::construction::finish(&crate::primitives::create_ellipse(
        seed,
        prefix,
        center,
        axis_end,
        second_axis,
    )?)
}

/// Creates an ellipse through a third pick using the first two axis endpoints.
///
/// # Errors
/// Rejects a third point outside the defining axis slab or on that axis.
pub fn create_ellipse_three_points(
    seed: &Sketch,
    prefix: &str,
    first: Position,
    second: Position,
    third: Position,
) -> Result<Sketch, SketchError> {
    crate::construction::finish(&crate::primitives::create_ellipse_three_points(
        seed, prefix, first, second, third,
    )?)
}

/// Creates an axis-aligned rectangle with persistent horizontal/vertical intent.
///
/// # Errors
/// Rejects degenerate corners, colliding IDs or inconsistent input geometry.
pub fn create_rectangle(
    seed: &Sketch,
    prefix: &str,
    first: Position,
    opposite: Position,
) -> Result<Sketch, SketchError> {
    crate::construction::finish(&crate::primitives::create_rectangle(
        seed, prefix, first, opposite,
    )?)
}

/// Creates the circular arc through three picks, retaining analytic endpoints.
///
/// # Errors
/// Rejects collinear/coincident picks or colliding IDs.
pub fn create_arc_three_points(
    seed: &Sketch,
    prefix: &str,
    points: [Position; 3],
) -> Result<Sketch, SketchError> {
    crate::construction::finish(&crate::primitives::create_arc_three_points(
        seed, prefix, points,
    )?)
}

/// Constructs a straight slot from its cap centres and total width.
///
/// # Errors
/// Rejects coincident centres, invalid width, ID collisions or unsatisfied constraints.
pub fn create_slot(
    seed: &Sketch,
    prefix: &str,
    start: Position,
    end: Position,
    width: Length,
) -> Result<Sketch, SketchError> {
    crate::construction::finish(&crate::construction::create_slot(
        seed, prefix, start, end, width,
    )?)
}

/// Rounds two joined segments while retaining their original dimensional intent.
///
/// Original segments become construction references; all their IDs and constraints
/// remain intact. New trimmed segments share tangent endpoints with the new arc.
///
/// # Errors
/// Rejects non-shared or straight corners, oversized radii, ID collisions and
/// constraints that cannot be preserved. The input remains unchanged on failure.
pub fn fillet(
    seed: &Sketch,
    prefix: &str,
    first: &EntityId,
    second: &EntityId,
    radius: Length,
) -> Result<Sketch, SketchError> {
    crate::construction::finish(&crate::fillet::fillet(seed, prefix, first, second, radius)?)
}
