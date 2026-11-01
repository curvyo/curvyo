//! Horizontal and Vertical distance tools, two-point selection and the dimension kinds of a
//! two-point selection (0066 AC44-AC48), through pointer events on the session (AC41).
use super::rev3_constraint_tests::{L1, P1, P2, P3, X_AXIS, Y_AXIS, click, global, kinds, session};
use super::*;
use curvyo_sketch_core::{ConstraintId, EntityId, Relation, SolveStatus};
use std::collections::BTreeSet;

fn snapshot(s: &Session) -> curvyo_document_core::SketchSnapshot {
    s.document.sketch(s.active_sketch().unwrap()).unwrap()
}

fn dof(s: &Session) -> usize {
    match snapshot(s).status {
        Some(SolveStatus::UnderConstrained(n)) => n,
        Some(SolveStatus::FullyConstrained) => 0,
        other => panic!("{other:?}"),
    }
}

fn at(s: &Session, id: &EntityId) -> (f64, f64) {
    let p = snapshot(s).solution.point(id).unwrap();
    (p.x.0, p.y.0)
}

/// The two selected points, in click order of `targets`.
fn select_points(s: &mut Session, targets: &[(f64, f64)]) -> Vec<EntityId> {
    let mut ids = Vec::new();
    for (i, target) in targets.iter().enumerate() {
        let before = s.sketch.entities.clone();
        click(s, *target, i > 0);
        ids.extend(s.sketch.entities.difference(&before).cloned());
    }
    ids
}

/// The relation of the constraint the last application added.
fn newest(s: &Session, before: &BTreeSet<ConstraintId>) -> (ConstraintId, Relation) {
    snapshot(s)
        .graph
        .constraints
        .into_iter()
        .find(|(id, _)| !before.contains(id))
        .map(|(id, c)| (id, c.relation))
        .unwrap()
}

fn constraint_ids(s: &Session) -> BTreeSet<ConstraintId> {
    snapshot(s).graph.constraints.keys().cloned().collect()
}

fn value_mm(relation: &Relation) -> f64 {
    let Some(curvyo_sketch_core::Expression::Literal(q)) = relation.expression() else {
        panic!("{relation:?}");
    };
    q.value()
}

#[test]
fn the_constraints_card_lists_horizontal_and_vertical_distance_after_dimension() {
    let s = session();
    let rail = s.sketch_view()["rail"].clone();
    let names: Vec<_> = rail
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["name"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(
        &names[names.len() - 3..],
        ["dimension", "horizontal-distance", "vertical-distance"]
    );
    for (name, label) in [
        ("horizontal-distance", "Horizontal distance"),
        ("vertical-distance", "Vertical distance"),
    ] {
        let tool = rail
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["name"] == name)
            .unwrap();
        assert_eq!(tool["label"], label);
        assert!(tool["tooltip"].as_str().unwrap().starts_with(label));
    }
}

/// AC48: Dimension, Horizontal distance and Vertical distance on the same two points, each at
/// once in one step, back in Select with the two points still selected (AC43).
#[test]
fn two_points_take_the_aligned_horizontal_and_vertical_distance() {
    let mut s = session();
    let points = select_points(&mut s, &[P1, P2]);
    assert_eq!(points.len(), 2);
    let (dx, dy) = (P2.0 - P1.0, P2.1 - P1.1);
    for (tool, kind, value) in [
        ("dimension", "distance", dx.hypot(dy)),
        ("horizontal-distance", "horizontal-distance", dx),
        ("vertical-distance", "vertical-distance", dy),
    ] {
        let before = constraint_ids(&s);
        let steps = s.undo_len();
        assert!(s.sketch_rail_press(tool).unwrap(), "{tool}");
        assert_eq!(s.undo_len(), steps + 1, "{tool}: one step");
        let (_, relation) = newest(&s, &before);
        assert_eq!(super::view::relation_name(&relation), kind, "{tool}");
        assert!(
            (value_mm(&relation) - value).abs() < 1e-6,
            "{tool}: {relation:?}"
        );
        assert_eq!(s.sketch_tool(), SketchTool::Select, "{tool}");
        assert_eq!(
            s.sketch.entities,
            points.iter().cloned().collect(),
            "{tool}: the two points stay selected"
        );
        s.sketch_close_dimension_edit();
    }
}

#[test]
fn a_line_gets_its_projection_lengths_at_once() {
    let mut s = session();
    click(&mut s, L1, false);
    let line = s.sketch.entities.clone();
    for (tool, value) in [("horizontal-distance", 30.0), ("vertical-distance", 5.0)] {
        let before = constraint_ids(&s);
        assert!(s.sketch_rail_press(tool).unwrap(), "{tool}");
        let (_, relation) = newest(&s, &before);
        assert!(
            (value_mm(&relation) - value).abs() < 1e-6,
            "{tool}: {relation:?}"
        );
        assert_eq!(s.sketch.entities, line, "{tool}: the line stays selected");
        s.sketch_close_dimension_edit();
    }
}

/// AC45: 30 mm on the diagonal between two points: a 30 mm horizontal projection, the
/// vertical component unchanged, one degree of freedom less; the annotation names the kind.
#[test]
fn a_typed_horizontal_distance_drives_only_the_x_projection() {
    let mut s = session();
    let points = select_points(&mut s, &[P1, P2]);
    let free = dof(&s);
    let before = constraint_ids(&s);
    assert!(s.sketch_rail_press("horizontal-distance").unwrap());
    let (id, _) = newest(&s, &before);
    assert_eq!(dof(&s), free - 1, "one degree of freedom less");
    assert_eq!(
        s.sketch.canvas_edit.as_ref(),
        Some(&id),
        "its entry is open"
    );
    let (a0, b0) = (at(&s, &points[0]), at(&s, &points[1]));
    assert!(s.sketch_dimension_text(&id, "30 mm", false).unwrap());
    let (a, b) = (at(&s, &points[0]), at(&s, &points[1]));
    assert!((b.0 - a.0 - 30.0).abs() < 1e-6, "{a:?} {b:?}");
    assert!(
        ((b.1 - a.1) - (b0.1 - a0.1)).abs() < 1e-6,
        "vertical component unchanged"
    );
    let annotation = s.sketch_view()["annotations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["id"] == id.0.as_str())
        .cloned()
        .unwrap();
    assert_eq!(annotation["text"], "↔ 30 mm");
    assert!(
        annotation["name"]
            .as_str()
            .unwrap()
            .starts_with("Horizontal distance")
    );
    s.undo();
    assert!(
        (at(&s, &points[1]).0 - b0.0).abs() < 1e-6,
        "undo restores the value"
    );
}

/// AC46: the side at creation is kept, zero aligns, a negative value is refused with the entry
/// kept.
#[test]
fn the_side_is_kept_zero_aligns_and_a_negative_value_is_refused() {
    let mut s = session();
    // P2 is right of P1; picked first, it still stays on the right.
    let points = select_points(&mut s, &[P2, P1]);
    let before = constraint_ids(&s);
    assert!(s.sketch_rail_press("horizontal-distance").unwrap());
    let (id, _) = newest(&s, &before);
    assert!(s.sketch_dimension_text(&id, "8", false).unwrap());
    let (right, left) = (at(&s, &points[0]), at(&s, &points[1]));
    assert!((right.0 - left.0 - 8.0).abs() < 1e-6, "{right:?} {left:?}");
    assert!(s.sketch_dimension_text(&id, "0 mm", false).unwrap());
    let (right, left) = (at(&s, &points[0]), at(&s, &points[1]));
    assert!(
        (right.0 - left.0).abs() < 1e-6,
        "zero aligns the points vertically"
    );
    assert!(s.sketch_edit_dimension(&id), "the entry opens again");
    let steps = s.undo_len();
    assert!(s.sketch_dimension_text(&id, "-5 mm", false).is_err());
    assert_eq!(s.undo_len(), steps, "a negative value adds no step");
    assert!(
        s.sketch.entry_error.is_some(),
        "the entry stays with its reason"
    );
    assert!(s.sketch_dimension_text(&id, "2 *", false).is_err());
}

#[test]
fn a_point_and_the_matching_axis_measure_its_coordinate() {
    let mut s = session();
    let before = constraint_ids(&s);
    select_points(&mut s, &[P1, Y_AXIS]);
    assert!(s.sketch_rail_press("horizontal-distance").unwrap());
    let (_, relation) = newest(&s, &before);
    assert!((value_mm(&relation) - P1.0).abs() < 1e-6, "{relation:?}");
    let mut s = session();
    let before = constraint_ids(&s);
    select_points(&mut s, &[X_AXIS, P1]);
    assert!(s.sketch_rail_press("vertical-distance").unwrap());
    let (_, relation) = newest(&s, &before);
    assert!((value_mm(&relation) - P1.1).abs() < 1e-6, "{relation:?}");
}

/// AC48: a selection that does not fit applies nothing and says what the tool needs; the tool
/// waits (AC43).
#[test]
fn three_points_apply_nothing_and_name_what_the_tool_needs() {
    let mut s = session();
    select_points(&mut s, &[P1, P2, P3]);
    let (steps, count) = (s.undo_len(), kinds(&s).len());
    assert!(!s.sketch_rail_press("vertical-distance").unwrap());
    assert_eq!((s.undo_len(), kinds(&s).len()), (steps, count));
    assert_eq!(
        s.sketch.constraint_tool.as_deref(),
        Some("vertical-distance")
    );
    s.pointer_hover(global(&s, (80.0, 110.0)), false, false);
    assert_eq!(
        s.sketch_view()["canvas"]["mode_label"]["error"],
        "Vertical distance: select two points, one line, or a point and the X axis"
    );
}

/// An armed Horizontal distance takes two point picks and returns to Select (AC43, AC44).
#[test]
fn an_armed_axis_distance_measures_on_its_last_pick() {
    let mut s = session();
    let steps = s.undo_len();
    assert!(s.sketch_rail_press("horizontal-distance").unwrap());
    click(&mut s, P1, false);
    click(&mut s, P2, false);
    assert_eq!(s.undo_len(), steps + 1);
    assert!(kinds(&s).contains(&"horizontal-distance".to_owned()));
    assert_eq!(s.sketch_tool(), SketchTool::Select);
    assert_eq!(s.sketch.entities.len(), 2);
}

/// AC47: Shift-click adds a point, a marquee takes the points inside it, Ctrl removes.
#[test]
fn two_points_are_selected_by_shift_click_and_by_marquee() {
    let mut s = session();
    let points = select_points(&mut s, &[P1, P2]);
    assert_eq!(points.len(), 2);
    assert_eq!(s.sketch.entities, points.iter().cloned().collect());

    let mut s = session();
    let from = global(&s, (55.0, 55.0));
    let to = global(&s, (80.0, 70.0));
    let (left, right) = if from.x <= to.x {
        (from, to)
    } else {
        (to, from)
    };
    s.pointer_hover(left, false, false);
    s.pointer_down(left, false);
    s.pointer_hover(right, false, true);
    s.pointer_up(right, false, false);
    assert_eq!(
        s.sketch.entities,
        points.iter().cloned().collect(),
        "the marquee pair"
    );
    // Ctrl-click removes one of them.
    let p = global(&s, P2);
    s.modifiers_changed(false, true, false);
    s.pointer_hover(p, false, true);
    s.pointer_down(p, false);
    s.pointer_up(p, false, false);
    s.modifiers_changed(false, false, false);
    assert_eq!(s.sketch.entities.len(), 1);
}

/// AC46: a reference axis distance follows a drag past the other point; made driving at the
/// value it shows, the dragged point keeps the side the drag gave it.
#[test]
fn a_reference_axis_distance_made_driving_keeps_the_dragged_side() {
    let mut s = session();
    let points = select_points(&mut s, &[P1, P2]);
    let before = constraint_ids(&s);
    assert!(s.sketch_rail_press("horizontal-distance").unwrap());
    let (id, _) = newest(&s, &before);
    assert!(s.sketch_dimension_text(&id, "15", true).unwrap());
    s.sketch_close_dimension_edit();
    s.sketch.entities.clear();
    let (from, to) = (global(&s, P2), global(&s, (45.0, P2.1)));
    s.pointer_hover(from, false, false);
    s.pointer_down(from, false);
    s.pointer_hover(to, false, true);
    s.pointer_up(to, false, false);
    let (a, b) = (at(&s, &points[0]), at(&s, &points[1]));
    assert!(b.0 < a.0, "the drag put b left of a: {a:?} {b:?}");
    let shown = a.0 - b.0;
    s.sketch_dimension_text(&id, &format!("{shown}"), false)
        .unwrap();
    let (a, b) = (at(&s, &points[0]), at(&s, &points[1]));
    assert!(b.0 < a.0, "b stays left of a: {a:?} {b:?}");
    assert!((a.0 - b.0 - shown).abs() < 1e-6, "{a:?} {b:?}");
}
