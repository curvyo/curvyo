//! Thin host bridge of plotter pages (`specs/0159-cutting-plotter` slice 1): the page's view as
//! JSON, the mat labels, and the commands of its machine, mat and operations.

use wasm_bindgen::prelude::*;

use crate::wasm_api::WasmSession;
use crate::wasm_operations::node;

/// The first 16 bytes of `random`, zero-padded: the caller passes `crypto.getRandomValues`.
fn sixteen(random: &[u8]) -> [u8; 16] {
    let mut bytes = [0_u8; 16];
    for (slot, byte) in bytes.iter_mut().zip(random) {
        *slot = *byte;
    }
    bytes
}

/// `Ok(true)`/`Ok(false)` as empty text, an error as its message.
fn message(result: Result<bool, String>) -> String {
    result.err().unwrap_or_default()
}

#[wasm_bindgen]
impl WasmSession {
    /// The active plotter page's view, `null` on another page.
    pub fn plotter_view_json(&self) -> String {
        self.session.plotter_view_json()
    }

    /// The machine list of Add page with Plotter.
    pub fn plotter_profiles_json(&self) -> String {
        self.session.plotter_profiles_json()
    }

    /// The mat's printed numbers in CSS px as JSON `[{"text", "x", "y", "size", "turned"}]`.
    pub fn plotter_mat_labels_json(&self) -> String {
        serde_json::to_string(&self.session.plotter_mat_labels()).unwrap_or_else(|_| "[]".into())
    }

    /// Pick profile `id` on the active plotter page; whether it changed.
    pub fn pick_plotter_profile(&mut self, id: &str) -> bool {
        self.session.pick_plotter_profile(id)
    }

    /// Save as profile; the new id, empty when nothing was saved.
    pub fn save_plotter_profile(&mut self, random: &[u8], now_ms: f64) -> String {
        #[allow(clippy::cast_possible_truncation)] // a Unix time in milliseconds
        let now = now_ms as i64;
        self.session
            .save_plotter_profile(sixteen(random), now)
            .unwrap_or_default()
    }

    /// Sets `default_pressure`, `default_speed` or `max_speed`; the field message, empty when
    /// it read.
    pub fn set_plotter_default(&mut self, field: &str, text: &str) -> String {
        message(self.session.set_plotter_default(field, text))
    }

    /// Set mat `size` (`12x12`, `12x24`); whether it changed.
    pub fn set_plotter_mat(&mut self, size: &str) -> bool {
        self.session.set_plotter_mat(size)
    }

    /// Set grip `grip` (`light`, `standard`, `strong`, `fabric`); whether it changed.
    pub fn set_plotter_grip(&mut self, grip: &str) -> bool {
        self.session.set_plotter_grip(grip)
    }

    /// The Mat grid switch; whether it changed.
    pub fn set_plotter_mat_grid(&mut self, on: bool) -> bool {
        self.session.set_plotter_mat_grid(on)
    }

    /// A press on Cut or Draw; the operation written to, empty when nothing changed.
    pub fn press_plotter_kind(&mut self, kind: &str, shift: bool) -> String {
        self.session
            .press_plotter_kind(kind, shift)
            .map(|id| id.full_id())
            .unwrap_or_default()
    }

    /// Takes the selection out of operation `id`; whether it changed.
    pub fn remove_selection_from_plotter_operation(&mut self, id: &str) -> bool {
        node(id).is_some_and(|id| self.session.remove_selection_from_plotter_operation(id))
    }

    /// Deletes plotter operation `id`; whether it was deleted.
    pub fn delete_plotter_operation(&mut self, id: &str) -> bool {
        node(id).is_some_and(|id| self.session.delete_plotter_operation(id))
    }

    /// Sets `pressure`, `speed` or `passes` of operation `id`; the field message, empty when it
    /// read.
    pub fn edit_plotter_setting(&mut self, id: &str, field: &str, text: &str) -> String {
        match node(id) {
            Some(id) => message(self.session.edit_plotter_setting(id, field, text)),
            None => String::new(),
        }
    }
}
