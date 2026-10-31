//! Page-owned workbench entry defers persistence until actual geometry.
use super::*;
use curvyo_document_core::Point;
use curvyo_sketch_core::{Construction, Position};

#[test]
fn first_geometry_is_one_centered_page_owned_step_without_new_action() {
    let mut s = Session::new(66190);
    let page = s.active_page();
    let before = s.document.version();
    s.set_workbench(Workbench::Sketch);
    assert_eq!(s.document.version(), before);
    assert!(s.active_sketch().is_none());
    s.sketch_construct(&Construction::Point(Position::mm(10.0, 20.0)))
        .unwrap();
    let id = s.active_sketch().unwrap();
    assert_eq!(s.document.page_sketch(&page.id), Some(id));
    assert_eq!(s.undo_len(), 1);
    assert_eq!(
        s.document
            .sketch(id)
            .unwrap()
            .placement
            .apply_point(Point::new(0.0, 0.0)),
        Point::new(
            page.origin.x + page.size.width.as_mm() / 2.0,
            page.origin.y + page.size.height.as_mm() / 2.0
        )
    );
    s.set_workbench(Workbench::Vector);
    s.set_workbench(Workbench::Sketch);
    assert_eq!(s.active_sketch(), Some(id));
    assert_eq!(s.undo_len(), 1);
}

#[test]
fn changing_page_cancels_picks_and_never_keeps_the_previous_owner() {
    let mut s = Session::new(66191);
    s.set_workbench(Workbench::Sketch);
    s.sketch_construct(&Construction::Point(Position::mm(10.0, 20.0)))
        .unwrap();
    let first = s.active_page().id;
    let owner = s.active_sketch().unwrap();
    let second = s.add_page();
    assert!(s.active_sketch().is_none());
    s.set_sketch_tool(SketchTool::CircleThreePoints);
    s.pointer_down(Point::new(20.0, 20.0), false);
    s.pointer_up(Point::new(20.0, 20.0), false, false);
    assert_eq!(s.sketch.picks.len(), 1);
    s.activate_page(&first);
    assert_eq!(s.sketch.picks, [] as [Position; 0]);
    assert_eq!(s.active_sketch(), Some(owner));
    assert!(s.document.page_sketch(&second).is_none());
}

#[test]
fn actual_click_creates_geometry_without_vector_attraction() {
    let mut s = Session::new(66192);
    s.set_workbench(Workbench::Sketch);
    s.set_sketch_tool(SketchTool::Point);
    let page = s.active_page();
    let point = Point::new(
        page.origin.x + page.size.width.as_mm() / 2.0 + 2.0,
        page.origin.y + page.size.height.as_mm() / 2.0 + 8.0,
    );
    s.pointer_down(point, false);
    s.pointer_up(point, false, false);
    let snapshot = s.document.sketch(s.active_sketch().unwrap()).unwrap();
    assert!(snapshot.graph.entities.values().any(|entity| matches!(entity,
        curvyo_sketch_core::Entity::Point {position, fixed:false} if position.distance(Position::mm(2.0,8.0)).0 < 1e-9)));
    assert_eq!(s.undo_len(), 1);
}

#[test]
fn pointer_axis_inference_persists_local_relation_without_vector_settings() {
    for enabled in [false, true] {
        let mut s = Session::new(66193);
        let mut settings = s.document.snapping();
        settings.enabled = enabled;
        s.set_snapping(settings);
        s.set_workbench(Workbench::Sketch);
        s.set_sketch_tool(SketchTool::Point);
        let page = s.active_page();
        let point = Point::new(
            page.origin.x + page.size.width.as_mm() / 2.0 + 12.0,
            page.origin.y + page.size.height.as_mm() / 2.0 + 0.1,
        );
        s.pointer_down(point, false);
        s.pointer_up(point, false, false);
        let snapshot = s.document.sketch(s.active_sketch().unwrap()).unwrap();
        assert!(
            snapshot
                .graph
                .constraints
                .values()
                .any(|constraint| matches!(
                    constraint.relation,
                    curvyo_sketch_core::Relation::OnAxis {
                        axis: curvyo_sketch_core::Axis::X,
                        ..
                    }
                ))
        );
        assert!(snapshot.graph.entities.values().any(|entity|matches!(entity,
            curvyo_sketch_core::Entity::Point {position,fixed:false} if position.y.0.abs()<1e-9 && (position.x.0-12.0).abs()<1e-9)));
    }
}

#[test]
fn secondary_press_cancels_each_pick_phase_without_materializing_empty_sketch() {
    for count in [0, 1, 2] {
        let mut s = Session::new(66194);
        s.set_workbench(Workbench::Sketch);
        s.set_sketch_tool(SketchTool::Arc);
        let before = s.document.version();
        for point in [Point::new(125.0, 170.0), Point::new(135.0, 180.0)]
            .into_iter()
            .take(count)
        {
            s.pointer_down(point, false);
            s.pointer_up(point, false, false);
        }
        s.pointer_down(Point::new(145.0, 170.0), false);
        assert!(s.sketch_secondary_press());
        s.pointer_up(Point::new(145.0, 170.0), false, false);
        assert!(s.active_sketch().is_none());
        assert_eq!(s.document.version(), before);
        assert_eq!(s.sketch.picks, [] as [Position; 0]);
        assert!(!s.sketch_secondary_press());
    }
}

#[test]
fn empty_source_page_renders_transient_geometry_without_history() {
    let mut s = Session::new(66195);
    s.set_workbench(Workbench::Sketch);
    s.set_sketch_tool(SketchTool::Rectangle);
    let before = s.document.version();
    s.pointer_down(Point::new(125.0, 170.0), false);
    s.pointer_hover(Point::new(155.0, 190.0), false, false);
    let preview = s.sketch_construction_preview().unwrap();
    assert_eq!(preview.candidate.entities.len(), 9);
    assert_ne!(
        s.sketch_draw_list().triangles,
        [] as [curvyo_render_core::Vertex; 0]
    );
    assert!(s.active_sketch().is_none());
    assert_eq!(s.document.version(), before);
    s.escape();
    assert!(s.sketch_construction_preview().is_none());
    assert_eq!(s.document.version(), before);
}

#[test]
fn quarter_page_targets_and_alt_do_not_attract_sketch_creation() {
    for (enabled, alt) in [(true, false), (true, true), (false, false), (false, true)] {
        let mut s = Session::new(66196);
        let mut settings = s.document.snapping();
        settings.enabled = enabled;
        s.set_snapping(settings);
        s.set_workbench(Workbench::Sketch);
        s.set_sketch_tool(SketchTool::Point);
        s.modifiers_changed(false, false, alt);
        let raw = Point::new(52.6, 74.3);
        let expected = s.sketch_local(raw).unwrap();
        s.pointer_down(raw, false);
        s.pointer_up(raw, false, false);
        let graph = s.document.sketch(s.active_sketch().unwrap()).unwrap().graph;
        assert!(graph.entities.values().any(|entity| matches!(entity,
            curvyo_sketch_core::Entity::Point {position,fixed:false} if position.distance(expected).0<1e-9)));
        assert!(graph.constraints.is_empty());
    }
}

#[test]
fn parameter_first_materializes_only_a_valid_edit_and_drives_later_geometry() {
    use curvyo_sketch_core::Dimension;
    let mut s = Session::new(66197);
    s.set_workbench(Workbench::Sketch);
    let before = s.document.version();
    assert!(
        s.sketch_add_parameter("width", "missing + 1 mm", Dimension::Length)
            .is_err()
    );
    assert_eq!(s.document.version(), before);
    assert!(s.active_sketch().is_none());
    let parameter = s
        .sketch_add_parameter("width", "12 mm", Dimension::Length)
        .unwrap();
    assert_eq!(s.undo_len(), 1);
    let id = s.active_sketch().unwrap();
    assert!(
        s.document
            .sketch(id)
            .unwrap()
            .graph
            .parameters
            .0
            .contains_key(&parameter)
    );
    s.set_sketch_tool(SketchTool::Point);
    s.sketch_complete_numeric("width", "20 mm").unwrap();
    assert_eq!(s.undo_len(), 2);
    assert!(s.document.sketch(id).unwrap().graph.entities.values().any(|entity|matches!(entity,
        curvyo_sketch_core::Entity::Point {position,fixed:false} if position.distance(Position::mm(12.0,20.0)).0<1e-9)));
}

#[test]
fn first_origin_edit_is_durable_but_unchanged_fixed_origin_is_not() {
    let mut s = Session::new(66198);
    s.set_workbench(Workbench::Sketch);
    let before = s.document.version();
    assert!(!s.sketch_fix_origin(true).unwrap());
    assert_eq!(s.document.version(), before);
    assert!(s.active_sketch().is_none());
    assert!(s.sketch_fix_origin(false).unwrap());
    assert_eq!(s.undo_len(), 1);
    let graph = s.document.sketch(s.active_sketch().unwrap()).unwrap().graph;
    assert!(matches!(
        graph.entities[&graph.origin],
        curvyo_sketch_core::Entity::Point { fixed: false, .. }
    ));
}

#[test]
fn newly_active_source_page_reveals_its_origin_even_when_frame_partly_overlaps() {
    let mut s = Session::new(66301);
    s.resize_viewport(1100.0, 950.0);
    s.set_workbench(Workbench::Sketch);
    for _ in 0..3 {
        s.add_page();
        let origin = s
            .sketch_placement()
            .unwrap()
            .apply_point(Point::new(0.0, 0.0));
        let (x, y) = s.view().document_to_screen(origin);
        assert!((0.0..=1100.0).contains(&x), "x={x}");
        assert!((0.0..=950.0).contains(&y), "y={y}");
    }
    s.viewport.pan_by_screen_delta(400.0, 200.0);
    let before = s.view().document_to_screen(Point::new(0.0, 0.0));
    s.set_workbench(Workbench::Vector);
    s.set_workbench(Workbench::Sketch);
    assert_eq!(s.view().document_to_screen(Point::new(0.0, 0.0)), before);
}
