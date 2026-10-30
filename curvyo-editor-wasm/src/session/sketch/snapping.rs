//! Analytic drag features extend the existing snapping index with stationary sketch points.
use super::Session;
use curvyo_document_core::Point;
use curvyo_ui_core::{SnapProbe, SnapTarget, TargetKind};

impl Session {
    pub(super) fn sketch_drag_probe(&self, pointer: Point) -> SnapProbe {
        let Some(drag) = &self.sketch.drag else {
            return SnapProbe::default();
        };
        let delta = drag.press.vector_to(pointer);
        let features = if let Some((id, handle)) = &drag.handle {
            curvyo_sketch_core::handle_positions(&drag.base.solution, id)
                .ok()
                .and_then(|positions| positions.get(handle).copied())
                .into_iter()
                .collect::<Vec<_>>()
        } else {
            drag.points
                .iter()
                .filter_map(|id| drag.base.solution.point(id).ok())
                .collect()
        };
        SnapProbe::points(
            &features
                .into_iter()
                .map(|position| {
                    drag.base
                        .placement
                        .apply_point(Point::new(position.x.0, position.y.0))
                        .translated(delta)
                })
                .collect::<Vec<_>>(),
        )
    }

    pub(super) fn add_stationary_sketch_targets(&mut self) {
        let Some(drag) = &self.sketch.drag else {
            return;
        };
        let graph = &drag.base.graph;
        let mut fixed = std::collections::BTreeSet::new();
        for (id, entity) in &graph.entities {
            if matches!(
                entity,
                curvyo_sketch_core::Entity::Point { fixed: true, .. }
            ) {
                fixed.insert(id.clone());
            }
        }
        for constraint in graph.constraints.values() {
            if let curvyo_sketch_core::Relation::Fixed(id) = &constraint.relation {
                fixed.insert(id.clone());
                if let Some(entity) = graph.entities.get(id) {
                    fixed.extend(entity.point_ids().into_iter().cloned());
                }
            }
        }
        let mut moving = drag
            .points
            .difference(&fixed)
            .cloned()
            .collect::<std::collections::BTreeSet<_>>();
        // A hard relation can move its other geometry: never use that dependent
        // component as a stationary target for its own drag.
        loop {
            let previous = moving.len();
            for (id, entity) in &graph.entities {
                let refs = entity.point_ids();
                if moving.contains(id) || refs.iter().any(|id| moving.contains(*id)) {
                    moving.insert(id.clone());
                }
            }
            for constraint in graph.constraints.values().filter(|c| !c.reference) {
                let ids = constraint.relation.entity_ids();
                if ids.iter().any(|id| moving.contains(*id)) {
                    for id in ids {
                        if fixed.contains(id) {
                            continue;
                        }
                        moving.insert(id.clone());
                        if let Some(entity) = graph.entities.get(id) {
                            moving.extend(
                                entity
                                    .point_ids()
                                    .into_iter()
                                    .filter(|id| !fixed.contains(*id))
                                    .cloned(),
                            );
                        }
                    }
                }
            }
            if previous == moving.len() {
                break;
            }
        }
        let targets = curvyo_ui_core::sketch_snap_points(graph, &drag.base.solution)
            .into_iter()
            .filter_map(|(id, position)| {
                if moving.contains(&id) {
                    return None;
                }
                let point = drag
                    .base
                    .placement
                    .apply_point(Point::new(position.x.0, position.y.0));
                Some(SnapTarget::point(point, (point, point), TargetKind::Node))
            })
            .collect::<Vec<_>>();
        self.snapping.extend_targets(targets);
    }
}
