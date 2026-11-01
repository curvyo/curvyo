//! Sketch keyboard routing protects vector tools from analytic editing shortcuts.
use super::{Session, SketchTool};
use crate::session::{EscapeStep, KeyInput, KeyOutcome};

impl Session {
    /// Cancels an unfinished Sketch action on a secondary press without committing it.
    pub fn sketch_secondary_press(&mut self) -> bool {
        if self.workbench() != super::Workbench::Sketch {
            return false;
        }
        let pending = self.sketch.press.is_some()
            || self.sketch.drag.is_some()
            || self.sketch.preview.is_some()
            || self.sketch.dimension_position.is_some()
            || self.sketch.marquee.is_some()
            || self.sketch.lasso.is_some()
            || !self.sketch.picks.is_empty();
        self.sketch.cancel();
        self.button_down = false;
        pending
    }

    /// The Sketch Escape cascade: an open chain ends in Select with the chain (AC42); a pending
    /// step is cancelled; otherwise the tool returns to Select (AC20).
    pub(in crate::session) fn sketch_escape(&mut self) -> EscapeStep {
        // Escape ends an open Polyline chain: back in Select with the chain (AC42).
        if self.sketch_end_chain() {
            self.end_snapping_drag();
            self.button_down = false;
            return EscapeStep::ClearedState;
        }
        let pending = self.sketch.drag.is_some()
            || self.sketch.preview.is_some()
            || self.sketch.dimension_position.is_some()
            || self.sketch.marquee.is_some()
            || self.sketch.lasso.is_some()
            || !self.sketch.picks.is_empty()
            || self.sketch.edit_constraint.is_some()
            || self.sketch.label_drag.is_some()
            || self.sketch.canvas_edit.is_some()
            || self.button_down;
        self.sketch_revert_refused();
        self.sketch.cancel();
        if !pending {
            // A drawing tool returns to Select with the previous selection (AC20, AC42);
            // Select and an armed constraint tool clear it.
            let drawing = !matches!(
                self.sketch.tool,
                SketchTool::Select | SketchTool::Constraints
            );
            self.sketch.tool = SketchTool::Select;
            self.sketch.constraint_tool = None;
            if !drawing {
                self.sketch.entities.clear();
                self.sketch.constraints.clear();
            }
        }
        self.end_snapping_drag();
        self.button_down = false;
        EscapeStep::ClearedState
    }

    pub(in crate::session) fn sketch_key(&mut self, input: KeyInput<'_>) -> KeyOutcome {
        if input.dom_blocked {
            return KeyOutcome::Ignored;
        }
        if input.ctrl {
            if input.key.eq_ignore_ascii_case("z") || input.key.eq_ignore_ascii_case("y") {
                let redo = input.key.eq_ignore_ascii_case("y") || input.shift;
                return self.history_key(if redo {
                    curvyo_document_core::Direction::Redo
                } else {
                    curvyo_document_core::Direction::Undo
                });
            }
            if input.key.eq_ignore_ascii_case("a") {
                if let Ok(snapshot) = self.sketch_snapshot_for_edit() {
                    self.sketch.entities = snapshot
                        .graph
                        .entities
                        .keys()
                        .filter(|id| **id != snapshot.graph.origin)
                        .cloned()
                        .collect();
                }
                return KeyOutcome::SelectedAll;
            }
            return KeyOutcome::Ignored;
        }
        match input.key {
            // Alt alone, once per press: never with Shift, Ctrl or Cmd (AltGr), never repeated.
            "Alt" if !input.repeat && !input.shift => {
                if self.cycle_sketch_mode() {
                    KeyOutcome::Consumed
                } else {
                    KeyOutcome::Ignored
                }
            }
            "Escape" => KeyOutcome::Escape(self.escape()),
            "Enter" => {
                if let Some(id) = self.sketch.constraints.iter().next().cloned() {
                    self.sketch_inspect_constraint(&id, true);
                    return KeyOutcome::Consumed;
                }
                // Enter finishes an open Polyline chain: back in Select with the chain (AC42).
                if self.sketch_end_chain() {
                    return KeyOutcome::Consumed;
                }
                self.sketch.cancel();
                self.end_snapping_drag();
                KeyOutcome::Consumed
            }
            "Delete" | "Backspace" => {
                let result = self.sketch_delete_selection();
                self.sketch.error = result.err().map(|e| e.to_string());
                KeyOutcome::Deleted
            }
            "v" | "V" => {
                self.set_sketch_tool(SketchTool::Select);
                KeyOutcome::ToolChanged
            }
            _ => KeyOutcome::Ignored,
        }
    }
}
