//! Independent acceptance tests for `specs/0102-laser-fixtures` through the public session, written
//! from the specification and ADR 0049 and ADR 0050 before the implementation was read. Covers the
//! Fixtures block and its rows (criteria 1 to 7), selection (8 to 12), grid (13 to 17), holders,
//! clamps and stops (18 to 23), snapping switch (24, 27 partly), the profile and its fixture set
//! (28 to 30), files and goldens (31 to 34). Criteria 25 and 26 (the drag ghost and the slot cue)
//! and the snap of a drag are not reachable here without a placed page drag; see the report.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::assert_is_empty,
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::case_sensitive_file_extension_comparisons,
    clippy::items_after_statements,
    clippy::len_zero,
    clippy::bool_assert_comparison,
    clippy::too_many_lines
)]

use std::path::PathBuf;

#[path = "cpu_time/mod.rs"]
mod cpu_time;

use curvyo_document_core::{
    Color, Document, Length, NodeId, Point, RectBounds, StyleEdit, pack, unpack,
};
use curvyo_editor_wasm::{KeyInput, KeyOutcome, RowPress, Session, Tool};
use curvyo_ui_core::{EntryOutcome, MoveEntryMode};
use serde_json::{Value, json};

const LASER: &str = "curvyo.laser";

// ---- helpers ------------------------------------------------------------------------------------

fn one_page() -> Session {
    let d = Document::new(1);
    let id = d.create_rect(RectBounds::from_corners(
        Point::new(10.0, 20.0),
        Point::new(50.0, 40.0),
    ));
    d.edit_style(&[id], &StyleEdit::FillEnabled(true)).unwrap();
    d.edit_style(&[id], &StyleEdit::FillColor(Color { r: 255, g: 0, b: 0 }))
        .unwrap();
    let mut s = Session::open(9201, &pack(&d, "0.1.0").unwrap()).unwrap();
    // An empty library, as the app loads it at start (its profiles are read before a save).
    s.load_machine_profiles_json("[]");
    s
}

fn laser(s: &mut Session) -> String {
    s.add_page_of_kind(LASER, None).expect("a laser page")
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

fn manufacturing(s: &Session) -> Value {
    serde_json::from_str(&s.manufacturing_view_json()).unwrap()
}

fn machine(s: &Session) -> Value {
    manufacturing(s)["machine"].clone()
}

fn key(s: &mut Session, k: &str, ctrl: bool) -> KeyOutcome {
    s.key_down(KeyInput {
        key: k,
        ctrl,
        ..KeyInput::default()
    })
}

/// A point of the active laser page at `(x, y)` millimetres from its top-left corner. Session
/// points are document units (1 unit = 1 mm); the zoom only maps them to the screen.
fn at(s: &Session, x: f64, y: f64) -> Point {
    let o = s.active_page().origin;
    Point::new(o.x + x, o.y + y)
}

fn click(s: &mut Session, p: Point) {
    s.set_tool(Tool::Select);
    s.pointer_down(p, false);
    s.pointer_up(p, false, false);
}

/// Opens the chip of the key `k` (`m`, `r`, `s`) on the fixture `id` and commits `texts` in it:
/// the rectangle's chips size, move and turn fixtures (revised criteria 14, 18, 22, 23; the
/// typed fields of the first revision are gone).
fn chip(s: &mut Session, id: NodeId, k: &str, texts: &[&str]) -> EntryOutcome {
    s.set_tool(Tool::Select);
    assert!(s.select_fixture(id));
    assert_eq!(
        key(s, k, false),
        KeyOutcome::EntryOpened,
        "{k} opens a chip"
    );
    if k == "m" {
        let mode = MoveEntryMode {
            absolute: false,
            copy: false,
        };
        return s.commit_move_entry(texts[0], texts[1], mode);
    }
    s.commit_transform_entry_fields(texts, 0)
}

/// What a screen reader says of the fixture `id` selected, with its sizes.
fn announced(s: &mut Session, id: NodeId) -> String {
    assert!(s.select_fixture(id));
    block(s)["selection_announcement"]
        .as_str()
        .unwrap_or_default()
        .to_owned()
}

fn triangles(s: &Session) -> usize {
    s.frame_draw_list().triangles.len()
}

fn opened(s: &Session) -> Document {
    unpack(9299, &s.pack("0.1.0").unwrap()).unwrap()
}

fn holder_with_two_slots(s: &mut Session) -> NodeId {
    let holder = s.add_fixture("holder").expect("a holder");
    let a = s.add_slot(holder).expect("slot 1");
    s.set_slot_shape(a, "circle");
    s.set_slot_length(a, "d_mm", Length::from_mm(12.0));
    let b = s.add_slot(holder).expect("slot 2");
    s.set_slot_length(b, "cx_mm", Length::from_mm(70.0));
    holder
}

/// The text of every record in a JSON list of `{name, text}` entries or of plain strings.
fn record_texts(json: &str) -> Vec<String> {
    let v: Value = serde_json::from_str(json).unwrap_or(Value::Null);
    let mut out = Vec::new();
    fn walk(v: &Value, out: &mut Vec<String>) {
        match v {
            Value::String(s) if s.contains("format_version") => out.push(s.clone()),
            Value::Array(a) => a.iter().for_each(|x| walk(x, out)),
            Value::Object(o) => o.values().for_each(|x| walk(x, out)),
            _ => {}
        }
    }
    walk(&v, &mut out);
    out
}

/// Opens a golden and activates its laser page (the first page that takes the block).
fn open_laser(peer: u64, name: &str) -> Session {
    let bytes = golden(name);
    let d = unpack(peer, &bytes).unwrap();
    let mut s = Session::open(peer, &bytes).unwrap();
    for page in d.pages() {
        if s.activate_page(&page.id) && block(&s) != Value::Null {
            return s;
        }
    }
    panic!("{name} has no laser page with a block");
}

fn golden(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../curvyo-document-core/tests/fixtures")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("golden {name}: {e}"))
}

// ---- criterion 1: the block and its rows --------------------------------------------------------

#[test]
fn ac1_a_new_laser_page_shows_an_empty_block_and_the_five_kinds_can_be_added() {
    let mut s = one_page();
    laser(&mut s);
    assert!(rows(&s).is_empty(), "no fixture on a new laser page");
    assert_eq!(block(&s)["can_add_grid"], true);
    for kind in ["grid", "holder", "clamp", "stop", "screw"] {
        assert!(
            s.add_fixture(kind).is_some(),
            "{kind} is one of the five buttons"
        );
    }
    let names: Vec<String> = rows(&s)
        .iter()
        .map(|r| r["name"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(
        names,
        ["Grid 1", "Holder 1", "Clamp 1", "Stop 1", "Screw 1"],
        "creation order, one row each"
    );
    let kinds: Vec<String> = rows(&s)
        .iter()
        .map(|r| r["kind"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(kinds, ["grid", "holder", "clamp", "stop", "screw"]);
    for r in rows(&s) {
        assert_eq!(r["active"], true, "a new fixture is Active");
        assert_eq!(r["visible"], true, "the eye is on");
    }
}

#[test]
fn ac1_the_fixtures_block_is_on_a_laser_page_only() {
    let s = one_page();
    assert_eq!(block(&s), Value::Null, "the block is null off a laser page");
}

// ---- criterion 2: one press adds one fixture, one undo step, proposal sizes --------------------

#[test]
fn ac2_each_add_is_one_undo_step_and_selects_the_new_fixture() {
    let mut s = one_page();
    laser(&mut s);
    let steps = s.undo_len();
    assert!(s.add_fixture("holder").is_some());
    assert_eq!(s.undo_len(), steps + 1, "one undo step per add");
    let id = s.selected_fixture().expect("the new fixture is selected");
    assert_eq!(
        Some(id),
        s.fixture_part_named(row(&s, "Holder 1")["id"].as_str().unwrap())
    );
}

#[test]
fn ac2_the_proposal_sizes_of_a_new_fixture() {
    let mut s = one_page();
    laser(&mut s);
    let holder = s.add_fixture("holder").unwrap();
    let clamp = s.add_fixture("clamp").unwrap();
    let stop = s.add_fixture("stop").unwrap();
    let screw = s.add_fixture("screw").unwrap();
    assert_eq!(
        announced(&mut s, holder),
        "Holder 1 selected, 100.0 by 60.0 mm."
    );
    assert_eq!(
        announced(&mut s, clamp),
        "Clamp 1 selected, 40.0 by 20.0 mm."
    );
    assert_eq!(
        announced(&mut s, stop),
        "Stop 1 selected, legs 60.0 and 60.0 mm, thickness 5.0 mm."
    );
    assert_eq!(
        announced(&mut s, screw),
        "Screw 1 selected, thread 6.0 mm, head 10.0 mm."
    );
}

#[test]
fn ac2_one_grid_per_page_hides_the_grid_button_and_offers_no_second_one() {
    let mut s = one_page();
    laser(&mut s);
    assert!(s.add_fixture("grid").is_some());
    assert_eq!(
        block(&s)["can_add_grid"],
        false,
        "the Grid button is hidden"
    );
    let steps = s.undo_len();
    assert!(s.add_fixture("grid").is_none(), "a second grid is refused");
    assert_eq!(s.undo_len(), steps, "a refused add is not a step");
}

#[test]
fn ac2_a_new_name_takes_the_lowest_free_number_of_its_kind() {
    let mut s = one_page();
    laser(&mut s);
    s.add_fixture("holder");
    s.add_fixture("holder");
    let first = part(&s, "Holder 1");
    assert!(s.delete_fixture(first));
    s.add_fixture("holder");
    let names: Vec<String> = rows(&s)
        .iter()
        .map(|r| r["name"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(
        names,
        ["Holder 2", "Holder 1"],
        "the lowest unused number is taken again"
    );
}

// ---- criterion 3: rename ------------------------------------------------------------------------

#[test]
fn ac3_rename_is_one_undo_step_and_names_may_repeat() {
    let mut s = one_page();
    laser(&mut s);
    s.add_fixture("holder");
    s.add_fixture("clamp");
    let clamp = part(&s, "Clamp 1");
    let steps = s.undo_len();
    assert!(
        s.rename_fixture(clamp, "Holder 1"),
        "names need not be unique"
    );
    assert_eq!(s.undo_len(), steps + 1);
    assert_eq!(
        rows(&s).iter().filter(|r| r["name"] == "Holder 1").count(),
        2
    );
    s.undo();
    assert_eq!(
        row(&s, "Clamp 1")["kind"],
        "clamp",
        "undo restores the old name"
    );
}

#[test]
fn ac3_a_name_is_up_to_40_characters() {
    let mut s = one_page();
    laser(&mut s);
    s.add_fixture("holder");
    let h = part(&s, "Holder 1");
    let long = "x".repeat(41);
    s.rename_fixture(h, &long);
    let name = rows(&s)[0]["name"].as_str().unwrap().to_owned();
    assert!(
        name.chars().count() <= 40,
        "clamped or refused, never longer than 40: {name}"
    );
}

// ---- criterion 4: delete with its slots, one undo step restores it exactly ----------------------

#[test]
fn ac4_delete_removes_the_fixture_and_its_slots_and_one_undo_restores_them() {
    let mut s = one_page();
    laser(&mut s);
    s.add_fixture("engraved_grid");
    let holder = holder_with_two_slots(&mut s);
    let before = rows(&s);
    let steps = s.undo_len();
    assert!(s.delete_fixture(holder));
    assert_eq!(
        s.undo_len(),
        steps + 1,
        "one undo step for the whole removal"
    );
    assert_eq!(
        rows(&s).len(),
        1,
        "only the holder is removed, the grid stays"
    );
    s.undo();
    assert_eq!(
        rows(&s),
        before,
        "one undo restores the holder with its slots, active and place"
    );
}

// ---- criterion 5: several Active, Active switch, one step each ----------------------------------

#[test]
fn ac5_several_fixtures_can_be_active_at_once_and_switching_off_hides_them_from_the_bed() {
    let mut s = one_page();
    laser(&mut s);
    s.add_fixture("holder");
    s.add_fixture("clamp");
    let on = triangles(&s);
    let holder = part(&s, "Holder 1");
    let steps = s.undo_len();
    assert!(s.set_fixture_active(holder, false));
    assert_eq!(s.undo_len(), steps + 1, "one undo step per switch");
    assert_eq!(row(&s, "Holder 1")["active"], false);
    assert_eq!(row(&s, "Clamp 1")["active"], true, "the other stays Active");
    assert_eq!(rows(&s).len(), 2, "the row stays");
    assert!(triangles(&s) < on, "an inactive fixture is not drawn");
    s.undo();
    assert_eq!(triangles(&s), on, "switching on restores all of it");
    assert_eq!(row(&s, "Holder 1")["active"], true);
}

// ---- criterion 6: the eye --------------------------------------------------------------------

#[test]
fn ac6_the_eye_hides_a_fixture_without_changing_its_active_state_in_one_step() {
    let mut s = one_page();
    laser(&mut s);
    s.add_fixture("holder");
    let shown = triangles(&s);
    let h = part(&s, "Holder 1");
    let steps = s.undo_len();
    assert!(s.set_fixture_visible(h, false));
    assert_eq!(s.undo_len(), steps + 1);
    assert_eq!(row(&s, "Holder 1")["visible"], false);
    assert_eq!(
        row(&s, "Holder 1")["active"],
        true,
        "the eye does not change Active"
    );
    assert!(triangles(&s) < shown, "a hidden fixture is not drawn");
    s.undo();
    assert_eq!(triangles(&s), shown);
}

// ---- criteria 8 to 12: selection ----------------------------------------------------------------

#[test]
fn ac8_a_press_on_the_outline_of_an_active_holder_selects_it_alone() {
    let mut s = one_page();
    laser(&mut s);
    s.add_fixture("holder");
    let h = part(&s, "Holder 1");
    s.escape();
    // The holder is centred on the 400 x 300 mm work area: its left edge lies at x = 150 mm.
    let outline = at(&s, 150.0, 150.0);
    click(&mut s, outline);
    assert_eq!(
        s.selected_fixture(),
        Some(h),
        "the outline selects the fixture"
    );
    assert_eq!(
        s.selected_object_count(),
        0,
        "no other object is in the selection"
    );
}

#[test]
fn ac9_a_press_inside_a_holder_does_not_select_it() {
    let mut s = one_page();
    laser(&mut s);
    s.add_fixture("holder");
    s.escape();
    let inside = at(&s, 200.0, 150.0);
    click(&mut s, inside);
    assert_eq!(s.selected_fixture(), None, "inside the board is not a hit");
}

#[test]
fn ac10_select_all_and_marquee_never_select_a_fixture() {
    let mut s = one_page();
    laser(&mut s);
    s.add_fixture("holder");
    s.add_fixture("clamp");
    s.escape();
    key(&mut s, "a", true);
    assert_eq!(s.selected_fixture(), None, "Select All selects no fixture");
    s.escape();
    s.set_tool(Tool::Select);
    let a = at(&s, 0.0, 0.0);
    let b = at(&s, 400.0, 300.0);
    s.pointer_down(Point::new(a.x - 20.0, a.y - 20.0), false);
    s.pointer_hover(Point::new(b.x + 20.0, b.y + 20.0), false, false);
    s.pointer_up(Point::new(b.x + 20.0, b.y + 20.0), false, false);
    assert_eq!(
        s.selected_fixture(),
        None,
        "a marquee over the page selects no fixture"
    );
}

#[test]
fn ac11_a_fixture_of_a_laser_page_that_is_not_active_is_not_hit() {
    let mut s = one_page();
    let first = laser(&mut s);
    s.add_fixture("holder");
    let outline = at(&s, 150.0, 150.0);
    let second = laser(&mut s);
    assert_ne!(first, second);
    s.escape();
    click(&mut s, outline);
    assert_eq!(
        s.selected_fixture(),
        None,
        "the press selects nothing on another page"
    );
    assert_eq!(
        s.active_page().id,
        first,
        "the press activates the laser page it lies on"
    );
}

#[test]
fn ac12_a_layers_row_selects_its_fixture_and_the_fixture_row_has_an_eye() {
    let mut s = one_page();
    laser(&mut s);
    s.add_fixture("holder");
    s.add_fixture("clamp");
    let clamp = part(&s, "Clamp 1");
    let holder = part(&s, "Holder 1");
    let tree = serde_json::from_str::<Value>(&s.layers_view_json(0, 50)).unwrap();
    let rows_l = tree["rows"].as_array().unwrap().clone();
    let fixture_rows: Vec<Value> = rows_l
        .into_iter()
        .filter(|r| r["kind"] == "fixture")
        .collect();
    assert_eq!(fixture_rows.len(), 2, "one Layers row per fixture");
    let id = fixture_rows[0]["id"]["counter"].as_u64();
    assert!(id.is_some());
    assert_eq!(fixture_rows[1]["fixture"]["active"], true);
    let _ = (fixture_rows[0]["fixture"]["active"].clone(), holder);
    // The Layers panel sends a fixture row's press as select_fixture(fixture.id) (LayersTab.tsx).
    let holder_row = fixture_rows
        .iter()
        .find(|r| r["fixture"]["id"] == row(&s, "Holder 1")["id"])
        .expect("the holder's Layers row");
    let holder_text = holder_row["fixture"]["id"].as_str().unwrap().to_owned();
    assert!(s.select_fixture(s.fixture_part_named(&holder_text).unwrap()));
    assert_eq!(
        s.selected_fixture(),
        Some(holder),
        "pressing the row selects the fixture"
    );
    let _ = (clamp, RowPress::Plain);
}

// ---- criterion 13: the grid is not moved or deleted by the Delete key --------------------------

#[test]
fn ac13_the_delete_key_does_not_remove_a_grid_and_says_why() {
    let mut s = one_page();
    laser(&mut s);
    s.add_fixture("grid");
    let grid = part(&s, "Grid 1");
    assert!(s.select_fixture(grid));
    key(&mut s, "Delete", false);
    assert_eq!(rows(&s).len(), 1, "the Delete key does nothing to a grid");
    assert_eq!(
        s.take_fixture_notice().unwrap_or_default(),
        "Remove the grid with its trash in the Fixtures block.",
        "the notice names the trash"
    );
}

#[test]
fn ac13_a_grid_has_no_position_to_type() {
    let mut s = one_page();
    laser(&mut s);
    s.add_fixture("grid");
    let grid = part(&s, "Grid 1");
    assert!(s.select_fixture(grid));
    assert_ne!(
        key(&mut s, "m", false),
        KeyOutcome::EntryOpened,
        "the grid never moves"
    );
}

// ---- criterion 14: move and turn one object, one step each -------------------------------------

#[test]
fn ac14_a_holder_moves_and_turns_as_one_object_with_its_slots_in_place() {
    let mut s = one_page();
    laser(&mut s);
    let holder = holder_with_two_slots(&mut s);
    let slots_before = row(&s, "Holder 1")["slots"].clone();
    let steps = s.undo_len();
    assert_eq!(
        chip(&mut s, holder, "m", &["20", "30"]),
        EntryOutcome::Committed
    );
    assert_eq!(s.undo_len(), steps + 1, "one undo step per move");
    assert_eq!(
        row(&s, "Holder 1")["slots"],
        slots_before,
        "the slots keep their place on the board"
    );
    let steps = s.undo_len();
    assert_eq!(chip(&mut s, holder, "r", &["30"]), EntryOutcome::Committed);
    assert_eq!(s.undo_len(), steps + 1, "one undo step per turn");
    assert_eq!(
        announced(&mut s, holder),
        "Holder 1 selected, 100.0 by 60.0 mm.",
        "never scaled"
    );
}

#[test]
fn ac14_a_clamp_moves_and_turns_by_its_chips() {
    let mut s = one_page();
    laser(&mut s);
    s.add_fixture("clamp");
    let c = part(&s, "Clamp 1");
    assert_eq!(
        chip(&mut s, c, "m", &["100", "50"]),
        EntryOutcome::Committed
    );
    assert_eq!(chip(&mut s, c, "r", &["90"]), EntryOutcome::Committed);
}

// ---- criteria 15 to 17: the grid's parameters, major lines and tapes ---------------------------

#[test]
fn ac15_the_grid_parameters_clamp_to_their_ranges_with_a_short_notice() {
    let mut s = one_page();
    laser(&mut s);
    s.add_fixture("grid");
    let g = part(&s, "Grid 1");
    assert!(s.set_fixture_length(g, "spacing_mm", Length::from_mm(0.0)));
    assert_eq!(
        row(&s, "Grid 1")["grid"]["spacing_mm"],
        1.0,
        "spacing 1 to 100 mm"
    );
    let notice = s.take_fixture_notice().unwrap_or_default();
    assert!(!notice.is_empty(), "a clamp is noticed");
    assert!(
        notice.chars().count() <= 70,
        "a notice of at most 70 characters"
    );
    assert!(s.set_fixture_length(g, "spacing_mm", Length::from_mm(250.0)));
    assert_eq!(row(&s, "Grid 1")["grid"]["spacing_mm"], 100.0);
    assert!(s.set_grid_major_every(g, 1.0));
    assert_eq!(
        row(&s, "Grid 1")["grid"]["major_every"],
        2,
        "major every 2 to 20"
    );
    assert!(s.set_grid_major_every(g, 25.0));
    assert_eq!(row(&s, "Grid 1")["grid"]["major_every"], 20);
}

#[test]
fn ac15_the_grid_defaults_and_tape_switches_are_one_step_each() {
    let mut s = one_page();
    laser(&mut s);
    s.add_fixture("grid");
    let g = part(&s, "Grid 1");
    let grid = row(&s, "Grid 1")["grid"].clone();
    assert_eq!(grid["spacing_mm"], 10.0);
    assert_eq!(grid["major_every"], 5);
    assert_eq!(grid["origin_x_mm"], 0.0);
    assert_eq!(grid["origin_y_mm"], 0.0);
    for edge in ["bottom", "left", "top", "right"] {
        assert_eq!(
            grid["tapes"][edge], true,
            "all four tapes on by default: {edge}"
        );
    }
    let steps = s.undo_len();
    assert!(s.set_grid_tape(g, "top", false));
    assert_eq!(s.undo_len(), steps + 1);
    assert_eq!(row(&s, "Grid 1")["grid"]["tapes"]["top"], false);
    assert!(
        !s.set_grid_tape(g, "middle", false),
        "an unknown edge is refused"
    );
}

#[test]
fn ac17_only_the_bottom_tape_gives_its_numbers_every_50_mm() {
    let mut s = one_page();
    laser(&mut s);
    s.add_fixture("grid");
    let g = part(&s, "Grid 1");
    for edge in ["top", "left", "right"] {
        assert!(s.set_grid_tape(g, edge, false));
    }
    let labels = s.fixture_labels();
    let bottom: Vec<String> = labels
        .iter()
        .filter(|l| l.y > 0.0)
        .map(|l| l.text.clone())
        .collect();
    // The generic laser is 400 mm wide: 0, 50, ..., 400 mm.
    assert_eq!(
        bottom,
        ["0", "50", "100", "150", "200", "250", "300", "350", "400"]
    );
}

// ---- criteria 18 to 23: holders, slots, clamps and stops ----------------------------------------

#[test]
fn ac18_the_holder_size_is_1_to_1000_mm_and_each_edit_is_one_step() {
    let mut s = one_page();
    laser(&mut s);
    s.add_fixture("holder");
    let h = part(&s, "Holder 1");
    assert!(
        !s.set_fixture_length(h, "width_mm", Length::from_mm(50.0)),
        "the block has no size field for a holder"
    );
    let steps = s.undo_len();
    assert_eq!(
        chip(&mut s, h, "s", &["0.2", "5000"]),
        EntryOutcome::Committed
    );
    assert_eq!(s.undo_len(), steps + 1);
    assert_eq!(announced(&mut s, h), "Holder 1 selected, 1.0 by 1000.0 mm.");
}

#[test]
fn ac19_a_slot_has_a_name_a_shape_and_its_centre_and_size_each_editable() {
    let mut s = one_page();
    laser(&mut s);
    let holder = s.add_fixture("holder").unwrap();
    let steps = s.undo_len();
    let slot = s.add_slot(holder).expect("Add slot");
    assert_eq!(s.undo_len(), steps + 1, "one undo step per add");
    let slots = row(&s, "Holder 1")["slots"].clone();
    assert_eq!(slots.as_array().unwrap().len(), 1);
    assert_eq!(slots[0]["name"], "Slot 1");
    assert_eq!(
        slots[0]["shape"], "rect",
        "a new slot is a 20 x 20 mm rectangle"
    );
    assert!(s.rename_slot(slot, "Pocket"));
    assert!(s.set_slot_shape(slot, "circle"));
    assert!(s.set_slot_length(slot, "d_mm", Length::from_mm(12.0)));
    let after = row(&s, "Holder 1")["slots"][0].clone();
    assert_eq!(after["name"], "Pocket");
    assert_eq!(after["shape"], "circle");
    assert!(s.remove_slot(slot));
    assert!(row(&s, "Holder 1")["slots"].as_array().unwrap().is_empty());
    s.undo();
    assert_eq!(
        row(&s, "Holder 1")["slots"].as_array().unwrap().len(),
        1,
        "undo restores the slot"
    );
}

#[test]
fn ac19_a_holder_holds_at_most_50_slots_and_the_51st_is_never_offered() {
    let mut s = one_page();
    laser(&mut s);
    let holder = s.add_fixture("holder").unwrap();
    for _ in 0..50 {
        assert!(s.add_slot(holder).is_some());
    }
    assert_eq!(
        row(&s, "Holder 1")["can_add_slot"],
        false,
        "the Add slot button is hidden"
    );
    assert!(s.add_slot(holder).is_none(), "a 51st slot is refused");
    assert_eq!(row(&s, "Holder 1")["slots"].as_array().unwrap().len(), 50);
}

#[test]
fn ac20_a_slot_outside_its_board_is_allowed_with_a_notice_and_sizes_clamp() {
    let mut s = one_page();
    laser(&mut s);
    let holder = s.add_fixture("holder").unwrap();
    let slot = s.add_slot(holder).unwrap();
    assert!(s.set_slot_length(slot, "cx_mm", Length::from_mm(500.0)));
    assert_eq!(
        row(&s, "Holder 1")["slots"][0]["cx_mm"],
        500.0,
        "outside is allowed"
    );
    let notice = s.take_fixture_notice().unwrap_or_default();
    assert_eq!(
        notice, "Slot Slot 1 lies outside Holder 1.",
        "the notice names both"
    );
    assert!(s.set_slot_length(slot, "w_mm", Length::from_mm(0.1)));
    assert_eq!(
        row(&s, "Holder 1")["slots"][0]["w_mm"],
        0.5,
        "below 0.5 mm is the nearest limit"
    );
    assert!(s.set_slot_length(slot, "w_mm", Length::from_mm(5000.0)));
    assert_eq!(row(&s, "Holder 1")["slots"][0]["w_mm"], 1000.0);
}

#[test]
fn ac22_23_a_clamp_and_a_stop_have_their_sizes_in_range() {
    let mut s = one_page();
    laser(&mut s);
    s.add_fixture("clamp");
    s.add_fixture("stop");
    let c = part(&s, "Clamp 1");
    assert_eq!(
        chip(&mut s, c, "s", &["0", "20.0"]),
        EntryOutcome::Committed
    );
    assert_eq!(announced(&mut s, c), "Clamp 1 selected, 1.0 by 20.0 mm.");
    let st = part(&s, "Stop 1");
    assert_eq!(
        chip(&mut s, st, "s", &["60.0", "60.0", "60"]),
        EntryOutcome::Committed
    );
    assert_eq!(
        announced(&mut s, st),
        "Stop 1 selected, legs 60.0 and 60.0 mm, thickness 50.0 mm.",
        "thickness 0.5 to 50 mm"
    );
    assert_eq!(
        chip(&mut s, st, "s", &["0", "60.0", "50.0"]),
        EntryOutcome::Committed
    );
    assert_eq!(
        announced(&mut s, st),
        "Stop 1 selected, legs 50.0 and 60.0 mm, thickness 50.0 mm.",
        "a leg never below the thickness (revised criterion 23)"
    );
}

// ---- criteria 24, 27: the Fixtures snap switch -------------------------------------------------

#[test]
fn ac24_the_fixtures_snap_switch_is_one_step_and_reads_on_by_default() {
    let mut s = one_page();
    laser(&mut s);
    assert_eq!(block(&s)["snapping_fixtures"], true, "absent reads as on");
    let steps = s.undo_len();
    assert!(s.set_snapping_fixtures(false));
    assert_eq!(s.undo_len(), steps + 1, "one step");
    assert_eq!(block(&s)["snapping_fixtures"], false);
    s.undo();
    assert_eq!(block(&s)["snapping_fixtures"], true);
}

// ---- criteria 28 to 30: the profile and its fixture set ----------------------------------------

#[test]
fn ac28_save_as_profile_stores_the_set_and_writes_format_2() {
    let mut s = one_page();
    laser(&mut s);
    s.add_fixture("grid");
    holder_with_two_slots(&mut s);
    let _ = s.save_machine_profile([7; 16], 50);
    let id = machine(&s)["profiles"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["builtin"] == false)
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(
        machine(&s)["fixtures_in_profile"],
        2,
        "Fixtures in profile: 2"
    );
    let writes = s.take_machine_writes_json();
    let texts = record_texts(&writes);
    let record: Value = serde_json::from_str(texts.last().expect("a record was written")).unwrap();
    assert_eq!(record["format_version"], 2, "any write is format 2");
    assert_eq!(record["fixtures"].as_array().unwrap().len(), 2);
    assert_eq!(record["id"], id);
    assert_eq!(record["fixtures"][1]["slots"].as_array().unwrap().len(), 2);
}

#[test]
fn ac28_30_a_format_1_profile_reads_as_an_empty_set_and_is_not_rewritten_on_read() {
    let mut s = one_page();
    let text = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../curvyo-laser-core/tests/fixtures/machine_profiles/format1_full_record.json"),
    )
    .unwrap();
    let name = curvyo_laser_core::record_file_name("6f1c2a4e-9b3d-4c1a-8e2f-0a1b2c3d4e5f");
    let library = json!([{ "name": name, "text": text }]).to_string();
    s.load_machine_profiles_json(&library);
    assert!(
        record_texts(&s.take_machine_writes_json()).is_empty(),
        "a read writes nothing"
    );
    laser(&mut s);
    assert_eq!(machine(&s)["fixtures_in_profile"], 0, "the empty set");
    assert!(
        record_texts(&s.take_machine_writes_json()).is_empty(),
        "opening writes nothing"
    );
}

#[test]
fn ac29_a_new_laser_page_from_a_profile_starts_with_its_set() {
    let mut a = one_page();
    laser(&mut a);
    a.add_fixture("grid");
    holder_with_two_slots(&mut a);
    let _ = a.save_machine_profile([8; 16], 50);
    let id = machine(&a)["profiles"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["builtin"] == false)
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let record = record_texts(&a.take_machine_writes_json())
        .last()
        .unwrap()
        .clone();
    let mut b = one_page();
    b.load_machine_profiles_json(
        &json!([{ "name": curvyo_laser_core::record_file_name(&id), "text": record }]).to_string(),
    );
    b.add_page_of_kind(LASER, Some(&id))
        .expect("a laser page from the profile");
    let names: Vec<String> = rows(&b)
        .iter()
        .map(|r| r["name"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(names.len(), 2, "the set arrives with the page: {names:?}");
}

#[test]
fn ac29_picking_a_profile_replaces_the_fixtures_in_one_undo_step_and_generic_removes_them() {
    let mut a = one_page();
    laser(&mut a);
    a.add_fixture("grid");
    holder_with_two_slots(&mut a);
    let _ = a.save_machine_profile([9; 16], 50);
    let id = machine(&a)["profiles"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["builtin"] == false)
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let record = record_texts(&a.take_machine_writes_json())
        .last()
        .unwrap()
        .clone();
    let mut b = one_page();
    b.load_machine_profiles_json(
        &json!([{ "name": curvyo_laser_core::record_file_name(&id), "text": record }]).to_string(),
    );
    laser(&mut b);
    b.add_fixture("clamp");
    let steps = b.undo_len();
    assert!(b.pick_machine_profile(&id));
    assert_eq!(b.undo_len(), steps + 1, "the pick is one undo step");
    assert_eq!(rows(&b).len(), 2, "the page's fixtures are the set");
    assert!(
        rows(&b).iter().all(|r| r["name"] != "Clamp 1"),
        "the old fixture is replaced"
    );
    b.undo();
    assert_eq!(rows(&b).len(), 1, "undo restores the old fixture");
    assert!(b.pick_machine_profile(&id));
    assert!(b.pick_machine_profile("generic-laser"));
    assert!(rows(&b).is_empty(), "Generic laser holds an empty set");
    assert!(
        b.take_fixture_notice()
            .is_some_and(|n| n.contains("undo restores"))
            || manufacturing(&b).to_string().contains("undo restores"),
        "the pick says the fixtures were removed"
    );
    b.undo();
    assert_eq!(rows(&b).len(), 2, "undo restores them");
}

#[test]
fn ac30_a_damaged_profile_record_is_skipped_with_its_notice() {
    let mut s = one_page();
    let bad =
        std::fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(
            "../curvyo-laser-core/tests/fixtures/machine_profiles/format2_damaged_fixture.json",
        ))
        .unwrap();
    let name = curvyo_laser_core::record_file_name("6f1c2a4e-9b3d-4c1a-8e2f-0a1b2c3d4e5f");
    s.load_machine_profiles_json(&json!([{ "name": name, "text": bad }]).to_string());
    laser(&mut s);
    assert_eq!(
        machine(&s)["profiles"].as_array().unwrap().len(),
        1,
        "only Generic laser is listed"
    );
    assert_eq!(
        machine(&s)["library_notices"].as_array().unwrap().len(),
        1,
        "one notice"
    );
}

// ---- criteria 31 to 34: files, goldens and round trip -------------------------------------------

#[test]
fn ac31_save_and_reopen_keep_every_value_of_the_fixtures() {
    let mut s = one_page();
    laser(&mut s);
    s.add_fixture("grid");
    let holder = holder_with_two_slots(&mut s);
    s.add_fixture("clamp");
    s.add_fixture("stop");
    s.add_fixture("screw");
    assert!(s.set_fixture_active(part(&s, "Grid 1"), false));
    assert!(s.set_fixture_visible(part(&s, "Clamp 1"), false));
    assert_eq!(
        chip(&mut s, holder, "m", &["33", "21"]),
        EntryOutcome::Committed
    );
    assert_eq!(chip(&mut s, holder, "r", &["30"]), EntryOutcome::Committed);
    let before = rows(&s);
    let bytes = s.pack("0.1.0").unwrap();
    let mut r = Session::open(9301, &bytes).unwrap();
    let d = unpack(9302, &bytes).unwrap();
    for page in d.pages() {
        if r.activate_page(&page.id) && block(&r) != Value::Null {
            break;
        }
    }
    assert_eq!(
        rows(&r),
        before,
        "every value, Active, eye, slot and place survive"
    );
}

#[test]
fn ac31_golden_v40_opens_its_laser_page_with_every_row() {
    let s = open_laser(9302, "fixtures_v40.curvyo");
    let rows = rows(&s);
    assert_eq!(
        rows.len(),
        6,
        "one grid, two holders, a clamp, a stop and a screw: {}",
        block(&s)
    );
    let grid = rows.iter().find(|r| r["kind"] == "grid").unwrap();
    assert_eq!(grid["active"], false, "the inactive grid");
    let clamp = rows.iter().find(|r| r["kind"] == "clamp").unwrap();
    assert_eq!(clamp["visible"], false, "the hidden clamp");
    let holders: Vec<&Value> = rows.iter().filter(|r| r["kind"] == "holder").collect();
    assert_eq!(holders.len(), 2);
    let slots: usize = holders
        .iter()
        .map(|h| h["slots"].as_array().unwrap().len())
        .sum();
    assert_eq!(slots, 4, "two holders with two slots each");
}

#[test]
fn ac32_an_unknown_kind_is_listed_as_unknown_and_only_renamed_or_removed() {
    let mut s = open_laser(9303, "fixtures_unknown_kind_v40.curvyo");
    let unknown: Vec<Value> = rows(&s)
        .into_iter()
        .filter(|r| r["kind"] == "unknown")
        .collect();
    assert_eq!(
        unknown.len(),
        1,
        "the unknown fixture is listed: {}",
        block(&s)
    );
    let name = unknown[0]["name"].as_str().unwrap().to_owned();
    let id = s
        .fixture_part_named(unknown[0]["id"].as_str().unwrap())
        .unwrap();
    let _ = s.set_fixture_active(id, false);
    assert_eq!(
        row(&s, &name)["active"],
        true,
        "an unknown fixture cannot be switched off"
    );
    let _ = s.select_fixture(id);
    let _ = key(&mut s, "m", false);
    assert!(
        rows(&s).iter().any(|r| r["kind"] == "unknown"),
        "and it is not moved away"
    );
    assert!(s.rename_fixture(id, "Turntable A"));
    assert!(s.delete_fixture(id), "it can be removed");
    assert!(!name.is_empty());
}

#[test]
fn ac33_a_damaged_golden_shows_one_notice_of_at_most_70_characters() {
    let s = open_laser(9304, "fixtures_damaged_v40.curvyo");
    let notices = block(&s)["damage_notices"].clone();
    let notice = notices
        .as_array()
        .and_then(|a| a.first())
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    assert!(!notice.is_empty(), "one notice per damaged fixture");
    assert!(
        notice.chars().count() <= 70,
        "at most 70 characters: {notice}"
    );
    assert!(
        notice.contains("damaged on"),
        "it counts the values: {notice}"
    );
}

#[test]
fn ac34_a_v39_grid_opens_active_with_four_tapes() {
    let s = open_laser(9305, "fixtures_grid_v39.curvyo");
    let grid = rows(&s)
        .into_iter()
        .find(|r| r["kind"] == "grid")
        .expect("the grid");
    assert_eq!(grid["active"], true);
    assert_eq!(grid["visible"], true);
    for edge in ["bottom", "left", "top", "right"] {
        assert_eq!(
            grid["grid"]["tapes"][edge], true,
            "tapes on all four edges by default"
        );
    }
}

// ---- criterion 36 and no network in the new code: sanity -----------------------------------------

#[test]
fn ac36_fixtures_add_nothing_to_an_output_and_are_in_no_svg() {
    let mut s = one_page();
    laser(&mut s);
    s.add_fixture("holder");
    s.add_fixture("grid");
    let d = opened(&s);
    assert_eq!(
        d.object_ids().len(),
        1,
        "only the drawing's rectangle, no fixture object"
    );
    assert_eq!(d.manufacturings().len(), 1);
    assert_eq!(d.manufacturings()[0].fixtures.len(), 2);
}

// ---- criterion 35: budgets, thread CPU time, release builds only --------------------------------

#[cfg(not(debug_assertions))]
mod budgets {
    use super::*;
    use curvyo_document_core::DocumentSize;

    fn page_with_50_fixtures() -> Session {
        let mut s = one_page();
        laser(&mut s);
        for i in 0..50 {
            let kind = ["holder", "clamp", "stop"][i % 3];
            s.add_fixture(kind).expect("a fixture");
            s.escape();
        }
        s
    }

    #[test]
    fn ac35b_a_press_on_a_page_with_50_fixtures_is_resolved_within_2_ms() {
        let mut s = page_with_50_fixtures();
        assert_eq!(rows(&s).len(), 50);
        let p = at(&s, 0.5, 0.5);
        s.set_tool(Tool::Select);
        let (_, cost) = cpu_time::cpu_time_of(|| {
            s.pointer_down(p, false);
            s.pointer_up(p, false, false);
        });
        assert!(cost.as_secs_f64() * 1000.0 <= 2.0, "press cost {cost:?}");
    }

    #[test]
    fn ac35c_the_fixtures_block_with_50_fixtures_shows_within_100_ms() {
        let s = page_with_50_fixtures();
        let (json, cost) = cpu_time::cpu_time_of(|| s.fixtures_view_json());
        assert!(json.len() > 2);
        assert!(cost.as_secs_f64() * 1000.0 <= 100.0, "block cost {cost:?}");
    }

    #[test]
    fn ac35d_the_grid_at_1_mm_over_1000_by_1000_mm_is_built_within_16_ms() {
        use curvyo_laser_core::{GridSpec, TapeEdges, layout};
        let spec = GridSpec {
            spacing: Length::from_mm(1.0),
            major_every: 5,
            tapes: TapeEdges::all(),
            corner: curvyo_document_core::OriginCorner::BottomLeft,
            origin_x: Length::from_mm(0.0),
            origin_y: Length::from_mm(0.0),
        };
        let (g, cost) =
            cpu_time::cpu_time_of(|| layout(spec, DocumentSize::from_mm(1000.0, 1000.0)));
        assert!(
            g.minor.len() + g.major.len() >= 2002,
            "2,002 lines and more"
        );
        assert!(cost.as_secs_f64() * 1000.0 <= 16.0, "grid cost {cost:?}");
    }
}
