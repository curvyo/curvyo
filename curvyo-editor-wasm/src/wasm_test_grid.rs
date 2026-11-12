//! Thin host bridge of the test grids (`specs/0028-material-test-library` criteria 1 to 14): the
//! add button, the Grid block's fields, Rebuild, the result table and its view; strings and
//! scalars only (ADR 0001 §5). A command's text result is its chip or refusal, empty when taken.

use curvyo_document_core::NodeId;
use wasm_bindgen::prelude::*;

use crate::wasm_api::WasmSession;

/// The node id a host sends back (its JSON text), `None` for anything else.
fn node(text: &str) -> Option<NodeId> {
    serde_json::from_str(text).ok()
}

#[wasm_bindgen]
impl WasmSession {
    /// Add test grid: `{"ok": true, "row": <id>}` or `{"ok": false, "text"}` with the refusal
    /// shown under the button.
    pub fn press_add_test_grid(&mut self) -> String {
        let result = match self.session.press_add_test_grid() {
            Ok(row) => serde_json::json!({"ok": true, "row": row}),
            Err(text) => serde_json::json!({"ok": false, "text": text}),
        };
        result.to_string()
    }

    /// Whether the Select bar hides Flip and Guide for a whole test grid (UX notes §6.5; Object to
    /// path and Linked clone are hidden by their own flags).
    #[must_use]
    pub fn select_bar_hides_grid_commands(&self) -> bool {
        self.session.select_bar_hides_grid_commands()
    }

    /// The grid row pressed in the list (its id text; empty: none).
    pub fn select_test_grid_row(&mut self, row: &str) {
        self.session.select_test_grid_row(node(row));
    }

    /// The Grid block's field `field` (`columns`, `rows`, `power_from`, `power_to`,
    /// `speed_from`, `speed_to`, `cell`, `gap`, `label_height`, `label_power`, `label_speed`,
    /// `label_passes`) typed as `typed`. The chip, empty when taken.
    pub fn set_test_grid_field(&mut self, field: &str, typed: &str) -> String {
        self.session
            .set_test_grid_field(field, typed)
            .err()
            .unwrap_or_default()
    }

    /// The Kind radiogroup (`cut`, `score`, `engrave`). The bed cause, empty when taken.
    pub fn set_test_grid_kind(&mut self, kind: &str) -> String {
        self.session
            .set_test_grid_kind(kind)
            .err()
            .unwrap_or_default()
    }

    /// The Labels switch. The cause, empty when taken.
    pub fn set_test_grid_labels(&mut self, on: bool) -> String {
        self.session
            .set_test_grid_labels(on)
            .err()
            .unwrap_or_default()
    }

    /// The label kind (`engrave`, `score`). The refusal, empty when taken.
    pub fn set_test_grid_label_kind(&mut self, kind: &str) -> String {
        self.session
            .set_test_grid_label_kind(kind)
            .err()
            .unwrap_or_default()
    }

    /// Rebuild of a hand-edited grid. Whether it rebuilt.
    pub fn rebuild_test_grid(&mut self) -> bool {
        self.session.rebuild_test_grid()
    }

    /// A press on the result table's cell: mark, or unmark the marked one.
    pub fn press_test_cell(&mut self, column: u8, row: u8) -> bool {
        self.session.press_test_cell(column, row)
    }

    /// The Grid block's "Select": the grid becomes the canvas selection.
    pub fn select_test_grid_on_canvas(&mut self) -> bool {
        self.session.select_focused_test_grid()
    }

    /// The Grid block as JSON, `null` when none shows.
    pub fn test_grid_view_json(&self) -> String {
        self.session.test_grid_view_json()
    }

    /// The refusal of the last Add test grid, `None` when it added one.
    pub fn test_grid_notice(&self) -> Option<String> {
        self.session.test_grid_notice()
    }
}
