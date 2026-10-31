//! Construction variants produce the same canonical analytic entities.
use crate::construction::insert;
use crate::{Entity, EntityId, Length, Position, Sketch, SketchError};

/// Creates connected segments whose endpoints are structurally shared.
///
/// # Errors
/// Rejects fewer than two vertices, degenerate edges or colliding IDs.
pub(crate) fn create_polyline(
    seed: &Sketch,
    prefix: &str,
    points: &[Position],
    closed: bool,
) -> Result<Sketch, SketchError> {
    if points.len() < 2 {
        return Err(invalid("polyline needs two points"));
    }
    let mut s = seed.clone();
    let id = |i: usize| EntityId::from(format!("{prefix}:p{i}"));
    for (i, position) in points.iter().enumerate() {
        insert(
            &mut s,
            id(i),
            Entity::Point {
                position: *position,
                fixed: false,
            },
        )?;
    }
    let edges = if closed {
        points.len()
    } else {
        points.len() - 1
    };
    for i in 0..edges {
        let next = (i + 1) % points.len();
        if points[i].distance(points[next]).0 <= 1e-9 {
            return Err(invalid("zero-length polyline edge"));
        }
        insert(
            &mut s,
            format!("{prefix}:line{i}").into(),
            Entity::Line {
                start: id(i),
                end: id(next),
            },
        )?;
    }
    Ok(s)
}
/// Creates a circle from centre and radius.
///
/// # Errors
/// Rejects invalid geometry, colliding IDs or an already inconsistent sketch.
pub(crate) fn create_circle(
    seed: &Sketch,
    prefix: &str,
    center: Position,
    radius: Length,
) -> Result<Sketch, SketchError> {
    let mut s = seed.clone();
    let center_id = EntityId::from(format!("{prefix}:center"));
    insert(
        &mut s,
        center_id.clone(),
        Entity::Point {
            position: center,
            fixed: false,
        },
    )?;
    insert(
        &mut s,
        format!("{prefix}:curve").into(),
        Entity::Circle {
            center: center_id,
            radius,
            fixed_radius: false,
        },
    )?;
    Ok(s)
}
/// Creates the unique circle through three non-collinear construction picks.
///
/// # Errors
/// Rejects coincident or collinear picks and invalid construction output.
pub(crate) fn create_circle_three_points(
    seed: &Sketch,
    prefix: &str,
    points: [Position; 3],
) -> Result<Sketch, SketchError> {
    let [origin, second, third] = points;
    let dx = second.x.0 - origin.x.0;
    let dy = second.y.0 - origin.y.0;
    let ex = third.x.0 - origin.x.0;
    let ey = third.y.0 - origin.y.0;
    let determinant = 2.0 * (dx * ey - dy * ex);
    let scale = dx.hypot(dy) * ex.hypot(ey);
    if !scale.is_finite() || scale <= 1e-18 || determinant.abs() <= 1e-12 * scale {
        return Err(invalid("circle picks are collinear or coincident"));
    }
    let d2 = dx * dx + dy * dy;
    let e2 = ex * ex + ey * ey;
    let center = Position::mm(
        origin.x.0 + (ey * d2 - dy * e2) / determinant,
        origin.y.0 + (dx * e2 - ex * d2) / determinant,
    );
    create_circle(seed, prefix, center, center.distance(origin))
}
/// Creates an ellipse from its centre, first axis endpoint and second semi-axis.
///
/// # Errors
/// Rejects zero/non-finite axes or colliding IDs.
pub(crate) fn create_ellipse(
    seed: &Sketch,
    prefix: &str,
    center: Position,
    axis_end: Position,
    second_axis: Length,
) -> Result<Sketch, SketchError> {
    let first = center.distance(axis_end).0;
    if !first.is_finite() || first <= 1e-9 || !second_axis.0.is_finite() || second_axis.0 <= 1e-9 {
        return Err(invalid("ellipse axes must be finite and positive"));
    }
    let mut direction = (
        (axis_end.x.0 - center.x.0) / first,
        (axis_end.y.0 - center.y.0) / first,
    );
    let (major, minor) = if first >= second_axis.0 {
        (first, second_axis.0)
    } else {
        direction = (-direction.1, direction.0);
        (second_axis.0, first)
    };
    let distance = ((major - minor) * (major + minor)).sqrt();
    let focus = Position::mm(
        center.x.0 + distance * direction.0,
        center.y.0 + distance * direction.1,
    );
    let mut s = seed.clone();
    let center_id = EntityId::from(format!("{prefix}:center"));
    let focus_id = EntityId::from(format!("{prefix}:focus"));
    insert(
        &mut s,
        center_id.clone(),
        Entity::Point {
            position: center,
            fixed: false,
        },
    )?;
    insert(
        &mut s,
        focus_id.clone(),
        Entity::Point {
            position: focus,
            fixed: false,
        },
    )?;
    insert(
        &mut s,
        format!("{prefix}:curve").into(),
        Entity::Ellipse {
            center: center_id,
            focus: focus_id,
            minor_radius: Length(minor),
            fixed_radius: false,
        },
    )?;
    Ok(s)
}
/// Creates an ellipse through a third pick using the first two axis endpoints.
///
/// # Errors
/// Rejects a third point outside the defining axis slab or on that axis.
pub(crate) fn create_ellipse_three_points(
    seed: &Sketch,
    prefix: &str,
    first: Position,
    second: Position,
    third: Position,
) -> Result<Sketch, SketchError> {
    let center = Position::mm(
        first.x.0.midpoint(second.x.0),
        first.y.0.midpoint(second.y.0),
    );
    let major = center.distance(second).0;
    if major <= 1e-9 || !major.is_finite() {
        return Err(invalid("ellipse endpoints coincide"));
    }
    let direction = (
        (second.x.0 - center.x.0) / major,
        (second.y.0 - center.y.0) / major,
    );
    let dx = third.x.0 - center.x.0;
    let dy = third.y.0 - center.y.0;
    let x = dx * direction.0 + dy * direction.1;
    let y = -dx * direction.1 + dy * direction.0;
    let remainder = 1.0 - (x / major).powi(2);
    if remainder <= 1e-12 || y.abs() <= 1e-9 {
        return Err(invalid("third point does not define a finite ellipse"));
    }
    create_ellipse(
        seed,
        prefix,
        center,
        second,
        Length(y.abs() / remainder.sqrt()),
    )
}
fn invalid(message: &str) -> SketchError {
    SketchError::Invalid(message.into())
}

/// Creates an axis-aligned rectangle with persistent horizontal/vertical intent.
///
/// # Errors
/// Rejects degenerate corners, colliding IDs or inconsistent input geometry.
pub(crate) fn create_rectangle(
    seed: &Sketch,
    prefix: &str,
    first: Position,
    opposite: Position,
) -> Result<Sketch, SketchError> {
    let points = [
        first,
        Position::mm(opposite.x.0, first.y.0),
        opposite,
        Position::mm(first.x.0, opposite.y.0),
    ];
    let mut sketch = create_polyline(seed, prefix, &points, true)?;
    for i in 0..4 {
        let line = EntityId::from(format!("{prefix}:line{i}"));
        let relation = if i % 2 == 0 {
            crate::Relation::Horizontal(line)
        } else {
            crate::Relation::Vertical(line)
        };
        crate::construction::relation(&mut sketch, format!("{prefix}:axis{i}"), relation)?;
    }
    Ok(sketch)
}

/// Creates the circular arc through three picks, retaining analytic endpoints.
///
/// # Errors
/// Rejects collinear/coincident picks or colliding IDs.
pub(crate) fn create_arc_three_points(
    seed: &Sketch,
    prefix: &str,
    points: [Position; 3],
) -> Result<Sketch, SketchError> {
    let mut s = create_circle_three_points(seed, prefix, points)?;
    let curve = EntityId::from(format!("{prefix}:curve"));
    let Some(Entity::Circle { center, radius, .. }) = s.entities.get(&curve).cloned() else {
        return Err(invalid("missing constructed circle"));
    };
    let c = s.point(&center)?;
    let angles = points.map(|p| (p.y.0 - c.y.0).atan2(p.x.0 - c.x.0));
    let reverse = (angles[1] - angles[0]).rem_euclid(std::f64::consts::TAU)
        > (angles[2] - angles[0]).rem_euclid(std::f64::consts::TAU);
    let start = EntityId::from(format!("{prefix}:start"));
    let end = EntityId::from(format!("{prefix}:end"));
    insert(
        &mut s,
        start.clone(),
        Entity::Point {
            position: points[if reverse { 2 } else { 0 }],
            fixed: false,
        },
    )?;
    insert(
        &mut s,
        end.clone(),
        Entity::Point {
            position: points[if reverse { 0 } else { 2 }],
            fixed: false,
        },
    )?;
    s.entities.insert(
        curve,
        Entity::Arc {
            center,
            start,
            end,
            radius,
            start_angle: crate::Angle(angles[if reverse { 2 } else { 0 }]),
            end_angle: crate::Angle(angles[if reverse { 0 } else { 2 }]),
            fixed_shape: false,
        },
    );
    Ok(s)
}
