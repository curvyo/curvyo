//! Customer revision 5 through pointer and key events on the session (0066 AC41): points win
//! over edges (AC50), the Symmetric centre (AC51), Symmetric about an axis or a line in both
//! flows (AC52) and the annotation of an edited dimension (AC49).
use super::rev3_constraint_tests::{
    C1, L1, L2, P1, P2, P3, X_AXIS, Y_AXIS, click, global, kinds, session,
};
use super::*;
use curvyo_document_core::Point;
use curvyo_sketch_core::{ConstraintId, Construction, EntityId, Position, Relation, SolveStatus};
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

fn position(s: &Session, id: &EntityId) -> (f64, f64) {
    let p = snapshot(s).solution.point(id).unwrap();
    (p.x.0, p.y.0)
}

fn constraint_ids(s: &Session) -> BTreeSet<ConstraintId> {
    snapshot(s).graph.constraints.keys().cloned().collect()
}

fn added_relations(s: &Session, before: &BTreeSet<ConstraintId>) -> Vec<Relation> {
    snapshot(s)
        .graph
        .constraints
        .into_iter()
        .filter(|(id, _)| !before.contains(id))
        .map(|(_, c)| c.relation)
        .collect()
}

/// The entity a plain click at `at` selects.
fn picked(s: &mut Session, at: (f64, f64)) -> EntityId {
    click(s, at, false);
    assert_eq!(s.sketch.entities.len(), 1, "{at:?}");
    s.sketch.entities.iter().next().unwrap().clone()
}

/// Zooms about the canvas origin with Ctrl+wheel until the view is near `percent`.
fn zoom_to(s: &mut Session, percent: i64) {
    for _ in 0..400 {
        let now = s.zoom_percent();
        if (now - percent).abs() * 20 <= percent {
            return;
        }
        s.wheel(
            0.0,
            if now < percent { -20.0 } else { 20.0 },
            0.0,
            0.0,
            false,
            true,
        );
    }
    panic!("zoom {} for {percent}", s.zoom_percent());
}

/// A place `along` screen px back from the end of L1 toward its start and `across` px off it.
fn near_l1_end(s: &Session, along: f64, across: f64) -> (f64, f64) {
    let px = 1.0 / s.view().scale();
    let (start, end) = ((10.0, 20.0), (40.0, 25.0));
    let (dx, dy) = (end.0 - start.0, end.1 - start.1);
    let length = f64::hypot(dx, dy);
    let (ux, uy) = (dx / length, dy / length);
    (
        end.0 - ux * along * px - uy * across * px,
        end.1 - uy * along * px + ux * across * px,
    )
}

/// AC50: 6 px from the end point and 1 px from the line picks the point; 14 px from it picks
/// the line; at 25 % and 400 %.
#[test]
fn a_point_within_ten_pixels_wins_over_a_nearer_edge_at_every_zoom() {
    for percent in [25, 100, 400] {
        let mut s = session();
        zoom_to(&mut s, percent);
        let end = picked(&mut s, (40.0, 25.0));
        let line = picked(&mut s, L1);
        click(&mut s, (200.0, 200.0), false);
        let near = near_l1_end(&s, (36.0_f64 - 1.0).sqrt(), 1.0);
        assert_eq!(
            picked(&mut s, near),
            end,
            "{percent} %: 6 px from the point"
        );
        click(&mut s, (200.0, 200.0), false);
        let far = near_l1_end(&s, (196.0_f64 - 1.0).sqrt(), 1.0);
        assert_eq!(
            picked(&mut s, far),
            line,
            "{percent} %: 14 px from the point"
        );
    }
}

/// AC50: with two points inside 10 px, the nearer one is picked.
#[test]
fn of_two_points_within_reach_the_nearer_wins() {
    let mut s = session();
    zoom_to(&mut s, 25);
    let p1 = picked(&mut s, P1);
    let p2 = picked(&mut s, P2);
    let px = 1.0 / s.view().scale();
    let (dx, dy) = (P1.0 - P2.0, P1.1 - P2.1);
    let length = f64::hypot(dx, dy);
    assert!(
        length / px < 19.0,
        "both within 10 px of a place between them"
    );
    let at =
        |from: (f64, f64), d: f64| (from.0 + dx / length * d * px, from.1 + dy / length * d * px);
    assert_eq!(picked(&mut s, at(P2, length / px - 6.0)), p1);
    assert_eq!(picked(&mut s, at(P2, 6.0)), p2);
}

/// Alt-click, as a lasso without movement, at one place.
fn alt_click(s: &mut Session, at: (f64, f64)) {
    let p = global(s, at);
    s.modifiers_changed(false, false, true);
    s.pointer_hover(p, false, false);
    s.pointer_down(p, false);
    s.pointer_up(p, false, false);
    s.modifiers_changed(false, false, false);
}

/// AC50 with AC34: Alt-click at the same place cycles from the point to the edge under it.
#[test]
fn alt_cycling_still_reaches_the_edge_under_a_winning_point() {
    let mut s = session();
    let end = picked(&mut s, (40.0, 25.0));
    let line = picked(&mut s, L1);
    let near = near_l1_end(&s, 5.0, 1.0);
    alt_click(&mut s, near);
    assert_eq!(s.sketch.entities, [end].into());
    alt_click(&mut s, near);
    assert_eq!(s.sketch.entities, [line].into());
}

/// A session with three collinear free points A(150, 120), B(160, 120), C(154, 120).
fn three_points() -> (Session, [EntityId; 3]) {
    let mut s = session();
    let places = [(150.0, 120.0), (160.0, 120.0), (154.0, 120.0)];
    for (x, y) in places {
        s.sketch_construct(&Construction::Point(Position::mm(x, y)))
            .unwrap();
    }
    s.set_sketch_tool(SketchTool::Select);
    let ids = places.map(|at| picked(&mut s, at));
    click(&mut s, (200.0, 200.0), false);
    (s, ids)
}

fn centre(relations: &[Relation]) -> EntityId {
    let [Relation::SymmetricPoint { center, .. }] = relations else {
        panic!("{relations:?}");
    };
    center.clone()
}

/// AC51: selected first in any of the six orders, the middle point is the centre.
#[test]
fn three_selected_points_take_the_middle_one_as_centre_in_every_order() {
    let places = [(150.0, 120.0), (160.0, 120.0), (154.0, 120.0)];
    for order in [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ] {
        let (mut s, ids) = three_points();
        for (i, k) in order.iter().enumerate() {
            click(&mut s, places[*k], i > 0);
        }
        let before = constraint_ids(&s);
        let steps = s.undo_len();
        assert!(s.sketch_rail_press("symmetric").unwrap(), "{order:?}");
        assert_eq!(centre(&added_relations(&s, &before)), ids[2], "{order:?}");
        assert_eq!(s.undo_len(), steps + 1);
        assert_eq!(s.sketch_tool(), SketchTool::Select);
    }
}

/// AC51: picked one by one with the armed tool, the last pick is the centre.
#[test]
fn rail_picks_take_the_last_point_as_centre() {
    let places = [(150.0, 120.0), (160.0, 120.0), (154.0, 120.0)];
    for (order, want) in [([0, 1, 2], 2), ([2, 0, 1], 1)] {
        let (mut s, ids) = three_points();
        assert!(s.sketch_rail_press("symmetric").unwrap());
        let before = constraint_ids(&s);
        let steps = s.undo_len();
        for k in order {
            click(&mut s, places[k], false);
        }
        assert_eq!(
            centre(&added_relations(&s, &before)),
            ids[want],
            "{order:?}"
        );
        assert_eq!(s.undo_len(), steps + 1, "{order:?}: one step");
        assert_eq!(s.sketch_tool(), SketchTool::Select, "{order:?}");
    }
}

/// AC51: three points equally near the middle of the others are refused with the reason.
#[test]
fn an_ambiguous_centre_is_refused_with_a_hint() {
    let mut s = session();
    let places = [
        (150.0, 120.0),
        (160.0, 120.0),
        (155.0, 120.0 + 75.0_f64.sqrt()),
    ];
    for (x, y) in places {
        s.sketch_construct(&Construction::Point(Position::mm(x, y)))
            .unwrap();
    }
    s.set_sketch_tool(SketchTool::Select);
    for (i, at) in places.iter().enumerate() {
        click(&mut s, *at, i > 0);
    }
    let before = constraint_ids(&s);
    assert!(!s.sketch_rail_press("symmetric").unwrap());
    assert_eq!(constraint_ids(&s), before);
    let hint = s.sketch.constraint_hint.clone().unwrap();
    assert!(
        hint.starts_with("Symmetric: the centre is ambiguous"),
        "{hint}"
    );
}

/// The mirror image of `p` about the line through `a` and `b`.
fn mirrored(p: (f64, f64), a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let t = ((p.0 - a.0) * dx + (p.1 - a.1) * dy) / (dx * dx + dy * dy);
    let foot = (a.0 + t * dx, a.1 + t * dy);
    (2.0 * foot.0 - p.0, 2.0 * foot.1 - p.1)
}

fn line_ends(s: &Session, line: &EntityId) -> ((f64, f64), (f64, f64)) {
    let graph = snapshot(s).graph;
    let curvyo_sketch_core::Entity::Line { start, end } = &graph.entities[line] else {
        panic!("{line:?}");
    };
    (position(s, start), position(s, end))
}

/// Drags the point at `from` by (8, -5) mm with the pointer in Select.
fn drag_point(s: &mut Session, from: (f64, f64)) {
    let to = (from.0 + 8.0, from.1 - 5.0);
    s.pointer_hover(global(s, from), false, false);
    s.pointer_down(global(s, from), false);
    for i in 1..=4 {
        let f = f64::from(i) / 4.0;
        let p = (from.0 + (to.0 - from.0) * f, from.1 + (to.1 - from.1) * f);
        s.pointer_hover(global(s, p), false, false);
    }
    s.pointer_up(global(s, to), false, false);
}

/// One AC52 case: two points and the mirror target, by selection first or by rail picks.
#[allow(clippy::many_single_char_names)] // p, q, a, b: the mirrored pair before and after.
fn symmetric_about(target: (f64, f64), construction: bool, rail: bool, target_first: bool) {
    let mut s = session();
    if construction {
        click(&mut s, target, false);
        s.sketch_selection_construction(true).unwrap();
        click(&mut s, (200.0, 200.0), false);
    }
    let source = picked(&mut s, P1);
    let image = picked(&mut s, P2);
    let mirror = if target == X_AXIS || target == Y_AXIS {
        None
    } else {
        Some(picked(&mut s, target))
    };
    click(&mut s, (200.0, 200.0), false);
    let before = constraint_ids(&s);
    let (steps, free) = (s.undo_len(), dof(&s));
    let picks: Vec<(f64, f64)> = if target_first {
        vec![target, P1, P2]
    } else {
        vec![P1, P2, target]
    };
    let label = format!("{target:?} construction {construction} rail {rail} first {target_first}");
    if rail {
        assert!(s.sketch_rail_press("symmetric").unwrap(), "{label}: arms");
        for at in &picks {
            click(&mut s, *at, false);
        }
    } else {
        for (i, at) in picks.iter().enumerate() {
            click(&mut s, *at, i > 0);
        }
        assert!(
            s.sketch_rail_press("symmetric").unwrap(),
            "{label}: applies"
        );
    }
    let relations = added_relations(&s, &before);
    match (&relations[..], &mirror) {
        ([Relation::SymmetricAxis { .. }], None) | ([Relation::Symmetric { .. }], Some(_)) => {}
        _ => panic!("{label}: {relations:?}"),
    }
    assert_eq!(s.undo_len(), steps + 1, "{label}: one step");
    assert_eq!(
        s.sketch_tool(),
        SketchTool::Select,
        "{label}: back in Select"
    );
    assert_eq!(dof(&s), free - 2, "{label}: DOF");
    let (Relation::SymmetricAxis {
        source: a,
        image: b,
        ..
    }
    | Relation::Symmetric {
        source: a,
        image: b,
        ..
    }) = &relations[0]
    else {
        unreachable!();
    };
    assert_eq!(
        BTreeSet::from([a.clone(), b.clone()]),
        BTreeSet::from([source.clone(), image.clone()])
    );
    click(&mut s, (200.0, 200.0), false);
    let start = position(&s, &source);
    drag_point(&mut s, start);
    let (p, q) = (position(&s, a), position(&s, b));
    let (m0, m1) = match &mirror {
        None if target == Y_AXIS => ((0.0, 0.0), (0.0, 1.0)),
        None => ((0.0, 0.0), (1.0, 0.0)),
        Some(line) => line_ends(&s, line),
    };
    let image_of_p = mirrored(p, m0, m1);
    assert!(
        (image_of_p.0 - q.0).abs() <= 1e-6 && (image_of_p.1 - q.1).abs() <= 1e-6,
        "{label}: {p:?} mirrors to {image_of_p:?}, not {q:?}"
    );
    s.undo();
    s.undo();
    assert_eq!(constraint_ids(&s), before, "{label}: undo");
}

/// AC52 (a)-(d): two points about the Y axis, the X axis, a construction line and a drawn line,
/// selected first and picked with the armed tool, the target last and first.
#[test]
fn symmetric_about_an_axis_or_a_line_works_in_both_flows() {
    for (target, construction) in [(Y_AXIS, false), (X_AXIS, false), (L2, true), (L1, false)] {
        for rail in [false, true] {
            for target_first in [false, true] {
                symmetric_about(target, construction, rail, target_first);
            }
        }
    }
}

/// AC52: two points and a circle are refused with the AC40 hint; nothing is applied.
#[test]
fn two_points_and_a_circle_are_refused_with_the_hint() {
    let mut s = session();
    for (i, at) in [P1, P2, C1].iter().enumerate() {
        click(&mut s, *at, i > 0);
    }
    let (before, steps) = (kinds(&s), s.undo_len());
    assert!(!s.sketch_rail_press("symmetric").unwrap());
    assert_eq!((kinds(&s), s.undo_len()), (before, steps));
    let hint = s.sketch.constraint_hint.clone().unwrap();
    assert!(hint.starts_with("Symmetric: select two points"), "{hint}");
}

/// AC52, the customer's cause: an axis under a dimension line, or under the origin's reach,
/// is still a pick. A press on the Y axis where a horizontal distance's line crosses it picks
/// the axis, and an Alt-click at the origin cycles to the axes.
#[test]
fn the_axes_stay_pickable_under_a_dimension_line_and_at_the_origin() {
    let mut s = session();
    // A horizontal distance between a point left and a point right of the Y axis.
    s.sketch_construct(&Construction::Point(Position::mm(-10.0, 40.0)))
        .unwrap();
    s.sketch_construct(&Construction::Point(Position::mm(60.0, 40.0)))
        .unwrap();
    s.set_sketch_tool(SketchTool::Select);
    click(&mut s, (-10.0, 40.0), false);
    click(&mut s, (60.0, 40.0), true);
    assert!(s.sketch_rail_press("horizontal-distance").unwrap());
    s.sketch_close_dimension_edit();
    click(&mut s, (200.0, 200.0), false);
    let snapshot = snapshot(&s);
    let record = s.sketch_dimension_records(&snapshot).remove(0);
    let crossing = record
        .marks
        .lines
        .iter()
        .find(|(a, b)| (a.x - b.x).abs() > 1.0)
        .map(|(a, _)| a.y)
        .unwrap();
    let local = snapshot
        .placement
        .inverse()
        .unwrap()
        .apply_point(Point::new(
            snapshot.placement.apply_point(Point::new(0.0, 0.0)).x,
            crossing,
        ));
    // The text sits mid-span, 25 mm right of the axis: the press is on the bare line.
    let on_line = (0.0, local.y);
    assert!(
        record
            .text_box()
            .center
            .vector_to(global(&s, on_line))
            .length()
            * s.view().scale()
            > 40.0
    );
    click(&mut s, on_line, false);
    assert_eq!(
        s.sketch.entities,
        [EntityId::from(curvyo_ui_core::SKETCH_AXIS_Y)].into(),
        "the Y axis under the dimension line"
    );
    // At the origin the origin point wins; Alt-click cycles on to the axes.
    let origin = snapshot.graph.origin.clone();
    alt_click(&mut s, (0.0, 0.0));
    assert_eq!(s.sketch.entities, [origin].into());
    alt_click(&mut s, (0.0, 0.0));
    let axis = s.sketch.entities.iter().next().unwrap().0.clone();
    assert!(axis.starts_with("@axis"), "{axis}");
}

/// AC49: while the entry box edits a dimension, its line is drawn whole: the hidden text
/// leaves no gap.
#[test]
fn an_edited_dimension_line_has_no_gap_where_its_hidden_text_was() {
    let mut s = session();
    click(&mut s, P1, false);
    click(&mut s, P3, true);
    assert!(s.sketch_rail_press("vertical-distance").unwrap());
    assert!(s.sketch.canvas_edit.is_some(), "the entry is open");
    let snapshot = snapshot(&s);
    let record = s.sketch_dimension_records(&snapshot).remove(0);
    // Two extension lines and one unbroken dimension line.
    assert_eq!(record.marks.lines.len(), 3, "{:?}", record.marks.lines);
    s.sketch_close_dimension_edit();
    let record = s.sketch_dimension_records(&snapshot).remove(0);
    assert_eq!(record.marks.lines.len(), 4, "broken around the shown text");
}
