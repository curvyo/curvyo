//! The Constraints rail card: each tool's state for the selection, and what a press does (AC24).
use super::{Session, SketchTool};
use curvyo_document_core::SketchEditError;
use curvyo_ui_core::{SKETCH_RAIL_TOOLS, SketchToolUse};
use serde_json::{Value, json};

impl Session {
    /// One entry per rail tool: name, label, `apply` / `arm` / `unavailable`, tooltip, pressed.
    pub(super) fn sketch_rail(&self) -> Value {
        let graph = self.sketch_snapshot_for_edit().ok().map(|s| s.graph);
        let selected = self.sketch.entities.iter().cloned().collect::<Vec<_>>();
        json!(
            SKETCH_RAIL_TOOLS
                .iter()
                .map(|tool| {
                    let state = graph.as_ref().map_or_else(
                        || SketchToolUse::Unavailable("Open a sketch first".into()),
                        |graph| curvyo_ui_core::sketch_tool_use(tool, graph, &selected),
                    );
                    let label = label(tool);
                    let (name, tooltip) = match &state {
                        SketchToolUse::Apply => ("apply", label.to_owned()),
                        SketchToolUse::Arm => ("arm", label.to_owned()),
                        SketchToolUse::Unavailable(reason) => (
                            "unavailable",
                            format!("{label}: {}", lowercase_first(reason)),
                        ),
                    };
                    json!({
                        "name": tool,
                        "label": label,
                        "state": name,
                        "tooltip": tooltip,
                        "pressed": self.sketch.tool == SketchTool::Constraints
                            && self.sketch.constraint_tool.as_deref() == Some(*tool),
                    })
                })
                .collect::<Vec<_>>()
        )
    }

    /// Presses a rail tool: applies it to an eligible selection (one history step, selection
    /// kept), otherwise arms it keeping the selection as its first picks; an unavailable tool
    /// does nothing.
    /// # Errors
    /// Reports a refused constraint without a history step.
    pub fn sketch_rail_press(&mut self, tool: &str) -> Result<bool, SketchEditError> {
        self.sync_sketch();
        let graph = self.sketch_snapshot_for_edit()?.graph;
        let selected = self.sketch.entities.iter().cloned().collect::<Vec<_>>();
        match curvyo_ui_core::sketch_tool_use(tool, &graph, &selected) {
            SketchToolUse::Apply => {
                if self.sketch.constraint_tool.as_deref() == Some("dimension") {
                    self.set_sketch_tool(SketchTool::Select);
                }
                self.sketch.entities = selected.into_iter().collect();
                self.sketch_apply_action(tool, "", false)
            }
            SketchToolUse::Arm => {
                let kept = self.sketch.entities.clone();
                let armed = self.set_sketch_constraint_tool(tool);
                self.sketch.entities = kept;
                Ok(armed)
            }
            SketchToolUse::Unavailable(_) => Ok(false),
        }
    }

    /// With an armed constraint tool, applies it once the picks are complete, then clears the
    /// picks and stays armed.
    pub(super) fn sketch_apply_armed(&mut self) {
        let Some(tool) = self.sketch.constraint_tool.clone() else {
            return;
        };
        if tool == "dimension" {
            return;
        }
        let Ok(snapshot) = self.sketch_snapshot_for_edit() else {
            return;
        };
        let picks = self.sketch.entities.iter().cloned().collect::<Vec<_>>();
        if curvyo_ui_core::sketch_tool_use(&tool, &snapshot.graph, &picks) != SketchToolUse::Apply {
            return;
        }
        if self.sketch_apply_action(&tool, "", false).is_ok() {
            self.sketch.entities.clear();
        }
    }
}

fn label(tool: &str) -> &'static str {
    match tool {
        "coincident" => "Coincident",
        "horizontal" => "Horizontal",
        "vertical" => "Vertical",
        "parallel" => "Parallel",
        "perpendicular" => "Perpendicular",
        "tangent" => "Tangent",
        "equal" => "Equal",
        "fixed" => "Fixed",
        "symmetric" => "Symmetric",
        "midpoint" => "Midpoint",
        _ => "Dimension",
    }
}

fn lowercase_first(text: &str) -> String {
    let mut chars = text.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_lowercase().chain(chars).collect()
    })
}
