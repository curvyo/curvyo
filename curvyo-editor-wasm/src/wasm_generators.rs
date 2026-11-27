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

    /// The Symbology group's press: switches the selected code to the kind `kind` in one step
    /// (`0144` criterion 20, ADR 0092 §2); `false` when nothing was switched.
    pub fn switch_generated_kind(&mut self, kind: &str) -> bool {
        self.session.switch_generated_kind(kind)
    }

    /// Detach part `key` of the selected rounded box: it gets a copy of the box's operations of its
    /// own (`0147` criterion 15); one step "Detach part". `false` when nothing changes.
    pub fn detach_generated_part(&mut self, key: &str) -> bool {
        self.session.detach_selected_part(key)
    }

    /// Attach part `key` of the selected rounded box: it runs with the box's operations again; one
    /// step "Attach part". `false` when it was not detached.
    pub fn attach_generated_part(&mut self, key: &str) -> bool {
        self.session.attach_selected_part(key)
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
    /// `only_this` (Shift at the gesture's start on a linked value) changes it alone.
    pub fn preview_sketch_instance_value(
        &mut self,
        key: &str,
        number: f64,
        only_this: bool,
    ) -> String {
        self.session
            .preview_sketch_instance_value(key, number, only_this)
    }

    /// Drops the preview of a stepped field.
    pub fn cancel_sketch_instance_preview(&mut self) {
        self.session.cancel_sketch_instance_preview();
    }

    /// A value typed or stepped into the field `key`; JSON `{written, notices}` or `{error}`.
    /// `only_this` as for `preview_sketch_instance_value`; a typed value passes `false`.
    pub fn set_sketch_instance_value(&mut self, key: &str, number: f64, only_this: bool) -> String {
        self.session
            .set_sketch_instance_value(key, number, only_this)
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

    /// Links the exposed values `a` and `b`; empty on success, else the refusal sentence.
    pub fn sketch_link(&mut self, a: &str, b: &str) -> String {
        self.session.sketch_link(a, b)
    }

    /// Takes the exposed `key` out of its link group; empty on success, else the refusal.
    pub fn sketch_unlink(&mut self, key: &str) -> String {
        self.session.sketch_unlink(key)
    }

    /// The open entry chip of a sketch-clone knob as JSON, empty while none is open.
    #[must_use]
    pub fn sketch_knob_entry_json(&self) -> String {
        self.session.sketch_knob_entry_json()
    }

    /// Enter in a knob's entry chip; JSON `{written, notices}` or `{error}`.
    pub fn commit_sketch_knob_entry(&mut self, text: &str) -> String {
        self.session.commit_sketch_knob_entry(text)
    }

    /// Escape or a blur in a knob's entry chip: nothing is written.
    pub fn cancel_sketch_knob_entry(&mut self) {
        self.session.cancel_sketch_knob_entry();
    }

    /// Whether a sketch-clone knob is held (the Parameters row follows each move).
    #[must_use]
    pub fn sketch_knob_dragging(&self) -> bool {
        self.session.sketch_knob_dragging()
    }

    /// The readout chip's glyph during a sketch-clone knob drag: `link`, `unlink` or empty.
    #[must_use]
    pub fn sketch_knob_chip_glyph(&self) -> String {
        self.session.sketch_knob_chip_glyph().to_owned()
    }
}
