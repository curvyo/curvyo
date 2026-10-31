//! Tab entry in the follow state and during a press-drag; typed sizes become driving dimensions
//! (0066 AC39, AC41).
use super::*;
use curvyo_document_core::Point;
use curvyo_sketch_core::{Expression, Relation, SolveStatus};

fn global(s: &Session, x: f64, y: f64) -> Point {
    s.sketch_placement().unwrap().apply_point(Point::new(x, y))
}

fn click(s: &mut Session, x: f64, y: f64) {
    let p = global(s, x, y);
    s.pointer_hover(p, false, false);
    s.pointer_down(p, false);
    s.pointer_up(p, false, false);
}

fn session(tool: SketchTool) -> Session {
    let mut s = Session::new(66_930);
    s.set_workbench(Workbench::Sketch);
    s.set_sketch_tool(tool);
    s
}

/// The labels and live values of the entry box Tab opens now.
fn fields(s: &Session) -> Vec<(String, String)> {
    s.sketch_view()["canvas"]["entry"]["fields"]
        .as_array()
        .unwrap_or_else(|| panic!("no entry box"))
        .iter()
        .map(|f| {
            (
                f["label"].as_str().unwrap().to_owned(),
                f["value"].as_str().unwrap().to_owned(),
            )
        })
        .collect()
}

/// Enter in the box: typed fields carry `typed`, the rest keep their live values.
fn enter(s: &mut Session, typed: &[Option<&str>]) {
    let live = fields(s);
    let texts: Vec<String> = live
        .iter()
        .zip(typed.iter().chain(std::iter::repeat(&None)))
        .map(|((_, value), typed)| typed.map_or_else(|| value.clone(), str::to_owned))
        .collect();
    let texts: Vec<&str> = texts.iter().map(String::as_str).collect();
    let mask: Vec<bool> = live
        .iter()
        .enumerate()
        .map(|(i, _)| typed.get(i).is_some_and(Option::is_some))
        .collect();
    s.sketch_complete_fields(&texts, &mask).unwrap();
}

fn relations(s: &Session) -> Vec<Relation> {
    let id = s.active_sketch().unwrap();
    s.document
        .sketch(id)
        .unwrap()
        .graph
        .constraints
        .values()
        .map(|c| c.relation.clone())
        .collect()
}

fn dof(s: &Session) -> usize {
    let id = s.active_sketch().unwrap();
    match s.document.sketch(id).unwrap().status {
        Some(SolveStatus::UnderConstrained(n)) => n,
        Some(SolveStatus::FullyConstrained) => 0,
        other => panic!("{other:?}"),
    }
}

fn value_mm(expression: &Expression) -> f64 {
    expression
        .evaluate(&std::collections::BTreeMap::new())
        .unwrap()
        .value()
}

#[test]
fn a_rectangle_with_only_its_width_typed_gets_one_width_dimension() {
    let mut s = session(SketchTool::Rectangle);
    click(&mut s, 20.0, 20.0);
    s.pointer_hover(global(&s, 50.0, 35.0), false, false);
    let labels: Vec<_> = fields(&s).into_iter().map(|(l, _)| l).collect();
    assert_eq!(labels, ["Width", "Height"]);
    enter(&mut s, &[Some("40")]);
    assert_eq!(s.undo_len(), 1, "one history step");
    let lengths: Vec<_> = relations(&s)
        .into_iter()
        .filter_map(|r| match r {
            Relation::Length { value, .. } => Some(value_mm(&value)),
            _ => None,
        })
        .collect();
    assert_eq!(lengths.len(), 1, "a width dimension, no height dimension");
    assert!((lengths[0] - 40.0).abs() < 1e-9);
    // Four corners (8) minus 4 horizontal/vertical minus the width: position and height stay.
    assert_eq!(dof(&s), 3);
}

#[test]
fn circle_radius_and_line_length_become_dimensions_and_untyped_stay_free() {
    let mut s = session(SketchTool::Circle);
    click(&mut s, 30.0, 30.0);
    s.pointer_hover(global(&s, 40.0, 30.0), false, false);
    assert_eq!(fields(&s)[0].0, "Radius");
    enter(&mut s, &[Some("12")]);
    assert!(relations(&s).iter().any(
        |r| matches!(r, Relation::Radius { value, .. } if (value_mm(value) - 12.0).abs() < 1e-9)
    ));
    assert_eq!(dof(&s), 2, "the centre stays free");

    let mut s = session(SketchTool::Polyline);
    click(&mut s, 30.0, 30.0);
    s.pointer_hover(global(&s, 50.0, 40.0), false, false);
    let labels: Vec<_> = fields(&s).into_iter().map(|(l, _)| l).collect();
    assert_eq!(labels, ["Length", "Angle"]);
    enter(&mut s, &[Some("25")]);
    let lengths = relations(&s)
        .iter()
        .filter(|r| matches!(r, Relation::Length { .. }))
        .count();
    assert_eq!(lengths, 1);
    assert_eq!(dof(&s), 3, "the angle stays free");
}

#[test]
fn a_slot_takes_length_angle_and_width_in_its_follow_state() {
    let mut s = session(SketchTool::Slot);
    click(&mut s, 30.0, 30.0);
    s.pointer_hover(global(&s, 50.0, 30.0), false, false);
    let labels: Vec<_> = fields(&s).into_iter().map(|(l, _)| l).collect();
    assert_eq!(labels, ["Length", "Angle", "Width"]);
    enter(&mut s, &[Some("30"), None, Some("10")]);
    assert_eq!(s.undo_len(), 1);
    let rels = relations(&s);
    assert!(rels.iter().any(
        |r| matches!(r, Relation::Distance { value, .. } if (value_mm(value) - 30.0).abs() < 1e-9)
    ));
    assert!(rels.iter().any(
        |r| matches!(r, Relation::Diameter { value, .. } if (value_mm(value) - 10.0).abs() < 1e-9)
    ));
    // Without a width the spine is set and the width pick follows.
    let mut s = session(SketchTool::Slot);
    click(&mut s, 30.0, 30.0);
    s.pointer_hover(global(&s, 50.0, 30.0), false, false);
    enter(&mut s, &[Some("30")]);
    assert_eq!(s.undo_len(), 0);
    assert_eq!(s.sketch.picks.len(), 2);
    click(&mut s, 45.0, 36.0);
    assert_eq!(s.undo_len(), 1);
    assert!(relations(&s).iter().any(
        |r| matches!(r, Relation::Distance { value, .. } if (value_mm(value) - 30.0).abs() < 1e-9)
    ));
}

#[test]
fn the_same_fields_open_during_a_press_drag_and_the_release_then_waits() {
    let mut s = session(SketchTool::Rectangle);
    let (a, b) = (global(&s, 20.0, 20.0), global(&s, 50.0, 35.0));
    s.pointer_hover(a, false, false);
    s.pointer_down(a, false);
    s.pointer_hover(b, false, false);
    let labels: Vec<_> = fields(&s).into_iter().map(|(l, _)| l).collect();
    assert_eq!(labels, ["Width", "Height"]);
    s.sketch_open_entry();
    s.pointer_up(b, false, false);
    assert_eq!(
        s.undo_len(),
        0,
        "the release does not commit while the box is open"
    );
    enter(&mut s, &[None, Some("8")]);
    assert_eq!(s.undo_len(), 1);
    let lengths = relations(&s)
        .iter()
        .filter(
            |r| matches!(r, Relation::Length { value, .. } if (value_mm(value) - 8.0).abs() < 1e-9),
        )
        .count();
    assert_eq!(lengths, 1, "the height dimension only");
}

#[test]
fn a_typed_line_angle_becomes_a_direction_dimension_and_removes_one_freedom() {
    let mut s = session(SketchTool::Polyline);
    click(&mut s, 30.0, 30.0);
    s.pointer_hover(global(&s, 50.0, 40.0), false, false);
    enter(&mut s, &[Some("25"), Some("30")]);
    let angles: Vec<_> = relations(&s)
        .into_iter()
        .filter_map(|r| match r {
            Relation::AxisAngle { value, .. } => Some(
                value
                    .evaluate(&std::collections::BTreeMap::new())
                    .unwrap()
                    .value(),
            ),
            _ => None,
        })
        .collect();
    assert_eq!(angles.len(), 1);
    assert!((angles[0].to_degrees() - 30.0).abs() < 1e-9);
    assert_eq!(dof(&s), 2, "length and angle typed: the start stays free");
}

#[test]
fn a_typed_slot_angle_and_ellipse_radii_become_dimensions() {
    let mut s = session(SketchTool::Slot);
    click(&mut s, 30.0, 30.0);
    s.pointer_hover(global(&s, 50.0, 30.0), false, false);
    enter(&mut s, &[Some("30"), Some("45"), Some("10")]);
    assert!(
        relations(&s)
            .iter()
            .any(|r| matches!(r, Relation::AxisAngle { .. }))
    );
    assert_eq!(dof(&s), 2, "only the position stays free");

    let mut s = session(SketchTool::Ellipse);
    click(&mut s, 30.0, 30.0);
    s.pointer_hover(global(&s, 50.0, 30.0), false, false);
    enter(&mut s, &[Some("20"), Some("8")]);
    let radii: Vec<_> = relations(&s)
        .into_iter()
        .filter_map(|r| match r {
            Relation::EllipseRadius { axis, value, .. } => Some((axis, value_mm(&value))),
            _ => None,
        })
        .collect();
    assert_eq!(
        radii,
        [
            (curvyo_sketch_core::EllipseAxis::Major, 20.0),
            (curvyo_sketch_core::EllipseAxis::Minor, 8.0)
        ]
    );
    // A free ellipse has 5: centre, focus and minor radius.
    assert_eq!(dof(&s), 3, "centre and rotation stay free");
}
