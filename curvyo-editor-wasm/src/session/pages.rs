//! Session commands targeting spatial page frames.
use super::{Session, SizeOutcome};
use curvyo_document_core::{DocumentSize, NodeId, ObjectSnapshot, Page, Point};
use curvyo_ui_core::object_bounds;
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
        let Some(page) = self.document.pages().into_iter().find(|p| p.id == id) else {
            return false;
        };
        self.page_view.active = Some(id.to_owned());
        let (w, h) = self.viewport.canvas_size();
        let min = self.view().screen_to_document(0.0, 0.0);
        let max = self.view().screen_to_document(w, h);
        if !page.overlaps(min, max) {
            let centre = Point::new(
                page.origin.x + page.size.width.as_mm() / 2.0,
                page.origin.y + page.size.height.as_mm() / 2.0,
            );
            let (x, y) = self.view().document_to_screen(centre);
            self.viewport.pan_by_screen_delta(x - w / 2.0, y - h / 2.0);
        }
        true
    }
    /// Adds and activates the next frame as one history step.
    pub fn add_page(&mut self) -> String {
        let page = self.recording(|this| this.document.add_page());
        self.activate_page(&page.id);
        page.id
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
    pub(super) fn contained_page_objects(&self, page: &Page, exclusive: bool) -> Vec<NodeId> {
        let pages = self.document.pages();
        self.document_read()
            .leaves
            .iter()
            .filter(|object| {
                let (min, max) = object_bounds(object);
                page.contains(min)
                    && page.contains(max)
                    && (!exclusive
                        || !pages
                            .iter()
                            .any(|p| p.id != page.id && p.overlaps(min, max)))
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
        let objects = if self.page_view.frame_only.contains(id) {
            Vec::new()
        } else {
            self.contained_page_objects(&page, false)
        };
        self.recording(|this| {
            this.document
                .move_page(id, page.origin.vector_to(origin), &objects)
                .unwrap_or(false)
        })
    }
    pub(super) fn resize_active_page(&mut self, size: DocumentSize) -> SizeOutcome {
        let page = self.active_page();
        let objects = self.contained_page_objects(&page, true);
        match self.document.resize_page(&page.id, size, &objects) {
            Ok(true) => SizeOutcome::Committed,
            Ok(false) => SizeOutcome::Unchanged,
            Err(_) => SizeOutcome::Invalid,
        }
    }
    pub(super) fn fit_active_page(&mut self) -> super::FitOutcome {
        let page = self.active_page();
        let read = self.document_read();
        let objects: Vec<_> = read
            .leaves
            .iter()
            .filter(|o| {
                let (min, max) = curvyo_ui_core::object_outline_bounds(o);
                page.overlaps(min, max)
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
        let size = DocumentSize::from_mm((max.x - min.x).max(1.0), (max.y - min.y).max(1.0));
        let shift = curvyo_document_core::Vec2::new(
            page.origin.x - min.x + (size.width.as_mm() - (max.x - min.x)) / 2.0,
            page.origin.y - min.y + (size.height.as_mm() - (max.y - min.y)) / 2.0,
        );
        let ids: Vec<_> = objects.iter().map(|o| o.id()).collect();
        let result = self.document.batch(
            "fit_document_to_content",
            |doc| -> Result<bool, curvyo_document_core::PageError> {
                curvyo_document_core::validated_document_side(size.width)?;
                curvyo_document_core::validated_document_side(size.height)?;
                let moved = shift.x.abs() > 1e-9 || shift.y.abs() > 1e-9;
                if moved {
                    doc.translate_objects(&ids, shift)?;
                }
                Ok(doc.resize_page(&page.id, size, &[])? || moved)
            },
        );
        match result {
            Ok(true) => super::FitOutcome::Fitted,
            Ok(false) => super::FitOutcome::AlreadyFits,
            Err(_) => super::FitOutcome::TooLarge(curvyo_ui_core::content_too_large_message(
                self.display_unit(),
            )),
        }
    }
    pub(super) fn page_selection_includes(&self, object: &ObjectSnapshot) -> bool {
        let page = self.active_page();
        if self.page_view.context.as_ref() != Some(&page.id) {
            return true;
        }
        let (min, max) = object_bounds(object);
        page.overlaps(min, max)
    }
    /// Page rows and screen rectangles, formatted for the host.
    #[must_use]
    pub fn pages_json(&self) -> String {
        let active = self.active_page().id;
        let pages = self
            .document_preview()
            .map_or_else(|| self.shown_pages(), |p| p.pages.clone());
        let rows:Vec<_>=pages.iter().map(|p| {
            let (x,y)=self.view().document_to_screen(p.origin);
            serde_json::json!({"id":p.id,"name":p.name,"active":p.id==active,"frameOnly":self.page_view.frame_only.contains(&p.id),"size":curvyo_ui_core::format_size(p.size,self.display_unit()),"x":p.origin.x,"y":p.origin.y,"xText":curvyo_ui_core::format_field_length(curvyo_document_core::Length::from_mm(p.origin.x),self.display_unit()),"yText":curvyo_ui_core::format_field_length(curvyo_document_core::Length::from_mm(p.origin.y),self.display_unit()),"screenX":x,"screenY":y,"screenWidth":p.size.width.as_mm()*self.view().scale(),"screenHeight":p.size.height.as_mm()*self.view().scale()})
        }).collect();
        serde_json::to_string(&rows).unwrap_or_else(|_| "[]".to_owned())
    }
}
