//! Undo and redo's `wasm-bindgen` surface (`specs/0020-undo-redo`): a `impl WasmSession` block that
//! passes straight through to `Session`. Strings and scalars only (ADR 0001 §5).

use curvyo_document_core::Direction;
use wasm_bindgen::prelude::*;

use crate::wasm_api::WasmSession;

fn direction_from(name: &str) -> Direction {
    if name == "redo" {
        Direction::Redo
    } else {
        Direction::Undo
    }
}

#[wasm_bindgen]
impl WasmSession {
    /// Takes the maker's latest step back (`direction` `"undo"`) or applies the latest undone step
    /// again (`"redo"`) and returns the one-line notice. Also the Undo and Redo buttons' action.
    pub fn history_press(&mut self, direction: &str) -> String {
        match direction_from(direction) {
            Direction::Undo => self.session.undo().notice,
            Direction::Redo => self.session.redo().notice,
        }
    }

    /// The notice of the last history key press, once (empty when there is none).
    pub fn take_history_notice(&mut self) -> String {
        self.session.take_history_notice().unwrap_or_default()
    }

    /// Whether the press would take long enough to show the long-operation state first: the step
    /// is large, or the history has not been read since the file was opened.
    pub fn history_needs_busy_state(&mut self, direction: &str) -> bool {
        self.session
            .history_needs_busy_state(direction_from(direction))
    }

    /// Reads the history in the background of the first paint (call it from an idle callback).
    pub fn warm_history(&mut self) {
        self.session.warm_history();
    }

    /// The operation name of the step a press would act on (`"Move"`), or empty when the stack
    /// has none: the Edit menu and the buttons' tooltips.
    pub fn history_target_name(&mut self, direction: &str) -> String {
        self.session
            .history_target_name(direction_from(direction))
            .unwrap_or_default()
            .to_owned()
    }

    /// How many steps are on the undo stack.
    #[must_use]
    pub fn undo_depth(&self) -> u32 {
        u32::try_from(self.session.undo_len()).unwrap_or(u32::MAX)
    }

    /// How many steps are on the redo stack.
    #[must_use]
    pub fn redo_depth(&self) -> u32 {
        u32::try_from(self.session.redo_len()).unwrap_or(u32::MAX)
    }

    /// Changes whenever the operation log does.
    #[must_use]
    pub fn history_stamp(&self) -> u32 {
        self.session.history_stamp()
    }

    /// How many steps the global history has.
    pub fn history_len(&mut self) -> u32 {
        u32::try_from(self.session.history_len()).unwrap_or(u32::MAX)
    }

    /// `count` rows of the global history from the `start`-th newest, newest first, as a JSON array
    /// (`Session::history_rows_json`).
    pub fn history_rows(&mut self, start: u32, count: u32) -> String {
        self.session
            .history_rows_json(start as usize, count as usize)
    }

    /// The shown id of the newest step in effect (the "Now" row), empty without a step.
    pub fn history_now_id(&mut self) -> String {
        self.session.history_now_id()
    }

    /// The pointer or focus rests on the row of `step` (`<peer>@<counter>`); empty ends the hover.
    pub fn set_history_hover(&mut self, step: &str) {
        self.session.set_history_hover(step);
    }

    /// Shift+Ctrl+H: the History tab.
    pub fn panel_shortcut_history(&mut self) {
        self.session.panel_shortcut_history();
    }

    /// What the History footer offers: `"none"` (no step), `"offered"` or `"shared"`.
    pub fn history_wipe_state(&mut self) -> String {
        match self.session.history_wipe_state() {
            crate::WipeState::NoSteps => "none",
            crate::WipeState::Offered(_) => "offered",
            crate::WipeState::Shared => "shared",
        }
        .to_owned()
    }

    /// Wipes the history. Returns the steps removed, or -1 when a drag, a chip or an unfinished Pen
    /// path made the session ignore it, or -2 for a shared document.
    pub fn history_wipe(&mut self) -> i32 {
        match self.session.wipe_history() {
            crate::WipeOutcome::Ignored => -1,
            crate::WipeOutcome::Shared => -2,
            crate::WipeOutcome::Wiped(removed) => i32::try_from(removed).unwrap_or(i32::MAX),
        }
    }
}
