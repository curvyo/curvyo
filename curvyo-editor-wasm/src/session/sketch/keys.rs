//! Sketch keyboard routing protects vector tools from analytic editing shortcuts.
use super::{Session, SketchTool};
use crate::session::{KeyInput, KeyOutcome};

impl Session {
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
            "Escape" => KeyOutcome::Escape(self.escape()),
            "Enter" => {
                if let Some(id) = self.sketch.constraints.iter().next().cloned() {
                    self.sketch_inspect_constraint(&id, true);
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
