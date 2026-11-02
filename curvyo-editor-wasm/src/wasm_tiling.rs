//! The `wasm-bindgen` surface of tiling (`specs/0069-tiling`): the Tile slot's state, the bar's
//! recipe and status as JSON, the circle handles' entry boxes, enter, apply, cancel and release
//! (ADR 0001 §5).

use curvyo_document_core::SpacingMode;
use curvyo_ui_core::tiling::TileButton;
use wasm_bindgen::prelude::*;

use crate::wasm_api::WasmSession;

#[wasm_bindgen]
impl WasmSession {
    /// The Tile slot of the Select bar: 0 nothing selected, 1 a tiling with other objects (both
    /// disabled), 2 "Tile", 3 "Edit tiling" with "Release tiling" (UX notes §1).
    #[must_use]
    pub fn tile_button_state(&self) -> u8 {
        match self.session.tile_button() {
            TileButton::NothingSelected => 0,
            TileButton::TilingWithOthers => 1,
            TileButton::Tile(_) => 2,
            TileButton::Edit(_) => 3,
        }
    }

    /// Opens the tiling bar; `seed` is the frontend's fresh random value for a new tiling.
    pub fn tiling_enter(&mut self, seed: u32) -> bool {
        self.session.tiling_enter(seed)
    }

    /// Whether the tiling bar is open.
    #[must_use]
    pub fn tiling_open(&self) -> bool {
        self.session.tiling_open()
    }

    /// The bar's recipe as JSON, empty while the bar is closed.
    #[must_use]
    pub fn tiling_recipe_json(&self) -> String {
        self.session.tiling_recipe_json()
    }

    /// Takes the bar's recipe; false when it is malformed or out of range.
    pub fn tiling_set_recipe_json(&mut self, json: &str) -> bool {
        self.session.tiling_set_recipe_json(json)
    }

    /// Switches Gap (false) and Pitch (true), converting the numbers.
    pub fn tiling_set_pitch(&mut self, pitch: bool) -> bool {
        self.session.tiling_set_spacing_mode(if pitch {
            SpacingMode::Pitch
        } else {
            SpacingMode::Gap
        })
    }

    /// The status as JSON (`Session::tiling_status_json`).
    #[must_use]
    pub fn tiling_status_json(&self) -> String {
        self.session.tiling_status_json()
    }

    /// Apply or Enter; true when the bar closed (written, or unchanged).
    pub fn tiling_apply(&mut self) -> bool {
        self.session.tiling_apply()
    }

    /// Cancel or Escape: nothing is written.
    pub fn tiling_cancel(&mut self) {
        self.session.tiling_cancel();
    }

    /// Opens the entry box of the circle handle `handle` (`"centre"`, `"radius"`, `"start"`,
    /// `"sweep"`); false outside a circle tiling.
    pub fn tiling_open_entry(&mut self, handle: &str) -> bool {
        crate::session::circle_handle(handle)
            .is_some_and(|handle| self.session.tiling_open_entry(handle))
    }

    /// Closes the open entry box: `restore` puts back the values it opened with (Escape, a press
    /// elsewhere, a blur), otherwise the typed values stay (Enter).
    pub fn tiling_close_entry(&mut self, restore: bool) {
        self.session.tiling_close_entry(restore);
    }

    /// The open entry box as JSON (`Session::tiling_entry_json`), empty while none is open.
    #[must_use]
    pub fn tiling_entry_json(&self) -> String {
        self.session.tiling_entry_json()
    }

    /// Release tiling: one step, the selection stays on the group.
    pub fn tiling_release(&mut self) -> bool {
        self.session.tiling_release()
    }
}
