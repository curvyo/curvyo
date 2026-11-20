//! Thin host bridge of the Adjustments group and the Before and after pane
//! (`specs/0135-image-optimisation`): the group's and the pane's JSON, the edits of the selected
//! picture's adjustments and the pane's switches. A field refusal is its range text, empty when
//! taken.

use wasm_bindgen::prelude::*;

use crate::wasm_api::WasmSession;
use crate::wasm_operations::node;

#[wasm_bindgen]
impl WasmSession {
    /// The Adjustments group of the selected picture as JSON (`AdjustmentsView`), `null` when the
    /// group is absent (not exactly one image or image clone selected).
    pub fn adjustments_view_json(&self) -> String {
        self.session.adjustments_view_json()
    }

    /// A press of preset `key` (`photo`, `graphics`, `black_white`, `line_art`, `dot_raster`,
    /// `newspaper`, `expert`) in the Preset strip: one step "Edit adjustments". Whether written.
    pub fn pick_adjust_preset(&mut self, key: &str) -> bool {
        self.session.pick_adjust_preset(key)
    }

    /// A press of mode `mode` (`threshold`, `ordered`, `floyd_steinberg`, `jarvis`, `atkinson`,
    /// `stucki`, `grey`, `halftone`, `line_screen`) in the picture's Dither strip: one step "Edit
    /// adjustments", Preset and Strength kept. Whether written.
    pub fn set_picture_dither(&mut self, mode: &str) -> bool {
        self.session.set_picture_dither(mode)
    }

    /// A field commit of the picture's dither (`threshold`, `screen_size`, `screen_angle`),
    /// `value` in its shown unit: the refusal text, empty when taken or nothing changed.
    pub fn edit_picture_dither_field(&mut self, field: &str, value: f64) -> String {
        self.session
            .edit_picture_dither_field(field, value)
            .err()
            .unwrap_or_default()
    }

    /// A typed Strength or one key on the slider: the refusal text, empty when taken.
    pub fn set_adjust_strength(&mut self, value: f64) -> String {
        self.session
            .set_adjust_strength(value)
            .err()
            .unwrap_or_default()
            .to_owned()
    }

    /// A tick of a Strength drag: shown, not written.
    pub fn hold_adjust_strength(&mut self, value: f64) {
        self.session.hold_adjust_strength(value);
    }

    /// The release of a Strength drag: one step. Whether written.
    pub fn release_adjust_strength(&mut self) -> bool {
        self.session.release_adjust_strength()
    }

    /// Escape during a Strength drag.
    pub fn cancel_adjust_strength(&mut self) {
        self.session.cancel_adjust_strength();
    }

    /// A press in the add toolbar: step `kind` (`denoise`, `levels`, `local_contrast`,
    /// `contours`, `sharpen`) at its pipeline place. Whether written.
    pub fn add_adjust_step(&mut self, kind: &str) -> bool {
        self.session.add_adjust_step(kind)
    }

    /// A field commit of row `index`: `key` as the row's field lists it, `value` in its shown
    /// unit. The refusal text, empty when taken or nothing changed.
    pub fn edit_adjust_field(&mut self, index: usize, key: &str, value: f64) -> String {
        self.session
            .edit_adjust_field(index, key, value)
            .err()
            .unwrap_or_default()
            .to_owned()
    }

    /// The power toggle of row `index`. Whether written.
    pub fn set_adjust_step_enabled(&mut self, index: usize, on: bool) -> bool {
        self.session.set_adjust_step_enabled(index, on)
    }

    /// The up (`up` true) or down arrow of row `index`. Whether written.
    pub fn move_adjust_step(&mut self, index: usize, up: bool) -> bool {
        self.session.move_adjust_step(index, up)
    }

    /// The Stretch button of Levels row `index`, one step. Whether written.
    pub fn stretch_adjust_levels(&mut self, index: usize) -> bool {
        self.session.stretch_adjust_levels(index)
    }

    /// A drag of row `from` landing at index `to` inside its band, one step. Whether written.
    pub fn move_adjust_step_to(&mut self, from: usize, to: usize) -> bool {
        self.session.move_adjust_step_to(from, to)
    }

    /// The second press of row `index`'s armed trash. Whether written.
    pub fn remove_adjust_step(&mut self, index: usize) -> bool {
        self.session.remove_adjust_step(index)
    }

    /// Make editable. Whether written.
    pub fn make_adjust_editable(&mut self) -> bool {
        self.session.make_adjust_editable()
    }

    /// Restore original. Whether written.
    pub fn restore_original_adjustments(&mut self) -> bool {
        self.session.restore_original_adjustments()
    }

    /// Own on a clone's Adjustments override row: a full copy of the original's. Whether written.
    pub fn own_adjustments(&mut self) -> bool {
        self.session.own_adjustments()
    }

    /// Original's on a clone's Adjustments override row (Follow original). Whether written.
    pub fn follow_original_adjustments(&mut self) -> bool {
        self.session.follow_original_adjustments()
    }

    /// The notice of the last edit ("Numbers fixed for Image 1, 0.10 mm."), once; empty when none.
    pub fn take_adjust_notice(&mut self) -> String {
        self.session
            .untracked_mut()
            .take_adjust_notice()
            .unwrap_or_default()
    }

    /// The Before and after pane as JSON (`BeforeAfterView`).
    pub fn before_after_view_json(&self) -> String {
        self.session.before_after_view_json()
    }

    /// The header button Before and after; whether the pane is open now.
    pub fn toggle_before_after(&mut self) -> bool {
        self.session.untracked_mut().toggle_before_after()
    }

    /// Close or Escape in the pane.
    pub fn close_before_after(&mut self) {
        self.session.untracked_mut().close_before_after();
    }

    /// The View strip: `split` or `side_by_side`. Whether it changed.
    pub fn set_before_after_view(&mut self, view: &str) -> bool {
        self.session.untracked_mut().set_before_after_view(view)
    }

    /// The operation pick above the view (the operation's id as JSON text). Whether taken.
    pub fn pick_before_after_operation(&mut self, op: &str) -> bool {
        node(op).is_some_and(|op| self.session.untracked_mut().pick_before_after_operation(op))
    }

    /// The RGBA8 bytes of the pane's half `before` or `after` (sizes in the pane JSON).
    pub fn before_after_pixels(&self, half: &str) -> Vec<u8> {
        self.session.before_after_pixels(half)
    }
}
