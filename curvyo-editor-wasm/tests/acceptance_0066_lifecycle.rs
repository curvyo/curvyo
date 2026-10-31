//! Independent sketch lifecycle, origin, shape dragging and state diagnosis.
#![allow(clippy::unwrap_used)]
use curvyo_document_core::{Document, Length, Point, RectBounds, pack, unpack};
use curvyo_editor_wasm::{RowFlag, Session, SketchTool, Workbench};
use curvyo_sketch_core as sk;
fn doc(s: &Session) -> Document {
    unpack(941, &s.pack("acceptance").unwrap()).unwrap()
}
fn state(s: &Session) -> curvyo_document_core::SketchSnapshot {
    doc(s).sketch(s.active_sketch().unwrap()).unwrap()
}
fn fresh() -> Session {
    let d = Document::new(940);
    d.create_page_sketch(
        &d.pages()[0].id,
        &sk::Sketch::new("origin".into()),
        &curvyo_document_core::Style::default(),
    )
    .unwrap();
    let mut s = Session::open(942, &pack(&d, "owned lifecycle fixture").unwrap()).unwrap();
    s.set_workbench(Workbench::Sketch);
    let mut snap = s.snapping();
    snap.enabled = false;
    s.set_snapping(snap);
    s
}
fn click(s: &mut Session, x: f64, y: f64) {
    let p = state(s).placement.apply_point(Point::new(x, y));
    s.pointer_down(p, false);
    s.pointer_up(p, false, false);
}
fn drag(s: &mut Session, start: Point, end: Point) {
    let placement = state(s).placement;
    let start = placement.apply_point(start);
    let end = placement.apply_point(end);
    s.pointer_down(start, false);
    s.pointer_hover(end, false, false);
    s.pointer_up(end, false, false);
}
#[test]
fn s01_source_page_creation_preserves_group_and_inert_layer_refusal() {
    let d = Document::new(950);
    let rect = d.create_rect(RectBounds {
        origin: Point::new(20.0, 20.0),
        width: Length::from_mm(20.0),
        height: Length::from_mm(10.0),
    });
    let other = d.create_rect(RectBounds {
        origin: Point::new(60.0, 20.0),
        width: Length::from_mm(10.0),
        height: Length::from_mm(10.0),
    });
    let group = d.group_objects(&[rect, other]).unwrap();
    let mut s = Session::open(951, &pack(&d, "acceptance").unwrap()).unwrap();
    s.double_click(Point::new(20.0, 25.0), false, false);
    assert_eq!(s.context_depth(), 1);
    let original_page = s.active_page().id;
    let steps = s.undo_len();
    s.set_workbench(Workbench::Sketch);
    assert_ne!(s.active_page().id, original_page);
    assert_eq!(
        s.undo_len(),
        steps + 1,
        "dedicated source page is a separate reversible operation"
    );
    assert!(s.active_sketch().is_none());
    s.set_sketch_tool(SketchTool::Point);
    s.sketch_complete_numeric("30 mm", "30 mm").unwrap();
    let id = s.active_sketch().unwrap();
    assert_eq!(s.undo_len(), steps + 2);
    assert_eq!(doc(&s).page_sketch(&s.active_page().id), Some(id));
    assert_eq!(doc(&s).parent_node(rect), Some(group));
    assert_eq!(doc(&s).parent_node(other), Some(group));
    s.undo();
    assert!(s.active_sketch().is_none());
    assert_eq!(doc(&s).parent_node(rect), Some(group));
    s.undo();
    assert_eq!(doc(&s).pages().len(), 1);
    for flag in [RowFlag::Locked, RowFlag::Visible] {
        let mut s = Session::new(952);
        s.new_layer_row();
        let layer = s.active_layer().unwrap();
        s.toggle_layer_row_flag(&serde_json::to_string(&layer).unwrap(), flag);
        let before = doc(&s).children(Some(layer));
        s.set_workbench(Workbench::Sketch);
        let steps = s.undo_len();
        s.set_sketch_tool(SketchTool::Point);
        assert!(s.sketch_complete_numeric("30 mm", "30 mm").is_err());
        assert_eq!(s.undo_len(), steps);
        assert!(doc(&s).contains_node(layer));
        assert_eq!(doc(&s).children(Some(layer)), before);
        assert_eq!(doc(&s).page_sketch(&s.active_page().id), s.active_sketch());
    }
}
#[test]
fn s08_origin_requires_release_then_move_refix_and_undo() {
    let mut s = fresh();
    let origin = state(&s).graph.origin;
    let steps = s.undo_len();
    drag(&mut s, Point::new(0.0, 0.0), Point::new(10.0, 10.0));
    assert!(
        state(&s)
            .solution
            .point(&origin)
            .unwrap()
            .distance(sk::Position::mm(0.0, 0.0))
            .0
            < 1e-6
    );
    assert_eq!(s.undo_len(), steps);
    s.sketch_fix_origin(false).unwrap();
    drag(&mut s, Point::new(0.0, 0.0), Point::new(10.0, 10.0));
    assert!(
        state(&s)
            .solution
            .point(&origin)
            .unwrap()
            .distance(sk::Position::mm(10.0, 10.0))
            .0
            < 1e-6
    );
    s.sketch_fix_origin(true).unwrap();
    assert!(matches!(
        state(&s).graph.entities[&origin],
        sk::Entity::Point { fixed: true, .. }
    ));
    s.undo();
    assert!(matches!(
        state(&s).graph.entities[&origin],
        sk::Entity::Point { fixed: false, .. }
    ));
    s.undo();
    assert!(
        state(&s)
            .solution
            .point(&origin)
            .unwrap()
            .distance(sk::Position::mm(0.0, 0.0))
            .0
            < 1e-6
    );
}
#[test]
fn s09_circle_body_preserves_radius_then_radius_handle_changes_it() {
    let mut s = fresh();
    s.set_sketch_tool(SketchTool::Circle);
    click(&mut s, 30.0, 30.0);
    click(&mut s, 40.0, 30.0);
    let graph = state(&s).graph;
    let (id, center) = graph
        .entities
        .iter()
        .find_map(|(id, e)| {
            if let sk::Entity::Circle { center, .. } = e {
                Some((id.clone(), center.clone()))
            } else {
                None
            }
        })
        .unwrap();
    s.set_sketch_tool(SketchTool::Select);
    let steps = s.undo_len();
    drag(&mut s, Point::new(30.0, 40.0), Point::new(35.0, 47.0));
    let moved = state(&s).solution;
    let sk::Entity::Circle { radius, .. } = moved.entities[&id] else {
        panic!()
    };
    assert!((radius.0 - 10.0).abs() < 1e-6);
    assert!(
        moved
            .point(&center)
            .unwrap()
            .distance(sk::Position::mm(35.0, 37.0))
            .0
            < 1e-6
    );
    assert_eq!(s.undo_len(), steps + 1);
    s.sketch_select_entity(&id, false);
    drag(&mut s, Point::new(45.0, 37.0), Point::new(50.0, 37.0));
    let resized = state(&s).solution;
    let sk::Entity::Circle { radius, .. } = resized.entities[&id] else {
        panic!()
    };
    assert!((radius.0 - 15.0).abs() < 1e-6);
    assert!(
        resized
            .point(&center)
            .unwrap()
            .distance(sk::Position::mm(35.0, 37.0))
            .0
            < 1e-6
    );
}
#[test]
fn s12_dimensions_redundancy_conflict_repair_and_conversion_undo() {
    let mut s = fresh();
    s.set_sketch_tool(SketchTool::Polyline);
    click(&mut s, 20.0, 20.0);
    click(&mut s, 40.0, 20.0);
    s.escape();
    let graph = state(&s).graph;
    let line = graph
        .entities
        .iter()
        .find_map(|(id, e)| matches!(e, sk::Entity::Line { .. }).then_some(id.clone()))
        .unwrap();
    assert!(s.sketch_select_entity(&line, false));
    s.sketch_apply_action("horizontal", "", false)
        .unwrap_or_else(|e| panic!("first H {e}: {}", s.sketch_view_json()));
    s.sketch_apply_action("horizontal", "", false)
        .unwrap_or_else(|e| panic!("second H {e}: {}", s.sketch_view_json()));
    assert!(!state(&s).redundant.is_empty());
    s.sketch_apply_action("length", "20 mm", false).unwrap();
    let old = state(&s).resolved;
    s.sketch_apply_action("length", "30 mm", false).unwrap();
    let conflict = state(&s);
    assert!(conflict.stale);
    assert_eq!(conflict.status, Some(sk::SolveStatus::Conflicting));
    assert_eq!(conflict.resolved, old);
    let row = conflict
        .graph
        .constraints
        .iter()
        .find_map(|(id, c)| {
            if let sk::Relation::Length { value, .. } = &c.relation {
                (value
                    .evaluate(&conflict.graph.parameters.evaluate().unwrap())
                    .unwrap()
                    .value()
                    - 30.0)
                    .abs()
                    .lt(&1e-6)
                    .then_some(id.clone())
            } else {
                None
            }
        })
        .unwrap();
    assert!(s.sketch_inspect_constraint(&row, false));
    let inspection: serde_json::Value = serde_json::from_str(&s.sketch_view_json()).unwrap();
    assert!(
        inspection["constraints"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| entry["id"] == row.0
                && entry["selected"] == true
                && entry["entities"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|id| id == &line.0))
    );
    assert_ne!(inspection["status"], serde_json::json!("FullyConstrained"));
    s.sketch_remove_constraint(&row).unwrap();
    assert!(!state(&s).stale);
    s.undo();
    assert!(state(&s).stale);
    s.redo();
    assert!(!state(&s).stale);
    let id = s.active_sketch().unwrap();
    s.sketch_convert_to_path().unwrap();
    assert_eq!(s.workbench(), Workbench::Vector, "conversion exits Sketch");
    assert_eq!(
        s.selected_object_count(),
        1,
        "converted output stays selected"
    );
    assert!(matches!(
        doc(&s).object(id),
        Some(curvyo_document_core::ObjectSnapshot::Path(_))
    ));
    s.undo();
    assert!(doc(&s).sketch(id).is_some());
}

#[test]
fn s13_invalid_typed_radius_cannot_commit_previous_valid_fillet_preview() {
    let mut s = fresh();
    s.set_sketch_tool(SketchTool::Polyline);
    click(&mut s, 20.0, 30.0);
    click(&mut s, 20.0, 20.0);
    click(&mut s, 30.0, 20.0);
    s.escape();
    let before = state(&s).graph;
    let lines: Vec<_> = before
        .entities
        .iter()
        .filter_map(|(id, e)| matches!(e, sk::Entity::Line { .. }).then_some(id.clone()))
        .collect();
    assert_eq!(lines.len(), 2);
    s.set_sketch_tool(SketchTool::Fillet);
    assert!(s.sketch_select_entity(&lines[0], false));
    assert!(s.sketch_select_entity(&lines[1], true));
    let steps = s.undo_len();
    s.sketch_preview_fillet_text("2 mm").unwrap();
    assert_eq!(s.undo_len(), steps);
    assert!(s.sketch_preview_fillet_text("not_a_parameter").is_err());
    assert!(
        !matches!(s.sketch_commit_preview(), Ok(true)),
        "invalid current radius must invalidate older committable preview"
    );
    assert_eq!(s.undo_len(), steps);
    assert_eq!(state(&s).graph, before);
    s.sketch_preview_fillet_text("1 mm").unwrap();
    assert!(s.sketch_commit_preview().unwrap());
    assert_eq!(s.undo_len(), steps + 1);
}
