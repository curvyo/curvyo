//! Move versus edit's `wasm-bindgen` surface (`specs/0106-move-versus-edit` criteria 9, 11 and
//! 14): the Style tab's muted line, the move-only flag of the Select bar and the canvas notice.
//! A further `impl WasmSession` block; strings and booleans only (ADR 0001 §5).

use wasm_bindgen::prelude::*;

use crate::wasm_api::WasmSession;

#[wasm_bindgen]
impl WasmSession {
    /// "On Drawing 2. Double-click to edit." for a move-only selection, else empty.
    #[must_use]
    pub fn out_of_context_line(&self) -> String {
        self.session.out_of_context_line()
    }

    /// Whether the selection is move-only.
    #[must_use]
    pub fn selection_move_only(&self) -> bool {
        self.session.selection_move_only()
    }

    /// The refusal of an edit chord or the no-drawing-page notice, once; empty when none.
    pub fn take_context_notice(&mut self) -> String {
        self.session.take_context_notice()
    }
}
