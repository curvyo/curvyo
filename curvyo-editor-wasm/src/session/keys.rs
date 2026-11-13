//! `Session`'s keyboard: the one gate and key table, the Escape cascade and
//! Delete (`specs/0010-edit-interaction-polish/`, Parts D and F; `adrs.md`
//! decisions 4 and 6).

use curvyo_ui_core::{FlipAxis, FlipOutcome, KeyEntryRefusal, NudgeEvent};

use super::key_table::{KeyAction, KeyState, Selected, decide};
use super::{Session, Tool};

/// One key event as the frontend reports it: the DOM `key` value, the
/// modifiers (`ctrl` already folds in Cmd) and the two facts only the DOM
/// knows.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
#[allow(clippy::struct_excessive_bools)] // one flag per DOM fact, as the host reports them
pub struct KeyInput<'a> {
    /// `KeyboardEvent.key`: `"r"`, `"*"`, `"Delete"`, `"Escape"` and so on.
    pub key: &'a str,
    /// Shift is down.
    pub shift: bool,
    /// Ctrl or Cmd is down.
    pub ctrl: bool,
    /// Alt is down.
    pub alt: bool,
    /// The event is an auto-repeat of a held key.
    pub repeat: bool,
    /// Focus is in a text field, select, button, switch or contenteditable
    /// element, an IME composition is running, or Space is held: knowledge
    /// only the DOM has.
    pub dom_blocked: bool,
    /// The platform is macOS, where Cmd+Y is not a redo key
    /// (`specs/0020-undo-redo` criterion 3).
    pub mac: bool,
    /// `KeyboardEvent.timeStamp` in milliseconds: decides whether a held arrow key is still the
    /// same run of nudges. Tests that do not care leave it at 0.
    pub time_ms: f64,
}

/// The one-line message of a key that cannot act (criterion 59); the
/// frontend owns the wording and shows it for 2 s.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyHint {
    /// Nothing is selected, or M or K was pressed outside the Select tool:
    /// "Select an object first".
    SelectFirst,
    /// Undo or redo while the Pen holds an unfinished path: "Finish the path (Enter) or cancel it
    /// (Esc) first." (`specs/0020-undo-redo` criterion 6e).
    FinishPenFirst,
    /// Ctrl+U or Ctrl+Shift+U with a tool other than Select and Node: "Object undo needs the
    /// Select or Node tool." (`specs/0041-object-history` criterion 5).
    ObjectTool,
    /// An arrow key would move the selection beyond the document's coordinate
    /// limit: "Too far from the document. Nothing was changed."
    TooFar,
}

/// What `Session::key_down` did.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum KeyOutcome {
    /// The gate or the key table said no: nothing happened, and the page's
    /// default for the key is not to be prevented.
    #[default]
    Ignored,
    /// The active tool changed.
    ToolChanged,
    /// A typed entry opened.
    EntryOpened,
    /// Delete or Backspace ran (it may have found nothing to delete).
    Deleted,
    /// Escape took this step.
    Escape(EscapeStep),
    /// Enter finished the Pen path.
    PenFinished,
    /// The key could not act; show this hint.
    Hint(KeyHint),
    /// Ctrl+Z ran (or found nothing to undo); the notice is in [`Session::take_history_notice`].
    Undo,
    /// Ctrl+Shift+Z ran (or found nothing to redo).
    Redo,
    /// Ctrl+U ran for the selected object (or found nothing to undo, or no single object); the
    /// notice is in [`Session::take_history_notice`].
    ObjectUndo,
    /// Ctrl+Shift+U ran for the selected object.
    ObjectRedo,
    /// Ctrl+U needs the long-operation state first.
    ObjectUndoBusy,
    /// Ctrl+Shift+U needs the long-operation state first.
    ObjectRedoBusy,
    /// Ctrl+Z needs the long-operation state first (a large step, or the history is not read yet):
    /// the host shows it, then calls `Session::undo`.
    UndoBusy,
    /// As [`KeyOutcome::UndoBusy`], for redo.
    RedoBusy,
    /// Ctrl+A selected every object (it may have found none).
    SelectedAll,
    /// Ctrl+G grouped the selection; the notice is in [`Session::take_grouping_result`].
    Group,
    /// Ctrl+Shift+G ungrouped the selected groups; the notice is in
    /// [`Session::take_grouping_result`].
    Ungroup,
    /// Enter entered the selected group.
    EnteredGroup,
    /// Home, End, Page Up or Page Down changed the stacking order of the selection (or found it
    /// already there); a refusal is in [`Session::take_layer_notice`].
    Reordered,
    /// O or Enter opened the Offset entry chip (`specs/0038-path-offset` criterion 25).
    OffsetChipOpened,
    /// Alt stepped the corner style of the Offset corner drag (`specs/0038-path-offset`
    /// criterion 13).
    OffsetCornerStyle,
    /// An arrow key moved the selection; `new_run` is true when the event
    /// opened a new run of held-key events (a new step once `0020` exists).
    Nudged {
        /// The event opened a new run.
        new_run: bool,
    },
    /// A document preview pauses editing: the key did nothing, and its page default is prevented.
    Paused,
    /// H or V flipped the selection, or found it looking the same flipped
    /// (`specs/0056-mirror-and-negative-scale/` criteria 3 and 4).
    Flipped(FlipAxis, FlipOutcome),
    /// Enter started a click-to-create placement (`specs/0052-click-to-create-shapes`
    /// criterion 26): the host opens the placement chip.
    PlacementStarted,
    /// Tab opened the Pen entry chip (`specs/0072-pen-angle-snap-and-typed-values` criterion 12).
    PenEntryOpened,
    /// Enter started text edit mode (`specs/0074-text-objects` criterion 10): the host focuses
    /// the input sink.
    TextEditStarted,
    /// The key was taken from the page and did nothing (Tab during a Pen drag, `0072` criterion
    /// 14).
    Consumed,
}

impl KeyOutcome {
    /// The string the frontend reads (ADR 0001 §5: strings and scalars across
    /// the wasm boundary). Everything but `"ignored"` means the key was
    /// handled and its page default is to be prevented.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Ignored => "ignored",
            Self::Paused => "paused",
            Self::PlacementStarted => "placement-started",
            Self::PenEntryOpened => "pen-entry",
            Self::TextEditStarted => "text-edit",
            Self::Consumed => "consumed",
            Self::ToolChanged => "tool",
            Self::EntryOpened => "entry",
            Self::Deleted => "deleted",
            Self::PenFinished => "pen",
            Self::Escape(EscapeStep::ClosedEntry) => "escape-entry",
            Self::Escape(EscapeStep::CancelledDrag) => "escape-drag",
            Self::Escape(EscapeStep::ClearedState) => "escape-state",
            Self::Escape(EscapeStep::LeftGroup) => "escape-group",
            Self::Escape(EscapeStep::LeftTool) => "escape-tool",
            Self::Escape(EscapeStep::Nothing) => "escape-none",
            Self::Hint(KeyHint::SelectFirst) => "hint-select-first",
            Self::Hint(KeyHint::FinishPenFirst) => "hint-finish-pen",
            Self::Hint(KeyHint::ObjectTool) => "hint-object-tool",
            Self::Undo => "undo",
            Self::Redo => "redo",
            Self::ObjectUndo => "object-undo",
            Self::ObjectRedo => "object-redo",
            Self::ObjectUndoBusy => "object-undo-busy",
            Self::ObjectRedoBusy => "object-redo-busy",
            Self::UndoBusy => "undo-busy",
            Self::RedoBusy => "redo-busy",
            Self::Hint(KeyHint::TooFar) => "hint-too-far",
            Self::SelectedAll => "select-all",
            Self::Group => "group",
            Self::Ungroup => "ungroup",
            Self::EnteredGroup => "enter-group",
            Self::Reordered => "reordered",
            Self::OffsetChipOpened => "offset-chip",
            Self::OffsetCornerStyle => "offset-corner-style",
            Self::Nudged { new_run: false } => "nudge",
            Self::Nudged { new_run: true } => "nudge-new",
            Self::Flipped(_, FlipOutcome::LooksTheSame) => "flip-unchanged",
            Self::Flipped(FlipAxis::Horizontal, _) => "flip-horizontal",
            Self::Flipped(FlipAxis::Vertical, _) => "flip-vertical",
        }
    }
}

/// The one step a single Escape press took (criterion 42); the document is
/// never written by any of them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EscapeStep {
    /// An open entry chip was closed.
    ClosedEntry,
    /// A drag in flight was cancelled.
    CancelledDrag,
    /// The active tool's own state was cleared: the Pen path, the Node
    /// tool's node and segment selection, the Select tool's object
    /// selection.
    ClearedState,
    /// The Select tool left the innermost entered group (`specs/0023-groups` criterion 17); the
    /// group just left is selected.
    LeftGroup,
    /// The tool was left for the Select tool.
    LeftTool,
    /// There was nothing to do.
    Nothing,
}

impl Session {
    /// The selection as the key table sees it: only the Select tool's
    /// object selection counts.
    fn selected_for_keys(&self) -> Selected {
        if !matches!(self.tool, Tool::Select | Tool::Offset) {
            return Selected::None;
        }
        match self.selection.ids().len() {
            0 => Selected::None,
            1 => Selected::One,
            _ => Selected::Many,
        }
    }

    /// Whether a drag is in flight in the active tool (move, resize, rotate,
    /// skew, parameter handle, create-drag, node or handle drag), or the
    /// canvas is being panned (criterion 55). The Pen's unfinished path is a
    /// separate fact.
    pub(super) fn operation_in_flight(&self) -> bool {
        let in_tool = match self.tool {
            Tool::Select => self.select.drag_in_flight(),
            Tool::Node => self.node.drag_in_flight(),
            Tool::Rectangle => self.rectangle.drag_in_flight(),
            Tool::Ellipse => self.ellipse.drag_in_flight(),
            Tool::PolygonStar => self.poly_star.drag_in_flight(),
            Tool::Offset => self.offset_drag_in_flight() || self.select.drag_in_flight(),
            Tool::Measure => self.dimension_drag_in_flight(),
            Tool::Trace => self.trace.mask.drag.is_some(),
            Tool::Pen | Tool::Text | Tool::TestGrid => false,
        };
        // A drag of a panel value field is a drag too (`specs/0020-undo-redo` criterion 6a).
        // A click-to-create placement is an operation in flight (`0052` criterion 8).
        // So is an open Pen entry chip (`0072` criterion 23).
        in_tool
            || self.sketch.drag.is_some()
            || self.page_drag.is_some()
            || self.placement().is_some()
            || self.pen_entry().is_some()
            || self.is_panning()
            || self.style.pending_edit().is_some()
            || self.select.bar_preview().is_some()
            || self.background_preview.is_some()
            || self.text_path_dragging()
            // The open tiling bar: a preview is not a history step (`0069` criterion 35).
            || self.tiling.is_some()
    }

    /// The open tiling bar keeps every key but a tool key, which cancels it (`0069` criterion
    /// 35), and C, R, A, S, which open a circle handle's entry box (UX notes §12.4); Enter and
    /// Escape are the bar's own. `None` while the bar is closed.
    fn tiling_key(&mut self, input: &KeyInput<'_>) -> Option<KeyOutcome> {
        self.tiling.as_ref()?;
        if self.tiling_entry_key(input) {
            return Some(KeyOutcome::EntryOpened);
        }
        Some(match decide(input, self.key_state()) {
            KeyAction::Ignore => KeyOutcome::Ignored,
            KeyAction::SetTool(tool) => {
                self.set_tool(tool);
                KeyOutcome::ToolChanged
            }
            _ => KeyOutcome::Paused,
        })
    }

    pub(super) fn pen_path_open(&self) -> bool {
        self.pen.in_progress_nodes().is_some()
    }

    /// The number of selected objects.
    #[must_use]
    pub fn selected_object_count(&self) -> usize {
        self.selection.ids().len()
    }

    /// Ctrl+G (`group`) or Ctrl+Shift+G: acts only when the command is available;
    /// otherwise the page keeps the chord.
    fn grouping_key(&mut self, group: bool) -> KeyOutcome {
        let availability = self.grouping_availability();
        if !(if group {
            availability.group
        } else {
            availability.ungroup
        }) {
            return KeyOutcome::Ignored;
        }
        let result = if group {
            self.group_selection()
        } else {
            self.ungroup_selection()
        };
        let acted = result.kind != super::grouping::GroupingKind::Ignored;
        self.grouping_result = Some(result);
        match (acted, group) {
            (false, _) => KeyOutcome::Ignored,
            (true, true) => KeyOutcome::Group,
            (true, false) => KeyOutcome::Ungroup,
        }
    }

    /// Enter: a move-only selection's page becomes active (`specs/0106` UX notes §4); one
    /// selected text, or the Text tool with nothing selected, starts edit mode
    /// (`specs/0074-text-objects` criterion 10); one selected group is entered.
    fn enter_key(&mut self) -> KeyOutcome {
        if self.enter_switch() {
            return KeyOutcome::Consumed;
        }
        if self.text_enter() {
            return KeyOutcome::TextEditStarted;
        }
        self.sync_context();
        if self.enter_selected_group() {
            KeyOutcome::EnteredGroup
        } else {
            KeyOutcome::Ignored
        }
    }

    /// The key table's action for `input`; [`KeyAction::Consume`] when a group of a laser page
    /// is entered and the action is refused there silently: only selection, Escape, undo and
    /// entering a nested group act (`specs/0111` criterion 11, UX notes T4).
    fn table_action(&self, input: &KeyInput<'_>) -> KeyAction {
        let action = decide(input, self.key_state());
        if !self.in_read_only_group() || read_only_key(action) {
            action
        } else {
            KeyAction::Consume
        }
    }

    /// The facts of the session the key table reads.
    pub(super) fn key_state(&self) -> KeyState {
        KeyState {
            tool: self.tool,
            operation_in_flight: self.operation_in_flight(),
            pen_open: self.pen_path_open(),
            pen_entry_ready: self.pen_last_node().is_some() && self.pen_entry().is_none(),
            pen_entry_open: self.pen_entry().is_some(),
            button_held: self.button_down,
            entry_open: self.select.has_entry()
                || self.offset_chip_open()
                || self.pen_entry().is_some(),
            selected: self.selected_for_keys(),
            offset_ready: self.offset_center().is_some(),
            offset_dragging: self.tool == Tool::Offset && self.offset_drag_in_flight(),
        }
    }

    /// A key a mode takes before the key table: Enter in text-path pick mode (`0080` criterion
    /// 2), the Trace tool's mask (Escape, Delete and the arrows, `specs/0114`), the tiling bar's.
    fn mode_key(&mut self, input: &KeyInput<'_>) -> Option<KeyOutcome> {
        if let Some(outcome) = self.trace_mask_key(input) {
            return Some(outcome);
        }
        if input.key == "Enter" && !input.dom_blocked && self.confirm_text_path_pick() {
            return Some(KeyOutcome::Consumed);
        }
        self.tiling_key(input)
    }

    /// One key press: the gate of criterion 55, then the key table of
    /// criterion 54. The frontend forwards the key and what the DOM alone
    /// knows and prevents the page's default for every outcome except
    /// [`KeyOutcome::Ignored`], so Ctrl+R and Ctrl+S keep their page and menu
    /// behaviour.
    pub fn key_down(&mut self, input: KeyInput<'_>) -> KeyOutcome {
        self.edit_context.refused = false;
        let repeat = input.repeat;
        let outcome = self.key_down_dispatch(input);
        // A chord that reached a guarded edit with a move-only selection says so once
        // (`specs/0106` criterion 9).
        self.note_refused_key(repeat);
        outcome
    }

    fn key_down_dispatch(&mut self, input: KeyInput<'_>) -> KeyOutcome {
        // A document preview pauses editing: every key of the table, undo and redo included
        // (`specs/0042-history-branches` criterion 19).
        if self.document_preview_active() {
            return KeyOutcome::Paused;
        }
        if let Some(outcome) = self.page_switch_key(&input) {
            return outcome;
        }
        if self.workbench() == super::Workbench::Sketch {
            return match self.sketch_drawing_key(&input) {
                Some(outcome) => outcome,
                None => self.sketch_key(input),
            };
        }
        if let Some(outcome) = self.mode_key(&input) {
            return outcome;
        }
        // A selected fixture takes the arrows, Delete and Escape (`specs/0102` criteria 13, 14).
        if let Some(outcome) = self.fixture_key(&input) {
            return outcome;
        }
        match self.table_action(&input) {
            KeyAction::Ignore => KeyOutcome::Ignored,
            KeyAction::SetTool(tool) => {
                self.set_tool(tool);
                KeyOutcome::ToolChanged
            }
            // S, R and K do nothing for a whole test grid; M moves it (`specs/0028` ux-notes 6.5).
            KeyAction::OpenEntry(key)
                if !matches!(key, curvyo_ui_core::EntryKey::Move)
                    && self.whole_test_grid_selected() =>
            {
                KeyOutcome::Consumed
            }
            KeyAction::OpenEntry(key) => {
                let objects = self.objects();
                match self
                    .select
                    .open_entry_for_key(&objects, &self.selection, key)
                {
                    Ok(()) => {
                        self.select.mint_conversion_ids(&mut self.minter);
                        KeyOutcome::EntryOpened
                    }
                    Err(KeyEntryRefusal::NothingSelected) => KeyOutcome::Hint(KeyHint::SelectFirst),
                }
            }
            KeyAction::Delete => {
                self.delete_selected();
                KeyOutcome::Deleted
            }
            KeyAction::StartPlacement => self.start_placement_key(),
            KeyAction::OpenPenEntry => {
                if self.open_pen_entry() {
                    KeyOutcome::PenEntryOpened
                } else {
                    KeyOutcome::Consumed
                }
            }
            KeyAction::Consume => KeyOutcome::Consumed,
            KeyAction::MeasureAlt => self.measure_key(true, &input),
            KeyAction::MeasureEnter => self.measure_key(false, &input),
            KeyAction::FinishPen => {
                self.finish_pen();
                KeyOutcome::PenFinished
            }
            KeyAction::Escape => KeyOutcome::Escape(self.escape()),
            KeyAction::Hint(hint) => KeyOutcome::Hint(hint),
            KeyAction::History(direction) => self.history_key(direction),
            KeyAction::ObjectHistory(direction) => self.object_history_key(direction),
            KeyAction::SelectAll => self.select_all(),
            KeyAction::Group => self.grouping_key(true),
            KeyAction::Ungroup => self.grouping_key(false),
            KeyAction::OpenOffsetChip => {
                if self.open_offset_chip() {
                    KeyOutcome::OffsetChipOpened
                } else {
                    KeyOutcome::Ignored
                }
            }
            KeyAction::StepOffsetCornerStyle => {
                if self.offset_step_drag_join() {
                    KeyOutcome::OffsetCornerStyle
                } else {
                    KeyOutcome::Ignored
                }
            }
            KeyAction::Flip(axis) => KeyOutcome::Flipped(axis, self.flip_selection(axis)),
            KeyAction::EnterGroup => self.enter_key(),
            KeyAction::Reorder(how) => self.reorder_selection(how),
            KeyAction::Nudge(arrow) => {
                let event = NudgeEvent {
                    arrow,
                    shift: input.shift,
                    repeat: input.repeat,
                    time_ms: input.time_ms,
                };
                // One text on path moves along its carrier (`0080` criterion 21).
                if self.nudge_text_path(event) {
                    return KeyOutcome::Nudged { new_run: true };
                }
                self.nudge_selection(event)
            }
        }
    }

    /// Cancels the drag in flight in the active tool, writing nothing: the
    /// shared step 2 of the Escape cascade, a lost pointer and a non-finite
    /// release. Returns whether there was one. The Pen has no separate drag:
    /// its step 2 and 3 are one, see [`Session::escape`].
    pub(super) fn cancel_drag_in_flight(&mut self) -> bool {
        self.end_snapping_drag();
        self.end_fixture_fit();
        self.clone_drop_cancel();
        if self.page_drag.take().is_some() {
            return true;
        }
        match self.tool {
            Tool::Select => {
                let was = self.select.drag_in_flight();
                if was {
                    self.select.escape();
                    self.clone_drops.escaped = self.button_down;
                }
                was
            }
            Tool::Node => {
                self.node_spiro_sources.clear();
                self.spiro_cache.borrow_mut().clear_node_previews();
                self.node.cancel_drag()
            }
            Tool::Rectangle | Tool::Ellipse | Tool::PolygonStar => self.shape_escape(),
            Tool::Offset => {
                let marquee = self.select.drag_in_flight();
                if marquee {
                    self.select.escape();
                }
                self.cancel_offset_drag() || marquee
            }
            Tool::Measure => self.measure_escape(),
            Tool::Pen | Tool::Text | Tool::Trace | Tool::TestGrid => false,
        }
    }

    /// Escape: exactly one step per press, the first that applies
    /// (`specs/0010-edit-interaction-polish/` criterion 42), and the document is
    /// never written:
    ///
    /// 1. an open entry chip closes;
    /// 2. a drag in flight is cancelled and the selection stays as it was;
    /// 3. the tool's own state is cleared: the Pen path (also during a handle
    ///    drag, steps 2 and 3 in one), the Node tool's node and segment
    ///    selection, the Select tool's object selection;
    /// 4. a tool other than Select is left for the Select tool, with the
    ///    object selection exactly as it was.
    ///
    /// With the pointer button still down steps 3 and 4 are skipped: only the
    /// drag is cancelled (criterion 49).
    pub fn escape(&mut self) -> EscapeStep {
        if let Some(step) = self.trace_mask_escape() {
            return step;
        }
        if let Some(step) = self.escape_entries_and_pickers() {
            return step;
        }
        if let Some(step) = self.escape_drags_and_pen() {
            return step;
        }
        self.escape_selection_and_tool()
    }

    /// Escape steps for the width, sketch, page, fixture, colour and entry states; `None` when
    /// none was open.
    fn escape_entries_and_pickers(&mut self) -> Option<EscapeStep> {
        if self.width_escape() {
            return Some(EscapeStep::ClearedState);
        }
        if self.workbench() == super::Workbench::Sketch {
            return Some(self.sketch_escape());
        }
        if self.page_drag.take().is_some() {
            return Some(EscapeStep::ClearedState);
        }
        // A fixture's drag, then its selection (`specs/0102-laser-fixtures` UX notes §10).
        if self.cancel_fixture_drag() {
            return Some(EscapeStep::CancelledDrag);
        }
        if self.selected_fixture().is_some() {
            self.clear_fixture_selection();
            return Some(EscapeStep::ClearedState);
        }
        // Picking a colour ends first (`0017` criterion 60).
        if self.colour_pick_target().is_some() {
            self.end_colour_pick();
            return Some(EscapeStep::ClearedState);
        }
        if self.select.has_entry() {
            self.select.cancel_entry();
            return Some(EscapeStep::ClosedEntry);
        }
        None
    }

    /// Escape steps that end an open entry chip, a drag, a pick mode, a placement or the Pen path;
    /// `None` when none was open.
    fn escape_drags_and_pen(&mut self) -> Option<EscapeStep> {
        // The Offset entry chip closes without writing; the selection stays (`0038` criterion 54).
        if self.offset_chip_open() {
            self.close_offset_chip();
            return Some(EscapeStep::ClosedEntry);
        }
        // The Pen entry chip closes alone; the path stays (`0072` criterion 22).
        if self.cancel_pen_entry() {
            return Some(EscapeStep::ClosedEntry);
        }
        if self.cancel_text_path_drag() {
            return Some(EscapeStep::CancelledDrag);
        }
        if self.cancel_drag_in_flight()
            || self.cancel_dimension_drag()
            || self.cancel_kept_point_drag()
        {
            return Some(EscapeStep::CancelledDrag);
        }
        // Pick mode ends before the selection clears (`0080` UX notes §2).
        if self.cancel_text_path_pick() {
            return Some(EscapeStep::ClearedState);
        }
        // A placement is step 2 too (`0052` criterion 7). It is not in `cancel_drag_in_flight`,
        // which a lost pointer and a window blur also call: those must not cancel it.
        if self.cancel_placement() {
            return Some(EscapeStep::CancelledDrag);
        }
        if self.tool == Tool::Pen && self.pen.escape() {
            // The Pen's step 2 and 3 are one: the unfinished path goes,
            // also during a handle drag.
            return Some(if self.button_down {
                EscapeStep::CancelledDrag
            } else {
                EscapeStep::ClearedState
            });
        }
        None
    }

    /// The last Escape steps: the key object, a text edit, the entered group, the selection, the
    /// innermost group, then the tool.
    fn escape_selection_and_tool(&mut self) -> EscapeStep {
        if self.button_down {
            return EscapeStep::Nothing;
        }
        // The key object of the align and distribute commands is cleared
        // first and keeps the selection (`specs/0061-align-and-distribute/`
        // criterion 5).
        if self.tool == Tool::Select && self.key_object.take().is_some() {
            return EscapeStep::ClearedState;
        }
        // The first Escape in text edit mode ends it and keeps the text: the one step of the
        // cascade that writes (`specs/0074-text-objects` criterion 7).
        if self.finish_text_edit() {
            return EscapeStep::ClearedState;
        }
        // Escape inside a group of a laser page leaves it in one press with the group selected
        // (`specs/0111` criterion 13, UX notes T5).
        if self.tool == Tool::Select && self.in_read_only_group() {
            self.select.end_cycle();
            self.leave_read_only_group(true);
            return EscapeStep::LeftGroup;
        }
        let cleared = match self.tool {
            Tool::Pen
            | Tool::Rectangle
            | Tool::Ellipse
            | Tool::PolygonStar
            | Tool::Text
            | Tool::Measure
            | Tool::Trace
            | Tool::TestGrid => false,
            Tool::Node => self.node.clear_selection(),
            Tool::Select | Tool::Offset => {
                let had = !self.selection.is_empty();
                self.selection.clear();
                self.select.end_cycle();
                had
            }
        };
        if cleared {
            return EscapeStep::ClearedState;
        }
        if self.tool == Tool::Select {
            // The selection is clear: leave the innermost entered group and select it
            // (`0023` criterion 17); at the root there is nothing more to do.
            self.sync_context();
            return if self.leave_group(true) {
                EscapeStep::LeftGroup
            } else {
                EscapeStep::Nothing
            };
        }
        self.set_tool(Tool::Select);
        EscapeStep::LeftTool
    }

    /// The browser took the pointer away (a system gesture, an alert) or the
    /// window lost focus: with the button down, cancels any drag in flight;
    /// either way forgets the button.
    pub fn pointer_cancelled(&mut self) {
        // Without a pressed button nothing was lost: a window blur must not
        // throw away an unfinished Pen path.
        if self.button_down {
            self.cancel_gesture();
        }
        self.button_down = false;
    }

    /// Cancels whatever gesture the active tool has in flight when the
    /// pointer is lost: a drag, or, in the Pen, the unfinished path (the Pen
    /// has no way to cancel only a handle drag).
    pub(super) fn cancel_gesture(&mut self) {
        self.page_drag = None;
        if self.width_cancel_drag() {
            return;
        }
        if !self.cancel_drag_in_flight() && self.tool == Tool::Pen {
            self.pen.escape();
        }
    }

    /// Acceptance criteria 19, 21 (Delete/Backspace, or the contextual
    /// toolbar's Delete button): removes every selected object when the
    /// Select or Node tool is active. A no-op for every other tool.
    pub fn delete_selected(&mut self) {
        // Nothing is deleted inside a group of a laser page (`specs/0111` criterion 11).
        if self.in_read_only_group() || self.width_delete_selected() {
            return;
        }
        self.recording_unguarded(|this| {
            this.flush_select_bar_preview();
            this.flush_style_preview();
            match this.tool {
                Tool::Select => {
                    let objects = this.objects();
                    // What the notice counts: the selected objects and groups (criterion 31).
                    this.deleted_objects = this.selected_units().len();
                    this.select
                        .delete_selected(&this.document, &objects, &mut this.selection);
                }
                Tool::Node => {
                    if this.document.can_delete_anchors(this.node.selection().node_pairs()) {
                        this.node.delete_selected(&this.document);
                    } else {
                        this.history.notice = Some("A closed B-spline needs at least three control points. Nothing changed.".to_owned());
                    }
                }
                Tool::Pen
                | Tool::Rectangle
                | Tool::Ellipse
                | Tool::PolygonStar
                | Tool::Offset
                | Tool::Text
                | Tool::Measure
                | Tool::Trace
                | Tool::TestGrid => {}
            }
        });
    }
}

/// Whether `action` may run inside a read-only group of a laser page (`specs/0111` criteria 11 to
/// 13): selection, Escape, undo and redo, entering a nested group and the Select tool letter.
fn read_only_key(action: KeyAction) -> bool {
    matches!(
        action,
        KeyAction::Ignore
            | KeyAction::Escape
            | KeyAction::Hint(_)
            | KeyAction::Consume
            | KeyAction::History(_)
            | KeyAction::SelectAll
            | KeyAction::EnterGroup
            | KeyAction::SetTool(Tool::Select)
    )
}

#[cfg(test)]
mod tests {
    use curvyo_ui_core::{Arrow, EntryKey};

    use super::*;

    const TOOLS: [Tool; 6] = [
        Tool::Select,
        Tool::Pen,
        Tool::Node,
        Tool::Rectangle,
        Tool::Ellipse,
        Tool::PolygonStar,
    ];
    /// One gate condition, applied to a key and a state.
    type Blocker = Box<dyn Fn(&mut KeyInput<'_>, &mut KeyState)>;

    const SELECTIONS: [Selected; 3] = [Selected::None, Selected::One, Selected::Many];

    fn state(tool: Tool, selected: Selected) -> KeyState {
        KeyState {
            tool,
            operation_in_flight: false,
            pen_open: false,
            pen_entry_ready: false,
            pen_entry_open: false,
            button_held: false,
            entry_open: false,
            selected,
            offset_ready: false,
            offset_dragging: false,
        }
    }

    fn key(key: &str) -> KeyInput<'_> {
        KeyInput {
            key,
            ..KeyInput::default()
        }
    }

    /// `specs/0038-path-offset` criteria 25, 53: in the Offset tool with an operand, O and Enter
    /// open the chip, gated like every shortcut; without one O stays the tool letter.
    /// `specs/0038-path-offset` criterion 13 (fourth round): Alt steps the corner style only
    /// during an Offset corner drag, once per press, with or without Shift and Ctrl.
    #[test]
    fn alt_steps_the_offset_corner_style_only_during_a_corner_drag() {
        let dragging = KeyState {
            offset_dragging: true,
            operation_in_flight: true,
            ..state(Tool::Offset, Selected::One)
        };
        let alt = KeyInput {
            alt: true,
            ..key("Alt")
        };
        assert_eq!(decide(&alt, dragging), KeyAction::StepOffsetCornerStyle);
        let held = KeyInput {
            shift: true,
            ctrl: true,
            ..alt
        };
        assert_eq!(decide(&held, dragging), KeyAction::StepOffsetCornerStyle);
        let repeat = KeyInput {
            repeat: true,
            ..alt
        };
        assert_eq!(decide(&repeat, dragging), KeyAction::Ignore);
        for tool in TOOLS {
            assert_eq!(decide(&alt, state(tool, Selected::One)), KeyAction::Ignore);
        }
        assert_eq!(
            decide(&alt, state(Tool::Offset, Selected::One)),
            KeyAction::Ignore,
            "no drag: not intercepted"
        );
    }

    #[test]
    fn o_and_enter_open_the_offset_chip_in_the_offset_tool() {
        let ready = KeyState {
            offset_ready: true,
            ..state(Tool::Offset, Selected::One)
        };
        assert_eq!(decide(&key("o"), ready), KeyAction::OpenOffsetChip);
        assert_eq!(decide(&key("Enter"), ready), KeyAction::OpenOffsetChip);
        assert_eq!(
            decide(&key("o"), state(Tool::Offset, Selected::One)),
            KeyAction::SetTool(Tool::Offset)
        );
        assert_eq!(
            decide(&key("o"), state(Tool::Select, Selected::One)),
            KeyAction::SetTool(Tool::Offset)
        );
        let blocked: [Blocker; 4] = [
            Box::new(|k, _| k.repeat = true),
            Box::new(|k, _| k.dom_blocked = true),
            Box::new(|_, s| s.entry_open = true),
            Box::new(|_, s| s.operation_in_flight = true),
        ];
        for block in &blocked {
            for name in ["o", "Enter"] {
                let (mut k, mut s) = (key(name), ready);
                block(&mut k, &mut s);
                assert_ne!(decide(&k, s), KeyAction::OpenOffsetChip, "{name}");
            }
        }
    }

    /// Criterion 54: B, N, E and `*` select their tool in every state; so does O
    /// (`specs/0038-path-offset` criterion 2).
    #[test]
    fn b_n_e_and_star_select_their_tool_in_every_state() {
        for tool in TOOLS {
            for selected in SELECTIONS {
                for (k, target) in [
                    ("b", Tool::Pen),
                    ("n", Tool::Node),
                    ("e", Tool::Ellipse),
                    ("*", Tool::PolygonStar),
                    ("o", Tool::Offset),
                ] {
                    assert_eq!(
                        decide(&key(k), state(tool, selected)),
                        KeyAction::SetTool(target),
                        "{k} in {tool:?} with {selected:?}"
                    );
                }
            }
        }
    }

    /// Criterion 54: R and S act on the selection only in the Select tool
    /// with an object selected (one or several, `multi-object-transform`
    /// criteria 33 to 37); every other state switches tool.
    #[test]
    fn r_and_s_act_on_the_selection_only_in_the_select_tool() {
        for tool in TOOLS {
            for selected in SELECTIONS {
                for (k, entry, target) in [
                    ("r", EntryKey::Angle, Tool::Rectangle),
                    ("s", EntryKey::Size, Tool::Select),
                ] {
                    let expected = match (tool, selected) {
                        (Tool::Select, Selected::One | Selected::Many) => {
                            KeyAction::OpenEntry(entry)
                        }
                        _ => KeyAction::SetTool(target),
                    };
                    assert_eq!(
                        decide(&key(k), state(tool, selected)),
                        expected,
                        "{k} in {tool:?} with {selected:?}"
                    );
                }
            }
        }
    }

    /// Letters are matched without regard to Caps Lock.
    #[test]
    fn letters_are_case_insensitive() {
        for (upper, lower) in [("B", "b"), ("N", "n"), ("E", "e"), ("R", "r"), ("S", "s")] {
            assert_eq!(
                decide(&key(upper), state(Tool::Select, Selected::One)),
                decide(&key(lower), state(Tool::Select, Selected::One)),
                "{upper}"
            );
        }
    }

    /// Criterion 61: Delete and Backspace pass the same gate; what they
    /// delete is the tool's own business.
    #[test]
    fn delete_and_backspace_are_one_action_in_every_tool() {
        for tool in TOOLS {
            for k in ["Delete", "Backspace"] {
                assert_eq!(
                    decide(&key(k), state(tool, Selected::One)),
                    KeyAction::Delete
                );
            }
        }
    }

    /// Keys that are not bound do nothing.
    #[test]
    fn unbound_keys_do_nothing() {
        for k in ["x", "1", "Tab", "F5", " ", "Shift"] {
            assert_eq!(
                decide(&key(k), state(Tool::Select, Selected::One)),
                KeyAction::Ignore,
                "{k}"
            );
        }
    }

    /// The gate (criterion 55), one condition at a time and in pairs, for
    /// every key it covers in every tool: no effect at all.
    #[test]
    fn every_gate_condition_blocks_every_gated_key() {
        let gated = [
            "b",
            "n",
            "e",
            "r",
            "s",
            "m",
            "k",
            "*",
            "Delete",
            "Backspace",
        ];
        let mut blockers: Vec<(&str, Blocker)> = vec![
            ("ctrl", Box::new(|i, _| i.ctrl = true)),
            ("alt", Box::new(|i, _| i.alt = true)),
            ("repeat", Box::new(|i, _| i.repeat = true)),
            ("dom", Box::new(|i, _| i.dom_blocked = true)),
            ("drag", Box::new(|_, s| s.operation_in_flight = true)),
            ("pen path", Box::new(|_, s| s.pen_open = true)),
            ("entry", Box::new(|_, s| s.entry_open = true)),
        ];
        // Shift blocks everything but `*` and K.
        blockers.push(("shift", Box::new(|i, _| i.shift = true)));
        for tool in TOOLS {
            for selected in SELECTIONS {
                for k in gated {
                    for (a, (name_a, block_a)) in blockers.iter().enumerate() {
                        for (b, (name_b, block_b)) in blockers.iter().enumerate() {
                            if b < a {
                                continue;
                            }
                            let mut input = key(k);
                            let mut st = state(tool, selected);
                            block_a(&mut input, &mut st);
                            block_b(&mut input, &mut st);
                            // `*` and K with only Shift are the allowed pairs.
                            let only_shift_on_star =
                                matches!(k, "*" | "k") && *name_a == "shift" && *name_b == "shift";
                            if only_shift_on_star {
                                continue;
                            }
                            assert_eq!(
                                decide(&input, st),
                                KeyAction::Ignore,
                                "{k} in {tool:?} with {selected:?} blocked by {name_a} + {name_b}"
                            );
                        }
                    }
                }
            }
        }
    }

    /// Criteria 54 and 59: M and K open their entry in the Select tool with
    /// one object selected (K is skew x, Shift+K skew y); several objects give
    /// the hint "select one"; every other state (nothing selected, any other
    /// tool) gives "select first" and changes no tool.
    #[test]
    fn m_and_k_open_an_entry_or_explain_themselves() {
        for tool in TOOLS {
            for selected in SELECTIONS {
                for (k, shift, entry) in [
                    ("m", false, EntryKey::Move),
                    ("M", false, EntryKey::Move),
                    ("k", false, EntryKey::SkewX),
                    ("K", true, EntryKey::SkewY),
                    ("k", true, EntryKey::SkewY),
                ] {
                    let input = KeyInput {
                        key: k,
                        shift,
                        ..KeyInput::default()
                    };
                    let expected = match (tool, selected) {
                        (Tool::Select, Selected::One | Selected::Many) => {
                            KeyAction::OpenEntry(entry)
                        }
                        // M is the Measure tool everywhere else (`0070` criterion 2).
                        _ if entry == EntryKey::Move => KeyAction::SetTool(Tool::Measure),
                        _ => KeyAction::Hint(KeyHint::SelectFirst),
                    };
                    assert_eq!(
                        decide(&input, state(tool, selected)),
                        expected,
                        "{k} shift {shift} in {tool:?} with {selected:?}"
                    );
                }
            }
        }
        // Shift+M is nothing.
        let shifted_m = KeyInput {
            key: "M",
            shift: true,
            ..KeyInput::default()
        };
        assert_eq!(
            decide(&shifted_m, state(Tool::Select, Selected::One)),
            KeyAction::Ignore
        );
    }

    /// Shift is allowed for `*` (it is Shift+8 on many layouts).
    #[test]
    fn shift_is_allowed_for_star_only() {
        let st = state(Tool::Select, Selected::One);
        let shifted = |k| KeyInput {
            key: k,
            shift: true,
            ..KeyInput::default()
        };
        assert_eq!(
            decide(&shifted("*"), st),
            KeyAction::SetTool(Tool::PolygonStar)
        );
        for k in ["B", "N", "E", "R", "S", "Delete"] {
            assert_eq!(decide(&shifted(k), st), KeyAction::Ignore, "Shift+{k}");
        }
    }

    /// Criterion 55: Ctrl+R, Ctrl+S, Cmd+R (the frontend folds Cmd into
    /// `ctrl`) and Alt+E change no tool.
    #[test]
    fn modified_letters_are_ignored() {
        let st = state(Tool::Select, Selected::None);
        for (k, ctrl, alt) in [("r", true, false), ("s", true, false), ("e", false, true)] {
            let input = KeyInput {
                key: k,
                ctrl,
                alt,
                ..KeyInput::default()
            };
            assert_eq!(decide(&input, st), KeyAction::Ignore, "{k}");
        }
    }

    /// Escape, Enter, Space and the modifier keys are never gated by the
    /// list of criterion 55; Escape alone is gated by repeat (criterion 60).
    #[test]
    fn escape_and_enter_are_not_gated_by_the_list() {
        let mut busy = state(Tool::Pen, Selected::None);
        busy.operation_in_flight = true;
        busy.pen_open = true;
        busy.entry_open = true;
        let escape = KeyInput {
            key: "Escape",
            ctrl: true,
            alt: true,
            dom_blocked: true,
            ..KeyInput::default()
        };
        assert_eq!(decide(&escape, busy), KeyAction::Escape);
        assert_eq!(
            decide(
                &KeyInput {
                    repeat: true,
                    ..escape
                },
                busy
            ),
            KeyAction::Ignore,
            "criterion 60: a held Escape acts once"
        );
        assert_eq!(decide(&key("Enter"), busy), KeyAction::FinishPen);
        assert_eq!(
            decide(&key("Enter"), state(Tool::Pen, Selected::None)),
            KeyAction::Ignore,
            "no path to finish"
        );
        let mut select_with_path = state(Tool::Select, Selected::One);
        select_with_path.pen_open = true;
        assert_eq!(decide(&key("Enter"), select_with_path), KeyAction::Ignore);
    }

    /// The codes the frontend reads are distinct and only `ignored` means
    /// "do not prevent the default".
    #[test]
    fn outcome_codes_are_distinct() {
        let all = [
            KeyOutcome::Ignored,
            KeyOutcome::ToolChanged,
            KeyOutcome::EntryOpened,
            KeyOutcome::Deleted,
            KeyOutcome::PenFinished,
            KeyOutcome::Escape(EscapeStep::ClosedEntry),
            KeyOutcome::Escape(EscapeStep::CancelledDrag),
            KeyOutcome::Escape(EscapeStep::ClearedState),
            KeyOutcome::Escape(EscapeStep::LeftGroup),
            KeyOutcome::Group,
            KeyOutcome::Ungroup,
            KeyOutcome::EnteredGroup,
            KeyOutcome::Escape(EscapeStep::LeftTool),
            KeyOutcome::Escape(EscapeStep::Nothing),
            KeyOutcome::Hint(KeyHint::SelectFirst),
            KeyOutcome::Hint(KeyHint::TooFar),
        ];
        let mut codes: Vec<&str> = all.iter().map(|o| o.code()).collect();
        codes.sort_unstable();
        codes.dedup();
        assert_eq!(codes.len(), all.len());
        assert_eq!(KeyOutcome::Ignored.code(), "ignored");
    }

    const ARROWS: [(&str, Arrow); 4] = [
        ("ArrowLeft", Arrow::Left),
        ("ArrowRight", Arrow::Right),
        ("ArrowUp", Arrow::Up),
        ("ArrowDown", Arrow::Down),
    ];

    /// Ctrl and Cmd are one flag; the key is matched without regard to case.
    fn ctrl(k: &str) -> KeyInput<'_> {
        KeyInput {
            key: k,
            ctrl: true,
            ..KeyInput::default()
        }
    }

    /// `0044` criteria 1 and 3, amended by `0053` criterion 22: Ctrl+A selects all in the
    /// Select tool and the Node tool, whatever is selected, and is ignored in every other.
    #[test]
    fn ctrl_a_selects_all_in_the_select_and_node_tools_and_is_ignored_in_every_other() {
        for tool in TOOLS {
            for selected in SELECTIONS {
                for k in ["a", "A"] {
                    let expected = if matches!(tool, Tool::Select | Tool::Node) {
                        KeyAction::SelectAll
                    } else {
                        KeyAction::Ignore
                    };
                    assert_eq!(
                        decide(&ctrl(k), state(tool, selected)),
                        expected,
                        "{k} in {tool:?} with {selected:?}"
                    );
                }
            }
        }
    }

    /// `0044` criterion 3 and the gate: Shift, Alt, a repeat, a focused field, a drag, an open
    /// chip and an unfinished Pen path each block Ctrl+A; the plain letter binds nothing.
    #[test]
    fn every_gate_condition_blocks_ctrl_a() {
        let blockers: Vec<(&str, Blocker)> = vec![
            ("shift", Box::new(|i, _| i.shift = true)),
            ("alt", Box::new(|i, _| i.alt = true)),
            ("repeat", Box::new(|i, _| i.repeat = true)),
            ("dom", Box::new(|i, _| i.dom_blocked = true)),
            ("drag", Box::new(|_, s| s.operation_in_flight = true)),
            ("pen path", Box::new(|_, s| s.pen_open = true)),
            ("entry", Box::new(|_, s| s.entry_open = true)),
        ];
        for (name, block) in &blockers {
            let mut input = ctrl("a");
            let mut st = state(Tool::Select, Selected::Many);
            block(&mut input, &mut st);
            assert_eq!(decide(&input, st), KeyAction::Ignore, "{name}");
        }
        assert_eq!(
            decide(&key("a"), state(Tool::Select, Selected::Many)),
            KeyAction::Ignore
        );
    }

    /// `0044` criteria 7 and 11: an arrow nudges in the Select tool with a selection, with or
    /// without Shift, and on every repeat event; every other tool or an empty selection leaves
    /// the key alone.
    #[test]
    fn an_arrow_nudges_only_the_select_tool_with_a_selection() {
        for tool in TOOLS {
            for selected in SELECTIONS {
                for (k, arrow) in ARROWS {
                    for (shift, repeat) in
                        [(false, false), (true, false), (false, true), (true, true)]
                    {
                        let input = KeyInput {
                            key: k,
                            shift,
                            repeat,
                            ..KeyInput::default()
                        };
                        let expected = if tool == Tool::Select && selected != Selected::None {
                            KeyAction::Nudge(arrow)
                        } else {
                            KeyAction::Ignore
                        };
                        assert_eq!(
                            decide(&input, state(tool, selected)),
                            expected,
                            "{k} shift {shift} repeat {repeat} in {tool:?} with {selected:?}"
                        );
                    }
                }
            }
        }
    }

    /// `0044` criteria 7 and 11: Ctrl, Cmd or Alt, a focused field, a drag, an open chip and an
    /// unfinished Pen path each leave the arrow key to whoever else wants it.
    #[test]
    fn every_gate_condition_leaves_the_arrow_keys_alone() {
        let blockers: Vec<(&str, Blocker)> = vec![
            ("ctrl", Box::new(|i, _| i.ctrl = true)),
            ("alt", Box::new(|i, _| i.alt = true)),
            ("dom", Box::new(|i, _| i.dom_blocked = true)),
            ("drag", Box::new(|_, s| s.operation_in_flight = true)),
            ("pen path", Box::new(|_, s| s.pen_open = true)),
            ("entry", Box::new(|_, s| s.entry_open = true)),
        ];
        for (k, _) in ARROWS {
            for (name, block) in &blockers {
                let mut input = key(k);
                let mut st = state(Tool::Select, Selected::Many);
                block(&mut input, &mut st);
                assert_eq!(decide(&input, st), KeyAction::Ignore, "{k} with {name}");
            }
        }
    }

    /// The letters still ignore a repeat; only the arrows accept one.
    #[test]
    fn the_letters_still_ignore_a_repeat() {
        let repeated = KeyInput {
            key: "r",
            repeat: true,
            ..KeyInput::default()
        };
        assert_eq!(
            decide(&repeated, state(Tool::Select, Selected::One)),
            KeyAction::Ignore
        );
    }
}
