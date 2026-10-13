//! The key table and its gate (`specs/0010-edit-interaction-polish/` criteria 54, 55, 60, 61;
//! `specs/0023-groups` criterion 7): a pure function from one key event and the facts of the
//! session to the action the session takes. The session's keyboard that carries the actions out
//! is in `keys.rs`.

use curvyo_document_core::Direction;
use curvyo_ui_core::{Arrow, EntryKey};

use super::Tool;
use super::keys::{KeyHint, KeyInput};

/// How many objects the Select tool has selected, as far as the key table
/// cares.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Selected {
    None,
    One,
    Many,
}

/// Everything about the session the key table reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)] // one flag per session fact the table reads
pub(super) struct KeyState {
    pub(super) tool: Tool,
    /// A drag is in flight in the active tool, or the canvas is panned.
    pub(super) operation_in_flight: bool,
    /// The Pen has an unfinished path.
    pub(super) pen_open: bool,
    /// An entry chip is open.
    pub(super) entry_open: bool,
    pub(super) selected: Selected,
    /// The Offset tool has a closed operand selected and no drag: O and Enter open its chip
    /// (`specs/0038-path-offset` criteria 25, 53).
    pub(super) offset_ready: bool,
}

/// What the key table decided.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum KeyAction {
    Ignore,
    SetTool(Tool),
    OpenEntry(EntryKey),
    Delete,
    FinishPen,
    Escape,
    Hint(KeyHint),
    History(Direction),
    ObjectHistory(Direction),
    SelectAll,
    Nudge(Arrow),
    Group,
    Ungroup,
    EnterGroup,
    OpenOffsetChip,
}

/// Whether Enter may enter a group: the Select tool, a plain press, nothing in flight.
fn enter_acts(input: &KeyInput<'_>, state: KeyState) -> bool {
    state.tool == Tool::Select
        && !input.repeat
        && !input.dom_blocked
        && !input.ctrl
        && !input.alt
        && !input.shift
        && !state.operation_in_flight
        && !state.pen_open
        && !state.entry_open
}

/// What Enter does: finish the Pen path, open the Offset entry chip, or enter the one selected
/// group (the session says whether there is one).
fn enter_action(input: &KeyInput<'_>, state: KeyState) -> KeyAction {
    if state.tool == Tool::Pen && state.pen_open {
        KeyAction::FinishPen
    } else if offset_enter_acts(input, state) {
        KeyAction::OpenOffsetChip
    } else if enter_acts(input, state) {
        KeyAction::EnterGroup
    } else {
        KeyAction::Ignore
    }
}

/// Whether Enter opens the Offset entry chip: the Offset tool with an operand, a plain press,
/// nothing in flight and no chip open.
fn offset_enter_acts(input: &KeyInput<'_>, state: KeyState) -> bool {
    state.tool == Tool::Offset
        && state.offset_ready
        && !input.repeat
        && !input.dom_blocked
        && !input.ctrl
        && !input.alt
        && !input.shift
        && !state.operation_in_flight
        && !state.entry_open
}

/// Whether the active tool selects objects: the Select tool and the Offset tool
/// (`specs/0038-path-offset` criterion 27) take Ctrl+A and the arrows.
fn selects(state: KeyState) -> bool {
    matches!(state.tool, Tool::Select | Tool::Offset)
}

/// The key table and its gate (criteria 54, 55, 60, 61) as a pure function:
/// the one place that says what a key does in which state.
pub(super) fn decide(input: &KeyInput<'_>, state: KeyState) -> KeyAction {
    match input.key {
        // Never gated by the list of criterion 55; a held Escape acts once.
        "Escape" => {
            return if input.repeat {
                KeyAction::Ignore
            } else {
                KeyAction::Escape
            };
        }
        "Enter" => return enter_action(input, state),
        _ => {}
    }
    // The two keys of `specs/0044-editing-quick-wins/`: they pass the same gate as the letters
    // (no text field, drag, entry chip or unfinished Pen path), and are decided before the line
    // below, which would ignore every key with Ctrl.
    let gate_open =
        !input.dom_blocked && !state.operation_in_flight && !state.pen_open && !state.entry_open;
    if let Some(arrow) = Arrow::from_key(input.key) {
        let acts = gate_open
            && !input.ctrl
            && !input.alt
            && selects(state)
            && state.selected != Selected::None;
        return if acts {
            KeyAction::Nudge(arrow)
        } else {
            KeyAction::Ignore
        };
    }
    if input.ctrl && input.key.eq_ignore_ascii_case("a") {
        let acts = gate_open && !input.alt && !input.shift && !input.repeat && selects(state);
        return if acts {
            KeyAction::SelectAll
        } else {
            KeyAction::Ignore
        };
    }
    // Group and Ungroup (`0023` criterion 7): once per press, gated like every shortcut.
    if input.ctrl && input.key.eq_ignore_ascii_case("g") {
        let acts = gate_open && !input.alt && !input.repeat && state.tool == Tool::Select;
        return match (acts, input.shift) {
            (true, false) => KeyAction::Group,
            (true, true) => KeyAction::Ungroup,
            (false, _) => KeyAction::Ignore,
        };
    }
    // Letters are matched without regard to Caps Lock.
    let letter = match input.key.chars().collect::<Vec<_>>().as_slice() {
        [c] if c.is_ascii_alphabetic() => Some(c.to_ascii_lowercase()),
        _ => None,
    };
    if let Some(direction) = super::history::history_direction(input, letter) {
        return super::history::history_action(direction, input, state);
    }
    if let Some(direction) = super::object_history::object_history_direction(input, letter) {
        return super::object_history::object_history_action(direction, input, state);
    }
    if input.repeat || input.ctrl || input.alt || input.dom_blocked {
        return KeyAction::Ignore;
    }
    let binding = match (letter, input.key) {
        (Some('b'), _) => Binding::Tool(Tool::Pen),
        (Some('n'), _) => Binding::Tool(Tool::Node),
        (Some('e'), _) => Binding::Tool(Tool::Ellipse),
        (Some('o'), _) => Binding::Tool(Tool::Offset),
        (_, "*") => Binding::Tool(Tool::PolygonStar),
        (Some('r'), _) => Binding::Entry(EntryKey::Angle, Tool::Rectangle),
        (Some('s'), _) => Binding::Entry(EntryKey::Size, Tool::Select),
        (Some('m'), _) => Binding::EntryOnly(EntryKey::Move),
        (Some('k'), _) => Binding::EntryOnly(if input.shift {
            EntryKey::SkewY
        } else {
            EntryKey::SkewX
        }),
        (_, "Delete" | "Backspace") => Binding::Delete,
        _ => return KeyAction::Ignore,
    };
    // Shift is allowed only for `*` and Shift+K.
    if input.shift && input.key != "*" && letter != Some('k') {
        return KeyAction::Ignore;
    }
    if state.operation_in_flight || state.pen_open || state.entry_open {
        return KeyAction::Ignore;
    }
    match binding {
        // O in the Offset tool with an operand opens the chip; otherwise it is the tool letter.
        Binding::Tool(Tool::Offset) if state.tool == Tool::Offset && state.offset_ready => {
            KeyAction::OpenOffsetChip
        }
        Binding::Tool(tool) => KeyAction::SetTool(tool),
        Binding::Delete => KeyAction::Delete,
        // The one state that changes a letter: the Select tool with an
        // object selected acts on it; every other state switches tool.
        Binding::Entry(entry, tool) => match (state.tool, state.selected) {
            (Tool::Select, Selected::One | Selected::Many) => KeyAction::OpenEntry(entry),
            _ => KeyAction::SetTool(tool),
        },
        // M and K have no tool to fall back to: outside the Select tool, or
        // with nothing selected, they only explain themselves.
        Binding::EntryOnly(entry) => match (state.tool, state.selected) {
            (Tool::Select, Selected::One | Selected::Many) => KeyAction::OpenEntry(entry),
            _ => KeyAction::Hint(KeyHint::SelectFirst),
        },
    }
}

/// What a key that passed the gate is bound to.
#[derive(Debug, Clone, Copy)]
enum Binding {
    Tool(Tool),
    /// The typed entry in the Select tool with a selection, else the tool.
    Entry(EntryKey, Tool),
    /// The typed entry in the Select tool with a selection, else a hint.
    EntryOnly(EntryKey),
    Delete,
}
