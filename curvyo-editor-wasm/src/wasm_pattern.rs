//! The Pattern block's `wasm-bindgen` surface (`specs/0071-pattern-fill`): its read as JSON and
//! the commands of its controls. Strings and scalars only (ADR 0001 §5); every method passes
//! straight to `Session` (`session/pattern.rs`).

use curvyo_ui_core::Grid;
use wasm_bindgen::prelude::*;

use crate::wasm_api::WasmSession;

fn grid(name: &str) -> Grid {
    Grid::from_name(name).unwrap_or(Grid::Normal)
}

/// `"committed"`, `"unchanged"` or `"invalid:<code>"`, as the other typed fields answer.
fn answer(result: Result<bool, &str>) -> String {
    match result {
        Ok(true) => "committed".to_owned(),
        Ok(false) => "unchanged".to_owned(),
        Err(code) => format!("invalid:{code}"),
    }
}

#[wasm_bindgen]
impl WasmSession {
    /// What the fill kind group and the Pattern block show, as JSON (`PatternPanelJson`).
    #[must_use]
    pub fn pattern_panel_json(&self) -> String {
        serde_json::to_string(&self.session.pattern_panel()).unwrap_or_default()
    }

    /// The fill kind group: `"none"`, `"solid"` or `"pattern"`; one commit.
    pub fn set_fill_kind(&mut self, kind: &str) -> bool {
        self.session.set_fill_kind(kind)
    }

    /// A generator tile by its id; one commit.
    pub fn set_pattern_generator(&mut self, id: &str) -> bool {
        self.session.set_pattern_generator(id)
    }

    /// Draw as `"lines"` or `"shapes"`; one commit.
    pub fn set_pattern_mode(&mut self, mode: &str) -> bool {
        self.session.set_pattern_mode(mode)
    }

    /// Whole pieces only on or off; one commit.
    pub fn set_pattern_whole_pieces(&mut self, on: bool) -> bool {
        self.session.set_pattern_whole_pieces(on)
    }

    /// Re-roll the seed; one commit.
    pub fn reroll_pattern_seed(&mut self) -> bool {
        self.session.reroll_pattern_seed()
    }

    /// Enter or Tab in the Seed field.
    pub fn set_pattern_seed_text(&mut self, text: &str) -> String {
        answer(self.session.set_pattern_seed_text(text))
    }

    /// A tick of a field drag at position `p`, rounded to `grid_name`; previewed only.
    pub fn preview_pattern_field(&mut self, name: &str, p: f64, grid_name: &str) {
        self.session.preview_pattern_field(name, p, grid(grid_name));
    }

    /// An arrow key on a field; previewed, the key-up commits.
    pub fn step_pattern_field(&mut self, name: &str, steps: i32, grid_name: &str) {
        self.session
            .step_pattern_field(name, steps, grid(grid_name));
    }

    /// The reset icon of a field; one commit.
    pub fn reset_pattern_field(&mut self, name: &str) -> bool {
        self.session.reset_pattern_field(name)
    }

    /// Enter or Tab in a field.
    pub fn set_pattern_field_text(&mut self, name: &str, text: &str) -> String {
        answer(self.session.set_pattern_field_text(name, text))
    }

    /// Convert to paths; one commit, the new groups selected.
    pub fn convert_pattern_to_paths(&mut self) -> bool {
        self.session.convert_pattern_to_paths()
    }

    /// The refusal of the last commit over the limit, once; empty when there is none.
    pub fn take_pattern_refusal(&mut self) -> String {
        self.session.take_pattern_refusal().unwrap_or_default()
    }
}
