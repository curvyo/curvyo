//! Thin host bridge for spatial page controls.
use crate::wasm_api::WasmSession;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
impl WasmSession {
    /// Page rows and screen rectangles as JSON.
    pub fn pages_json(&self) -> String {
        self.session.pages_json()
    }
    /// What the rulers count from: the active page's origin corner, as
    /// `"<x>,<y>,<corner>"` with the corner's global position in millimetres
    /// (`specs/0090-grid` criterion 17). The host repaints the rulers when it changes.
    pub fn ruler_origin_key(&self) -> String {
        let page = self.session.active_page();
        let zero = page.zero_point();
        format!("{},{},{}", zero.x, zero.y, page.grid.origin.as_str())
    }
    /// Activates an existing page.
    pub fn activate_page(&mut self, id: &str) -> bool {
        self.session.activate_page(id)
    }
    /// Adds a page and returns its stable identity.
    pub fn add_page(&mut self) -> String {
        self.session.add_page()
    }
    /// Add page with the Kind switch (`specs/0098` criteria 1 to 4): `drawing`, `sketch` or an
    /// output process with the machine profile `profile` (empty for the last pick). The new
    /// page's id, empty when refused.
    pub fn add_page_of_kind(&mut self, kind: &str, profile: &str) -> String {
        let profile = (!profile.is_empty()).then_some(profile);
        self.session
            .add_page_of_kind(kind, profile)
            .unwrap_or_default()
    }
    /// Ctrl+Page Down (`forward`) and Ctrl+Page Up: the next or previous page.
    pub fn activate_adjacent_page(&mut self, forward: bool) -> bool {
        self.session.activate_adjacent_page(forward)
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
    /// Begins dragging the row of page `id` out of the page list (`specs/0099` criterion 8);
    /// `false` for a row that places nothing.
    pub fn row_drag_begin(&mut self, id: &str) -> bool {
        self.session.row_drag_begin(id)
    }
    /// One pointer move of the row drag at canvas position `x`, `y` (CSS px), or off the canvas
    /// (`on_canvas` false). JSON `{badge, target, faint}`; `badge` is `none`, `clone`,
    /// `not_on_sketch_page`, `no_sketch`, `placed_page` or `only_on_laser_page`.
    pub fn row_drag_move(&mut self, x: f64, y: f64, on_canvas: bool) -> String {
        let point = on_canvas.then(|| self.session.screen_to_document(x, y));
        serde_json::to_string(&self.session.row_drag_move(point)).unwrap_or_default()
    }
    /// Ends the row drag: places the clone where the last move allowed it. Whether it did.
    pub fn row_drag_drop(&mut self) -> bool {
        self.session.row_drag_drop()
    }
    /// Cancels the row drag; nothing is written.
    pub fn row_drag_cancel(&mut self) {
        self.session.row_drag_cancel();
    }
    /// Place clone on (`specs/0099` criterion 11): a clone of the active sketch at the centre of
    /// page `id`, which becomes active.
    pub fn place_sketch_clone_on(&mut self, id: &str) -> bool {
        self.session.place_sketch_clone_on(id)
    }
    /// The answer of the Select move in flight over the canvas (`specs/0100` criteria 1 to 5):
    /// JSON `{badge, target, faint}` with `badge` `none`, `clone` (Linked clone), `move` or
    /// `copy`; an empty string while no move is in flight.
    pub fn clone_drop_answer(&self) -> String {
        self.session
            .clone_drop_answer()
            .and_then(|answer| serde_json::to_string(&answer).ok())
            .unwrap_or_default()
    }
    /// The notice of the last cloning drop (guides left out, depth refusal), once; empty if
    /// none.
    pub fn take_clone_drop_notice(&mut self) -> String {
        self.session.take_clone_drop_notice()
    }
    /// Place page (`specs/0100` criterion 15): places drawing page `id` at the centre of the
    /// active laser page. Whether it did.
    pub fn place_page_on_active(&mut self, id: &str) -> bool {
        self.session.place_page_on_active(id)
    }
    /// The laser pages Place on… offers (`specs/0100` criterion 25): JSON
    /// `[{id, name, width_mm, height_mm}]`; empty hides the button.
    pub fn place_on_targets(&self) -> String {
        serde_json::to_string(&self.session.place_on_targets()).unwrap_or_default()
    }
    /// Place on… laser page `id` (criterion 25). Whether clones were placed.
    pub fn place_on(&mut self, id: &str) -> bool {
        self.session.place_on(id)
    }
    /// The selected placed page's Style block (`specs/0100` criterion 17): "Placed page of
    /// <name>." with the name cut at 40 characters; empty when none is selected.
    pub fn placed_page_block(&self) -> String {
        self.session.placed_page_block()
    }
    /// The name tags of the placed pages (`specs/0100` UX notes §5): JSON `[{id, name, x, y,
    /// width, selected}]` in canvas CSS px.
    pub fn placed_pages_json(&self) -> String {
        self.session.placed_pages_json()
    }
}
