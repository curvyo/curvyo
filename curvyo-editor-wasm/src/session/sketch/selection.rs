//! Analytic selection gestures retain the existing marquee and Alt-stroke conventions.
use super::Session;
use curvyo_document_core::Point;
use curvyo_sketch_core::Length;
impl Session {
    pub(super) fn finish_sketch_marquee(&mut self, start: Point, end: Point) {
        if start.vector_to(end).length() * self.view().scale() <= 3.0 {
            return;
        }
        let Ok(snapshot) = self.sketch_snapshot_for_edit() else {
            return;
        };
        let from = curvyo_sketch_core::Position::mm(start.x, start.y);
        let to = curvyo_sketch_core::Position::mm(end.x, end.y);
        let contain = curvyo_ui_core::MarqueeMode::for_drag(start, end, self.held.alt)
            == curvyo_ui_core::MarqueeMode::Contain;
        let found = curvyo_ui_core::sketch_marquee_placed(
            &snapshot.graph,
            &snapshot.solution,
            from,
            to,
            snapshot.placement,
            contain,
        );
        match curvyo_ui_core::SelectionCombine::from_modifiers(self.held.shift, self.held.ctrl) {
            curvyo_ui_core::SelectionCombine::Replace => self.sketch.entities = found,
            curvyo_ui_core::SelectionCombine::Add => self.sketch.entities.extend(found),
            curvyo_ui_core::SelectionCombine::Remove => {
                self.sketch.entities.retain(|id| !found.contains(id));
            }
        }
    }

    pub(super) fn finish_sketch_lasso(&mut self, stroke: &[Point], end: Point) {
        let Some(start) = stroke.first() else {
            return;
        };
        let Ok(snapshot) = self.sketch_snapshot_for_edit() else {
            return;
        };
        if stroke
            .iter()
            .all(|p| start.vector_to(*p).length() * self.view().scale() <= 3.0)
        {
            let Some(local) = self.sketch_local(*start) else {
                return;
            };
            // Alt-click cycles through everything in reach: points, edges, then the axes
            // (AC34, AC50, AC52).
            let ids = curvyo_ui_core::sketch_pick_order(
                &snapshot.graph,
                &snapshot.solution,
                local,
                Length(1.0 / self.view().scale()),
            )
            .into_iter()
            .map(|h| h.entity)
            .chain(self.sketch_axes_at(local))
            .collect::<Vec<_>>();
            let index = match &self.sketch.hit_cycle {
                Some((point, old, index))
                    if point.vector_to(*start).length() * self.view().scale() <= 1.0
                        && old == &ids
                        && !ids.is_empty() =>
                {
                    (index + 1) % ids.len()
                }
                _ => 0,
            };
            if let Some(id) = ids.get(index) {
                self.sketch.entities = [id.clone()].into();
            }
            self.sketch.hit_cycle = Some((*start, ids, index));
            return;
        }
        self.sketch.hit_cycle = None;
        let local = stroke
            .iter()
            .chain(std::iter::once(&end))
            .filter_map(|p| self.sketch_local(*p))
            .collect::<Vec<_>>();
        let found = curvyo_ui_core::sketch_lasso(
            snapshot.id,
            &snapshot.graph,
            &snapshot.solution,
            &local,
            Length(curvyo_ui_core::SKETCH_EDGE_PICK_PX / self.view().scale()),
        );
        match curvyo_ui_core::SelectionCombine::from_modifiers(self.held.shift, self.held.ctrl) {
            curvyo_ui_core::SelectionCombine::Replace => self.sketch.entities = found,
            curvyo_ui_core::SelectionCombine::Add => self.sketch.entities.extend(found),
            curvyo_ui_core::SelectionCombine::Remove => {
                self.sketch.entities.retain(|id| !found.contains(id));
            }
        }
    }
}
