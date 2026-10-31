//! Revised selection gestures never modify analytic geometry.
use super::*;
use curvyo_document_core::Point;
use curvyo_sketch_core::{Construction, Position};
fn overlapping_lines() -> Session {
    let mut s = Session::new(66240);
    s.set_workbench(Workbench::Sketch);
    for _ in 0..2 {
        s.sketch_construct(&Construction::Polyline(vec![
            Position::mm(10.0, 10.0),
            Position::mm(30.0, 10.0),
        ]))
        .unwrap();
    }
    s.set_sketch_tool(SketchTool::Select);
    s
}
#[test]
fn alt_click_cycles_tied_edges_without_dragging_the_selected_geometry() {
    let mut s = overlapping_lines();
    let map = s.sketch_placement().unwrap();
    let p = map.apply_point(Point::new(20.0, 10.0));
    let version = s.document.version();
    s.modifiers_changed(false, false, true);
    s.pointer_down(p, false);
    s.pointer_up(p, false, false);
    let first = s.sketch.entities.clone();
    assert_eq!(first.len(), 1);
    s.pointer_down(p, false);
    s.pointer_up(p, false, false);
    assert_eq!(s.sketch.entities.len(), 1);
    assert_ne!(s.sketch.entities, first);
    assert_eq!(s.document.version(), version);
}
#[test]
fn alt_stroke_selects_touched_edges_and_ctrl_alt_removes_without_copy_or_move() {
    let mut s = overlapping_lines();
    let map = s.sketch_placement().unwrap();
    let from = map.apply_point(Point::new(20.0, -2.0));
    let to = map.apply_point(Point::new(20.0, 22.0));
    let version = s.document.version();
    s.modifiers_changed(false, false, true);
    s.pointer_down(from, false);
    s.pointer_hover(to, false, false);
    s.pointer_up(to, false, false);
    assert_eq!(s.sketch.entities.len(), 2);
    s.modifiers_changed(false, true, true);
    s.pointer_down(from, false);
    s.pointer_hover(to, false, true);
    s.pointer_up(to, false, true);
    assert_eq!(s.sketch.entities.len(), 0);
    assert_eq!(s.document.version(), version);
}
#[test]
fn numeric_focus_values_are_on_demand_and_leave_history_unchanged() {
    let mut s = Session::new(66241);
    s.set_workbench(Workbench::Sketch);
    s.set_sketch_tool(SketchTool::Polyline);
    let map = s.sketch_placement().unwrap();
    let start = map.apply_point(Point::new(10.0, 10.0));
    s.pointer_down(start, false);
    s.pointer_up(start, false, false);
    s.pointer_hover(map.apply_point(Point::new(13.0, 14.0)), false, false);
    let version = s.document.version();
    let values = s.sketch_numeric_values("");
    assert!((values[0].parse::<f64>().unwrap() - 5.0).abs() < 1e-9);
    assert!((values[1].parse::<f64>().unwrap() - 53.130_102_354_155_98).abs() < 1e-9);
    assert_eq!(s.document.version(), version);
}
