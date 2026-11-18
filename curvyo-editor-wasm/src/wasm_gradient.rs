//! The gradient fill's `wasm-bindgen` surface (`specs/0124-gradient-fill`): the Gradient block
//! and the canvas handles as JSON and the commands of their controls. Strings and scalars only
//! (ADR 0001 §5); every method passes straight to `Session` (`session/gradient/`). The Fill
//! switch's Gradient item is `set_fill_kind("gradient")` (`wasm_pattern.rs`); a picker drag ends
//! with `commit_style_preview` or `cancel_style_preview`; pointer gestures and Escape, Delete and
//! the double-click go through the canvas's own pointer and key calls.

use wasm_bindgen::prelude::*;

use crate::wasm_api::WasmSession;

/// `"committed"`, `"unchanged"` or `"invalid:<code>"`, as the other typed fields answer.
fn answer(result: Result<bool, &str>) -> String {
    match result {
        Ok(true) => "committed".to_owned(),
        Ok(false) => "unchanged".to_owned(),
        Err(code) => format!("invalid:{code}"),
    }
}

/// An index from the host, which sends numbers as `f64`; a negative or fractional one names no
/// stop.
fn index(value: u32) -> usize {
    usize::try_from(value).unwrap_or(usize::MAX)
}

#[wasm_bindgen]
impl WasmSession {
    /// What the Gradient block shows, as JSON (`GradientViewJson`).
    #[must_use]
    pub fn gradient_view_json(&self) -> String {
        serde_json::to_string(&self.session.gradient_view()).unwrap_or_default()
    }

    /// What the canvas draws of the gradient handles, their readout and entry box, as JSON
    /// (`GradientHandlesJson`).
    #[must_use]
    pub fn gradient_handles_json(&self) -> String {
        serde_json::to_string(&self.session.gradient_handles()).unwrap_or_default()
    }

    /// The Kind switch: `"linear"` or `"radial"`; one step.
    pub fn set_gradient_kind(&mut self, kind: &str) -> bool {
        self.session.set_gradient_kind(kind)
    }

    /// Enter or Tab in the Angle field.
    pub fn set_gradient_angle_text(&mut self, text: &str) -> String {
        answer(self.session.set_gradient_angle_text(text))
    }

    /// Enter or Tab in the Radius field.
    pub fn set_gradient_radius_text(&mut self, text: &str) -> String {
        answer(self.session.set_gradient_radius_text(text))
    }

    /// Enter or Tab in the hex field of stop `stop`.
    pub fn set_gradient_stop_hex(&mut self, stop: u32, text: &str) -> String {
        answer(self.session.set_gradient_stop_hex(index(stop), text))
    }

    /// Enter or Tab in the Position field of stop `stop`.
    pub fn set_gradient_stop_position_text(&mut self, stop: u32, text: &str) -> String {
        answer(
            self.session
                .set_gradient_stop_position_text(index(stop), text),
        )
    }

    /// A tick of a picker drag on stop `stop`; previewed only.
    pub fn preview_gradient_stop_hsv(&mut self, stop: u32, hue: f64, saturation: f64, value: f64) {
        self.session
            .preview_gradient_stop_hsv(index(stop), hue, saturation, value);
    }

    /// Add stop; one step.
    pub fn add_gradient_stop(&mut self) -> bool {
        self.session.add_gradient_stop()
    }

    /// Remove on the row of inner stop `stop`; one step.
    pub fn remove_gradient_stop(&mut self, stop: u32) -> bool {
        self.session.remove_gradient_stop(index(stop))
    }

    /// Reverse; one step.
    pub fn reverse_gradient(&mut self) -> bool {
        self.session.reverse_gradient()
    }

    /// Focus or a press entered the row of stop `stop`: it becomes the active stop.
    pub fn set_gradient_active_stop(&mut self, stop: u32) {
        self.session.set_gradient_active_stop(index(stop));
    }

    /// Enter in the open entry box: its field and hex together, one step.
    pub fn commit_gradient_entry(&mut self, field_text: &str, hex: &str) -> String {
        answer(self.session.commit_gradient_entry(field_text, hex))
    }

    /// The canvas notice of the gradient features the last SVG Open or Import lost, once, one
    /// sentence per line (`\n`); empty when nothing was lost.
    pub fn take_gradient_import_notice(&mut self) -> String {
        self.session.take_gradient_import_notice()
    }

    /// Escape or a press outside the entry box: it closes, nothing is written.
    pub fn close_gradient_entry(&mut self) {
        self.session.close_gradient_entry();
    }
}
