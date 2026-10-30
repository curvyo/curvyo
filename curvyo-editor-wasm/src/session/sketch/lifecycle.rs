//! Sketch workbench navigation resolves originals without writing document records.
use super::{Session, SketchTool, Workbench};
use curvyo_document_core::{Affine, NodeId, ObjectSnapshot, SketchEditError, Style};
use curvyo_sketch_core::Sketch;

impl Session {
    /// Current editing context; changing it does not create artwork.
    #[must_use]
    pub const fn workbench(&self) -> Workbench {
        self.sketch.workbench
    }

    /// Switches context while cancelling only unfinished interaction.
    pub fn set_workbench(&mut self, workbench: Workbench) {
        if workbench == self.sketch.workbench {
            return;
        }
        self.cancel_gesture();
        self.cancel_placement();
        self.cancel_pen_entry();
        self.end_snapping_drag();
        self.button_down = false;
        self.sketch.cancel();
        self.sketch.workbench = workbench;
        if workbench == Workbench::Sketch {
            self.press_panel_tab("sketches");
            let selected = self.selection.ids().to_vec();
            for id in selected {
                if self.enter_sketch(id) {
                    break;
                }
            }
        }
        self.sync_sketch();
    }

    /// Active original sketch, excluding deleted or converted objects.
    #[must_use]
    pub fn active_sketch(&self) -> Option<NodeId> {
        self.sketch
            .active
            .filter(|id| self.document.sketch(*id).is_some())
    }

    /// Resolves the picker identity without exposing document internals to the host.
    pub fn enter_sketch_named(&mut self, name: &str) -> bool {
        let id = self
            .document
            .object_ids()
            .into_iter()
            .find(|id| id.full_id() == name);
        id.is_some_and(|id| self.enter_sketch(id))
    }

    /// Re-enters an original, or resolves a linked clone to its parametric root.
    pub fn enter_sketch(&mut self, id: NodeId) -> bool {
        let original = match self.document.object(id) {
            Some(ObjectSnapshot::Sketch(_)) => id,
            Some(ObjectSnapshot::Clone(clone))
                if self.document.sketch(clone.chain.root).is_some() =>
            {
                clone.chain.root
            }
            _ => return false,
        };
        self.sketch.cancel();
        self.sketch.active = Some(original);
        self.sketch.entities.clear();
        self.sketch.constraints.clear();
        self.sketch.revision = Some(self.document.version());
        self.selection.select_single(original);
        true
    }

    /// Creates the fixed local origin as one document history step.
    /// # Errors
    /// Refuses creation outside Sketch or when the document rejects the graph.
    pub fn new_sketch(&mut self) -> Result<NodeId, SketchEditError> {
        if self.workbench() != Workbench::Sketch {
            return Err(SketchEditError("Choose the Sketch workbench first".into()));
        }
        self.recording(|this| {
            if let Some(layer) = this.active_layer()
                && this.document_read().inert.contains(&layer)
            {
                return Err(SketchEditError("Choose a visible unlocked layer".into()));
            }
            this.document.set_draw_into_initial_layer(true);
            let origin = this.minter.mint().to_hex().into();
            let id = this.document.create_sketch(
                &Sketch::new(origin),
                Affine::IDENTITY,
                &Style::default(),
            )?;
            this.enter_sketch(id);
            Ok(id)
        })
    }

    /// Sets the analytic tool without committing an unfinished construction.
    pub fn set_sketch_tool(&mut self, tool: SketchTool) {
        self.sketch.cancel();
        self.end_snapping_drag();
        self.sketch.tool = tool;
    }

    /// The selected sketch tool and construction variant.
    #[must_use]
    pub const fn sketch_tool(&self) -> SketchTool {
        self.sketch.tool
    }

    pub(in crate::session) fn sync_sketch(&mut self) {
        if self.sketch.workbench == Workbench::Vector && self.sketch.active.is_none() {
            return;
        }
        let version = self.document.version();
        if self.sketch.revision.as_ref() == Some(&version) {
            return;
        }
        self.sketch.cancel();
        self.sketch.active = self.active_sketch();
        if let Some(snapshot) = self.sketch.active.and_then(|id| self.document.sketch(id)) {
            // Selection refers to current intent, never historical fallback geometry.
            // Local constraint edits keep their subjects selected for the next action.
            self.sketch
                .entities
                .retain(|id| snapshot.graph.entities.contains_key(id));
            self.sketch
                .constraints
                .retain(|id| snapshot.graph.constraints.contains_key(id));
        } else {
            self.sketch.entities.clear();
            self.sketch.constraints.clear();
        }
        self.sketch.revision = Some(version);
    }
}
