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
            return self.empty_sketch_draw_list(&render);
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
        let selected = self.sketch_highlighted(&snapshot);
        let selected_outline = filtered_outline(&snapshot, solution, &selected);
        list.extend(solved_draw_list(
            &snapshot,
            solution,
            &selected,
            self.view(),
        ));
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
        list.extend(self.sketch_axes_draw_list());
        list.extend(self.sketch_lock_draw_list());
        list.extend(self.sketch_capture_draw_list());
        list.extend(self.sketch_dimension_draw_list(&snapshot));
        if let Some(stroke) = &self.sketch.lasso {
            list.extend(curvyo_render_core::build_marquee_overlay(
                self.view(),
                &curvyo_render_core::MarqueeOverlay::Lasso(stroke.clone()),
                1.0,
            ));
        }
        if let Some((from, to)) = self.sketch.marquee {
            let contain = curvyo_ui_core::MarqueeMode::for_drag(from, to, self.held.alt)
                == curvyo_ui_core::MarqueeMode::Contain;
            list.extend(curvyo_render_core::build_marquee_overlay(
                self.view(),
                &curvyo_render_core::MarqueeOverlay::Box { from, to, contain },
                1.0,
            ));
        }
        list
    }

    /// Selected, preselected and hovered-annotation geometry, drawn in the accent.
    fn sketch_highlighted(&self, snapshot: &SketchSnapshot) -> BTreeSet<EntityId> {
        let mut selected = self.sketch.entities.clone();
        selected.extend(self.sketch.preselection.iter().cloned());
        for id in &self.sketch.constraints {
            if let Some(c) = snapshot.graph.constraints.get(id) {
                selected.extend(c.relation.entity_ids().into_iter().cloned());
            }
        }
        selected.extend(self.sketch_hover_participants(snapshot));
        selected
    }

    /// Every dimension's lines and arrowheads, bottom to top as the records are ordered (AC31).
    fn sketch_dimension_draw_list(&self, snapshot: &SketchSnapshot) -> DrawList {
        let mut list = DrawList::default();
        for record in self.sketch_dimension_records(snapshot) {
            let arrows = record
                .marks
                .arrows
                .iter()
                .map(|a| (a.tip, a.back))
                .collect::<Vec<_>>();
            list.extend(curvyo_render_core::build_sketch_dimension(
                &curvyo_render_core::SketchDimensionMarks {
                    lines: &record.marks.lines,
                    arrows: &arrows,
                    reference: record.reference,
                    hovered: record.hovered,
                    selected: record.selected,
                },
                self.view(),
            ));
        }
        list
    }

    fn empty_sketch_draw_list(&self, render: &[ObjectSnapshot]) -> DrawList {
        let mut list = curvyo_render_core::build_artwork_with_guides(
            render,
            &self.document_read().guides,
            self.view(),
        );
        if let Some(placement) = self.sketch_placement() {
            if let Some(preview) = self.sketch_construction_preview()
                && let Ok(paths) =
                    curvyo_document_core::sketch_preview_outline(&preview.candidate, placement)
            {
                list.extend(curvyo_render_core::build_sketch_preview(
                    &paths,
                    self.view(),
                ));
            }
            let points = std::iter::once(curvyo_sketch_core::Position::mm(0.0, 0.0))
                .chain(self.sketch.picks.iter().copied())
                .chain(self.sketch.hover.iter().copied())
                .enumerate()
                .map(|(i, p)| SketchPointGlyph {
                    position: placement.apply_point(Point::new(p.x.0, p.y.0)),
                    selected: i > 0,
                    origin: i == 0,
                })
                .collect::<Vec<_>>();
            list.extend(curvyo_render_core::build_sketch_overlay(
                None,
                None,
                &points,
                self.view(),
            ));
        }
        list.extend(self.sketch_axes_draw_list());
        list.extend(self.sketch_lock_draw_list());
        list
    }

    fn sketch_axes_draw_list(&self) -> DrawList {
        let Some(placement) = self.sketch_placement() else {
            return DrawList::default();
        };
        let page = self.active_page();
        curvyo_render_core::build_sketch_axes(
            placement,
            curvyo_document_core::Vec2::new(
                page.size.width.as_mm() / 2.0,
                page.size.height.as_mm() / 2.0,
            ),
            self.sketch.hover.map(|p| Point::new(p.x.0, p.y.0)),
            self.view(),
        )
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
/// Fully constrained geometry in `--sketch-solved`, under the selection (AC32 proposal).
fn solved_draw_list(
    snapshot: &SketchSnapshot,
    solution: &Sketch,
    selected: &BTreeSet<EntityId>,
    view: curvyo_document_core::ViewTransform,
) -> DrawList {
    let Some(solved) = &snapshot.fully_constrained else {
        return DrawList::default();
    };
    let solved = solved
        .iter()
        .filter(|id| !selected.contains(*id) && !solution.construction.contains(*id))
        .cloned()
        .collect::<BTreeSet<_>>();
    filtered_outline(snapshot, solution, &solved).map_or_else(DrawList::default, |path| {
        curvyo_render_core::build_sketch_solved(&path, view)
    })
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
