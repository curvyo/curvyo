//! Independent acceptance tests for the revision 2 of `specs/0102-laser-fixtures` (2026-11-10,
//! after the review of PR #313) through the public session: the fifth kind `screw` (criteria 2, 37
//! to 40), no typed size or position fields for holder, clamp, stop and screw (criteria 14, 18, 22,
//! 23, D31), the handles and chips of a holder and a stop and the screw's diameter knobs (criteria
//! 14, 18, 22, 23, 38; ux-notes D22 to D27), the typed value giving way at its limit, one undo step
//! per commit, and the eye as one undo step. Written from the specification, ux-notes D22 to D27 and
//! ADR 0049 before the implementation was read for these points.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::too_many_lines,
    clippy::items_after_statements,
    clippy::cast_precision_loss
)]

use curvyo_document_core::{Color, Document, Length, NodeId, Point, RectBounds, StyleEdit, pack};
use curvyo_editor_wasm::{KeyInput, KeyOutcome, Session, Tool};
use curvyo_ui_core::EntryOutcome;
use serde_json::Value;

// ---- helpers (as in acceptance_0102_tester.rs) ---------------------------------------------------

fn one_page() -> Session {
    let d = Document::new(1);
    let id = d.create_rect(RectBounds::from_corners(
        Point::new(10.0, 20.0),
        Point::new(50.0, 40.0),
    ));
    d.edit_style(&[id], &StyleEdit::FillEnabled(true)).unwrap();
    d.edit_style(&[id], &StyleEdit::FillColor(Color { r: 255, g: 0, b: 0 }))
        .unwrap();
    let mut s = Session::open(9501, &pack(&d, "0.1.0").unwrap()).unwrap();
    s.load_machine_profiles_json("[]");
    s
}

fn laser(s: &mut Session) {
    s.add_page_of_kind("curvyo.laser", None)
        .expect("a laser page");
}

fn block(s: &Session) -> Value {
    serde_json::from_str(&s.fixtures_view_json()).unwrap()
}

fn rows(s: &Session) -> Vec<Value> {
    block(s)["rows"].as_array().cloned().unwrap_or_default()
}

fn row(s: &Session, name: &str) -> Value {
    rows(s)
        .into_iter()
        .find(|r| r["name"] == name)
        .unwrap_or_else(|| panic!("no row named {name}: {}", block(s)))
}

fn part(s: &Session, name: &str) -> NodeId {
    let id = row(s, name)["id"].as_str().unwrap().to_owned();
    s.fixture_part_named(&id).expect("the row's node")
}

fn key(s: &mut Session, k: &str) -> KeyOutcome {
    s.key_down(KeyInput {
        key: k,
        ..KeyInput::default()
    })
}

/// A point of the active laser page at `(x, y)` millimetres from its top-left corner.
fn at(s: &Session, x: f64, y: f64) -> Point {
    let o = s.active_page().origin;
    Point::new(o.x + x, o.y + y)
}

fn click(s: &mut Session, p: Point) {
    s.set_tool(Tool::Select);
    s.pointer_down(p, false);
    s.pointer_up(p, false, false);
}

/// Zooms to `percent` (the knobs are drawn from 72 px on screen, D25; at 100 % a 10 mm head is
/// 10 px and has no knob).
fn zoom_to_pct(s: &mut Session, percent: i64) {
    let current = s.zoom_percent() as f64;
    let factor = percent as f64 / current;
    s.wheel(0.0, -400.0 * factor.log2(), 0.0, 0.0, false, true);
    assert!((s.zoom_percent() - percent).abs() <= 1, "zoom {percent} %");
}

/// A double-click on a handle: two presses, then the double-click route (as the existing tests).
fn dbl(s: &mut Session, p: Point) {
    s.pointer_down(p, false);
    s.pointer_up(p, false, false);
    s.pointer_down(p, false);
    s.pointer_up(p, false, false);
    let _ = s.double_click(p, false, false);
}

/// A drag of a handle: press at `from`, move to `to`, release.
fn drag(s: &mut Session, from: Point, to: Point) {
    s.pointer_down(from, false);
    s.pointer_hover(to, false, false);
    s.pointer_up(to, false, false);
}

fn chip(s: &mut Session, id: NodeId, k: &str, texts: &[&str]) -> EntryOutcome {
    s.set_tool(Tool::Select);
    assert!(s.select_fixture(id));
    assert_eq!(key(s, k), KeyOutcome::EntryOpened, "{k} opens a chip");
    s.commit_transform_entry_fields(texts, 0)
}

/// What the selection announcement of the selected fixture says.
fn announced(s: &mut Session, id: NodeId) -> String {
    assert!(s.select_fixture(id));
    block(s)["selection_announcement"]
        .as_str()
        .unwrap_or_default()
        .to_owned()
}

/// True when a JSON key of this name occurs anywhere in the value.
fn has_key(v: &Value, name: &str) -> bool {
    match v {
        Value::Object(o) => o.iter().any(|(k, x)| k == name || has_key(x, name)),
        Value::Array(a) => a.iter().any(|x| has_key(x, name)),
        _ => false,
    }
}

// Work area of `one_page` with a laser page: 400 x 300 mm, a fixture of 100 x 60 mm centred at
// (200, 150). A screw is centred on the work area at (200, 150) with a head of 10 mm and a
// thread of 6 mm (criterion 37, Proposal).

// ---- criterion 2 and 37: the screw kind, its defaults and ranges -------------------------------

#[test]
fn ac2_37_a_screw_is_added_with_a_6_mm_thread_and_a_10_mm_head() {
    let mut s = one_page();
    laser(&mut s);
    let steps = s.undo_len();
    assert!(
        s.add_fixture("screw").is_some(),
        "the Screw button adds one"
    );
    assert_eq!(s.undo_len(), steps + 1, "one undo step 'Add screw'");
    let screw = row(&s, "Screw 1");
    assert_eq!(screw["active"], true, "a new fixture starts Active");
    let id = part(&s, "Screw 1");
    let ann = announced(&mut s, id);
    assert!(
        ann.contains("10.0"),
        "head 10 mm in the announcement: {ann}"
    );
    assert!(
        ann.contains("6.0"),
        "thread 6 mm in the announcement: {ann}"
    );
    assert!(
        !has_key(&block(&s), "size") && !has_key(&block(&s), "selected_pose"),
        "no typed size or pose in the view"
    );
}

#[test]
fn ac2_37_the_screw_is_listed_with_the_other_four_kinds_and_the_name_is_the_lowest_free_number() {
    let mut s = one_page();
    laser(&mut s);
    s.add_fixture("screw");
    s.add_fixture("screw");
    let names: Vec<String> = rows(&s)
        .iter()
        .map(|r| r["name"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(names, ["Screw 1", "Screw 2"]);
}

#[test]
fn ac37_the_diameters_have_the_range_0_5_to_1000_mm() {
    let mut s = one_page();
    laser(&mut s);
    s.add_fixture("screw");
    let id = part(&s, "Screw 1");
    // Head 1000 is allowed; thread 0.2 is set to its minimum 0.5 with the Action notice.
    assert_eq!(
        chip(&mut s, id, "s", &["1000", "0.2"]),
        EntryOutcome::Committed
    );
    let ann = announced(&mut s, id);
    assert!(ann.contains("1000.0"), "head 1000 mm kept: {ann}");
    assert!(ann.contains("0.5"), "thread 0.2 set to 0.5: {ann}");
    assert!(!ann.contains("0.2 "), "the typed 0.2 is gone: {ann}");
}

#[test]
fn ac37_a_typed_thread_above_the_head_is_set_to_the_head_and_the_head_stays() {
    let mut s = one_page();
    laser(&mut s);
    s.add_fixture("screw");
    let id = part(&s, "Screw 1");
    let steps = s.undo_len();
    // Head 10 (kept), thread typed 20: the thread gives way to the head.
    assert_eq!(
        chip(&mut s, id, "s", &["10", "20"]),
        EntryOutcome::Committed
    );
    assert_eq!(s.undo_len(), steps + 1, "one undo step per commit");
    let ann = announced(&mut s, id);
    assert!(!ann.contains("20.0"), "the thread is not 20 mm: {ann}");
    assert!(ann.contains("10.0"), "the head is 10 mm: {ann}");
}

#[test]
fn ac37_a_typed_head_below_the_thread_is_set_to_the_thread() {
    let mut s = one_page();
    laser(&mut s);
    s.add_fixture("screw");
    let id = part(&s, "Screw 1");
    // Thread 6 (default, kept), head typed 3: the head gives way to the thread.
    assert_eq!(chip(&mut s, id, "s", &["3", "6"]), EntryOutcome::Committed);
    let ann = announced(&mut s, id);
    assert!(!ann.contains("3.0"), "the head is not 3 mm: {ann}");
    assert!(
        ann.contains("6.0"),
        "the head is set to the thread's 6 mm: {ann}"
    );
}

// ---- criterion 38: the knobs of the screw, no rotate, no resize -----------------------------------

#[test]
fn ac38_the_head_knob_drag_stops_at_the_thread_without_moving_past_it() {
    let mut s = one_page();
    laser(&mut s);
    zoom_to_pct(&mut s, 800);
    s.add_fixture("screw");
    let id = part(&s, "Screw 1");
    // Select by its outline (the head circle at 5 mm from the centre), then drag the Head knob at
    // 0 degrees (east) to the centre: the head stops at the thread's 6 mm.
    let p0 = at(&s, 205.0, 150.0);
    click(&mut s, p0);
    assert_eq!(s.selected_fixture(), Some(id));
    let steps = s.undo_len();
    let p0 = at(&s, 205.0, 150.0);
    let p1 = at(&s, 200.0, 150.0);
    drag(&mut s, p0, p1);
    assert_eq!(
        s.undo_len(),
        steps + 1,
        "one undo step per drag, 'Resize fixture'"
    );
    let ann = announced(&mut s, id);
    assert!(!ann.contains("10.0"), "the head is no longer 10 mm: {ann}");
    assert!(ann.contains("6.0"), "the head stops at 6 mm: {ann}");
}

#[test]
fn ac38_the_thread_knob_drag_stops_at_the_head() {
    let mut s = one_page();
    laser(&mut s);
    zoom_to_pct(&mut s, 800);
    s.add_fixture("screw");
    let id = part(&s, "Screw 1");
    let p0 = at(&s, 205.0, 150.0);
    click(&mut s, p0);
    // The Thread knob sits on the thread circle at 180 degrees (west), at 3 mm from the centre.
    let p0 = at(&s, 197.0, 150.0);
    let p1 = at(&s, 100.0, 150.0);
    drag(&mut s, p0, p1);
    let ann = announced(&mut s, id);
    assert!(!ann.contains("6.0"), "the thread is no longer 6 mm: {ann}");
    assert!(
        ann.contains("10.0"),
        "the thread stops at the head's 10 mm: {ann}"
    );
}

#[test]
fn ac38_the_screw_has_no_rotate_no_r_and_no_size_squircle_entry() {
    let mut s = one_page();
    laser(&mut s);
    s.add_fixture("screw");
    let id = part(&s, "Screw 1");
    assert!(s.select_fixture(id));
    assert_ne!(
        key(&mut s, "r"),
        KeyOutcome::EntryOpened,
        "R does not turn a screw"
    );
    s.escape();
    // The corner of the box around the head circle carries no resize squircle: a double-click
    // there opens no chip.
    assert!(!s.double_click(at(&s, 205.0, 155.0), false, false));
}

#[test]
fn ac38_a_double_click_on_the_head_knob_opens_one_field_chip_that_takes_a_value() {
    let mut s = one_page();
    laser(&mut s);
    zoom_to_pct(&mut s, 800);
    s.add_fixture("screw");
    let id = part(&s, "Screw 1");
    let p0 = at(&s, 205.0, 150.0);
    click(&mut s, p0);
    let k = at(&s, 205.0, 150.0);
    dbl(&mut s, k);
    assert_eq!(
        s.commit_transform_entry_fields(&["12"], 0),
        EntryOutcome::Committed,
        "one field, mm"
    );
    let ann = announced(&mut s, id);
    assert!(ann.contains("12.0"), "head 12 mm: {ann}");
}

// ---- criterion 14, 18, 22, 23: no typed size or position fields -------------------------------

#[test]
fn d31_no_fixture_has_a_typed_size_pose_or_stop_field_in_the_view() {
    let mut s = one_page();
    laser(&mut s);
    s.add_fixture("holder");
    s.add_fixture("clamp");
    s.add_fixture("stop");
    s.add_fixture("screw");
    for name in ["Holder 1", "Clamp 1", "Stop 1", "Screw 1"] {
        let id = part(&s, name);
        assert!(s.select_fixture(id));
        let v = block(&s);
        assert!(!has_key(&v, "selected_pose"), "{name}: no selected_pose");
        assert!(!has_key(&v, "size"), "{name}: no size");
        assert!(!has_key(&v, "stop"), "{name}: no stop");
        let r = row(&s, name);
        for k in [
            "width",
            "height",
            "leg_a",
            "leg_b",
            "thickness",
            "head",
            "thread",
            "x",
            "y",
            "angle",
        ] {
            assert!(r.get(k).is_none(), "{name}: row has no typed field {k}");
        }
        assert!(
            !s.set_fixture_length(id, "width_mm", Length::from_mm(50.0)),
            "{name}: no width is set through the block"
        );
    }
}

#[test]
fn d31_a_selected_holder_resize_is_one_step_and_a_slot_keeps_its_world_place() {
    let mut s = one_page();
    laser(&mut s);
    let holder = s.add_fixture("holder").unwrap();
    let slot = s.add_slot(holder).expect("Add slot");
    let _ = slot;
    // Holder 100 x 60 centred at (200, 150): top-left (150, 120); the new 20 x 20 slot is centred
    // on the board at local (50, 30), world (200, 150).
    let p0 = at(&s, 150.0, 150.0);
    click(&mut s, p0);
    let steps = s.undo_len();
    // The north edge squircle (top edge midpoint at (200, 120)) drags up by 20 mm: the top edge
    // moves to y = 100 and the bottom edge stays at y = 180.
    let p0 = at(&s, 200.0, 120.0);
    let p1 = at(&s, 200.0, 100.0);
    drag(&mut s, p0, p1);
    assert_eq!(s.undo_len(), steps + 1, "one undo step per release");
    let slot_row = row(&s, "Holder 1")["slots"][0].clone();
    // The slot keeps its place on the page: its centre in the board's frame moves by the origin.
    let cy = slot_row["cy_mm"].as_f64().expect("cy_mm in the view");
    assert!(
        (cy - 50.0).abs() < 1e-6,
        "a north resize writes the slot centre in the same step: cy_mm {cy}"
    );
}

#[test]
fn d31_a_typed_holder_size_outside_1_to_1000_gives_way_with_a_notice_and_one_step() {
    let mut s = one_page();
    laser(&mut s);
    let h = s.add_fixture("holder").unwrap();
    let steps = s.undo_len();
    assert_eq!(
        chip(&mut s, h, "s", &["0.2", "5000"]),
        EntryOutcome::Committed
    );
    assert_eq!(s.undo_len(), steps + 1);
    assert_eq!(
        announced(&mut s, h),
        "Holder 1 selected, 1.0 by 1000.0 mm.",
        "set to 1 and 1000"
    );
}

#[test]
fn d31_a_holder_and_clamp_are_turned_and_moved_by_their_chips_as_one_object() {
    let mut s = one_page();
    laser(&mut s);
    let c = s.add_fixture("clamp").unwrap();
    let steps = s.undo_len();
    assert_eq!(chip(&mut s, c, "m", &["50", "50"]), EntryOutcome::Committed);
    assert_eq!(s.undo_len(), steps + 1, "Move fixture is one step");
    let steps = s.undo_len();
    assert_eq!(chip(&mut s, c, "r", &["45"]), EntryOutcome::Committed);
    assert_eq!(s.undo_len(), steps + 1, "Rotate fixture is one step");
}

// ---- criterion 23 and D24: the stop ---------------------------------------------------------------

#[test]
fn d24_a_stop_leg_below_the_thickness_is_set_to_the_thickness() {
    let mut s = one_page();
    laser(&mut s);
    let st = s.add_fixture("stop").unwrap();
    // Thickness 5 (default); leg 1 typed 3 gives way to 5; leg 2 stays 60.
    assert_eq!(
        chip(&mut s, st, "s", &["3", "60", "5"]),
        EntryOutcome::Committed
    );
    let ann = announced(&mut s, st);
    assert!(
        ann.contains("legs 5.0 and 60.0"),
        "leg 1 is set to 5 mm, leg 2 kept: {ann}"
    );
}

#[test]
fn d24_a_stop_thickness_above_the_shorter_leg_is_set_to_that_leg_or_50() {
    let mut s = one_page();
    laser(&mut s);
    let st = s.add_fixture("stop").unwrap();
    // Legs 20 and 60: the thickness is at most the shorter leg (20).
    assert_eq!(
        chip(&mut s, st, "s", &["20", "60", "40"]),
        EntryOutcome::Committed
    );
    let ann = announced(&mut s, st);
    assert!(
        ann.contains("thickness 20.0"),
        "thickness at most 20: {ann}"
    );
    assert!(
        ann.contains("legs 20.0 and 60.0"),
        "the legs do not move: {ann}"
    );
}

#[test]
fn d23_a_stop_has_leg_and_thickness_knobs_and_no_corner_or_edge_resize() {
    let mut s = one_page();
    laser(&mut s);
    zoom_to_pct(&mut s, 800);
    let st = s.add_fixture("stop").unwrap();
    assert!(s.select_fixture(st));
    // A double-click on the middle of the top edge of leg 1 is the Thickness knob (its chip).
    // The stop's bounding box is centred on (200, 150), 60 x 60 mm: its top-left is (170, 120).
    // Leg 1 runs along the top from (170, 120) to (230, 120), its thickness knob at (200, 120).
    let k = at(&s, 200.0, 120.0);
    dbl(&mut s, k);
    assert_eq!(
        s.commit_transform_entry_fields(&["8"], 0),
        EntryOutcome::Committed
    );
    assert!(announced(&mut s, st).contains("thickness 8.0"));
    // The bottom-right corner of the box carries no resize squircle and no chip.
    assert!(!s.double_click(at(&s, 230.0, 180.0), false, false));
}

// ---- criterion 6 and the eye ------------------------------------------------------------------

#[test]
fn ac6_each_eye_switch_is_one_undo_step_for_every_kind() {
    let mut s = one_page();
    laser(&mut s);
    s.add_fixture("grid");
    s.add_fixture("holder");
    s.add_fixture("clamp");
    s.add_fixture("stop");
    s.add_fixture("screw");
    for name in ["Grid 1", "Holder 1", "Clamp 1", "Stop 1", "Screw 1"] {
        let id = part(&s, name);
        let steps = s.undo_len();
        assert!(s.set_fixture_visible(id, false), "{name} eye off");
        assert_eq!(s.undo_len(), steps + 1, "{name}: one undo step for the eye");
        assert_eq!(
            row(&s, name)["active"],
            true,
            "{name}: the eye keeps Active"
        );
    }
}

// ---- criterion 40 and 5: the screw is not a fit target and an inactive screw is not drawn ------

#[test]
fn ac40_an_inactive_screw_is_not_drawn_and_an_active_one_is() {
    let mut s = one_page();
    laser(&mut s);
    let base = s.frame_draw_list().triangles.len();
    s.add_fixture("screw");
    let on = s.frame_draw_list().triangles.len();
    assert!(on > base, "an active screw is drawn");
    let id = part(&s, "Screw 1");
    assert!(s.set_fixture_active(id, false));
    assert_eq!(
        s.frame_draw_list().triangles.len(),
        base,
        "inactive: not drawn"
    );
}

// ---- criterion 8 and 9: the selected fixture's handles are hit before its outline --------------

#[test]
fn ac8_the_selected_holder_north_handle_is_hit_before_its_outline() {
    let mut s = one_page();
    laser(&mut s);
    let h = s.add_fixture("holder").unwrap();
    let p0 = at(&s, 150.0, 150.0);
    click(&mut s, p0);
    assert_eq!(s.selected_fixture(), Some(h));
    // The top edge midpoint is both the outline and the N squircle: the handle wins, so a drag
    // resizes (one step) and does not move the holder (its left edge stays at x = 150).
    let steps = s.undo_len();
    let p0 = at(&s, 200.0, 120.0);
    let p1 = at(&s, 200.0, 110.0);
    drag(&mut s, p0, p1);
    assert_eq!(s.undo_len(), steps + 1);
    let ann = announced(&mut s, h);
    assert!(
        ann.contains("100.0 by 70.0"),
        "the north edge changes the height only: {ann}"
    );
}

// ---- criterion 40 with the profile: a screw is in the profile set (Frage 175 A) ----------------

#[test]
fn ac28_a_screw_is_stored_in_the_profile_set() {
    let mut s = one_page();
    laser(&mut s);
    s.add_fixture("screw");
    let _ = s.save_machine_profile([9; 16], 51);
    let machine =
        serde_json::from_str::<Value>(&s.manufacturing_view_json()).unwrap()["machine"].clone();
    assert_eq!(machine["fixtures_in_profile"], 1, "Fixtures in profile: 1");
}
