//! Sketch view records expose actionable domain state to the existing Properties shell.
use super::Session;
use curvyo_document_core::ObjectSnapshot;
use curvyo_sketch_core::{Entity, Expression, Quantity, Relation};
use serde_json::{Value, json};

impl Session {
    /// Read-only workbench, sketch choices, geometry, diagnostics and editable values.
    #[must_use]
    pub fn sketch_view_json(&self) -> String {
        self.sketch_view().to_string()
    }
    pub(super) fn sketch_view(&self) -> Value {
        let read = self.document_read();
        let sketches=read.nodes.iter().filter(|node| matches!(node.object(),Some(ObjectSnapshot::Sketch(_)))).map(|node| {
            json!({"id":node.id.full_id(),"name":node.registers.name.clone().unwrap_or_else(||format!("Sketch {}",node.id.short_id())),"editable":node.visible&&!node.locked})
        }).collect::<Vec<_>>();
        let snapshot = self.sketch.active.and_then(|id| self.document.sketch(id));
        let mut view = json!({"workbench":if self.workbench()==super::Workbench::Sketch {"sketch"} else {"vector"},"tool":self.sketch_tool().name(),"active":snapshot.as_ref().map(|s| s.id.full_id()),"sketches":sketches,"error":self.sketch.error,"pending_picks":self.sketch.picks.len(),"entities":[],"constraints":[],"parameters":[],"actions":[],"status":null,"stale":false});
        view["draft_epoch"] = json!(self.sketch.draft_epoch);
        view["can_resume"] = json!(self.sketch_can_resume());
        view["constraint_tool"] = json!(self.sketch.constraint_tool);
        view["dimension_position"] = json!(self.sketch.dimension_position);
        view["dimension_action"] = json!(self.sketch_dimension_action());
        view["page"] = json!(self.active_page().id);
        view["mode"] = json!(
            super::modes::MODES
                .iter()
                .find(|(mode, _)| *mode == self.sketch.mode)
                .map(|(_, name)| *name)
        );
        view["degenerate"] = json!(self.sketch.degenerate);
        view["cursor"] = json!(self.sketch_cursor());
        view["canvas"] =
            json!({"mode_label": self.sketch_mode_label(), "entry": self.sketch_entry()});
        let mut annotations = snapshot
            .as_ref()
            .map(|s| self.sketch_annotations_for(s))
            .unwrap_or_default();
        annotations.extend(self.sketch_arc_pick_annotations());
        annotations.extend(self.sketch_degenerate_annotation());
        annotations.extend(self.sketch_capture_annotation());
        annotations.extend(self.sketch_lock_annotations());
        view["annotations"] = json!(annotations);
        view["rail"] = self.sketch_rail();
        let (symbols, more) = snapshot.as_ref().map_or_else(
            || (Vec::new(), Vec::new()),
            |s| self.sketch_symbol_records(s),
        );
        view["symbols"] =
            json!(symbols.iter().map(|r| json!({
            "id": r.id.0, "glyph": r.glyph, "index": r.index, "name": r.name,
            "x": r.screen.0, "y": r.screen.1, "selected": r.selected, "hovered": r.hovered,
            "conflicting": r.conflicting, "dot": r.dot,
        })).collect::<Vec<_>>());
        view["symbols_more"] = json!(
            more.iter()
                .map(|m| json!({
                    "count": m.count, "x": m.screen.0, "y": m.screen.1,
                }))
                .collect::<Vec<_>>()
        );
        view["edit_constraint"] = json!(self.sketch.edit_constraint);
        view["affected_constraints"] = json!(self.sketch.affected);
        if let Some(snapshot) = snapshot {
            let solution = self.sketch_preview().unwrap_or(&snapshot.solution);
            view["entities"]=json!(snapshot.graph.entities.iter().filter(|(id,_)|!internal_focus(&snapshot.graph,id)).map(|(id,entity)| {
                json!({"id":id.0,"kind":entity_name(entity),"selected":self.sketch.entities.contains(id),"construction":snapshot.graph.construction.contains(id),"fully_constrained":snapshot.fully_constrained.as_ref().map(|set|set.contains(id)),"origin":*id==snapshot.graph.origin})
            }).collect::<Vec<_>>());
            view["constraints"]=json!(snapshot.graph.constraints.iter().map(|(id,constraint)| {
                let expression=constraint.relation.expression().and_then(|e|e.display_text(&snapshot.graph.parameters).ok());
                let measured=curvyo_sketch_core::measure(solution,&constraint.relation).ok();
                json!({"id":id.0,"kind":relation_name(&constraint.relation),"entities":constraint.relation.entity_ids().iter().map(|id|id.0.as_str()).collect::<Vec<_>>(),"reference":constraint.reference,"expression":expression,"value":measured,"conflicting":snapshot.conflicting.contains(id),"redundant":snapshot.redundant.contains(id),"selected":self.sketch.constraints.contains(id)})
            }).collect::<Vec<_>>());
            let values = snapshot.graph.parameters.evaluate().unwrap_or_default();
            view["parameters"]=json!(snapshot.graph.parameters.0.iter().map(|(id,p)|json!({"id":id.0,"name":p.name,"dimension":p.dimension,"expression":p.expression.display_text(&snapshot.graph.parameters).ok(),"value":values.get(id)})).collect::<Vec<_>>());
            let selected = self.sketch.entities.iter().cloned().collect::<Vec<_>>();
            view["actions"] = json!(
                curvyo_ui_core::SKETCH_ACTIONS
                    .iter()
                    .map(|action| {
                        let value = if *action == "angle" {
                            Quantity::angle(1.0)
                        } else {
                            Quantity::length(1.0)
                        }
                        .ok()
                        .map(Expression::Literal);
                        let result = curvyo_ui_core::sketch_relation(
                            action,
                            &snapshot.graph,
                            &selected,
                            value.as_ref(),
                        );
                        json!({"name":action,"enabled":result.is_ok(),"reason":result.err()})
                    })
                    .collect::<Vec<_>>()
            );
            view["status"] = json!(snapshot.status);
            view["stale"] = json!(snapshot.stale);
            view["diagnosis"] = json!(snapshot.diagnosis);
            view["origin_fixed"] = json!(matches!(
                snapshot.graph.entities.get(&snapshot.graph.origin),
                Some(Entity::Point { fixed: true, .. })
            ));
        }
        view
    }
}
pub(super) fn internal_focus(
    graph: &curvyo_sketch_core::Sketch,
    id: &curvyo_sketch_core::EntityId,
) -> bool {
    graph
        .entities
        .values()
        .any(|e| matches!(e,Entity::Ellipse {focus,..} if focus==id))
}
fn entity_name(entity: &Entity) -> &'static str {
    match entity {
        Entity::Point { .. } => "point",
        Entity::Line { .. } => "line",
        Entity::Circle { .. } => "circle",
        Entity::Arc { .. } => "arc",
        Entity::Ellipse { .. } => "ellipse",
    }
}
pub(super) fn relation_name(relation: &Relation) -> &'static str {
    match relation {
        Relation::OnAxis { axis, .. } => match axis {
            curvyo_sketch_core::Axis::X => "on-x-axis",
            curvyo_sketch_core::Axis::Y => "on-y-axis",
        },
        Relation::On { .. } => "on",
        Relation::Sweep { .. } => "sweep",
        Relation::Coincident(..) => "coincident",
        Relation::Horizontal(..) => "horizontal",
        Relation::Vertical(..) => "vertical",
        Relation::Parallel(..) => "parallel",
        Relation::Perpendicular(..) | Relation::PerpendicularAt { .. } => "perpendicular",
        Relation::Tangent(..) => "tangent",
        Relation::Equal(..) => "equal",
        Relation::Fixed(..) => "fixed",
        Relation::Symmetric { .. } => "symmetric",
        Relation::Midpoint { .. } => "midpoint",
        Relation::Distance { .. } => "distance",
        Relation::Length { .. } => "length",
        Relation::Radius { .. } => "radius",
        Relation::Diameter { .. } => "diameter",
        Relation::Angle { .. } => "angle",
    }
}
