//! Session commands targeting spatial page frames.
use super::{Session, SizeOutcome};
use curvyo_document_core::{DocumentSize, NodeId, ObjectSnapshot, Page, PageKind, Point};
use curvyo_ui_core::{PageMembership, Reach, object_bounds};

/// How far a press may travel, in screen px, and still be a click that activates a page.
const CLICK_SLOP_PX: f64 = 4.0;
impl Session {
    /// The current active frame, resolved after undo, merge or deletion.
    #[must_use]
    #[allow(clippy::missing_panics_doc)] // Document::pages always exposes a frame.
    pub fn active_page(&self) -> Page {
        let pages = self.document.pages();
        // invariant: the document exposes at least one frame.
        #[allow(clippy::unwrap_used)]
        self.page_view.active(&pages).unwrap().clone()
    }
    /// Activates a row, panning only when the frame is outside the viewport.
    pub fn activate_page(&mut self, id: &str) -> bool {
        let Some((page, changed, left)) = self.switch_active_page(id) else {
            return false;
        };
        let (w, h) = self.viewport.canvas_size();
        let min = self.view().screen_to_document(0.0, 0.0);
        let max = self.view().screen_to_document(w, h);
        let centre = Point::new(
            page.origin.x + page.size.width.as_mm() / 2.0,
            page.origin.y + page.size.height.as_mm() / 2.0,
        );
        let (x, y) = self.view().document_to_screen(centre);
        // A newly active sketch page, and any page whose kind differs from the one left (the
        // way back from a sketch page beside the drawing, main's b1b9d359), shows its centre.
        let workbench = self.workbench();
        let reveal = changed
            && (workbench == super::Workbench::Sketch || workbench != left)
            && (!(0.0..=w).contains(&x) || !(0.0..=h).contains(&y));
        if !page.overlaps(min, max) || reveal {
            self.viewport.pan_by_screen_delta(x - w / 2.0, y - h / 2.0);
        }
        true
    }

    /// Activates the page `id` without moving the view: the switch of a double-click or Enter
    /// on an out-of-context object (`specs/0106` criteria 13 to 16, UX notes §4).
    pub(super) fn activate_page_in_place(&mut self, id: &str) -> bool {
        self.switch_active_page(id).is_some()
    }

    /// Makes `id` the active page: tools, tabs and the sketch follow. The page, whether it
    /// changed and the workbench left; `None` for an unknown page.
    fn switch_active_page(&mut self, id: &str) -> Option<(Page, bool, super::Workbench)> {
        // One decode of the frame list per call: with hundreds of frames it dominates the cost.
        let pages = self.document.pages();
        let page = pages.iter().find(|p| p.id == id).cloned()?;
        let changed = self.page_view.active(&pages).is_none_or(|p| p.id != id);
        if changed {
            // A page switch cancels the open tiling bar (`0069` criterion 35).
            self.tiling = None;
            self.note_active_drawing_page();
        }
        self.page_view.active = Some(id.to_owned());
        let left = self.sketch.workbench;
        // The tools and tabs follow the page's kind in the same frame (`specs/0098` criterion 8).
        self.sync_workbench();
        self.sync_sketch_page();
        self.note_active_drawing_page();
        if page.kind != PageKind::Sketch {
            self.last_work_page = Some(page.id.clone());
            // A page's sketch is selected only on its sketch page (ADR 0043 §2).
            self.drop_fixed_sketches_from_selection();
        }
        Some((page, changed, left))
    }
    /// Adds and activates the next drawing page as one history step.
    pub fn add_page(&mut self) -> String {
        let page = self.recording_unguarded(|this| this.document.add_page());
        self.activate_page(&page.id);
        page.id
    }

    /// Add page with the Kind switch (`specs/0098` criteria 1 to 4): `drawing`, `sketch` or an
    /// enabled output process (`curvyo.laser`, with the machine `profile` or the last pick).
    /// The new page is active; `None` for a kind that cannot be added.
    pub fn add_page_of_kind(&mut self, kind: &str, profile: Option<&str>) -> Option<String> {
        let kind = match kind {
            "drawing" => PageKind::Drawing,
            "sketch" => PageKind::Sketch,
            process => return self.add_output_page_with(process, profile),
        };
        let page = self.recording_unguarded(|this| this.document.add_page_of(&kind).ok())?;
        self.activate_page(&page.id);
        Some(page.id)
    }

    /// Ctrl+Page Down and Ctrl+Page Up (Cmd on macOS; criterion 8): the next or previous page in
    /// list order, stopping at the ends. `false` at an end.
    pub fn activate_adjacent_page(&mut self, forward: bool) -> bool {
        let pages = self.document.pages();
        let active = self.active_page().id;
        let Some(index) = pages.iter().position(|p| p.id == active) else {
            return false;
        };
        let next = if forward {
            index + 1
        } else {
            match index.checked_sub(1) {
                Some(previous) => previous,
                None => return false,
            }
        };
        match pages.get(next) {
            Some(page) => {
                let id = page.id.clone();
                self.activate_page(&id)
            }
            None => false,
        }
    }

    /// The page switch chord, before every other key (criterion 8): Ctrl or Cmd with Page Down
    /// or Page Up, not in a text field.
    pub(super) fn page_switch_key(
        &mut self,
        input: &super::KeyInput<'_>,
    ) -> Option<super::KeyOutcome> {
        let forward = match input.key {
            "PageDown" => true,
            "PageUp" => false,
            _ => return None,
        };
        if !input.ctrl || input.shift || input.alt || input.dom_blocked {
            return None;
        }
        self.activate_adjacent_page(forward);
        Some(super::KeyOutcome::Consumed)
    }

    /// A press inside the frame of another page whose workbench differs (or another sketch
    /// page): activates that page only and starts nothing (criterion 8).
    pub(super) fn activate_page_of_other_kind_at(&mut self, point: Point) -> bool {
        let active = self.active_page();
        if active.contains(point) {
            return false;
        }
        let current = self.workbench();
        let pages = self.document.pages();
        let Some(page) = pages.iter().rev().find(|p| p.contains(point)) else {
            return false;
        };
        let other = self.workbench_of_kind(&page.kind);
        if other == current && current != super::Workbench::Sketch {
            return false;
        }
        // A press on an object of any context selects it and keeps the active page
        // (`specs/0106` criterion 2); the page's sketch is never pickable. On a sketch page a
        // press on sketch geometry wins (criterion 3).
        if self.press_reaches_selection(point)
            && (current != super::Workbench::Sketch || !self.sketch_geometry_at(point))
        {
            return false;
        }
        let id = page.id.clone();
        self.button_down = false;
        self.activate_page(&id)
    }
    /// The page context of a vector press at `point`: a sketch frame is no page of a drawing
    /// object pressed over it (`specs/0099` criterion 2); a press there that hits nothing already
    /// activated the sketch page.
    pub(super) fn press_page_view(&mut self, point: Point) {
        let pages = self.non_sketch_pages();
        // A press on an object keeps the active page (`specs/0106` criterion 2). With the Select
        // tool an empty press activates its page on the release of a click, so neither a move
        // nor a marquee started there switches the page (criteria 2, 3 and 5).
        let hit = self.press_reaches_selection(point);
        let defer = !hit && self.tool == super::Tool::Select;
        self.edit_context.empty_press = defer.then_some(point);
        self.page_view.press(&pages, point, hit || defer);
        self.note_active_drawing_page();
    }

    /// The release of a vector press: an empty Select click activates the page it was in.
    pub(super) fn release_page_view(&mut self, point: Point) {
        let Some(press) = self.edit_context.empty_press.take() else {
            return;
        };
        if press.vector_to(point).length() * self.view().scale() > CLICK_SLOP_PX {
            return;
        }
        let pages = self.non_sketch_pages();
        self.page_view.press(&pages, press, false);
        self.note_active_drawing_page();
    }

    fn non_sketch_pages(&self) -> Vec<Page> {
        self.document
            .pages()
            .into_iter()
            .filter(|p| p.kind != PageKind::Sketch)
            .collect()
    }
    /// Whether a press at `point` reaches an object or the Select tool's box and handles of the
    /// selection: such a press never activates a page (`specs/0106` criteria 2 and 5).
    pub(super) fn press_reaches_selection(&self, point: Point) -> bool {
        self.press_hits_object(point)
            || (self.tool == super::Tool::Select && self.press_reaches_selection_box(point))
    }

    /// Whether a press at `point` reaches the selection's box or one of its handles.
    pub(super) fn press_reaches_selection_box(&self, point: Point) -> bool {
        // On a sketch page the page's sketch is selected and has no box (ADR 0043 §2).
        if self.selection.ids().is_empty() || self.workbench() == super::Workbench::Sketch {
            return false;
        }
        let objects = self.objects();
        !matches!(
            curvyo_ui_core::classify_press(
                &objects,
                &self.pickable_objects(),
                &self.selection,
                point,
                self.object_tolerance(),
                self.transform_handle_tolerances(),
                curvyo_ui_core::Modifiers {
                    alt: false,
                    ..self.held
                },
            ),
            curvyo_ui_core::PressTarget::Empty | curvyo_ui_core::PressTarget::Lasso
        )
    }

    /// Whether a press at `point` hits a pickable object of the context.
    pub(super) fn press_hits_object(&self, point: Point) -> bool {
        !curvyo_ui_core::hit_test_objects(&self.pickable_objects(), point, self.object_tolerance())
            .is_empty()
    }
    /// Renames a frame, returning false for invalid or unchanged names.
    pub fn rename_page(&mut self, id: &str, name: &str) -> bool {
        self.recording_unguarded(|this| this.document.rename_page(id, name).unwrap_or(false))
    }
    /// Deletes a frame, choosing the adjacent row if it was active.
    pub fn delete_page(&mut self, id: &str) -> bool {
        let pages = self.document.pages();
        let Some(index) = pages.iter().position(|p| p.id == id) else {
            return false;
        };
        let active = self.active_page().id == id;
        let changed = self.recording_unguarded(|this| this.document.delete_page(id).is_ok());
        if changed && active {
            let pages = self.document.pages();
            self.page_view.active = Some(pages[index.min(pages.len() - 1)].id.clone());
        }
        self.sync_workbench();
        changed
    }
    /// Reorders a frame without changing its position.
    pub fn reorder_page(&mut self, id: &str, index: usize) -> bool {
        self.recording_unguarded(|this| this.document.reorder_page(id, index).unwrap_or(false))
    }
    /// Sets the per-user frame-only movement switch.
    pub fn set_page_frame_only(&mut self, id: &str, on: bool) {
        if on {
            self.page_view.frame_only.insert(id.to_owned());
        } else {
            self.page_view.frame_only.remove(id);
        }
    }
    /// The membership read of ADR 0043 §1 over every page.
    pub(super) fn page_membership(&self) -> PageMembership {
        PageMembership::of(&self.document, &self.document.pages())
    }

    /// What a frame move of `page` carries (`0059` criterion 9, ADR 0043 §3): the members wholly
    /// on it; for a sketch page exactly its sketch.
    /// Whether a frame move of `page` leaves its content: the per-user switch, which a sketch
    /// page does not have (`specs/0099-sketch-pages` criterion 5).
    pub(super) fn frame_moves_alone(&self, page: &Page) -> bool {
        page.kind != PageKind::Sketch && self.page_view.frame_only.contains(&page.id)
    }

    pub(super) fn contained_page_objects(&self, page: &Page) -> Vec<NodeId> {
        if page.kind == PageKind::Sketch {
            return self.document.page_sketch(&page.id).into_iter().collect();
        }
        let membership = self.page_membership();
        self.document_read()
            .leaves
            .iter()
            .filter(|object| {
                membership.is_member(page, object.id(), object_bounds(object), Reach::Inside)
            })
            .map(ObjectSnapshot::id)
            .collect()
    }
    /// Moves the frame to a global position, optionally carrying contained objects.
    pub fn move_page_to(&mut self, id: &str, origin: Point) -> bool {
        let Some(page) = self.document.pages().into_iter().find(|p| p.id == id) else {
            return false;
        };
        let objects = if self.frame_moves_alone(&page) {
            Vec::new()
        } else {
            self.contained_page_objects(&page)
        };
        self.recording_unguarded(|this| {
            this.document
                .move_page(id, page.origin.vector_to(origin), &objects)
                .unwrap_or(false)
        })
    }
    pub(super) fn resize_active_page(&mut self, size: DocumentSize) -> SizeOutcome {
        let page = self.active_page();
        // A laser page is as big as its machine; an unknown page keeps its size (ADR 0042 §3).
        if page.kind.fixed_size() {
            return SizeOutcome::Unchanged;
        }
        match self.document.resize_page(&page.id, size) {
            Ok(true) => SizeOutcome::Committed,
            Ok(false) => SizeOutcome::Unchanged,
            Err(_) => SizeOutcome::Invalid,
        }
    }
    pub(super) fn fit_active_page(&mut self) -> super::FitOutcome {
        let page = self.active_page();
        let read = self.document_read();
        let membership = self.page_membership();
        let objects: Vec<_> = read
            .leaves
            .iter()
            .filter(|o| {
                let bounds = curvyo_ui_core::object_outline_bounds(o);
                membership.is_member(&page, o.id(), bounds, Reach::Touches)
            })
            .collect();
        if objects.is_empty() {
            return super::FitOutcome::Empty;
        }
        let mut min = Point::new(f64::INFINITY, f64::INFINITY);
        let mut max = Point::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
        for object in &objects {
            let (a, b) = curvyo_ui_core::object_outline_bounds(object);
            min = Point::new(min.x.min(a.x), min.y.min(a.y));
            max = Point::new(max.x.max(b.x), max.y.max(b.y));
        }
        // The frame goes onto the content; no object moves (ADR 0031 decision 9a).
        match self.document.fit_page_to_content(&page.id, (min, max)) {
            Ok(true) => super::FitOutcome::Fitted,
            Ok(false) => super::FitOutcome::AlreadyFits,
            Err(_) => super::FitOutcome::TooLarge(curvyo_ui_core::content_too_large_message(
                self.display_unit(),
            )),
        }
    }
    /// The objects of `objects` Select all takes.
    pub(super) fn page_selection_ids(&self, objects: &[ObjectSnapshot]) -> Vec<NodeId> {
        // Select all is "edit everything here": the objects in context of the active page
        // (`specs/0106` criterion 3).
        let targets: std::collections::HashSet<NodeId> =
            self.edit_targets().iter().map(ObjectSnapshot::id).collect();
        objects
            .iter()
            .map(ObjectSnapshot::id)
            .filter(|id| targets.contains(id))
            .collect()
    }
    /// The sketch facts of a sketch page's row (`specs/0099` criteria 10, 14, 16): its state, how
    /// many clones follow it and whether it has something to draw; `null` for other kinds.
    fn sketch_row_json(&self, page: &Page) -> serde_json::Value {
        let Some(state) = self.sketch_row_state(page) else {
            return serde_json::Value::Null;
        };
        let owner = self.document.page_sketch(&page.id);
        let usage = owner.map(|id| self.sketch_usage(id)).unwrap_or_default();
        serde_json::json!({
            "state": state,
            "clones": usage.clones,
            "pages": usage.pages,
            "drawable": owner.is_some_and(|id| self.sketch_has_geometry(id)),
        })
    }
    /// Page rows and screen rectangles, formatted for the host.
    #[must_use]
    pub fn pages_json(&self) -> String {
        let active = self.active_page().id;
        let drop_target = self.row_drag_target();
        // The pages a move-only selection belongs to: semibold labels (`specs/0106` UX §2).
        let homes = self.home_page_ids();
        let pages = self
            .document_preview()
            .map_or_else(|| self.shown_pages(), |p| p.pages.clone());
        let rows:Vec<_>=pages.iter().map(|p| {
            let (x,y)=self.view().document_to_screen(p.origin);
            let chip = self.kind_chip(&p.kind);
            let sketch = self.sketch_row_json(p);
            serde_json::json!({"id":p.id,"name":p.name,"active":p.id==active,"frameOnly":self.frame_moves_alone(p),"kind":chip.kind,"kindWord":chip.word,"kindIcon":chip.icon,"kindOff":chip.off,"fixedSize":p.kind.fixed_size(),"manufacturing":p.kind.manufacturing().map(NodeId::full_id),"machine":self.machine_name_of(&p.kind),"size":curvyo_ui_core::format_size(p.size,self.display_unit()),"x":p.origin.x,"y":p.origin.y,"screenX":x,"screenY":y,"screenWidth":p.size.width.as_mm()*self.view().scale(),"screenHeight":p.size.height.as_mm()*self.view().scale(),"sketch":sketch,"dropTarget":drop_target==Some(p.id.as_str()),"home":homes.contains(&p.id)})
        }).collect();
        serde_json::to_string(&rows).unwrap_or_else(|_| "[]".to_owned())
    }
}
