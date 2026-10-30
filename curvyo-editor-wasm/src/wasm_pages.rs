//! Thin host bridge for spatial page controls.
use crate::wasm_api::WasmSession;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
impl WasmSession {
    /// Page rows and screen rectangles as JSON.
    pub fn pages_json(&self) -> String {
        self.session.pages_json()
    }
    /// Ruler origin in global millimetres.
    pub fn active_page_x(&self) -> f64 {
        self.session.active_page().origin.x
    }
    /// Ruler origin in global millimetres.
    pub fn active_page_y(&self) -> f64 {
        self.session.active_page().origin.y
    }
    /// Activates an existing page.
    pub fn activate_page(&mut self, id: &str) -> bool {
        self.session.activate_page(id)
    }
    /// Adds a page and returns its stable identity.
    pub fn add_page(&mut self) -> String {
        self.session.add_page()
    }
    /// Renames a page.
    pub fn rename_page(&mut self, id: &str, name: &str) -> bool {
        self.session.rename_page(id, name)
    }
    /// Deletes a page frame, keeping its objects.
    pub fn delete_page(&mut self, id: &str) -> bool {
        self.session.delete_page(id)
    }
    /// Reorders a page frame in the list.
    pub fn reorder_page(&mut self, id: &str, index: usize) -> bool {
        self.session.reorder_page(id, index)
    }
    /// Sets the per-user move-frame-only switch.
    pub fn set_page_frame_only(&mut self, id: &str, on: bool) {
        self.session.set_page_frame_only(id, on);
    }
    /// Applies one page-position field in display units.
    pub fn set_page_position(&mut self, id: &str, axis: &str, text: &str) -> bool {
        self.session.set_page_position(id, axis, text)
    }
    /// Moves a frame to global millimetre coordinates.
    pub fn move_page_to(&mut self, id: &str, x: f64, y: f64) -> bool {
        self.session
            .move_page_to(id, curvyo_document_core::Point::new(x, y))
    }
}
