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
        let mut view = json!({"workbench":match self.workbench() { super::Workbench::Sketch => "sketch", super::Workbench::Vector => "vector", super::Workbench::Manufacturing(_) => "manufacturing" },"tool":self.sketch_tool().name(),"active":snapshot.as_ref().map(|s| s.id.full_id()),"sketches":sketches,"error":self.sketch.error,"pending_picks":self.sketch.picks.len(),"entities":[],"constraints":[],"parameters":[],"actions":[],"status":null,"stale":false});
        view["draft_epoch"] = json!(self.sketch.draft_epoch);
        view["can_resume"] = json!(self.sketch_can_resume());
        view["constraint_tool"] = json!(self.sketch.constraint_tool);
        view["dimension_position"] = json!(self.sketch.dimension_position);
        view["dimension_action"] = json!(self.sketch_dimension_action());
        view["page"] = json!(self.active_page().id);
        // The Use group of the Sketches tab (`specs/0099` criteria 11, 12).
        let owner = self.document.page_sketch(&self.active_page().id);
        view["usage"] = json!(owner.map(|id| self.sketch_usage(id)).unwrap_or_default());
        view["drawable"] = json!(owner.is_some_and(|id| self.sketch_has_geometry(id)));
        view["mode"] = json!(
            super::modes::MODES
                .iter()
                .find(|(mode, _)| *mode == self.sketch.mode)
                .map(|(_, name)| *name)
        );
        view["degenerate"] = json!(self.sketch.degenerate);
        view["cursor"] = json!(self.sketch_cursor());
        let (mode_label, entry) = (self.sketch_mode_label(), self.sketch_entry());
        // A refusal shown at the object on the canvas is not repeated in Sketches (UX notes
        // revision 2 review, changes 5 and 7).
        if !mode_label["error"].is_null() || !entry["error"].is_null() {
            view["error"] = Value::Null;
        }
        view["canvas"] = json!({"mode_label": mode_label, "entry": entry});
        let mut annotations = snapshot
            .as_ref()
            .map(|s| self.sketch_annotations_for(s))
            .unwrap_or_default();
        annotations.extend(self.sketch_arc_pick_annotations());
        annotations.extend(self.sketch_degenerate_annotation());
        annotations.extend(self.sketch_capture_annotation());
        annotations.extend(self.sketch_lock_annotations());
        view["annotations"] = json!(annotations);
        // An exposed dimension's pill carries the mark (`0099` criterion 23).
        if let Some(snapshot) = snapshot.as_ref() {
            mark_exposed_pills(&mut view["annotations"], snapshot);
        }
        view["rail"] = self.sketch_rail();
        let (symbols, more) = snapshot.as_ref().map_or_else(
            || (Vec::new(), Vec::new()),
            |s| self.sketch_symbol_records(s),
        );
        view["symbols"] =
            json!(symbols.iter().map(|r| json!({
            "id": r.id.0, "glyph": r.glyph, "index": r.index, "name": r.name,
            "x": r.screen.0, "y": r.screen.1, "selected": r.selected, "hovered": r.hovered,
            "conflicting": r.conflicting, "redundant": r.redundant, "dot": r.dot,
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
                json!({"id":id.0,"kind":constraint_kind(&snapshot.graph,&constraint.relation),"entities":constraint.relation.entity_ids().iter().map(|id|id.0.as_str()).collect::<Vec<_>>(),"reference":constraint.reference,"expression":expression,"value":measured,"conflicting":snapshot.conflicting.contains(id),"redundant":snapshot.redundant.contains(id),"selected":self.sketch.constraints.contains(id),
                    "expose": self.dimension_expose(&snapshot, id)})
            }).collect::<Vec<_>>());
            let values = snapshot.graph.parameters.evaluate().unwrap_or_default();
            view["parameters"]=json!(snapshot.graph.parameters.0.iter().map(|(id,p)|{
                json!({"id":id.0,"name":p.name,"dimension":p.dimension,"expression":p.expression.display_text(&snapshot.graph.parameters).ok(),"value":values.get(id),
                    "expose": self.parameter_expose(&snapshot, id, p.dimension)})}).collect::<Vec<_>>());
            view["actions"] = self.sketch_actions_view(&snapshot);
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
impl Session {
    /// What Expose shows for the driving dimension `id` (`0099` criterion 23): its key, whether it
    /// is exposed with its name and limits, how many clones hold an own value of it and the name
    /// it would be exposed under; `null` for a reference dimension or one with no value.
    fn dimension_expose(
        &self,
        snapshot: &curvyo_document_core::SketchSnapshot,
        id: &curvyo_sketch_core::ConstraintId,
    ) -> Value {
        use curvyo_sketch_core::Dimension;
        let key = curvyo_document_core::ExposedKey::dimension(id);
        let exposed = snapshot.exposed.iter().find(|param| param.key == key);
        let dimension = snapshot.graph.exposable_dimension(id);
        if !dimension.is_some_and(|d| d != Dimension::Scalar) {
            return Value::Null;
        }
        json!({
            "key": key,
            "on": exposed.is_some(),
            "name": exposed.map(|param| param.label.clone()),
            "min": exposed.and_then(|param| param.min),
            "max": exposed.and_then(|param| param.max),
            "holders": self.clones_holding(snapshot.id, key.as_str()),
            "suggest": suggested_name(&snapshot.graph, id, &snapshot.exposed),
            "angle": dimension == Some(Dimension::Angle),
        })
    }

    /// What Expose shows for the parameter `id`; `null` for a unitless one.
    fn parameter_expose(
        &self,
        snapshot: &curvyo_document_core::SketchSnapshot,
        id: &curvyo_sketch_core::ParameterId,
        dimension: curvyo_sketch_core::Dimension,
    ) -> Value {
        use curvyo_sketch_core::Dimension;
        if dimension == Dimension::Scalar {
            return Value::Null;
        }
        let key = curvyo_document_core::ExposedKey::parameter(id);
        let exposed = snapshot.exposed.iter().find(|param| param.key == key);
        json!({
            "key": key,
            "on": exposed.is_some(),
            "min": exposed.and_then(|param| param.min),
            "max": exposed.and_then(|param| param.max),
            "holders": self.clones_holding(snapshot.id, key.as_str()),
            "angle": dimension == Dimension::Angle,
        })
    }
}

impl Session {
    /// Which actions of the Sketches tab apply to the selected entities, and why not.
    fn sketch_actions_view(&self, snapshot: &curvyo_document_core::SketchSnapshot) -> Value {
        let selected = self.sketch.entities.iter().cloned().collect::<Vec<_>>();
        json!(
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
        )
    }
}

/// Adds `"exposed": true` to the dimension pills whose dimension a clone may set.
fn mark_exposed_pills(annotations: &mut Value, snapshot: &curvyo_document_core::SketchSnapshot) {
    let exposed: std::collections::HashSet<&str> = snapshot
        .exposed
        .iter()
        .filter_map(|param| param.key.as_str().strip_prefix("d:"))
        .collect();
    let Some(list) = annotations.as_array_mut() else {
        return;
    };
    for annotation in list {
        let marked = annotation["dimension"] == true
            && annotation["id"]
                .as_str()
                .is_some_and(|id| exposed.contains(id));
        if marked {
            annotation["exposed"] = json!(true);
        }
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
pub(super) fn entity_name(entity: &Entity) -> &'static str {
    match entity {
        Entity::Point { .. } => "point",
        Entity::Line { .. } => "line",
        Entity::Circle { .. } => "circle",
        Entity::Arc { .. } => "arc",
        Entity::Ellipse { .. } => "ellipse",
    }
}
/// The kind Sketches names a constraint by; a point-on constraint names its curve (AC37).
pub(super) fn constraint_kind(
    graph: &curvyo_sketch_core::Sketch,
    relation: &Relation,
) -> &'static str {
    match relation {
        Relation::PointOn { curve, .. } => match graph.entities.get(curve) {
            Some(Entity::Line { .. }) => "point-on-line",
            Some(Entity::Circle { .. }) => "point-on-circle",
            Some(Entity::Arc { .. }) => "point-on-arc",
            Some(Entity::Ellipse { .. }) => "point-on-ellipse",
            _ => "point-on",
        },
        _ => relation_name(relation),
    }
}
pub(super) fn relation_name(relation: &Relation) -> &'static str {
    match relation {
        Relation::PointOn { .. } => "point-on",
        Relation::OnAxis { axis, .. } => match axis {
            curvyo_sketch_core::Axis::X => "on-x-axis",
            curvyo_sketch_core::Axis::Y => "on-y-axis",
        },
        Relation::On { .. } => "on",
        Relation::Sweep { .. } => "sweep",
        Relation::Coincident(..) => "coincident",
        Relation::Horizontal(..) | Relation::HorizontalPoints(..) => "horizontal",
        Relation::Vertical(..) | Relation::VerticalPoints(..) => "vertical",
        Relation::Parallel(..) => "parallel",
        Relation::Perpendicular(..) | Relation::PerpendicularAt { .. } => "perpendicular",
        Relation::Tangent(..) => "tangent",
        Relation::Equal(..) => "equal",
        Relation::Fixed(..) => "fixed",
        Relation::Symmetric { .. }
        | Relation::SymmetricPoint { .. }
        | Relation::SymmetricAxis { .. } => "symmetric",
        Relation::Midpoint { .. } => "midpoint",
        Relation::Distance { .. } => "distance",
        Relation::HorizontalDistance { .. } => "horizontal-distance",
        Relation::VerticalDistance { .. } => "vertical-distance",
        Relation::Length { .. } => "length",
        Relation::Radius { .. } | Relation::EllipseRadius { .. } => "radius",
        Relation::Diameter { .. } => "diameter",
        Relation::Angle { .. } | Relation::AxisAngle { .. } => "angle",
    }
}

/// The name prefilled for exposing the dimension `id` (`0099` UX notes 12.1): "Distance 1",
/// "Radius 1", "Diameter 1", "Angle 1", numbered per kind in list order, never one in use.
fn suggested_name(
    graph: &curvyo_sketch_core::Sketch,
    id: &curvyo_sketch_core::ConstraintId,
    exposed: &[curvyo_document_core::ExposedParam],
) -> String {
    let word = |relation: &Relation| match relation {
        Relation::Radius { .. } | Relation::EllipseRadius { .. } => "Radius",
        Relation::Diameter { .. } => "Diameter",
        Relation::Angle { .. } | Relation::AxisAngle { .. } | Relation::Sweep { .. } => "Angle",
        _ => "Distance",
    };
    let Some(constraint) = graph.constraints.get(id) else {
        return "Distance 1".to_owned();
    };
    let kind = word(&constraint.relation);
    let mut number = 1 + graph
        .constraints
        .iter()
        .take_while(|(other, _)| *other != id)
        .filter(|(_, other)| {
            !other.reference
                && curvyo_sketch_core::dimension_expression(&other.relation).is_some()
                && word(&other.relation) == kind
        })
        .count();
    let mut name = format!("{kind} {number}");
    while exposed.iter().any(|param| param.label == name) {
        number += 1;
        name = format!("{kind} {number}");
    }
    name
}
