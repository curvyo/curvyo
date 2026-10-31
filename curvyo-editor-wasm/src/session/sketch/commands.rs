//! Sketch commands persist only explicit differences from their exact resolved base.
use super::Session;
use curvyo_document_core::{SketchDelta, SketchEditError, SketchEntityRecord};
use curvyo_sketch_core::{Construction, Sketch, construct};

impl Session {
    /// Completes one analytic construction as one undoable explicit edit.
    /// # Errors
    /// Rejects missing/currently invalid sketches or degenerate construction input.
    pub fn sketch_construct(&mut self, command: &Construction) -> Result<bool, SketchEditError> {
        let prefix = self.minter.mint().to_hex();
        self.sketch_construct_named(&prefix, command)
    }

    pub(super) fn sketch_construct_named(
        &mut self,
        prefix: &str,
        command: &Construction,
    ) -> Result<bool, SketchEditError> {
        self.recording(|this| {
            this.sync_sketch_page();
            let base = this.sketch_creation_base()?;
            let version = this.document.version();
            let mut operation = construct(&base, prefix, command)
                .map_err(|error| SketchEditError(error.to_string()))?;
            this.constrain_inferred_axes(&base, &mut operation)
                .map_err(|error| SketchEditError(error.to_string()))?;
            let delta = explicit_delta(&base, &operation.intent);
            if this.document.version() != version {
                return Err(SketchEditError("Sketch changed during construction".into()));
            }
            if let Some(id) = this.active_sketch() {
                this.document
                    .apply_sketch_preview(id, &delta, &operation.candidate)
            } else {
                this.materialize_sketch(&operation.intent)
            }
        })
    }
}

/// Both inputs describe the same operation base, never unrelated stored/solved graphs.
pub(super) fn explicit_delta(base: &Sketch, intent: &Sketch) -> SketchDelta {
    let mut delta = SketchDelta::default();
    for id in base.entities.keys().chain(intent.entities.keys()) {
        if base.entities.get(id) != intent.entities.get(id)
            || base.construction.contains(id) != intent.construction.contains(id)
        {
            delta.entities.insert(
                id.clone(),
                intent.entities.get(id).map(|entity| SketchEntityRecord {
                    entity: entity.clone(),
                    construction: intent.construction.contains(id),
                }),
            );
        }
    }
    for id in base.constraints.keys().chain(intent.constraints.keys()) {
        if base.constraints.get(id) != intent.constraints.get(id) {
            delta
                .constraints
                .insert(id.clone(), intent.constraints.get(id).cloned());
        }
    }
    for id in base.parameters.0.keys().chain(intent.parameters.0.keys()) {
        if base.parameters.0.get(id) != intent.parameters.0.get(id) {
            delta
                .parameters
                .insert(id.clone(), intent.parameters.0.get(id).cloned());
        }
    }
    delta
}
