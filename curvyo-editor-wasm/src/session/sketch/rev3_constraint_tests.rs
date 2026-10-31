//! Selection-first constraints: each valid selection applies in one step, invalid ones wait for
//! picks; Coincident and Symmetric targets (0066 AC37, AC40, AC41).
use super::*;
use curvyo_document_core::Point;
use curvyo_sketch_core::{Construction, Entity, EntityId, Length, Position};

/// Clickable places of the test geometry (local millimetres).
const P1: (f64, f64) = (60.0, 60.0);
const P2: (f64, f64) = (75.0, 65.0);
const P3: (f64, f64) = (70.0, 80.0);
const L1: (f64, f64) = (25.0, 22.5);
const L2: (f64, f64) = (25.0, 45.0);
const L3: (f64, f64) = (25.0, 66.0);
const C1: (f64, f64) = (108.0, 30.0);
const C2: (f64, f64) = (136.0, 30.0);
const A1: (f64, f64) = (110.0, 70.0);
const E1: (f64, f64) = (155.0, 80.0);
const L4: (f64, f64) = (20.0, 90.0);
const A2: (f64, f64) = (40.0, 95.0);
const X_AXIS: (f64, f64) = (70.0, 0.0);
const Y_AXIS: (f64, f64) = (0.0, 70.0);

fn global(s: &Session, at: (f64, f64)) -> Point {
    s.sketch_placement()
        .unwrap()
        .apply_point(Point::new(at.0, at.1))
}

fn click(s: &mut Session, at: (f64, f64), shift: bool) {
    let p = global(s, at);
    s.pointer_hover(p, shift, false);
    s.pointer_down(p, shift);
    s.pointer_up(p, shift, false);
}

fn session() -> Session {
    let mut s = Session::new(66_920);
    s.set_workbench(Workbench::Sketch);
    let p = |x: f64, y: f64| Position::mm(x, y);
    let mut commands = vec![
        Construction::Point(p(P1.0, P1.1)),
        Construction::Point(p(P2.0, P2.1)),
        Construction::Point(p(P3.0, P3.1)),
        Construction::Polyline(vec![p(10.0, 20.0), p(40.0, 25.0)]),
        Construction::Polyline(vec![p(10.0, 40.0), p(40.0, 50.0)]),
        Construction::Polyline(vec![p(10.0, 60.0), p(40.0, 72.0)]),
        Construction::Circle {
            center: p(100.0, 30.0),
            radius: Length(8.0),
        },
        Construction::Circle {
            center: p(130.0, 30.0),
            radius: Length(6.0),
        },
        Construction::Arc([p(100.0, 60.0), p(110.0, 70.0), p(100.0, 80.0)]),
        Construction::Ellipse {
            center: p(140.0, 80.0),
            axis_end: p(155.0, 80.0),
            second_axis: Length(6.0),
        },
        Construction::Polyline(vec![p(10.0, 90.0), p(30.0, 90.0)]),
    ];
    for command in commands.drain(..) {
        s.sketch_construct(&command).unwrap();
    }
    // A2 continues from the end of L4: two curves meeting at a shared endpoint.
    let graph = s.document.sketch(s.active_sketch().unwrap()).unwrap().graph;
    let start = graph
        .entities
        .iter()
        .find_map(|(id, e)| match e {
            Entity::Point { position, .. } if position.distance(p(30.0, 90.0)).0 < 1e-9 => {
                Some(id.clone())
            }
            _ => None,
        })
        .unwrap();
    s.sketch_construct(&Construction::ArcFrom {
        start,
        through: p(40.0, 95.0),
        end: p(45.0, 105.0),
    })
    .unwrap();
    s.set_sketch_tool(SketchTool::Select);
    s
}

fn kinds(s: &Session) -> Vec<String> {
    let mut kinds: Vec<String> = s.sketch_view()["constraints"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["kind"].as_str().unwrap().to_owned())
        .collect();
    kinds.sort();
    kinds
}

fn added(before: &[String], after: &[String]) -> Vec<String> {
    let mut rest = after.to_vec();
    for kind in before {
        if let Some(i) = rest.iter().position(|k| k == kind) {
            rest.remove(i);
        }
    }
    rest
}

/// Selects `targets` by clicks (Shift after the first), presses `tool` on the rail and checks
/// the added kinds, one history step, the kept selection and tool, and undo.
fn applies(tool: &str, targets: &[(f64, f64)], expected: &[&str]) {
    let mut s = session();
    for (i, at) in targets.iter().enumerate() {
        click(&mut s, *at, i > 0);
    }
    let selected = s.sketch.entities.clone();
    assert_eq!(
        selected.len(),
        targets.len(),
        "{tool} {targets:?}: selection"
    );
    let before = kinds(&s);
    let steps = s.undo_len();
    assert!(
        s.sketch_rail_press(tool).unwrap(),
        "{tool} {targets:?} applies"
    );
    assert_eq!(s.undo_len(), steps + 1, "{tool} {targets:?}: one step");
    let mut want: Vec<String> = expected.iter().map(|k| (*k).to_owned()).collect();
    want.sort();
    assert_eq!(added(&before, &kinds(&s)), want, "{tool} {targets:?}");
    assert_eq!(s.sketch.entities, selected, "{tool}: selection unchanged");
    assert_eq!(
        s.sketch_tool(),
        SketchTool::Select,
        "{tool}: tool unchanged"
    );
    let snapshot = s.document.sketch(s.active_sketch().unwrap()).unwrap();
    assert!(
        snapshot.conflicting.is_empty()
            && snapshot.status != Some(curvyo_sketch_core::SolveStatus::SolverFailure),
        "{tool} {targets:?}: {:?}",
        snapshot.status
    );
    s.undo();
    assert_eq!(kinds(&s), before, "{tool} {targets:?}: undo");
}

#[test]
fn coincident_takes_a_point_with_a_point_line_axis_circle_arc_or_ellipse() {
    applies("coincident", &[P1, P2], &["coincident"]);
    applies("coincident", &[P1, L1], &["point-on-line"]);
    applies("coincident", &[L1, P1], &["point-on-line"]);
    applies("coincident", &[P1, X_AXIS], &["on-x-axis"]);
    applies("coincident", &[Y_AXIS, P1], &["on-y-axis"]);
    applies("coincident", &[P1, C1], &["point-on-circle"]);
    applies("coincident", &[P1, A1], &["point-on-arc"]);
    applies("coincident", &[P3, E1], &["point-on-ellipse"]);
}

#[test]
fn symmetric_takes_two_points_about_a_point_a_line_or_an_axis() {
    applies("symmetric", &[P1, P2, P3], &["symmetric"]);
    applies("symmetric", &[P1, P2, L1], &["symmetric"]);
    applies("symmetric", &[P1, P2, X_AXIS], &["symmetric"]);
    applies("symmetric", &[Y_AXIS, P1, P2], &["symmetric"]);
}

#[test]
fn horizontal_vertical_parallel_perpendicular_and_midpoint() {
    applies("horizontal", &[L1], &["horizontal"]);
    applies("horizontal", &[P1, P2], &["horizontal"]);
    applies("vertical", &[L1], &["vertical"]);
    applies("vertical", &[P1, P2], &["vertical"]);
    applies("parallel", &[L1, L2], &["parallel"]);
    applies("parallel", &[L1, L2, L3], &["parallel", "parallel"]);
    applies("perpendicular", &[L1, L2], &["perpendicular"]);
    applies("midpoint", &[P1, L1], &["midpoint"]);
}

#[test]
fn tangent_equal_and_fixed() {
    applies("tangent", &[L1, C1], &["tangent"]);
    applies("tangent", &[L2, A1], &["tangent"]);
    applies("tangent", &[C1, C2], &["tangent"]);
    applies("tangent", &[A1, C2], &["tangent"]);
    applies("tangent", &[L3, E1], &["tangent"]);
    applies("tangent", &[L4, A2], &["tangent"]);
    applies("equal", &[L1, L2], &["equal"]);
    applies("equal", &[L1, L2, L3], &["equal", "equal"]);
    applies("equal", &[C1, C2], &["equal"]);
    applies("equal", &[C1, A1], &["equal"]);
    applies("fixed", &[P1], &["fixed"]);
    applies("fixed", &[P1, L1, C1], &["fixed", "fixed", "fixed"]);
}

#[test]
fn the_point_on_symbol_names_its_kind() {
    let mut s = session();
    click(&mut s, P1, false);
    click(&mut s, L1, true);
    s.sketch_rail_press("coincident").unwrap();
    let names: Vec<String> = s.sketch_view()["symbols"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| b["name"].as_str().unwrap().to_owned())
        .collect();
    assert!(
        names
            .iter()
            .any(|n| n == "Coincident, point on line constraint"),
        "{names:?}"
    );
}

#[test]
fn a_dimension_preselection_applies_the_measured_value_at_once_and_opens_its_entry() {
    for (targets, kind, label) in [
        (vec![L1], "length", "Length"),
        (vec![P1, P2], "distance", "Distance"),
        (vec![C1], "diameter", "Diameter"),
        (vec![A1], "radius", "Radius"),
        (vec![L1, L2], "angle", "Angle"),
    ] {
        let mut s = session();
        for (i, at) in targets.iter().enumerate() {
            click(&mut s, *at, i > 0);
        }
        let selected = s.sketch.entities.clone();
        let (steps, count) = (s.undo_len(), kinds(&s).len());
        assert!(s.sketch_rail_press("dimension").unwrap(), "{label}");
        assert_eq!(s.undo_len(), steps + 1, "{label}: one history step");
        assert_eq!(kinds(&s).len(), count + 1, "{label}");
        assert!(kinds(&s).contains(&kind.to_owned()), "{label}");
        assert_eq!(
            s.sketch.tool,
            SketchTool::Select,
            "{label}: the tool is unchanged"
        );
        assert_eq!(s.sketch.entities, selected, "{label}: nothing new selected");
        let view = s.sketch_view();
        let entry = &view["canvas"]["entry"];
        assert_eq!(entry["kind"], "edit", "{label}: {entry}");
        assert_eq!(entry["fields"][0]["label"], label);
        // The measured value: the geometry does not move.
        let status = s.document.sketch(s.active_sketch().unwrap()).unwrap();
        assert!(
            status.conflicting.is_empty() && status.redundant.is_empty(),
            "{label}"
        );
        // The value typed into the open box replaces it.
        let id = s.sketch.canvas_edit.clone().unwrap();
        let value = if label == "Angle" { "30" } else { "12" };
        assert!(
            s.sketch_dimension_text(&id, value, false).unwrap(),
            "{label}"
        );
        s.undo();
        s.undo();
        assert_eq!(kinds(&s).len(), count, "{label}: undo removes it");
    }
}

#[test]
fn an_incomplete_dimension_selection_arms_the_tool_for_more_picks() {
    let mut s = session();
    click(&mut s, P1, false);
    let steps = s.undo_len();
    s.sketch_rail_press("dimension").unwrap();
    assert_eq!(s.undo_len(), steps);
    assert_eq!(s.sketch.tool, SketchTool::Constraints);
    assert_eq!(s.sketch.constraint_tool.as_deref(), Some("dimension"));
    let mut empty = session();
    empty.sketch_rail_press("dimension").unwrap();
    assert_eq!(empty.sketch.constraint_tool.as_deref(), Some("dimension"));
}

#[test]
fn an_invalid_selection_applies_nothing_then_takes_fresh_picks() {
    let mut s = session();
    click(&mut s, C1, false);
    let steps = s.undo_len();
    assert!(!s.sketch_rail_press("midpoint").unwrap());
    assert_eq!(s.undo_len(), steps);
    s.pointer_hover(global(&s, (80.0, 110.0)), false, false);
    assert_eq!(
        s.sketch_view()["canvas"]["mode_label"]["error"],
        "Midpoint: select one point and one segment"
    );
    // The armed tool takes fresh picks; the circle is not one of them.
    click(&mut s, P1, false);
    click(&mut s, L1, false);
    assert_eq!(s.undo_len(), steps + 1);
    assert!(kinds(&s).contains(&"midpoint".to_owned()));
    let selected: Vec<EntityId> = s.sketch.entities.iter().cloned().collect();
    assert!(
        selected.is_empty(),
        "the armed tool clears its picks after applying"
    );
}

/// Selects `start`, presses `tool` (it arms: the start is valid but incomplete), clicks
/// `completion` and checks that the start was kept: the tool applies in one step.
fn completes(tool: &str, start: &[(f64, f64)], completion: (f64, f64), expected: &[&str]) {
    let mut s = session();
    for (i, at) in start.iter().enumerate() {
        click(&mut s, *at, i > 0);
    }
    let before = kinds(&s);
    let steps = s.undo_len();
    s.sketch_rail_press(tool).unwrap();
    assert_eq!(
        s.undo_len(),
        steps,
        "{tool} {start:?}: an incomplete start arms"
    );
    click(&mut s, completion, false);
    assert_eq!(
        s.undo_len(),
        steps + 1,
        "{tool} {start:?} + {completion:?}: one step"
    );
    let mut want: Vec<String> = expected.iter().map(|k| (*k).to_owned()).collect();
    want.sort();
    assert_eq!(added(&before, &kinds(&s)), want, "{tool} {start:?}");
    s.undo();
    assert_eq!(kinds(&s), before, "{tool} {start:?}: undo");
}

#[test]
fn a_valid_start_stays_as_the_first_picks_of_every_constraint_tool() {
    completes("coincident", &[P1], P2, &["coincident"]);
    completes("coincident", &[P1], L1, &["point-on-line"]);
    completes("coincident", &[P1], X_AXIS, &["on-x-axis"]);
    completes("horizontal", &[P1], P2, &["horizontal"]);
    completes("vertical", &[P1], P2, &["vertical"]);
    completes("parallel", &[L1], L2, &["parallel"]);
    completes("perpendicular", &[L1], L2, &["perpendicular"]);
    completes("tangent", &[L1], C1, &["tangent"]);
    completes("equal", &[C1], C2, &["equal"]);
    completes("midpoint", &[P1], L1, &["midpoint"]);
    completes("symmetric", &[P1, P2], P3, &["symmetric"]);
    completes("symmetric", &[P1, P2], Y_AXIS, &["symmetric"]);
}

#[test]
fn one_selected_point_then_dimension_then_a_second_point_dimensions_at_once() {
    let mut s = session();
    click(&mut s, P1, false);
    let steps = s.undo_len();
    s.sketch_rail_press("dimension").unwrap();
    assert_eq!(s.undo_len(), steps);
    click(&mut s, P2, false);
    assert_eq!(
        s.undo_len(),
        steps + 1,
        "the second point completes the pair"
    );
    assert!(kinds(&s).contains(&"distance".to_owned()));
    let entry = &s.sketch_view()["canvas"]["entry"];
    assert_eq!(entry["kind"], "edit", "{entry}");
    assert_eq!(entry["fields"][0]["label"], "Distance");
    assert_eq!(
        s.sketch.tool,
        SketchTool::Select,
        "as a complete preselection"
    );
}

/// Annotation of the dimension `id` in the view.
fn annotation(s: &Session, id: &curvyo_sketch_core::ConstraintId) -> serde_json::Value {
    s.sketch_view()["annotations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["id"] == id.0.as_str())
        .cloned()
        .unwrap()
}

#[test]
fn a_typed_value_that_conflicts_keeps_the_box_open_with_the_reason() {
    for double_click in [false, true] {
        let mut s = session();
        click(&mut s, L1, false);
        s.sketch_rail_press("dimension").unwrap();
        let first = s.sketch.canvas_edit.clone().unwrap();
        click(&mut s, L1, false);
        s.sketch_rail_press("dimension").unwrap();
        let second = s.sketch.canvas_edit.clone().unwrap();
        assert_ne!(first, second);
        if double_click {
            s.sketch_close_dimension_edit();
            assert!(s.sketch_edit_dimension(&second));
        }
        let steps = s.undo_len();
        assert!(s.sketch_dimension_text(&second, "25 mm", false).unwrap());
        let view = s.sketch_view();
        let entry = &view["canvas"]["entry"];
        assert_eq!(
            entry["constraint"],
            second.0.as_str(),
            "{double_click}: {entry}"
        );
        assert!(entry["error"].is_object(), "{double_click}: {entry}");
        // The label shows the value the dimension asks for, as its box does.
        let text = annotation(&s, &second)["text"].as_str().unwrap().to_owned();
        assert_eq!(text, "! 25 mm", "{double_click}");
        // Escape in the box cancels the refused value; undo then removes the second dimension.
        s.escape();
        assert_eq!(
            s.undo_len(),
            steps,
            "{double_click}: Escape takes the value back"
        );
        s.undo();
        assert!(
            s.sketch_view()["annotations"]
                .as_array()
                .unwrap()
                .iter()
                .all(|a| !a["text"].as_str().unwrap().starts_with('!')),
            "{double_click}"
        );
    }
}
