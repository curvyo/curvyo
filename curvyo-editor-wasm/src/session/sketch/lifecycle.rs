//! Sketch workbench navigation resolves originals without writing document records.
use super::{Session, SketchTool, Workbench};
use curvyo_document_core::{NodeId, ObjectSnapshot, SketchEditError, Style};
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
        if workbench == Workbench::Vector
            && self.panel_tabs.get().active() == curvyo_ui_core::PanelTab::Parameters
        {
            self.press_panel_tab("document");
        }
        if workbench == Workbench::Sketch {
            self.press_panel_tab("sketches");
            let selected = self.selection.ids().to_vec();
            for id in selected {
                if self.enter_sketch(id) {
                    break;
                }
            }
        }
        if workbench == Workbench::Sketch
            && self.document.page_sketch(&self.active_page().id).is_none()
        {
            let page = self.active_page();
            let populated = self.document_read().leaves.iter().any(|object| {
                let (min, max) = curvyo_ui_core::object_bounds(object);
                page.overlaps(min, max)
            });
            // Proposal A: preserve populated Vector pages; create a dedicated source page.
            if populated {
                self.add_page();
            }
        }
        self.sync_sketch_page();
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
        if let Some(page) = self.document.sketch_page(original) {
            if self.document.page_sketch(&page) != Some(original) {
                return false;
            }
            self.activate_page(&page);
            self.sketch.page = Some(page);
        }
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
        if let Some(id) = self.document.page_sketch(&self.active_page().id) {
            return Ok(id);
        }
        self.recording(|this| {
            if let Some(layer) = this.active_layer()
                && this.document_read().inert.contains(&layer)
            {
                return Err(SketchEditError("Choose a visible unlocked layer".into()));
            }
            this.document.set_draw_into_initial_layer(true);
            let origin = this.minter.mint().to_hex().into();
            let id = this.document.create_page_sketch(
                &this.active_page().id,
                &Sketch::new(origin),
                &Style::default(),
            )?;
            this.enter_sketch(id);
            Ok(id)
        })
    }

    pub(in crate::session) fn sync_sketch_page(&mut self) {
        if self.workbench() != Workbench::Sketch {
            return;
        }
        let page = self.active_page().id;
        let owner = self.document.page_sketch(&page);
        if self.sketch.page.as_ref() == Some(&page) && self.sketch.active == owner {
            return;
        }
        self.sketch.cancel();
        self.button_down = false;
        self.sketch.entities.clear();
        self.sketch.constraints.clear();
        self.sketch.page = Some(page);
        self.sketch.active = owner;
        self.sketch.empty = owner
            .is_none()
            .then(|| Sketch::new(self.minter.mint().to_hex().into()));
        if let Some(id) = owner {
            self.selection.select_single(id);
        } else {
            self.selection.clear();
        }
        self.sketch.revision = Some(self.document.version());
    }

    pub(super) fn sketch_intent_for_edit(&self) -> Result<Sketch, SketchEditError> {
        if self.active_sketch().is_some() {
            return Ok(self.sketch_snapshot_for_edit()?.graph);
        }
        self.sketch
            .empty
            .clone()
            .ok_or_else(|| SketchEditError("Choose the Sketch workbench first".into()))
    }

    pub(super) fn materialize_sketch(&mut self, graph: &Sketch) -> Result<bool, SketchEditError> {
        if let Some(layer) = self.active_layer()
            && self.document_read().inert.contains(&layer)
        {
            return Err(SketchEditError("Choose a visible unlocked layer".into()));
        }
        self.document.set_draw_into_initial_layer(true);
        let id =
            self.document
                .create_page_sketch(&self.active_page().id, graph, &Style::default())?;
        self.enter_sketch(id);
        Ok(true)
    }

    pub(super) fn sketch_creation_base(&self) -> Result<Sketch, SketchEditError> {
        if self.active_sketch().is_some() {
            let snapshot = self.sketch_snapshot_for_edit()?;
            if snapshot.stale {
                return Err(SketchEditError(
                    "Repair the sketch before adding geometry".into(),
                ));
            }
            Ok(snapshot.solution.clone())
        } else {
            self.sketch
                .empty
                .clone()
                .ok_or_else(|| SketchEditError("Choose the Sketch workbench first".into()))
        }
    }

    pub(super) fn sketch_placement(&self) -> Option<curvyo_document_core::Affine> {
        if let Some(id) = self.active_sketch() {
            return self.document.sketch(id).map(|s| s.placement);
        }
        self.sketch.empty.as_ref()?;
        let page = self.active_page();
        Some(curvyo_document_core::Affine::translation(
            curvyo_document_core::Vec2::new(
                page.origin.x + page.size.width.as_mm() / 2.0,
                page.origin.y + page.size.height.as_mm() / 2.0,
            ),
        ))
    }

    /// Sets the analytic tool without committing an unfinished construction.
    pub fn set_sketch_tool(&mut self, tool: SketchTool) {
        if matches!(self.sketch.tool, SketchTool::Arc | SketchTool::ArcCenter)
            && matches!(tool, SketchTool::Arc | SketchTool::ArcCenter)
        {
            self.sketch.tool = tool;
            self.sketch.arc_radius = None;
            self.refresh_sketch_construction_preview();
            return;
        }
        self.sketch.cancel();
        self.end_snapping_drag();
        self.sketch.tool = tool;
        self.sketch.constraint_tool = None;
        self.sketch.mode = curvyo_sketch_core::ContinuationMode::FreeLine;
        if matches!(tool, SketchTool::Polyline | SketchTool::Arc) {
            self.resume_sketch_endpoint();
        }
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
        self.sync_sketch_page();
        let version = self.document.version();
        if self.sketch.revision.as_ref() == Some(&version) {
            return;
        }
        self.sketch.cancel();
        self.sketch.active = self.active_sketch();
        if let Some(snapshot) = self.sketch.active.and_then(|id| self.document.sketch(id)) {
            // Selection refers to current intent, never historical fallback geometry.
            // Local constraint edits keep their subjects selected for the next action.
            self.sketch.entities.retain(|id| {
                snapshot.graph.entities.contains_key(id)
                    || curvyo_ui_core::sketch_axis(id).is_some()
            });
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
