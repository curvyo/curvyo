//! Sketch relationship edits retain inspectable conflicts as explicit document intent.
use super::Session;
use curvyo_document_core::{SketchDelta, SketchEditError, SketchEntityRecord};
use curvyo_sketch_core::{Constraint, ConstraintId, Entity, EntityId, Parameter, ParameterId};
use std::collections::{BTreeMap, BTreeSet};

impl Session {
    /// Adds or replaces one relationship or dimension in one history step.
    /// # Errors
    /// Rejects malformed references, units or expressions; geometric conflicts remain inspectable.
    pub fn sketch_set_constraint(
        &mut self,
        id: ConstraintId,
        constraint: Constraint,
    ) -> Result<bool, SketchEditError> {
        self.sketch_apply_delta(&SketchDelta {
            constraints: BTreeMap::from([(id, Some(constraint))]),
            ..SketchDelta::default()
        })
    }

    /// Removes one constraint without deleting the geometry it described.
    /// # Errors
    /// Refuses when no sketch is active.
    pub fn sketch_remove_constraint(&mut self, id: &ConstraintId) -> Result<bool, SketchEditError> {
        self.sketch_apply_delta(&SketchDelta {
            constraints: BTreeMap::from([(id.clone(), None)]),
            ..SketchDelta::default()
        })
    }

    /// Adds or replaces one named typed parameter, preserving its stable identity.
    /// # Errors
    /// Rejects duplicate names, invalid units, missing references and expression cycles.
    pub fn sketch_set_parameter(
        &mut self,
        id: ParameterId,
        parameter: Parameter,
    ) -> Result<bool, SketchEditError> {
        self.sketch_apply_delta(&SketchDelta {
            parameters: BTreeMap::from([(id, Some(parameter))]),
            ..SketchDelta::default()
        })
    }

    /// Deletes a parameter only when no authoritative expression still depends on it.
    /// # Errors
    /// Reports invalid remaining parameter/dimension references.
    pub fn sketch_remove_parameter(&mut self, id: &ParameterId) -> Result<bool, SketchEditError> {
        self.sketch_apply_delta(&SketchDelta {
            parameters: BTreeMap::from([(id.clone(), None)]),
            ..SketchDelta::default()
        })
    }

    /// Releases or fixes the local origin explicitly.
    /// # Errors
    /// Refuses missing sketch/origin records.
    pub fn sketch_fix_origin(&mut self, fixed: bool) -> Result<bool, SketchEditError> {
        let snapshot = self.sketch_snapshot_for_edit()?;
        let mut entity = snapshot
            .graph
            .entities
            .get(&snapshot.graph.origin)
            .cloned()
            .ok_or_else(|| SketchEditError("Origin no longer exists".into()))?;
        if let Entity::Point { fixed: value, .. } = &mut entity {
            *value = fixed;
        }
        self.sketch_apply_delta(&SketchDelta {
            entities: BTreeMap::from([(
                snapshot.graph.origin,
                Some(SketchEntityRecord {
                    entity,
                    construction: false,
                }),
            )]),
            ..SketchDelta::default()
        })
    }

    /// Deletes selected current entities and their explicitly dependent relationships.
    /// # Errors
    /// Refuses the origin, missing identities or invalid remaining geometry.
    pub fn sketch_delete_entities(
        &mut self,
        ids: &BTreeSet<EntityId>,
    ) -> Result<bool, SketchEditError> {
        let snapshot = self.sketch_snapshot_for_edit()?;
        let intent = curvyo_sketch_core::remove_entities(&snapshot.graph, ids)
            .map_err(|e| SketchEditError(e.to_string()))?;
        self.sketch_apply_delta(&super::commands::explicit_delta(&snapshot.graph, &intent))
    }

    /// Deletes the inspected relationships or selected non-origin geometry in one edit.
    /// # Errors
    /// Reports invalid current editing context or dependent geometry.
    pub fn sketch_delete_selection(&mut self) -> Result<bool, SketchEditError> {
        if !self.sketch.constraints.is_empty() {
            let delta = SketchDelta {
                constraints: self
                    .sketch
                    .constraints
                    .iter()
                    .map(|id| (id.clone(), None))
                    .collect(),
                ..SketchDelta::default()
            };
            return self.sketch_apply_delta(&delta);
        }
        let snapshot = self.sketch_snapshot_for_edit()?;
        let mut ids = self.sketch.entities.clone();
        ids.remove(&snapshot.graph.origin);
        self.sketch_delete_entities(&ids)
    }

    /// Changes construction membership of the current analytic selection.
    /// # Errors
    /// Reports invalid editing context or missing entities.
    pub fn sketch_selection_construction(
        &mut self,
        construction: bool,
    ) -> Result<bool, SketchEditError> {
        self.sketch_set_construction(&self.sketch.entities.clone(), construction)
    }

    /// Toggles whether selected geometry participates in production output.
    /// # Errors
    /// Rejects missing current entity identities.
    pub fn sketch_set_construction(
        &mut self,
        ids: &BTreeSet<EntityId>,
        construction: bool,
    ) -> Result<bool, SketchEditError> {
        let snapshot = self.sketch_snapshot_for_edit()?;
        let mut delta = SketchDelta::default();
        for id in ids {
            let entity = snapshot
                .graph
                .entities
                .get(id)
                .cloned()
                .ok_or_else(|| SketchEditError("Entity no longer exists".into()))?;
            delta.entities.insert(
                id.clone(),
                Some(SketchEntityRecord {
                    entity,
                    construction,
                }),
            );
        }
        self.sketch_apply_delta(&delta)
    }

    pub(super) fn sketch_snapshot_for_edit(
        &self,
    ) -> Result<curvyo_document_core::SketchSnapshot, SketchEditError> {
        if let Some(id) = self.active_sketch()
            && self.document_read().inert.contains(&id)
        {
            return Err(SketchEditError("Sketch is hidden or locked".into()));
        }
        self.active_sketch()
            .and_then(|id| self.document.sketch(id))
            .ok_or_else(|| SketchEditError("Create or select a sketch first".into()))
    }

    pub(super) fn sketch_apply_delta(
        &mut self,
        delta: &SketchDelta,
    ) -> Result<bool, SketchEditError> {
        let result = self.recording(|this| {
            let id = this.sketch_snapshot_for_edit()?.id;
            this.document.apply_sketch_delta(id, delta)
        });
        self.sketch.error = result.as_ref().err().map(ToString::to_string);
        result
    }
}
