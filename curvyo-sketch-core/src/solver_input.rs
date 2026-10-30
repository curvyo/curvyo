//! The ACS request adapter isolates transient solver identities from stable records.
use crate::{
    Angle, ConstraintId, Entity, EntityId, Length, Position, Sketch, SketchError, SolveMode,
};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) struct Input {
    pub request: Value,
    pub entities: BTreeMap<EntityId, String>,
    pub constraints: BTreeMap<String, ConstraintId>,
    pub contacts: BTreeMap<ConstraintId, String>,
}
impl Input {
    pub fn new(s: &Sketch, mode: &SolveMode, seed_goals: bool) -> Result<Self, SketchError> {
        let entities = s
            .entities
            .keys()
            .enumerate()
            .map(|(i, id)| (id.clone(), format!("e{i}")))
            .collect();
        let mut input = Self {
            request: Value::Null,
            entities,
            constraints: BTreeMap::new(),
            contacts: BTreeMap::new(),
        };
        let mut primitives = Vec::new();
        for (id, e) in &s.entities {
            let mut primitive = input.entity(id, e);
            let goal = match mode {
                SolveMode::Drag { point, target } if point == id => Some(target),
                SolveMode::DragPoints { goals } => goals.get(id),
                _ => None,
            };
            // The pointer is an initial guess, never an authoritative fixed target.
            // Hard constraints below still come from the original graph.
            if seed_goals
                && matches!(e, Entity::Point { fixed: false, .. })
                && let Some(goal) = goal
            {
                primitive["x"] = goal.x.0.into();
                primitive["y"] = goal.y.0.into();
            }
            primitives.push(primitive);
        }
        crate::solver_relations::append(s, &mut input, &mut primitives)?;
        match mode {
            SolveMode::Edit => {}
            SolveMode::DragHandle {
                entity,
                handle,
                target,
            } => crate::handle_goals::append(&input, s, &mut primitives, entity, *handle, *target)?,
            SolveMode::Drag { point, target } => {
                input.append_goal(s, &mut primitives, point, *target, 0)?;
            }
            SolveMode::DragPoints { goals } => {
                for (index, (point, target)) in goals.iter().enumerate() {
                    input.append_goal(s, &mut primitives, point, *target, index)?;
                }
            }
        }
        input.request = json!({"version":1,"analyze":*mode==SolveMode::Edit,"maxIterations":if *mode==SolveMode::Edit{100}else{12},"primitives":primitives});
        Ok(input)
    }
    pub(crate) fn append_goal(
        &self,
        sketch: &Sketch,
        primitives: &mut Vec<Value>,
        point: &EntityId,
        target: Position,
        index: usize,
    ) -> Result<(), SketchError> {
        sketch.point(point)?;
        if !target.x.is_finite() || !target.y.is_finite() {
            return Err(SketchError::Invalid("non-finite drag target".into()));
        }
        let point = self.id(point);
        for (axis, value) in [("x", target.x.0), ("y", target.y.0)] {
            primitives.push(json!({"id":format!("drag_{index}_{axis}"),"type":axis,"point":point,"value":value,"temporary":true}));
        }
        Ok(())
    }
    pub fn id(&self, id: &EntityId) -> &str {
        // invariant: the graph was validated before adapter construction.
        self.entities.get(id).map_or("", String::as_str)
    }
    fn entity(&self, id: &EntityId, e: &Entity) -> Value {
        let id = self.id(id);
        match e {
            Entity::Point { position, fixed } => {
                json!({"type":"point","id":id,"x":position.x.0,"y":position.y.0,"fixed":fixed})
            }
            Entity::Line { start, end } => {
                json!({"type":"line","id":id,"p1_id":self.id(start),"p2_id":self.id(end)})
            }
            Entity::Circle {
                center,
                radius,
                fixed_radius,
            } => {
                json!({"type":"circle","id":id,"c_id":self.id(center),"radius":radius.0,"fixed":fixed_radius})
            }
            Entity::Arc {
                center,
                start,
                end,
                radius,
                start_angle,
                end_angle,
                fixed_shape,
            } => {
                json!({"type":"arc","id":id,"c_id":self.id(center),"start_id":self.id(start),"end_id":self.id(end),"radius":radius.0,"start_angle":start_angle.0,"end_angle":end_angle.0,"fixed":fixed_shape})
            }
            Entity::Ellipse {
                center,
                focus,
                minor_radius,
                fixed_radius,
            } => {
                json!({"type":"ellipse","id":id,"c_id":self.id(center),"focus1_id":self.id(focus),"radmin":minor_radius.0,"fixed":fixed_radius})
            }
        }
    }
    pub fn add_constraint(
        &mut self,
        primitives: &mut Vec<Value>,
        original: &ConstraintId,
        mut value: Value,
    ) {
        let id = format!("k{}", self.constraints.len());
        value["id"] = id.clone().into();
        self.constraints.insert(id, original.clone());
        primitives.push(value);
    }
    pub fn map_fully_constrained(&self, sketch: &Sketch, response: &Value) -> BTreeSet<EntityId> {
        let ids: BTreeSet<_> = response["fullyConstrained"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect();
        // ACS classifies only a primitive's own variables. Analytic geometry also
        // depends on every defining point; lines have no own variables at all.
        sketch
            .entities
            .iter()
            .filter(|(id, entity)| {
                let own = matches!(entity, Entity::Line { .. }) || ids.contains(self.id(id));
                own && entity
                    .point_ids()
                    .iter()
                    .all(|point| ids.contains(self.id(point)))
            })
            .map(|(id, _)| id.clone())
            .collect()
    }
    pub fn map_diagnostics(&self, response: &Value, field: &str) -> BTreeSet<ConstraintId> {
        response[field]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .filter_map(|id| self.constraints.get(id).cloned())
            .collect()
    }
    pub fn read_contacts(
        &self,
        response: &Value,
    ) -> Result<BTreeMap<ConstraintId, Position>, SketchError> {
        let ps = response["primitives"]
            .as_array()
            .ok_or_else(|| SketchError::Solver("missing geometry".into()))?;
        self.contacts
            .iter()
            .map(|(id, q)| {
                let p = ps
                    .iter()
                    .find(|p| p["id"] == *q)
                    .ok_or_else(|| SketchError::Solver("missing contact".into()))?;
                Ok((id.clone(), Position::mm(number(p, "x")?, number(p, "y")?)))
            })
            .collect()
    }
    pub fn read_candidate(&self, s: &Sketch, response: &Value) -> Result<Sketch, SketchError> {
        let primitives = response["primitives"]
            .as_array()
            .ok_or_else(|| SketchError::Solver("missing solved geometry".into()))?;
        let mut candidate = s.clone();
        for (id, entity) in &mut candidate.entities {
            let p = primitives
                .iter()
                .find(|p| p["id"] == self.id(id))
                .ok_or_else(|| SketchError::Solver(format!("missing solved entity {}", id.0)))?;
            match entity {
                Entity::Point { position, .. } => {
                    *position = Position::mm(number(p, "x")?, number(p, "y")?);
                }
                Entity::Circle { radius, .. } => *radius = Length(number(p, "radius")?),
                Entity::Arc {
                    radius,
                    start_angle,
                    end_angle,
                    ..
                } => {
                    *radius = Length(number(p, "radius")?);
                    *start_angle = Angle(number(p, "start_angle")?);
                    *end_angle = Angle(number(p, "end_angle")?);
                }
                Entity::Ellipse { minor_radius, .. } => {
                    *minor_radius = Length(number(p, "radmin")?);
                }
                Entity::Line { .. } => {}
            }
        }
        candidate.validate()?;
        Ok(candidate)
    }
}
fn number(p: &Value, field: &str) -> Result<f64, SketchError> {
    p[field]
        .as_f64()
        .filter(|v| v.is_finite())
        .ok_or_else(|| SketchError::Solver(format!("non-finite or missing {field}")))
}
