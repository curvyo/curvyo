//! Centre-defined arcs retain their picked centre and start radius.
use crate::{Angle, Entity, Position, Sketch, SketchError};
pub(crate) fn build(
    base: &Sketch,
    prefix: &str,
    center: Position,
    start: Position,
    end: Position,
) -> Result<Sketch, SketchError> {
    let radius = center.distance(start);
    let distance = center.distance(end).0;
    if !radius.0.is_finite() || radius.0 <= 1e-9 || !distance.is_finite() || distance <= 1e-9 {
        return Err(SketchError::Invalid(
            "Arc directions and radius must be finite and nonzero".into(),
        ));
    }
    let end = Position::mm(
        center.x.0 + (end.x.0 - center.x.0) * radius.0 / distance,
        center.y.0 + (end.y.0 - center.y.0) * radius.0 / distance,
    );
    if start.distance(end).0 <= 1e-9 {
        return Err(SketchError::Invalid(
            "Choose a distinct arc end direction".into(),
        ));
    }
    let mut graph = base.clone();
    for (suffix, position) in [("center", center), ("start", start), ("end", end)] {
        crate::construction::insert(
            &mut graph,
            format!("{prefix}:{suffix}").into(),
            Entity::Point {
                position,
                fixed: false,
            },
        )?;
    }
    let a = (start.y.0 - center.y.0).atan2(start.x.0 - center.x.0);
    let b = (end.y.0 - center.y.0).atan2(end.x.0 - center.x.0);
    crate::construction::insert(
        &mut graph,
        format!("{prefix}:curve").into(),
        Entity::Arc {
            center: format!("{prefix}:center").into(),
            start: format!("{prefix}:start").into(),
            end: format!("{prefix}:end").into(),
            radius,
            start_angle: Angle(a),
            end_angle: Angle(a + (b - a).rem_euclid(std::f64::consts::TAU)),
            fixed_shape: false,
        },
    )?;
    Ok(graph)
}

pub(crate) fn from_endpoint(
    base: &Sketch,
    prefix: &str,
    start: &crate::EntityId,
    through: Position,
    end: Position,
) -> Result<Sketch, SketchError> {
    let position = base.point(start)?;
    let mut graph =
        crate::primitives::create_arc_three_points(base, prefix, [position, through, end])?;
    let curve: crate::EntityId = format!("{prefix}:curve").into();
    let Some(Entity::Arc {
        start: a, end: b, ..
    }) = graph.entities.get(&curve)
    else {
        return Err(SketchError::Invalid("Missing constructed arc".into()));
    };
    let (a, b) = (a.clone(), b.clone());
    let replaced = if graph.point(&a)?.distance(position).0 <= 1e-9 {
        a.clone()
    } else {
        b.clone()
    };
    if let Some(Entity::Arc {
        start: a, end: b, ..
    }) = graph.entities.get_mut(&curve)
    {
        if a == &replaced {
            *a = start.clone();
        } else {
            *b = start.clone();
        }
    }
    graph.entities.remove(&replaced);
    Ok(graph)
}
