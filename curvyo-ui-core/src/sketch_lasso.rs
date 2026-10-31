//! Sketch lasso proximity reuses ordinary outline selection semantics.
use curvyo_document_core::{
    Affine, NodeId, ObjectSnapshot, Point, Style, Tolerance, sketch_outline,
};
use curvyo_sketch_core::{Entity, EntityId, Length, Position, Sketch};
use kurbo::ParamCurveNearest;
use std::collections::BTreeSet;

/// Finds current analytic geometry touched by an open freehand selection stroke.
#[must_use]
pub fn sketch_lasso(
    owner: NodeId,
    intent: &Sketch,
    solution: &Sketch,
    stroke: &[Position],
    tolerance: Length,
) -> BTreeSet<EntityId> {
    if stroke.is_empty() {
        return BTreeSet::new();
    }
    let points = stroke
        .iter()
        .map(|p| Point::new(p.x.0, p.y.0))
        .collect::<Vec<_>>();
    let focus = solution
        .entities
        .values()
        .filter_map(|entity| {
            if let Entity::Ellipse { focus, .. } = entity {
                Some(focus)
            } else {
                None
            }
        })
        .collect::<BTreeSet<_>>();
    solution
        .entities
        .iter()
        .filter_map(|(id, entity)| {
            if !intent.entities.contains_key(id) || focus.contains(id) {
                return None;
            }
            if let Entity::Point { position, .. } = entity {
                let target = kurbo::Point::new(position.x.0, position.y.0);
                let near = stroke.windows(2).any(|pair| {
                    kurbo::Line::new((pair[0].x.0, pair[0].y.0), (pair[1].x.0, pair[1].y.0))
                        .nearest(target, 1e-9)
                        .distance_sq
                        <= tolerance.0 * tolerance.0
                });
                return near.then(|| id.clone());
            }
            let mut graph = solution.clone();
            graph
                .entities
                .retain(|key, e| key == id || matches!(e, Entity::Point { .. }));
            graph.constraints.clear();
            graph.parameters.0.clear();
            graph.construction.clear();
            graph.contact_seeds.clear();
            // The original identity labels only this transient one-entity query outline.
            let path = sketch_outline(owner, &graph, Affine::IDENTITY, &Style::default()).ok()?;
            let hits = crate::hit_test_objects_along(
                &[ObjectSnapshot::Path(path)],
                &points,
                Tolerance::from_mm(tolerance.0),
            );
            (!hits.is_empty()).then(|| id.clone())
        })
        .collect()
}
