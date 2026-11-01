//! Thin host bridge for the Measure tool and the Vector dimensions' overlay
//! (`specs/0070-measure-and-dimension`).
use crate::wasm_api::WasmSession;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
impl WasmSession {
    /// The overlay of the last frame drawn, as JSON: the dimension texts with their screen
    /// positions, the Measure tool's mode label and its readout chip.
    #[must_use]
    pub fn measure_view_json(&self) -> String {
        self.session.measure_view_json()
    }
}
