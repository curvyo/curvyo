//! The gate of the file chords (`specs/0051-sidebar-and-no-menu/` criterion 33, `adrs.md`
//! decision 7). Which key press is which command is `curvyo-ui-core`'s table; this says whether
//! the session lets it act now. It is a method of its own, apart from `key_down`, so the editor's
//! own key table still ignores Ctrl+S and the page keeps the key.

use curvyo_ui_core::{FileCommand, Platform, file_chord, is_import_chord};

use super::Session;
use super::keys::KeyInput;

impl Session {
    /// The file command the key press `input` is, if it may act now. `input.ctrl` is the primary
    /// modifier (Cmd on macOS). A chord is ignored during a key repeat, while the page reports it
    /// blocked (an IME composition, a long operation), while a pointer drag runs in any tool and
    /// while a typed-entry chip is open. An unfinished Pen path does not block it: the path is not
    /// part of the document. The page prevents the browser's default only for a `Some`.
    #[must_use]
    pub fn file_chord(&self, input: &KeyInput<'_>, platform: Platform) -> Option<FileCommand> {
        let command = file_chord(input.key, input.shift, input.ctrl, input.alt, platform)?;
        let gated = input.repeat
            || input.dom_blocked
            || self.button_down
            || self.operation_in_flight()
            || self.select.has_entry();
        (!gated).then_some(command)
    }

    /// Whether the key press `input` is Ctrl+I (Cmd+I) for the Import button and may act now
    /// (`specs/0058-bitmap-import` criterion 3): gated as the file chords are, and never in the
    /// on-canvas text editor, where the key stays with the text.
    #[must_use]
    pub fn import_chord(&self, input: &KeyInput<'_>) -> bool {
        is_import_chord(input.key, input.shift, input.ctrl, input.alt)
            && !(input.repeat
                || input.dom_blocked
                || self.button_down
                || self.operation_in_flight()
                || self.select.has_entry()
                || self.text_editing())
    }
}
