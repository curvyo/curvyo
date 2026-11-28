//! Thin host bridge of plotter pages (`specs/0159-cutting-plotter` slice 1, ADR 0098): the Add page
//! machine list, the mat labels and the mat's commands of the Fixtures tab.

use wasm_bindgen::prelude::*;

use crate::wasm_api::WasmSession;

#[wasm_bindgen]
impl WasmSession {
    /// The machine list of Add page with Plotter.
    pub fn plotter_profiles_json(&self) -> String {
        self.session.plotter_profiles_json()
    }

    /// The mat's printed numbers in CSS px as JSON `[{"text", "x", "y", "size", "turned"}]`.
    pub fn plotter_mat_labels_json(&self) -> String {
        serde_json::to_string(&self.session.plotter_mat_labels()).unwrap_or_else(|_| "[]".into())
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
}
