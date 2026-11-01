//! Completed shapes return to Select with exactly the new geometry selected; Polyline only when
//! its chain ends (0066 AC42), through pointer and key events (AC41).
use super::*;
use crate::session::{KeyInput, KeyOutcome};
use curvyo_document_core::Point;
use curvyo_sketch_core::{Construction, Entity, EntityId, Position};
use std::collections::BTreeSet;

fn global(s: &Session, x: f64, y: f64) -> Point {
    s.sketch_placement().unwrap().apply_point(Point::new(x, y))
}

fn click(s: &mut Session, x: f64, y: f64) {
    let p = global(s, x, y);
    s.pointer_hover(p, false, false);
    s.pointer_down(p, false);
    s.pointer_up(p, false, false);
}

fn key(s: &mut Session, key: &str) -> KeyOutcome {
    s.key_down(KeyInput {
        key,
        shift: false,
        ctrl: false,
        alt: false,
        repeat: false,
        dom_blocked: false,
        mac: false,
        time_ms: 0.0,
    })
}

/// A sketch with one far-away point, so that the first shape does not create the sketch.
fn session() -> Session {
    let mut s = Session::new(66_940);
    s.set_workbench(Workbench::Sketch);
    s.sketch_construct(&Construction::Point(Position::mm(200.0, 200.0)))
        .unwrap();
    s.set_sketch_tool(SketchTool::Select);
    s
}

fn entity_ids(s: &Session) -> BTreeSet<EntityId> {
    s.document
        .sketch(s.active_sketch().unwrap())
        .unwrap()
        .graph
        .entities
        .keys()
        .cloned()
        .collect()
}

/// The shapes created since `before`: new entities other than the points that define another
/// new entity (a line's endpoints, a circle's centre).
fn shapes(s: &Session, before: &BTreeSet<EntityId>) -> BTreeSet<EntityId> {
    let graph = s.document.sketch(s.active_sketch().unwrap()).unwrap().graph;
    let created: BTreeSet<EntityId> = entity_ids(s).difference(before).cloned().collect();
    let defining: BTreeSet<EntityId> = created
        .iter()
        .flat_map(|id| graph.entities[id].point_ids())
        .cloned()
        .collect();
    created.difference(&defining).cloned().collect()
}

/// Drags a created point by 5 mm along X in Select: it moves, in one step (AC29).
fn drag_a_created_point(s: &mut Session, tool: &str, before: &BTreeSet<EntityId>) {
    let snapshot = s.document.sketch(s.active_sketch().unwrap()).unwrap();
    let created: BTreeSet<EntityId> = entity_ids(s).difference(before).cloned().collect();
    let (id, at) = created
        .iter()
        .find_map(|id| match snapshot.graph.entities.get(id) {
            Some(Entity::Point { .. }) => Some((id.clone(), snapshot.solution.point(id).ok()?)),
            _ => None,
        })
        .unwrap_or_else(|| panic!("{tool}: a created point exists"));
    let steps = s.undo_len();
    let from = global(s, at.x.0, at.y.0);
    let to = global(s, at.x.0 + 5.0, at.y.0);
    s.pointer_hover(from, false, false);
    s.pointer_down(from, false);
    s.pointer_hover(to, false, true);
    s.pointer_up(to, false, false);
    let moved = s
        .document
        .sketch(s.active_sketch().unwrap())
        .unwrap()
        .solution
        .point(&id)
        .unwrap();
    assert!(
        moved.distance(at).0 > 1.0,
        "{tool}: the created point moved"
    );
    assert_eq!(s.undo_len(), steps + 1, "{tool}: the drag is one step");
}

/// Draws with `tool` by clicking `picks`, then checks Select, the selection, one step and a drag.
fn returns(tool: SketchTool, picks: &[(f64, f64)]) {
    let mut s = session();
    s.set_sketch_tool(tool);
    let before = entity_ids(&s);
    let steps = s.undo_len();
    for (x, y) in picks {
        click(&mut s, *x, *y);
    }
    let name = tool.name();
    assert_eq!(
        s.sketch_tool(),
        SketchTool::Select,
        "{name}: back in Select"
    );
    let created = shapes(&s, &before);
    assert!(!created.is_empty(), "{name}: drew something");
    assert_eq!(s.sketch.entities, created, "{name}: the new geometry");
    assert_eq!(s.undo_len(), steps + 1, "{name}: one step");
    drag_a_created_point(&mut s, name, &before);
}

#[test]
fn every_drawing_tool_returns_to_select_with_its_new_geometry() {
    returns(SketchTool::Point, &[(20.0, 20.0)]);
    returns(SketchTool::Circle, &[(30.0, 30.0), (40.0, 30.0)]);
    returns(
        SketchTool::CircleThreePoints,
        &[(20.0, 30.0), (30.0, 40.0), (40.0, 30.0)],
    );
    returns(SketchTool::Arc, &[(20.0, 30.0), (30.0, 38.0), (40.0, 30.0)]);
    returns(
        SketchTool::ArcCenter,
        &[(30.0, 30.0), (40.0, 30.0), (30.0, 40.0)],
    );
    returns(
        SketchTool::Ellipse,
        &[(30.0, 30.0), (45.0, 30.0), (30.0, 36.0)],
    );
    returns(SketchTool::Rectangle, &[(20.0, 20.0), (50.0, 40.0)]);
    returns(
        SketchTool::Slot,
        &[(20.0, 30.0), (50.0, 30.0), (50.0, 35.0)],
    );
}

#[test]
fn a_fillet_returns_to_select_with_its_new_geometry() {
    let mut s = session();
    s.set_sketch_tool(SketchTool::Polyline);
    click(&mut s, 0.0, 20.0);
    click(&mut s, 20.0, 20.0);
    click(&mut s, 20.0, 40.0);
    key(&mut s, "Enter");
    let lines: BTreeSet<EntityId> = s
        .document
        .sketch(s.active_sketch().unwrap())
        .unwrap()
        .graph
        .entities
        .iter()
        .filter(|(_, e)| matches!(e, Entity::Line { .. }))
        .map(|(id, _)| id.clone())
        .collect();
    s.set_sketch_tool(SketchTool::Fillet);
    s.sketch.entities = lines;
    let before = entity_ids(&s);
    let steps = s.undo_len();
    assert!(s.sketch_complete_numeric("4 mm", "").unwrap());
    assert_eq!(s.sketch_tool(), SketchTool::Select);
    let created = shapes(&s, &before);
    assert!(!created.is_empty());
    assert_eq!(s.sketch.entities, created);
    assert_eq!(s.undo_len(), steps + 1);
}

#[test]
fn a_typed_shape_returns_to_select_with_its_new_geometry() {
    let mut s = session();
    s.set_sketch_tool(SketchTool::Rectangle);
    let before = entity_ids(&s);
    click(&mut s, 20.0, 20.0);
    assert!(s.sketch_complete_numeric("30", "10").unwrap());
    assert_eq!(s.sketch_tool(), SketchTool::Select);
    assert_eq!(s.sketch.entities, shapes(&s, &before));
}

/// Three committed segments of a polyline; the tool stays Polyline while the chain is open.
fn open_chain() -> (Session, BTreeSet<EntityId>, usize) {
    let mut s = session();
    s.set_sketch_tool(SketchTool::Polyline);
    let before = entity_ids(&s);
    let steps = s.undo_len();
    for (x, y) in [(10.0, 10.0), (40.0, 10.0), (40.0, 30.0), (10.0, 30.0)] {
        click(&mut s, x, y);
        assert_eq!(
            s.sketch_tool(),
            SketchTool::Polyline,
            "the chain stays open"
        );
    }
    (s, before, steps)
}

fn chain_selected(s: &Session, before: &BTreeSet<EntityId>, steps: usize, how: &str) {
    assert_eq!(s.sketch_tool(), SketchTool::Select, "{how}: back in Select");
    let created = shapes(s, before);
    assert_eq!(
        created
            .iter()
            .filter(|id| matches!(
                s.document
                    .sketch(s.active_sketch().unwrap())
                    .unwrap()
                    .graph
                    .entities
                    .get(*id),
                Some(Entity::Line { .. })
            ))
            .count(),
        if how == "close" { 4 } else { 3 },
        "{how}: every segment"
    );
    assert_eq!(s.sketch.entities, created, "{how}: the whole chain");
    let segments = if how == "close" { 4 } else { 3 };
    assert_eq!(
        s.undo_len(),
        steps + segments,
        "{how}: one step per segment"
    );
}

#[test]
fn a_polyline_returns_to_select_with_its_chain_when_the_chain_ends() {
    let (mut s, before, steps) = open_chain();
    key(&mut s, "Enter");
    chain_selected(&s, &before, steps, "enter");

    let (mut s, before, steps) = open_chain();
    assert!(matches!(key(&mut s, "Escape"), KeyOutcome::Escape(_)));
    chain_selected(&s, &before, steps, "escape");

    let (mut s, before, steps) = open_chain();
    click(&mut s, 10.0, 10.0);
    chain_selected(&s, &before, steps, "close");
    drag_a_created_point(&mut s, "polyline", &before);
}

#[test]
fn escape_without_a_committed_segment_keeps_the_previous_selection() {
    let mut s = session();
    let previous = entity_ids(&s)
        .into_iter()
        .filter(|id| id.0 != "origin")
        .collect::<BTreeSet<_>>();
    s.sketch.entities.clone_from(&previous);
    s.set_sketch_tool(SketchTool::Polyline);
    click(&mut s, 10.0, 10.0);
    key(&mut s, "Escape");
    key(&mut s, "Escape");
    assert_eq!(s.sketch_tool(), SketchTool::Select);
    assert_eq!(s.sketch.entities, previous);
}

/// Tester finding (revision 4 acceptance): a point left selected by the Point tool is no
/// endpoint to resume from, so choosing Arc or Polyline and switching the Arc variant stay
/// silent.
#[test]
fn a_tool_or_variant_change_does_not_resume_from_a_free_point() {
    for tool in [SketchTool::Arc, SketchTool::Polyline] {
        let mut s = session();
        s.set_sketch_tool(SketchTool::Point);
        click(&mut s, 50.0, 50.0);
        assert_eq!(s.sketch.entities.len(), 1, "the new point is selected");
        s.set_sketch_tool(tool);
        let p = global(&s, 80.0, 80.0);
        s.pointer_hover(p, false, false);
        if tool == SketchTool::Arc {
            assert_eq!(key(&mut s, "Alt"), KeyOutcome::Consumed);
            assert_eq!(s.sketch_tool(), SketchTool::ArcCenter);
        }
        let view = s.sketch_view();
        assert!(view["canvas"]["mode_label"]["error"].is_null(), "{tool:?}");
        assert!(view["error"].is_null(), "{tool:?}");
        assert!(s.sketch.chain.is_none(), "{tool:?}: no chain");
    }
}
