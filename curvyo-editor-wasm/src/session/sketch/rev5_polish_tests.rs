//! Customer polish of revision 5 (2026-11-01): a hovered, picked or selected local axis is
//! highlighted, and the entry box shows at most three decimals while an untyped Enter keeps the
//! full value.
use super::rev3_constraint_tests::{P1, P2, X_AXIS, Y_AXIS, click, global, session};
use super::*;
use curvyo_render_core::SketchAxisState::{Hovered, Idle, Selected};
use curvyo_sketch_core::{ConstraintId, Expression};

fn hover(s: &mut Session, at: (f64, f64)) {
    let p = global(s, at);
    s.pointer_hover(p, false, false);
}

/// The literal value of the dimension `id`.
fn literal(s: &Session, id: &ConstraintId) -> f64 {
    let snapshot = s.document.sketch(s.active_sketch().unwrap()).unwrap();
    match snapshot.graph.constraints[id].relation.expression() {
        Some(Expression::Literal(q)) => q.value(),
        other => panic!("{other:?}"),
    }
}

#[test]
fn an_axis_under_the_pointer_or_selected_is_highlighted() {
    let mut s = session();
    s.set_sketch_tool(SketchTool::Select);
    hover(&mut s, (200.0, 200.0));
    assert_eq!(s.sketch_axis_states(), [Idle, Idle]);
    hover(&mut s, X_AXIS);
    assert_eq!(
        s.sketch_axis_states(),
        [Hovered, Idle],
        "hovering the X axis"
    );
    click(&mut s, Y_AXIS, false);
    assert_eq!(
        s.sketch_axis_states(),
        [Idle, Selected],
        "the Y axis selected"
    );
    hover(&mut s, X_AXIS);
    assert_eq!(s.sketch_axis_states(), [Hovered, Selected]);
    // Geometry under the pointer wins over an axis: P1 is no axis.
    hover(&mut s, P1);
    assert_eq!(s.sketch_axis_states(), [Idle, Selected]);
}

#[test]
fn an_armed_constraint_tool_highlights_the_axis_it_would_pick() {
    let mut s = session();
    s.set_sketch_tool(SketchTool::Select);
    s.sketch_rail_press("symmetric").unwrap();
    assert_eq!(s.sketch.tool, SketchTool::Constraints, "armed");
    click(&mut s, P1, false);
    click(&mut s, P2, false);
    hover(&mut s, Y_AXIS);
    assert_eq!(
        s.sketch_axis_states(),
        [Idle, Hovered],
        "the axis it would pick"
    );
}

#[test]
fn the_dimension_edit_shows_three_decimals_and_an_untyped_enter_keeps_the_value() {
    let mut s = session();
    click(&mut s, P1, false);
    click(&mut s, P2, true);
    assert!(s.sketch_rail_press("dimension").unwrap());
    let entry = s.sketch_view()["canvas"]["entry"].clone();
    assert_eq!(entry["kind"], "edit", "{entry}");
    assert_eq!(entry["fields"][0]["value"], "15.811", "{entry}");
    let id: ConstraintId = entry["constraint"].as_str().unwrap().into();
    let full = f64::hypot(15.0, 5.0);
    assert!((literal(&s, &id) - full).abs() < 1e-9);
    // Enter on the shown text keeps the measured value; nothing is rounded into the sketch.
    s.sketch_dimension_text(&id, "15.811", false).unwrap();
    assert!(
        (literal(&s, &id) - full).abs() < 1e-9,
        "{}",
        literal(&s, &id)
    );
    // A typed value is taken as typed, with all its decimals.
    assert!(s.sketch_edit_dimension(&id));
    s.sketch_dimension_text(&id, "15.81139", false).unwrap();
    assert!((literal(&s, &id) - 15.811_39).abs() < 1e-12);
}

#[test]
fn a_placed_dimension_shows_three_decimals_and_an_untyped_enter_keeps_the_value() {
    let mut s = Session::new(66_930);
    s.set_workbench(Workbench::Sketch);
    s.set_sketch_tool(SketchTool::Circle);
    for at in [(10.0, 10.0), (20.0, 13.0)] {
        click(&mut s, at, false);
    }
    let circle = s.sketch_view()["entities"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["kind"] == "circle")
        .map(|e| e["id"].as_str().unwrap().to_owned())
        .unwrap();
    s.set_sketch_tool(SketchTool::Select);
    s.escape();
    assert!(s.sketch_rail_press("dimension").unwrap_or(false));
    assert!(s.sketch_select_entity(&circle.as_str().into(), false));
    click(&mut s, (40.0, 40.0), false);
    let entry = s.sketch_view()["canvas"]["entry"].clone();
    assert_eq!(entry["kind"], "dimension", "{entry}");
    let action = entry["action"].as_str().unwrap().to_owned();
    let radius = f64::hypot(10.0, 3.0);
    let expected = if action == "diameter" {
        2.0 * radius
    } else {
        radius
    };
    let shown = entry["fields"][0]["value"].as_str().unwrap().to_owned();
    assert_eq!(shown, format!("{expected:.3}"), "{action}");
    let before = s
        .document
        .sketch(s.active_sketch().unwrap())
        .unwrap()
        .graph
        .constraints
        .keys()
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();
    s.sketch_apply_action(&action, &shown, false).unwrap();
    let snapshot = s.document.sketch(s.active_sketch().unwrap()).unwrap();
    let id = snapshot
        .graph
        .constraints
        .keys()
        .find(|id| !before.contains(*id))
        .unwrap()
        .clone();
    assert!(
        (literal(&s, &id) - expected).abs() < 1e-9,
        "{action}: {}",
        literal(&s, &id)
    );
}
