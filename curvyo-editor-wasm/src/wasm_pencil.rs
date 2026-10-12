//! The Pencil on the `wasm-bindgen` surface (`specs/0032-pen-tablet-input`): one call per DOM
//! pointer event with all its samples, the Pencil bar's state and its three settings. A further
//! `impl WasmSession` block, every method a pass-through to `Session`. No pointer id, device id
//! or device name crosses this boundary (criterion 16).

use curvyo_document_core::Length;
use curvyo_ui_core::{PointerSample, PressureCurve};
use wasm_bindgen::prelude::*;

use crate::session::PencilPhase;
use crate::wasm_api::WasmSession;

/// Values per sample in [`WasmSession::pencil_samples`]: x, y (canvas CSS pixels), pressure,
/// tiltX, tiltY (degrees) and the event time (milliseconds).
const VALUES_PER_SAMPLE: usize = 6;

/// What the Pencil bar shows (criteria 14 and 15).
#[wasm_bindgen]
#[derive(Debug, Clone)]
pub struct PencilBarView {
    /// A pen has been seen in this session: the bar is shown.
    pub pen_seen: bool,
    /// A stroke with pressure has been drawn: Min, Max and Curve are shown.
    pub pressure_seen: bool,
    /// The last reading came from a pen: the readout is shown.
    pub has_readout: bool,
    /// The readout's pressure, 0 to 1.
    pub pressure: f64,
    /// The readout's tilt along X, degrees.
    pub tilt_x: f64,
    /// The readout's tilt along Y, degrees.
    pub tilt_y: f64,
    /// The Minimum width, mm.
    pub minimum_mm: f64,
    /// The Maximum width, mm.
    pub maximum_mm: f64,
    /// The curve: `"soft"`, `"linear"` or `"firm"`.
    #[wasm_bindgen(getter_with_clone)]
    pub curve: String,
}

fn curve_code(curve: PressureCurve) -> String {
    match curve {
        PressureCurve::Soft => "soft",
        PressureCurve::Linear => "linear",
        PressureCurve::Firm => "firm",
    }
    .to_string()
}

#[wasm_bindgen]
impl WasmSession {
    /// One pointer event of the Pencil: `phase` is `"down"`, `"move"` or `"up"`, `pointer_type`
    /// the DOM's `pointerType`, `buttons` the event's `buttons`, and `values` six numbers per
    /// sample (see `VALUES_PER_SAMPLE`), the coalesced samples of a move first and the event's
    /// own last. An unknown phase does nothing.
    pub fn pencil_samples(
        &mut self,
        phase: &str,
        pointer_type: &str,
        buttons: u32,
        values: &[f64],
    ) {
        let phase = match phase {
            "down" => PencilPhase::Down,
            "move" => PencilPhase::Move,
            "up" => PencilPhase::Up,
            _ => return,
        };
        let samples: Vec<PointerSample> = values
            .as_chunks::<VALUES_PER_SAMPLE>()
            .0
            .iter()
            .map(|chunk| {
                let point = self.session.screen_to_document(chunk[0], chunk[1]);
                PointerSample::from_values(
                    pointer_type,
                    buttons,
                    [point.x, point.y, chunk[2], chunk[3], chunk[4], chunk[5]],
                )
            })
            .collect();
        self.session.pencil_input(phase, buttons, &samples);
    }

    /// What the Pencil bar shows.
    #[must_use]
    pub fn pencil_bar(&self) -> PencilBarView {
        let bar = self.session.pencil_bar();
        let readout = bar.readout;
        PencilBarView {
            pen_seen: bar.pen_seen,
            pressure_seen: bar.pressure_seen,
            has_readout: readout.is_some(),
            pressure: readout.map_or(0.0, |r| r.pressure),
            tilt_x: readout.map_or(0.0, |r| r.tilt_x.as_radians().to_degrees()),
            tilt_y: readout.map_or(0.0, |r| r.tilt_y.as_radians().to_degrees()),
            minimum_mm: bar.settings.minimum().as_mm(),
            maximum_mm: bar.settings.maximum().as_mm(),
            curve: curve_code(bar.settings.curve),
        }
    }

    /// Sets the Minimum width in mm for the next stroke (clamped, criterion 14).
    pub fn set_pencil_minimum(&mut self, millimetres: f64) {
        self.session
            .set_pencil_minimum(Length::from_mm(millimetres));
    }

    /// Sets the Maximum width in mm for the next stroke (clamped and never below the Minimum).
    pub fn set_pencil_maximum(&mut self, millimetres: f64) {
        self.session
            .set_pencil_maximum(Length::from_mm(millimetres));
    }

    /// Sets the curve, `"soft"`, `"linear"` or `"firm"`; anything else is ignored.
    pub fn set_pencil_curve(&mut self, curve: &str) {
        let curve = match curve {
            "soft" => PressureCurve::Soft,
            "linear" => PressureCurve::Linear,
            "firm" => PressureCurve::Firm,
            _ => return,
        };
        self.session.set_pencil_curve(curve);
    }
}
