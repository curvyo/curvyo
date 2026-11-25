//! The session side of sketch instances, slice 2 (`specs/0099-sketch-pages` criteria 25, 32 to
//! 35): Link and Unlink on the sketch page, the knob of a selected clone, a coupled drag with its
//! readout, Shift at the press, Ctrl snapping, Escape and the one step on release.
#![allow(clippy::unwrap_used)]

use curvyo_document_core::{Document, ObjectSnapshot, PageKind, Point, Style, Vec2, pack};
use curvyo_editor_wasm::{Session, Tool};
use curvyo_sketch_core as sk;

/// Two lines from the origin: `la` 60 mm along x and `lb` 40 mm along y, each with a driving
/// length and its axis angle.
fn graph() -> sk::Sketch {
    let mut s = sk::Sketch::new("origin".into());
    for (point, x, y) in [("a", 60.0, 0.0), ("b", 0.0, 40.0)] {
        s.entities.insert(
            point.into(),
            sk::Entity::Point {
                position: sk::Position::mm(x, y),
                fixed: false,
            },
        );
    }
    for (line, end, mm, deg) in [("la", "a", 60.0, 0.0), ("lb", "b", 40.0, 90.0)] {
        s.entities.insert(
            line.into(),
            sk::Entity::Line {
                start: "origin".into(),
                end: end.into(),
            },
        );
        s.constraints.insert(
            format!("len_{line}").as_str().into(),
            sk::Constraint::driving(sk::Relation::Length {
                curve: line.into(),
                value: sk::Expression::Literal(sk::Quantity::length(mm).unwrap()),
            }),
        );
        s.constraints.insert(
            format!("ang_{line}").as_str().into(),
            sk::Constraint::driving(sk::Relation::AxisAngle {
                line: line.into(),
                value: sk::Expression::Literal(sk::Quantity::angle(f64::to_radians(deg)).unwrap()),
            }),
        );
    }
    s
}

/// The sketch page with both lengths exposed and linked, a clone on the first page, selected.
fn scene() -> (Session, String, Point) {
    let d = Document::new(9950);
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
    let mut s = Session::open(9951, &pack(&d, "t").unwrap()).unwrap();
    s.resize_viewport(1920.0, 1080.0);
    s.activate_page(&page.id);
    assert_eq!(s.sketch_expose("d:len_la", "Leg 1"), "");
    assert_eq!(s.sketch_expose("d:len_lb", "Leg 2"), "");
    assert_eq!(s.sketch_expose("d:ang_la", "Angle 1"), "");
    // Criterion 32: the Link list offers the other length, never the angle.
    let view: serde_json::Value = serde_json::from_str(&s.sketch_view_json()).unwrap();
    let text = view.to_string();
    assert!(
        text.contains("\"options\":[{\"group\":null,\"label\":\"Leg 2\",\"with\":\"d:len_lb\"}]"),
        "{text}"
    );
    assert_eq!(s.sketch_link("d:len_la", "d:len_lb"), "");
    assert_eq!(
        s.sketch_link("d:len_la", "d:ang_la"),
        "These values cannot be linked"
    );
    let at = Point::new(anchor.x, anchor.y);
    select(&mut s, at);
    (s, page.id, at)
}

fn select(s: &mut Session, at: Point) {
    s.activate_page(curvyo_document_core::FIRST_PAGE_ID);
    s.set_tool(Tool::Select);
    s.pointer_down(at, false);
    s.pointer_up(at, false, false);
}

fn rows(s: &Session) -> serde_json::Value {
    serde_json::from_str::<serde_json::Value>(&s.sketch_instance_json()).unwrap()["rows"].clone()
}

fn knob(s: &Session, key: &str) -> Point {
    s.sketch_knob_points()
        .into_iter()
        .find(|(k, _)| k == key)
        .unwrap_or_else(|| panic!("no knob {key}: {:?}", s.sketch_knob_points()))
        .1
}

/// Criteria 25 and 33: a drag of a linked length's knob moves its partner live, reads both in
/// one chip and writes one step "Change parameter" on release; nothing is written before.
#[test]
fn a_coupled_knob_drag_moves_the_partner_and_writes_one_step() {
    let (mut s, _, _) = scene();
    let steps = s.history_len();
    let at = knob(&s, "d:len_la");
    s.pointer_down(at, false);
    let to = Point::new(at.x + 10.0, at.y);
    s.pointer_hover(to, false, false);
    assert_eq!(s.history_len(), steps, "nothing written during the drag");
    assert_eq!(s.sketch_knob_chip_glyph(), "link");
    assert_eq!(s.live_readout().unwrap().text, "Leg 1 = Leg 2 70.0 mm");
    assert_eq!(rows(&s)[0]["origin"], "sketch", "the row keeps its state");
    assert!(s.sketch_knob_dragging());
    assert_eq!(
        rows(&s)[1]["value"].as_f64(),
        Some(70.0),
        "the partner's field follows live"
    );
    s.pointer_up(to, false, false);
    assert_eq!(s.history_len(), steps + 1);
    let rows = rows(&s);
    assert_eq!(
        (rows[0]["value"].as_f64(), rows[1]["value"].as_f64()),
        (Some(70.0), Some(70.0))
    );
    assert_eq!(rows[0]["origin"], "own");
}

/// Criterion 34: Shift at the press changes only that value and unlinks it; Shift no longer snaps.
#[test]
fn shift_at_the_press_changes_only_this_value() {
    let (mut s, _, _) = scene();
    let at = knob(&s, "d:len_la");
    s.pointer_down(at, true);
    let to = Point::new(at.x + 10.4, at.y);
    s.pointer_hover(to, true, false);
    assert_eq!(s.sketch_knob_chip_glyph(), "unlink");
    assert_eq!(
        s.live_readout().unwrap().text,
        "Leg 1 70.4 mm",
        "Shift does not snap"
    );
    s.pointer_up(to, true, false);
    let rows = rows(&s);
    assert_eq!(rows[0]["link"], "unlinked");
    assert_eq!(rows[1]["origin"], "sketch", "the partner stays");
}

/// Criterion 25: Ctrl (Cmd) snaps whole millimetres; Escape restores and writes nothing.
#[test]
fn ctrl_snaps_and_escape_writes_nothing() {
    let (mut s, _, _) = scene();
    let steps = s.history_len();
    let at = knob(&s, "d:len_la");
    s.pointer_down(at, false);
    s.pointer_hover(Point::new(at.x + 10.4, at.y), false, true);
    assert_eq!(s.live_readout().unwrap().text, "Leg 1 = Leg 2 70.0 mm");
    s.escape();
    assert_eq!(s.sketch_knob_chip_glyph(), "");
    s.pointer_up(Point::new(at.x + 10.4, at.y), false, false);
    assert_eq!(s.history_len(), steps);
    assert_eq!(rows(&s)[0]["origin"], "sketch");
}

/// Criterion 35: Unlink on the sketch page dissolves a group of two.
#[test]
fn unlink_dissolves_a_group_of_two() {
    let (mut s, page, at) = scene();
    assert_eq!(rows(&s)[0]["group"], "A");
    s.activate_page(&page);
    assert_eq!(s.sketch_unlink("d:len_lb"), "");
    select(&mut s, at);
    assert_eq!(rows(&s)[0]["link"], "none");
}

/// Criterion 25: a double-click on a knob opens its entry chip (not Select original); Enter
/// commits one step coupled through the link, Escape writes nothing.
#[test]
fn a_double_click_on_a_knob_opens_its_entry_chip() {
    let (mut s, _, _) = scene();
    let steps = s.history_len();
    let at = knob(&s, "d:len_la");
    s.double_click(at, false, false);
    let entry: serde_json::Value = serde_json::from_str(&s.sketch_knob_entry_json()).unwrap();
    assert_eq!(
        (entry["label"].as_str(), entry["glyph"].as_str()),
        (Some("Leg 1"), Some("link"))
    );
    assert!(
        s.sketch_instance_json().contains("Leg 1"),
        "the clone stays selected"
    );
    s.cancel_sketch_knob_entry();
    assert_eq!(s.history_len(), steps, "Escape writes nothing");
    s.double_click(at, false, false);
    assert!(
        s.commit_sketch_knob_entry("75")
            .contains("\"written\":true")
    );
    assert_eq!(s.history_len(), steps + 1, "one step");
    assert_eq!(s.sketch_knob_entry_json(), "");
    let rows = rows(&s);
    assert_eq!(
        (rows[0]["value"].as_f64(), rows[1]["value"].as_f64()),
        (Some(75.0), Some(75.0))
    );
}

/// Criterion 25, UX notes 12.3 and 12.9.4: hovering a linked knob shows its own and its
/// partner's dimension guides (only its own with Shift); a coupled drag keeps both.
#[test]
fn hover_and_drag_show_the_dimension_guides_of_the_knob_and_its_partners() {
    let (mut s, _, _) = scene();
    let at = knob(&s, "d:len_la");
    s.pointer_hover(at, false, false);
    assert_eq!(s.sketch_knob_guide_keys(), ["d:len_la", "d:len_lb"]);
    s.pointer_hover(at, true, false);
    assert_eq!(s.sketch_knob_guide_keys(), ["d:len_la"], "Shift: only this");
    s.pointer_down(at, false);
    s.pointer_hover(Point::new(at.x + 5.0, at.y), false, false);
    assert_eq!(s.sketch_knob_guide_keys(), ["d:len_la", "d:len_lb"]);
    s.pointer_up(Point::new(at.x + 5.0, at.y), false, false);
    s.pointer_hover(Point::new(at.x + 400.0, at.y + 400.0), false, false);
    assert_eq!(s.sketch_knob_guide_keys(), Vec::<String>::new());
}
