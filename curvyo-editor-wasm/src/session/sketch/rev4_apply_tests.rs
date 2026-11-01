//! Constraint and dimension tools return to Select after they apply (0066 AC43), through
//! pointer events on the session (AC41).
use super::rev3_constraint_tests::{C1, L1, L2, P1, P2, P3, added, click, kinds, session};
use super::*;
use curvyo_sketch_core::EntityId;
use std::collections::BTreeSet;

/// Arms `tool` with nothing selected, picks `targets` and checks: one step, the kinds added,
/// Select, and the picked geometry selected.
fn armed_applies(tool: &str, targets: &[(f64, f64)], expected: &[&str]) {
    let mut s = session();
    let steps = s.undo_len();
    let before = kinds(&s);
    assert!(s.sketch_rail_press(tool).unwrap(), "{tool}: arms");
    assert_eq!(s.sketch.tool, SketchTool::Constraints, "{tool}: armed");
    let mut picked = BTreeSet::<EntityId>::new();
    for at in targets {
        let before_pick = s.sketch.entities.clone();
        click(&mut s, *at, false);
        picked.extend(
            s.sketch
                .entities
                .difference(&before_pick)
                .cloned()
                .collect::<Vec<_>>(),
        );
    }
    assert_eq!(s.undo_len(), steps + 1, "{tool}: one step");
    let mut want: Vec<String> = expected.iter().map(|k| (*k).to_owned()).collect();
    want.sort();
    assert_eq!(added(&before, &kinds(&s)), want, "{tool}");
    assert_eq!(s.sketch.tool, SketchTool::Select, "{tool}: back in Select");
    assert_eq!(s.sketch.constraint_tool, None, "{tool}");
    assert!(
        !s.sketch.entities.is_empty(),
        "{tool}: its geometry is selected"
    );
}

#[test]
fn an_armed_constraint_tool_returns_to_select_after_its_last_pick() {
    armed_applies("coincident", &[P1, P2], &["coincident"]);
    armed_applies("horizontal", &[L1], &["horizontal"]);
    armed_applies("vertical", &[P1, P2], &["vertical"]);
    armed_applies("parallel", &[L1, L2], &["parallel"]);
    armed_applies("perpendicular", &[L1, L2], &["perpendicular"]);
    armed_applies("tangent", &[L1, C1], &["tangent"]);
    armed_applies("equal", &[L1, L2], &["equal"]);
    armed_applies("fixed", &[P1], &["fixed"]);
    armed_applies("symmetric", &[P1, P2, P3], &["symmetric"]);
    armed_applies("midpoint", &[P1, L1], &["midpoint"]);
}

#[test]
fn a_refused_use_leaves_the_tool_armed() {
    for (tool, start) in [("midpoint", C1), ("parallel", P1), ("horizontal", C1)] {
        let mut s = session();
        click(&mut s, start, false);
        let steps = s.undo_len();
        assert!(!s.sketch_rail_press(tool).unwrap(), "{tool}: refused");
        assert_eq!(s.undo_len(), steps, "{tool}: no step");
        assert_eq!(
            s.sketch.tool,
            SketchTool::Constraints,
            "{tool}: still armed"
        );
        assert_eq!(s.sketch.constraint_tool.as_deref(), Some(tool));
    }
}

#[test]
fn a_dimension_placed_and_typed_returns_to_select_with_its_geometry() {
    let mut s = session();
    let steps = s.undo_len();
    assert!(s.sketch_rail_press("dimension").unwrap());
    click(&mut s, L1, false);
    let line = s.sketch.entities.clone();
    assert_eq!(line.len(), 1);
    // The label is placed on the canvas, then the value is typed into its box.
    click(&mut s, (60.0, 10.0), false);
    assert!(s.sketch.dimension_position.is_some());
    assert!(s.sketch_apply_action("length", "35 mm", false).unwrap());
    assert_eq!(s.undo_len(), steps + 1);
    assert_eq!(s.sketch.tool, SketchTool::Select);
    assert_eq!(s.sketch.entities, line);
    assert_eq!(s.sketch.canvas_edit, None, "no edit state left open");
}

#[test]
fn a_preselected_dimension_returns_to_select_with_its_entry_open() {
    let mut s = session();
    click(&mut s, L1, false);
    let line = s.sketch.entities.clone();
    let steps = s.undo_len();
    assert!(s.sketch_rail_press("dimension").unwrap());
    assert_eq!(s.undo_len(), steps + 1);
    assert_eq!(s.sketch.tool, SketchTool::Select);
    assert_eq!(s.sketch.entities, line);
    assert!(s.sketch.canvas_edit.is_some(), "its value entry is open");
}
