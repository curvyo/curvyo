//! Workbench navigation never changes the sketch's persistent intent.
use super::*;
use curvyo_document_core::ObjectSnapshot;
use curvyo_sketch_core::{Construction, Position};

#[test]
fn navigation_new_reentry_and_undo_have_distinct_commit_boundaries() {
    let mut session = Session::new(66);
    let before = session.document.version();
    session.set_workbench(Workbench::Sketch);
    assert_eq!(session.document.version(), before);
    assert_eq!(session.active_sketch(), None);
    let id = session.new_sketch().unwrap();
    assert_eq!(session.undo_len(), 1);
    assert_eq!(session.active_sketch(), Some(id));
    session.set_workbench(Workbench::Vector);
    session.set_workbench(Workbench::Sketch);
    assert_eq!(session.active_sketch(), Some(id));
    assert_eq!(session.undo_len(), 1);
    session.undo();
    assert_eq!(session.active_sketch(), None);
    session.redo();
    assert!(session.enter_sketch(id));
    assert_eq!(session.active_sketch(), Some(id));
}

#[test]
fn construction_commits_only_explicit_records_and_clone_reentry_routes_original() {
    let mut session = Session::new(67);
    session.set_workbench(Workbench::Sketch);
    let id = session.new_sketch().unwrap();
    session
        .sketch_construct(&Construction::Rectangle(
            Position::mm(1.0, 2.0),
            Position::mm(11.0, 12.0),
        ))
        .unwrap();
    assert_eq!(session.undo_len(), 2);
    let graph = session.document.sketch(id).unwrap().graph;
    assert_eq!(graph.entities.len(), 9);
    let clone = session.recording(|this| this.document.linked_clone(&[id], &[]).unwrap()[0]);
    assert!(session.enter_sketch(clone));
    assert_eq!(session.active_sketch(), Some(id));
    assert!(matches!(
        session.document.object(clone),
        Some(ObjectSnapshot::Clone(_))
    ));
    session.undo();
    session.undo();
    assert_eq!(session.document.sketch(id).unwrap().graph.entities.len(), 1);
}

#[test]
fn creation_modifier_changes_never_reposition_the_pressed_anchor() {
    use curvyo_document_core::Point;
    let mut session = Session::new(70);
    session.set_workbench(Workbench::Sketch);
    let id = session.new_sketch().unwrap();
    session.set_sketch_tool(SketchTool::Rectangle);
    session.pointer_down(Point::new(1.0, 1.0), false);
    let anchor = session.sketch.press_anchor.unwrap();
    session.modifiers_changed(false, false, true);
    session.pointer_hover(Point::new(30.0, 40.0), false, false);
    session.pointer_up(Point::new(30.0, 40.0), false, false);
    let graph = session.document.sketch(id).unwrap().graph;
    assert!(graph.entities.iter().any(|(id, e)| id.0.ends_with(":p0")
        && matches!(e,curvyo_sketch_core::Entity::Point {position,..} if *position == anchor)));
}

#[test]
fn every_multipick_variant_commits_once_and_polyline_closes_shared_ids() {
    use curvyo_document_core::Point;
    for tool in [
        SketchTool::Point,
        SketchTool::Arc,
        SketchTool::CircleThreePoints,
        SketchTool::Ellipse,
        SketchTool::EllipseThreePoints,
        SketchTool::Slot,
    ] {
        let mut session = Session::new(72);
        session.set_workbench(Workbench::Sketch);
        let id = session.new_sketch().unwrap();
        session.set_snapping(curvyo_document_core::DocumentSnapping {
            enabled: false,
            ..session.snapping()
        });
        session.set_sketch_tool(tool);
        let before = session.undo_len();
        let picks = if tool == SketchTool::Point {
            vec![Point::new(20.0, 20.0)]
        } else {
            vec![
                Point::new(20.0, 20.0),
                Point::new(40.0, 20.0),
                Point::new(30.0, 30.0),
            ]
        };
        for point in picks {
            session.pointer_down(point, false);
            session.pointer_up(point, false, false);
        }
        assert_eq!(
            session.undo_len(),
            before + 1,
            "{tool:?}: {:?}",
            session.sketch.error
        );
        assert!(session.document.sketch(id).unwrap().graph.entities.len() > 1);
    }
    let mut session = Session::new(73);
    session.set_workbench(Workbench::Sketch);
    let id = session.new_sketch().unwrap();
    session.set_snapping(curvyo_document_core::DocumentSnapping {
        enabled: false,
        ..session.snapping()
    });
    session.set_sketch_tool(SketchTool::Polyline);
    let before = session.undo_len();
    for point in [
        Point::new(20.0, 20.0),
        Point::new(40.0, 20.0),
        Point::new(30.0, 30.0),
        Point::new(20.0, 20.0),
    ] {
        session.pointer_down(point, false);
        session.pointer_up(point, false, false);
    }
    let snapshot = session.document.sketch(id).unwrap();
    assert_eq!(session.undo_len(), before + 3);
    assert_eq!(snapshot.graph.entities.len(), 7);
    assert!(snapshot.resolved.closed);
    assert!(session.sketch.chain.is_none());
}

#[test]
fn numeric_creation_and_inspection_view_use_real_domain_values() {
    use curvyo_document_core::Point;
    let mut session = Session::new(79);
    session.set_workbench(Workbench::Sketch);
    let id = session.new_sketch().unwrap();
    session.set_snapping(curvyo_document_core::DocumentSnapping {
        enabled: false,
        ..session.snapping()
    });
    session.set_sketch_tool(SketchTool::Rectangle);
    session.pointer_down(Point::new(20.0, 30.0), false);
    session.pointer_up(Point::new(20.0, 30.0), false, false);
    session.sketch_complete_numeric("40", "15").unwrap();
    let snapshot = session.document.sketch(id).unwrap();
    assert_eq!(snapshot.graph.entities.len(), 9);
    let value: serde_json::Value = serde_json::from_str(&session.sketch_view_json()).unwrap();
    assert_eq!(value["constraints"].as_array().unwrap().len(), 4);
    assert_eq!(value["actions"].as_array().unwrap().len(), 15);
    assert_eq!(value["workbench"], "sketch");
    assert_ne!(session.draw_list().triangles.len(), 0);
}

#[test]
fn revision_sync_retains_only_current_analytic_selection() {
    use curvyo_sketch_core::{Constraint, Relation};
    let mut session = Session::new(84);
    session.set_workbench(Workbench::Sketch);
    let id = session.new_sketch().unwrap();
    for name in ["a", "b"] {
        session
            .sketch_construct_named(name, &Construction::Point(Position::mm(10.0, 20.0)))
            .unwrap();
    }
    let graph = session.document.sketch(id).unwrap().graph;
    let a = graph
        .entities
        .keys()
        .find(|id| id.0.starts_with('a'))
        .unwrap()
        .clone();
    let b = graph
        .entities
        .keys()
        .find(|id| id.0.starts_with('b'))
        .unwrap()
        .clone();
    session.sketch.entities = [a.clone(), b.clone()].into();
    session
        .sketch_set_constraint(
            "fixed".into(),
            Constraint::driving(Relation::Fixed(b.clone())),
        )
        .unwrap();
    assert_eq!(session.sketch.entities, [a.clone(), b.clone()].into());
    session.sketch.constraints.insert("fixed".into());
    session.sketch_delete_entities(&[a.clone()].into()).unwrap();
    assert_eq!(session.sketch.entities, [b.clone()].into());
    assert!(session.sketch.constraints.contains(&"fixed".into()));
    session.sketch_remove_constraint(&"fixed".into()).unwrap();
    assert!(session.sketch.constraints.is_empty());
    session.undo();
    assert_eq!(session.sketch.entities, [b].into());
    assert!(session.sketch.constraints.is_empty());
    session.add_page();
    assert!(session.sketch.entities.is_empty());
    assert!(session.sketch.constraints.is_empty());
}

#[test]
fn conversion_switches_only_on_success_and_keeps_one_undo_step() {
    let mut session = Session::new(85);
    session.set_workbench(Workbench::Sketch);
    let history = session.undo_len();
    assert!(session.sketch_convert_to_path().is_err());
    assert_eq!(session.workbench(), Workbench::Sketch);
    assert_eq!(session.undo_len(), history);
    let id = session.new_sketch().unwrap();
    session
        .sketch_construct(&Construction::Rectangle(
            Position::mm(10.0, 20.0),
            Position::mm(30.0, 40.0),
        ))
        .unwrap();
    let before = session.document.sketch(id).unwrap().graph;
    let history = session.undo_len();
    session.sketch_convert_to_path().unwrap();
    assert_eq!(session.workbench(), Workbench::Vector);
    assert_eq!(session.tool(), crate::Tool::Select);
    assert_eq!(session.selection.ids(), &[id]);
    assert_eq!(session.undo_len(), history + 1);
    session.undo();
    assert_eq!(session.document.sketch(id).unwrap().graph, before);
    session.set_workbench(Workbench::Sketch);
    assert_eq!(session.active_sketch(), Some(id));
}

#[test]
fn typed_fillet_preview_rejects_invalid_units_without_history() {
    let mut session = Session::new(88);
    session.set_workbench(Workbench::Sketch);
    session.new_sketch().unwrap();
    let before = session.undo_len();
    assert!(session.sketch_preview_fillet_text("2 rad").is_err());
    assert_eq!(session.undo_len(), before);
}

#[test]
fn free_geometry_inspection_selects_current_entities_without_history() {
    let mut session = Session::new(89);
    session.set_workbench(Workbench::Sketch);
    session.new_sketch().unwrap();
    session
        .sketch_construct(&Construction::Point(Position::mm(30.0, 40.0)))
        .unwrap();
    let before = session.undo_len();
    assert!(session.sketch_show_free());
    assert!(!session.sketch.entities.is_empty());
    assert_eq!(session.undo_len(), before);
}

#[test]
fn parameter_inspection_follows_transitive_dimension_dependencies() {
    use curvyo_sketch_core::{Dimension, Entity};
    let mut session = Session::new(90);
    session.set_workbench(Workbench::Sketch);
    let id = session.new_sketch().unwrap();
    session
        .sketch_construct(&Construction::Rectangle(
            Position::mm(10.0, 10.0),
            Position::mm(30.0, 20.0),
        ))
        .unwrap();
    let parameter = session
        .sketch_add_parameter("base", "20 mm", Dimension::Length)
        .unwrap();
    session
        .sketch_add_parameter("width", "base * 2", Dimension::Length)
        .unwrap();
    let line = session
        .document
        .sketch(id)
        .unwrap()
        .graph
        .entities
        .iter()
        .find(|(_, e)| matches!(e, Entity::Line { .. }))
        .unwrap()
        .0
        .clone();
    session.sketch_select_entity(&line, false);
    session
        .sketch_apply_action("length", "width", false)
        .unwrap();
    let before = session.undo_len();
    assert!(session.sketch_inspect_parameter(&parameter));
    assert_eq!(session.sketch.constraints.len(), 1);
    assert!(session.sketch.entities.contains(&line));
    assert_eq!(session.undo_len(), before);
}

#[test]
fn no_drawing_tool_creates_a_redundant_or_conflicting_constraint_on_a_fresh_draw() {
    use curvyo_document_core::Point;
    let clicks = |tool: SketchTool| match tool {
        SketchTool::Point => vec![Point::new(20.0, 20.0)],
        SketchTool::Polyline => vec![
            Point::new(20.0, 20.0),
            Point::new(40.0, 20.0),
            Point::new(30.0, 30.0),
            Point::new(20.0, 20.0),
        ],
        _ => vec![
            Point::new(20.0, 20.0),
            Point::new(40.0, 20.0),
            Point::new(30.0, 30.0),
        ],
    };
    for tool in [
        SketchTool::Point,
        SketchTool::Polyline,
        SketchTool::Arc,
        SketchTool::ArcCenter,
        SketchTool::Circle,
        SketchTool::CircleThreePoints,
        SketchTool::Ellipse,
        SketchTool::EllipseThreePoints,
        SketchTool::Rectangle,
        SketchTool::Slot,
    ] {
        let mut session = Session::new(74);
        session.set_workbench(Workbench::Sketch);
        let id = session.new_sketch().unwrap();
        session.set_snapping(curvyo_document_core::DocumentSnapping {
            enabled: false,
            ..session.snapping()
        });
        session.set_sketch_tool(tool);
        if matches!(tool, SketchTool::Rectangle | SketchTool::Circle) {
            session.pointer_down(Point::new(20.0, 20.0), false);
            session.pointer_hover(Point::new(45.0, 35.0), false, false);
            session.pointer_up(Point::new(45.0, 35.0), false, false);
        } else {
            for point in clicks(tool) {
                session.pointer_down(point, false);
                session.pointer_up(point, false, false);
            }
        }
        let snapshot = session.document.sketch(id).unwrap();
        assert!(
            snapshot.graph.constraints.len() + snapshot.graph.entities.len() > 1,
            "{tool:?} drew nothing: {:?}",
            session.sketch.error
        );
        assert!(
            snapshot.redundant.is_empty(),
            "{tool:?}: {:?}",
            snapshot.redundant
        );
        assert!(
            snapshot.conflicting.is_empty(),
            "{tool:?}: {:?}",
            snapshot.conflicting
        );
    }
}
