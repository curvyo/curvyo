//! Inspection follows current analytic relationships without editing geometry.
use super::Session;
use curvyo_document_core::Point;
use curvyo_sketch_core::ParameterId;
use std::collections::BTreeSet;

impl Session {
    /// Selects entities whose validated rank analysis leaves movement available.
    pub fn sketch_show_free(&mut self) -> bool {
        let Ok(snapshot) = self.sketch_snapshot_for_edit() else {
            return false;
        };
        let Some(fixed) = &snapshot.fully_constrained else {
            return false;
        };
        self.sketch.entities = snapshot
            .graph
            .entities
            .keys()
            .filter(|id| !fixed.contains(*id) && !super::view::internal_focus(&snapshot.graph, id))
            .cloned()
            .collect();
        self.sketch_inspect_selection();
        true
    }

    /// Highlights relationships involving the selected current geometry.
    pub fn sketch_inspect_selection(&mut self) {
        let Ok(snapshot) = self.sketch_snapshot_for_edit() else {
            return;
        };
        self.sketch.constraints = snapshot
            .graph
            .constraints
            .iter()
            .filter(|(_, c)| {
                c.relation
                    .entity_ids()
                    .iter()
                    .any(|id| self.sketch.entities.contains(*id))
            })
            .map(|(id, _)| id.clone())
            .collect();
        self.sketch_reveal_selection();
    }

    /// Highlights dimension dependents of a parameter, including transitive formulas.
    pub fn sketch_inspect_parameter(&mut self, id: &ParameterId) -> bool {
        let Ok(snapshot) = self.sketch_snapshot_for_edit() else {
            return false;
        };
        if !snapshot.graph.parameters.0.contains_key(id) {
            return false;
        }
        let mut dependents = BTreeSet::from([id.clone()]);
        loop {
            let before = dependents.len();
            for (id, parameter) in &snapshot.graph.parameters.0 {
                let mut references = Vec::new();
                parameter.expression.references(&mut references);
                if references.iter().any(|id| dependents.contains(id)) {
                    dependents.insert(id.clone());
                }
            }
            if before == dependents.len() {
                break;
            }
        }
        self.sketch.constraints = snapshot
            .graph
            .constraints
            .iter()
            .filter(|(_, c)| {
                let mut references = Vec::new();
                if let Some(expression) = c.relation.expression() {
                    expression.references(&mut references);
                }
                references.iter().any(|id| dependents.contains(id))
            })
            .map(|(id, _)| id.clone())
            .collect();
        self.sketch.entities = self
            .sketch
            .constraints
            .iter()
            .filter_map(|id| snapshot.graph.constraints.get(id))
            .flat_map(|c| c.relation.entity_ids())
            .filter(|id| snapshot.graph.entities.contains_key(*id))
            .cloned()
            .collect();
        self.sketch.edit_constraint = None;
        self.sketch_reveal_selection();
        true
    }

    pub(super) fn sketch_reveal_selection(&mut self) {
        let Some(snapshot) = self.active_sketch().and_then(|id| self.document.sketch(id)) else {
            return;
        };
        let positions = self
            .sketch
            .entities
            .iter()
            .filter(|id| snapshot.graph.entities.contains_key(*id))
            .filter_map(|id| snapshot.solution.entities.get(id))
            .flat_map(|entity| match entity {
                curvyo_sketch_core::Entity::Point { position, .. } => vec![*position],
                entity => entity
                    .point_ids()
                    .into_iter()
                    .filter_map(|id| snapshot.solution.point(id).ok())
                    .collect(),
            })
            .collect::<Vec<_>>();
        if positions.is_empty() {
            return;
        }
        let (w, h) = self.viewport.canvas_size();
        let screens = positions
            .iter()
            .map(|p| {
                self.view()
                    .document_to_screen(snapshot.placement.apply_point(Point::new(p.x.0, p.y.0)))
            })
            .collect::<Vec<_>>();
        if screens
            .iter()
            .any(|(x, y)| *x >= 0.0 && *x <= w && *y >= 0.0 && *y <= h)
        {
            return;
        }
        let (x, y) = screens[0];
        self.viewport.pan_by_screen_delta(x - w / 2.0, y - h / 2.0);
    }
}
