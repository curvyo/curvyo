//! Alt cycles every mode and variant; the mode label and entry box come from Rust (AC33-AC35).
use super::*;
use crate::session::{KeyInput, KeyOutcome};
use curvyo_document_core::Point;
use serde_json::Value;

fn hover(s: &mut Session, x: f64, y: f64) {
    let p = s.sketch_placement().unwrap().apply_point(Point::new(x, y));
    s.pointer_hover(p, false, false);
}

fn click(s: &mut Session, x: f64, y: f64) {
    let p = s.sketch_placement().unwrap().apply_point(Point::new(x, y));
    s.pointer_hover(p, false, false);
    s.pointer_down(p, false);
    s.pointer_up(p, false, false);
}

fn key(s: &mut Session, key: &str, shift: bool, ctrl: bool, blocked: bool) -> KeyOutcome {
    s.key_down(KeyInput {
        key,
        shift,
        ctrl,
        alt: key == "Alt",
        repeat: false,
        dom_blocked: blocked,
        mac: false,
        time_ms: 0.0,
    })
}

fn alt(s: &mut Session) -> KeyOutcome {
    key(s, "Alt", false, false, false)
}

fn sketch(tool: SketchTool) -> Session {
    let mut s = Session::new(66_600);
    s.set_workbench(Workbench::Sketch);
    s.set_sketch_tool(tool);
    s
}

fn label(s: &Session) -> Value {
    s.sketch_view()["canvas"]["mode_label"].clone()
}

fn entry(s: &Session) -> Value {
    s.sketch_view()["canvas"]["entry"].clone()
}

fn screen(s: &Session, x: f64, y: f64) -> (f64, f64) {
    let placement = s.sketch_placement().unwrap();
    s.view()
        .document_to_screen(placement.apply_point(Point::new(x, y)))
}

#[test]
fn the_m_key_and_alt_chords_never_change_the_mode() {
    let mut s = sketch(SketchTool::Polyline);
    click(&mut s, 10.0, 10.0);
    for (k, shift, ctrl, blocked) in [
        ("m", false, false, false),
        ("M", true, false, false),
        ("Alt", true, false, false),
        ("Alt", false, true, false),
        ("Alt", false, false, true),
    ] {
        assert_eq!(
            key(&mut s, k, shift, ctrl, blocked),
            KeyOutcome::Ignored,
            "{k}"
        );
        assert_eq!(
            s.sketch.mode,
            curvyo_sketch_core::ContinuationMode::FreeLine
        );
    }
    assert_eq!(alt(&mut s), KeyOutcome::Consumed);
}

#[test]
fn the_label_names_each_polyline_mode_at_the_segment_start() {
    let mut s = sketch(SketchTool::Polyline);
    hover(&mut s, 10.0, 10.0);
    let (px, py) = screen(&s, 10.0, 10.0);
    assert_eq!(
        label(&s)["text"],
        "Line",
        "shown at the pointer before the first pick"
    );
    assert!((label(&s)["x"].as_f64().unwrap() - px).abs() < 1e-6);
    click(&mut s, 10.0, 10.0);
    hover(&mut s, 30.0, 25.0);
    let mut names = vec![label(&s)["text"].as_str().unwrap().to_owned()];
    for _ in 0..5 {
        alt(&mut s);
        names.push(label(&s)["text"].as_str().unwrap().to_owned());
    }
    assert_eq!(
        names,
        [
            "Line",
            "Perpendicular line",
            "Tangent line",
            "Tangent arc",
            "Arc left",
            "Arc right"
        ]
    );
    let at = label(&s);
    assert!((at["x"].as_f64().unwrap() - px).abs() < 1e-6);
    assert!((at["y"].as_f64().unwrap() - py).abs() < 1e-6);
    let undo = s.undo_len();
    alt(&mut s);
    assert_eq!(s.undo_len(), undo, "a mode change adds no history step");
}

#[test]
fn the_label_takes_the_opposite_quadrant_when_the_segment_runs_into_it() {
    let mut s = sketch(SketchTool::Polyline);
    click(&mut s, 30.0, 30.0);
    let (sx, sy) = screen(&s, 30.0, 30.0);
    for (x, y) in [(50.0, 50.0), (50.0, 10.0), (10.0, 50.0), (10.0, 10.0)] {
        hover(&mut s, x, y);
        let (hx, hy) = screen(&s, x, y);
        let at = label(&s);
        let away = (
            at["away_x"].as_f64().unwrap(),
            at["away_y"].as_f64().unwrap(),
        );
        let into = hx > sx && hy > sy;
        assert_eq!(
            away,
            if into { (-1.0, -1.0) } else { (1.0, 1.0) },
            "{x} {y}"
        );
    }
}

#[test]
fn alt_cycles_arc_circle_and_ellipse_variants_before_the_first_pick() {
    for (tool, names) in [
        (SketchTool::Arc, ["Arc, 3 points", "Arc, centre"]),
        (SketchTool::Circle, ["Circle, centre", "Circle, 3 points"]),
        (
            SketchTool::Ellipse,
            ["Ellipse, centre", "Ellipse, 3 points"],
        ),
    ] {
        let mut s = sketch(tool);
        hover(&mut s, 10.0, 10.0);
        assert_eq!(label(&s)["text"], names[0]);
        assert_eq!(alt(&mut s), KeyOutcome::Consumed);
        assert_eq!(label(&s)["text"], names[1]);
        assert_eq!(alt(&mut s), KeyOutcome::Consumed);
        assert_eq!(label(&s)["text"], names[0], "wraps");
        assert_eq!(s.sketch_tool(), tool);
    }
}

#[test]
fn alt_after_a_variant_pick_hints_instead_of_discarding() {
    let mut s = sketch(SketchTool::Circle);
    click(&mut s, 10.0, 10.0);
    assert_eq!(alt(&mut s), KeyOutcome::Consumed);
    assert_eq!(s.sketch_tool(), SketchTool::Circle);
    assert_eq!(s.sketch.picks.len(), 1, "the pick is kept");
    assert_eq!(
        label(&s)["hint"],
        "Finish the shape or press Escape to change the method"
    );
}

#[test]
fn single_mode_tools_have_no_label_and_ignore_alt() {
    for tool in [SketchTool::Point, SketchTool::Rectangle, SketchTool::Select] {
        let mut s = sketch(tool);
        hover(&mut s, 10.0, 10.0);
        assert!(label(&s).is_null(), "{tool:?}");
        assert_eq!(alt(&mut s), KeyOutcome::Ignored, "{tool:?}");
    }
}

#[test]
fn the_entry_box_names_its_fields_and_sits_at_the_object() {
    let mut s = sketch(SketchTool::Polyline);
    hover(&mut s, 10.0, 10.0);
    assert!(entry(&s).is_null(), "no pending pick, no entry");
    click(&mut s, 10.0, 10.0);
    hover(&mut s, 30.0, 10.0);
    let box_ = entry(&s);
    assert_eq!(box_["group"], "Line");
    let names: Vec<_> = box_["fields"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["label"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["Length", "Angle"]);
    assert_eq!(box_["fields"][0]["value"], "20");
    assert_eq!(box_["fields"][1]["suffix"], "°");
    let (mx, my) = screen(&s, 20.0, 10.0);
    assert!(
        (box_["x"].as_f64().unwrap() - mx).abs() < 1e-6,
        "at the midpoint"
    );
    assert!((box_["y"].as_f64().unwrap() - my).abs() < 1e-6);
    s.set_sketch_mode("tangent-arc");
    hover(&mut s, 30.0, 20.0);
    let box_ = entry(&s);
    assert_eq!(box_["group"], "Arc");
    assert_eq!(box_["fields"].as_array().unwrap().len(), 1);
    assert_eq!(box_["fields"][0]["label"], "Radius");
}

#[test]
fn circle_and_dimension_entries_use_their_own_anchor_and_kind() {
    let mut s = sketch(SketchTool::Circle);
    click(&mut s, 10.0, 10.0);
    hover(&mut s, 10.0, 15.0);
    let box_ = entry(&s);
    assert_eq!(box_["group"], "Circle");
    assert_eq!(box_["kind"], "construction");
    let (rx, ry) = screen(&s, 15.0, 10.0);
    assert!(
        (box_["x"].as_f64().unwrap() - rx).abs() < 1e-6,
        "right of the perimeter"
    );
    assert!((box_["y"].as_f64().unwrap() - ry).abs() < 1e-6);
    let mut s = sketch(SketchTool::Point);
    hover(&mut s, 3.0, 4.0);
    let names: Vec<_> = entry(&s)["fields"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["name"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(names, ["Sketch X", "Sketch Y"]);
}

#[test]
fn a_placed_dimension_keeps_its_entry_when_the_pointer_leaves_the_canvas() {
    let mut s = sketch(SketchTool::Circle);
    click(&mut s, 10.0, 10.0);
    click(&mut s, 20.0, 10.0);
    let circle = s.sketch_view()["entities"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["kind"] == "circle")
        .map(|e| e["id"].as_str().unwrap().to_owned())
        .unwrap();
    s.set_sketch_tool(SketchTool::Select);
    // Armed without a selection (a preselected circle is dimensioned at once, AC40), the circle
    // is picked on the canvas and the label placed.
    assert!(s.sketch_rail_press("dimension").unwrap_or(false));
    assert!(s.sketch_select_entity(&circle.as_str().into(), false));
    click(&mut s, 40.0, 40.0);
    assert_eq!(entry(&s)["kind"], "dimension", "{}", entry(&s));
    // The pointer moves onto the entry box itself, which lies outside the canvas element.
    s.pointer_leave();
    let box_ = entry(&s);
    assert_eq!(box_["kind"], "dimension", "{box_}");
    // With no pointer on the canvas there is no cursor to keep clear of: the anchor stands in.
    assert_eq!(box_["pointer_x"], box_["x"]);
    assert_eq!(box_["pointer_y"], box_["y"]);
}
