//! Thin host bridge of laser pages (`specs/0093`, `specs/0098`): the kind list and the active
//! laser page as JSON, and the commands of its Machine block (the Fixtures block's are in
//! `wasm_fixtures.rs`).

use curvyo_document_core::Length;
use wasm_bindgen::prelude::*;

use crate::wasm_api::WasmSession;

#[wasm_bindgen]
impl WasmSession {
    /// The active page's workbench and chip, the page kind list, the extensions and manifests.
    pub fn workbenches_json(&self) -> String {
        self.session.workbenches_json()
    }

    /// The active manufacturing's values for its tab and the status bar; `null` otherwise.
    pub fn manufacturing_view_json(&self) -> String {
        self.session.manufacturing_view_json()
    }

    /// The machine list of the Add page group: every profile, the starting one chosen.
    pub fn machine_profiles_json(&self) -> String {
        self.session.machine_profiles_json()
    }

    /// Deletes the selected placement of a laser page (`specs/0098` criterion 12).
    pub fn delete_selected_placement(&mut self) -> bool {
        self.session
            .selected_placement()
            .is_some_and(|id| self.session.delete_placement(id))
    }

    /// Switches an extension on or off; returns the text of `extensions.toml` to keep.
    pub fn set_extension_enabled(&mut self, id: &str, enabled: bool) -> String {
        self.session.set_extension_enabled(id, enabled)
    }

    /// Reads the kept `extensions.toml`.
    pub fn load_extension_settings(&mut self, text: &str) {
        self.session.load_extension_settings(text);
    }
}
