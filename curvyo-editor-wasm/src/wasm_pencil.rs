//! The Pencil on the `wasm-bindgen` surface (`specs/0158-pen-display` slice 1, ADR 0095
//! decision 1; slice 2b criteria 2 and 3): the frame's pen samples and what the Pencil bar and the
//! Pen strip set for the next stroke. A further `impl WasmSession` block, every method a
//! pass-through to `Session`.

use curvyo_document_core::Length;
use curvyo_ui_core::{PEN_SAMPLE_STRIDE, PressureCurve};
use wasm_bindgen::prelude::*;

use crate::wasm_api::WasmSession;

/// The host's name of a pressure curve.
fn curve_name(curve: PressureCurve) -> &'static str {
    match curve {
        PressureCurve::Soft => "soft",
        PressureCurve::Linear => "linear",
        PressureCurve::Firm => "firm",
    }
}

#[wasm_bindgen]
impl WasmSession {
    /// Numbers per flat pen sample record (9); the host asserts the same layout.
    #[must_use]
    pub fn pen_sample_stride() -> usize {
        PEN_SAMPLE_STRIDE
    }

    /// One frame's pen samples, flat records in `PenSample` field order: x, y (canvas-relative
    /// CSS px), pressure (NaN none), tilt x, tilt y (NaN none), end (0 tip, 1 eraser), phase
    /// (0 hover, 1 contact, 2 out), barrel bits, time (ms). Only the Pencil reads them.
    pub fn pen_samples(&mut self, flat: &[f64]) {
        self.session.pen_samples(flat);
    }

    /// The pen window's Eraser button: on, the tip erases as the eraser end does.
    pub fn set_pencil_eraser(&mut self, on: bool) {
        self.session.set_pencil_eraser(on);
    }

    /// Whether the Eraser button is on.
    #[must_use]
    pub fn pencil_eraser(&self) -> bool {
        self.session.pencil_eraser()
    }

    /// The Pen strip's layer pick: the active page's layers, front-most first, as a JSON array of
    /// `{id, name, hidden, locked, active}`; `[]` for a page without layers.
    #[must_use]
    pub fn pen_layers_json(&self) -> String {
        serde_json::to_string(&self.session.pen_layers()).unwrap_or_else(|_| "[]".to_owned())
    }

    /// Makes the layer of the JSON id `id` the active layer (no step); false for another id.
    pub fn set_pen_layer(&mut self, id: &str) -> bool {
        self.session.set_pen_layer(id)
    }

    /// Smoothing as the fit accuracy in mm (the bar's percent / 100).
    #[must_use]
    pub fn pencil_smoothing(&self) -> f64 {
        self.session.pencil_settings().smoothing.as_mm()
    }

    /// Sets Smoothing (mm); false for a negative or not-a-number value.
    pub fn set_pencil_smoothing(&mut self, mm: f64) -> bool {
        self.session.set_pencil_smoothing(Length::from_mm(mm))
    }

    /// Whether "Simulate pressure" is on.
    #[must_use]
    pub fn pencil_simulate(&self) -> bool {
        self.session.pencil_settings().simulate
    }

    /// Turns "Simulate pressure" on or off.
    pub fn set_pencil_simulate(&mut self, on: bool) {
        self.session.set_pencil_simulate(on);
    }

    /// Min width in mm.
    #[must_use]
    pub fn pencil_min_mm(&self) -> f64 {
        self.session.pencil_settings().widths.min().as_mm()
    }

    /// Max width in mm.
    #[must_use]
    pub fn pencil_max_mm(&self) -> f64 {
        self.session.pencil_settings().widths.max().as_mm()
    }

    /// Sets Min in mm (clamped; Max is raised to it if below).
    pub fn set_pencil_min_mm(&mut self, mm: f64) {
        self.session.set_pencil_min(Length::from_mm(mm));
    }

    /// Sets Max in mm (clamped and raised to Min if below).
    pub fn set_pencil_max_mm(&mut self, mm: f64) {
        self.session.set_pencil_max(Length::from_mm(mm));
    }

    /// The pressure curve: `"soft"`, `"linear"` or `"firm"`.
    #[must_use]
    pub fn pencil_curve(&self) -> String {
        curve_name(self.session.pencil_settings().curve).to_string()
    }

    /// Sets the pressure curve by name; false for an unknown name.
    pub fn set_pencil_curve(&mut self, name: &str) -> bool {
        let curve = match name {
            "soft" => PressureCurve::Soft,
            "linear" => PressureCurve::Linear,
            "firm" => PressureCurve::Firm,
            _ => return false,
        };
        self.session.set_pencil_curve(curve);
        true
    }

    /// Whether the last finished stroke had pen pressure: 1 yes, 0 no, -1 before the first.
    #[must_use]
    pub fn pencil_last_had_pressure(&self) -> i32 {
        self.session
            .pencil_last_had_pressure()
            .map_or(-1, i32::from)
    }
}
