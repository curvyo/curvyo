//! Constrained pointer previews commit only explicitly dragged geometry records.
use super::Session;
use curvyo_document_core::{
    DocumentVersion, Point, SketchDelta, SketchEntityRecord, SketchSnapshot,
};
use curvyo_sketch_core::{Entity, EntityId, Length, Position, Sketch, SolveMode, solve_seeded};
use std::collections::{BTreeMap, BTreeSet};

pub(in crate::session) struct SketchDrag {
    pub base: SketchSnapshot,
    pub revision: DocumentVersion,
    pub press: Point,
    pub points: BTreeSet<EntityId>,
    pub handle: Option<(EntityId, curvyo_sketch_core::SketchHandle)>,
    pub candidate: Option<Sketch>,
}

impl Session {
    pub(super) fn sketch_select_press(&mut self, point: Point) {
        let Ok(base) = self.sketch_snapshot_for_edit() else {
            return;
        };
        let Some(local) = self.sketch_local(point) else {
            return;
        };
        if self.held.alt && self.sketch.tool == super::SketchTool::Select {
            self.sketch.lasso = Some(vec![point]);
            self.button_down = true;
            return;
        }
        let handle = curvyo_ui_core::sketch_handle_hit(
            &base.graph,
            &base.solution,
            &self.sketch.entities,
            local,
            Length(6.0 / self.view().scale()),
        );
        let hit = handle
            .as_ref()
            .map(|(id, _)| curvyo_ui_core::SketchHit {
                entity: id.clone(),
                point: false,
            })
            .or_else(|| {
                let hits = curvyo_ui_core::sketch_hits(
                    &base.graph,
                    &base.solution,
                    local,
                    Length(6.0 / self.view().scale()),
                );
                let ids = hits
                    .iter()
                    .map(|hit| hit.entity.clone())
                    .collect::<Vec<_>>();
                let index = match &self.sketch.hit_cycle {
                    Some((previous, old, index))
                        if previous.vector_to(point).length() * self.view().scale() <= 1.0
                            && *old == ids
                            && !ids.is_empty() =>
                    {
                        (index + 1) % ids.len()
                    }
                    _ => 0,
                };
                self.sketch.hit_cycle = Some((point, ids, index));
                hits.into_iter().nth(index)
            });
        let Some(hit) = hit else {
            if self.sketch_place_dimension(point) {
                return;
            }
            self.sketch.marquee = Some((point, point));
            self.button_down = true;
            if !self.held.shift && !self.held.ctrl {
                self.sketch.entities.clear();
            }
            return;
        };
        if self.sketch.tool == super::SketchTool::Constraints {
            self.sketch.dimension_position = None;
            if !self.sketch.entities.insert(hit.entity.clone()) {
                self.sketch.entities.remove(&hit.entity);
            }
            return;
        }
        if self.held.shift {
            if !self.sketch.entities.insert(hit.entity.clone()) {
                self.sketch.entities.remove(&hit.entity);
                return;
            }
        } else if !self.sketch.entities.contains(&hit.entity) {
            self.sketch.entities.clear();
            self.sketch.entities.insert(hit.entity.clone());
        }
        if base.stale {
            return;
        }
        let mut points = BTreeSet::new();
        for id in &self.sketch.entities {
            if let Some(entity) = base.graph.entities.get(id) {
                if matches!(entity, Entity::Point { .. }) {
                    points.insert(id.clone());
                }
                points.extend(entity.point_ids().into_iter().cloned());
            }
        }
        self.sketch.drag = Some(SketchDrag {
            base,
            revision: self.document.version(),
            press: point,
            points,
            handle,
            candidate: None,
        });
        self.button_down = true;
    }

    pub(super) fn sketch_drag_hover(&mut self, point: Point) {
        if self.sketch.lasso.is_some() {
            let scale = self.view().scale();
            if let Some(stroke) = &mut self.sketch.lasso
                && stroke
                    .last()
                    .is_none_or(|last| last.vector_to(point).length() * scale >= 1.0)
            {
                stroke.push(point);
            }
            return;
        }
        if let Some((_, current)) = &mut self.sketch.marquee {
            *current = point;
            return;
        }
        if self.sketch.drag.is_none() {
            if self.sketch.constraint_tool.as_deref() == Some("dimension") {
                self.sketch.hover = self.sketch_local(point);
            }
            self.sketch.preselection = self
                .sketch_snapshot_for_edit()
                .ok()
                .and_then(|snapshot| {
                    self.sketch_local(point).and_then(|local| {
                        curvyo_ui_core::sketch_hit(
                            &snapshot.graph,
                            &snapshot.solution,
                            local,
                            Length(6.0 / self.view().scale()),
                        )
                    })
                })
                .map(|hit| hit.entity);
            return;
        }
        let Some(drag) = self.sketch.drag.as_ref() else {
            return;
        };
        if self.document.version() != drag.revision {
            self.sketch.drag = None;
            return;
        }
        if drag.press.vector_to(point).length() * self.view().scale() <= 3.0
            && drag.candidate.is_none()
        {
            return;
        }
        let Some(inverse) = drag.base.placement.inverse() else {
            return;
        };
        let from = inverse.apply_point(drag.press);
        let to = inverse.apply_point(point);
        let goals = drag
            .points
            .iter()
            .filter_map(|id| {
                let p = drag.base.solution.point(id).ok()?;
                Some((
                    id.clone(),
                    Position::mm(p.x.0 + to.x - from.x, p.y.0 + to.y - from.y),
                ))
            })
            .collect::<BTreeMap<_, _>>();
        let seed = drag.candidate.as_ref().unwrap_or(&drag.base.solution);
        let mode =
            drag.handle
                .as_ref()
                .map_or(SolveMode::DragPoints { goals }, |(entity, handle)| {
                    SolveMode::DragHandle {
                        entity: entity.clone(),
                        handle: *handle,
                        target: {
                            let handle =
                                curvyo_sketch_core::handle_positions(&drag.base.solution, entity)
                                    .ok()
                                    .and_then(|positions| positions.get(handle).copied())
                                    .unwrap_or(Position::mm(from.x, from.y));
                            Position::mm(handle.x.0 + to.x - from.x, handle.y.0 + to.y - from.y)
                        },
                    }
                });
        match solve_seeded(&drag.base.graph, seed, &mode) {
            Ok(result) => {
                if let Some(candidate) = result.candidate {
                    if let Some(drag) = &mut self.sketch.drag {
                        drag.candidate = Some(candidate);
                    }
                    self.sketch.error = None;
                } else {
                    self.sketch.error = result.message;
                }
            }
            Err(error) => self.sketch.error = Some(error.to_string()),
        }
    }

    pub(super) fn sketch_drag_release(&mut self, point: Point) {
        self.sketch_drag_hover(point);
        self.end_snapping_drag();
        if let Some(stroke) = self.sketch.lasso.take() {
            self.finish_sketch_lasso(&stroke, point);
            return;
        }
        if let Some((start, end)) = self.sketch.marquee.take() {
            self.finish_sketch_marquee(start, end);
            return;
        }
        let Some(drag) = self.sketch.drag.take() else {
            return;
        };
        let Some(candidate) = drag.candidate else {
            return;
        };
        if self.document.version() != drag.revision {
            return;
        }
        let mut delta = SketchDelta::default();
        let mut explicit = drag.points;
        if let Some((id, _)) = drag.handle {
            explicit.insert(id);
        }
        for id in explicit {
            if candidate.entities.get(&id) != drag.base.solution.entities.get(&id)
                && let Some(entity) = candidate.entities.get(&id)
            {
                delta.entities.insert(
                    id.clone(),
                    Some(SketchEntityRecord {
                        entity: entity.clone(),
                        construction: drag.base.graph.construction.contains(&id),
                    }),
                );
            }
        }
        self.sketch.error = self
            .document
            .apply_sketch_preview(drag.base.id, &delta, &candidate)
            .err()
            .map(|e| e.to_string());
    }

    /// Last independently valid constrained pose; no preview writes document history.
    #[must_use]
    pub fn sketch_preview(&self) -> Option<&Sketch> {
        let drag = self.sketch.drag.as_ref()?;
        (drag.revision == self.document.version())
            .then_some(drag.candidate.as_ref())
            .flatten()
    }
}
