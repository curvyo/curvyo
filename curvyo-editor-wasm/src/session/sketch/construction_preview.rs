//! In-progress analytic construction retains the last valid ephemeral pose.
use super::{Session, SketchTool};
use curvyo_document_core::SketchEditError;
use curvyo_sketch_core::{Construction, Length};

impl Session {
    pub(super) fn refresh_sketch_construction_preview(&mut self) {
        let Some(local) = self.sketch.hover else {
            return;
        };
        if self.sketch.tool == SketchTool::Fillet
            || matches!(
                self.sketch.tool,
                SketchTool::Select | SketchTool::Constraints
            )
        {
            return;
        }
        let mut picks = self.sketch.picks.clone();
        if picks.is_empty()
            && super::creation::pick_count(self.sketch.tool) == 2
            && self.button_down
            && let Some(anchor) = self.sketch.press_anchor
        {
            picks.push(anchor);
        }
        picks.push(local);
        let Some(prefix) = self.sketch.pending_prefix.as_ref() else {
            return;
        };
        self.sketch.degenerate = false;
        let Some(mut command) = super::creation::construction(
            self.sketch.tool,
            &picks,
            prefix,
            self.sketch.chain.as_ref(),
        )
        .or_else(|| {
            (self.sketch.tool == SketchTool::ArcCenter && picks.len() == 2).then(|| {
                Construction::Circle {
                    center: picks[0],
                    radius: picks[0].distance(picks[1]),
                }
            })
        }) else {
            return;
        };
        if let (Some(previous), Some((_, start))) = (&self.sketch.chain_curve, &self.sketch.chain)
            && self.sketch.tool == SketchTool::Polyline
        {
            command = Construction::Continue {
                previous: previous.clone(),
                start: start.clone(),
                end: format!("{prefix}:end").into(),
                position: local,
                mode: self.sketch.mode,
            };
        }
        if let Some((first, last)) = self.sketch_close_target()
            && let Some(previous) = &self.sketch.chain_curve
        {
            command = Construction::Continue {
                previous: previous.clone(),
                start: last,
                end: first,
                position: local,
                mode: self.sketch.mode,
            };
        }
        let step = match self.sketch_capture_join(prefix, &picks) {
            Some(joined) => super::first_segment::PolylineStep::plain(joined),
            None => self.sketch_polyline_step(prefix, &picks[..picks.len() - 1], command, local),
        };
        let step = self.with_lock_relation(prefix, step);
        self.sketch.degenerate = step.degenerate;
        self.sketch.preview = None;
        self.sketch.preview_base = None;
        let Ok(base) = self.sketch_creation_base() else {
            return;
        };
        if let Ok(mut result) = curvyo_sketch_core::construct_preview(&base, prefix, &step.command)
        {
            if let Some((id, constraint)) = step.relation {
                result
                    .intent
                    .constraints
                    .insert(id.clone(), constraint.clone());
                result.candidate.constraints.insert(id, constraint);
            }
            if self.constrain_inferred_axes(&base, &mut result).is_err() {
                self.sketch.preview = None;
                self.sketch.preview_base = None;
                return;
            }
            self.sketch.preview_base = Some((self.document.version(), base));
            self.sketch.preview = Some(result);
        }
    }

    pub(super) fn sketch_construction_preview(
        &self,
    ) -> Option<&curvyo_sketch_core::ConstructionResult> {
        let (revision, _) = self.sketch.preview_base.as_ref()?;
        if *revision != self.document.version() {
            return None;
        }
        self.sketch.preview.as_ref()
    }

    /// Previews a selected joined corner without modifying its existing relationships.
    /// # Errors
    /// Reports invalid radius/selection or unsatisfied relationships; implicated IDs remain visible.
    pub fn sketch_preview_fillet(&mut self, radius: Length) -> Result<(), SketchEditError> {
        self.sketch.preview = None;
        self.sketch.preview_base = None;
        let snapshot = self.sketch_snapshot_for_edit()?;
        if snapshot.stale {
            return Err(SketchEditError(
                "Repair sketch before editing historical geometry".into(),
            ));
        }
        let selected = self.sketch.entities.iter().cloned().collect::<Vec<_>>();
        let [first, second] = selected.as_slice() else {
            return Err(SketchEditError("Select two joined segments".into()));
        };
        let mut references = self.sketch.entities.clone();
        for id in &selected {
            if let Some(entity) = snapshot.graph.entities.get(id) {
                references.extend(entity.point_ids().into_iter().cloned());
            }
        }
        self.sketch.affected = snapshot
            .graph
            .constraints
            .iter()
            .filter(|(_, c)| {
                c.relation
                    .entity_ids()
                    .iter()
                    .any(|id| references.contains(*id))
            })
            .map(|(id, _)| id.clone())
            .collect();
        let prefix = self
            .sketch
            .pending_prefix
            .get_or_insert_with(|| self.minter.mint().to_hex())
            .clone();
        let result = curvyo_sketch_core::construct_preview(
            &snapshot.solution,
            &prefix,
            &Construction::Fillet {
                first: first.clone(),
                second: second.clone(),
                radius,
            },
        );
        match result {
            Ok(preview) => {
                self.sketch.preview_base =
                    Some((self.document.version(), snapshot.solution.clone()));
                self.sketch.preview = Some(preview);
                self.sketch.error = None;
                Ok(())
            }
            Err(error) => {
                self.sketch.preview = None;
                self.sketch.error = Some(error.to_string());
                Err(SketchEditError(error.to_string()))
            }
        }
    }

    /// Commits a previously validated construction preview as one explicit edit.
    /// # Errors
    /// Rejects missing previews or a revision changed since preview construction.
    pub fn sketch_commit_preview(&mut self) -> Result<bool, SketchEditError> {
        self.recording(|this| {
            let snapshot = this.sketch_snapshot_for_edit()?;
            let preview = this
                .sketch
                .preview
                .take()
                .ok_or_else(|| SketchEditError("No valid construction preview".into()))?;
            let (revision, base) = this
                .sketch
                .preview_base
                .take()
                .ok_or_else(|| SketchEditError("Missing construction base".into()))?;
            if revision != this.document.version() {
                return Err(SketchEditError("Sketch changed during preview".into()));
            }
            let delta = super::commands::explicit_delta(&base, &preview.intent);
            this.document
                .apply_sketch_preview(snapshot.id, &delta, &preview.candidate)
        })
    }
}
