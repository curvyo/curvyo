//! The keyboard's `wasm-bindgen` surface (`specs/0010-edit-interaction-polish/`,
//! Parts D and F): a second `impl WasmSession` block, so `wasm_api.rs` does
//! not grow. Strings and scalars only (ADR 0001 §5); every method passes
//! straight through to `Session`, which holds the key table, the gate and the
//! Escape cascade.

use wasm_bindgen::prelude::*;

use curvyo_ui_core::{FileCommand, Platform, command_tooltip};

use crate::session::KeyInput;
use crate::wasm_api::{LiveReadout, WasmSession};

/// The tooltip of a File tab button, with the chord this build has for it ("New (Ctrl+N)", "New"
/// in the browser): `command` is one of `"new"`, `"open"`, `"save"`, `"save-as"`, `"close"`,
/// `"quit"`, `platform` one of `"linux"`, `"windows"`, `"macos"`, `"browser"`. An unknown name
/// gives an empty string. The key text comes from the table that matches the keys
/// (`specs/0051-sidebar-and-no-menu/` criterion 34).
#[wasm_bindgen]
#[must_use]
pub fn file_command_tooltip(command: &str, platform: &str) -> String {
    match (
        FileCommand::from_name(command),
        Platform::from_name(platform),
    ) {
        (Some(command), Some(platform)) => command_tooltip(command, platform),
        _ => String::new(),
    }
}

#[wasm_bindgen]
impl WasmSession {
    /// One key press: `key` is `KeyboardEvent.key`, `ctrl` already folds in
    /// Cmd, `repeat` is the event's auto-repeat flag and `dom_blocked` is
    /// true when focus is in a text field, select, button, switch or
    /// contenteditable element, an IME composition is running, or Space is
    /// held; `mac` is true on macOS (Cmd+Y is no redo key there). Returns what happened: `"ignored"`, `"tool"`, `"entry"`,
    /// `"deleted"`, `"pen"`, `"escape-entry"`, `"escape-drag"`,
    /// `"escape-state"`, `"escape-tool"`, `"escape-none"`,
    /// `"hint-select-first"`, `"hint-too-far"`, `"hint-finish-pen"`,
    /// `"select-all"`, `"group"`, `"ungroup"` (read the notice with `take_grouping_result`),
    /// `"enter-group"`, `"escape-group"` (left an entered group), `"nudge"` (an arrow key moved the selection), `"nudge-new"` (and opened a
    /// new run of held-key events), `"undo"`, `"redo"` (read the notice with
    /// `take_history_notice`), `"undo-busy"` or `"redo-busy"` (show the long-operation state, then
    /// call `history_press`). `time_ms` is `KeyboardEvent.timeStamp`; `mac` is true on macOS (Cmd+Y
    /// is no redo key there). The host prevents the page's default for every result except
    /// `"ignored"` and re-reads the session state for them.
    #[allow(clippy::fn_params_excessive_bools, clippy::too_many_arguments)] // one scalar per DOM fact
    pub fn key_down(
        &mut self,
        key: &str,
        shift: bool,
        ctrl: bool,
        alt: bool,
        repeat: bool,
        dom_blocked: bool,
        mac: bool,
        time_ms: f64,
    ) -> String {
        self.session
            .key_down(KeyInput {
                key,
                shift,
                ctrl,
                alt,
                repeat,
                dom_blocked,
                mac,
                time_ms,
            })
            .code()
            .to_string()
    }

    /// One key press that may be a file chord (`specs/0051-sidebar-and-no-menu/` criteria 32 and
    /// 33). `key` is `KeyboardEvent.key`, `primary` is Ctrl (Cmd on macOS), `blocked` is true
    /// during an IME composition or a long operation, `platform` is `"linux"`, `"windows"`,
    /// `"macos"` or `"browser"`. Returns the command to run (`"new"`, `"open"`, `"save"`,
    /// `"save-as"`, `"close"`, `"quit"`, or `"import"` for the Import button) or an empty string when the press is no chord or may not
    /// act now; the host prevents the page's default for a command only.
    #[allow(clippy::fn_params_excessive_bools, clippy::too_many_arguments)] // one scalar per DOM fact
    #[must_use]
    pub fn file_chord(
        &self,
        key: &str,
        shift: bool,
        primary: bool,
        alt: bool,
        repeat: bool,
        blocked: bool,
        platform: &str,
    ) -> String {
        let Some(platform) = Platform::from_name(platform) else {
            return String::new();
        };
        let input = KeyInput {
            key,
            shift,
            ctrl: primary,
            alt,
            repeat,
            dom_blocked: blocked,
            ..KeyInput::default()
        };
        // Ctrl+I of the Import button (`specs/0058-bitmap-import` criterion 3).
        if self.session.import_chord(&input) {
            return "import".to_owned();
        }
        self.session
            .file_chord(&input, platform)
            .map_or_else(String::new, |command| command.name().to_string())
    }

    /// The move readout of the nudge step that is running ("Δ 3.0, 0.0 mm"),
    /// anchored at the selection's centre in canvas pixels, or `undefined`
    /// before any nudge.
    #[must_use]
    pub fn nudge_readout(&self) -> Option<LiveReadout> {
        let view = self.session.view();
        self.session
            .nudge_readout()
            .map(|readout| LiveReadout::from_document_space(readout, view))
    }

    /// What a screen reader hears when the running nudge step ends ("Moved 11
    /// mm right."), or an empty string before any nudge.
    #[must_use]
    pub fn nudge_announcement(&self) -> String {
        self.session.nudge_announcement()
    }

    /// The browser took the pointer away (`pointercancel`, a window blur):
    /// cancels any drag in flight and forgets the pressed button.
    pub fn pointer_cancelled(&mut self) {
        self.session.pointer_cancelled();
    }

    /// How many objects are selected: the rail's tooltips say "(Esc, R)"
    /// while the Select tool has one (criterion 62).
    #[must_use]
    pub fn selection_count(&self) -> u32 {
        u32::try_from(self.session.selected_object_count()).unwrap_or(u32::MAX)
    }
}
