//! Sketch drawing substitutes validated live geometry while keeping edit cues out of output.
use super::{Session, view::internal_focus};
use curvyo_document_core::{ObjectSnapshot, PathSnapshot, Point, SketchSnapshot, sketch_outline};
use curvyo_render_core::{DrawList, SketchPointGlyph};
use curvyo_sketch_core::{Entity, EntityId, Sketch};
use std::collections::{BTreeSet, HashMap};

impl Session {
    pub(in crate::session) fn sketch_draw_list(&self) -> DrawList {
        let render = self.render_objects();
        let Some(snapshot) = self.sketch.active.and_then(|id| self.document.sketch(id)) else {
            return curvyo_render_core::build_artwork_with_guides(
                &render,
                &self.document_read().guides,
                self.view(),
            );
        };
        let solution = self
            .sketch_preview()
            .or_else(|| self.sketch_construction_preview().map(|p| &p.candidate))
            .unwrap_or(&snapshot.solution);
        let previewing =
            self.sketch_preview().is_some() || self.sketch_construction_preview().is_some();
        let mut artwork = render.to_vec();
        if previewing {
            self.sketch_replace_preview(&snapshot, solution, &mut artwork);
        }
        let mut list = curvyo_render_core::build_artwork_with_guides(
            &artwork,
            &self.document_read().guides,
            self.view(),
        );
        let construction = filtered_outline(&snapshot, solution, &solution.construction);
        let mut selected = self.sketch.entities.clone();
        for id in &self.sketch.constraints {
            if let Some(c) = snapshot.graph.constraints.get(id) {
                selected.extend(c.relation.entity_ids().into_iter().cloned());
            }
        }
        let selected_outline = filtered_outline(&snapshot, solution, &selected);
        let mut points = solution
            .entities
            .iter()
            .filter_map(|(id, entity)| {
                if internal_focus(solution, id)
                    || (!previewing && !snapshot.graph.entities.contains_key(id))
                {
                    return None;
                }
                let Entity::Point { position, .. } = entity else {
                    return None;
                };
                Some(SketchPointGlyph {
                    position: snapshot
                        .placement
                        .apply_point(Point::new(position.x.0, position.y.0)),
                    selected: selected.contains(id),
                    origin: *id == solution.origin,
                })
            })
            .collect::<Vec<_>>();
        for id in &self.sketch.entities {
            for (_, position) in
                curvyo_sketch_core::handle_positions(solution, id).unwrap_or_default()
            {
                points.push(SketchPointGlyph {
                    position: snapshot
                        .placement
                        .apply_point(Point::new(position.x.0, position.y.0)),
                    selected: true,
                    origin: false,
                });
            }
        }
        for pick in self.sketch.picks.iter().chain(self.sketch.hover.iter()) {
            points.push(SketchPointGlyph {
                position: snapshot
                    .placement
                    .apply_point(Point::new(pick.x.0, pick.y.0)),
                selected: true,
                origin: false,
            });
        }
        list.extend(curvyo_render_core::build_sketch_overlay(
            construction.as_ref(),
            selected_outline.as_ref(),
            &points,
            self.view(),
        ));
        list.extend(self.snap_marker_list());
        list
    }

    fn sketch_replace_preview(
        &self,
        snapshot: &SketchSnapshot,
        solution: &Sketch,
        artwork: &mut [ObjectSnapshot],
    ) {
        let Ok(path) = sketch_outline(snapshot.id, solution, snapshot.placement, &snapshot.style)
        else {
            return;
        };
        let mut changed = snapshot.clone();
        changed.solution = solution.clone();
        changed.resolved = path;
        let changed = ObjectSnapshot::Sketch(Box::new(changed));
        let all = self.document_read();
        let clones = curvyo_document_core::clone_previews(
            &all.leaves,
            std::slice::from_ref(&changed),
            usize::MAX,
            false,
        )
        .unwrap_or_default();
        let clones = clones
            .into_iter()
            .map(|p| (p.id, p))
            .collect::<HashMap<_, _>>();
        for object in artwork {
            if object.id() == snapshot.id {
                *object = changed.clone();
            } else if let Some(path) = clones.get(&object.id()) {
                *object = ObjectSnapshot::Path(path.clone());
            }
        }
    }
}
fn filtered_outline(
    snapshot: &SketchSnapshot,
    solution: &Sketch,
    ids: &BTreeSet<EntityId>,
) -> Option<PathSnapshot> {
    if ids.is_empty() {
        return None;
    }
    let mut graph = solution.clone();
    graph
        .entities
        .retain(|id, entity| ids.contains(id) || matches!(entity, Entity::Point { .. }));
    graph.construction.clear();
    graph.constraints.clear();
    graph.parameters.0.clear();
    graph.contact_seeds.clear();
    sketch_outline(snapshot.id, &graph, snapshot.placement, &snapshot.style).ok()
}
