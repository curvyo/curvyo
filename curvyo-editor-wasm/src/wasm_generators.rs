//! The Generators tool and the Generator block's `wasm-bindgen` surface
//! (`specs/0141-parametric-generators`): strings and scalars only (ADR 0001 §5); every method
//! passes straight to `Session` (`session/generators.rs`).

use wasm_bindgen::prelude::*;

use crate::wasm_api::WasmSession;

#[wasm_bindgen]
impl WasmSession {
    /// The kinds the Generators bar offers, as JSON `[{id, label, icon}]`.
    #[must_use]
    pub fn generator_kinds_json(&self) -> String {
        self.session.generator_kinds_json()
    }

    /// The kind the tool places (`<extension>/<kind>`).
    #[must_use]
    pub fn generator_kind(&self) -> String {
        self.session.generator_kind()
    }

    /// The bar's choice.
    pub fn set_generator_kind(&mut self, kind: &str) {
        self.session.set_generator_kind(kind);
    }

    /// Places the chosen kind with its centre at the document point `(x, y)`; one step.
    pub fn place_generated(&mut self, x: f64, y: f64) -> bool {
        self.session.place_generated(x, y)
    }

    /// The Generator block of the one selected generated object as JSON, or `null`.
    #[must_use]
    pub fn generated_panel_json(&self) -> String {
        self.session.generated_panel_json()
    }

    /// A value typed or stepped into a row of the block (JSON text); the answer is JSON
    /// `{written, notices}` or `{error}`.
    pub fn set_generated_value(&mut self, key: &str, value: &str) -> String {
        self.session.set_generated_value(key, value)
    }

    /// A value stepped into a row, shown without writing; the answer is JSON `{notices}` or
    /// `{error}`. The commit is `set_generated_value`.
    pub fn preview_generated_value(&mut self, key: &str, value: &str) -> String {
        self.session.preview_generated_value(key, value)
    }

    /// Drops the preview of a stepped value.
    pub fn cancel_generated_preview(&mut self) {
        self.session.cancel_generated_preview();
    }

    /// Convert to paths of the selected generated objects; one step.
    pub fn convert_selected_generated(&mut self) -> bool {
        self.session.convert_selected_generated()
    }

    /// The status of an opened file with generator objects this build cannot rebuild, once.
    pub fn take_generated_notice(&mut self) -> String {
        self.session.take_generated_notice()
    }
}
