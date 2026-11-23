//! Print and cut's `wasm-bindgen` surface (`specs/0121-print-and-cut`): the Document tab's Add
//! registration marks, the Registration marks block, and the Print and cut group of a laser
//! page. A further `impl WasmSession` block; every method a pass-through to `Session`, which
//! holds the rules and the words.

use wasm_bindgen::prelude::*;

use crate::wasm_api::WasmSession;

#[wasm_bindgen]
impl WasmSession {
    /// Whether the Document tab shows Add registration marks (a Drawing page is active).
    #[must_use]
    pub fn registration_marks_offered(&self) -> bool {
        self.session.registration_marks_offered()
    }

    /// Add registration marks: four marks on the active Drawing page in one step, selected.
    pub fn add_registration_marks(&mut self) -> bool {
        self.session.add_registration_marks()
    }

    /// The Registration marks block as JSON (`group`, `count`, `mark`, `diameter`, `line`,
    /// `add`), `undefined` when the selection is no marks object.
    #[must_use]
    pub fn registration_marks_view(&self) -> Option<String> {
        self.session.registration_marks_view()
    }

    /// Size and Line width of the selected marks object in mm, one step; `false` when refused.
    pub fn set_registration_mark_size(&mut self, diameter_mm: f64, line_mm: f64) -> bool {
        self.session
            .set_registration_mark_size(diameter_mm, line_mm)
    }

    /// Add mark n of the selected marks object, one step.
    pub fn add_registration_mark(&mut self) -> bool {
        self.session.add_registration_mark()
    }

    /// The Print and cut group of the active laser page as JSON, `undefined` when it is absent.
    #[must_use]
    pub fn print_and_cut_view(&self) -> Option<String> {
        self.session.print_and_cut_view()
    }

    /// The Sheet picker: `id` is a sheet's `id` from the view, as JSON.
    pub fn print_and_cut_choose_sheet(&mut self, id: &str) {
        if let Some(id) = crate::wasm_trace::id_of(id) {
            self.session.print_and_cut_choose_sheet(id);
        }
    }

    /// A typed row: mark `number` at `x_mm`, `y_mm` machine millimetres, one step.
    pub fn print_and_cut_measure(&mut self, number: u8, x_mm: f64, y_mm: f64) -> bool {
        self.session.print_and_cut_measure(number, x_mm, y_mm)
    }

    /// Take head position: the connector's last work position in mm for mark `number`.
    pub fn print_and_cut_take_head_position(&mut self, number: u8, x_mm: f64, y_mm: f64) -> bool {
        self.session
            .print_and_cut_take_head_position(number, x_mm, y_mm)
    }

    /// Clear on mark `number`'s row, one step.
    pub fn print_and_cut_clear(&mut self, number: u8) -> bool {
        self.session.print_and_cut_clear(number)
    }

    /// The Tolerance field in mm, one step.
    pub fn print_and_cut_tolerance(&mut self, mm: f64) -> bool {
        self.session.print_and_cut_tolerance(mm)
    }

    /// Apply: the refusal text, or empty.
    pub fn print_and_cut_apply(&mut self) -> String {
        self.session.print_and_cut_apply()
    }

    /// The number tags of the crosses and rings and the "Fit" tag over the laser canvas, as a JSON
    /// array in screen CSS px (`kind`, `text`, `x`, `y`).
    #[must_use]
    pub fn print_and_cut_labels_json(&self) -> String {
        serde_json::to_string(&self.session.print_and_cut_labels())
            .unwrap_or_else(|_| "[]".to_owned())
    }

    /// Reset placement, one step.
    pub fn print_and_cut_reset(&mut self) -> bool {
        self.session.print_and_cut_reset()
    }
}
