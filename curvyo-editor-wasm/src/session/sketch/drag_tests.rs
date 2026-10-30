//! Workbench navigation never changes the sketch's persistent intent.
use super::*;
use curvyo_sketch_core::{Construction, Position};

#[test]
fn rectangle_clicks_and_circle_drag_use_actual_pointer_routes() {
    use curvyo_document_core::Point;
    let mut session = Session::new(68);
    session.set_workbench(Workbench::Sketch);
    let id = session.new_sketch().unwrap();
    session.set_snapping(curvyo_document_core::DocumentSnapping {
        enabled: false,
        ..session.snapping()
    });
    let baseline = session.undo_len();
    session.set_sketch_tool(SketchTool::Rectangle);
    session.pointer_down(Point::new(10.0, 20.0), false);
    session.pointer_up(Point::new(10.0, 20.0), false, false);
    assert_eq!(session.undo_len(), baseline);
    session.pointer_hover(Point::new(30.0, 40.0), false, false);
    session.pointer_down(Point::new(30.0, 40.0), false);
    session.pointer_up(Point::new(30.0, 40.0), false, false);
    assert_eq!(session.undo_len(), baseline + 1);
    assert_eq!(session.document.sketch(id).unwrap().graph.entities.len(), 9);
    session.set_sketch_tool(SketchTool::Circle);
    session.pointer_down(Point::new(50.0, 50.0), false);
    session.pointer_hover(Point::new(60.0, 50.0), false, false);
    session.pointer_up(Point::new(60.0, 50.0), false, false);
    assert_eq!(session.undo_len(), baseline + 2);
    assert_eq!(
        session.document.sketch(id).unwrap().graph.entities.len(),
        11
    );
    session.set_sketch_tool(SketchTool::Arc);
    session.pointer_down(Point::new(80.0, 80.0), false);
    session.pointer_up(Point::new(80.0, 80.0), false, false);
    session.escape();
    assert_eq!(session.undo_len(), baseline + 2);
    assert_eq!(session.sketch.picks.len(), 0);
}

#[test]
fn ellipse_body_drag_is_live_shape_preserving_cancelable_and_one_commit() {
    use curvyo_document_core::Point;
    use curvyo_sketch_core::{Entity, Length};
    let mut session = Session::new(71);
    session.set_workbench(Workbench::Sketch);
    let id = session.new_sketch().unwrap();
    session.set_snapping(curvyo_document_core::DocumentSnapping {
        enabled: false,
        ..session.snapping()
    });
    session
        .sketch_construct(&Construction::Ellipse {
            center: Position::mm(30.0, 30.0),
            axis_end: Position::mm(40.0, 30.0),
            second_axis: Length(5.0),
        })
        .unwrap();
    session.set_sketch_tool(SketchTool::Select);
    let before = session.document.sketch(id).unwrap();
    let history = session.undo_len();
    session.pointer_down(Point::new(30.0, 35.0), false);
    session.pointer_hover(Point::new(37.0, 42.0), false, false);
    let preview = session.sketch_preview().unwrap();
    let center = before
        .graph
        .entities
        .values()
        .find_map(|e| match e {
            Entity::Ellipse { center, .. } => Some(center),
            _ => None,
        })
        .unwrap();
    assert!(
        preview
            .point(center)
            .unwrap()
            .distance(Position::mm(37.0, 37.0))
            .0
            < 1e-6
    );
    assert_eq!(session.document.sketch(id).unwrap().graph, before.graph);
    session.escape();
    assert!(session.sketch_preview().is_none());
    assert_eq!(session.undo_len(), history);
    // A second Escape clears selection; the first preserves it after cancel.
    session.escape();
    session.pointer_down(Point::new(30.0, 35.0), false);
    session.pointer_hover(Point::new(37.0, 42.0), false, false);
    session.pointer_up(Point::new(37.0, 42.0), false, false);
    assert_eq!(session.undo_len(), history + 1);
    let after = session.document.sketch(id).unwrap();
    assert!(
        after
            .solution
            .point(center)
            .unwrap()
            .distance(Position::mm(37.0, 37.0))
            .0
            < 1e-6
    );
    session.undo();
    assert_eq!(session.document.sketch(id).unwrap().graph, before.graph);
}

#[test]
fn constrained_release_persists_feasible_point_even_after_dimension_removal() {
    use curvyo_document_core::Point;
    use curvyo_sketch_core::{Constraint, Entity, Expression, Quantity, Relation};
    let mut session = Session::new(74);
    session.set_workbench(Workbench::Sketch);
    let id = session.new_sketch().unwrap();
    session.set_snapping(curvyo_document_core::DocumentSnapping {
        enabled: false,
        ..session.snapping()
    });
    session
        .sketch_construct_named("point", &Construction::Point(Position::mm(10.0, 0.0)))
        .unwrap();
    let snapshot = session.document.sketch(id).unwrap();
    let point = snapshot
        .graph
        .entities
        .iter()
        .find(|(key, entity)| {
            **key != snapshot.graph.origin && matches!(entity, Entity::Point { .. })
        })
        .unwrap()
        .0
        .clone();
    session
        .sketch_set_constraint(
            "distance".into(),
            Constraint::driving(Relation::Distance {
                a: snapshot.graph.origin,
                b: point.clone(),
                value: Expression::Literal(Quantity::length(10.0).unwrap()),
            }),
        )
        .unwrap();
    session.pointer_down(Point::new(10.0, 0.0), false);
    session.pointer_hover(Point::new(20.0, 20.0), false, false);
    let expected = Position::mm(10.0 / 2.0_f64.sqrt(), 10.0 / 2.0_f64.sqrt());
    assert!(
        session
            .sketch_preview()
            .unwrap()
            .point(&point)
            .unwrap()
            .distance(expected)
            .0
            < 1e-6
    );
    session.pointer_up(Point::new(20.0, 20.0), false, false);
    assert!(
        session
            .document
            .sketch(id)
            .unwrap()
            .graph
            .point(&point)
            .unwrap()
            .distance(expected)
            .0
            < 1e-6
    );
    session
        .sketch_remove_constraint(&"distance".into())
        .unwrap();
    assert!(
        session
            .document
            .sketch(id)
            .unwrap()
            .solution
            .point(&point)
            .unwrap()
            .distance(expected)
            .0
            < 1e-6
    );
    session.undo();
    assert!(
        session
            .document
            .sketch(id)
            .unwrap()
            .graph
            .constraints
            .contains_key(&"distance".into())
    );
    assert!(
        session
            .document
            .sketch(id)
            .unwrap()
            .solution
            .point(&point)
            .unwrap()
            .distance(expected)
            .0
            < 1e-6
    );
}

#[test]
fn selected_ellipse_actual_axis_handle_edits_axis_without_moving_center() {
    use curvyo_document_core::Point;
    use curvyo_sketch_core::{Entity, Length, SketchHandle, handle_positions};
    let mut session = Session::new(78);
    session.set_workbench(Workbench::Sketch);
    let id = session.new_sketch().unwrap();
    session.set_snapping(curvyo_document_core::DocumentSnapping {
        enabled: false,
        ..session.snapping()
    });
    session
        .sketch_construct_named(
            "ellipse",
            &Construction::Ellipse {
                center: Position::mm(30.0, 30.0),
                axis_end: Position::mm(50.0, 30.0),
                second_axis: Length(10.0),
            },
        )
        .unwrap();
    assert!(session.sketch_select_entity(&"ellipse:curve".into(), false));
    let history = session.undo_len();
    session.pointer_down(Point::new(50.5, 30.0), false);
    session.pointer_hover(Point::new(60.5, 30.0), false, false);
    let preview = session.sketch_preview().unwrap();
    assert!(
        handle_positions(preview, &"ellipse:curve".into()).unwrap()[&SketchHandle::MajorAxis]
            .distance(Position::mm(60.0, 30.0))
            .0
            < 1e-6
    );
    session.pointer_up(Point::new(60.5, 30.0), false, false);
    let after = session.document.sketch(id).unwrap();
    assert_eq!(session.undo_len(), history + 1);
    let Entity::Ellipse { center, .. } = &after.solution.entities[&"ellipse:curve".into()] else {
        panic!("ellipse");
    };
    assert!(
        after
            .solution
            .point(center)
            .unwrap()
            .distance(Position::mm(30.0, 30.0))
            .0
            < 1e-6
    );
}

#[test]
fn offset_press_snaps_actual_point_to_stationary_sibling_with_alt_inversion() {
    use curvyo_document_core::Point;
    let mut session = Session::new(80);
    session.set_workbench(Workbench::Sketch);
    let id = session.new_sketch().unwrap();
    for (name, x) in [("moving", 10.0), ("target", 30.0)] {
        session
            .sketch_construct_named(name, &Construction::Point(Position::mm(x, 20.0)))
            .unwrap();
    }
    session.set_snapping(curvyo_document_core::DocumentSnapping {
        enabled: true,
        objects: false,
        guides: false,
        page: false,
        nodes: true,
        ..session.snapping()
    });
    session.pointer_down(Point::new(10.5, 20.0), false);
    session.pointer_hover(Point::new(29.5, 20.0), false, false);
    let point = session
        .document
        .sketch(id)
        .unwrap()
        .graph
        .entities
        .iter()
        .find(|(key, _)| key.0.starts_with("moving"))
        .unwrap()
        .0
        .clone();
    assert!(
        session
            .sketch_preview()
            .unwrap()
            .point(&point)
            .unwrap()
            .distance(Position::mm(30.0, 20.0))
            .0
            < 1e-6
    );
    session.modifiers_changed(false, false, true);
    assert!(
        session
            .sketch_preview()
            .unwrap()
            .point(&point)
            .unwrap()
            .distance(Position::mm(29.0, 20.0))
            .0
            < 1e-6
    );
    session.modifiers_changed(false, false, false);
    assert!(
        session
            .sketch_preview()
            .unwrap()
            .point(&point)
            .unwrap()
            .distance(Position::mm(30.0, 20.0))
            .0
            < 1e-6
    );
    session.pointer_up(Point::new(29.5, 20.0), false, false);
    assert!(
        session
            .document
            .sketch(id)
            .unwrap()
            .graph
            .point(&point)
            .unwrap()
            .distance(Position::mm(30.0, 20.0))
            .0
            < 1e-6
    );
}

#[test]
fn fixed_origin_does_not_hide_other_constrained_component_snap_targets() {
    use curvyo_document_core::Point;
    use curvyo_sketch_core::{Constraint, Expression, Quantity, Relation};
    let mut session = Session::new(83);
    session.set_workbench(Workbench::Sketch);
    let id = session.new_sketch().unwrap();
    for (name, p) in [
        ("moving", Position::mm(10.0, 20.0)),
        ("target", Position::mm(20.0, 10.0)),
    ] {
        session
            .sketch_construct_named(name, &Construction::Point(p))
            .unwrap();
    }
    let graph = session.document.sketch(id).unwrap().graph;
    for name in ["moving", "target"] {
        let point = graph
            .entities
            .keys()
            .find(|key| key.0.starts_with(name))
            .unwrap()
            .clone();
        session
            .sketch_set_constraint(
                name.into(),
                Constraint::driving(Relation::Distance {
                    a: graph.origin.clone(),
                    b: point,
                    value: Expression::Literal(Quantity::length(500.0_f64.sqrt()).unwrap()),
                }),
            )
            .unwrap();
    }
    session.set_snapping(curvyo_document_core::DocumentSnapping {
        enabled: true,
        objects: false,
        guides: false,
        page: false,
        nodes: true,
        ..session.snapping()
    });
    session.pointer_down(Point::new(10.0, 20.0), false);
    session.pointer_hover(Point::new(20.5, 10.5), false, false);
    let point = graph
        .entities
        .keys()
        .find(|key| key.0.starts_with("moving"))
        .unwrap();
    assert!(
        session
            .sketch_preview()
            .unwrap()
            .point(point)
            .unwrap()
            .distance(Position::mm(20.0, 10.0))
            .0
            < 1e-6
    );
}

#[test]
fn repeated_modifier_snapshot_does_not_advance_an_existing_drag_preview() {
    use curvyo_document_core::Point;
    let mut session = Session::new(94);
    session.set_workbench(Workbench::Sketch);
    session.new_sketch().unwrap();
    session.set_snapping(curvyo_document_core::DocumentSnapping {
        enabled: false,
        ..session.snapping()
    });
    session
        .sketch_construct(&Construction::Point(Position::mm(20.0, 20.0)))
        .unwrap();
    session.set_sketch_tool(SketchTool::Select);
    session.pointer_down(Point::new(20.0, 20.0), false);
    session.pointer_hover(Point::new(30.0, 35.0), false, false);
    let preview = session.sketch_preview().unwrap().clone();
    let steps = session.undo_len();
    session.modifiers_changed(false, false, false);
    assert_eq!(session.sketch_preview(), Some(&preview));
    assert_eq!(session.undo_len(), steps);
    session.modifiers_changed(false, false, true);
    session.modifiers_changed(false, false, false);
    assert!(session.sketch_preview().is_some());
}
