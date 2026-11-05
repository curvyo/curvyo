//! Sketch workbench navigation resolves originals without writing document records.
use super::{Session, SketchTool, Workbench};
use curvyo_document_core::{NodeId, ObjectSnapshot, PageKind, SketchEditError, Style};
use curvyo_sketch_core::Sketch;

impl Session {
    /// The workbench of the active page (ADR 0042 §7): derived from its kind, never set.
    #[must_use]
    pub fn workbench(&self) -> Workbench {
        self.workbench_of_kind(&self.active_kind())
    }

    /// The kind of the active page.
    pub(in crate::session) fn active_kind(&self) -> PageKind {
        // The stored active page is read by its own registers; only a page that is gone needs
        // the page list's fallback.
        self.page_view
            .active
            .as_deref()
            .and_then(|id| self.document.page_kind(id))
            .unwrap_or_else(|| self.active_page().kind)
    }

    /// The workbench a page of `kind` shows: a sketch page the Sketch workbench, an output page
    /// of an enabled process its manufacturing, every other page (drawing, unknown, an output
    /// page whose extension is off) the vector tools.
    #[must_use]
    pub fn workbench_of_kind(&self, kind: &PageKind) -> Workbench {
        match kind {
            PageKind::Sketch => Workbench::Sketch,
            PageKind::Output {
                manufacturing,
                process,
            } if self.process_enabled(process) => Workbench::Manufacturing(*manufacturing),
            _ => Workbench::Vector,
        }
    }

    /// Applies a change of the active page's workbench (criteria 8, 9): unfinished interaction
    /// ends, the tab follows (the tab last used on a page of that kind when the open one is not
    /// there, else Document) and the Sketch workbench enters the page's sketch. Not a history
    /// step; nothing is written.
    pub(in crate::session) fn sync_workbench(&mut self) {
        let to = self.workbench();
        let from = self.sketch.workbench;
        if to != from {
            self.cancel_gesture();
            self.cancel_placement();
            self.cancel_pen_entry();
            self.end_snapping_drag();
            self.button_down = false;
            self.sketch.cancel();
            self.sketch.workbench = to;
        }
        // Two pages of one workbench can differ in kind (a drawing and a laser page with its
        // extension off): the tab follows the kind.
        self.follow_tab(to);
        if to == from {
            return;
        }
        if to == Workbench::Sketch {
            self.sketch.page = None;
            self.sync_sketch_page();
            self.sync_sketch();
        } else if from == Workbench::Sketch {
            self.sketch.active = None;
            self.sketch.page = None;
            self.sketch.entities.clear();
            self.sketch.constraints.clear();
        }
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
        self.sketch.chain_created.clear();
        self.end_snapping_drag();
        self.sketch.tool = tool;
        self.sketch.constraint_tool = None;
        self.sketch.mode = curvyo_sketch_core::ContinuationMode::FreeLine;
        // Only a selected open endpoint resumes; any other leftover selection (a free point the
        // Point tool left selected) is no continuation start and stays silent.
        if matches!(tool, SketchTool::Polyline | SketchTool::Arc) && self.sketch_can_resume() {
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
