//! The Constraints rail card: each tool's state for the selection, and what a press does (AC24).
use super::state::PickStart;
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
    /// kept), arms it keeping a valid start as its first picks, or arms it without picks and
    /// names on the canvas what it needs (AC40).
    /// # Errors
    /// Reports a refused constraint without a history step.
    pub fn sketch_rail_press(&mut self, tool: &str) -> Result<bool, SketchEditError> {
        self.sync_sketch();
        let graph = self.sketch_snapshot_for_edit()?.graph;
        let selected = self.sketch.entities.iter().cloned().collect::<Vec<_>>();
        match curvyo_ui_core::sketch_tool_use(tool, &graph, &selected) {
            SketchToolUse::Apply if tool == "dimension" => self.sketch_dimension_selection(),
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
                // A valid start is not yet a valid target: it stays as the first picks and the
                // canvas says what the tool still needs (AC40).
                if !kept.is_empty()
                    && let Err(reason) =
                        curvyo_ui_core::sketch_relations(tool, &graph, &selected, None)
                {
                    self.sketch.constraint_hint =
                        Some(format!("{}: {}", label(tool), lowercase_first(&reason)));
                }
                self.sketch.pick_start = (!kept.is_empty()).then_some(PickStart::Kept);
                self.sketch.entities = kept;
                Ok(armed)
            }
            // The selection cannot become eligible: nothing is applied, no step is added; the
            // tool waits for fresh picks and says on the canvas what it needs (AC40). The
            // selection stays until the first pick replaces it.
            SketchToolUse::Unavailable(reason) => {
                let kept = self.sketch.entities.clone();
                self.set_sketch_constraint_tool(tool);
                self.sketch.entities = kept;
                self.sketch.pick_start = Some(PickStart::Replaced);
                self.sketch.constraint_hint =
                    Some(format!("{}: {}", label(tool), lowercase_first(&reason)));
                Ok(false)
            }
        }
    }

    /// Dimensions the selection at once with its measured value, as a driving dimension at the
    /// default label place, in one history step; the entry box opens on it so the value can be
    /// typed right away (AC40, lead decision 2026-10-31). The tool state is unchanged.
    fn sketch_dimension_selection(&mut self) -> Result<bool, SketchEditError> {
        let snapshot = self.sketch_snapshot_for_edit()?;
        let kind = self
            .sketch_dimension_action()
            .ok_or_else(|| SketchEditError(super::relation_tools::DIMENSION_NEEDS.into()))?;
        let selected = self.sketch.entities.iter().cloned().collect::<Vec<_>>();
        let unit = if kind == "angle" {
            curvyo_sketch_core::Quantity::angle(1.0)
        } else {
            curvyo_sketch_core::Quantity::length(1.0)
        }
        .map_err(|e| SketchEditError(e.to_string()))?;
        let probe = curvyo_sketch_core::Expression::Literal(unit);
        let mut relation =
            curvyo_ui_core::sketch_relation(kind, &snapshot.graph, &selected, Some(&probe))
                .map_err(SketchEditError)?;
        let measured = curvyo_sketch_core::measure(&snapshot.solution, &relation)
            .map_err(|e| SketchEditError(e.to_string()))?;
        if let Some(value) = relation.expression_mut() {
            *value = curvyo_sketch_core::Expression::Literal(measured);
        }
        let id: curvyo_sketch_core::ConstraintId = self.minter.mint().to_hex().into();
        let changed = self.sketch_set_constraint(
            id.clone(),
            curvyo_sketch_core::Constraint::driving(relation),
        )?;
        self.sketch_edit_dimension(&id);
        Ok(changed)
    }

    /// An armed constraint tool's pick of `entity`: a refused selection is replaced, a valid
    /// start is completed (AC40); the pick toggles and the tool applies once the picks fit.
    pub(super) fn sketch_armed_pick(&mut self, entity: &curvyo_sketch_core::EntityId) {
        self.sketch.constraint_hint = None;
        let start = self.sketch.pick_start.take();
        if start == Some(PickStart::Replaced) {
            self.sketch.entities.clear();
        }
        if !self.sketch.entities.insert(entity.clone()) {
            self.sketch.entities.remove(entity);
        }
        if start == Some(PickStart::Kept) && self.sketch_complete_dimension_start() {
            return;
        }
        self.sketch_apply_armed();
    }

    /// A Dimension press with one selected point, completed by a pick: dimensioned at once as
    /// if the whole selection had been made first, back in Select (AC40). `false` when the
    /// picks do not fit a dimension yet.
    fn sketch_complete_dimension_start(&mut self) -> bool {
        if self.sketch.constraint_tool.as_deref() != Some("dimension") {
            return false;
        }
        let Ok(snapshot) = self.sketch_snapshot_for_edit() else {
            return false;
        };
        let picks = self.sketch.entities.iter().cloned().collect::<Vec<_>>();
        if curvyo_ui_core::sketch_tool_use("dimension", &snapshot.graph, &picks)
            != SketchToolUse::Apply
        {
            return false;
        }
        let picks = std::mem::take(&mut self.sketch.entities);
        self.set_sketch_tool(SketchTool::Select);
        self.sketch.entities = picks;
        // A refusal is shown by `sketch_dimension_selection` itself (`sketch.error`).
        let _ = self.sketch_dimension_selection();
        true
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

pub(super) fn label(tool: &str) -> &'static str {
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
