//! Independent acceptance tests for `specs/0101-laser-operations` through the public session:
//! the Operations tab and its place (criterion 1), adding, naming, colouring and deleting
//! operations with their undo steps (2 to 4), Cuts last (5, 6), assignment by selection, tick
//! and removal (8, 9), guides (10), copies (14), Show and Output (15, 16), the summary (17),
//! the view switch (18) and silent drops on delete (27).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::too_many_lines,
    clippy::similar_names,
    clippy::many_single_char_names,
    clippy::assert_is_empty,
    clippy::needless_pass_by_value,
    missing_docs
)]

use curvyo_document_core::{
    Document, DocumentSize, NewOutputPage, NodeId, OperationSwitch, Page, Point, RectBounds, pack,
    unpack,
};
use curvyo_editor_wasm::{KeyInput, Session, Tool};
use serde_json::Value;

const LASER: &str = "curvyo.laser";

struct Fx {
    s: Session,
    l: Page,
    r1: NodeId,
}

fn rect(d: &Document, page: &Page, x: f64, y: f64) -> NodeId {
    d.create_rect(RectBounds::from_corners(
        Point::new(page.origin.x + x, page.origin.y + y),
        Point::new(page.origin.x + x + 20.0, page.origin.y + y + 10.0),
    ))
}

/// A laser page with three rectangles on it, the laser page active, no history.
fn fixture() -> Fx {
    let d = Document::new(1);
    let made = d
        .add_output_page(NewOutputPage {
            process: LASER,
            label: "Laser",
            profile: None,
            work_area: DocumentSize::from_mm(400.0, 300.0),
        })
        .unwrap();
    let l = d.pages().into_iter().find(|p| p.id == made.id).unwrap();
    let r1 = rect(&d, &l, 10.0, 10.0);
    rect(&d, &l, 60.0, 10.0);
    rect(&d, &l, 10.0, 100.0);
    let mut s = Session::open(7, &pack(&d, "0101-tester").unwrap()).unwrap();
    s.resize_viewport(1600.0, 1000.0);
    s.set_tool(Tool::Select);
    s.activate_page(&l.id);
    s.wipe_history();
    Fx { s, l, r1 }
}

/// The top edge midpoint of the rectangle placed at `(x, y)` on `page`, in document units. Hits
/// select by outline, so a click lands on the edge.
fn centre(page: &Page, x: f64, y: f64) -> Point {
    Point::new(page.origin.x + x + 10.0, page.origin.y + y)
}

fn click(s: &mut Session, at: Point) {
    s.pointer_hover(at, false, false);
    s.pointer_down(at, false);
    s.pointer_up(at, false, false);
}

fn shift_click(s: &mut Session, at: Point) {
    s.modifiers_changed(true, false, false);
    s.pointer_hover(at, true, false);
    s.pointer_down(at, true);
    s.pointer_up(at, true, false);
    s.modifiers_changed(false, false, false);
}

fn key(s: &mut Session, k: &str, ctrl: bool, shift: bool) {
    s.key_down(KeyInput {
        key: k,
        ctrl,
        shift,
        ..KeyInput::default()
    });
}

fn view(s: &Session) -> Value {
    serde_json::from_str(&s.operations_view_json()).unwrap()
}

fn rows(s: &Session) -> Vec<Value> {
    view(s)["rows"].as_array().unwrap().clone()
}

fn names(s: &Session) -> Vec<String> {
    rows(s)
        .iter()
        .map(|r| r["name"].as_str().unwrap().to_owned())
        .collect()
}

fn count(s: &Session, key: &str) -> u64 {
    view(s)["summary"][key].as_u64().unwrap()
}

fn row_items(s: &Session, name: &str) -> u64 {
    rows(s)
        .iter()
        .find(|r| r["name"] == name)
        .map(|r| r["items"].as_u64().unwrap())
        .unwrap()
}

fn tab_names(s: &Session) -> Vec<&'static str> {
    s.panel_view().tabs.iter().map(|t| t.name).collect()
}

// ---- criterion 1: the tab ----------------------------------------------------------------------

#[test]
fn c1_operations_tab_exists_on_a_laser_page_after_document_only() {
    let mut f = fixture();
    let tabs = tab_names(&f.s);
    let at = tabs.iter().position(|t| *t == "operations");
    let doc = tabs.iter().position(|t| *t == "document").unwrap();
    assert_eq!(at, Some(doc + 1), "tabs were {tabs:?}");

    let drawing = f.s.add_page_of_kind("drawing", None).unwrap();
    f.s.activate_page(&drawing);
    assert!(
        !tab_names(&f.s).contains(&"operations"),
        "a drawing page has no Operations tab"
    );
    let sketch = f.s.add_page_of_kind("sketch", None).unwrap();
    f.s.activate_page(&sketch);
    assert!(!tab_names(&f.s).contains(&"operations"));
}

#[test]
fn c1_a_new_laser_page_has_an_empty_list_and_nothing_runs() {
    let f = fixture();
    assert!(rows(&f.s).is_empty());
    assert_eq!(count(&f.s, "operations"), 0);
    assert_eq!(count(&f.s, "running"), 0);
    assert_eq!(
        count(&f.s, "not_assigned"),
        3,
        "three rectangles, none assigned"
    );
    assert!(
        view(&f.s)["cuts_last"].as_bool().unwrap(),
        "Cuts last defaults on"
    );
}

// ---- criterion 2: adding -----------------------------------------------------------------------

#[test]
fn c2_press_adds_named_operation_in_section_with_show_and_output_on() {
    let mut f = fixture();
    let before = f.s.undo_len();
    let cut = f.s.press_operation_kind("cut").unwrap();
    assert_eq!(f.s.undo_len(), before + 1, "one undo step");
    let row = rows(&f.s)[0].clone();
    assert_eq!(row["name"], "Cut 1");
    assert_eq!(row["kind"], "cut");
    // Customer 2026-11-10: no colour in the row (tester change: the colour assertions were removed).
    assert!(row.get("colour").is_none());
    assert_eq!(row["section"], "cutting");
    assert_eq!(row["show"], true);
    assert_eq!(row["output"], true);
    assert_eq!(row["margin_mm"].as_f64().unwrap(), 0.0);
    assert!(row["id"].is_object(), "the row carries its node id");
    assert!(format!("{cut:?}").contains("NodeId"));

    f.s.press_operation_kind("cut").unwrap();
    f.s.press_operation_kind("score").unwrap();
    assert_eq!(
        names(&f.s),
        vec!["Score 1", "Cut 1", "Cut 2"],
        "marking lands above cutting"
    );
    // Customer 2026-11-10: the colour list of this test was removed (tester change).
}

#[test]
fn c2_press_undo_restores_the_list_in_one_step() {
    let mut f = fixture();
    f.s.press_operation_kind("engrave").unwrap();
    assert_eq!(names(&f.s), vec!["Engrave 1"]);
    f.s.undo();
    assert!(
        rows(&f.s).is_empty(),
        "one undo removes the added operation"
    );
}

#[test]
fn c2_name_takes_the_lowest_unused_number_of_that_kind() {
    let mut f = fixture();
    let c1 = f.s.press_operation_kind("cut").unwrap();
    f.s.press_operation_kind("cut").unwrap();
    assert_eq!(names(&f.s), vec!["Cut 1", "Cut 2"]);
    assert!(f.s.delete_operation(c1));
    f.s.press_operation_kind("cut").unwrap();
    assert_eq!(names(&f.s), vec!["Cut 2", "Cut 1"], "Cut 1 is free again");
}

// ---- criterion 3: rename (the colour part was removed by the customer, 2026-11-10) -----------

#[test]
fn c3_rename_shows_at_once_and_is_one_undo_step() {
    let mut f = fixture();
    let id = f.s.press_operation_kind("cut").unwrap();
    let before = f.s.undo_len();
    assert!(f.s.rename_operation(id, "Outer frame"));
    assert_eq!(f.s.undo_len(), before + 1);
    assert_eq!(names(&f.s), vec!["Outer frame"]);
    // Customer 2026-11-10: the colour step of this test was removed (tester change).
    let _ = id;
}

#[test]
fn c3_an_empty_name_reverts_and_names_need_not_be_unique() {
    let mut f = fixture();
    let a = f.s.press_operation_kind("cut").unwrap();
    f.s.press_operation_kind("score").unwrap();
    let _ = f.s.rename_operation(a, "");
    assert_eq!(
        names(&f.s),
        vec!["Score 1", "Cut 1"],
        "empty name keeps the old one"
    );
    assert!(
        f.s.rename_operation(a, "Score 1"),
        "a duplicate name is allowed"
    );
    assert_eq!(names(&f.s), vec!["Score 1", "Score 1"]);
}

// ---- criterion 4: delete -----------------------------------------------------------------------

#[test]
fn c4_delete_removes_memberships_and_undo_restores_them_exactly() {
    let mut f = fixture();
    let cut = f.s.press_operation_kind("cut").unwrap();
    click(&mut f.s, centre(&f.l, 10.0, 10.0));
    assert!(f.s.set_selection_in_operation(cut, true));
    assert_eq!(count(&f.s, "running"), 1);
    assert_eq!(count(&f.s, "not_assigned"), 2);

    assert!(f.s.delete_operation(cut));
    assert!(rows(&f.s).is_empty());
    assert_eq!(
        count(&f.s, "not_assigned"),
        3,
        "the item becomes Not assigned"
    );
    assert_eq!(count(&f.s, "running"), 0);

    f.s.undo();
    assert_eq!(names(&f.s), vec!["Cut 1"]);
    assert_eq!(row_items(&f.s, "Cut 1"), 1, "undo restores the membership");
    assert_eq!(count(&f.s, "running"), 1);
}

// ---- criterion 5 and 6: Cuts last --------------------------------------------------------------

#[test]
fn c5_cuts_last_on_refuses_a_cut_dragged_above_the_section_line() {
    let mut f = fixture();
    let cut = f.s.press_operation_kind("cut").unwrap();
    f.s.press_operation_kind("score").unwrap();
    assert_eq!(names(&f.s), vec!["Score 1", "Cut 1"]);
    let before = f.s.undo_len();
    assert!(
        !f.s.move_operation(cut, 0),
        "a Cut above Marking is refused"
    );
    assert_eq!(names(&f.s), vec!["Score 1", "Cut 1"], "the row returns");
    assert_eq!(f.s.undo_len(), before, "a refused move writes no step");
}

#[test]
fn c6_cuts_last_off_moves_nothing_then_on_moves_cutting_below_marking() {
    let mut f = fixture();
    let cut = f.s.press_operation_kind("cut").unwrap();
    f.s.press_operation_kind("engrave").unwrap();
    assert_eq!(names(&f.s), vec!["Engrave 1", "Cut 1"]);

    let before = f.s.undo_len();
    assert!(f.s.set_cuts_last(false));
    assert_eq!(
        names(&f.s),
        vec!["Engrave 1", "Cut 1"],
        "switching off moves nothing"
    );
    assert_eq!(f.s.undo_len(), before + 1, "one undo step Cuts last off");

    assert!(
        f.s.move_operation(cut, 0),
        "with Cuts last off any place is allowed"
    );
    assert_eq!(names(&f.s), vec!["Cut 1", "Engrave 1"]);

    assert!(f.s.set_cuts_last(true));
    assert_eq!(
        names(&f.s),
        vec!["Engrave 1", "Cut 1"],
        "switching on moves cutting below marking, each group in its order"
    );
}

#[test]
fn c6_cuts_last_off_new_operation_lands_at_the_end_of_the_list() {
    let mut f = fixture();
    f.s.press_operation_kind("cut").unwrap();
    f.s.set_cuts_last(false);
    f.s.press_operation_kind("score").unwrap();
    assert_eq!(names(&f.s), vec!["Cut 1", "Score 1"]);
}

// ---- criterion 8 and 9: assignment -------------------------------------------------------------

#[test]
fn c8_kind_button_with_a_selection_creates_and_assigns_in_one_step() {
    let mut f = fixture();
    click(&mut f.s, centre(&f.l, 10.0, 10.0));
    let before = f.s.undo_len();
    let id = f.s.press_operation_kind("engrave");
    assert!(id.is_some());
    assert_eq!(
        f.s.undo_len(),
        before + 1,
        "one undo step Assign to Engrave 1"
    );
    assert_eq!(names(&f.s), vec!["Engrave 1"]);
    assert_eq!(row_items(&f.s, "Engrave 1"), 1);
    assert_eq!(count(&f.s, "running"), 1);
    assert_eq!(count(&f.s, "not_assigned"), 2);
}

#[test]
fn c8_ticking_an_operation_adds_the_selection_and_unticking_removes_it() {
    let mut f = fixture();
    let cut = f.s.press_operation_kind("cut").unwrap();
    click(&mut f.s, centre(&f.l, 10.0, 10.0));
    shift_click(&mut f.s, centre(&f.l, 60.0, 10.0));
    assert!(f.s.set_selection_in_operation(cut, true));
    assert_eq!(row_items(&f.s, "Cut 1"), 2);
    assert_eq!(count(&f.s, "running"), 2);
    assert_eq!(count(&f.s, "not_assigned"), 1);

    assert!(f.s.set_selection_in_operation(cut, false));
    assert_eq!(row_items(&f.s, "Cut 1"), 0);
    assert_eq!(count(&f.s, "running"), 0);
}

#[test]
fn c9_removing_one_membership_keeps_the_other_operations() {
    let mut f = fixture();
    let cut = f.s.press_operation_kind("cut").unwrap();
    let engrave = f.s.press_operation_kind("engrave").unwrap();
    click(&mut f.s, centre(&f.l, 10.0, 10.0));
    assert!(f.s.set_selection_in_operation(cut, true));
    assert!(f.s.set_selection_in_operation(engrave, true));
    assert_eq!(count(&f.s, "operations"), 2);

    click(&mut f.s, centre(&f.l, 10.0, 10.0));
    assert!(f.s.set_selection_in_operation(cut, false));
    assert_eq!(row_items(&f.s, "Cut 1"), 0);
    assert_eq!(
        row_items(&f.s, "Engrave 1"),
        1,
        "the other membership stays"
    );
    assert_eq!(count(&f.s, "running"), 1);
    assert_eq!(count(&f.s, "operations"), 1);
}

#[test]
fn c8_number_key_toggles_membership_in_the_nth_operation() {
    let mut f = fixture();
    f.s.press_operation_kind("cut").unwrap();
    f.s.press_operation_kind("engrave").unwrap();
    click(&mut f.s, centre(&f.l, 10.0, 10.0));
    assert_eq!(
        names(&f.s),
        vec!["Engrave 1", "Cut 1"],
        "Cuts last puts Cut 1 second"
    );
    assert!(f.s.toggle_selection_by_number(2));
    assert_eq!(
        row_items(&f.s, "Cut 1"),
        1,
        "number 2 toggles operation 2 of the list"
    );
    assert_eq!(row_items(&f.s, "Engrave 1"), 0);
    assert!(f.s.toggle_selection_by_number(2));
    assert_eq!(count(&f.s, "running"), 0, "a second press removes it");
}

// ---- criterion 10: guides ----------------------------------------------------------------------

#[test]
fn c10_guides_get_no_operation_and_the_notice_names_them() {
    let d = Document::new(1);
    let made = d
        .add_output_page(NewOutputPage {
            process: LASER,
            label: "Laser",
            profile: None,
            work_area: DocumentSize::from_mm(400.0, 300.0),
        })
        .unwrap();
    let l = d.pages().into_iter().find(|p| p.id == made.id).unwrap();
    let guide = rect(&d, &l, 10.0, 10.0);
    let item = rect(&d, &l, 60.0, 10.0);
    d.set_guide(&[guide], true).unwrap();
    let mut s = Session::open(7, &pack(&d, "0101-guide").unwrap()).unwrap();
    s.resize_viewport(1600.0, 1000.0);
    s.set_tool(Tool::Select);
    s.activate_page(&l.id);
    s.wipe_history();

    click(&mut s, centre(&l, 10.0, 10.0));
    shift_click(&mut s, centre(&l, 60.0, 10.0));
    let id = s.press_operation_kind("cut").unwrap();
    assert_eq!(row_items(&s, "Cut 1"), 1, "only the item is assigned");
    assert_eq!(count(&s, "running"), 1);
    let notice = s.operations_notice().unwrap_or_default();
    assert!(notice.contains("1 guide"), "notice was {notice:?}");
    assert!(
        notice.contains("Guides never run."),
        "notice was {notice:?}"
    );
    assert!(notice.chars().count() <= 70, "at most 70 characters");
    let _ = (id, item);
}

#[test]
fn c10_a_selection_of_only_guides_adds_nothing_and_writes_no_step() {
    let d = Document::new(1);
    let made = d
        .add_output_page(NewOutputPage {
            process: LASER,
            label: "Laser",
            profile: None,
            work_area: DocumentSize::from_mm(400.0, 300.0),
        })
        .unwrap();
    let l = d.pages().into_iter().find(|p| p.id == made.id).unwrap();
    let guide = rect(&d, &l, 10.0, 10.0);
    d.set_guide(&[guide], true).unwrap();
    let mut s = Session::open(7, &pack(&d, "0101-guide-only").unwrap()).unwrap();
    s.resize_viewport(1600.0, 1000.0);
    s.set_tool(Tool::Select);
    s.activate_page(&l.id);
    s.wipe_history();
    click(&mut s, centre(&l, 10.0, 10.0));
    let before = s.undo_len();
    let _ = s.press_operation_kind("cut");
    assert_eq!(count(&s, "running"), 0);
    assert_eq!(s.undo_len(), before, "no step for guides only");
    let notice = s.operations_notice().unwrap_or_default();
    assert!(
        notice.contains("Guides never run."),
        "notice was {notice:?}"
    );
}

// ---- criterion 14: copies inherit --------------------------------------------------------------

/// Criterion 14 and ADR 0046 §4: a linked clone is a new target and starts with no keys. The
/// session has no public Duplicate, Ctrl copy or Paste entry point to drive here, so copy
/// inheritance is reported as a missing seam, not tested.
#[test]
fn c14_a_linked_clone_starts_not_assigned_and_leaves_the_original_running() {
    let mut f = fixture();
    f.s.press_operation_kind("engrave").unwrap();
    click(&mut f.s, centre(&f.l, 10.0, 10.0));
    assert!(f.s.set_selection_in_operation(f.s.operations_view().unwrap().rows[0].id, true));
    assert_eq!(count(&f.s, "running"), 1);
    let _ = f.s.linked_clone();
    assert_eq!(
        count(&f.s, "running"),
        1,
        "the clone does not inherit a membership on its own"
    );
    assert_eq!(
        count(&f.s, "not_assigned"),
        3,
        "original, the clone and two rectangles"
    );
    assert_eq!(row_items(&f.s, "Engrave 1"), 1);
}

// ---- criterion 15 and 16: Show and Output ------------------------------------------------------

#[test]
fn c15_show_is_view_only_and_output_off_stops_what_runs() {
    let mut f = fixture();
    let cut = f.s.press_operation_kind("cut").unwrap();
    click(&mut f.s, centre(&f.l, 10.0, 10.0));
    assert!(f.s.set_selection_in_operation(cut, true));
    assert_eq!(count(&f.s, "running"), 1);

    assert!(f.s.set_operation_switch(cut, OperationSwitch::Show, false));
    assert_eq!(rows(&f.s)[0]["show"], false);
    assert_eq!(count(&f.s, "running"), 1, "Show never changes what runs");

    assert!(f.s.set_operation_switch(cut, OperationSwitch::Output, false));
    assert_eq!(count(&f.s, "running"), 0, "Output off runs nothing");
    assert_eq!(
        count(&f.s, "output_off"),
        1,
        "the item sits in an Output-off operation"
    );
    assert_eq!(count(&f.s, "operations"), 0);
    assert_eq!(rows(&f.s)[0]["output"], false);
}

#[test]
fn c16_output_on_with_show_off_runs() {
    let mut f = fixture();
    let cut = f.s.press_operation_kind("cut").unwrap();
    click(&mut f.s, centre(&f.l, 10.0, 10.0));
    f.s.set_selection_in_operation(cut, true);
    assert!(f.s.set_operation_switch(cut, OperationSwitch::Show, false));
    f.s.set_operation_switch(cut, OperationSwitch::Output, true);
    assert_eq!(count(&f.s, "running"), 1);
    assert_eq!(count(&f.s, "output_off"), 0);
}

// ---- criterion 17: summary ---------------------------------------------------------------------

#[test]
fn c17_summary_counts_are_disjoint_and_cover_every_leaf() {
    let mut f = fixture();
    let cut = f.s.press_operation_kind("cut").unwrap();
    click(&mut f.s, centre(&f.l, 10.0, 10.0));
    f.s.set_selection_in_operation(cut, true);
    let running = count(&f.s, "running");
    let not_assigned = count(&f.s, "not_assigned");
    assert_eq!(
        running + not_assigned,
        3,
        "each of the three leaves is in exactly one"
    );
    assert_eq!(count(&f.s, "runs_while_hidden"), 0);
    assert_eq!(count(&f.s, "operations"), 1);
    let items = view(&f.s)["items"]["not_assigned"]
        .as_array()
        .unwrap()
        .len();
    assert_eq!(
        items as u64, not_assigned,
        "the count buttons select what they count"
    );
}

#[test]
fn c17_the_count_button_selects_the_items_it_counts() {
    let mut f = fixture();
    let cut = f.s.press_operation_kind("cut").unwrap();
    click(&mut f.s, centre(&f.l, 10.0, 10.0));
    f.s.set_selection_in_operation(cut, true);
    key(&mut f.s, "Escape", false, false);
    let ok = f.s.select_operation_items("not_assigned");
    assert!(ok);
    assert_eq!(f.s.selected_object_count(), 2);
    assert!(!f.s.select_operation_items("unknown_count"));
}

// ---- criterion 18: the view switch -------------------------------------------------------------

#[test]
fn c18_view_defaults_to_operations_and_drawn_is_session_state() {
    let mut f = fixture();
    assert_eq!(view(&f.s)["view_mode"], "operations");
    assert!(f.s.set_operations_view_mode(curvyo_editor_wasm::OperationsViewMode::Drawn));
    assert_eq!(view(&f.s)["view_mode"], "drawn");
    let before = f.s.undo_len();
    assert_eq!(f.s.undo_len(), before, "the view switch is not a step");
}

// ---- criterion 27: silent drops on delete ------------------------------------------------------

#[test]
fn c27_deleting_an_assigned_object_drops_the_assignment_silently() {
    let mut f = fixture();
    let cut = f.s.press_operation_kind("cut").unwrap();
    click(&mut f.s, centre(&f.l, 10.0, 10.0));
    f.s.set_selection_in_operation(cut, true);
    assert_eq!(count(&f.s, "running"), 1);

    click(&mut f.s, centre(&f.l, 10.0, 10.0));
    key(&mut f.s, "Delete", false, false);
    assert_eq!(count(&f.s, "running"), 0);
    assert_eq!(count(&f.s, "not_assigned"), 2);
    assert_eq!(row_items(&f.s, "Cut 1"), 0);
    let text = f.s.operations_view_json().to_lowercase();
    assert!(!text.contains("missing"), "no Missing item row or count");
    assert!(f.s.operations_notice().is_none(), "no notice");

    f.s.undo();
    assert_eq!(
        count(&f.s, "running"),
        1,
        "undo brings the object and its assignment back"
    );
    assert_eq!(row_items(&f.s, "Cut 1"), 1);
}

#[test]
fn c27_deleting_a_group_drops_the_assignments_of_its_members_and_undo_restores() {
    let mut f = fixture();
    let cut = f.s.press_operation_kind("cut").unwrap();
    click(&mut f.s, centre(&f.l, 10.0, 10.0));
    shift_click(&mut f.s, centre(&f.l, 60.0, 10.0));
    let _ = f.s.group_selection();
    click(&mut f.s, centre(&f.l, 10.0, 10.0));
    f.s.set_selection_in_operation(cut, true);
    assert_eq!(count(&f.s, "running"), 2, "the group's two members run");

    key(&mut f.s, "Delete", false, false);
    assert_eq!(count(&f.s, "running"), 0);
    assert!(f.s.operations_notice().is_none());

    f.s.undo();
    assert_eq!(
        count(&f.s, "running"),
        2,
        "undo restores the group and its assignment"
    );
}

#[test]
fn c27_undo_of_a_delete_restores_nothing_it_did_not_remove() {
    let mut f = fixture();
    let cut = f.s.press_operation_kind("cut").unwrap();
    click(&mut f.s, centre(&f.l, 60.0, 10.0));
    f.s.set_selection_in_operation(cut, true);
    click(&mut f.s, centre(&f.l, 10.0, 10.0));
    key(&mut f.s, "Delete", false, false);
    assert_eq!(count(&f.s, "running"), 1, "the untouched r2 still runs");
}

// ---- a few session-level checks of the opened golden (criterion 25) -----------------------------

#[test]
fn c25_the_five_kinds_golden_opens_with_its_list_intact() {
    let bytes = std::fs::read(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../curvyo-document-core/tests/fixtures/operations_v39.curvyo"),
    )
    .unwrap();
    let mut s = Session::open(7, &bytes).unwrap();
    s.resize_viewport(1600.0, 1000.0);
    let laser: Value = serde_json::from_str(&s.pages_json()).unwrap();
    let id = laser
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["kind"] == LASER)
        .map(|p| p["id"].as_str().unwrap().to_owned())
        .unwrap();
    s.activate_page(&id);
    let kinds: Vec<String> = rows(&s)
        .iter()
        .map(|r| r["kind"].as_str().unwrap().to_owned())
        .collect();
    for kind in ["cut", "score", "engrave", "engrave_outline", "cut_out"] {
        assert!(
            kinds.iter().any(|k| k == kind),
            "golden lacks {kind}: {kinds:?}"
        );
    }
    assert!(count(&s, "not_assigned") <= 100);
}

// ---- criteria 25 and 26 on the session: the unknown golden and the view contract ----------------

fn laser_page_of(s: &Session) -> String {
    let pages: Value = serde_json::from_str(&s.pages_json()).unwrap();
    pages
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["kind"] == LASER)
        .map(|p| p["id"].as_str().unwrap().to_owned())
        .unwrap()
}

#[test]
fn c26_the_unknown_kind_golden_shows_unknown_operation_and_counts_it_nowhere() {
    let bytes = std::fs::read(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../curvyo-document-core/tests/fixtures/operations_unknown_kind_v39.curvyo"),
    )
    .unwrap();
    let mut s = Session::open(7, &bytes).unwrap();
    s.resize_viewport(1600.0, 1000.0);
    let page = laser_page_of(&s);
    s.activate_page(&page);
    let unknown: Vec<Value> = rows(&s)
        .into_iter()
        .filter(|r| r["kind"] == "unknown")
        .collect();
    assert!(
        !unknown.is_empty(),
        "the unknown operation is listed: {}",
        s.operations_view_json()
    );
    assert!(unknown.iter().all(|r| r["name"].as_str().is_some()));
}

/// The live view JSON has the same top-level keys and row keys as the shared fixtures, so the
/// TypeScript decoder reads what Rust writes (`adrs.md` decision 6).
#[test]
fn c17_the_live_view_json_has_the_fixture_keys() {
    let fixture_text = std::fs::read_to_string(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/operations_view/five_kinds_cuts_last.json"),
    )
    .unwrap();
    let fixture: Value = serde_json::from_str(&fixture_text).unwrap();
    let mut s = fixture_session();
    s.press_operation_kind("cut").unwrap();
    let live = view(&s);
    let keys = |v: &Value| -> Vec<String> {
        let mut k: Vec<String> = v.as_object().unwrap().keys().cloned().collect();
        k.sort();
        k
    };
    assert_eq!(keys(&live), keys(&fixture), "top-level keys");
    assert_eq!(
        keys(&live["rows"][0]),
        keys(&fixture["rows"][0]),
        "row keys"
    );
    assert_eq!(
        keys(&live["summary"]),
        keys(&fixture["summary"]),
        "summary keys"
    );
    assert_eq!(keys(&live["items"]), keys(&fixture["items"]), "item lists");
}

fn fixture_session() -> Session {
    let d = Document::new(1);
    let made = d
        .add_output_page(NewOutputPage {
            process: LASER,
            label: "Laser",
            profile: None,
            work_area: DocumentSize::from_mm(400.0, 300.0),
        })
        .unwrap();
    let mut s = Session::open(7, &pack(&d, "0101-keys").unwrap()).unwrap();
    s.resize_viewport(1600.0, 1000.0);
    s.activate_page(&made.id);
    s
}

// ---- criterion 27 (ungroup) and criterion 28 (the laser page deleted) --------------------------

/// The membership paths of the operation `name` in a saved copy of the session.
fn saved_members(s: &Session, name: &str) -> Vec<Vec<NodeId>> {
    let saved = unpack(9, &s.pack("0.1.0").unwrap()).unwrap();
    saved
        .manufacturings()
        .iter()
        .flat_map(|m| saved.operations(m.id).unwrap())
        .find(|op| op.name == name)
        .map(|op| op.members)
        .unwrap()
}

/// Criteria 11 and 12: a group on the output page is an item that holds its own assignment (the
/// example "group G is in a Cut-out operation"). A group assigned as a whole is stored as the
/// group's node, not as each of its leaves.
#[test]
fn c12_a_selected_group_is_assigned_as_the_group_node() {
    let mut f = fixture();
    let cut = f.s.press_operation_kind("cut").unwrap();
    click(&mut f.s, centre(&f.l, 10.0, 10.0));
    shift_click(&mut f.s, centre(&f.l, 60.0, 10.0));
    let _ = f.s.group_selection();
    click(&mut f.s, centre(&f.l, 10.0, 10.0));
    f.s.set_selection_in_operation(cut, true);
    let saved = unpack(9, &f.s.pack("0.1.0").unwrap()).unwrap();
    let leaf_chain = saved.node_chain(f.r1);
    assert!(
        leaf_chain.len() >= 2,
        "the leaf sits in a group: {leaf_chain:?}"
    );
    let group = leaf_chain[1];
    let members = saved_members(&f.s, "Cut 1");
    assert_eq!(
        members,
        vec![vec![group]],
        "the group node holds the assignment, not its two leaves"
    );
}

/// Criterion 27 with ungroup as a delete path: the group's assignment goes with the group,
/// silently, and one undo brings it back.
#[test]
fn c27_ungrouping_drops_the_group_assignment_silently_and_undo_restores_it() {
    let mut f = fixture();
    let cut = f.s.press_operation_kind("cut").unwrap();
    click(&mut f.s, centre(&f.l, 10.0, 10.0));
    shift_click(&mut f.s, centre(&f.l, 60.0, 10.0));
    let _ = f.s.group_selection();
    click(&mut f.s, centre(&f.l, 10.0, 10.0));
    f.s.set_selection_in_operation(cut, true);
    assert_eq!(count(&f.s, "running"), 2);

    let result = f.s.ungroup_selection();
    assert_eq!(
        result.kind,
        curvyo_editor_wasm::GroupingKind::Ungrouped,
        "the group was ungrouped"
    );
    assert_eq!(
        count(&f.s, "running"),
        0,
        "the group's assignment is dropped with the group; the leaves are Not assigned"
    );
    assert!(f.s.operations_notice().is_none(), "no notice");
    assert_eq!(row_items(&f.s, "Cut 1"), 0);

    f.s.undo();
    assert_eq!(
        count(&f.s, "running"),
        2,
        "undo of the ungroup restores the group and its assignment"
    );
}

#[test]
fn c28_deleting_the_laser_page_deletes_its_operations_and_one_undo_restores_them() {
    let mut f = fixture();
    let cut = f.s.press_operation_kind("cut").unwrap();
    click(&mut f.s, centre(&f.l, 10.0, 10.0));
    f.s.set_selection_in_operation(cut, true);
    assert_eq!(count(&f.s, "running"), 1);

    let page = f.l.id.clone();
    assert!(f.s.delete_page(&page), "the laser page is deleted");
    let pages: Value = serde_json::from_str(&f.s.pages_json()).unwrap();
    assert!(
        !pages.as_array().unwrap().iter().any(|p| p["id"] == page),
        "the page is gone"
    );

    f.s.undo();
    let pages: Value = serde_json::from_str(&f.s.pages_json()).unwrap();
    assert!(
        pages.as_array().unwrap().iter().any(|p| p["id"] == page),
        "one undo brings the laser page back"
    );
    f.s.activate_page(&page);
    assert_eq!(
        names(&f.s),
        vec!["Cut 1"],
        "its operations come back with it"
    );
    assert_eq!(count(&f.s, "running"), 1, "and its memberships");
}
