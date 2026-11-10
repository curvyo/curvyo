//! Tester acceptance cases for `specs/0101-laser-operations` after the customer's design of
//! 2026-11-10: no operation colour anywhere in the view, the look of each kind (criterion 18,
//! two colours), Show as a canvas switch (15), rename and reorder rules at the section line
//! (3, 5), and the 500 chip cap of the canvas symbols (`ux-notes.md` 13.4). Written from the
//! specification and the UX notes; the colour seam is the draw list's vertex colours.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::too_many_lines,
    clippy::similar_names,
    clippy::many_single_char_names,
    clippy::needless_pass_by_value,
    missing_docs
)]

use curvyo_document_core::{
    Document, DocumentSize, NewOutputPage, NodeId, OperationSwitch, Page, Point, RectBounds,
    ViewTransform, pack,
};
use curvyo_editor_wasm::{OperationsViewMode, Session, Tool};
use curvyo_render_core::{CHIP_CAP, ChipItem, Glyph, chips_list};
use serde_json::Value;

const LASER: &str = "curvyo.laser";
/// The Cut red (`--cut`, ux-notes 13.1).
const CUT_RED: (u8, u8, u8) = (0xC8, 0x10, 0x2E);
/// The pale orange hatch (`--engrave-hatch`, ux-notes 13.1).
const HATCH: (u8, u8, u8) = (0xC4, 0x6A, 0x1C);

struct Fx {
    s: Session,
    l: Page,
}

fn rect(d: &Document, page: &Page, x: f64, y: f64) -> NodeId {
    d.create_rect(RectBounds::from_corners(
        Point::new(page.origin.x + x, page.origin.y + y),
        Point::new(page.origin.x + x + 20.0, page.origin.y + y + 10.0),
    ))
}

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
    rect(&d, &l, 10.0, 10.0);
    rect(&d, &l, 60.0, 10.0);
    rect(&d, &l, 10.0, 100.0);
    let mut s = Session::open(7, &pack(&d, "0101-tester").unwrap()).unwrap();
    s.resize_viewport(1600.0, 1000.0);
    s.set_tool(Tool::Select);
    s.activate_page(&l.id);
    s.wipe_history();
    Fx { s, l }
}

fn centre(page: &Page, x: f64, y: f64) -> Point {
    Point::new(page.origin.x + x + 10.0, page.origin.y + y)
}

fn click(s: &mut Session, at: Point) {
    s.pointer_hover(at, false, false);
    s.pointer_down(at, false);
    s.pointer_up(at, false, false);
}

fn view(s: &Session) -> Value {
    serde_json::from_str(&s.operations_view_json()).unwrap()
}

fn names(s: &Session) -> Vec<String> {
    view(s)["rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["name"].as_str().unwrap().to_owned())
        .collect()
}

/// Every key of a JSON value, at any depth.
fn keys_deep(v: &Value, out: &mut Vec<String>) {
    match v {
        Value::Array(items) => items.iter().for_each(|i| keys_deep(i, out)),
        Value::Object(map) => {
            for (k, child) in map {
                out.push(k.clone());
                keys_deep(child, out);
            }
        }
        _ => {}
    }
}

/// The colours of every vertex of the frame (the draw list the canvas paints).
fn colours(s: &Session) -> Vec<(u8, u8, u8)> {
    s.draw_list()
        .triangles
        .iter()
        .map(|v| (v.color.r, v.color.g, v.color.b))
        .collect()
}

fn has(colours: &[(u8, u8, u8)], want: (u8, u8, u8)) -> bool {
    colours.contains(&want)
}

// ---- criteria 2, 18: no colour in the view or the list ----------------------------------------

#[test]
fn c2_c18_the_view_json_has_no_colour_or_machine_key_after_all_five_kinds() {
    let mut f = fixture();
    let ids: Vec<NodeId> = ["cut", "score", "engrave", "engrave_outline", "cut_out"]
        .iter()
        .map(|k| f.s.press_operation_kind(k).unwrap())
        .collect();
    click(&mut f.s, centre(&f.l, 10.0, 10.0));
    f.s.set_selection_in_operation(ids[0], true);
    let mut keys = Vec::new();
    keys_deep(&view(&f.s), &mut keys);
    let bad: Vec<&String> = keys
        .iter()
        .filter(|k| {
            let k = k.to_ascii_lowercase();
            k.contains("colour")
                || k.contains("color")
                || k.contains("swatch")
                || k.contains("rgb")
                || k.contains("hex")
                || k == "power"
                || k == "speed"
                || k == "passes"
        })
        .collect();
    assert!(bad.is_empty(), "keys that must not exist: {bad:?}");
}

#[test]
fn c2_no_row_carries_a_colour_even_after_rename_and_move() {
    let mut f = fixture();
    let cut = f.s.press_operation_kind("cut").unwrap();
    assert!(f.s.rename_operation(cut, "Outer"));
    f.s.press_operation_kind("engrave").unwrap();
    for row in view(&f.s)["rows"].as_array().unwrap() {
        let obj = row.as_object().unwrap();
        assert!(
            !obj.keys().any(|k| k.contains("colo")),
            "row keys {:?}",
            obj.keys().collect::<Vec<_>>()
        );
    }
}

// ---- criterion 18: the look, two colours ------------------------------------------------------

#[test]
fn c18_cut_draws_in_cut_red_and_the_drawing_is_not_red_under_view_drawn() {
    let mut f = fixture();
    let before = colours(&f.s);
    assert!(
        !has(&before, CUT_RED),
        "a plain drawing has no operation red"
    );
    let cut = f.s.press_operation_kind("cut").unwrap();
    click(&mut f.s, centre(&f.l, 10.0, 10.0));
    f.s.set_selection_in_operation(cut, true);
    assert!(
        has(&colours(&f.s), CUT_RED),
        "a Cut item is drawn in the Cut red under View: Operations"
    );
    assert!(f.s.set_operations_view_mode(OperationsViewMode::Drawn));
    assert!(
        !has(&colours(&f.s), CUT_RED),
        "View: Drawn shows the drawing's own style"
    );
}

#[test]
fn c18_engrave_is_hatched_in_pale_orange_and_is_not_cut_red() {
    let mut f = fixture();
    let eng = f.s.press_operation_kind("engrave").unwrap();
    click(&mut f.s, centre(&f.l, 60.0, 10.0));
    f.s.set_selection_in_operation(eng, true);
    let c = colours(&f.s);
    assert!(has(&c, HATCH), "Engrave is drawn in the pale orange hatch");
    assert!(
        !has(&c, CUT_RED),
        "an Engrave item with no Cut carries no Cut red"
    );
}

#[test]
fn c18_score_is_ink_and_never_red() {
    let mut f = fixture();
    let sc = f.s.press_operation_kind("score").unwrap();
    click(&mut f.s, centre(&f.l, 10.0, 10.0));
    f.s.set_selection_in_operation(sc, true);
    let c = colours(&f.s);
    assert!(
        !has(&c, CUT_RED),
        "Score is not drawn in cut red (red is for cut lines only)"
    );
    assert!(!has(&c, HATCH), "Score has no hatch");
}

#[test]
fn c18_not_assigned_items_carry_no_operation_colour() {
    let f = fixture();
    let c = colours(&f.s);
    assert!(!has(&c, CUT_RED));
    assert!(!has(&c, HATCH), "a Not assigned item has no hatch");
}

#[test]
fn c15_show_off_takes_the_item_off_the_canvas_and_output_on_still_runs() {
    let mut f = fixture();
    let cut = f.s.press_operation_kind("cut").unwrap();
    click(&mut f.s, centre(&f.l, 10.0, 10.0));
    f.s.set_selection_in_operation(cut, true);
    assert!(
        has(&colours(&f.s), CUT_RED),
        "shown: the Cut red is on the canvas"
    );
    assert!(f.s.set_operation_switch(cut, OperationSwitch::Show, false));
    assert!(
        !has(&colours(&f.s), CUT_RED),
        "Show off: the item is not drawn"
    );
    let running = view(&f.s)["summary"]["running"].as_u64().unwrap();
    assert_eq!(running, 1, "Show never changes what runs");
}

// ---- criterion 3: rename in place is one step, names are checked -----------------------------

#[test]
fn c3_a_name_of_129_characters_is_not_stored_longer_than_128() {
    let mut f = fixture();
    let id = f.s.press_operation_kind("cut").unwrap();
    let long: String = "x".repeat(129);
    let _ = f.s.rename_operation(id, &long);
    let stored = names(&f.s).pop().unwrap();
    assert!(
        stored.chars().count() <= 128,
        "the name is capped at 128 characters (ux-notes 4.4); stored {} characters",
        stored.chars().count()
    );
}

#[test]
fn c3_a_blank_name_reverts_to_the_old_name() {
    let mut f = fixture();
    let id = f.s.press_operation_kind("cut").unwrap();
    let before = names(&f.s);
    let _ = f.s.rename_operation(id, "   ");
    assert_eq!(
        names(&f.s),
        before,
        "a blank name reverts (Proposal: whitespace is empty)"
    );
}

#[test]
fn c3_rename_is_one_undo_step_and_undo_brings_the_old_name_back() {
    let mut f = fixture();
    let id = f.s.press_operation_kind("cut").unwrap();
    let before = f.s.undo_len();
    assert!(f.s.rename_operation(id, "Frame"));
    assert_eq!(f.s.undo_len(), before + 1);
    f.s.undo();
    assert_eq!(names(&f.s), vec!["Cut 1"]);
}

// ---- criterion 4: delete is one undo step with its memberships ------------------------------

#[test]
fn c4_delete_is_one_undo_step_and_the_items_become_not_assigned() {
    let mut f = fixture();
    let cut = f.s.press_operation_kind("cut").unwrap();
    click(&mut f.s, centre(&f.l, 10.0, 10.0));
    f.s.set_selection_in_operation(cut, true);
    let before = f.s.undo_len();
    assert!(f.s.delete_operation(cut));
    assert_eq!(f.s.undo_len(), before + 1, "one step");
    assert_eq!(names(&f.s).len(), 0);
    assert_eq!(view(&f.s)["summary"]["not_assigned"].as_u64().unwrap(), 3);
    f.s.undo();
    assert_eq!(names(&f.s), vec!["Cut 1"], "undo restores the operation");
}

// ---- criterion 5, 6: reorder inside a section; the section line is never crossed -------------

#[test]
fn c5_a_move_inside_cutting_is_one_step_and_undoes() {
    let mut f = fixture();
    f.s.press_operation_kind("score").unwrap();
    f.s.press_operation_kind("cut").unwrap();
    let cut2 = f.s.press_operation_kind("cut").unwrap();
    assert_eq!(names(&f.s), vec!["Score 1", "Cut 1", "Cut 2"]);
    let before = f.s.undo_len();
    assert!(
        f.s.move_operation(cut2, 1),
        "Cut 2 moves above Cut 1 inside Cutting"
    );
    assert_eq!(f.s.undo_len(), before + 1, "one step");
    assert_eq!(names(&f.s), vec!["Score 1", "Cut 2", "Cut 1"]);
    f.s.undo();
    assert_eq!(names(&f.s), vec!["Score 1", "Cut 1", "Cut 2"]);
}

#[test]
fn c5_a_score_dragged_below_the_section_line_into_cutting_is_refused() {
    let mut f = fixture();
    let sc = f.s.press_operation_kind("score").unwrap();
    f.s.press_operation_kind("cut").unwrap();
    assert_eq!(names(&f.s), vec!["Score 1", "Cut 1"]);
    let before = f.s.undo_len();
    assert!(
        !f.s.move_operation(sc, 1),
        "Score below the Cut would cross the section line from Marking"
    );
    assert_eq!(names(&f.s), vec!["Score 1", "Cut 1"]);
    assert_eq!(f.s.undo_len(), before);
}

#[test]
fn c6_cuts_last_off_lets_a_cut_go_above_marking_and_back_on_moves_it_down() {
    let mut f = fixture();
    let cut = f.s.press_operation_kind("cut").unwrap();
    f.s.press_operation_kind("engrave").unwrap();
    assert!(f.s.set_cuts_last(false));
    assert!(
        f.s.move_operation(cut, 0),
        "with Cuts last off any place is allowed"
    );
    assert_eq!(names(&f.s), vec!["Cut 1", "Engrave 1"]);
    assert!(f.s.set_cuts_last(true));
    assert_eq!(
        names(&f.s),
        vec!["Engrave 1", "Cut 1"],
        "Cuts last on moves cutting below marking"
    );
}

// ---- criterion 17 and ux 13.4: the 500 chip cap ----------------------------------------------

#[test]
fn c18_the_canvas_draws_at_most_500_symbol_chips_per_frame() {
    assert_eq!(CHIP_CAP, 500);
    let grid = |n: usize| -> Vec<ChipItem> {
        (0..n)
            .map(|i| {
                let x = f64::from(u32::try_from(i % 30).unwrap()) * 30.0;
                let y = f64::from(u32::try_from(i / 30).unwrap()) * 30.0;
                ChipItem {
                    bounds: (Point::new(x, y), Point::new(x + 10.0, y + 10.0)),
                    glyphs: vec![Glyph::Cut],
                }
            })
            .collect()
    };
    let canvas = (2000.0, 2000.0);
    let view = ViewTransform::identity();
    let at_499 = chips_list(&grid(499), canvas, view).triangle_count();
    let at_500 = chips_list(&grid(500), canvas, view).triangle_count();
    let at_600 = chips_list(&grid(600), canvas, view).triangle_count();
    assert!(
        at_499 < at_500,
        "the 500th chip is drawn ({at_499} < {at_500} triangles)"
    );
    assert_eq!(at_600, at_500, "chips past the cap are not drawn");
}
