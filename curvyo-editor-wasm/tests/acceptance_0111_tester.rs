//! Tester acceptance cases for `specs/0111-laser-operation-tools`, written from the specification
//! before the implementation was read: the kind buttons of the Select bar and their reuse rule
//! (criteria 1 to 8), group assignment and the group tick (9, 10), the read-only entered group
//! and its refusals (11 to 13), clones as a whole and old clone-member keys (14, Frage 212 A) and
//! the Operations view toggle at the page label (15). Black box through the public session.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::too_many_lines,
    clippy::similar_names,
    clippy::many_single_char_names,
    clippy::needless_pass_by_value,
    clippy::assert_is_empty,
    clippy::format_push_string,
    clippy::cast_possible_truncation,
    missing_docs
)]

use std::path::PathBuf;

use curvyo_document_core::{
    Direction, Document, DocumentSize, NewOperation, NewOutputPage, NodeId, Page, PageKind, Point,
    RectBounds, pack, unpack,
};
use curvyo_editor_wasm::{KeyInput, Session, Tool};
use curvyo_ui_core::FontBlock;
use serde_json::Value;

const LASER: &str = "curvyo.laser";
const KINDS: [&str; 5] = ["cut", "score", "engrave", "engrave_outline", "cut_out"];
/// The Cut red (`--cut`, `0101` ux-notes 13.1).
const CUT_RED: (u8, u8, u8) = (0xC8, 0x10, 0x2E);

// ---- fixture ----------------------------------------------------------------------------------

struct Fx {
    s: Session,
    l: Page,
    /// A drawing page with one rectangle (`dr`).
    dp: Page,
    dr: NodeId,
    m: NodeId,
}

fn rect_at(d: &Document, page: &Page, x: f64, y: f64) -> NodeId {
    d.create_rect(RectBounds::from_corners(
        Point::new(page.origin.x + x, page.origin.y + y),
        Point::new(page.origin.x + x + 20.0, page.origin.y + y + 10.0),
    ))
}

fn laser_doc() -> (Document, Page, Page, NodeId) {
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
    let dp = d.pages().into_iter().find(|p| p.id == "first").unwrap();
    let dr = rect_at(&d, &dp, 10.0, 10.0);
    (d, l, dp, dr)
}

fn open(d: &Document, l: &Page, dp: Page, dr: NodeId) -> Fx {
    let mut s = Session::open(7, &pack(d, "0111-tester").unwrap()).unwrap();
    s.resize_viewport(1600.0, 1000.0);
    let mut snap = s.snapping();
    snap.enabled = false;
    s.set_snapping(snap);
    s.set_tool(Tool::Select);
    assert!(s.activate_page(&l.id));
    s.wipe_history();
    let m = l.kind.manufacturing().unwrap();
    Fx {
        s,
        l: l.clone(),
        dp,
        dr,
        m,
    }
}

fn add_op(d: &Document, m: NodeId, kind: &str, name: &str, members: &[NodeId]) -> NodeId {
    let index = d.operations(m).unwrap().len();
    let keys: Vec<Vec<NodeId>> = members.iter().map(|&n| vec![n]).collect();
    d.add_operation(m, &NewOperation { kind, name }, index, &keys)
        .unwrap()
}

fn doc(s: &Session) -> Document {
    unpack(99, &s.pack("0.1.0").unwrap()).unwrap()
}

fn bounds(d: &Document, id: NodeId) -> (Point, Point) {
    curvyo_ui_core::object_bounds(&d.object(id).unwrap())
}

fn edge_of(d: &Document, id: NodeId) -> Point {
    let (min, max) = bounds(d, id);
    Point::new(min.x + 0.3 * (max.x - min.x), min.y)
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

fn dbl(s: &mut Session, at: Point) {
    s.pointer_hover(at, false, false);
    s.pointer_down(at, false);
    s.pointer_up(at, false, false);
    s.pointer_down(at, false);
    s.pointer_up(at, false, false);
    s.double_click(at, false, false);
}

fn key(s: &mut Session, k: &str, ctrl: bool, shift: bool) {
    s.key_down(KeyInput {
        key: k,
        ctrl,
        shift,
        ..KeyInput::default()
    });
}

fn press_on(s: &mut Session, id: NodeId) {
    let at = edge_of(&doc(s), id);
    click(s, at);
}

fn shift_on(s: &mut Session, id: NodeId) {
    let at = edge_of(&doc(s), id);
    shift_click(s, at);
}

fn dbl_on(s: &mut Session, id: NodeId) {
    let at = edge_of(&doc(s), id);
    dbl(s, at);
}

fn view(s: &Session) -> Value {
    serde_json::from_str(&s.operations_view_json()).unwrap()
}

fn rows(s: &Session) -> Vec<Value> {
    view(s)["rows"].as_array().cloned().unwrap_or_default()
}

fn names(s: &Session) -> Vec<String> {
    rows(s)
        .iter()
        .map(|r| r["name"].as_str().unwrap().to_owned())
        .collect()
}

fn row(s: &Session, name: &str) -> Value {
    rows(s)
        .into_iter()
        .find(|r| r["name"] == name)
        .unwrap_or_else(|| panic!("no row {name}: {:?}", names(s)))
}

fn id_of(v: &Value) -> NodeId {
    serde_json::from_value(v["id"].clone()).unwrap()
}

/// The stored keys of the operation named `name`, sorted.
fn keys(s: &Session, m: NodeId, name: &str) -> Vec<Vec<NodeId>> {
    let d = doc(s);
    let mut k = d
        .operations(m)
        .unwrap()
        .into_iter()
        .find(|o| o.name == name)
        .unwrap_or_else(|| panic!("no operation {name}"))
        .members;
    k.sort_by_key(|p| p.iter().map(|n| n.full_id()).collect::<Vec<_>>());
    k
}

fn one(ids: &[NodeId]) -> Vec<Vec<NodeId>> {
    let mut k: Vec<Vec<NodeId>> = ids.iter().map(|&n| vec![n]).collect();
    k.sort_by_key(|p| p.iter().map(|n| n.full_id()).collect::<Vec<_>>());
    k
}

/// The undo step's name as the maker reads it: the Undo target, or the newest row of the
/// History panel when that names the step more closely.
fn undo_name(s: &mut Session) -> String {
    let target = s.history_target_name(Direction::Undo).unwrap_or_default();
    let rows = s.history_rows_json(0, 200);
    for word in ["Assign to ", "Remove from "] {
        if target.starts_with(word)
            && let Some(at) = rows.find(word)
        {
            let rest = &rows[at..];
            let end = rest.find('"').unwrap_or(rest.len());
            let named = &rest[..end];
            if named != target {
                return named.to_owned();
            }
        }
    }
    target
}

fn bar_kinds(s: &Session) -> Option<Vec<String>> {
    let v = view(s);
    let kinds = v.get("select_bar_kinds")?.as_array()?;
    Some(
        kinds
            .iter()
            .map(|k| k["kind"].as_str().unwrap().to_owned())
            .collect(),
    )
}

fn layer_rows(s: &mut Session) -> Vec<Value> {
    let v: Value = serde_json::from_str(&s.layers_view_json(0, 500)).unwrap();
    v["rows"].as_array().unwrap().clone()
}

fn selected(s: &mut Session) -> Vec<NodeId> {
    let mut v: Vec<NodeId> = layer_rows(s)
        .into_iter()
        .filter(|r| r["mark"] == "selected")
        .filter_map(|r| serde_json::from_value(r["id"].clone()).ok())
        .collect();
    v.sort_by_key(|n| n.full_id());
    v
}

/// The objects as the document reads them, with their boxes.
fn snap(s: &Session) -> String {
    let d = doc(s);
    let mut out = String::new();
    for o in d.read_objects().objects.iter() {
        let id = o.id();
        let (a, b) = curvyo_ui_core::object_bounds(o);
        out += &format!("{id:?} {:.6} {:.6} {:.6} {:.6};", a.x, a.y, b.x, b.y);
    }
    out
}

fn entered(s: &Session) -> Value {
    view(s)["entered_group"].clone()
}

/// Laser page with three free rectangles; nothing assigned.
fn three() -> (Fx, [NodeId; 3]) {
    let (d, l, dp, dr) = laser_doc();
    let a = rect_at(&d, &l, 10.0, 10.0);
    let b = rect_at(&d, &l, 60.0, 10.0);
    let c = rect_at(&d, &l, 10.0, 100.0);
    (open(&d, &l, dp, dr), [a, b, c])
}

// ---- criteria 1, 2: the buttons --------------------------------------------------------------

#[test]
fn c1_five_kinds_in_order_for_a_laser_selection_and_none_without() {
    let (mut f, [a, ..]) = three();
    assert_eq!(bar_kinds(&f.s), None, "no selection: no buttons");
    press_on(&mut f.s, a);
    assert_eq!(f.s.selected_object_count(), 1);
    assert_eq!(
        bar_kinds(&f.s).unwrap(),
        KINDS.map(str::to_owned).to_vec(),
        "Cut, Score, Engrave, Engrave + Outline, Cut-out"
    );
}

#[test]
fn c1_no_buttons_on_a_drawing_page_or_for_an_out_of_context_object() {
    let (mut f, [a, ..]) = three();
    // An object of the drawing page selected while the laser page stays active (`0106` 4).
    press_on(&mut f.s, f.dr);
    assert_eq!(
        f.s.active_page().id,
        f.l.id,
        "the press keeps the laser page"
    );
    assert!(f.s.selected_object_count() >= 1);
    assert_eq!(
        bar_kinds(&f.s),
        None,
        "out-of-context selection: no buttons"
    );
    // A mixed selection with an out-of-context object.
    f.s.escape();
    press_on(&mut f.s, a);
    let at = edge_of(&doc(&f.s), f.dr);
    shift_click(&mut f.s, at);
    assert_eq!(bar_kinds(&f.s), None, "a mixed selection: no buttons");
    // On the drawing page itself.
    assert!(f.s.activate_page(&f.dp.id));
    press_on(&mut f.s, f.dr);
    assert_eq!(bar_kinds(&f.s), None, "a drawing page: no buttons");
    assert_eq!(f.s.press_operation_kind("cut", false), None);
}

// ---- criteria 3 to 8: the reuse rule ----------------------------------------------------------

#[test]
fn c3_a_press_reuses_the_last_operation_of_that_kind_in_one_step() {
    let (d, l, dp, dr) = laser_doc();
    let a = rect_at(&d, &l, 10.0, 10.0);
    let m = l.kind.manufacturing().unwrap();
    add_op(&d, m, "engrave", "Engrave 1", &[]);
    let e2 = add_op(&d, m, "engrave", "Engrave 2", &[]);
    let mut f = open(&d, &l, dp, dr);
    press_on(&mut f.s, a);
    let before = f.s.undo_len();
    let got = f.s.press_operation_kind("engrave", false);
    assert_eq!(got, Some(e2), "Engrave 2, the lowest of its kind");
    assert_eq!(f.s.undo_len(), before + 1, "one step");
    assert_eq!(keys(&f.s, f.m, "Engrave 2"), one(&[a]));
    assert!(
        keys(&f.s, f.m, "Engrave 1").is_empty(),
        "A is in Engrave 2 only"
    );
    assert_eq!(names(&f.s).len(), 2, "no operation created");
    let _ = f.s.undo();
    assert!(
        keys(&f.s, f.m, "Engrave 2").is_empty(),
        "undo takes the whole step"
    );
}

#[test]
fn c3_the_last_of_a_kind_follows_the_list_order_not_the_creation_order() {
    let (d, l, dp, dr) = laser_doc();
    let a = rect_at(&d, &l, 10.0, 10.0);
    let m = l.kind.manufacturing().unwrap();
    let e1 = add_op(&d, m, "engrave", "Engrave 1", &[]);
    add_op(&d, m, "engrave", "Engrave 2", &[]);
    let mut f = open(&d, &l, dp, dr);
    // Move Engrave 1 below Engrave 2: it is now the lowest in the list.
    assert!(f.s.move_operation(e1, 1));
    assert_eq!(names(&f.s), ["Engrave 2", "Engrave 1"]);
    press_on(&mut f.s, a);
    assert_eq!(f.s.press_operation_kind("engrave", false), Some(e1));
    assert_eq!(keys(&f.s, f.m, "Engrave 1"), one(&[a]));
}

#[test]
fn c4_no_operation_of_the_kind_creates_one_at_the_end_of_its_section() {
    let (d, l, dp, dr) = laser_doc();
    let a = rect_at(&d, &l, 10.0, 10.0);
    let m = l.kind.manufacturing().unwrap();
    add_op(&d, m, "engrave", "Engrave 1", &[]);
    add_op(&d, m, "cut", "Cut 1", &[]);
    let mut f = open(&d, &l, dp, dr);
    press_on(&mut f.s, a);
    let before = f.s.undo_len();
    let id = f.s.press_operation_kind("score", false).unwrap();
    assert_eq!(f.s.undo_len(), before + 1, "create and assign is one step");
    let r = row(&f.s, "Score 1");
    assert_eq!(id_of(&r), id);
    assert_eq!(r["show"], true);
    assert_eq!(r["output"], true);
    assert_eq!(r["kind"], "score");
    assert_eq!(
        names(&f.s),
        ["Engrave 1", "Score 1", "Cut 1"],
        "at the end of the marking section, above the cuts"
    );
    assert_eq!(keys(&f.s, f.m, "Score 1"), one(&[a]));
    let _ = f.s.undo();
    assert_eq!(
        names(&f.s),
        ["Engrave 1", "Cut 1"],
        "undo removes the new row too"
    );
}

#[test]
fn c4_c5_every_kind_creates_its_first_operation_with_the_kind_name() {
    let (mut f, [a, ..]) = three();
    let words = [
        "Cut 1",
        "Score 1",
        "Engrave 1",
        "Engrave + Outline 1",
        "Cut-out 1",
    ];
    for (kind, word) in KINDS.iter().zip(words) {
        press_on(&mut f.s, a);
        f.s.press_operation_kind(kind, false).unwrap();
        assert!(
            names(&f.s).contains(&word.to_owned()),
            "{word}: {:?}",
            names(&f.s)
        );
    }
    assert_eq!(names(&f.s).len(), 5);
}

#[test]
fn c5_shift_always_creates_and_assigns_only_the_new_one() {
    let (d, l, dp, dr) = laser_doc();
    let a = rect_at(&d, &l, 10.0, 10.0);
    let m = l.kind.manufacturing().unwrap();
    add_op(&d, m, "engrave", "Engrave 1", &[]);
    add_op(&d, m, "engrave", "Engrave 2", &[]);
    let mut f = open(&d, &l, dp, dr);
    press_on(&mut f.s, a);
    let before = f.s.undo_len();
    let id = f.s.press_operation_kind("engrave", true).unwrap();
    assert_eq!(f.s.undo_len(), before + 1);
    assert_eq!(id_of(&row(&f.s, "Engrave 3")), id);
    assert_eq!(keys(&f.s, f.m, "Engrave 3"), one(&[a]));
    assert!(keys(&f.s, f.m, "Engrave 2").is_empty());
    // A second Shift press creates again, even with the selection already in Engrave 3.
    assert!(f.s.press_operation_kind("engrave", true).is_some());
    assert_eq!(names(&f.s).len(), 4);
}

#[test]
fn c6_an_unknown_or_damaged_operation_is_never_the_last_of_its_kind() {
    let (d, l, dp, dr) = laser_doc();
    let a = rect_at(&d, &l, 10.0, 10.0);
    let m = l.kind.manufacturing().unwrap();
    let damaged = add_op(&d, m, "engrave", "Engrave 1", &[]);
    let bad = serde_json::json!({"bogus": 1});
    d.write_operation_options(damaged, bad.as_object().unwrap())
        .unwrap();
    add_op(&d, m, "laser_weld", "Weld 1", &[]);
    let mut f = open(&d, &l, dp, dr);
    press_on(&mut f.s, a);
    let id = f.s.press_operation_kind("engrave", false).unwrap();
    assert_ne!(id, damaged, "the damaged Engrave 1 receives nothing");
    assert!(keys(&f.s, f.m, "Engrave 1").is_empty());
    assert_eq!(
        names(&f.s).len(),
        3,
        "a new engrave was created: {:?}",
        names(&f.s)
    );
    assert!(keys(&f.s, f.m, "Weld 1").is_empty());
}

fn fonts_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../frontend/public/fonts")
}

fn with_fonts(s: &mut Session) {
    let index: Value =
        serde_json::from_str(&std::fs::read_to_string(fonts_dir().join("index.json")).unwrap())
            .unwrap();
    let faces = index["faces"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| {
            (
                f["file"].as_str().unwrap().to_owned(),
                f["family"].as_str().unwrap().to_owned(),
                f["style"].as_str().unwrap().to_owned(),
                0,
            )
        })
        .collect();
    s.set_font_list(FontBlock::Included, faces, Vec::new());
    let _ = s.press_add_test_grid();
    for file in s.wanted_font_files() {
        let bytes = std::fs::read(fonts_dir().join(&file)).unwrap();
        assert!(s.add_font_file(&file, &bytes));
    }
}

#[test]
fn c6_a_test_grid_row_is_never_the_last_of_its_kind() {
    let (mut f, [a, ..]) = three();
    with_fonts(&mut f.s);
    let grid_rows: Vec<Value> = rows(&f.s)
        .into_iter()
        .filter(|r| !r["grid"].is_null())
        .collect();
    if grid_rows.is_empty() {
        f.s.press_add_test_grid().unwrap();
    }
    let grid = rows(&f.s)
        .into_iter()
        .find(|r| !r["grid"].is_null())
        .expect("a grid row");
    let kind = grid["kind"].as_str().unwrap().to_owned();
    let grid_id = id_of(&grid);
    let grid_items = grid["items"].clone();
    f.s.escape();
    press_on(&mut f.s, a);
    assert_eq!(f.s.selected_object_count(), 1);
    let before = rows(&f.s).len();
    let id = f.s.press_operation_kind(&kind, false).unwrap();
    assert_ne!(
        id, grid_id,
        "a generated row never receives hand-assigned objects"
    );
    assert_eq!(rows(&f.s).len(), before + 1, "a new {kind} was created");
    let grid_after = rows(&f.s)
        .into_iter()
        .find(|r| id_of(r) == grid_id)
        .unwrap();
    assert_eq!(grid_after["items"], grid_items);
}

#[test]
fn c7_a_press_that_changes_nothing_writes_no_step() {
    let (mut f, [a, b, _]) = three();
    press_on(&mut f.s, a);
    let at = edge_of(&doc(&f.s), b);
    shift_click(&mut f.s, at);
    let cut = f.s.press_operation_kind("cut", false).unwrap();
    let len = f.s.undo_len();
    let packed = keys(&f.s, f.m, "Cut 1");
    assert_eq!(
        f.s.press_operation_kind("cut", false),
        None,
        "nothing changes"
    );
    assert_eq!(f.s.undo_len(), len, "no history step");
    assert_eq!(keys(&f.s, f.m, "Cut 1"), packed);
    // Half of the selection in: the rest is added, one step.
    assert!(f.s.set_selection_in_operation(cut, false));
    f.s.escape();
    press_on(&mut f.s, a);
    assert!(f.s.set_selection_in_operation(cut, true));
    shift_on(&mut f.s, b);
    let len = f.s.undo_len();
    assert_eq!(f.s.press_operation_kind("cut", false), Some(cut));
    assert_eq!(f.s.undo_len(), len + 1);
    assert_eq!(keys(&f.s, f.m, "Cut 1"), one(&[a, b]));
}

#[test]
fn c7_guides_are_skipped_with_the_notice_and_guides_only_do_nothing() {
    let (d, l, dp, dr) = laser_doc();
    let g = rect_at(&d, &l, 10.0, 10.0);
    let item = rect_at(&d, &l, 60.0, 10.0);
    d.set_guide(&[g], true).unwrap();
    let mut f = open(&d, &l, dp, dr);
    press_on(&mut f.s, g);
    let len = f.s.undo_len();
    assert_eq!(f.s.press_operation_kind("cut", false), None);
    assert_eq!(
        f.s.press_operation_kind("cut", true),
        None,
        "not with Shift either"
    );
    assert_eq!(f.s.undo_len(), len, "guides only: no step");
    assert!(names(&f.s).is_empty(), "guides only: no operation");
    let notice = f.s.operations_notice().unwrap_or_default();
    assert!(notice.contains("Guides never run."), "notice {notice:?}");
    shift_on(&mut f.s, item);
    assert_eq!(f.s.selected_object_count(), 2);
    f.s.press_operation_kind("cut", false).unwrap();
    assert_eq!(
        keys(&f.s, f.m, "Cut 1"),
        one(&[item]),
        "only the item is assigned"
    );
    let notice = f.s.operations_notice().unwrap_or_default();
    assert!(notice.contains("1 guide"), "notice {notice:?}");
    assert!(notice.chars().count() <= 70);
}

#[test]
fn c8_no_empty_operation_by_a_press_and_the_number_keys_stay() {
    let (mut f, [a, ..]) = three();
    for kind in KINDS {
        assert_eq!(f.s.press_operation_kind(kind, false), None);
        assert_eq!(f.s.press_operation_kind(kind, true), None);
    }
    assert!(
        names(&f.s).is_empty(),
        "no empty operation with nothing selected"
    );
    assert_eq!(f.s.undo_len(), 0);
    press_on(&mut f.s, a);
    let cut = f.s.press_operation_kind("cut", false).unwrap();
    assert!(f.s.toggle_selection_by_number(1), "number key 1 unticks");
    assert!(keys(&f.s, f.m, "Cut 1").is_empty());
    assert!(f.s.set_selection_in_operation(cut, true), "the tick stays");
    assert_eq!(keys(&f.s, f.m, "Cut 1"), one(&[a]));
}

#[test]
fn c3_c4_c5_c10_the_undo_step_names_the_operation() {
    let (mut f, [a, ..]) = three();
    press_on(&mut f.s, a);
    f.s.press_operation_kind("score", false).unwrap();
    assert_eq!(
        undo_name(&mut f.s),
        "Assign to Score 1",
        "create and assign (4)"
    );
    f.s.escape();
    press_on(&mut f.s, a);
    let id = f.s.press_operation_kind("engrave", true).unwrap();
    assert_eq!(undo_name(&mut f.s), "Assign to Engrave 1", "Shift (5)");
    assert!(f.s.set_selection_in_operation(id, false));
    assert_eq!(undo_name(&mut f.s), "Remove from Engrave 1", "untick (10)");
    assert_eq!(f.s.press_operation_kind("engrave", false), Some(id));
    assert_eq!(undo_name(&mut f.s), "Assign to Engrave 1", "reuse (3)");
}

// ---- criteria 9, 10: groups ------------------------------------------------------------------

struct GroupFx {
    f: Fx,
    g: NodeId,
    a: NodeId,
    b: NodeId,
    other: NodeId,
}

/// Group G of A and B; B is in Engrave 1; one free rectangle `other`.
fn group_fx() -> GroupFx {
    let (d, l, dp, dr) = laser_doc();
    let a = rect_at(&d, &l, 10.0, 10.0);
    let b = rect_at(&d, &l, 60.0, 10.0);
    let other = rect_at(&d, &l, 200.0, 200.0);
    let g = d.group_objects(&[a, b]).unwrap();
    let m = l.kind.manufacturing().unwrap();
    add_op(&d, m, "engrave", "Engrave 1", &[b]);
    GroupFx {
        f: open(&d, &l, dp, dr),
        g,
        a,
        b,
        other,
    }
}

#[test]
fn c9_one_click_selects_the_group_and_a_press_assigns_every_leaf() {
    let GroupFx { mut f, g, a, b, .. } = group_fx();
    press_on(&mut f.s, a);
    assert_eq!(selected(&mut f.s), vec![g], "the group is one item");
    assert!(bar_kinds(&f.s).is_some(), "the bar shows the kind buttons");
    let len = f.s.undo_len();
    f.s.press_operation_kind("cut", false).unwrap();
    assert_eq!(f.s.undo_len(), len + 1, "one step");
    assert_eq!(
        keys(&f.s, f.m, "Cut 1"),
        one(&[g, b]),
        "the group's key and the key of B, which holds its own"
    );
    assert_eq!(
        keys(&f.s, f.m, "Engrave 1"),
        one(&[b]),
        "B still runs in Engrave 1"
    );
    assert!(
        !keys(&f.s, f.m, "Cut 1").contains(&vec![a]),
        "A has no own key: none written"
    );
    assert_eq!(row(&f.s, "Cut 1")["tick"], "all");
}

#[test]
fn c9_members_at_any_depth_that_hold_own_keys_are_added() {
    let (d, l, dp, dr) = laser_doc();
    let a = rect_at(&d, &l, 10.0, 10.0);
    let b = rect_at(&d, &l, 60.0, 10.0);
    let c = rect_at(&d, &l, 110.0, 10.0);
    let h = d.group_objects(&[b, c]).unwrap();
    let g = d.group_objects(&[a, h]).unwrap();
    let m = l.kind.manufacturing().unwrap();
    add_op(&d, m, "engrave", "Engrave 1", &[c]);
    add_op(&d, m, "score", "Score 1", &[h]);
    let mut f = open(&d, &l, dp, dr);
    press_on(&mut f.s, a);
    assert_eq!(selected(&mut f.s), vec![g]);
    f.s.press_operation_kind("cut", false).unwrap();
    assert_eq!(keys(&f.s, f.m, "Cut 1"), one(&[g, h, c]));
    assert_eq!(keys(&f.s, f.m, "Engrave 1"), one(&[c]));
    assert_eq!(keys(&f.s, f.m, "Score 1"), one(&[h]));
    // Pressing again changes nothing.
    let len = f.s.undo_len();
    assert_eq!(f.s.press_operation_kind("cut", false), None);
    assert_eq!(f.s.undo_len(), len);
}

#[test]
fn c10_the_group_tick_is_mixed_then_checked_and_unticking_removes_every_key() {
    let GroupFx { mut f, g, a, b, .. } = group_fx();
    let len0 = f.s.undo_len();
    press_on(&mut f.s, a);
    let e1 = id_of(&row(&f.s, "Engrave 1"));
    let r = row(&f.s, "Engrave 1");
    assert_eq!(r["tick"], "some", "B runs in Engrave 1, A not");
    let part = r["group_in_part"].as_str().unwrap_or_default().to_owned();
    // The session sends the group's name; the row's second line is "<name> in part".
    assert!(part.starts_with("Group"), "second line {part:?}");
    // Put B in a Cut too, then fill Engrave 1 by the mixed tick.
    f.s.escape();
    dbl_on(&mut f.s, b);
    press_on(&mut f.s, b);
    f.s.press_operation_kind("cut", true).unwrap();
    f.s.escape();
    f.s.escape();
    press_on(&mut f.s, a);
    assert_eq!(selected(&mut f.s), vec![g]);
    assert!(f.s.set_selection_in_operation(e1, true));
    let r = row(&f.s, "Engrave 1");
    assert_eq!(r["tick"], "all");
    assert!(r["group_in_part"].is_null());
    assert!(keys(&f.s, f.m, "Engrave 1").contains(&vec![g]));
    // Untick: the group key and the member keys of Engrave 1 go, the Cut of B stays.
    assert!(f.s.set_selection_in_operation(e1, false));
    assert!(
        keys(&f.s, f.m, "Engrave 1").is_empty(),
        "{:?}",
        keys(&f.s, f.m, "Engrave 1")
    );
    assert_eq!(row(&f.s, "Engrave 1")["tick"], "none");
    assert_eq!(
        keys(&f.s, f.m, "Cut 1"),
        one(&[b]),
        "other memberships stay"
    );
    let _ = len0;
}

#[test]
fn c10_c14_the_tab_never_names_members() {
    let GroupFx { mut f, a, .. } = group_fx();
    press_on(&mut f.s, a);
    f.s.press_operation_kind("cut", false).unwrap();
    for r in rows(&f.s) {
        let obj = r.as_object().unwrap();
        for k in obj.keys() {
            assert!(
                !k.contains("member") && !k.contains("children"),
                "row key {k} lists members"
            );
        }
        assert!(r["items"].is_number(), "a count, not a list");
    }
}

#[test]
fn c10_a_group_with_a_member_of_its_own_counts_as_one_item() {
    let GroupFx { mut f, a, .. } = group_fx();
    press_on(&mut f.s, a);
    f.s.press_operation_kind("cut", false).unwrap();
    assert_eq!(
        row(&f.s, "Cut 1")["items"],
        1,
        "the group counts as one item"
    );
}

// ---- criteria 11 to 13: the entered group ----------------------------------------------------

#[test]
fn c11_a_double_click_enters_read_only_with_the_crumb_names() {
    let GroupFx { mut f, a, .. } = group_fx();
    let len = f.s.undo_len();
    dbl_on(&mut f.s, a);
    let e = entered(&f.s);
    assert!(!e.is_null(), "entered");
    assert_eq!(e["page"], f.l.name.as_str());
    assert_eq!(e["groups"].as_array().unwrap().len(), 1);
    assert_eq!(f.s.undo_len(), len, "entering is no history step");
}

#[test]
fn c11_inside_nothing_moves_deletes_or_edits() {
    let GroupFx { mut f, a, b, .. } = group_fx();
    dbl_on(&mut f.s, a);
    press_on(&mut f.s, a);
    assert_eq!(selected(&mut f.s), vec![a], "a member is selectable");
    let before = snap(&f.s);
    let len = f.s.undo_len();
    // Nudge, delete, clipboard, duplicate, group, ungroup.
    for (k, ctrl, shift) in [
        ("ArrowRight", false, false),
        ("ArrowLeft", false, true),
        ("Delete", false, false),
        ("Backspace", false, false),
        ("x", true, false),
        ("c", true, false),
        ("v", true, false),
        ("d", true, false),
        ("g", true, false),
        ("g", true, true),
        ("j", true, false),
    ] {
        key(&mut f.s, k, ctrl, shift);
        assert!(!entered(&f.s).is_null(), "{k}: still entered");
        assert_eq!(snap(&f.s), before, "{k} changed the drawing");
        assert_eq!(f.s.undo_len(), len, "{k} wrote a step");
    }
    f.s.delete_selected();
    assert_eq!(snap(&f.s), before, "delete_selected");
    // A drag of a member.
    let from = edge_of(&doc(&f.s), a);
    let to = Point::new(from.x + 40.0, from.y + 30.0);
    f.s.pointer_hover(from, false, false);
    f.s.pointer_down(from, false);
    f.s.pointer_hover(Point::new(from.x + 10.0, from.y + 5.0), false, false);
    f.s.pointer_hover(to, false, false);
    f.s.pointer_up(to, false, false);
    assert_eq!(snap(&f.s), before, "a drag moved a member");
    assert_eq!(f.s.undo_len(), len);
    let _ = b;
}

#[test]
fn c11_an_svg_import_inside_is_refused() {
    let GroupFx { mut f, a, .. } = group_fx();
    dbl_on(&mut f.s, a);
    press_on(&mut f.s, a);
    let len = f.s.undo_len();
    let count = doc(&f.s).read_objects().objects.len();
    let _ = f.s.import_svg(SVG, "drop.svg");
    assert_eq!(
        doc(&f.s).read_objects().objects.len(),
        count,
        "Import SVG added objects"
    );
    assert_eq!(f.s.undo_len(), len, "Import SVG wrote a step");
}

#[test]
fn c11_a_file_drop_inside_is_refused() {
    let GroupFx { mut f, a, .. } = group_fx();
    dbl_on(&mut f.s, a);
    press_on(&mut f.s, a);
    let len = f.s.undo_len();
    let count = doc(&f.s).read_objects().objects.len();
    f.s.import_begin(Some((800.0, 500.0)));
    f.s.import_add(SVG, "drop.svg");
    let _ = f.s.import_finish(true);
    assert_eq!(
        doc(&f.s).read_objects().objects.len(),
        count,
        "a drop added objects"
    );
    assert_eq!(f.s.undo_len(), len, "a drop wrote a step");
}

const SVG: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" width="10mm" height="10mm" viewBox="0 0 10 10"><rect width="5" height="5"/></svg>"#;

#[test]
fn c11_the_kind_buttons_work_inside_and_handles_are_absent() {
    let GroupFx { mut f, a, .. } = group_fx();
    dbl_on(&mut f.s, a);
    press_on(&mut f.s, a);
    assert_eq!(
        bar_kinds(&f.s).unwrap(),
        KINDS.map(str::to_owned).to_vec(),
        "the bar holds the kind buttons"
    );
}

#[test]
fn c11_nested_groups_enter_one_level_per_double_click() {
    let (d, l, dp, dr) = laser_doc();
    let a = rect_at(&d, &l, 10.0, 10.0);
    let b = rect_at(&d, &l, 60.0, 10.0);
    let c = rect_at(&d, &l, 110.0, 10.0);
    let h = d.group_objects(&[b, c]).unwrap();
    let _g = d.group_objects(&[a, h]).unwrap();
    let mut f = open(&d, &l, dp, dr);
    let at = edge_of(&doc(&f.s), b);
    dbl(&mut f.s, at);
    assert_eq!(
        entered(&f.s)["groups"].as_array().unwrap().len(),
        1,
        "one level"
    );
    press_on(&mut f.s, b);
    assert_eq!(
        selected(&mut f.s),
        vec![h],
        "inside G a press selects the nested group"
    );
    dbl(&mut f.s, at);
    assert_eq!(
        entered(&f.s)["groups"].as_array().unwrap().len(),
        2,
        "the second level"
    );
    press_on(&mut f.s, b);
    assert_eq!(selected(&mut f.s), vec![b]);
    // A double-click on a leaf inside opens no edit.
    let before = snap(&f.s);
    dbl(&mut f.s, at);
    key(&mut f.s, "x", false, false);
    assert_eq!(snap(&f.s), before);
    assert_eq!(entered(&f.s)["groups"].as_array().unwrap().len(), 2);
    assert_eq!(f.s.undo_len(), 0);
}

#[test]
fn c11_a_group_on_a_drawing_page_is_entered_editable_as_built() {
    let (d, l, dp, dr) = laser_doc();
    let other = rect_at(&d, &dp, 60.0, 10.0);
    let _g = d.group_objects(&[dr, other]).unwrap();
    let mut f = open(&d, &l, dp.clone(), dr);
    assert!(f.s.activate_page(&dp.id));
    let at = edge_of(&doc(&f.s), dr);
    dbl(&mut f.s, at);
    press_on(&mut f.s, dr);
    assert_eq!(selected(&mut f.s), vec![dr], "a member selected inside");
    let before = snap(&f.s);
    key(&mut f.s, "ArrowRight", false, false);
    assert_ne!(
        snap(&f.s),
        before,
        "a nudge moves a member on a drawing page"
    );
}

#[test]
fn c12_a_member_gets_its_own_assignment_and_the_ticks_follow_it() {
    let GroupFx { mut f, g, a, b, .. } = group_fx();
    dbl_on(&mut f.s, a);
    press_on(&mut f.s, b);
    assert_eq!(selected(&mut f.s), vec![b]);
    assert_eq!(row(&f.s, "Engrave 1")["tick"], "all", "the tick follows B");
    press_on(&mut f.s, a);
    assert_eq!(row(&f.s, "Engrave 1")["tick"], "none", "the tick follows A");
    let len = f.s.undo_len();
    f.s.press_operation_kind("score", false).unwrap();
    assert_eq!(f.s.undo_len(), len + 1);
    assert_eq!(
        keys(&f.s, f.m, "Score 1"),
        one(&[a]),
        "A as an item of its own, not G"
    );
    assert!(!keys(&f.s, f.m, "Score 1").contains(&vec![g]));
}

#[test]
fn c12_a_marquee_inside_selects_members() {
    let (d, l, dp, dr) = laser_doc();
    let a = rect_at(&d, &l, 10.0, 10.0);
    let b = rect_at(&d, &l, 60.0, 60.0);
    let _g = d.group_objects(&[a, b]).unwrap();
    let mut f = open(&d, &l, dp, dr);
    dbl_on(&mut f.s, a);
    assert!(!entered(&f.s).is_null());
    // From an empty spot inside the group's box over B only.
    let from = Point::new(l.origin.x + 45.0, l.origin.y + 40.0);
    let to = Point::new(l.origin.x + 85.0, l.origin.y + 75.0);
    let before = snap(&f.s);
    f.s.pointer_hover(from, false, false);
    f.s.pointer_down(from, false);
    f.s.pointer_hover(Point::new(from.x + 5.0, from.y + 5.0), false, false);
    f.s.pointer_hover(to, false, false);
    f.s.pointer_up(to, false, false);
    assert_eq!(snap(&f.s), before, "a marquee moves nothing");
    assert!(
        !entered(&f.s).is_null(),
        "a marquee inside keeps the group entered"
    );
    assert_eq!(
        selected(&mut f.s),
        vec![b],
        "the marquee selects the member it covers"
    );
}

#[test]
fn c13_escape_leaves_with_the_group_selected_and_no_step() {
    let GroupFx { mut f, g, a, .. } = group_fx();
    dbl_on(&mut f.s, a);
    press_on(&mut f.s, a);
    f.s.escape();
    assert!(entered(&f.s).is_null(), "left");
    assert_eq!(selected(&mut f.s), vec![g], "the group stays selected");
    assert_eq!(f.s.undo_len(), 0, "entering and leaving are no steps");
    // After leaving, editing works again.
    let before = snap(&f.s);
    key(&mut f.s, "ArrowRight", false, false);
    assert_ne!(snap(&f.s), before, "a nudge moves the group after leaving");
}

#[test]
fn c13_a_press_outside_leaves_and_selects_what_it_hits() {
    let GroupFx {
        mut f, a, other, ..
    } = group_fx();
    dbl_on(&mut f.s, a);
    press_on(&mut f.s, other);
    assert!(entered(&f.s).is_null(), "left by a press outside");
    assert_eq!(selected(&mut f.s), vec![other]);
    // On empty canvas.
    dbl_on(&mut f.s, a);
    assert!(!entered(&f.s).is_null());
    click(
        &mut f.s,
        Point::new(f.l.origin.x + 350.0, f.l.origin.y + 280.0),
    );
    assert!(entered(&f.s).is_null(), "left by a press on empty canvas");
    assert_eq!(f.s.undo_len(), 0);
}

#[test]
fn c13_switching_the_page_leaves_the_group() {
    let GroupFx { mut f, a, .. } = group_fx();
    dbl_on(&mut f.s, a);
    assert!(!entered(&f.s).is_null());
    assert!(f.s.activate_page(&f.dp.id));
    assert!(f.s.activate_page(&f.l.id));
    assert!(entered(&f.s).is_null(), "not entered after coming back");
}

// ---- criterion 14: clones ------------------------------------------------------------------

#[test]
fn c14_a_clone_is_assigned_as_a_whole() {
    let GroupFx { mut f, g, a, .. } = group_fx();
    press_on(&mut f.s, a);
    assert_eq!(selected(&mut f.s), vec![g]);
    let _ = f.s.linked_clone();
    let sel = selected(&mut f.s);
    assert_eq!(sel.len(), 1, "the clone is selected");
    let clone = sel[0];
    assert_ne!(clone, g);
    f.s.press_operation_kind("cut", false).unwrap();
    assert_eq!(
        keys(&f.s, f.m, "Cut 1"),
        one(&[clone]),
        "one key: the clone's own"
    );
}

#[test]
fn c14_old_clone_member_keys_are_ignored_and_saved_back_unchanged() {
    let bytes = std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../curvyo-document-core/tests/fixtures/operations_clone_member_v39.curvyo"),
    )
    .unwrap();
    let original = unpack(5, &bytes).unwrap();
    let m = original.manufacturings()[0].id;
    let ops_before = original.operations(m).unwrap();
    let mut s = Session::open(7, &bytes).unwrap();
    let pages: Value = serde_json::from_str(&s.pages_json()).unwrap();
    let list = pages
        .as_array()
        .cloned()
        .unwrap_or_else(|| pages["pages"].as_array().cloned().unwrap());
    let laser = list
        .into_iter()
        .find(|p| p["manufacturing"] == m.full_id().as_str())
        .unwrap();
    assert!(s.activate_page(laser["id"].as_str().unwrap()));
    let v = view(&s);
    assert!(v["rows"].is_array(), "the page opens");
    let saved = unpack(6, &s.pack_for_save().unwrap()).unwrap();
    let ops_after = saved.operations(m).unwrap();
    assert_eq!(
        format!(
            "{:?}",
            ops_before.iter().map(|o| &o.members).collect::<Vec<_>>()
        ),
        format!(
            "{:?}",
            ops_after.iter().map(|o| &o.members).collect::<Vec<_>>()
        ),
        "every key, legacy paths included, saved back unchanged"
    );
    // Counted nowhere: no row counts more items than it has one-id keys.
    for (op, r) in ops_after.iter().zip(v["rows"].as_array().unwrap()) {
        let own = op.members.iter().filter(|p| p.len() == 1).count();
        assert!(
            r["items"].as_u64().unwrap() as usize <= own,
            "{}: {} items, {own} one-id keys",
            op.name,
            r["items"]
        );
    }
}

// ---- criterion 15: the Operations view toggle -------------------------------------------------

fn page_json(s: &Session, id: &str) -> Value {
    let v: Value = serde_json::from_str(&s.pages_json()).unwrap();
    let list = v
        .as_array()
        .cloned()
        .unwrap_or_else(|| v["pages"].as_array().cloned().unwrap());
    list.into_iter().find(|p| p["id"] == id).unwrap()
}

fn colours(s: &Session) -> Vec<(u8, u8, u8)> {
    s.draw_list()
        .triangles
        .iter()
        .map(|v| (v.color.r, v.color.g, v.color.b))
        .collect()
}

#[test]
fn c15_the_toggle_is_per_laser_page_session_only_and_no_step() {
    let (d, l, dp, dr) = laser_doc();
    let a = rect_at(&d, &l, 10.0, 10.0);
    let made = d
        .add_output_page(NewOutputPage {
            process: LASER,
            label: "Laser 2",
            profile: None,
            work_area: DocumentSize::from_mm(400.0, 300.0),
        })
        .unwrap();
    let mut f = open(&d, &l, dp.clone(), dr);
    assert_eq!(
        page_json(&f.s, &l.id)["operationsView"],
        true,
        "pressed by default"
    );
    assert_eq!(page_json(&f.s, &made.id)["operationsView"], true);
    assert!(
        page_json(&f.s, &dp.id)["operationsView"].is_null(),
        "no toggle on a drawing page"
    );
    assert!(
        !f.s.toggle_operations_view(&dp.id),
        "a drawing page has no toggle"
    );
    let before = f.s.pack("0.1.0").unwrap();
    assert!(f.s.toggle_operations_view(&l.id));
    assert_eq!(page_json(&f.s, &l.id)["operationsView"], false);
    assert_eq!(
        page_json(&f.s, &made.id)["operationsView"],
        true,
        "per page"
    );
    assert_eq!(f.s.undo_len(), 0, "no history step");
    assert_eq!(view(&f.s)["operations_view"], false);
    // Not saved.
    let after = f.s.pack("0.1.0").unwrap();
    let reopened = Session::open(8, &after).unwrap();
    assert_eq!(
        page_json(&reopened, &l.id)["operationsView"],
        true,
        "not in the file"
    );
    let _ = (before, a);
    // Survives a page switch within the session.
    assert!(f.s.activate_page(&dp.id));
    assert!(f.s.activate_page(&l.id));
    assert_eq!(
        page_json(&f.s, &l.id)["operationsView"],
        false,
        "session state"
    );
    assert!(f.s.toggle_operations_view(&l.id));
    assert_eq!(page_json(&f.s, &l.id)["operationsView"], true);
    let _ = PageKind::Drawing;
}

#[test]
fn c15_unpressed_draws_the_drawing_style_without_the_operation_look() {
    let (mut f, [a, ..]) = three();
    press_on(&mut f.s, a);
    f.s.press_operation_kind("cut", false).unwrap();
    f.s.escape();
    assert!(colours(&f.s).contains(&CUT_RED), "pressed: the Cut look");
    assert!(f.s.toggle_operations_view(&f.l.id));
    assert!(!colours(&f.s).contains(&CUT_RED), "unpressed: no Cut look");
    assert!(f.s.toggle_operations_view(&f.l.id));
    assert!(colours(&f.s).contains(&CUT_RED));
}
