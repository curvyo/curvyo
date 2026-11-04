//! Thin host bridge of the manufacturing workbenches (`specs/0093-laser-workbench-shell`): the
//! registry and the active manufacturing as JSON, and the commands of the Laser tab.

use curvyo_document_core::Length;
use wasm_bindgen::prelude::*;

use crate::wasm_api::WasmSession;

#[wasm_bindgen]
impl WasmSession {
    /// The switcher's entries, Add manufacturing choices, extension list and manifests.
    pub fn workbenches_json(&self) -> String {
        self.session.workbenches_json()
    }

    /// The active manufacturing's values for its tab and the status bar; `null` otherwise.
    pub fn manufacturing_view_json(&self) -> String {
        self.session.manufacturing_view_json()
    }

    /// Add manufacturing: the new manufacturing's id, or an empty text when refused.
    pub fn add_manufacturing(&mut self, process: &str) -> String {
        self.session
            .add_manufacturing(process)
            .map(curvyo_document_core::NodeId::full_id)
            .unwrap_or_default()
    }

    /// Deletes the manufacturing named by its full id.
    pub fn delete_manufacturing(&mut self, id: &str) -> bool {
        self.session
            .manufacturing_named(id)
            .is_some_and(|id| self.session.delete_manufacturing(id))
    }

    /// Sets the active work area in mm; returns the notice of a side set into its range, empty
    /// when there is none.
    pub fn set_work_area(&mut self, width_mm: f64, height_mm: f64) -> String {
        self.session
            .set_work_area_limited(Length::from_mm(width_mm), Length::from_mm(height_mm))
            .1
            .unwrap_or_default()
    }

    /// Installs a fixture of `kind` in the active setup.
    pub fn add_fixture(&mut self, kind: &str) -> bool {
        self.session.add_fixture(kind).is_some()
    }

    /// Removes the fixture named by its full id.
    pub fn delete_fixture(&mut self, id: &str) -> bool {
        self.fixture(id)
            .is_some_and(|id| self.session.delete_fixture(id))
    }

    /// Hides or shows the fixture named by its full id.
    pub fn set_fixture_visible(&mut self, id: &str, visible: bool) -> bool {
        self.fixture(id)
            .is_some_and(|id| self.session.set_fixture_visible(id, visible))
    }

    /// Sets a length parameter of the fixture named by its full id, in mm.
    pub fn set_fixture_length(&mut self, id: &str, key: &str, mm: f64) -> bool {
        self.fixture(id).is_some_and(|id| {
            self.session
                .set_fixture_length(id, key, Length::from_mm(mm))
        })
    }

    /// Sets the role (`cut`, `engrave`, `ignore`; anything else clears it) of the Layers block row
    /// `key` (a layer's `peer:counter` or `outside`) in the active manufacturing.
    pub fn set_layer_role(&mut self, key: &str, role: &str) -> bool {
        self.session
            .set_layer_role(key, curvyo_laser_core::Role::parse(role))
    }

    /// Switches Role view of the output page on or off (view state).
    pub fn set_role_view(&mut self, on: bool) {
        self.session.set_role_view(on);
    }

    /// Fit page of the View card.
    pub fn fit_output_page(&mut self) {
        self.session.fit_output_page();
    }

    /// Switches an extension on or off; returns the text of `extensions.toml` to keep.
    pub fn set_extension_enabled(&mut self, id: &str, enabled: bool) -> String {
        self.session.set_extension_enabled(id, enabled)
    }

    /// Reads the kept `extensions.toml`.
    pub fn load_extension_settings(&mut self, text: &str) {
        self.session.load_extension_settings(text);
    }

    fn fixture(&self, id: &str) -> Option<curvyo_document_core::NodeId> {
        self.session.fixture_named(id)
    }
}
