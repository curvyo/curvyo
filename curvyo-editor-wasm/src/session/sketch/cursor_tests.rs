//! The Sketch cursor follows hover, press, drag, release and Escape in the event path (AC30).
use super::*;
use curvyo_document_core::{Point, SketchDelta};
use curvyo_sketch_core::{Constraint, Construction, Length, Position, Relation};

fn placed(s: &Session, x: f64, y: f64) -> Point {
    s.sketch_placement().unwrap().apply_point(Point::new(x, y))
}

fn cursor(s: &Session) -> String {
    let view: serde_json::Value = serde_json::from_str(&s.sketch_view_json()).unwrap();
    view["cursor"].as_str().unwrap().to_owned()
}

fn circle_session() -> Session {
    let mut s = Session::new(66_700);
    s.set_workbench(Workbench::Sketch);
    s.new_sketch().unwrap();
    s.sketch_construct(&Construction::Circle {
        center: Position::mm(30.0, 30.0),
        radius: Length(10.0),
    })
    .unwrap();
    s.set_sketch_tool(SketchTool::Select);
    s
}

#[test]
fn select_cursor_is_grab_on_hover_grabbing_while_dragging_and_resets() {
    let mut s = circle_session();
    s.pointer_hover(placed(&s, 60.0, 60.0), false, false);
    assert_eq!(cursor(&s), "default", "empty canvas");
    s.pointer_hover(placed(&s, 40.0, 30.0), false, false);
    assert_eq!(cursor(&s), "grab", "hover over the circle edge");
    s.pointer_down(placed(&s, 40.0, 30.0), false);
    assert_eq!(cursor(&s), "grabbing", "press");
    s.pointer_hover(placed(&s, 44.0, 33.0), false, false);
    assert_eq!(cursor(&s), "grabbing", "drag");
    s.pointer_up(placed(&s, 44.0, 33.0), false, false);
    assert_eq!(cursor(&s), "grab", "release over the moved circle");
    s.pointer_down(placed(&s, 44.0, 33.0), false);
    s.pointer_hover(placed(&s, 50.0, 40.0), false, false);
    assert_eq!(cursor(&s), "grabbing");
    s.escape();
    assert_eq!(
        cursor(&s),
        "default",
        "Escape restores; nothing under the pointer"
    );
}

#[test]
fn fixed_geometry_and_drawing_tools_keep_their_normal_cursor() {
    let mut s = circle_session();
    let circle = s
        .document
        .sketch(s.active_sketch().unwrap())
        .unwrap()
        .graph
        .entities
        .iter()
        .find_map(|(id, e)| {
            matches!(e, curvyo_sketch_core::Entity::Circle { .. }).then(|| id.clone())
        })
        .unwrap();
    let id: curvyo_sketch_core::ConstraintId = s.minter.mint().to_hex().into();
    s.sketch_apply_delta(&SketchDelta {
        constraints: [(id, Some(Constraint::driving(Relation::Fixed(circle))))].into(),
        ..Default::default()
    })
    .unwrap();
    s.pointer_hover(placed(&s, 40.0, 30.0), false, false);
    assert_eq!(cursor(&s), "default", "fully fixed");
    s.pointer_down(placed(&s, 40.0, 30.0), false);
    assert_eq!(cursor(&s), "default", "press on fixed geometry");
    s.pointer_up(placed(&s, 40.0, 30.0), false, false);
    s.pointer_hover(placed(&s, 0.0, 0.0), false, false);
    assert_eq!(cursor(&s), "default", "fixed origin");
    s.set_sketch_tool(SketchTool::Polyline);
    s.pointer_hover(placed(&s, 40.0, 30.0), false, false);
    assert_eq!(cursor(&s), "crosshair");
}
