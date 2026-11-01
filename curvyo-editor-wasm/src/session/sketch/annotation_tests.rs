//! Rail constraint card, dimension annotations and constraint symbols through the session (AC24, AC31, AC32).
use super::*;
use curvyo_document_core::Point;
use curvyo_sketch_core::{
    Constraint, Construction, Expression, Length, Position, Quantity, Relation,
};
use serde_json::Value;

fn sketch_with(commands: &[Construction]) -> (Session, curvyo_document_core::NodeId) {
    let mut s = Session::new(66_700);
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
fn placed(s: &Session, x: f64, y: f64) -> Point {
    s.sketch_placement().unwrap().apply_point(Point::new(x, y))
}
fn click(s: &mut Session, x: f64, y: f64) {
    let p = placed(s, x, y);
    s.pointer_hover(p, false, false);
    s.pointer_down(p, false);
    s.pointer_up(p, false, false);
}
fn lines(s: &Session, id: curvyo_document_core::NodeId) -> Vec<curvyo_sketch_core::EntityId> {
    let graph = s.document.sketch(id).unwrap().graph;
    graph
        .entities
        .iter()
        .filter(|(_, e)| matches!(e, curvyo_sketch_core::Entity::Line { .. }))
        .map(|(id, _)| id.clone())
        .collect()
}
fn rail(s: &Session, tool: &str) -> Value {
    s.sketch_view()["rail"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["name"] == tool)
        .unwrap()
        .clone()
}
fn kinds(s: &Session, id: curvyo_document_core::NodeId) -> Vec<&'static str> {
    s.document
        .sketch(id)
        .unwrap()
        .graph
        .constraints
        .values()
        .map(|c| super::view::relation_name(&c.relation))
        .collect()
}
fn length(value: f64) -> Expression {
    Expression::Literal(Quantity::length(value).unwrap())
}
fn add(s: &mut Session, id: &str, relation: Relation, reference: bool) {
    s.sketch_set_constraint(
        id.into(),
        Constraint {
            relation,
            reference,
        },
    )
    .unwrap();
}
fn annotation(s: &Session, id: &str) -> Value {
    s.sketch_view()["annotations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["id"] == id)
        .unwrap_or_else(|| panic!("no annotation {id}"))
        .clone()
}
fn screen_point(s: &Session, annotation: &Value) -> Point {
    s.view().screen_to_document(
        annotation["x"].as_f64().unwrap(),
        annotation["y"].as_f64().unwrap(),
    )
}

#[test]
fn the_rail_lists_thirteen_tools_and_classifies_the_selection() {
    let (mut s, id) = sketch_with(&[
        line((20.0, 20.0), (40.0, 30.0)),
        line((20.0, 50.0), (40.0, 60.0)),
    ]);
    let names: Vec<_> = s.sketch_view()["rail"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap().to_owned())
        .collect();
    // Revision 4 adds Horizontal distance and Vertical distance (AC44).
    assert_eq!(names.len(), 13);
    assert!(
        s.sketch_view()["rail"]
            .as_array()
            .unwrap()
            .iter()
            .all(|t| t["state"] == "arm")
    );
    let ids = lines(&s, id);
    s.sketch.entities.insert(ids[0].clone());
    assert_eq!(rail(&s, "horizontal")["state"], "apply");
    assert_eq!(rail(&s, "parallel")["state"], "arm");
    // A line and a point make a point-on Coincident (AC37); only Tangent has no completion.
    assert_eq!(rail(&s, "coincident")["state"], "arm");
    assert_eq!(rail(&s, "tangent")["state"], "unavailable");
    assert_eq!(
        rail(&s, "tangent")["tooltip"],
        "Tangent: select two tangent-capable curves"
    );
}

#[test]
fn an_eligible_selection_applies_at_once_in_one_step_and_stays_selected() {
    let (mut s, id) = sketch_with(&[line((20.0, 20.0), (40.0, 30.0))]);
    let ids = lines(&s, id);
    s.sketch.entities.insert(ids[0].clone());
    let before = s.document.version();
    assert!(s.sketch_rail_press("horizontal").unwrap());
    assert_ne!(s.document.version(), before);
    assert!(kinds(&s, id).contains(&"horizontal"));
    assert!(s.sketch.entities.contains(&ids[0]));
    assert_eq!(s.sketch_tool(), SketchTool::Select);
    // One history step: one undo removes it.
    s.undo();
    assert!(!kinds(&s, id).contains(&"horizontal"));
}

#[test]
fn an_unavailable_tool_applies_nothing_and_says_what_it_needs_on_the_canvas() {
    let (mut s, id) = sketch_with(&[Construction::Circle {
        center: Position::mm(30.0, 30.0),
        radius: Length(5.0),
    }]);
    let circle = s
        .document
        .sketch(id)
        .unwrap()
        .graph
        .entities
        .iter()
        .find(|(_, entity)| matches!(entity, curvyo_sketch_core::Entity::Circle { .. }))
        .map(|(e, _)| e.clone())
        .unwrap();
    s.sketch.entities.insert(circle);
    let before = s.document.version();
    let steps = s.undo_len();
    assert_eq!(rail(&s, "horizontal")["state"], "unavailable");
    assert!(!s.sketch_rail_press("horizontal").unwrap());
    assert_eq!(s.document.version(), before);
    assert_eq!(s.undo_len(), steps);
    // AC40: the tool waits for picks and names what it needs at the pointer.
    assert_eq!(rail(&s, "horizontal")["pressed"], true);
    let p = placed(&s, 60.0, 60.0);
    s.pointer_hover(p, false, false);
    let label = &s.sketch_view()["canvas"]["mode_label"];
    assert_eq!(label["text"], "Horizontal");
    assert_eq!(
        label["error"],
        "Horizontal: select one segment or two points"
    );
}

#[test]
fn an_armed_tool_applies_on_the_last_pick_and_returns_to_select() {
    let (mut s, id) = sketch_with(&[
        line((20.0, 20.0), (60.0, 30.0)),
        line((20.0, 50.0), (60.0, 70.0)),
    ]);
    assert!(s.sketch_rail_press("parallel").unwrap());
    assert_eq!(rail(&s, "parallel")["pressed"], true);
    click(&mut s, 40.0, 25.0);
    assert!(!kinds(&s, id).contains(&"parallel"));
    click(&mut s, 40.0, 60.0);
    assert!(kinds(&s, id).contains(&"parallel"), "{:?}", kinds(&s, id));
    // AC43 (revision 4): back in Select with both lines selected.
    assert_eq!(s.sketch.entities.len(), 2);
    assert_eq!(s.sketch.constraint_tool, None);
    assert_eq!(s.sketch_tool(), SketchTool::Select);
}

#[test]
fn dimensions_show_formatted_text_at_a_screen_constant_offset() {
    let (mut s, id) = sketch_with(&[
        line((0.0, 0.0), (50.0, 0.0)),
        Construction::Circle {
            center: Position::mm(0.0, 60.0),
            radius: Length(25.0),
        },
    ]);
    let top = lines(&s, id)[0].clone();
    let graph = s.document.sketch(id).unwrap().graph;
    let circle = graph
        .entities
        .iter()
        .find(|(_, e)| matches!(e, curvyo_sketch_core::Entity::Circle { .. }))
        .map(|(id, _)| id.clone())
        .unwrap();
    add(
        &mut s,
        "len",
        Relation::Length {
            curve: top.clone(),
            value: length(50.0),
        },
        false,
    );
    add(
        &mut s,
        "rad",
        Relation::Radius {
            curve: circle.clone(),
            value: length(25.0),
        },
        false,
    );
    add(
        &mut s,
        "dia",
        Relation::Diameter {
            curve: circle,
            value: length(50.0),
        },
        true,
    );
    assert_eq!(annotation(&s, "len")["text"], "50 mm");
    assert_eq!(annotation(&s, "rad")["text"], "R 25 mm");
    assert_eq!(annotation(&s, "dia")["text"], "(⌀ 50 mm)");
    assert_eq!(annotation(&s, "dia")["reference"], true);
    // The length text stands 24px above the line at every zoom.
    for _ in 0..2 {
        let line_y = s.view().document_to_screen(placed(&s, 25.0, 0.0)).1;
        let text = annotation(&s, "len");
        assert!((text["y"].as_f64().unwrap() - (line_y - 24.0)).abs() < 1e-6);
        s.wheel(0.0, -500.0, 300.0, 300.0, false, true);
    }
    // Sketch-only marks: dimensions add triangles in the driving colour.
    let colors = s.sketch_draw_list();
    assert!(
        colors
            .triangles
            .iter()
            .any(|v| v.color == curvyo_render_core::RgbaColor::opaque(0xBF, 0x29, 0x33))
    );
}

#[test]
fn dragging_a_dimension_text_stores_its_place_in_one_step_and_escape_restores() {
    let (mut s, id) = sketch_with(&[line((0.0, 0.0), (50.0, 0.0))]);
    let top = lines(&s, id)[0].clone();
    add(
        &mut s,
        "len",
        Relation::Length {
            curve: top,
            value: length(50.0),
        },
        false,
    );
    let text = annotation(&s, "len");
    let press = screen_point(&s, &text);
    s.pointer_hover(press, false, false);
    assert_eq!(s.sketch_view()["cursor"], "grab");
    s.pointer_down(press, false);
    assert!(s.sketch.constraints.contains(&"len".into()));
    let to = s.view().screen_to_document(
        text["x"].as_f64().unwrap() + 40.0,
        text["y"].as_f64().unwrap() - 30.0,
    );
    s.pointer_hover(to, false, false);
    assert_eq!(s.sketch_view()["cursor"], "grabbing");
    // Escape mid-drag restores the start state without a step.
    let before = s.document.version();
    s.escape();
    assert_eq!(s.document.version(), before);
    assert!(
        (annotation(&s, "len")["x"].as_f64().unwrap() - text["x"].as_f64().unwrap()).abs() < 1e-6
    );
    // A completed drag stores the label once.
    s.pointer_down(press, false);
    s.pointer_hover(to, false, false);
    s.pointer_up(to, false, false);
    assert_ne!(s.document.version(), before);
    let moved = annotation(&s, "len");
    assert!((moved["x"].as_f64().unwrap() - text["x"].as_f64().unwrap() - 40.0).abs() < 1e-6);
    assert!((moved["y"].as_f64().unwrap() - text["y"].as_f64().unwrap() + 30.0).abs() < 1e-6);
    assert!(
        s.document
            .sketch(id)
            .unwrap()
            .labels
            .contains_key(&"len".into())
    );
    s.undo();
    assert!(
        !s.document
            .sketch(id)
            .unwrap()
            .labels
            .contains_key(&"len".into())
    );
}

#[test]
fn double_click_on_a_dimension_opens_the_entry_box_at_its_text() {
    let (mut s, id) = sketch_with(&[line((0.0, 0.0), (50.0, 0.0))]);
    let top = lines(&s, id)[0].clone();
    add(
        &mut s,
        "len",
        Relation::Length {
            curve: top,
            value: length(50.0),
        },
        false,
    );
    let text = annotation(&s, "len");
    s.double_click(screen_point(&s, &text), false, false);
    let entry = s.sketch_view()["canvas"]["entry"].clone();
    assert_eq!(entry["kind"], "edit");
    assert_eq!(entry["constraint"], "len");
    assert_eq!(entry["fields"][0]["value"], "50");
    assert_eq!(entry["x"], text["x"]);
    assert_eq!(annotation(&s, "len")["editing"], true);
    s.sketch_dimension_text(&"len".into(), "40", false).unwrap();
    assert_eq!(annotation(&s, "len")["text"], "40 mm");
    assert!(s.sketch_view()["canvas"]["entry"].is_null());
}

fn symbols(s: &Session) -> Vec<Value> {
    s.sketch_view()["symbols"].as_array().unwrap().clone()
}

#[test]
fn a_slot_shows_equal_and_tangent_symbols_and_no_symbol_on_chain_joints() {
    let (s, _) = sketch_with(&[
        Construction::Slot {
            start: Position::mm(0.0, 0.0),
            end: Position::mm(50.0, 0.0),
            width: Length(20.0),
        },
        Construction::Polyline(vec![
            Position::mm(0.0, 40.0),
            Position::mm(30.0, 40.0),
            Position::mm(30.0, 60.0),
        ]),
    ]);
    let glyphs: Vec<_> = symbols(&s)
        .iter()
        .map(|v| v["glyph"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(
        glyphs.iter().filter(|g| *g == "equal").count(),
        2,
        "{glyphs:?}"
    );
    assert_eq!(
        glyphs.iter().filter(|g| *g == "tangent").count(),
        4,
        "{glyphs:?}"
    );
    assert!(!glyphs.contains(&"coincident".to_owned()), "{glyphs:?}");
    for symbol in symbols(&s).iter().filter(|v| v["glyph"] == "equal") {
        assert_eq!(symbol["index"], 1);
        assert_eq!(symbol["name"], "Equal constraint 1");
    }
    // The slot's radius and spacing are dimensions; its structural sweeps are not.
    let texts: Vec<_> = s.sketch_view()["annotations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["text"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(texts, ["R 10 mm", "50 mm"], "{texts:?}");
}

#[test]
fn clicking_a_symbol_selects_its_constraint_and_participants() {
    let (mut s, id) = sketch_with(&[line((20.0, 20.0), (60.0, 20.0))]);
    let top = lines(&s, id)[0].clone();
    add(&mut s, "h", Relation::Horizontal(top.clone()), false);
    let symbol = symbols(&s)[0].clone();
    assert_eq!(symbol["glyph"], "horizontal");
    assert_eq!(symbol["name"], "Horizontal constraint");
    s.pointer_down(screen_point(&s, &symbol), false);
    assert!(s.sketch.constraints.contains(&"h".into()));
    assert!(s.sketch.entities.contains(&top));
    assert_eq!(symbols(&s)[0]["selected"], true);
}

#[test]
fn fully_constrained_geometry_draws_in_the_solved_colour() {
    let (mut s, id) = sketch_with(&[line((20.0, 20.0), (60.0, 20.0))]);
    let solved = |s: &Session| {
        s.sketch_draw_list()
            .triangles
            .iter()
            .any(|v| v.color == curvyo_render_core::RgbaColor::opaque(0x12, 0x73, 0x3A))
    };
    assert!(!solved(&s));
    let top = lines(&s, id)[0].clone();
    add(&mut s, "fix", Relation::Fixed(top), false);
    s.sketch.entities.clear();
    assert!(solved(&s));
}

#[test]
fn a_redundant_dimension_is_named_redundant_not_conflicting() {
    let (mut s, id) = sketch_with(&[line((0.0, 0.0), (40.0, 0.0))]);
    let l = lines(&s, id)[0].clone();
    for name in ["d1", "d2"] {
        add(
            &mut s,
            name,
            Relation::Length {
                curve: l.clone(),
                value: length(40.0),
            },
            false,
        );
    }
    let snapshot = s.document.sketch(id).unwrap();
    let redundant: Vec<_> = snapshot.redundant.iter().map(|c| c.0.clone()).collect();
    assert!(
        snapshot.conflicting.is_empty(),
        "{:?}",
        snapshot.conflicting
    );
    assert!(!redundant.is_empty(), "two equal lengths are redundant");
    for name in &redundant {
        let a = annotation(&s, name);
        assert_eq!(a["redundant"], true, "{a}");
        assert_eq!(a["conflicting"], false, "{a}");
        let label = a["name"].as_str().unwrap();
        assert!(label.ends_with(", redundant"), "{label}");
        // The same non-colour marker as a conflict (UX notes, "States").
        assert!(a["text"].as_str().unwrap().starts_with("! "), "{a}");
    }
}
