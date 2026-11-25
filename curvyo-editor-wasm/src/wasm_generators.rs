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

    /// Whether the Select bar shows Box from path: one closed outline is selected.
    #[must_use]
    pub fn box_from_path_shown(&self) -> bool {
        self.session.box_from_path_shown()
    }

    /// Box from path on the selected outline; empty on success, else the refusal sentence.
    pub fn apply_box_from_path(&mut self) -> String {
        self.session.apply_box_from_path()
    }

    /// Replace outline: arms the pick of a new outline for the selected path box.
    pub fn begin_replace_outline(&mut self) -> bool {
        self.session.begin_replace_outline()
    }

    /// The status of an opened file with generator objects this build cannot rebuild, once.
    pub fn take_generated_notice(&mut self) -> String {
        self.session.take_generated_notice()
    }
}

/// The Parameters override of clones of a sketch and Expose (`specs/0099-sketch-pages` criteria 23
/// and 24): strings, numbers and flags only; every method passes straight to `Session`
/// (`session/sketch_instance.rs`).
#[wasm_bindgen]
impl WasmSession {
    /// The Parameters row of the selected clones as JSON, or `null`.
    #[must_use]
    pub fn sketch_instance_json(&self) -> String {
        self.session.sketch_instance_json()
    }

    /// A value stepped into the field `key`, shown without writing; JSON `{notices, no_solution}`.
    pub fn preview_sketch_instance_value(&mut self, key: &str, number: f64) -> String {
        self.session.preview_sketch_instance_value(key, number)
    }

    /// Drops the preview of a stepped field.
    pub fn cancel_sketch_instance_preview(&mut self) {
        self.session.cancel_sketch_instance_preview();
    }

    /// A value typed or stepped into the field `key`; JSON `{written, notices}` or `{error}`.
    pub fn set_sketch_instance_value(&mut self, key: &str, number: f64) -> String {
        self.session.set_sketch_instance_value(key, number)
    }

    /// Back to the sketch's value of `key`, or Original's for an empty `key`; one step.
    pub fn reset_sketch_instance(&mut self, key: &str) -> bool {
        self.session.reset_sketch_instance(key)
    }

    /// Own: every value shown becomes the clones' own; one step.
    pub fn own_sketch_instance(&mut self) -> bool {
        self.session.own_sketch_instance()
    }

    /// Exposes `key` of the active sketch; empty on success, else the refusal sentence.
    pub fn sketch_expose(&mut self, key: &str, name: &str) -> String {
        self.session.sketch_expose(key, name)
    }

    /// Stops exposing `key`; empty on success, else the refusal sentence.
    pub fn sketch_unexpose(&mut self, key: &str) -> String {
        self.session.sketch_unexpose(key)
    }

    /// Limits and name of the exposed `key`; empty on success, else the refusal sentence.
    pub fn sketch_exposed_limits(&mut self, key: &str, min: &str, max: &str, name: &str) -> String {
        self.session.sketch_exposed_limits(key, min, max, name)
    }
}
