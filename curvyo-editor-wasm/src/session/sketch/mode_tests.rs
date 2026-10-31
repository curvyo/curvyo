//! Revised drawing modes retain connected intent across actual input events.
use super::*;
use curvyo_document_core::Point;
use curvyo_sketch_core::{Construction, Position};
#[test]
fn pointer_chain_changes_to_tangent_arc_without_committing_mode_change() {
    let mut s = Session::new(66199);
    s.set_workbench(Workbench::Sketch);
    s.set_sketch_tool(SketchTool::Polyline);
    let placement = s.sketch_placement().unwrap();
    for p in [Point::new(20.0, 20.0), Point::new(30.0, 20.0)] {
        let p = placement.apply_point(p);
        s.pointer_down(p, false);
        s.pointer_up(p, false, false);
    }
    assert_eq!(s.undo_len(), 1);
    assert!(s.set_sketch_mode("tangent-arc"));
    assert_eq!(s.undo_len(), 1);
    let p = placement.apply_point(Point::new(40.0, 30.0));
    s.pointer_down(p, false);
    s.pointer_up(p, false, false);
    assert_eq!(s.undo_len(), 2);
    let graph = s.document.sketch(s.active_sketch().unwrap()).unwrap().graph;
    assert!(
        graph
            .entities
            .values()
            .any(|e| matches!(e, curvyo_sketch_core::Entity::Arc { .. }))
    );
    assert!(
        graph
            .constraints
            .values()
            .any(|c| matches!(c.relation, curvyo_sketch_core::Relation::Tangent(..)))
    );
}

#[test]
fn typed_tangent_line_preserves_length_in_the_requested_direction() {
    let mut s = Session::new(66209);
    s.set_workbench(Workbench::Sketch);
    s.set_sketch_tool(SketchTool::Polyline);
    s.sketch_construct(&Construction::Polyline(vec![
        Position::mm(10.0, 10.0),
        Position::mm(20.0, 10.0),
    ]))
    .unwrap();
    let snapshot = s.document.sketch(s.active_sketch().unwrap()).unwrap();
    let (curve, end) = snapshot
        .graph
        .entities
        .iter()
        .find_map(|(id, entity)| {
            if let curvyo_sketch_core::Entity::Line { end, .. } = entity {
                Some((id.clone(), end.clone()))
            } else {
                None
            }
        })
        .unwrap();
    s.sketch.chain = Some((end.clone(), end));
    s.sketch.chain_curve = Some(curve);
    s.sketch.picks = vec![Position::mm(20.0, 10.0)];
    assert!(s.set_sketch_mode("tangent-line"));
    s.sketch_complete_numeric("12 mm", "45 deg").unwrap();
    let snapshot = s.document.sketch(s.active_sketch().unwrap()).unwrap();
    let (_, last) = s.sketch.chain.as_ref().unwrap();
    assert!(
        snapshot
            .solution
            .point(last)
            .unwrap()
            .distance(Position::mm(32.0, 10.0))
            .0
            < 1e-6
    );
    assert!(
        snapshot
            .graph
            .constraints
            .values()
            .any(|c| matches!(c.relation, curvyo_sketch_core::Relation::Parallel(..)))
    );
}

#[test]
fn dimension_placement_is_ephemeral_until_one_atomic_typed_commit() {
    let mut s = Session::new(66230);
    s.set_workbench(Workbench::Sketch);
    s.sketch_construct(&Construction::Polyline(vec![
        Position::mm(10.0, 10.0),
        Position::mm(20.0, 10.0),
    ]))
    .unwrap();
    let id = s.active_sketch().unwrap();
    let before = s.document.sketch(id).unwrap();
    let line = before
        .graph
        .entities
        .iter()
        .find_map(|(id, e)| {
            matches!(e, curvyo_sketch_core::Entity::Line { .. }).then(|| id.clone())
        })
        .unwrap();
    s.sketch.entities.insert(line);
    assert!(s.set_sketch_constraint_tool("dimension"));
    let local = Position::mm(15.0, -12.0);
    let global = before
        .placement
        .apply_point(Point::new(local.x.0, local.y.0));
    let version = s.document.version();
    s.pointer_hover(global, false, false);
    assert!(s.sketch_annotations().iter().any(|label| label.preview));
    s.pointer_down(global, false);
    s.pointer_up(global, false, false);
    assert_eq!(s.sketch.dimension_position, Some(local));
    assert_eq!(s.document.version(), version);
    assert!(
        s.sketch_apply_action("length", "unknown_parameter", false)
            .is_err()
    );
    assert_eq!(s.document.version(), version);
    let count = s.undo_len();
    s.sketch_apply_action("length", "10 mm", false).unwrap();
    assert_eq!(s.undo_len(), count + 1);
    let after = s.document.sketch(id).unwrap();
    assert_eq!(after.labels.values().copied().collect::<Vec<_>>(), [local]);
    s.undo();
    assert_eq!(s.document.sketch(id).unwrap().graph, before.graph);
    assert!(s.document.sketch(id).unwrap().labels.is_empty());
}

#[test]
fn invalid_dimension_position_cannot_write_a_constraint_before_failing() {
    let mut s = Session::new(66231);
    s.set_workbench(Workbench::Sketch);
    s.sketch_construct(&Construction::Polyline(vec![
        Position::mm(10.0, 10.0),
        Position::mm(20.0, 10.0),
    ]))
    .unwrap();
    let id = s.active_sketch().unwrap();
    let before = s.document.sketch(id).unwrap();
    let line = before
        .graph
        .entities
        .iter()
        .find_map(|(id, e)| {
            matches!(e, curvyo_sketch_core::Entity::Line { .. }).then(|| id.clone())
        })
        .unwrap();
    s.sketch.entities.insert(line);
    s.set_sketch_constraint_tool("dimension");
    let version = s.document.version();
    s.pointer_down(Point::new(f64::NAN, 20.0), false);
    s.pointer_up(Point::new(f64::NAN, 20.0), false, false);
    assert!(s.sketch_apply_action("length", "10 mm", false).is_err());
    assert_eq!(s.document.version(), version);
    assert_eq!(s.document.sketch(id).unwrap().graph, before.graph);
}

#[test]
fn centre_arc_numeric_radius_is_ephemeral_and_does_not_move_pinned_start() {
    let mut s = Session::new(66232);
    s.set_workbench(Workbench::Sketch);
    s.set_sketch_tool(SketchTool::ArcCenter);
    let frame = s.sketch_placement().unwrap();
    let global = |x, y| frame.apply_point(Point::new(x, y));
    let before = s.document.version();
    s.pointer_down(global(10.0, 10.0), false);
    s.pointer_up(global(10.0, 10.0), false, false);
    s.sketch_complete_numeric("5 mm", "").unwrap();
    assert_eq!(s.document.version(), before);
    s.pointer_down(global(30.0, 10.0), false);
    s.pointer_up(global(30.0, 10.0), false, false);
    assert!(s.sketch.picks[1].distance(Position::mm(15.0, 10.0)).0 < 1e-9);
    assert!(s.sketch_complete_numeric("9 mm", "").is_err());
    assert!(s.sketch.picks[1].distance(Position::mm(15.0, 10.0)).0 < 1e-9);
    s.pointer_down(global(10.0, 30.0), false);
    s.pointer_up(global(10.0, 30.0), false, false);
    let snapshot = s.document.sketch(s.active_sketch().unwrap()).unwrap();
    let arc = snapshot
        .solution
        .entities
        .values()
        .find(|e| matches!(e, curvyo_sketch_core::Entity::Arc { .. }))
        .unwrap();
    let curvyo_sketch_core::Entity::Arc {
        radius,
        center,
        start,
        ..
    } = arc
    else {
        panic!("arc")
    };
    assert!((radius.0 - 5.0).abs() < 1e-6);
    assert!(
        snapshot
            .solution
            .point(center)
            .unwrap()
            .distance(Position::mm(10.0, 10.0))
            .0
            < 1e-6
    );
    assert!(
        snapshot
            .solution
            .point(start)
            .unwrap()
            .distance(Position::mm(15.0, 10.0))
            .0
            < 1e-6
    );
    assert_eq!(s.undo_len(), 1);
}

#[test]
fn clicking_existing_open_endpoint_reuses_its_identity_for_continuation() {
    let mut s = Session::new(66233);
    s.set_workbench(Workbench::Sketch);
    s.sketch_construct(&Construction::Polyline(vec![
        Position::mm(10.0, 10.0),
        Position::mm(20.0, 10.0),
    ]))
    .unwrap();
    let id = s.active_sketch().unwrap();
    let before = s.document.sketch(id).unwrap();
    let endpoint = before
        .solution
        .entities
        .iter()
        .find_map(|(id, e)| match e {
            curvyo_sketch_core::Entity::Point { position, .. }
                if position.distance(Position::mm(20.0, 10.0)).0 < 1e-9 =>
            {
                Some(id.clone())
            }
            _ => None,
        })
        .unwrap();
    s.set_sketch_tool(SketchTool::Polyline);
    let p = before.placement.apply_point(Point::new(20.0, 10.0));
    let version = s.document.version();
    s.pointer_down(p, false);
    s.pointer_up(p, false, false);
    assert_eq!(s.sketch.chain.as_ref().unwrap().1, endpoint);
    assert_eq!(s.document.version(), version);
    let p = before.placement.apply_point(Point::new(30.0, 20.0));
    s.pointer_down(p, false);
    s.pointer_up(p, false, false);
    let after = s.document.sketch(id).unwrap();
    assert_eq!(after.graph.entities.len(), before.graph.entities.len() + 2);
    assert_eq!(after.graph.entities.values().filter(|e|matches!(e,curvyo_sketch_core::Entity::Line{start,end} if start==&endpoint || end==&endpoint)).count(),2);
}

#[test]
fn axis_inference_does_not_duplicate_an_implied_perpendicular_coordinate() {
    let mut s = Session::new(66234);
    s.set_workbench(Workbench::Sketch);
    s.set_sketch_tool(SketchTool::Polyline);
    let map = s.sketch_placement().unwrap();
    for p in [Point::new(-20.0, 0.0), Point::new(0.0, 0.0)] {
        let p = map.apply_point(p);
        s.pointer_down(p, false);
        s.pointer_up(p, false, false);
    }
    assert!(s.set_sketch_mode("perpendicular-line"));
    let p = map.apply_point(Point::new(0.0, 20.0));
    s.pointer_down(p, false);
    s.pointer_up(p, false, false);
    let snapshot = s.document.sketch(s.active_sketch().unwrap()).unwrap();
    assert!(snapshot.redundant.is_empty(), "{:?}", snapshot.redundant);
    assert!(
        snapshot
            .graph
            .constraints
            .values()
            .any(|c| matches!(c.relation, curvyo_sketch_core::Relation::Perpendicular(..)))
    );
}

#[test]
fn standalone_arc_resumes_line_and_arc_endpoints_as_one_undoable_step() {
    for base in [
        Construction::Polyline(vec![Position::mm(10.0, 10.0), Position::mm(20.0, 10.0)]),
        Construction::Arc([
            Position::mm(10.0, 10.0),
            Position::mm(15.0, 15.0),
            Position::mm(20.0, 10.0),
        ]),
    ] {
        let mut s = Session::new(66235);
        s.set_workbench(Workbench::Sketch);
        s.sketch_construct(&base).unwrap();
        let id = s.active_sketch().unwrap();
        let before = s.document.sketch(id).unwrap();
        let endpoint = before
            .solution
            .entities
            .iter()
            .find_map(|(id, e)| match e {
                curvyo_sketch_core::Entity::Point { position, .. }
                    if position.distance(Position::mm(20.0, 10.0)).0 < 1e-9 =>
                {
                    Some(id.clone())
                }
                _ => None,
            })
            .unwrap();
        s.set_sketch_tool(SketchTool::Arc);
        let version = s.document.version();
        for (index, p) in [
            Point::new(20.0, 10.0),
            Point::new(25.0, 15.0),
            Point::new(30.0, 10.0),
        ]
        .into_iter()
        .enumerate()
        {
            let p = before.placement.apply_point(p);
            s.pointer_down(p, false);
            s.pointer_up(p, false, false);
            if index < 2 {
                assert_eq!(s.document.version(), version);
            }
        }
        let after = s.document.sketch(id).unwrap();
        assert_eq!(after.graph.entities.len(), before.graph.entities.len() + 3);
        assert_eq!(
            after
                .graph
                .entities
                .values()
                .filter(|e| match e {
                    curvyo_sketch_core::Entity::Line { start, end }
                    | curvyo_sketch_core::Entity::Arc { start, end, .. } =>
                        start == &endpoint || end == &endpoint,
                    _ => false,
                })
                .count(),
            2
        );
        assert_eq!(s.undo_len(), 2);
        s.undo();
        assert_eq!(s.document.sketch(id).unwrap().graph, before.graph);
    }
}

#[test]
fn radius_preference_updates_pending_dimension_without_document_writes() {
    let mut s = Session::new(66236);
    s.set_workbench(Workbench::Sketch);
    s.sketch_construct(&Construction::Circle {
        center: Position::mm(20.0, 20.0),
        radius: curvyo_sketch_core::Length(6.0),
    })
    .unwrap();
    let snapshot = s.document.sketch(s.active_sketch().unwrap()).unwrap();
    let circle = snapshot
        .graph
        .entities
        .iter()
        .find_map(|(id, entity)| {
            matches!(entity, curvyo_sketch_core::Entity::Circle { .. }).then(|| id.clone())
        })
        .unwrap();
    s.sketch.entities.insert(circle);
    s.set_sketch_constraint_tool("dimension");
    s.pointer_hover(
        snapshot.placement.apply_point(Point::new(30.0, 30.0)),
        false,
        false,
    );
    let version = s.document.version();
    assert_eq!(
        s.sketch_annotations()
            .iter()
            .find(|a| a.preview)
            .unwrap()
            .text,
        "12.00 mm"
    );
    s.set_sketch_circle_dimension(true);
    assert_eq!(s.sketch_dimension_action(), Some("radius"));
    assert_eq!(
        s.sketch_annotations()
            .iter()
            .find(|a| a.preview)
            .unwrap()
            .text,
        "6.00 mm"
    );
    s.set_sketch_circle_dimension(false);
    assert_eq!(
        s.sketch_annotations()
            .iter()
            .find(|a| a.preview)
            .unwrap()
            .text,
        "12.00 mm"
    );
    assert_eq!(s.document.version(), version);
}
