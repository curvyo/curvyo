//! Analytic sketch geometry becomes connected output contours.
use crate::{Entity, EntityId, Length, Sketch, SketchError};
use kurbo::{Arc, BezPath, Ellipse, Point, Shape, Vec2};
use std::collections::BTreeMap;

/// Builds output geometry without construction lines, dimensions or constraints.
///
/// # Errors
/// Rejects malformed geometry or a nonpositive approximation tolerance.
pub fn outline(s: &Sketch, tolerance: Length) -> Result<BezPath, SketchError> {
    s.validate()?;
    if !tolerance.0.is_finite() || tolerance.0 <= 0.0 {
        return Err(SketchError::Invalid("invalid outline tolerance".into()));
    }
    let mut result = BezPath::new();
    let mut edges = BTreeMap::new();
    for (id, entity) in &s.entities {
        if s.construction.contains(id) {
            continue;
        }
        match entity {
            Entity::Line { start, end } | Entity::Arc { start, end, .. } => {
                edges.insert(id.clone(), (start.clone(), end.clone()));
            }
            Entity::Circle { center, radius, .. } => {
                let center = s.point(center)?;
                let path =
                    kurbo::Circle::new((center.x.0, center.y.0), radius.0).to_path(tolerance.0);
                result.extend(path);
            }
            Entity::Ellipse {
                center,
                focus,
                minor_radius,
                ..
            } => {
                let center = s.point(center)?;
                let focus = s.point(focus)?;
                let dx = focus.x.0 - center.x.0;
                let dy = focus.y.0 - center.y.0;
                let path = Ellipse::new(
                    (center.x.0, center.y.0),
                    (minor_radius.0.hypot(dx.hypot(dy)), minor_radius.0),
                    dy.atan2(dx),
                )
                .to_path(tolerance.0);
                result.extend(path);
                // Kurbo emits a full-turn arc without an explicit contour closure.
                result.close_path();
            }
            Entity::Point { .. } => {}
        }
    }
    append_chains(s, &edges, &mut result, tolerance)?;
    Ok(result)
}
fn append_chains(
    s: &Sketch,
    edges: &BTreeMap<EntityId, (EntityId, EntityId)>,
    out: &mut BezPath,
    tolerance: Length,
) -> Result<(), SketchError> {
    for contour in crate::topology::contours(s, edges) {
        if let Some((first, reverse)) = contour.edges.first() {
            let (start, end) = &edges[first];
            let p = s.point(if *reverse { end } else { start })?;
            out.move_to((p.x.0, p.y.0));
            for (id, reverse) in contour.edges {
                append_edge(s, &id, reverse, out, tolerance)?;
            }
            if contour.closed {
                out.close_path();
            }
        }
    }
    Ok(())
}
fn append_edge(
    s: &Sketch,
    id: &EntityId,
    reverse: bool,
    out: &mut BezPath,
    tolerance: Length,
) -> Result<(), SketchError> {
    match s.entities.get(id) {
        Some(Entity::Line { start, end }) => {
            let p = s.point(if reverse { start } else { end })?;
            out.line_to((p.x.0, p.y.0));
        }
        Some(Entity::Arc {
            center,
            radius,
            start_angle,
            end_angle,
            ..
        }) => {
            let c = s.point(center)?;
            let sweep = crate::measure::sweep(start_angle.0, end_angle.0);
            let arc = Arc::new(
                Point::new(c.x.0, c.y.0),
                Vec2::new(radius.0, radius.0),
                if reverse { end_angle.0 } else { start_angle.0 },
                if reverse { -sweep } else { sweep },
                0.0,
            );
            out.extend(arc.to_path(tolerance.0).elements().iter().skip(1).copied());
        }
        _ => return Err(SketchError::Invalid("missing contour edge".into())),
    }
    Ok(())
}
