//! The key table and its gate (`specs/0010-edit-interaction-polish/` criteria 54, 55, 60, 61;
//! `specs/0023-groups` criterion 7): a pure function from one key event and the facts of the
//! session to the action the session takes. The session's keyboard that carries the actions out
//! is in `keys.rs`.

use curvyo_document_core::Direction;
use curvyo_ui_core::{Arrow, EntryKey, FlipAxis, Reorder};

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
    /// The Pen's path in progress has a last node and the Pen entry chip is closed: Tab opens it
    /// (`specs/0072-pen-angle-snap-and-typed-values` criterion 12).
    pub(super) pen_entry_ready: bool,
    /// The Pen entry chip is open: a stray Enter on the canvas does not finish the path
    /// (`0072` criterion 19).
    pub(super) pen_entry_open: bool,
    /// A pointer button is held: Tab is consumed and opens nothing (`0072` criterion 14).
    pub(super) button_held: bool,
    /// An entry chip is open.
    pub(super) entry_open: bool,
    pub(super) selected: Selected,
    /// The Offset tool has a closed operand selected and no drag: O and Enter open its chip
    /// (`specs/0038-path-offset` criteria 25, 53).
    pub(super) offset_ready: bool,
    /// A corner drag of the Offset tool is in flight: Alt steps its corner style
    /// (`specs/0038-path-offset` criterion 13).
    pub(super) offset_dragging: bool,
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
    Reorder(Reorder),
    OpenOffsetChip,
    StepOffsetCornerStyle,
    /// H or V in the Select tool with a selection (`specs/0056-mirror-and-negative-scale/`
    /// criterion 4, Proposal).
    Flip(FlipAxis),
    StartPlacement,
    /// Tab in the Pen with a last node (`0072` criterion 12).
    OpenPenEntry,
    /// The key is taken from the page and does nothing (`0072` criterion 14).
    Consume,
}

/// What Tab does: in the Pen with a path in progress it opens the entry chip or, with a button
/// held or as a repeat, is consumed so focus stays on the canvas (`0072` criteria 12, 14).
/// Elsewhere, and with Ctrl, Alt or Cmd, it is not handled.
fn tab_action(input: &KeyInput<'_>, state: KeyState) -> KeyAction {
    if state.tool != Tool::Pen || !state.pen_open || input.ctrl || input.alt || input.dom_blocked {
        return KeyAction::Ignore;
    }
    if state.pen_entry_ready && !state.button_held && !input.repeat {
        KeyAction::OpenPenEntry
    } else {
        KeyAction::Consume
    }
}

/// Whether Enter may enter a group: the Select tool, a plain press, nothing in flight.
fn enter_acts(input: &KeyInput<'_>, state: KeyState) -> bool {
    matches!(state.tool, Tool::Select | Tool::Text)
        && !input.repeat
        && !input.dom_blocked
        && !input.ctrl
        && !input.alt
        && !input.shift
        && !state.operation_in_flight
        && !state.pen_open
        && !state.entry_open
}

/// The stacking-order keys (`specs/0050-layers-and-objects-panel` criterion 33): Home, End,
/// Page Up and Page Down act in the Select tool with a selection, plain presses, gated like every
/// letter. `None` for any other key.
fn reorder_action(input: &KeyInput<'_>, state: KeyState, gate_open: bool) -> Option<KeyAction> {
    let how = match input.key {
        "Home" => Reorder::Front,
        "End" => Reorder::Back,
        "PageUp" => Reorder::Forward,
        "PageDown" => Reorder::Backward,
        _ => return None,
    };
    let acts = gate_open
        && !input.ctrl
        && !input.alt
        && !input.shift
        && !input.repeat
        && state.tool == Tool::Select
        && state.selected != Selected::None;
    Some(if acts {
        KeyAction::Reorder(how)
    } else {
        KeyAction::Ignore
    })
}

/// What Enter does: finish the Pen path, open the Offset entry chip, or enter the one selected
/// group (the session says whether there is one).
fn enter_action(input: &KeyInput<'_>, state: KeyState) -> KeyAction {
    // With the Pen entry chip open a stray Enter on the canvas does not finish the path (`0072`
    // criterion 19); the chip's own Enter places.
    if state.tool == Tool::Pen && state.pen_entry_open {
        KeyAction::Ignore
    } else if state.tool == Tool::Pen && state.pen_open {
        KeyAction::FinishPen
    } else if offset_enter_acts(input, state) {
        KeyAction::OpenOffsetChip
    } else if placement_enter_acts(input, state) {
        KeyAction::StartPlacement
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

/// Whether Enter starts a click-to-create placement (`specs/0052-click-to-create-shapes`
/// criterion 26): a creation tool, a plain press, nothing in flight (a running placement
/// included) and no chip open.
fn placement_enter_acts(input: &KeyInput<'_>, state: KeyState) -> bool {
    matches!(
        state.tool,
        Tool::Rectangle | Tool::Ellipse | Tool::PolygonStar
    ) && !input.repeat
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

/// Group and Ungroup (`0023` criterion 7): Ctrl+G and Shift+Ctrl+G, once per press, gated like
/// every shortcut. `None` for any other key.
fn group_action(input: &KeyInput<'_>, state: KeyState, gate_open: bool) -> Option<KeyAction> {
    if !(input.ctrl && input.key.eq_ignore_ascii_case("g")) {
        return None;
    }
    let acts = gate_open && !input.alt && !input.repeat && state.tool == Tool::Select;
    Some(match (acts, input.shift) {
        (true, false) => KeyAction::Group,
        (true, true) => KeyAction::Ungroup,
        (false, _) => KeyAction::Ignore,
    })
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
        "Tab" => return tab_action(input, state),
        // Alt during an Offset corner drag steps its corner style; held Alt steps once
        // (`specs/0038-path-offset` criterion 13). Elsewhere Alt is not intercepted.
        "Alt" => {
            return if state.offset_dragging && !input.repeat {
                KeyAction::StepOffsetCornerStyle
            } else {
                KeyAction::Ignore
            };
        }
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
    if let Some(action) = reorder_action(input, state, gate_open) {
        return action;
    }
    if input.ctrl && input.key.eq_ignore_ascii_case("a") {
        // The Node tool selects every node of its editing set (`0053` criterion 22).
        let tool_acts = selects(state) || state.tool == Tool::Node;
        let acts = gate_open && !input.alt && !input.shift && !input.repeat && tool_acts;
        return if acts {
            KeyAction::SelectAll
        } else {
            KeyAction::Ignore
        };
    }
    if let Some(action) = group_action(input, state, gate_open) {
        return action;
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
    letter_action(input, state, letter)
}

/// The action of a plain letter, `*`, Delete or Backspace that passed the gate.
fn letter_action(input: &KeyInput<'_>, state: KeyState, letter: Option<char>) -> KeyAction {
    let binding = match (letter, input.key) {
        (Some('b'), _) => Binding::Tool(Tool::Pen),
        (Some('n'), _) => Binding::Tool(Tool::Node),
        (Some('e'), _) => Binding::Tool(Tool::Ellipse),
        (Some('o'), _) => Binding::Tool(Tool::Offset),
        (Some('t'), _) => Binding::Tool(Tool::Text),
        // The Trace tool (`specs/0025-raster-trace` criterion 1); Ctrl+I stays Import.
        (Some('i'), _) => Binding::Tool(Tool::Trace),
        (_, "*") => Binding::Tool(Tool::PolygonStar),
        (Some('r'), _) => Binding::Entry(EntryKey::Angle, Tool::Rectangle),
        (Some('s'), _) => Binding::Entry(EntryKey::Size, Tool::Select),
        (Some('m'), _) => Binding::EntryOnly(EntryKey::Move),
        (Some('k'), _) => Binding::EntryOnly(if input.shift {
            EntryKey::SkewY
        } else {
            EntryKey::SkewX
        }),
        (Some('h'), _) => Binding::Flip(FlipAxis::Horizontal),
        (Some('v'), _) => Binding::Flip(FlipAxis::Vertical),
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
        // H and V flip in the Select tool only; with nothing selected they explain
        // themselves as M and K do, and every other tool ignores them (`0056` criterion 4).
        Binding::Flip(axis) => match (state.tool, state.selected) {
            (Tool::Select, Selected::One | Selected::Many) => KeyAction::Flip(axis),
            (Tool::Select, Selected::None) => KeyAction::Hint(KeyHint::SelectFirst),
            _ => KeyAction::Ignore,
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
    /// A flip in the Select tool with a selection, else a hint (Select) or nothing.
    Flip(FlipAxis),
    Delete,
}
