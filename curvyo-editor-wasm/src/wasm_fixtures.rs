//! Thin host bridge of the laser fixtures of the active laser page (`specs/0102-laser-fixtures`):
//! the Fixtures block's view and commands, the canvas labels, the fit cue and the snap switch.
//! Ids are the full id texts the view JSON carries.

use curvyo_document_core::{Length, NodeId};
use wasm_bindgen::prelude::*;

use crate::wasm_api::WasmSession;

#[wasm_bindgen]
impl WasmSession {
    /// The Fixtures block of the active laser page as JSON; `null` on another page.
    pub fn fixtures_view_json(&self) -> String {
        self.session.fixtures_view_json()
    }

    /// The tape numbers and names over the canvas as a JSON array, in screen CSS px.
    pub fn fixture_labels_json(&self) -> String {
        serde_json::to_string(&self.session.fixture_labels()).unwrap_or_else(|_| "[]".to_owned())
    }

    /// The fit cue of the move in flight as JSON (`{slot, name, turned}`); `null` without one.
    pub fn fixture_fit_cue_json(&self) -> String {
        serde_json::to_string(&self.session.fixture_fit_cue()).unwrap_or_else(|_| "null".to_owned())
    }

    /// One of the five add buttons: `grid`, `holder`, `clamp`, `stop`, `screw` (criterion 2).
    pub fn add_fixture(&mut self, kind: &str) -> bool {
        self.session.add_fixture(kind).is_some()
    }

    /// Selects the fixture `id` (its row in the Fixtures block or Layers, criterion 10).
    pub fn select_fixture(&mut self, id: &str) -> bool {
        self.part(id)
            .is_some_and(|id| self.session.select_fixture(id))
    }

    /// Selects the slot `id` in its holder's row.
    pub fn select_slot(&mut self, id: &str) -> bool {
        self.part(id).is_some_and(|id| self.session.select_slot(id))
    }

    /// Renames the fixture `id` (criterion 3).
    pub fn rename_fixture(&mut self, id: &str, name: &str) -> bool {
        self.part(id)
            .is_some_and(|id| self.session.rename_fixture(id, name))
    }

    /// Removes the fixture `id` with its slots: the second press of its armed trash (criterion 4).
    pub fn delete_fixture(&mut self, id: &str) -> bool {
        self.part(id)
            .is_some_and(|id| self.session.delete_fixture(id))
    }

    /// The Active check box of the fixture `id` (criterion 5).
    pub fn set_fixture_active(&mut self, id: &str, active: bool) -> bool {
        self.part(id)
            .is_some_and(|id| self.session.set_fixture_active(id, active))
    }

    /// The eye of the fixture `id` (criterion 6).
    pub fn set_fixture_visible(&mut self, id: &str, visible: bool) -> bool {
        self.part(id)
            .is_some_and(|id| self.session.set_fixture_visible(id, visible))
    }

    /// A length field of the grid `id` in mm: `spacing_mm`, `origin_x_mm`, `origin_y_mm`
    /// (criterion 15). Holder, clamp, stop and screw are sized on the canvas.
    pub fn set_fixture_length(&mut self, id: &str, key: &str, mm: f64) -> bool {
        self.part(id).is_some_and(|id| {
            self.session
                .set_fixture_length(id, key, Length::from_mm(mm))
        })
    }

    /// The grid's Major every (criterion 15).
    pub fn set_grid_major_every(&mut self, id: &str, count: f64) -> bool {
        self.part(id)
            .is_some_and(|id| self.session.set_grid_major_every(id, count))
    }

    /// The grid's tape on `edge`: `bottom`, `left`, `top` or `right` (criterion 15).
    pub fn set_grid_tape(&mut self, id: &str, edge: &str, on: bool) -> bool {
        self.part(id)
            .is_some_and(|id| self.session.set_grid_tape(id, edge, on))
    }

    /// Add slot on the holder `id` (criterion 19); the new slot's id, `None` when none was added.
    pub fn add_slot(&mut self, id: &str) -> Option<String> {
        let id = self.part(id)?;
        self.session.add_slot(id).map(NodeId::full_id)
    }

    /// Removes the slot `id` (criterion 19).
    pub fn remove_slot(&mut self, id: &str) -> bool {
        self.part(id).is_some_and(|id| self.session.remove_slot(id))
    }

    /// Renames the slot `id`.
    pub fn rename_slot(&mut self, id: &str, name: &str) -> bool {
        self.part(id)
            .is_some_and(|id| self.session.rename_slot(id, name))
    }

    /// The slot's Shape: `rect` or `circle`.
    pub fn set_slot_shape(&mut self, id: &str, shape: &str) -> bool {
        self.part(id)
            .is_some_and(|id| self.session.set_slot_shape(id, shape))
    }

    /// A slot field in mm: `cx_mm`, `cy_mm`, `w_mm`, `h_mm`, `d_mm` (criteria 19, 20).
    pub fn set_slot_length(&mut self, id: &str, key: &str, mm: f64) -> bool {
        self.part(id)
            .is_some_and(|id| self.session.set_slot_length(id, key, Length::from_mm(mm)))
    }

    /// The Fixtures snap target switch (criterion 24): one step.
    pub fn set_snapping_fixtures(&mut self, on: bool) -> bool {
        self.session.set_snapping_fixtures(on)
    }

    /// The notice of the last fixture command, once; empty when there is none.
    pub fn take_fixture_notice(&mut self) -> String {
        self.session.take_fixture_notice().unwrap_or_default()
    }

    /// Whether the pointer is coarse: the fixture outline hit grows to 12 px.
    pub fn set_coarse_pointer(&mut self, coarse: bool) {
        self.session.set_coarse_pointer(coarse);
    }

    fn part(&self, id: &str) -> Option<NodeId> {
        self.session.fixture_part_named(id)
    }
}
