//! Analytic sketch selection returns stable entity identities instead of output anchors.
use curvyo_sketch_core::{Entity, EntityId, Length, Position, Sketch};
use kurbo::{ParamCurveNearest, Shape};

/// A selectable analytic body or its defining point.
#[derive(Debug, Clone, PartialEq)]
pub struct SketchHit {
    /// Current authoritative identity.
    pub entity: EntityId,
    /// True for a point handle, false for the body of a curve.
    pub point: bool,
}

/// How far from an edge, in screen pixels, a press still picks it.
pub const SKETCH_EDGE_PICK_PX: f64 = 6.0;

/// How far from a point, in screen pixels, a press picks it before any edge (0066 AC50).
pub const SKETCH_POINT_PICK_PX: f64 = 10.0;

/// Picks the winning geometry: a point within reach before any edge, else the nearest edge.
/// `px` is one screen pixel in document mm. Current intent filters stale last-valid topology
/// out of the editable hit set.
#[must_use]
pub fn sketch_hit(
    intent: &Sketch,
    solution: &Sketch,
    pointer: Position,
    px: Length,
) -> Option<SketchHit> {
    sketch_hits(intent, solution, pointer, px)
        .into_iter()
        .next()
}

/// The winning hits: the nearest points within [`SKETCH_POINT_PICK_PX`], else the nearest edges
/// within [`SKETCH_EDGE_PICK_PX`] (`px` is one screen pixel in document mm); several only on an
/// equal-distance tie.
#[must_use]
pub fn sketch_hits(
    intent: &Sketch,
    solution: &Sketch,
    pointer: Position,
    px: Length,
) -> Vec<SketchHit> {
    let order = ranked(intent, solution, pointer, px);
    let Some(&(nearest, point, _)) = order.first() else {
        return Vec::new();
    };
    order
        .into_iter()
        .filter(|candidate| candidate.1 == point && candidate.0 <= nearest + 1e-9)
        .map(|(_, point, entity)| SketchHit { entity, point })
        .collect()
}

/// Every hit in pick order: points within reach nearest first, then edges within reach nearest
/// first (`px` is one screen pixel in document mm), so Alt cycling reaches the edge under a winning point (AC34, AC50).
#[must_use]
pub fn sketch_pick_order(
    intent: &Sketch,
    solution: &Sketch,
    pointer: Position,
    px: Length,
) -> Vec<SketchHit> {
    ranked(intent, solution, pointer, px)
        .into_iter()
        .map(|(_, point, entity)| SketchHit { entity, point })
        .collect()
}

/// Candidates as (distance, is point, id): points within reach first, then edges, each nearest
/// first, ties by id.
fn ranked(
    intent: &Sketch,
    solution: &Sketch,
    pointer: Position,
    px: Length,
) -> Vec<(f64, bool, EntityId)> {
    let point_reach = Length(px.0 * SKETCH_POINT_PICK_PX);
    let tolerance = Length(px.0 * SKETCH_EDGE_PICK_PX);
    let internal_focus = solution
        .entities
        .values()
        .filter_map(|e| match e {
            Entity::Ellipse { focus, .. } => Some(focus),
            _ => None,
        })
        .collect::<std::collections::BTreeSet<_>>();
    let mut candidates = Vec::new();
    for (id, entity) in &solution.entities {
        if !intent.entities.contains_key(id) {
            continue;
        }
        if let Entity::Point { position, .. } = entity {
            let distance = position.distance(pointer).0;
            if !internal_focus.contains(id) && distance <= point_reach.0 {
                candidates.push((distance, true, id.clone()));
            }
        } else if let Some(distance) = distance_to_entity(solution, entity, pointer, tolerance)
            && distance <= tolerance.0
        {
            candidates.push((distance, false, id.clone()));
        }
    }
    candidates.sort_by(|a, b| {
        b.1.cmp(&a.1)
            .then_with(|| a.0.total_cmp(&b.0))
            .then_with(|| a.2.cmp(&b.2))
    });
    candidates
}

/// Picks selected curve axes independently of internal focus-point records.
#[must_use]
pub fn sketch_handle_hit(
    intent: &Sketch,
    solution: &Sketch,
    selected: &std::collections::BTreeSet<EntityId>,
    pointer: Position,
    tolerance: Length,
) -> Option<(EntityId, curvyo_sketch_core::SketchHandle)> {
    selected
        .iter()
        .filter(|id| intent.entities.contains_key(*id))
        .flat_map(|id| {
            curvyo_sketch_core::handle_positions(solution, id)
                .unwrap_or_default()
                .into_iter()
                .map(move |(handle, position)| (position.distance(pointer).0, id.clone(), handle))
        })
        .filter(|(distance, _, _)| *distance <= tolerance.0)
        .min_by(|a, b| {
            a.0.total_cmp(&b.0)
                .then_with(|| a.1.cmp(&b.1))
                .then_with(|| a.2.cmp(&b.2))
        })
        .map(|(_, id, handle)| (id, handle))
}

fn distance_to_entity(
    sketch: &Sketch,
    entity: &Entity,
    pointer: Position,
    tolerance: Length,
) -> Option<f64> {
    let point = kurbo::Point::new(pointer.x.0, pointer.y.0);
    let center = |id| {
        sketch
            .point(id)
            .ok()
            .map(|pointer| kurbo::Point::new(pointer.x.0, pointer.y.0))
    };
    let path = match entity {
        Entity::Point { .. } => return None,
        Entity::Line { start, end } => {
            return Some(
                kurbo::Line::new(center(start)?, center(end)?)
                    .nearest(point, 1e-9)
                    .distance_sq
                    .sqrt(),
            );
        }
        Entity::Circle {
            center: centre,
            radius,
            ..
        } => return Some((center(centre)?.distance(point) - radius.0).abs()),
        Entity::Arc {
            center: centre,
            radius,
            start_angle,
            end_angle,
            ..
        } => {
            let sweep = (end_angle.0 - start_angle.0).rem_euclid(std::f64::consts::TAU);
            kurbo::Arc::new(
                center(centre)?,
                kurbo::Vec2::new(radius.0, radius.0),
                start_angle.0,
                sweep,
                0.0,
            )
            .to_path((tolerance.0 * 0.01).max(1e-9))
        }
        Entity::Ellipse {
            center: centre,
            focus,
            minor_radius,
            ..
        } => {
            let centre = center(centre)?;
            let focus_point = center(focus)?;
            let axis = focus_point - centre;
            kurbo::Ellipse::new(
                centre,
                (minor_radius.0.hypot(axis.hypot()), minor_radius.0),
                axis.y.atan2(axis.x),
            )
            .to_path((tolerance.0 * 0.01).max(1e-9))
        }
    };
    path.segments()
        .map(|segment| segment.nearest(point, 1e-9).distance_sq.sqrt())
        .min_by(f64::total_cmp)
}

/// Current analytic points and visible axis handles usable as atomic snap targets.
#[must_use]
pub fn sketch_snap_points(intent: &Sketch, solution: &Sketch) -> Vec<(EntityId, Position)> {
    let mut points = Vec::new();
    for (id, entity) in &intent.entities {
        let focus = intent
            .entities
            .values()
            .any(|e| matches!(e, Entity::Ellipse {focus,..} if focus==id));
        if matches!(entity, Entity::Point { .. }) && !focus {
            if let Ok(point) = solution.point(id) {
                points.push((id.clone(), point));
            }
        } else if let Ok(handles) = curvyo_sketch_core::handle_positions(solution, id) {
            points.extend(handles.into_values().map(|point| (id.clone(), point)));
        }
    }
    points
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line_and_point() -> Sketch {
        let graph = curvyo_sketch_core::create_polyline(
            &Sketch::new("origin".into()),
            "l",
            &[Position::mm(100.0, 0.0), Position::mm(200.0, 0.0)],
            false,
        )
        .unwrap();
        let mut graph = graph;
        graph.entities.insert(
            "q".into(),
            Entity::Point {
                position: Position::mm(108.0, 4.0),
                fixed: false,
            },
        );
        graph
    }

    fn first(graph: &Sketch, x: f64, y: f64) -> SketchHit {
        sketch_hit(graph, graph, Position::mm(x, y), Length(1.0)).unwrap()
    }

    /// 0066 AC50: a point within 10 screen px wins over a nearer edge; the edge only without
    /// one. Here 1 mm is 1 screen px.
    #[test]
    fn a_point_within_ten_pixels_wins_over_a_nearer_edge() {
        let graph = line_and_point();
        let near = (36.0_f64 - 1.0).sqrt();
        let hit = first(&graph, 200.0 - near, 1.0);
        assert_eq!(hit.entity.0, "l:p1");
        assert!(hit.point);
        let far = (196.0_f64 - 1.0).sqrt();
        assert_eq!(first(&graph, 200.0 - far, 1.0).entity.0, "l:line0");
        // Two points within 10 px: the nearer one.
        assert_eq!(first(&graph, 104.5, 1.0).entity.0, "l:p0");
        assert_eq!(first(&graph, 105.5, 2.0).entity.0, "q");
    }

    /// 0066 AC50, AC34: the full pick order still reaches the edge under a winning point.
    #[test]
    fn the_pick_order_lists_the_edge_after_the_point() {
        let graph = line_and_point();
        let order = sketch_pick_order(&graph, &graph, Position::mm(195.0, 1.0), Length(1.0));
        let ids = order
            .iter()
            .map(|h| h.entity.0.as_str())
            .collect::<Vec<_>>();
        assert_eq!(ids, ["l:p1", "l:line0"]);
    }
    #[test]
    fn ellipse_body_hit_retains_analytic_identity_and_stale_ids_are_excluded() {
        let graph = curvyo_sketch_core::create_ellipse(
            &Sketch::new("origin".into()),
            "e",
            Position::mm(20.0, 20.0),
            Position::mm(30.0, 20.0),
            Length(5.0),
        )
        .unwrap();
        let hit = sketch_hit(&graph, &graph, Position::mm(20.0, 25.0), Length(0.1)).unwrap();
        assert_eq!(
            hit,
            SketchHit {
                entity: "e:curve".into(),
                point: false
            }
        );
        let mut current = graph.clone();
        current.entities.remove(&"e:curve".into());
        assert_eq!(
            sketch_hit(&current, &graph, Position::mm(20.0, 25.0), Length(0.1)),
            None
        );
    }
}
