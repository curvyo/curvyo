//! Independent tester acceptance tests for `specs/0052-click-to-create-shapes/specification.md`.
//! Written from the specification before the implementation diff was read; everything goes
//! through `Session`'s public API.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]
#![allow(clippy::too_many_lines, clippy::similar_names, clippy::doc_markdown)]
#![allow(missing_docs, clippy::many_single_char_names)]

use curvyo_document_core::{DisplayUnit, Document, Point, PrimitiveSnapshot, Shape, pack, unpack};
use curvyo_editor_wasm::{KeyInput, KeyOutcome, Session, Tool};
use curvyo_ui_core::PlacementField;

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

fn new_session(tool: Tool) -> Session {
    let mut s = Session::new(1);
    s.set_tool(tool);
    s
}

fn doc_of(s: &Session) -> Document {
    unpack(99, &s.pack("0.1.0").unwrap()).expect("reopens")
}

fn n_objects(s: &Session) -> usize {
    doc_of(s).object_ids().len()
}

fn prim(s: &Session, i: usize) -> PrimitiveSnapshot {
    let d = doc_of(s);
    d.primitive(d.object_ids()[i]).unwrap()
}

/// (x, y, w, h, rotation degrees) of the rectangle at index `i`.
fn rect(s: &Session, i: usize) -> (f64, f64, f64, f64, f64) {
    let p = prim(s, i);
    match p.shape {
        Shape::Rect { bounds, .. } => (
            bounds.origin.x,
            bounds.origin.y,
            bounds.width.as_mm(),
            bounds.height.as_mm(),
            p.rotation.as_radians().to_degrees(),
        ),
        other => panic!("rectangle expected, got {other:?}"),
    }
}

/// (cx, cy, rx, ry, rotation degrees).
fn ellipse(s: &Session, i: usize) -> (f64, f64, f64, f64, f64) {
    let p = prim(s, i);
    match p.shape {
        Shape::Ellipse { frame } => (
            frame.center.x,
            frame.center.y,
            frame.rx.as_mm(),
            frame.ry.as_mm(),
            p.rotation.as_radians().to_degrees(),
        ),
        other => panic!("ellipse expected, got {other:?}"),
    }
}

/// (cx, cy, radius, angle degrees) of a polygon or star.
fn star(s: &Session, i: usize) -> (f64, f64, f64, f64) {
    let p = prim(s, i);
    match p.shape {
        Shape::Polygon { frame, .. } | Shape::Star { frame, .. } => (
            frame.center.x,
            frame.center.y,
            frame.radius.as_mm(),
            frame.angle.as_radians().to_degrees(),
        ),
        other => panic!("polygon or star expected, got {other:?}"),
    }
}

fn hover(s: &mut Session, p: Point) {
    s.pointer_hover(p, false, false);
}

fn start(s: &mut Session, a: Point) {
    hover(s, a);
    s.pointer_down(a, false);
    s.pointer_up(a, false, false);
}

fn drag(s: &mut Session, from: Point, to: Point) {
    hover(s, from);
    s.pointer_down(from, false);
    hover(s, to);
    s.pointer_up(to, false, false);
}

fn key(s: &mut Session, k: &str, ctrl: bool, shift: bool) -> KeyOutcome {
    s.key_down(KeyInput {
        key: k,
        ctrl,
        shift,
        ..KeyInput::default()
    })
}

/// A second press at `b` followed by its release.
fn second_press(s: &mut Session, b: Point, shift: bool, ctrl: bool) {
    s.pointer_hover(b, shift, ctrl);
    s.pointer_down(b, shift);
    s.pointer_up(b, shift, ctrl);
}

const R: Tool = Tool::Rectangle;

// ---- Part A -----------------------------------------------------------------------------

#[test]
fn ac1_a_click_starts_a_placement_for_each_creation_tool_and_creates_nothing() {
    for tool in [Tool::Rectangle, Tool::Ellipse, Tool::PolygonStar] {
        let mut s = new_session(tool);
        let k = s.view().scale();
        let a = pt(100.0, 50.0);
        hover(&mut s, a);
        s.pointer_down(a, false);
        s.pointer_up(pt(a.x + 0.5, a.y), false, false);
        assert!(0.5 * k < 3.0, "test premise: under 3 px");
        assert!(s.placement().is_some(), "{tool:?}");
        assert_eq!(n_objects(&s), 0);
        assert_eq!(s.tool(), tool);
    }
}

#[test]
fn ac1_three_pixels_or_more_is_a_drag_as_today() {
    let mut s = new_session(R);
    drag(&mut s, pt(100.0, 50.0), pt(101.0, 50.0));
    assert!(s.placement().is_none());
    assert_eq!(n_objects(&s), 1);
    let (x, y, w, h, _) = rect(&s, 0);
    assert!(near(x, 100.0) && near(y, 50.0) && near(w, 1.0) && near(h, 0.0));
}

#[test]
fn ac1_touch_uses_eight_pixels() {
    let mut s = new_session(R);
    let k = s.view().scale();
    s.set_touch_input(true);
    let mm = 6.0 / k; // 6 px: a drag for a mouse, a tap for touch
    hover(&mut s, pt(100.0, 50.0));
    s.pointer_down(pt(100.0, 50.0), false);
    s.pointer_up(pt(100.0 + mm, 50.0), false, false);
    assert!(s.placement().is_some());
    assert_eq!(n_objects(&s), 0);
    s.cancel_placement();
    s.set_touch_input(false);
    hover(&mut s, pt(100.0, 50.0));
    s.pointer_down(pt(100.0, 50.0), false);
    s.pointer_up(pt(100.0 + mm, 50.0), false, false);
    assert!(s.placement().is_none());
    assert_eq!(n_objects(&s), 1);
}

#[test]
fn ac2_preview_follows_the_pointer_without_a_button_and_not_within_3px() {
    let mut s = new_session(R);
    let a = pt(100.0, 50.0);
    start(&mut s, a);
    let near_a = s.draw_list().triangles.len();
    hover(&mut s, pt(130.0, 40.0));
    let far = s.draw_list().triangles.len();
    assert!(far > near_a, "the outline appears away from A");
    hover(&mut s, pt(100.2, 50.1));
    assert_eq!(
        s.draw_list().triangles.len(),
        near_a,
        "no outline within 3 px"
    );
}

#[test]
fn ac12_chip_fields_show_the_live_shape() {
    let mut s = new_session(R);
    start(&mut s, pt(100.0, 50.0));
    hover(&mut s, pt(130.0, 40.0));
    assert_eq!(
        s.placement_field_text(PlacementField::Width).unwrap(),
        "30.0"
    );
    assert_eq!(
        s.placement_field_text(PlacementField::Height).unwrap(),
        "10.0"
    );
    assert_eq!(s.placement_field_text(PlacementField::Angle).unwrap(), "0");
}

#[test]
fn ac12_ellipse_circle_shows_r_and_ellipse_leaves_it_empty() {
    let mut s = new_session(Tool::Ellipse);
    start(&mut s, pt(50.0, 50.0));
    s.modifiers_changed(false, true, false);
    hover(&mut s, pt(70.0, 70.0));
    assert_eq!(
        s.placement_field_text(PlacementField::Size).unwrap(),
        "10.0"
    );
    s.modifiers_changed(false, false, false);
    hover(&mut s, pt(80.0, 60.0));
    assert_eq!(s.placement_field_text(PlacementField::Size).unwrap(), "");
}

#[test]
fn ac4_second_press_commits_what_a_drag_commits() {
    let mut a = new_session(R);
    start(&mut a, pt(100.0, 50.0));
    hover(&mut a, pt(130.0, 40.0));
    let before = a.undo_len();
    a.pointer_down(pt(130.0, 40.0), false);
    assert!(a.placement().is_none(), "placed at the press");
    // The rest of the press does nothing.
    a.pointer_hover(pt(200.0, 200.0), false, false);
    a.pointer_up(pt(200.0, 200.0), false, false);
    assert_eq!(n_objects(&a), 1);
    assert_eq!(a.undo_len(), before + 1);
    assert_eq!(a.tool(), Tool::Select);
    assert_eq!(a.selected_object_count(), 1);

    let mut b = new_session(R);
    drag(&mut b, pt(100.0, 50.0), pt(130.0, 40.0));
    assert_eq!(rect(&a, 0), rect(&b, 0));
    let (x, y, w, h, r) = rect(&a, 0);
    assert!(near(x, 100.0) && near(y, 40.0) && near(w, 30.0) && near(h, 10.0) && near(r, 0.0));
}

#[test]
fn ac4_second_press_on_an_existing_object_does_not_select_it() {
    let mut s = new_session(R);
    drag(&mut s, pt(0.0, 0.0), pt(200.0, 200.0));
    s.set_tool(R);
    start(&mut s, pt(50.0, 50.0));
    second_press(&mut s, pt(100.0, 100.0), false, false);
    assert_eq!(n_objects(&s), 2);
    assert_eq!(s.selected_object_count(), 1);
}

#[test]
fn ac5_a_press_near_a_is_ignored_double_click_makes_no_sliver() {
    for tool in [Tool::Rectangle, Tool::Ellipse, Tool::PolygonStar] {
        let mut s = new_session(tool);
        let a = pt(100.0, 50.0);
        start(&mut s, a);
        s.pointer_down(pt(100.3, 50.0), false);
        s.pointer_up(pt(100.3, 50.0), false, false);
        assert!(s.placement().is_some(), "{tool:?} continues");
        assert_eq!(n_objects(&s), 0);
    }
}

#[test]
fn ac3_ctrl_makes_a_square_shift_centres_and_both_live_with_the_pointer_still() {
    let mut s = new_session(R);
    start(&mut s, pt(100.0, 50.0));
    hover(&mut s, pt(130.0, 40.0));
    s.modifiers_changed(false, true, false); // pointer still
    assert_eq!(
        s.placement_field_text(PlacementField::Width).unwrap(),
        "30.0"
    );
    assert_eq!(
        s.placement_field_text(PlacementField::Height).unwrap(),
        "30.0"
    );
    s.modifiers_changed(true, false, false);
    assert_eq!(
        s.placement_field_text(PlacementField::Width).unwrap(),
        "60.0"
    );
    assert_eq!(
        s.placement_field_text(PlacementField::Height).unwrap(),
        "20.0"
    );
    s.pointer_down(pt(130.0, 40.0), true);
    let (x, y, w, h, _) = rect(&s, 0);
    assert!(
        near(x, 70.0) && near(y, 40.0) && near(w, 60.0) && near(h, 20.0),
        "{x} {y} {w} {h}"
    );
}

#[test]
fn ac3_modifier_held_at_the_first_click_latches_on() {
    // Customer feedback 2026-10-16, round 3: a key held at the click stays on after its release.
    let mut s = new_session(R);
    s.pointer_hover(pt(100.0, 50.0), false, true);
    s.pointer_down(pt(100.0, 50.0), false);
    s.pointer_up(pt(100.0, 50.0), false, true);
    hover(&mut s, pt(110.0, 50.0));
    hover(&mut s, pt(130.0, 40.0));
    assert_eq!(
        s.placement_field_text(PlacementField::Height).unwrap(),
        "30.0"
    );
}

// ---- Escape, cancel, gate ---------------------------------------------------------------

#[test]
fn ac7_escape_cancels_then_leaves_the_tool_and_writes_nothing() {
    let mut s = new_session(R);
    let before = s.pack("0.1.0").unwrap();
    let steps = s.undo_len();
    start(&mut s, pt(100.0, 50.0));
    hover(&mut s, pt(130.0, 40.0));
    assert!(matches!(
        key(&mut s, "Escape", false, false),
        KeyOutcome::Escape(_)
    ));
    assert!(s.placement().is_none());
    assert_eq!(s.tool(), R, "the tool stays active");
    assert_eq!(s.pack("0.1.0").unwrap(), before);
    assert_eq!(s.undo_len(), steps);
    key(&mut s, "Escape", false, false);
    assert_eq!(s.tool(), Tool::Select);
}

#[test]
fn ac7_a_tool_switch_cancels_and_the_other_tool_starts_fresh() {
    let mut s = new_session(R);
    start(&mut s, pt(100.0, 50.0));
    s.set_tool(Tool::Ellipse);
    assert!(s.placement().is_none());
    assert_eq!(n_objects(&s), 0);
}

#[test]
fn ac7_a_lost_pointer_does_not_cancel() {
    let mut s = new_session(R);
    start(&mut s, pt(100.0, 50.0));
    s.pointer_cancelled();
    assert!(s.placement().is_some());
}

#[test]
fn ac7_window_blur_releases_modifiers_and_keeps_the_placement() {
    let mut s = new_session(R);
    start(&mut s, pt(100.0, 50.0));
    hover(&mut s, pt(130.0, 40.0));
    s.modifiers_changed(false, true, false);
    s.modifiers_changed(false, false, false); // what the host does on blur
    s.pointer_cancelled();
    assert!(s.placement().is_some());
    assert_eq!(
        s.placement_field_text(PlacementField::Height).unwrap(),
        "10.0"
    );
}

#[test]
fn ac8_the_key_gate_blocks_every_document_key_during_a_placement() {
    let mut s = new_session(R);
    drag(&mut s, pt(0.0, 0.0), pt(10.0, 10.0));
    s.set_tool(R);
    let doc = s.pack("0.1.0").unwrap();
    let steps = s.undo_len();
    let sel = s.selected_object_count();
    start(&mut s, pt(100.0, 50.0));
    let chords: [(&str, bool, bool); 18] = [
        ("e", false, false),
        ("v", false, false),
        ("p", false, false),
        ("m", false, false),
        ("r", false, false),
        ("s", false, false),
        ("k", false, false),
        ("Delete", false, false),
        ("Backspace", false, false),
        ("a", true, false),
        ("g", true, false),
        ("g", true, true),
        ("z", true, false),
        ("z", true, true),
        ("n", true, false),
        ("o", true, false),
        ("s", true, false),
        ("s", true, true),
    ];
    for (k, ctrl, shift) in chords {
        key(&mut s, k, ctrl, shift);
        assert_eq!(s.tool(), R, "{k} ctrl={ctrl} shift={shift}");
        assert!(s.placement().is_some(), "{k}");
        assert_eq!(s.undo_len(), steps, "{k}");
        assert_eq!(s.selected_object_count(), sel, "{k}");
    }
    for (k, ctrl, shift) in [("w", true, false), ("q", true, false)] {
        key(&mut s, k, ctrl, shift);
        assert!(s.placement().is_some());
    }
    assert_eq!(s.pack("0.1.0").unwrap(), doc);
}

#[test]
fn ac9_zoom_and_pan_keep_a_fixed_in_document_space() {
    let mut s = new_session(R);
    start(&mut s, pt(100.0, 50.0));
    s.pan_to(0.0, 0.0);
    hover(&mut s, pt(130.0, 40.0));
    s.pointer_down(pt(130.0, 40.0), false);
    let (x, y, w, h, _) = rect(&s, 0);
    assert!(near(x, 100.0) && near(y, 40.0) && near(w, 30.0) && near(h, 10.0));
}

#[test]
fn ac9_leaving_the_viewport_keeps_the_last_shape() {
    let mut s = new_session(R);
    start(&mut s, pt(100.0, 50.0));
    hover(&mut s, pt(130.0, 40.0));
    s.pointer_leave();
    assert_eq!(
        s.placement_field_text(PlacementField::Width).unwrap(),
        "30.0"
    );
    assert!(s.placement().is_some());
}

// ---- Part B -----------------------------------------------------------------------------

fn frozen_rect(s: &mut Session) {
    start(s, pt(100.0, 50.0));
    hover(s, pt(130.0, 40.0));
}

#[test]
fn ac14_first_edit_freezes_pointer_and_modifiers_change_nothing() {
    let mut s = new_session(R);
    frozen_rect(&mut s);
    s.placement_edit(0, "40");
    hover(&mut s, pt(0.0, 0.0));
    s.modifiers_changed(true, true, false);
    assert_eq!(s.placement_field_text(PlacementField::Width).unwrap(), "40");
    assert_eq!(
        s.placement_field_text(PlacementField::Height).unwrap(),
        "10.0"
    );
    assert!(s.place_placement());
    let (x, y, w, h, r) = rect(&s, 0);
    assert!(near(x, 100.0) && near(y, 40.0) && near(w, 40.0) && near(h, 10.0) && near(r, 0.0));
}

#[test]
fn ac16a_width_height() {
    let mut s = new_session(R);
    frozen_rect(&mut s);
    s.placement_edit(0, "40");
    s.placement_edit(1, "20");
    assert!(s.place_placement());
    let (x, y, w, h, r) = rect(&s, 0);
    assert!(near(x, 100.0) && near(y, 30.0) && near(w, 40.0) && near(h, 20.0) && near(r, 0.0));
    assert_eq!(s.tool(), Tool::Select);
    assert_eq!(s.selected_object_count(), 1);
}

#[test]
fn ac16b_angle_turns_about_a() {
    let mut s = new_session(R);
    frozen_rect(&mut s);
    s.placement_edit(0, "40");
    s.placement_edit(1, "20");
    s.placement_edit(2, "90");
    assert!(s.place_placement());
    let p = prim(&s, 0);
    let Shape::Rect { bounds, .. } = p.shape else {
        panic!()
    };
    let cx = bounds.origin.x + bounds.width.as_mm() / 2.0;
    let cy = bounds.origin.y + bounds.height.as_mm() / 2.0;
    assert!(near(cx, 110.0) && near(cy, 70.0), "{cx} {cy}");
    assert!(near(bounds.width.as_mm(), 40.0) && near(bounds.height.as_mm(), 20.0));
    assert!(near(p.rotation.as_radians().to_degrees(), 90.0));
}

#[test]
fn ac16c_shift_kept_centres_on_a() {
    let mut s = new_session(R);
    start(&mut s, pt(100.0, 50.0));
    s.modifiers_changed(true, false, false);
    s.pointer_hover(pt(130.0, 40.0), true, false);
    s.placement_edit(0, "40");
    s.placement_edit(1, "20");
    assert!(s.place_placement());
    let (x, y, w, h, _) = rect(&s, 0);
    assert!(near(x, 80.0) && near(y, 40.0) && near(w, 40.0) && near(h, 20.0));
}

#[test]
fn ac17_ctrl_kept_links_width_and_height() {
    let mut s = new_session(R);
    start(&mut s, pt(100.0, 50.0));
    s.modifiers_changed(false, true, false);
    s.pointer_hover(pt(130.0, 40.0), false, true);
    s.placement_edit(0, "25");
    assert!(s.place_placement());
    let (x, y, w, h, _) = rect(&s, 0);
    assert!(near(x, 100.0) && near(y, 25.0) && near(w, 25.0) && near(h, 25.0));
}

#[test]
fn ac17_typing_into_height_also_sets_width() {
    let mut s = new_session(R);
    start(&mut s, pt(100.0, 50.0));
    s.modifiers_changed(false, true, false);
    s.pointer_hover(pt(130.0, 40.0), false, true);
    s.placement_edit(1, "12");
    assert!(s.place_placement());
    let (_, _, w, h, _) = rect(&s, 0);
    assert!(near(w, 12.0) && near(h, 12.0));
}

fn ellipse_at(s: &mut Session) {
    start(s, pt(50.0, 50.0));
    hover(s, pt(70.0, 80.0));
}

#[test]
fn ac18_ellipse_examples() {
    // "10", Enter
    let mut s = new_session(Tool::Ellipse);
    ellipse_at(&mut s);
    s.placement_edit(0, "10");
    assert!(s.place_placement());
    let (cx, cy, rx, ry, _) = ellipse(&s, 0);
    assert!(near(cx, 60.0) && near(cy, 60.0) && near(rx, 10.0) && near(ry, 10.0));
    // Space, "10", Enter: diameter
    let mut s = new_session(Tool::Ellipse);
    ellipse_at(&mut s);
    s.placement_toggle_diameter();
    s.placement_edit(0, "10");
    assert!(s.place_placement());
    let (cx, cy, rx, _, _) = ellipse(&s, 0);
    assert!(near(cx, 55.0) && near(cy, 55.0) && near(rx, 5.0));
    // Shift kept
    let mut s = new_session(Tool::Ellipse);
    start(&mut s, pt(50.0, 50.0));
    s.modifiers_changed(true, false, false);
    s.pointer_hover(pt(70.0, 80.0), true, false);
    s.placement_edit(0, "10");
    assert!(s.place_placement());
    let (cx, cy, rx, _, _) = ellipse(&s, 0);
    assert!(near(cx, 50.0) && near(cy, 50.0) && near(rx, 10.0));
    // turned
    let mut s = new_session(Tool::Ellipse);
    ellipse_at(&mut s);
    s.placement_edit(0, "10");
    s.placement_edit(1, "90");
    assert!(s.place_placement());
    let (cx, cy, rx, _, rot) = ellipse(&s, 0);
    assert!(near(cx, 40.0) && near(cy, 60.0) && near(rx, 10.0) && near(rot, 90.0));
}

#[test]
fn ac18_untouched_r_keeps_rx_and_ry() {
    let mut s = new_session(Tool::Ellipse);
    ellipse_at(&mut s);
    s.placement_edit(1, "0"); // freezes by editing the angle only
    assert!(s.place_placement());
    let (_, _, rx, ry, _) = ellipse(&s, 0);
    assert!(near(rx, 10.0) && near(ry, 15.0));
}

fn hexagon(s: &mut Session) {
    s.set_tool(Tool::PolygonStar);
    s.set_poly_star_point_count(curvyo_document_core::PointCount::new(6).unwrap());
    s.set_poly_star_mode(curvyo_ui_core::PolyStarMode::Polygon);
    start(s, pt(100.0, 100.0));
    hover(s, pt(110.0, 100.0));
}

#[test]
fn ac19_polygon_examples() {
    let mut s = new_session(Tool::PolygonStar);
    hexagon(&mut s);
    assert_eq!(s.placement_field_text(PlacementField::Angle).unwrap(), "0");
    s.placement_edit(0, "25");
    assert!(s.place_placement());
    let (cx, cy, r, a) = star(&s, 0);
    assert!(near(cx, 100.0) && near(cy, 100.0) && near(r, 25.0) && near(a, 0.0));

    let mut s = new_session(Tool::PolygonStar);
    hexagon(&mut s);
    s.placement_edit(0, "25");
    s.placement_edit(1, "-90");
    assert!(s.place_placement());
    let (_, _, r, a) = star(&s, 0);
    assert!(near(r, 25.0) && near(a, -90.0));
}

#[test]
fn ac19_star_mode_keeps_ratio_from_the_bar() {
    let mut s = new_session(Tool::PolygonStar);
    s.set_poly_star_mode(curvyo_ui_core::PolyStarMode::Star);
    s.set_poly_star_ratio(curvyo_document_core::InnerRatio::new(0.5).unwrap());
    start(&mut s, pt(100.0, 100.0));
    hover(&mut s, pt(110.0, 100.0));
    s.placement_edit(0, "25");
    assert!(s.place_placement());
    let p = prim(&s, 0);
    let Shape::Star {
        frame, inner_ratio, ..
    } = p.shape
    else {
        panic!("star expected: {:?}", p.shape)
    };
    assert!(near(frame.radius.as_mm(), 25.0));
    assert!(near(inner_ratio.get(), 0.5));
}

#[test]
fn ac20_space_toggles_diameter_and_edited_text_is_read_in_the_new_mode() {
    let mut s = new_session(Tool::Ellipse);
    ellipse_at(&mut s);
    s.placement_edit(0, "20");
    s.placement_toggle_diameter();
    assert!(s.place_placement());
    let (_, _, rx, _, _) = ellipse(&s, 0);
    assert!(near(rx, 10.0), "{rx}");
}

#[test]
fn ac20_an_untouched_field_is_reshown_without_changing_the_shape() {
    let mut s = new_session(Tool::PolygonStar);
    start(&mut s, pt(100.0, 100.0));
    hover(&mut s, pt(112.0, 100.0));
    assert_eq!(
        s.placement_field_text(PlacementField::Size).unwrap(),
        "12.0"
    );
    s.placement_toggle_diameter();
    assert_eq!(
        s.placement_field_text(PlacementField::Size).unwrap(),
        "24.0"
    );
    assert!(s.place_placement());
    assert!(near(star(&s, 0).2, 12.0));
}

#[test]
fn ac21_enter_with_nothing_edited_places_the_live_shape() {
    let mut s = new_session(R);
    start(&mut s, pt(100.0, 50.0));
    hover(&mut s, pt(130.0, 40.0));
    assert!(s.place_placement());
    let (x, y, w, h, _) = rect(&s, 0);
    assert!(near(x, 100.0) && near(y, 40.0) && near(w, 30.0) && near(h, 10.0));
}

#[test]
fn ac22_a_frozen_press_places_anywhere_with_the_typed_values() {
    let mut s = new_session(R);
    frozen_rect(&mut s);
    s.placement_edit(0, "40");
    s.placement_edit(1, "20");
    second_press(&mut s, pt(300.0, 300.0), false, false);
    assert_eq!(n_objects(&s), 1);
    let (x, y, w, h, _) = rect(&s, 0);
    assert!(near(x, 100.0) && near(y, 30.0) && near(w, 40.0) && near(h, 20.0));
}

#[test]
fn ac22_a_frozen_press_near_a_also_places() {
    let mut s = new_session(R);
    frozen_rect(&mut s);
    s.placement_edit(0, "40");
    s.pointer_down(pt(100.1, 50.0), false);
    assert_eq!(n_objects(&s), 1);
}

#[test]
fn ac23_invalid_fields_refuse_and_the_placement_continues() {
    let cases: [(&str, &str); 7] = [
        ("abc", "w"),
        ("", "w"),
        ("1.2.3", "w"),
        ("0", "w"),
        ("-5", "w"),
        ("20000000", "w"),
        ("5 kg", "w"),
    ];
    for (text, _) in cases {
        let mut s = new_session(R);
        frozen_rect(&mut s);
        let steps = s.undo_len();
        s.placement_edit(0, text);
        assert!(!s.place_placement(), "{text:?}");
        assert!(s.placement().is_some(), "{text:?}");
        assert_eq!(n_objects(&s), 0);
        assert_eq!(s.undo_len(), steps);
        assert_eq!(s.tool(), R);
        // a second click is refused the same way
        s.pointer_down(pt(200.0, 200.0), false);
        assert!(s.placement().is_some(), "{text:?} second click");
        assert_eq!(n_objects(&s), 0);
    }
}

#[test]
fn ac23_accepted_spellings() {
    for (text, expect) in [("12,5", 12.5), ("+12.5", 12.5), ("\u{2212}12.5", f64::NAN)] {
        let mut s = new_session(R);
        frozen_rect(&mut s);
        s.placement_edit(0, text);
        let placed = s.place_placement();
        if expect.is_nan() {
            assert!(
                !placed,
                "minus is accepted as '-', so a negative size is refused"
            );
        } else {
            assert!(placed, "{text}");
            assert!(near(rect(&s, 0).2, expect));
        }
    }
}

#[test]
fn ac23_any_finite_angle_is_accepted_and_shown_normalised() {
    let mut s = new_session(R);
    frozen_rect(&mut s);
    s.placement_edit(0, "40");
    s.placement_edit(1, "20");
    s.placement_edit(2, "450");
    assert!(s.place_placement());
    let (.., rot) = rect(&s, 0);
    assert!(near(rot, 90.0), "{rot}");
}

#[test]
fn ac24_unit_suffixes() {
    for (text, mm) in [("1in", 25.4), ("2 cm", 20.0), ("3MM", 3.0), ("1 IN", 25.4)] {
        let mut s = new_session(R);
        frozen_rect(&mut s);
        s.placement_edit(0, text);
        assert!(s.place_placement(), "{text}");
        assert!(near(rect(&s, 0).2, mm), "{text}: {}", rect(&s, 0).2);
    }
    let mut s = new_session(R);
    frozen_rect(&mut s);
    s.placement_edit(1, "5 °");
    assert!(!s.place_placement(), "a degree sign on a length is refused");
}

#[test]
fn ac24_centimetre_documents_show_and_read_cm() {
    let mut s = new_session(R);
    assert!(s.set_display_unit(DisplayUnit::Cm));
    frozen_rect(&mut s);
    assert_eq!(
        s.placement_field_text(PlacementField::Width).unwrap(),
        "3.00"
    );
    s.placement_edit(0, "4");
    assert!(s.place_placement());
    assert!(near(rect(&s, 0).2, 40.0));
}

#[test]
fn ac24_angle_accepts_a_degree_sign() {
    let mut s = new_session(R);
    frozen_rect(&mut s);
    s.placement_edit(2, "30°");
    assert!(s.place_placement());
    assert!(near(rect(&s, 0).4, 30.0));
}

#[test]
fn ac25_bar_changes_reach_the_placement_and_typed_values_are_kept() {
    let mut s = new_session(Tool::PolygonStar);
    s.set_poly_star_mode(curvyo_ui_core::PolyStarMode::Polygon);
    s.set_poly_star_point_count(curvyo_document_core::PointCount::new(5).unwrap());
    start(&mut s, pt(100.0, 100.0));
    hover(&mut s, pt(110.0, 100.0));
    let before = s.draw_list().triangles.len();
    s.placement_edit(0, "25");
    s.set_poly_star_point_count(curvyo_document_core::PointCount::new(20).unwrap());
    assert!(s.placement().is_some());
    assert_ne!(s.draw_list().triangles.len(), before);
    s.set_poly_star_mode(curvyo_ui_core::PolyStarMode::Star);
    assert!(s.place_placement());
    let p = prim(&s, 0);
    let Shape::Star {
        frame, point_count, ..
    } = p.shape
    else {
        panic!("{:?}", p.shape)
    };
    assert!(near(frame.radius.as_mm(), 25.0));
    assert_eq!(point_count.get(), 20);
}

// ---- Part C -----------------------------------------------------------------------------

#[test]
fn ac26_enter_starts_a_placement_at_the_pointer() {
    let mut s = new_session(R);
    hover(&mut s, pt(60.0, 70.0));
    key(&mut s, "Enter", false, false);
    assert!(s.placement().is_some());
    s.placement_edit(0, "30");
    s.placement_edit(1, "20");
    assert!(s.place_placement());
    let (x, y, w, h, _) = rect(&s, 0);
    assert!(near(x, 60.0) && near(y, 70.0) && near(w, 30.0) && near(h, 20.0));
}

#[test]
fn ac26_enter_without_pointer_uses_the_viewport_centre() {
    let mut s = new_session(R);
    s.resize_viewport(800.0, 600.0);
    s.pointer_leave();
    key(&mut s, "Enter", false, false);
    assert!(s.placement().is_some());
    let c = s.view().screen_to_document(400.0, 300.0);
    s.placement_edit(0, "30");
    s.placement_edit(1, "20");
    assert!(s.place_placement());
    let (x, y, w, h, _) = rect(&s, 0);
    assert!(
        near(x, c.x) && near(y, c.y) && near(w, 30.0) && near(h, 20.0),
        "{x} {y} vs {c:?}"
    );
}

#[test]
fn ac26_enter_with_an_untouched_size_places_nothing() {
    for tool in [R, Tool::Ellipse, Tool::PolygonStar] {
        let mut s = new_session(tool);
        hover(&mut s, pt(60.0, 70.0));
        key(&mut s, "Enter", false, false);
        assert!(s.placement().is_some());
        assert!(!s.place_placement(), "{tool:?}");
        assert!(s.placement().is_some());
        assert_eq!(n_objects(&s), 0);
    }
}

#[test]
fn ac26_rectangle_with_only_the_width_typed_refuses_the_zero_height() {
    let mut s = new_session(R);
    hover(&mut s, pt(60.0, 70.0));
    key(&mut s, "Enter", false, false);
    s.placement_edit(0, "30");
    assert!(!s.place_placement());
    assert_eq!(n_objects(&s), 0);
}

#[test]
fn ac26_enter_is_not_a_start_with_the_select_tool_or_a_running_placement() {
    let mut s = new_session(Tool::Select);
    key(&mut s, "Enter", false, false);
    assert!(s.placement().is_none());
    let mut s = new_session(R);
    start(&mut s, pt(100.0, 50.0));
    hover(&mut s, pt(130.0, 40.0));
    key(&mut s, "Enter", false, false);
    assert!(
        s.placement().is_some(),
        "Enter in the session does not place; the chip does"
    );
}

#[test]
fn ac27_after_a_keyboard_placement_the_select_tool_is_active() {
    let mut s = new_session(Tool::Ellipse);
    hover(&mut s, pt(60.0, 70.0));
    key(&mut s, "Enter", false, false);
    s.placement_edit(0, "5");
    assert!(s.place_placement());
    assert_eq!(s.tool(), Tool::Select);
}

// ---- Part D -----------------------------------------------------------------------------

#[test]
fn ac28_the_pivot_marker_shows_at_a_even_within_3px() {
    let mut s = new_session(R);
    let empty = s.draw_list().triangles.len();
    start(&mut s, pt(100.0, 50.0));
    assert!(s.draw_list().triangles.len() > empty, "marker at A");
    let a = pt(100.0, 50.0);
    assert!(
        s.draw_list()
            .triangles
            .iter()
            .any(|v| { (v.position.x - a.x).abs() < 3.0 && (v.position.y - a.y).abs() < 3.0 }),
        "something is drawn at A"
    );
    s.cancel_placement();
    assert_eq!(s.draw_list().triangles.len(), empty, "gone in that frame");
}

#[test]
fn ac30_live_region_sentences() {
    let mut s = new_session(R);
    start(&mut s, pt(100.0, 50.0));
    assert_eq!(
        s.take_placement_announcement().as_deref(),
        Some(
            "New rectangle. Type the width, Tab for height and angle. Enter places, Escape cancels."
        )
    );
    frozen_rect_inplace(&mut s);
    assert_eq!(
        s.take_placement_announcement().as_deref(),
        Some("Rectangle placed, 40 by 20 millimetres.")
    );
    s.set_tool(R);
    start(&mut s, pt(10.0, 10.0));
    let _ = s.take_placement_announcement();
    s.cancel_placement();
    assert_eq!(
        s.take_placement_announcement().as_deref(),
        Some("Cancelled.")
    );

    let mut s = new_session(Tool::Ellipse);
    start(&mut s, pt(10.0, 10.0));
    assert_eq!(
        s.take_placement_announcement().as_deref(),
        Some(
            "New ellipse. Type the radius, Space for diameter, Tab for angle. Enter places, Escape cancels."
        )
    );
    s.placement_edit(0, "10");
    s.place_placement();
    assert_eq!(
        s.take_placement_announcement().as_deref(),
        Some("Circle placed, radius 10 millimetres.")
    );
    let mut s = new_session(Tool::Ellipse);
    ellipse_at(&mut s);
    let _ = s.take_placement_announcement();
    s.place_placement();
    assert_eq!(
        s.take_placement_announcement().as_deref(),
        Some("Ellipse placed, 10 by 15 millimetres.")
    );
}

fn frozen_rect_inplace(s: &mut Session) {
    hover(s, pt(130.0, 40.0));
    s.placement_edit(0, "40");
    s.placement_edit(1, "20");
    s.place_placement();
}

#[test]
fn ac30_polygon_and_star_sentences() {
    let mut s = new_session(Tool::PolygonStar);
    s.set_poly_star_mode(curvyo_ui_core::PolyStarMode::Polygon);
    start(&mut s, pt(10.0, 10.0));
    let said = s.take_placement_announcement().unwrap();
    assert!(
        said.starts_with("New polygon.") && said.contains("outer radius"),
        "{said}"
    );
    s.placement_edit(0, "25");
    s.place_placement();
    assert_eq!(
        s.take_placement_announcement().as_deref(),
        Some("Polygon placed, outer radius 25 millimetres.")
    );
    s.set_tool(Tool::PolygonStar);
    s.set_poly_star_mode(curvyo_ui_core::PolyStarMode::Star);
    start(&mut s, pt(10.0, 10.0));
    s.placement_edit(0, "25");
    s.place_placement();
    let _ = s.take_placement_announcement();
}

#[test]
fn ac31_two_tap_creation() {
    let mut s = new_session(R);
    s.set_touch_input(true);
    start(&mut s, pt(100.0, 50.0));
    assert!(s.placement().is_some());
    // A second tap closer than 8 px to A is ignored.
    let k = s.view().scale();
    let close = pt(100.0 + 5.0 / k, 50.0);
    s.set_touch_input(true);
    s.pointer_down(close, false);
    s.pointer_up(close, false, false);
    assert!(
        s.placement().is_some(),
        "5 px is inside the touch dead zone"
    );
    s.set_touch_input(true);
    s.pointer_down(pt(130.0, 40.0), false);
    s.pointer_up(pt(130.0, 40.0), false, false);
    assert!(s.placement().is_none());
    let (x, y, w, h, _) = rect(&s, 0);
    assert!(near(x, 100.0) && near(y, 40.0) && near(w, 30.0) && near(h, 10.0));
}

// ---- Part E -----------------------------------------------------------------------------

#[test]
fn ac34_the_pen_and_select_tools_start_no_placement() {
    for tool in [Tool::Pen, Tool::Select, Tool::Node] {
        let mut s = new_session(tool);
        start(&mut s, pt(100.0, 50.0));
        assert!(s.placement().is_none(), "{tool:?}");
    }
}

#[test]
fn ac34_a_drag_still_shows_no_placement_and_escape_cancels_it() {
    let mut s = new_session(R);
    hover(&mut s, pt(0.0, 0.0));
    s.pointer_down(pt(0.0, 0.0), false);
    hover(&mut s, pt(50.0, 50.0));
    assert!(s.placement().is_none());
    s.escape();
    s.pointer_up(pt(50.0, 50.0), false, false);
    assert_eq!(n_objects(&s), 0);
}

#[test]
fn ac35_one_undo_removes_the_placed_shape_and_restores_the_selection() {
    let mut s = new_session(R);
    let before = s.pack("0.1.0").unwrap();
    frozen_rect(&mut s);
    s.placement_edit(0, "40");
    s.placement_edit(1, "20");
    let steps = s.undo_len();
    assert!(s.place_placement());
    assert_eq!(s.undo_len(), steps + 1);
    s.undo();
    assert_eq!(n_objects(&s), 0);
    assert_eq!(s.selected_object_count(), 0);
    let _ = before;
}

#[test]
fn ac35_typing_and_cancelling_add_no_history_and_write_nothing() {
    let mut s = new_session(Tool::Ellipse);
    let before = s.pack("0.1.0").unwrap();
    let steps = s.undo_len();
    ellipse_at(&mut s);
    s.placement_edit(0, "10");
    s.placement_toggle_diameter();
    s.cancel_placement();
    assert_eq!(s.pack("0.1.0").unwrap(), before);
    assert_eq!(s.undo_len(), steps);
}

#[test]
fn ac36_a_placed_shape_round_trips_through_the_file() {
    let mut s = new_session(R);
    frozen_rect(&mut s);
    s.placement_edit(0, "40");
    s.placement_edit(1, "20");
    s.placement_edit(2, "90");
    assert!(s.place_placement());
    let bytes = s.pack("0.1.0").unwrap();
    let d = unpack(5, &bytes).unwrap();
    let again = pack(&d, "0.1.0").unwrap();
    let d2 = unpack(6, &again).unwrap();
    assert_eq!(d.object_ids().len(), d2.object_ids().len());
}

#[test]
fn edge_zero_extent_on_one_axis_and_huge_values_do_not_panic() {
    let mut s = new_session(R);
    start(&mut s, pt(100.0, 50.0));
    hover(&mut s, pt(130.0, 50.0)); // zero height
    s.placement_edit(0, "30");
    assert!(!s.place_placement(), "a kept zero height is refused");
    s.placement_edit(0, "1e308");
    let _ = s.place_placement();
    s.placement_edit(0, "9999999");
    s.placement_edit(1, "9999999");
    let _ = s.place_placement();
    assert!(n_objects(&s) <= 1);
}

#[test]
fn edge_placement_outside_a_creation_tool_is_a_no_op() {
    let mut s = new_session(Tool::Select);
    s.placement_edit(0, "5");
    s.placement_toggle_diameter();
    assert!(!s.place_placement());
    assert!(!s.cancel_placement());
}
