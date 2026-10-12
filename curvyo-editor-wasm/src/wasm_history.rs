//! Undo and redo's `wasm-bindgen` surface (`specs/0020-undo-redo`): a `impl WasmSession` block that
//! passes straight through to `Session`. Strings and scalars only (ADR 0001 §5).

use curvyo_document_core::Direction;
use wasm_bindgen::prelude::*;

use crate::wasm_api::WasmSession;

/// The directions the panel names: `"undo"`, `"redo"` and, for the selected object,
/// `"object-undo"` and `"object-redo"`.
fn direction_from(name: &str) -> Direction {
    if name == "redo" || name == "object-redo" {
        Direction::Redo
    } else {
        Direction::Undo
    }
}

/// Whether the name is one of the object history's two.
fn is_object(name: &str) -> bool {
    name.starts_with("object-")
}

#[wasm_bindgen]
impl WasmSession {
    /// Takes the maker's latest step back (`direction` `"undo"`) or applies the latest undone step
    /// again (`"redo"`) and returns the one-line notice. Also the Undo and Redo buttons' action.
    pub fn history_press(&mut self, direction: &str) -> String {
        match (is_object(direction), direction_from(direction)) {
            (false, Direction::Undo) => self.session.undo().notice,
            (false, Direction::Redo) => self.session.redo().notice,
            (true, Direction::Undo) => self.session.object_undo().notice,
            (true, Direction::Redo) => self.session.object_redo().notice,
        }
    }

    /// The notice of the last history key press, once (empty when there is none).
    pub fn take_history_notice(&mut self) -> String {
        self.session.take_history_notice().unwrap_or_default()
    }

    /// Whether the press would take long enough to show the long-operation state first: the step
    /// is large, or the history has not been read since the file was opened.
    pub fn history_needs_busy_state(&mut self, direction: &str) -> bool {
        if is_object(direction) {
            return self
                .session
                .object_history_needs_busy_state(direction_from(direction));
        }
        self.session
            .history_needs_busy_state(direction_from(direction))
    }

    /// Reads the history in the background of the first paint (call it from an idle callback).
    pub fn warm_history(&mut self) {
        self.session.warm_history();
    }

    /// The operation name of the step a press would act on (`"Move"`), or empty when the stack
    /// has none: the History tab's buttons and their tooltips.
    pub fn history_target_name(&mut self, direction: &str) -> String {
        let direction_name = direction_from(direction);
        if is_object(direction) {
            return self
                .session
                .object_history_target_name(direction_name)
                .unwrap_or_default();
        }
        self.session
            .history_target_name(direction_name)
            .unwrap_or_default()
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

    /// The shape of the History list of the Document scope (`object` false) or the selected
    /// object's, as JSON (`Session::history_layout_json`).
    pub fn history_layout(&mut self, object: bool) -> String {
        self.session.history_layout_json(object)
    }

    /// `count` items of the History list from the `start`-th, as a JSON array: step rows, branch
    /// header rows and the "more" row, each with its lane gutter (`Session::history_items_json`).
    pub fn history_items(&mut self, object: bool, start: u32, count: u32) -> String {
        self.session
            .history_items_json(object, start as usize, count as usize)
    }

    /// A press or Enter on the header row or the "more" row at `index`: folds or unfolds it.
    pub fn history_toggle(&mut self, object: bool, index: u32) -> bool {
        self.session.toggle_history_item(object, index as usize)
    }

    /// The Mode switch of the Object scope: `true` for Live (`specs/0042-history-branches`).
    pub fn set_history_live(&mut self, live: bool) {
        self.session.set_history_live(live);
    }

    /// Whether the Mode switch stands at Live.
    #[must_use]
    pub fn history_live(&self) -> bool {
        self.session.history_live()
    }

    /// Whether a press on item `index` needs the long-operation state first.
    pub fn history_item_is_far(&mut self, object: bool, index: u32) -> bool {
        self.session.history_item_is_far(object, index as usize)
    }

    /// A press, Enter or Space on item `index` in Preview mode: `"previewed"`, `"ended"` (the
    /// object already has that version), `"ignored"` (a drag, a chip or an unfinished Pen path) or
    /// `"none"` (no step row).
    pub fn history_preview_item(&mut self, object: bool, index: u32) -> String {
        match self.session.preview_history_item(object, index as usize) {
            crate::RowPress::Ignored => "ignored",
            crate::RowPress::NotARow => "none",
            crate::RowPress::Previewed => "previewed",
            crate::RowPress::Ended => "ended",
        }
        .to_owned()
    }

    /// A press, Enter or Space on item `index` of the Object scope in Live mode, as JSON:
    /// `{"kind": "went" | "same" | "ignored" | "none", "notice"}`.
    pub fn history_go_to_item(&mut self, index: u32) -> String {
        let (kind, notice) = match self.session.go_to_history_item(index as usize) {
            crate::GoToOutcome::Ignored => ("ignored", String::new()),
            crate::GoToOutcome::NotARow => ("none", String::new()),
            crate::GoToOutcome::Same => ("same", String::new()),
            crate::GoToOutcome::Went(notice) => ("went", notice),
        };
        serde_json::json!({"kind": kind, "notice": notice}).to_string()
    }

    /// Clone on item `index` (`touched`: which object of a Document-scope row); returns the
    /// notice, empty when the press was ignored or refused.
    pub fn history_clone_item(&mut self, object: bool, index: u32, touched: u32) -> String {
        match self
            .session
            .clone_history_item(object, index as usize, touched as usize)
        {
            crate::CloneOutcome::Cloned(notice) => notice,
            crate::CloneOutcome::Ignored | crate::CloneOutcome::Refused => String::new(),
        }
    }

    /// Whether Restore previous state is offered: an exploration lasts.
    pub fn history_exploring(&mut self) -> bool {
        self.session.history_exploring()
    }

    /// Restore previous state; returns the notice, empty when nothing was restored.
    pub fn history_restore_exploration(&mut self) -> String {
        self.session.restore_exploration().unwrap_or_default()
    }

    /// A change of tab ends the exploration.
    pub fn history_end_exploration(&mut self) {
        self.session.end_exploration();
    }

    /// Ends the preview.
    pub fn history_preview_end(&mut self) {
        self.session.end_history_preview();
    }

    /// The preview as JSON (`Session::history_preview_json`).
    pub fn history_preview(&mut self) -> String {
        self.session.history_preview_json()
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
    /// path made the session ignore it, -2 for a shared document or -3 when the log could not be rewritten.
    pub fn history_wipe(&mut self) -> i32 {
        match self.session.wipe_history() {
            crate::WipeOutcome::Ignored => -1,
            crate::WipeOutcome::Shared => -2,
            crate::WipeOutcome::Failed => -3,
            crate::WipeOutcome::Wiped(removed) => i32::try_from(removed).unwrap_or(i32::MAX),
        }
    }

    /// The Object scope's subject as JSON (kind, ids, the steps the two buttons act on, what a wipe
    /// would hide); empty unless exactly one object is selected.
    pub fn object_history_info(&mut self) -> String {
        self.session.object_history_info_json()
    }

    /// How many rows the selected object's timeline has.
    pub fn object_history_len(&mut self) -> u32 {
        u32::try_from(self.session.object_history_len()).unwrap_or(u32::MAX)
    }

    /// `count` rows of the selected object's timeline from the `start`-th newest, as a JSON array.
    pub fn object_history_rows(&mut self, start: u32, count: u32) -> String {
        self.session
            .object_history_rows_json(start as usize, count as usize)
    }

    /// Hides the selected object's earlier history. Returns the rows hidden, -1 when the session
    /// ignored it (a drag, a chip, an unfinished Pen path, no single object) or -3 when the log
    /// could not take the step.
    pub fn object_history_wipe(&mut self) -> i32 {
        match self.session.wipe_object_history() {
            crate::ObjectWipeOutcome::Ignored => -1,
            crate::ObjectWipeOutcome::Failed => -3,
            crate::ObjectWipeOutcome::Wiped(hidden) => i32::try_from(hidden).unwrap_or(i32::MAX),
        }
    }

    /// The hint shown for an undo or redo while the Pen holds an unfinished path.
    #[must_use]
    pub fn finish_pen_first_notice(&self) -> String {
        curvyo_ui_core::FINISH_PEN_FIRST.to_owned()
    }

    /// The hint shown for Ctrl+U and Ctrl+Shift+U with a tool that is neither Select nor Node.
    #[must_use]
    pub fn object_tool_notice(&self) -> String {
        curvyo_ui_core::OBJECT_TOOL_NOTICE.to_owned()
    }

    /// Whether the Pen holds an unfinished path, which makes a wipe refuse.
    #[must_use]
    pub fn history_wipe_blocked(&self) -> bool {
        self.session.history_wipe_blocked()
    }
}
