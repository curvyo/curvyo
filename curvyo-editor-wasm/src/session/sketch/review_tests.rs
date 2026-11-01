//! The revision 2 UI review changes through the session: small geometry, badge clearance,
//! dimension kinds, entry refusals and canvas refusals (0066 UX notes, "Revision 2 UI review").
use super::*;
use curvyo_document_core::Point;
use curvyo_sketch_core::{Constraint, Construction, Entity, EntityId, Length, Position, Relation};
use serde_json::Value;

fn sketch_with(commands: &[Construction]) -> (Session, curvyo_document_core::NodeId) {
    let mut s = Session::new(66_900);
    s.set_workbench(Workbench::Sketch);
    let id = s.new_sketch().unwrap();
    for command in commands {
        s.sketch_construct(command).unwrap();
    }
    s.set_sketch_tool(SketchTool::Select);
    (s, id)
}
fn line(from: (f64, f64), to: (f64, f64)) -> Construction {
    Construction::Polyline(vec![Position::mm(from.0, from.1), Position::mm(to.0, to.1)])
}
fn entities(s: &Session, id: curvyo_document_core::NodeId, kind: &str) -> Vec<EntityId> {
    s.document
        .sketch(id)
        .unwrap()
        .graph
        .entities
        .iter()
        .filter(|(_, e)| super::view::entity_name(e) == kind)
        .map(|(id, _)| id.clone())
        .collect()
}
fn add(s: &mut Session, id: &str, relation: Relation) {
    s.sketch_set_constraint(id.into(), Constraint::driving(relation))
        .unwrap();
}
fn symbols(s: &Session) -> Vec<Value> {
    s.sketch_view()["symbols"].as_array().unwrap().clone()
}
fn placed(s: &Session, x: f64, y: f64) -> Point {
    s.sketch_placement().unwrap().apply_point(Point::new(x, y))
}
fn click(s: &mut Session, x: f64, y: f64) {
    let p = placed(s, x, y);
    s.pointer_hover(p, false, false);
    s.pointer_down(p, false);
    s.pointer_up(p, false, false);
}

// ---------------------------------------------------------------- change 1: small geometry

#[test]
fn symbols_of_geometry_under_forty_pixels_show_only_while_hovered_or_selected() {
    let mm = |px: f64, s: &Session| px / s.view().scale();
    let (mut s, id) = sketch_with(&[]);
    let short = mm(30.0, &s);
    let long = mm(120.0, &s);
    s.sketch_construct(&line((0.0, 0.0), (short, 0.0))).unwrap();
    s.sketch_construct(&line((0.0, 40.0), (long, 40.0)))
        .unwrap();
    let lines = entities(&s, id, "line");
    let length = |s: &Session, line: &EntityId| {
        let snapshot = s.document.sketch(id).unwrap();
        let Entity::Line { start, end } = &snapshot.graph.entities[line] else {
            unreachable!()
        };
        let (a, b) = (
            snapshot.solution.point(start).unwrap(),
            snapshot.solution.point(end).unwrap(),
        );
        a.distance(b).0
    };
    let (small, big) = if length(&s, &lines[0]) < length(&s, &lines[1]) {
        (lines[0].clone(), lines[1].clone())
    } else {
        (lines[1].clone(), lines[0].clone())
    };
    add(&mut s, "h-small", Relation::Horizontal(small.clone()));
    add(&mut s, "h-big", Relation::Horizontal(big));
    let shown = |s: &Session| {
        symbols(s)
            .iter()
            .map(|v| v["id"].as_str().unwrap().to_owned())
            .collect::<Vec<_>>()
    };
    assert_eq!(shown(&s), ["h-big"], "the 30px line's symbol is hidden");
    assert!(s.sketch_select_entity(&small, false));
    assert!(
        shown(&s).contains(&"h-small".to_owned()),
        "selected geometry shows its symbols: {:?}",
        shown(&s)
    );
}

// ---------------------------------------------------------------- change 9: badge clearance

#[test]
fn a_fixed_badge_never_covers_its_point_glyph() {
    let (mut s, id) = sketch_with(&[line((0.0, 0.0), (60.0, 0.0))]);
    let point = entities(&s, id, "point")
        .into_iter()
        .find(|p| *p != s.document.sketch(id).unwrap().graph.origin)
        .unwrap();
    add(&mut s, "fixed", Relation::Fixed(point.clone()));
    let snapshot = s.document.sketch(id).unwrap();
    let at = snapshot.solution.point(&point).unwrap();
    let (px, py) = s
        .view()
        .document_to_screen(snapshot.placement.apply_point(Point::new(at.x.0, at.y.0)));
    let badge = symbols(&s)
        .into_iter()
        .find(|v| v["id"] == "fixed")
        .unwrap();
    let (bx, by) = (badge["x"].as_f64().unwrap(), badge["y"].as_f64().unwrap());
    // A 16px badge and the 10px point glyph do not overlap.
    assert!(
        (bx - px).abs() >= 13.0 || (by - py).abs() >= 13.0,
        "badge ({bx}, {by}) over point ({px}, {py})"
    );
}

// ---------------------------------------------------------------- change 2: line plus point

#[test]
fn the_dimension_tool_measures_a_point_to_a_line() {
    let (mut s, id) = sketch_with(&[
        line((0.0, 0.0), (0.0, 30.0)),
        Construction::Circle {
            center: Position::mm(12.5, 15.0),
            radius: Length(3.0),
        },
    ]);
    let edge = entities(&s, id, "line")[0].clone();
    let centre = s
        .document
        .sketch(id)
        .unwrap()
        .graph
        .entities
        .values()
        .find_map(|e| match e {
            Entity::Circle { center, .. } => Some(center.clone()),
            _ => None,
        })
        .unwrap();
    assert!(s.sketch_select_entity(&edge, false));
    assert!(s.sketch_select_entity(&centre, true));
    assert!(s.set_sketch_constraint_tool("dimension"));
    assert_eq!(s.sketch_view()["dimension_action"], "distance");
    // Placing the label opens the entry box with the live distance.
    click(&mut s, 6.0, 40.0);
    let entry = s.sketch_view()["canvas"]["entry"].clone();
    assert_eq!(entry["fields"][0]["label"], "Distance", "{entry}");
    assert_eq!(entry["fields"][0]["value"], "12.5", "{entry}");
    assert!(s.sketch_apply_action("distance", "12.5", false).unwrap());
    let texts: Vec<_> = s.sketch_view()["annotations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["text"].as_str().unwrap().to_owned())
        .collect();
    assert!(texts.contains(&"12.5 mm".to_owned()), "{texts:?}");
}

#[test]
fn an_ineligible_dimension_pick_says_what_the_tool_needs_on_the_canvas() {
    let (mut s, id) = sketch_with(&[
        line((0.0, 0.0), (0.0, 30.0)),
        Construction::Circle {
            center: Position::mm(12.5, 15.0),
            radius: Length(3.0),
        },
    ]);
    let edge = entities(&s, id, "line")[0].clone();
    let circle = entities(&s, id, "circle")[0].clone();
    assert!(s.set_sketch_constraint_tool("dimension"));
    assert!(s.sketch_select_entity(&edge, false));
    assert!(s.sketch_select_entity(&circle, true));
    s.pointer_hover(placed(&s, 20.0, 20.0), false, false);
    let label = s.sketch_view()["canvas"]["mode_label"].clone();
    assert!(
        label["error"]
            .as_str()
            .is_some_and(|e| e.contains("point and a segment")),
        "{label}"
    );
}

// ---------------------------------------------------------------- change 3: radius or diameter

#[test]
fn an_arc_dimensions_its_radius_and_a_circle_its_diameter_and_alt_switches_with_the_label() {
    let (mut s, id) = sketch_with(&[
        Construction::Circle {
            center: Position::mm(0.0, 0.0),
            radius: Length(10.0),
        },
        Construction::ArcCenter {
            center: Position::mm(50.0, 0.0),
            start: Position::mm(75.0, 0.0),
            end: Position::mm(25.0, 0.0),
        },
    ]);
    let arc = entities(&s, id, "arc")[0].clone();
    let circle = entities(&s, id, "circle")[0].clone();
    assert!(s.sketch_select_entity(&arc, false));
    assert!(s.set_sketch_constraint_tool("dimension"));
    assert_eq!(s.sketch_view()["dimension_action"], "radius");
    s.pointer_hover(placed(&s, 60.0, 30.0), false, false);
    assert_eq!(s.sketch_view()["canvas"]["mode_label"]["text"], "Radius");
    s.key_down(crate::session::KeyInput {
        key: "Alt",
        alt: true,
        ..crate::session::KeyInput::default()
    });
    assert_eq!(s.sketch_view()["dimension_action"], "diameter");
    assert_eq!(s.sketch_view()["canvas"]["mode_label"]["text"], "Diameter");
    assert!(s.set_sketch_constraint_tool("dimension"));
    assert!(s.sketch_select_entity(&circle, false));
    assert_eq!(s.sketch_view()["dimension_action"], "diameter");
}

// ---------------------------------------------------------------- change 5: field refusals

fn entry(s: &Session) -> Value {
    s.sketch_view()["canvas"]["entry"].clone()
}

#[test]
fn a_refused_entry_marks_only_its_field_and_stays_out_of_sketches() {
    let (mut s, _) = sketch_with(&[]);
    s.set_sketch_tool(SketchTool::Polyline);
    click(&mut s, 10.0, 10.0);
    s.pointer_hover(placed(&s, 40.0, 10.0), false, false);
    assert!(s.sketch_complete_numeric("0", "0").is_err());
    let box_ = entry(&s);
    assert_eq!(box_["error"]["field"], 0, "{box_}");
    assert_eq!(box_["error"]["message"], "Must be more than 0 mm", "{box_}");
    assert!(
        s.sketch_view()["error"].is_null(),
        "the message lives in the box"
    );
    assert!(s.sketch_complete_numeric("20", "north").is_err());
    assert_eq!(entry(&s)["error"]["field"], 1, "{}", entry(&s));
    // A valid entry commits and clears the message.
    assert!(s.sketch_complete_numeric("20", "0").unwrap());
    assert!(entry(&s)["error"].is_null(), "{}", entry(&s));
}

// ---------------------------------------------------------------- change 6: unsolvable value

#[test]
fn a_dimension_that_leaves_no_valid_shape_keeps_its_box_open_with_the_reason_and_is_marked() {
    let (mut s, id) = sketch_with(&[line((0.0, 0.0), (20.0, 0.0))]);
    let edge = entities(&s, id, "line")[0].clone();
    assert!(s.sketch_select_entity(&edge, false));
    assert!(s.set_sketch_constraint_tool("dimension"));
    click(&mut s, 10.0, 10.0);
    assert!(s.sketch_apply_action("length", "20", false).unwrap());
    assert!(entry(&s).is_null(), "a solvable value closes the box");
    assert!(s.sketch_select_entity(&edge, false));
    // The first dimension returned to Select (AC43): arm Dimension again.
    assert!(s.set_sketch_constraint_tool("dimension"));
    click(&mut s, 10.0, -10.0);
    assert!(s.sketch_apply_action("length", "30", false).unwrap());
    let box_ = entry(&s);
    assert_eq!(
        box_["kind"], "edit",
        "the box stays on the new dimension: {box_}"
    );
    assert_eq!(box_["error"]["field"], 0, "{box_}");
    assert!(
        box_["error"]["message"]
            .as_str()
            .unwrap()
            .contains("last shape"),
        "{box_}"
    );
    let new = box_["constraint"].as_str().unwrap().to_owned();
    let marked = s.sketch_view()["annotations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["id"] == new.as_str())
        .unwrap()
        .clone();
    assert!(
        marked["text"].as_str().unwrap().starts_with("! "),
        "{marked}"
    );
    // Typing the solvable value into the box repairs it in place.
    assert!(
        s.sketch_dimension_text(&new.as_str().into(), "20", false)
            .unwrap()
    );
    assert!(entry(&s)["error"].is_null(), "{}", entry(&s));
}

// ---------------------------------------------------------------- change 7: canvas refusals

#[test]
fn a_refused_pick_is_said_on_the_mode_label_not_only_in_sketches() {
    let (mut s, _) = sketch_with(&[]);
    s.set_sketch_tool(SketchTool::Polyline);
    click(&mut s, 10.0, 10.0);
    click(&mut s, 40.0, 10.0);
    assert!(s.set_sketch_mode("tangent-arc"));
    // On the tangent's own line the arc is degenerate: the click is refused.
    click(&mut s, 70.0, 10.0);
    let label = s.sketch_view()["canvas"]["mode_label"].clone();
    assert_eq!(label["text"], "Tangent arc", "{label}");
    assert_eq!(label["error"], super::creation::DEGENERATE, "{label}");
    assert!(
        s.sketch_view()["error"].is_null(),
        "shown on the canvas instead"
    );
}
