//! The session side of sketch instances (`specs/0099-sketch-pages` criteria 23, 24, 26, 27): Expose
//! on the sketch page, the Parameters row of a selected clone, a typed value with its notice, Own,
//! Original's, Back, and the stored result after Undo.
#![allow(clippy::unwrap_used)]

use curvyo_document_core::{Document, ObjectSnapshot, PageKind, Point, Style, Vec2, pack};
use curvyo_editor_wasm::{Session, Tool};
use curvyo_sketch_core as sk;

fn graph() -> sk::Sketch {
    let mut s = sk::Sketch::new("origin".into());
    s.entities.insert(
        "end".into(),
        sk::Entity::Point {
            position: sk::Position::mm(60.0, 0.0),
            fixed: false,
        },
    );
    s.entities.insert(
        "line".into(),
        sk::Entity::Line {
            start: "origin".into(),
            end: "end".into(),
        },
    );
    s.parameters.insert(
        "width".into(),
        sk::Parameter::literal("width", sk::Quantity::length(60.0).unwrap()),
    );
    s.constraints.insert(
        "horizontal".into(),
        sk::Constraint::driving(sk::Relation::Horizontal("line".into())),
    );
    s.constraints.insert(
        "length".into(),
        sk::Constraint::driving(sk::Relation::Length {
            curve: "line".into(),
            value: sk::Expression::Reference("width".into()),
        }),
    );
    s
}

fn scene() -> (Session, String, Point) {
    let d = Document::new(9940);
    let page = d.add_page_of(&PageKind::Sketch).unwrap();
    let sketch = d
        .create_page_sketch(&page.id, &graph(), &Style::default())
        .unwrap();
    let clone = d.place_linked_clone(sketch, Vec2::new(0.0, 400.0)).unwrap();
    let read = d.read_objects();
    let ObjectSnapshot::Clone(c) = read.get(clone).unwrap() else {
        panic!("clone")
    };
    let anchor = c.resolved.anchors[0].point;
    let at = Point::new(anchor.x + 1.0, anchor.y);
    let mut s = Session::open(9941, &pack(&d, "t").unwrap()).unwrap();
    s.resize_viewport(1920.0, 1080.0);
    (s, page.id, at)
}

fn view(s: &Session) -> serde_json::Value {
    serde_json::from_str(&s.sketch_view_json()).unwrap()
}

fn select_clone(s: &mut Session, at: Point) {
    s.activate_page(curvyo_document_core::FIRST_PAGE_ID);
    s.set_tool(Tool::Select);
    s.pointer_down(at, false);
    s.pointer_up(at, false, false);
}

#[test]
fn the_fields_follow_a_stepped_value_before_it_is_committed() {
    let (mut s, page, at) = scene();
    s.activate_page(&page);
    assert_eq!(s.sketch_expose("d:length", "Distance 1"), "");
    select_clone(&mut s, at);
    let shown = |s: &Session| -> serde_json::Value {
        serde_json::from_str(&s.sketch_instance_json()).unwrap()
    };
    s.preview_sketch_instance_value("d:length", 77.0);
    assert_eq!(
        shown(&s)["rows"][0]["value"].as_f64(),
        Some(77.0),
        "the field shows the stepped value while the pointer is held"
    );
    s.cancel_sketch_instance_preview();
    assert_eq!(shown(&s)["rows"][0]["origin"], "sketch", "Escape drops it");
}

#[test]
fn expose_on_the_sketch_page_then_a_clone_sets_its_own_value() {
    let (mut s, page, at) = scene();
    s.activate_page(&page);
    let v = view(&s);
    let dimension = v["constraints"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["id"] == "length")
        .unwrap();
    assert_eq!(dimension["expose"]["on"], false);
    assert_eq!(dimension["expose"]["suggest"], "Distance 1");
    assert!(
        v["constraints"][0]["expose"].is_null()
            || v["constraints"]
                .as_array()
                .unwrap()
                .iter()
                .any(|r| r["id"] == "horizontal" && r["expose"].is_null()),
        "a relation without a value has no Expose"
    );
    assert_eq!(s.sketch_expose("d:length", "Distance 1"), "");
    assert_eq!(
        s.sketch_expose("d:length", "Distance 2"),
        "This value cannot be exposed",
        "exposed already"
    );
    let v = view(&s);
    assert!(
        v["constraints"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row["id"] == "length" && row["expose"]["on"] == true)
    );
    select_clone(&mut s, at);
    let panel: serde_json::Value = serde_json::from_str(&s.sketch_instance_json()).unwrap();
    assert_eq!(panel["state"], "original");
    assert_eq!(panel["rows"][0]["label"], "Distance 1");
    assert_eq!(panel["rows"][0]["origin"], "sketch");
    let answer = s.set_sketch_instance_value("d:length", 90.0);
    assert!(answer.contains("\"written\":true"), "{answer}");
    let panel: serde_json::Value = serde_json::from_str(&s.sketch_instance_json()).unwrap();
    assert_eq!(
        (panel["state"].as_str(), panel["rows"][0]["value"].as_f64()),
        (Some("own"), Some(90.0))
    );
    assert!(s.reset_sketch_instance("d:length"));
    let panel: serde_json::Value = serde_json::from_str(&s.sketch_instance_json()).unwrap();
    assert_eq!(panel["state"], "original");
    assert!(s.own_sketch_instance(), "Own writes the shown values");
    assert!(s.reset_sketch_instance(""), "Original's drops them");
    assert!(!s.reset_sketch_instance(""), "nothing left to drop");
    s.undo();
    let panel: serde_json::Value = serde_json::from_str(&s.sketch_instance_json()).unwrap();
    assert_eq!(
        panel["state"], "own",
        "Undo of Original's brings the own values back"
    );
}

#[test]
fn a_clone_of_a_sketch_without_exposed_values_has_no_parameters_row() {
    let (mut s, _, at) = scene();
    select_clone(&mut s, at);
    assert_eq!(s.sketch_instance_json(), "null");
}
