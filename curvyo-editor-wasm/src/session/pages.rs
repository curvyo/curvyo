//! Session commands targeting spatial page frames.
use super::{Session, SizeOutcome};
use curvyo_document_core::{DocumentSize, NodeId, ObjectSnapshot, Page, PageKind, Point};
use curvyo_ui_core::{PageMembership, Reach, object_bounds};
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
        // One decode of the frame list per call: with hundreds of frames it dominates the cost.
        let pages = self.document.pages();
        let Some(page) = pages.iter().find(|p| p.id == id).cloned() else {
            return false;
        };
        let changed = self.page_view.active(&pages).is_none_or(|p| p.id != id);
        if changed {
            // A page switch cancels the open tiling bar (`0069` criterion 35).
            self.tiling = None;
        }
        self.page_view.active = Some(id.to_owned());
        let left = self.sketch.workbench;
        // The tools and tabs follow the page's kind in the same frame (`specs/0098` criterion 8).
        self.sync_workbench();
        self.sync_sketch_page();
        if page.kind != PageKind::Sketch {
            self.last_work_page = Some(page.id.clone());
            // A page's sketch is selected only on its sketch page (ADR 0043 §2).
            self.drop_fixed_sketches_from_selection();
        }
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
    /// Adds and activates the next drawing page as one history step.
    pub fn add_page(&mut self) -> String {
        let page = self.recording(|this| this.document.add_page());
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
        let page = self.recording(|this| this.document.add_page_of(&kind).ok())?;
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
        // A drawing object over a sketch page is pressed, not the page (`specs/0099` criterion 2,
        // `adrs.md` note 3); the page's sketch is never pickable.
        if page.kind == PageKind::Sketch
            && current != super::Workbench::Sketch
            && self.press_hits_object(point)
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
        let pages: Vec<Page> = self
            .document
            .pages()
            .into_iter()
            .filter(|p| p.kind != PageKind::Sketch)
            .collect();
        self.page_view.press(&pages, point);
    }
    /// Whether a press at `point` hits a pickable object of the context.
    pub(super) fn press_hits_object(&self, point: Point) -> bool {
        !curvyo_ui_core::hit_test_objects(&self.pickable_objects(), point, self.object_tolerance())
            .is_empty()
    }
    /// Renames a frame, returning false for invalid or unchanged names.
    pub fn rename_page(&mut self, id: &str, name: &str) -> bool {
        self.recording(|this| this.document.rename_page(id, name).unwrap_or(false))
    }
    /// Deletes a frame, choosing the adjacent row if it was active.
    pub fn delete_page(&mut self, id: &str) -> bool {
        let pages = self.document.pages();
        let Some(index) = pages.iter().position(|p| p.id == id) else {
            return false;
        };
        let active = self.active_page().id == id;
        let changed = self.recording(|this| this.document.delete_page(id).is_ok());
        if changed && active {
            let pages = self.document.pages();
            self.page_view.active = Some(pages[index.min(pages.len() - 1)].id.clone());
        }
        self.sync_workbench();
        changed
    }
    /// Reorders a frame without changing its position.
    pub fn reorder_page(&mut self, id: &str, index: usize) -> bool {
        self.recording(|this| this.document.reorder_page(id, index).unwrap_or(false))
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
    /// Applies a global position field in the document display unit.
    pub fn set_page_position(&mut self, id: &str, axis: &str, text: &str) -> bool {
        let Ok(value) = text.trim().parse::<f64>() else {
            return false;
        };
        if !value.is_finite() {
            return false;
        }
        let Some(page) = self.document.pages().into_iter().find(|p| p.id == id) else {
            return false;
        };
        let value = match self.display_unit() {
            curvyo_document_core::DisplayUnit::Mm => value,
            curvyo_document_core::DisplayUnit::Cm => value * 10.0,
            curvyo_document_core::DisplayUnit::In => value * 25.4,
        };
        let origin = match axis {
            "x" => Point::new(value, page.origin.y),
            "y" => Point::new(page.origin.x, value),
            _ => return false,
        };
        self.move_page_to(id, origin)
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
        self.recording(|this| {
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
    /// The objects of `objects` Select all takes: all of them, or the members of the active page
    /// while a page selection context is on.
    pub(super) fn page_selection_ids(&self, objects: &[ObjectSnapshot]) -> Vec<NodeId> {
        let page = self.active_page();
        let scoped = self.page_view.context.as_ref() == Some(&page.id);
        let membership = scoped.then(|| self.page_membership());
        objects
            .iter()
            .filter(|object| {
                membership.as_ref().is_none_or(|m| {
                    m.is_member(&page, object.id(), object_bounds(object), Reach::Touches)
                })
            })
            .map(ObjectSnapshot::id)
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
        let pages = self
            .document_preview()
            .map_or_else(|| self.shown_pages(), |p| p.pages.clone());
        let rows:Vec<_>=pages.iter().map(|p| {
            let (x,y)=self.view().document_to_screen(p.origin);
            let chip = self.kind_chip(&p.kind);
            let sketch = self.sketch_row_json(p);
            serde_json::json!({"id":p.id,"name":p.name,"active":p.id==active,"frameOnly":self.frame_moves_alone(p),"kind":chip.kind,"kindWord":chip.word,"kindIcon":chip.icon,"kindOff":chip.off,"fixedSize":p.kind.fixed_size(),"manufacturing":p.kind.manufacturing().map(NodeId::full_id),"machine":self.machine_name_of(&p.kind),"size":curvyo_ui_core::format_size(p.size,self.display_unit()),"x":p.origin.x,"y":p.origin.y,"xText":curvyo_ui_core::format_field_length(curvyo_document_core::Length::from_mm(p.origin.x),self.display_unit()),"yText":curvyo_ui_core::format_field_length(curvyo_document_core::Length::from_mm(p.origin.y),self.display_unit()),"screenX":x,"screenY":y,"screenWidth":p.size.width.as_mm()*self.view().scale(),"screenHeight":p.size.height.as_mm()*self.view().scale(),"sketch":sketch,"dropTarget":drop_target==Some(p.id.as_str())})
        }).collect();
        serde_json::to_string(&rows).unwrap_or_else(|_| "[]".to_owned())
    }
}
