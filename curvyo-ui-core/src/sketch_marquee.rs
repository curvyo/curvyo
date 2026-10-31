//! Analytic marquee queries retain current identities while testing actual curve geometry.
use curvyo_sketch_core::{Entity, EntityId, Length, Position, Sketch};
use kurbo::{ParamCurve, Shape};
use std::collections::BTreeSet;

/// Selects contained or crossed analytic entities, excluding internal ellipse focus handles.
#[must_use]
pub fn sketch_marquee(
    intent: &Sketch,
    solution: &Sketch,
    from: Position,
    to: Position,
    contain: bool,
) -> BTreeSet<EntityId> {
    sketch_marquee_placed(
        intent,
        solution,
        from,
        to,
        curvyo_document_core::Affine::IDENTITY,
        contain,
    )
}

/// Tests a global marquee against the original's placed analytic geometry.
#[must_use]
pub fn sketch_marquee_placed(
    intent: &Sketch,
    solution: &Sketch,
    from: Position,
    to: Position,
    placement: curvyo_document_core::Affine,
    contain: bool,
) -> BTreeSet<EntityId> {
    let rect = kurbo::Rect::from_points((from.x.0, from.y.0), (to.x.0, to.y.0));
    let focus = solution
        .entities
        .values()
        .filter_map(|e| {
            if let Entity::Ellipse { focus, .. } = e {
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
                let point = placement
                    .apply_point(curvyo_document_core::Point::new(position.x.0, position.y.0));
                return rect
                    .contains(kurbo::Point::new(point.x, point.y))
                    .then(|| id.clone());
            }
            let mut graph = solution.clone();
            graph
                .entities
                .retain(|key, e| key == id || matches!(e, Entity::Point { .. }));
            graph.constraints.clear();
            graph.parameters.0.clear();
            graph.construction.clear();
            graph.contact_seeds.clear();
            let mut path = curvyo_sketch_core::outline(&graph, Length(0.001)).ok()?;
            path.apply_affine(kurbo::Affine::new(placement.to_array()));
            let bounds = path.bounding_box();
            let fully = rect.contains(bounds.origin())
                && rect.contains(kurbo::Point::new(bounds.x1, bounds.y1));
            let edges = [
                kurbo::Line::new((rect.x0, rect.y0), (rect.x1, rect.y0)),
                kurbo::Line::new((rect.x1, rect.y0), (rect.x1, rect.y1)),
                kurbo::Line::new((rect.x1, rect.y1), (rect.x0, rect.y1)),
                kurbo::Line::new((rect.x0, rect.y1), (rect.x0, rect.y0)),
            ];
            let touches = fully
                || (!contain
                    && path.segments().any(|segment| {
                        rect.contains(segment.start())
                            || rect.contains(segment.end())
                            || edges
                                .iter()
                                .any(|edge| !segment.intersect_line(*edge).is_empty())
                    }));
            touches.then(|| id.clone())
        })
        .collect()
}
