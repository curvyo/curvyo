//! The Constraints rail card: each tool's state for the selection, and what a press does (AC24).
use super::state::PickStart;
use super::{Session, SketchTool};
use curvyo_document_core::SketchEditError;
use curvyo_sketch_core::EntityId;
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
                        SketchToolUse::Arm => (
                            "arm",
                            pick_rule(tool).map_or_else(
                                || label.to_owned(),
                                |rule| format!("{label}: {rule}"),
                            ),
                        ),
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
    /// kept, back in Select, AC43), arms it keeping a valid start as its first picks, or arms it
    /// without picks and names on the canvas what it needs (AC40).
    /// # Errors
    /// Reports a refused constraint without a history step.
    pub fn sketch_rail_press(&mut self, tool: &str) -> Result<bool, SketchEditError> {
        self.sync_sketch();
        let graph = self.sketch_snapshot_for_edit()?.graph;
        let selected = self.sketch.entities.iter().cloned().collect::<Vec<_>>();
        match curvyo_ui_core::sketch_tool_use(tool, &graph, &selected) {
            SketchToolUse::Apply if tool == "dimension" => self.sketch_dimension_selection(None),
            SketchToolUse::Apply if axis_distance(tool) => {
                self.sketch_dimension_selection(Some(tool))
            }
            SketchToolUse::Apply => {
                if self.sketch.constraint_tool.as_deref() == Some("dimension") {
                    self.set_sketch_tool(SketchTool::Select);
                }
                self.sketch.entities = selected.iter().cloned().collect();
                let applied = self.sketch_apply_action(tool, "", false)?;
                self.sketch_applied_select(selected.into_iter().collect());
                Ok(applied)
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
    /// typed right away (AC40, lead decision 2026-10-31), back in Select (AC43).
    /// `kind` names a Horizontal or Vertical distance tool; `None` is the smart Dimension.
    fn sketch_dimension_selection(&mut self, kind: Option<&str>) -> Result<bool, SketchEditError> {
        let snapshot = self.sketch_snapshot_for_edit()?;
        let kind = match kind {
            Some(kind) => kind,
            None => self
                .sketch_dimension_action()
                .ok_or_else(|| SketchEditError(super::relation_tools::DIMENSION_NEEDS.into()))?,
        };
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
        let mut measured = curvyo_sketch_core::measure(&snapshot.solution, &relation)
            .map_err(|e| SketchEditError(e.to_string()))?;
        // An axis distance keeps the side the solved geometry has: the value is its size.
        if measured.value() < 0.0
            && let curvyo_sketch_core::Relation::HorizontalDistance { a, b, .. }
            | curvyo_sketch_core::Relation::VerticalDistance { a, b, .. } = &mut relation
        {
            std::mem::swap(a, b);
            measured = curvyo_sketch_core::Quantity::length(-measured.value())
                .map_err(|e| SketchEditError(e.to_string()))?;
        }
        if let Some(value) = relation.expression_mut() {
            *value = curvyo_sketch_core::Expression::Literal(measured);
        }
        let id: curvyo_sketch_core::ConstraintId = self.minter.mint().to_hex().into();
        let changed = self.sketch_set_constraint(
            id.clone(),
            curvyo_sketch_core::Constraint::driving(relation),
        )?;
        self.sketch_applied_select(selected.iter().cloned().collect());
        self.sketch_edit_dimension(&id);
        // The entry selects the dimension's points; the geometry picked for it stays (AC43).
        self.sketch.entities = selected.into_iter().collect();
        Ok(changed)
    }

    /// An armed constraint tool's pick of `entity`: a refused selection is replaced, a valid
    /// start is completed (AC40); the pick toggles and the tool applies once the picks fit. A
    /// pick after which the picks can no longer fit is refused with the tool's hint (AC52).
    pub(super) fn sketch_armed_pick(&mut self, entity: &EntityId) {
        self.sketch.constraint_hint = None;
        let start = self.sketch.pick_start.take();
        if start == Some(PickStart::Replaced) {
            self.sketch.entities.clear();
        }
        let before = self.sketch.entities.clone();
        if !self.sketch.entities.insert(entity.clone()) {
            self.sketch.entities.remove(entity);
        }
        if start == Some(PickStart::Kept) && self.sketch_complete_dimension_start() {
            return;
        }
        if self.sketch_symmetric_about_last_pick(entity) {
            return;
        }
        if let Some(reason) = self.sketch_armed_refusal() {
            self.sketch.entities = before;
            self.sketch.constraint_hint = Some(reason);
            return;
        }
        self.sketch_apply_armed();
    }

    /// Armed Symmetric with three picked points: the last pick is the centre, the other two are
    /// mirrored about it, as `FreeCAD` does (0066 AC51); one step, back in Select. `false` when the
    /// picks are not three points ending on `last`.
    fn sketch_symmetric_about_last_pick(&mut self, last: &EntityId) -> bool {
        if self.sketch.constraint_tool.as_deref() != Some("symmetric") {
            return false;
        }
        let Ok(snapshot) = self.sketch_snapshot_for_edit() else {
            return false;
        };
        let picks = self.sketch.entities.iter().cloned().collect::<Vec<_>>();
        let Some(relation) = curvyo_ui_core::sketch_symmetric_about(&snapshot.graph, &picks, last)
        else {
            return false;
        };
        let id: curvyo_sketch_core::ConstraintId = self.minter.mint().to_hex().into();
        let result =
            self.sketch_set_constraint(id, curvyo_sketch_core::Constraint::driving(relation));
        self.sketch.error = result.as_ref().err().map(ToString::to_string);
        if result.is_ok() {
            self.sketch_applied_select(picks.into_iter().collect());
        }
        true
    }

    /// Why the armed tool's picks can no longer become a valid target, as the canvas hint
    /// (AC40, AC52); `None` while they fit or may still fit. The Dimension tool collects its own.
    fn sketch_armed_refusal(&self) -> Option<String> {
        let tool = self.sketch.constraint_tool.as_deref()?;
        if tool == "dimension" {
            return None;
        }
        let graph = self.sketch_snapshot_for_edit().ok()?.graph;
        let picks = self.sketch.entities.iter().cloned().collect::<Vec<_>>();
        match curvyo_ui_core::sketch_tool_use(tool, &graph, &picks) {
            SketchToolUse::Unavailable(reason) => {
                Some(format!("{}: {}", label(tool), lowercase_first(&reason)))
            }
            SketchToolUse::Apply | SketchToolUse::Arm => None,
        }
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
        let _ = self.sketch_dimension_selection(None);
        true
    }

    /// With an armed constraint tool, applies it once the picks are complete, then returns to
    /// Select with the picks selected (AC43).
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
        if axis_distance(&tool) {
            // Measured at once with its entry open, as a complete preselection (AC44).
            let _ = self.sketch_dimension_selection(Some(&tool));
            return;
        }
        if self.sketch_apply_action(&tool, "", false).is_ok() {
            self.sketch_applied_select(picks.into_iter().collect());
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
        "horizontal-distance" => "Horizontal distance",
        "vertical-distance" => "Vertical distance",
        _ => "Dimension",
    }
}

/// What an armed tool's picks mean beyond its name, said in its tooltip and on the canvas: the
/// last picked point is the Symmetric centre (AC51).
pub(super) fn pick_rule(tool: &str) -> Option<&'static str> {
    (tool == "symmetric").then_some("picked one by one, the last point is the centre")
}

/// Whether `tool` is the Horizontal or the Vertical distance tool (AC44).
fn axis_distance(tool: &str) -> bool {
    matches!(tool, "horizontal-distance" | "vertical-distance")
}

fn lowercase_first(text: &str) -> String {
    let mut chars = text.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_lowercase().chain(chars).collect()
    })
}
