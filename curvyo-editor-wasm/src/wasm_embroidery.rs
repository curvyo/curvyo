//! Thin host bridge of embroidery pages (`specs/0160-embroidery` slice 1, ADR 0098): the Add page
//! machine list and Set hoop of the Fixtures tab.

use wasm_bindgen::prelude::*;

use crate::wasm_api::WasmSession;

#[wasm_bindgen]
impl WasmSession {
    /// The machine list of Add page with Embroidery.
    pub fn embroidery_profiles_json(&self) -> String {
        self.session.embroidery_profiles_json()
    }

    /// Set hoop `key` (`150x250`, `100x100`); whether it changed.
    pub fn set_embroidery_hoop(&mut self, key: &str) -> bool {
        self.session.set_embroidery_hoop(key)
    }
}
