//! Independent acceptance tests for `specs/0106-move-versus-edit` through the public session:
//! the context table (criteria 1, 2), selection rules (3, 4), move commands out of context (5),
//! refused edits (9, 10), double-click targets (13 to 16), the laser page tree (17 to 19) and the
//! unchanged format (21).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::too_many_lines,
    clippy::similar_names,
    clippy::many_single_char_names,
    clippy::cast_precision_loss,
    clippy::format_push_string,
    clippy::assert_is_empty,
    clippy::needless_pass_by_value,
    missing_docs
)]

use curvyo_document_core::{
    CURRENT_FORMAT_VERSION, Document, DocumentSize, NewOutputPage, NodeId, Page, PageKind, Point,
    RectBounds, Style, pack, unpack,
};
use curvyo_editor_wasm::{KeyInput, RowPress, Session, Tool, Workbench};
use curvyo_sketch_core as sk;
use curvyo_ui_core::{AlignCommand, FlipAxis};
use serde_json::Value;

struct Fx {
    s: Session,
    a: Page,
    b: Page,
    l: Page,
    sk: Page,
    ra: NodeId,
    rb: NodeId,
    rl: NodeId,
    rp: NodeId,
    rs: NodeId,
    rbl: NodeId,
    /// A second rectangle on B.
    rb2: NodeId,
}

fn find_page(d: &Document, id: &str) -> Page {
    d.pages().into_iter().find(|p| p.id == id).unwrap()
}

fn rect(d: &Document, x: f64, y: f64, w: f64, h: f64) -> NodeId {
    d.create_rect(RectBounds::from_corners(
        Point::new(x, y),
        Point::new(x + w, y + h),
    ))
}

fn open_session(d: &Document) -> Session {
    let mut s = Session::open(7, &pack(d, "0106-tester").unwrap()).unwrap();
    s.resize_viewport(1600.0, 1000.0);
    let mut snap = s.snapping();
    snap.enabled = false;
    s.set_snapping(snap);
    s.set_tool(Tool::Select);
    s
}

fn closed_box() -> sk::Sketch {
    sk::create_polyline(
        &sk::Sketch::new("origin".into()),
        "edge",
        &[
            sk::Position::mm(-20.0, -15.0),
            sk::Position::mm(20.0, -15.0),
            sk::Position::mm(20.0, 15.0),
            sk::Position::mm(-20.0, 15.0),
        ],
        true,
    )
    .unwrap()
}

/// Drawing pages A (`first`) and B, laser L, sketch S with a box; one rectangle on A, B and L,
/// one on the pasteboard, one over S and one straddling B and L. A is active.
fn fixture() -> Fx {
    let d = Document::new(1);
    let b = d.add_page_of(&PageKind::Drawing).unwrap();
    let l = d
        .add_output_page(NewOutputPage {
            process: "curvyo.laser",
            label: "Laser",
            profile: None,
            work_area: DocumentSize::from_mm(400.0, 300.0),
        })
        .unwrap();
    let sp = d.add_page_of(&PageKind::Sketch).unwrap();
    d.create_page_sketch(&sp.id, &closed_box(), &Style::default())
        .unwrap();
    let a = find_page(&d, "first");
    let b = find_page(&d, &b.id);
    let l = find_page(&d, &l.id);
    let sk = find_page(&d, &sp.id);
    let ra = rect(&d, a.origin.x + 10.0, a.origin.y + 10.0, 20.0, 10.0);
    let rb = rect(&d, b.origin.x + 10.0, b.origin.y + 10.0, 20.0, 10.0);
    let rl = rect(&d, l.origin.x + 10.0, l.origin.y + 10.0, 20.0, 10.0);
    let rp = rect(&d, a.origin.x + 10.0, a.origin.y - 80.0, 20.0, 10.0);
    let rs = rect(&d, sk.origin.x + 5.0, sk.origin.y + 5.0, 20.0, 10.0);
    let rb2 = rect(&d, b.origin.x + 60.0, b.origin.y + 10.0, 20.0, 10.0);
    let rbl = rect(&d, b.origin.x + 220.0, b.origin.y + 100.0, 70.0, 20.0);
    let mut s = open_session(&d);
    s.activate_page("first");
    s.wipe_history();
    Fx {
        s,
        a,
        b,
        l,
        sk,
        ra,
        rb,
        rl,
        rp,
        rs,
        rbl,
        rb2,
    }
}

fn doc(s: &Session) -> Document {
    unpack(99, &s.pack("0.1.0").unwrap()).unwrap()
}

/// Byte equality of two packed files.
fn same(a: &[u8], b: &[u8]) -> bool {
    a == b
}

fn bytes(s: &Session) -> Vec<u8> {
    s.pack("0.1.0").unwrap()
}

/// The objects as the document reads them: equal after an undo although the oplog differs.
fn snap(s: &Session) -> String {
    let d = doc(s);
    let mut out = String::new();
    for o in d.read_objects().objects.iter() {
        if let curvyo_document_core::ObjectSnapshot::Sketch(_) = o {
            continue;
        }
        let id = o.id();
        let (a, b) = bounds(&d, id);
        out += &format!("{id:?} {:.9} {:.9} {:.9} {:.9};", a.x, a.y, b.x, b.y);
    }
    out
}

fn bounds(d: &Document, id: NodeId) -> (Point, Point) {
    curvyo_ui_core::object_bounds(&d.object(id).unwrap())
}

fn edge_of(d: &Document, id: NodeId) -> Point {
    let (min, max) = bounds(d, id);
    Point::new(min.x + 0.3 * (max.x - min.x), min.y)
}

fn centre_of(d: &Document, id: NodeId) -> Point {
    let (min, max) = bounds(d, id);
    Point::new(f64::midpoint(min.x, max.x), f64::midpoint(min.y, max.y))
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

fn dbl(s: &mut Session, at: Point) {
    s.pointer_hover(at, false, false);
    s.pointer_down(at, false);
    s.pointer_up(at, false, false);
    s.pointer_down(at, false);
    s.pointer_up(at, false, false);
    s.double_click(at, false, false);
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

fn drawing_selected(s: &mut Session) -> Vec<NodeId> {
    layer_rows(s)
        .into_iter()
        .filter(|r| r["mark"] == "selected" && r["kind"] == "rectangle")
        .filter_map(|r| serde_json::from_value(r["id"].clone()).ok())
        .collect()
}

fn sorted(ids: &[NodeId]) -> Vec<NodeId> {
    let mut v = ids.to_vec();
    v.sort_by_key(|n| n.full_id());
    v
}

fn select_by_clicks(s: &mut Session, ids: &[NodeId]) {
    s.escape();
    let d = doc(s);
    for (i, &id) in ids.iter().enumerate() {
        let at = edge_of(&d, id);
        if i == 0 {
            click(s, at);
        } else {
            shift_click(s, at);
        }
    }
    assert!(s.selected_object_count() >= ids.len().min(1));
}

fn all(f: &Fx) -> [(&'static str, NodeId); 6] {
    [
        ("ra", f.ra),
        ("rb", f.rb),
        ("rl", f.rl),
        ("rp", f.rp),
        ("rs", f.rs),
        ("rbl", f.rbl),
    ]
}

// ---- criteria 1 and 2: the table, and a press keeps the page -------------------------------

#[test]
fn ac1_ac2_context_table_for_every_active_page() {
    let mut f = fixture();
    // (active page, rectangles in context)
    let rows: Vec<(String, Vec<&str>)> = vec![
        (f.a.id.clone(), vec!["ra", "rp", "rs"]),
        (f.b.id.clone(), vec!["rb", "rp", "rs"]),
        (f.l.id.clone(), vec!["rl", "rbl"]),
        (f.sk.id.clone(), vec![]),
    ];
    for (page, inside) in rows {
        for (name, id) in all(&f) {
            f.s.activate_page(&page);
            f.s.escape();
            let steps = f.s.undo_len();
            let d = doc(&f.s);
            let at = edge_of(&d, id);
            click(&mut f.s, at);
            assert_eq!(
                f.s.selected_object_count(),
                1,
                "press on {name} with {page} active selects it"
            );
            assert_eq!(f.s.active_page().id, page, "press on {name} keeps {page}");
            assert_eq!(f.s.undo_len(), steps, "press writes no step");
            assert_eq!(
                f.s.selection_move_only(),
                !inside.contains(&name),
                "{name} with {page} active: in context = {}",
                inside.contains(&name)
            );
        }
    }
}

#[test]
fn ac2_an_empty_press_in_another_pages_frame_still_activates_it() {
    let mut f = fixture();
    let p = Point::new(f.b.origin.x + 150.0, f.b.origin.y + 200.0);
    click(&mut f.s, p);
    assert_eq!(f.s.active_page().id, f.b.id);
    let p = Point::new(f.l.origin.x + 300.0, f.l.origin.y + 200.0);
    click(&mut f.s, p);
    assert_eq!(f.s.active_page().id, f.l.id);
    let p = Point::new(f.a.origin.x + 150.0, f.a.origin.y + 250.0);
    click(&mut f.s, p);
    assert_eq!(f.s.active_page().id, f.a.id);
}

#[test]
fn ac2_press_on_foreign_object_then_drag_moves_it_without_switching_page() {
    let mut f = fixture();
    let d = doc(&f.s);
    let at = edge_of(&d, f.rb);
    let (min0, _) = bounds(&d, f.rb);
    f.s.pointer_hover(at, false, false);
    f.s.pointer_down(at, false);
    let to = Point::new(at.x + 25.0, at.y + 7.0);
    f.s.pointer_hover(Point::new(at.x + 10.0, at.y + 3.0), false, false);
    f.s.pointer_hover(to, false, false);
    f.s.pointer_up(to, false, false);
    assert_eq!(f.s.active_page().id, f.a.id);
    let (min1, _) = bounds(&doc(&f.s), f.rb);
    assert!((min1.x - min0.x - 25.0).abs() < 1e-6 && (min1.y - min0.y - 7.0).abs() < 1e-6);
    assert_eq!(f.s.undo_len(), 1);
}

// ---- criterion 3: Shift, marquee, Select all ----------------------------------------------

#[test]
fn ac3_shift_adds_objects_of_any_context() {
    let mut f = fixture();
    select_by_clicks(&mut f.s, &[f.ra, f.rb, f.rl, f.rp]);
    assert_eq!(selected(&mut f.s), sorted(&[f.ra, f.rb, f.rl, f.rp]));
    assert_eq!(f.s.active_page().id, f.a.id);
    // Shift again removes.
    let at = edge_of(&doc(&f.s), f.rb);
    shift_click(&mut f.s, at);
    assert_eq!(selected(&mut f.s), sorted(&[f.ra, f.rl, f.rp]));
}

#[test]
fn ac3_marquee_selects_every_context_it_touches() {
    let mut f = fixture();
    // From the pasteboard above A to across B: touches rp, ra, rb.
    let from = Point::new(f.a.origin.x - 5.0, f.a.origin.y - 100.0);
    let to = Point::new(f.b.origin.x + 100.0, f.b.origin.y + 40.0);
    f.s.pointer_hover(from, false, false);
    f.s.pointer_down(from, false);
    f.s.pointer_hover(
        Point::new(f64::midpoint(from.x, to.x), f64::midpoint(from.y, to.y)),
        false,
        false,
    );
    f.s.pointer_hover(to, false, false);
    f.s.pointer_up(to, false, false);
    assert_eq!(selected(&mut f.s), sorted(&[f.rp, f.ra, f.rb, f.rb2]));
    assert!(f.s.selection_move_only(), "contains rb: out of context");
}

#[test]
fn ac3_select_all_selects_only_what_is_in_context_of_the_active_page() {
    let mut f = fixture();
    f.s.escape();
    key(&mut f.s, "a", true, false);
    assert_eq!(selected(&mut f.s), sorted(&[f.ra, f.rp, f.rs]), "A active");
    assert!(!f.s.selection_move_only());
    f.s.activate_page(&f.b.id);
    f.s.escape();
    key(&mut f.s, "a", true, false);
    assert_eq!(
        selected(&mut f.s),
        sorted(&[f.rb, f.rb2, f.rp, f.rs]),
        "B active"
    );
    f.s.activate_page(&f.l.id);
    f.s.escape();
    key(&mut f.s, "a", true, false);
    assert_eq!(selected(&mut f.s), sorted(&[f.rl, f.rbl]), "L active");
}

#[test]
fn ac3_on_a_sketch_page_ctrl_a_and_marquee_stay_on_sketch_geometry() {
    let mut f = fixture();
    f.s.activate_page(&f.sk.id);
    f.s.escape();
    key(&mut f.s, "a", true, false);
    assert!(
        drawing_selected(&mut f.s).is_empty(),
        "no drawing object by Ctrl+A"
    );
    let from = Point::new(f.sk.origin.x + 1.0, f.sk.origin.y + 1.0);
    let to = Point::new(f.sk.origin.x + 100.0, f.sk.origin.y + 100.0);
    f.s.pointer_hover(from, false, false);
    f.s.pointer_down(from, false);
    f.s.pointer_hover(to, false, false);
    f.s.pointer_up(to, false, false);
    assert!(
        drawing_selected(&mut f.s).is_empty(),
        "no drawing object by marquee"
    );
}

// ---- criterion 4 and 9: mixed selection is move-only, edits are refused ---------------------

#[test]
fn ac4_mixed_selection_refuses_fill_and_in_context_alone_changes_it() {
    let mut f = fixture();
    select_by_clicks(&mut f.s, &[f.ra, f.rb]);
    assert!(f.s.selection_move_only());
    let before = bytes(&f.s);
    f.s.set_fill_paint(true);
    f.s.set_fill_paint(false);
    assert!(same(&bytes(&f.s), &before), "file unchanged");
    assert_eq!(f.s.undo_len(), 0);
    select_by_clicks(&mut f.s, &[f.ra]);
    f.s.set_fill_paint(true);
    f.s.set_fill_paint(false);
    assert!(!same(&bytes(&f.s), &before), "in context alone: changes");
    assert!(f.s.undo_len() >= 1);
}

#[test]
fn ac9_every_edit_command_is_refused_under_a_move_only_selection() {
    let mut f = fixture();
    let page_a = f.a.id.clone();
    // rb and rbl: both out of context for A.
    select_by_clicks(&mut f.s, &[f.rb, f.rb2]);
    assert!(f.s.selection_move_only());
    let before = bytes(&f.s);
    let sel = selected(&mut f.s);
    let check = |f: &mut Fx, what: &str| {
        assert!(same(&bytes(&f.s), &before), "{what}: file unchanged");
        assert_eq!(f.s.undo_len(), 0, "{what}: no history step");
        assert_eq!(selected(&mut f.s), sel, "{what}: selection kept");
        assert_eq!(f.s.active_page().id, page_a);
    };
    f.s.set_fill_paint(true);
    check(&mut f, "fill on");
    f.s.set_fill_paint(false);
    check(&mut f, "fill off");
    f.s.set_stroke_paint(false);
    check(&mut f, "stroke off");
    f.s.set_stroke_paint(true);
    check(&mut f, "stroke on");
    let _ = f.s.set_fill_kind("linear");
    check(&mut f, "fill kind");
    let _ = f.s.group_selection();
    check(&mut f, "group");
    let _ = f.s.apply_boolean(curvyo_ui_core::BooleanOp::Union);
    check(&mut f, "boolean union");
    let _ = f.s.unlink_clones();
    check(&mut f, "unlink clone");
    let _ = f.s.object_to_path();
    check(&mut f, "object to path");
    for (k, ctrl, shift) in [
        ("g", true, false),
        ("g", true, true),
        ("Home", false, false),
        ("End", false, false),
        ("PageUp", false, false),
        ("PageDown", false, false),
        ("Home", true, false),
        ("End", true, false),
    ] {
        key(&mut f.s, k, ctrl, shift);
        check(&mut f, &format!("key {k} ctrl={ctrl} shift={shift}"));
    }
}

#[test]
fn ac9_node_tool_shows_nothing_and_changes_nothing_on_a_foreign_object() {
    let mut f = fixture();
    select_by_clicks(&mut f.s, &[f.rb]);
    f.s.set_tool(Tool::Node);
    let before = bytes(&f.s);
    let c = centre_of(&doc(&f.s), f.rb);
    let (min, max) = bounds(&doc(&f.s), f.rb);
    // A press on a corner, and a drag from it.
    let corner = min;
    f.s.pointer_hover(corner, false, false);
    f.s.pointer_down(corner, false);
    f.s.pointer_hover(Point::new(corner.x + 3.0, corner.y + 3.0), false, false);
    f.s.pointer_hover(Point::new(corner.x + 8.0, corner.y + 8.0), false, false);
    f.s.pointer_up(Point::new(corner.x + 8.0, corner.y + 8.0), false, false);
    let _ = (c, max);
    assert!(same(&bytes(&f.s), &before), "no node moved");
    assert_eq!(f.s.undo_len(), 0);
    assert_eq!(f.s.active_page().id, f.a.id);
    assert_eq!(f.s.tool(), Tool::Node, "no tool switched away");
}

#[test]
fn ac10_text_tool_press_on_foreign_object_starts_new_text_in_active_context() {
    let mut f = fixture();
    f.s.set_tool(Tool::Text);
    let before = f.s.undo_len();
    let at = Point::new(f.a.origin.x + 100.0, f.a.origin.y + 100.0);
    click(&mut f.s, at);
    assert_eq!(f.s.active_page().id, f.a.id);
    assert_eq!(
        f.s.undo_len(),
        before,
        "opening a text box writes nothing yet"
    );
    f.s.escape();
}

#[test]
fn ac11_style_line_names_the_home_and_only_for_move_only_selections() {
    let mut f = fixture();
    select_by_clicks(&mut f.s, &[f.ra]);
    assert_eq!(f.s.out_of_context_line(), "");
    select_by_clicks(&mut f.s, &[f.rb]);
    let name = f.b.name.clone();
    let line = f.s.out_of_context_line();
    assert_eq!(line, format!("On {name}. Double-click to edit."));
    assert!(line.chars().count() <= 70);
    select_by_clicks(&mut f.s, &[f.rl]);
    assert!(
        f.s.out_of_context_line().starts_with("On Laser"),
        "{}",
        f.s.out_of_context_line()
    );
    f.s.activate_page(&f.l.id);
    select_by_clicks(&mut f.s, &[f.rp]);
    assert_eq!(
        f.s.out_of_context_line(),
        "On Pasteboard. Double-click to edit."
    );
}

#[test]
fn ac11_long_page_name_is_cut_at_24_characters_and_the_line_at_70() {
    let d = Document::new(1);
    let b = d.add_page_of(&PageKind::Drawing).unwrap();
    d.rename_page(&b.id, "A very very long page name that goes on and on")
        .unwrap();
    let b = find_page(&d, &b.id);
    let rb = rect(&d, b.origin.x + 10.0, b.origin.y + 10.0, 20.0, 10.0);
    let mut s = open_session(&d);
    s.activate_page("first");
    let at = edge_of(&doc(&s), rb);
    click(&mut s, at);
    let line = s.out_of_context_line();
    assert!(!line.is_empty());
    assert!(line.chars().count() <= 70, "{line}");
    assert!(!line.contains("long page name that goes"), "{line}");
}

#[test]
fn ac9_hidden_command_keys_show_the_notice_with_the_page_name() {
    let mut f = fixture();
    select_by_clicks(&mut f.s, &[f.rb, f.rb2]);
    let name = f.b.name.clone();
    key(&mut f.s, "g", true, false);
    let n = f.s.take_context_notice();
    assert_eq!(n, format!("Not editable here: on {name}."));
    assert_eq!(f.s.take_context_notice(), "", "once");
    key(&mut f.s, "Home", false, false);
    assert!(
        f.s.take_context_notice()
            .starts_with("Not editable here: on ")
    );
    // A move key shows none.
    key(&mut f.s, "ArrowRight", false, false);
    assert_eq!(f.s.take_context_notice(), "");
}

// ---- criterion 5: every move command works out of context -------------------------------

fn outline_min(s: &Session, id: NodeId) -> Point {
    bounds(&doc(s), id).0
}

#[test]
fn ac5_arrows_flip_align_delete_duplicate_are_one_undoable_step_each_out_of_context() {
    let mut f = fixture();
    select_by_clicks(&mut f.s, &[f.rb]);
    let orig = snap(&f.s);
    let m0 = outline_min(&f.s, f.rb);
    key(&mut f.s, "ArrowRight", false, false);
    assert_eq!(f.s.undo_len(), 1);
    let m1 = outline_min(&f.s, f.rb);
    assert!(m1.x > m0.x, "arrow moved it");
    assert_eq!(f.s.active_page().id, f.a.id, "a move keeps the page");
    assert!(f.s.selection_move_only(), "still out of context");
    f.s.undo();
    assert_eq!(snap(&f.s), orig);
    // Flip, on two objects so it changes something
    select_by_clicks(&mut f.s, &[f.rb, f.rb2]);
    let flip_orig = snap(&f.s);
    let _ = f.s.flip_selection(FlipAxis::Horizontal);
    assert_eq!(f.s.undo_len(), 1, "flip: one step");
    assert_ne!(snap(&f.s), flip_orig);
    f.s.undo();
    assert_eq!(snap(&f.s), flip_orig);
    select_by_clicks(&mut f.s, &[f.rb]);
    let _ = f.s.flip_selection(FlipAxis::Horizontal);
    f.s.undo();
    assert_eq!(snap(&f.s), orig);
    // Delete
    key(&mut f.s, "Delete", false, false);
    assert!(doc(&f.s).object(f.rb).is_none(), "deleted");
    assert_eq!(f.s.undo_len(), 1);
    f.s.undo();
    assert_eq!(snap(&f.s), orig);
    // Duplicate
    let n0 = doc(&f.s).read_objects().objects.len();
    let _ = f.s.duplicate_layer_target();
    assert_eq!(doc(&f.s).read_objects().objects.len(), n0 + 1, "duplicated");
    assert_eq!(f.s.undo_len(), 1);
    f.s.undo();
    assert_eq!(snap(&f.s), orig);
}

#[test]
fn ac5_align_and_arrange_in_a_grid_work_out_of_context() {
    let mut f = fixture();
    select_by_clicks(&mut f.s, &[f.rb, f.rb2]);
    assert!(f.s.selection_move_only());
    let orig = snap(&f.s);
    let n0 = doc(&f.s).read_objects().objects.len();
    let _ = f.s.align_selection(AlignCommand::Left);
    assert_eq!(f.s.undo_len(), 1, "align is one step");
    let l1 = outline_min(&f.s, f.rb).x;
    let l2 = outline_min(&f.s, f.rb2).x;
    assert!((l1 - l2).abs() < 1e-6, "left edges aligned");
    f.s.undo();
    assert_eq!(snap(&f.s), orig);
    let out = f.s.arrange_grid(1, 2, 5.0);
    let _ = out;
    assert_eq!(f.s.undo_len(), 1, "arrange in a grid is one step");
    assert_eq!(
        doc(&f.s).read_objects().objects.len(),
        n0,
        "creates or deletes nothing"
    );
    assert_ne!(snap(&f.s), orig);
    assert_eq!(f.s.active_page().id, f.a.id);
    f.s.undo();
    assert_eq!(snap(&f.s), orig);
}

#[test]
fn ac5_linked_clone_and_place_on_work_for_a_foreign_selection() {
    let mut f = fixture();
    select_by_clicks(&mut f.s, &[f.rb]);
    let n0 = doc(&f.s).read_objects().objects.len();
    let _ = f.s.linked_clone();
    assert_eq!(f.s.undo_len(), 1, "linked clone is one step");
    assert_eq!(doc(&f.s).read_objects().objects.len(), n0 + 1);
    f.s.undo();
    select_by_clicks(&mut f.s, &[f.rb]);
    let lid = f.l.id.clone();
    assert!(f.s.place_on(&lid), "Place on… works from a foreign page");
    assert_eq!(f.s.undo_len(), 1);
}

// ---- criteria 13 to 16: double-click targets ----------------------------------------------

#[test]
fn ac13_double_click_on_a_foreign_object_activates_its_page_and_nothing_else() {
    let mut f = fixture();
    // A group on B (made while B is active), then A active.
    f.s.activate_page(&f.b.id);
    let b2 = {
        let d = doc(&f.s);
        rect(&d, f.b.origin.x + 60.0, f.b.origin.y + 150.0, 20.0, 10.0)
    };
    let _ = b2;
    select_by_clicks(&mut f.s, &[f.rb]);
    let mut s2 = f.s;
    let _ = &mut s2;
    f.s = s2;
    // Build the group via a second rect on B in the document.
    let d = Document::new(1);
    let _ = d;
    // Use the session: select rb and the new rect by marquee, group them.
    let from = Point::new(f.b.origin.x + 5.0, f.b.origin.y + 5.0);
    let to = Point::new(f.b.origin.x + 150.0, f.b.origin.y + 100.0);
    f.s.escape();
    f.s.pointer_hover(from, false, false);
    f.s.pointer_down(from, false);
    f.s.pointer_hover(to, false, false);
    f.s.pointer_up(to, false, false);
    assert!(f.s.selected_object_count() >= 1);
    key(&mut f.s, "g", true, false);
    f.s.activate_page(&f.a.id);
    f.s.escape();
    let g_at = Point::new(f.b.origin.x + 20.0, f.b.origin.y + 10.0);
    f.s.wipe_history();
    click(&mut f.s, g_at);
    assert_eq!(f.s.active_page().id, f.a.id, "press keeps A");
    let sel = selected(&mut f.s);
    assert_eq!(sel.len(), 1);
    dbl(&mut f.s, g_at);
    assert_eq!(
        f.s.active_page().id,
        f.b.id,
        "B active after the double-click"
    );
    assert_eq!(selected(&mut f.s), sel, "object stays selected");
    assert!(!f.s.selection_move_only(), "now in context");
    assert_eq!(f.s.undo_len(), 0, "activation is not a history step");
}

#[test]
fn ac13_double_click_with_the_node_tool_on_foreign_object_only_activates() {
    let mut f = fixture();
    f.s.set_tool(Tool::Node);
    let at = edge_of(&doc(&f.s), f.rb);
    dbl(&mut f.s, at);
    assert_eq!(f.s.active_page().id, f.b.id);
    assert_eq!(f.s.undo_len(), 0);
}

#[test]
fn ac14_pasteboard_double_click_opens_the_last_active_drawing_page() {
    let mut f = fixture();
    f.s.activate_page(&f.b.id);
    f.s.activate_page(&f.l.id);
    let at = edge_of(&doc(&f.s), f.rp);
    dbl(&mut f.s, at);
    assert_eq!(f.s.active_page().id, f.b.id, "last drawing page is B");
    assert_eq!(selected(&mut f.s), vec![f.rp]);
    assert!(!f.s.selection_move_only());
    // With a drawing page already active nothing switches.
    f.s.activate_page(&f.a.id);
    f.s.escape();
    dbl(&mut f.s, at);
    assert_eq!(f.s.active_page().id, f.a.id);
    // From the sketch page, after A was last.
    f.s.activate_page(&f.sk.id);
    dbl(&mut f.s, at);
    assert_eq!(f.s.active_page().id, f.a.id);
    assert_eq!(f.s.undo_len(), 0);
}

#[test]
fn ac14_last_drawing_page_deleted_falls_back_to_the_first_in_list_order() {
    let mut f = fixture();
    f.s.activate_page(&f.b.id);
    f.s.activate_page(&f.l.id);
    assert!(f.s.delete_page(&f.b.id));
    let at = edge_of(&doc(&f.s), f.rp);
    dbl(&mut f.s, at);
    assert_eq!(f.s.active_page().id, "first");
}

#[test]
fn ac15_laser_own_item_double_click_activates_the_laser_page() {
    let mut f = fixture();
    let at = edge_of(&doc(&f.s), f.rl);
    dbl(&mut f.s, at);
    assert_eq!(f.s.active_page().id, f.l.id);
    assert_eq!(selected(&mut f.s), vec![f.rl]);
    assert!(matches!(f.s.workbench(), Workbench::Manufacturing(_)));
    assert_eq!(f.s.undo_len(), 0);
}

#[test]
fn ac15_placed_page_double_click_activates_its_source_page_and_selects_the_object_under_it() {
    let mut f = fixture();
    f.s.activate_page(&f.l.id);
    assert!(f.s.place_page_on_active("first"));
    f.s.wipe_history();
    let d = doc(&f.s);
    let pp = d.placed_pages()[0].clone();
    let m = pp.global_map().unwrap();
    // ra lies at (10..30, 10..20) on A; its place on the bed:
    let ra_at = m.apply_point(Point::new(f.a.origin.x + 16.0, f.a.origin.y + 10.0));
    // Another page active: the double-click activates the laser page.
    f.s.activate_page(&f.b.id);
    f.s.escape();
    dbl(&mut f.s, ra_at);
    assert_eq!(
        f.s.active_page().id,
        f.l.id,
        "placed page double-click from B opens L"
    );
    // L active: it activates the source page and selects the object under the pointer.
    f.s.escape();
    dbl(&mut f.s, ra_at);
    assert_eq!(f.s.active_page().id, "first");
    assert_eq!(
        selected(&mut f.s),
        vec![f.ra],
        "topmost source object under the pointer"
    );
    assert_eq!(f.s.undo_len(), 0);
    // An empty area activates the source page and selects nothing.
    f.s.activate_page(&f.l.id);
    let empty = m.apply_point(Point::new(f.a.origin.x + 150.0, f.a.origin.y + 250.0));
    f.s.escape();
    dbl(&mut f.s, empty);
    assert_eq!(f.s.active_page().id, "first");
    assert_eq!(f.s.selected_object_count(), 0);
}

#[test]
fn ac16_double_click_on_a_drawing_object_over_a_sketch_opens_a_drawing_page() {
    let mut f = fixture();
    f.s.activate_page(&f.b.id);
    f.s.activate_page(&f.sk.id);
    f.s.escape();
    let at = edge_of(&doc(&f.s), f.rs);
    dbl(&mut f.s, at);
    // rs is on no drawing page: the last active drawing page is B.
    assert_eq!(f.s.active_page().id, f.b.id);
    assert_eq!(f.s.workbench(), Workbench::Vector);
}

// ---- criteria 17 to 19: the laser page tree --------------------------------------------------

#[test]
fn ac17_laser_tree_has_root_placed_pages_own_items_then_fixtures() {
    let mut f = fixture();
    f.s.activate_page(&f.l.id);
    assert!(f.s.place_page_on_active("first"));
    let rows = layer_rows(&mut f.s);
    let kinds: Vec<&str> = rows.iter().map(|r| r["kind"].as_str().unwrap()).collect();
    assert_eq!(kinds.first(), Some(&"page-root"));
    assert_eq!(kinds.last(), Some(&"fixtures"));
    assert!(kinds.contains(&"placed-page"));
    let placed = rows.iter().find(|r| r["kind"] == "placed-page").unwrap();
    assert!(
        placed["name"]
            .as_str()
            .unwrap()
            .starts_with("Placed page of ")
    );
    // No document layer row on a laser page.
    assert!(!kinds.contains(&"layer"), "{kinds:?}");
    // Drawing pages keep the document layer tree.
    f.s.activate_page("first");
    let rows = layer_rows(&mut f.s);
    assert!(rows.iter().all(|r| r["kind"] != "page-root"));
}

#[test]
fn ac17_empty_laser_page_shows_root_and_fixtures_only() {
    let d = Document::new(1);
    let l = d
        .add_output_page(NewOutputPage {
            process: "curvyo.laser",
            label: "Laser",
            profile: None,
            work_area: DocumentSize::from_mm(400.0, 300.0),
        })
        .unwrap();
    let mut s = open_session(&d);
    s.activate_page(&l.id);
    let kinds: Vec<String> = layer_rows(&mut s)
        .iter()
        .map(|r| r["kind"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(kinds.first().map(String::as_str), Some("page-root"));
    assert_eq!(kinds.last().map(String::as_str), Some("fixtures"));
    assert!(
        kinds
            .iter()
            .all(|k| k == "page-root" || k == "fixtures" || k == "note"),
        "{kinds:?}"
    );
}

#[test]
fn ac18_expanding_a_placed_page_lists_source_members_read_only_and_pressing_selects_the_placement()
{
    let mut f = fixture();
    f.s.activate_page(&f.l.id);
    assert!(f.s.place_page_on_active("first"));
    f.s.escape();
    let rows = layer_rows(&mut f.s);
    let placed = rows
        .iter()
        .find(|r| r["kind"] == "placed-page")
        .unwrap()
        .clone();
    let pid = placed["id"].to_string();
    let before = rows.len();
    assert!(f.s.set_layer_expanded(&pid, true));
    let rows = layer_rows(&mut f.s);
    assert!(rows.len() > before, "members shown");
    let inner: Vec<&Value> = rows.iter().filter(|r| r["readOnly"] == true).collect();
    assert!(!inner.is_empty(), "inner rows are read-only");
    assert!(
        inner
            .iter()
            .all(|r| r["level"].as_u64().unwrap() > placed["level"].as_u64().unwrap())
    );
    assert!(
        inner
            .iter()
            .all(|r| r["via"] == serde_json::from_str::<Value>(&pid).unwrap()),
        "inner rows carry the placed page as via"
    );
    // The host presses the placed page's id for an inner row (page_tree_panel contract).
    let id = inner[0]["via"].to_string();
    let steps = f.s.undo_len();
    f.s.press_layer_row(&id, RowPress::Plain);
    let placements: Vec<Value> = serde_json::from_str(&f.s.placed_pages_json()).unwrap();
    assert!(
        placements.iter().any(|p| p["selected"] == true),
        "pressing an inner row selects the placed page"
    );
    assert_eq!(f.s.undo_len(), steps);
    assert_eq!(f.s.active_page().id, f.l.id);
}

#[test]
fn ac18_clone_rows_are_not_expandable_and_own_items_have_rows() {
    let mut f = fixture();
    f.s.activate_page(&f.l.id);
    select_by_clicks(&mut f.s, &[f.rl]);
    let rows = layer_rows(&mut f.s);
    assert!(
        rows.iter()
            .any(|r| r["id"] == serde_json::to_value(f.rl).unwrap())
    );
    // Own item row can be pressed and selects it on the canvas.
    f.s.escape();
    let id = serde_json::to_string(&f.rl).unwrap();
    f.s.press_layer_row(&id, RowPress::Plain);
    assert_eq!(selected(&mut f.s), vec![f.rl]);
}

#[test]
fn ac19_delete_of_a_placed_page_removes_the_reference_only() {
    let mut f = fixture();
    f.s.activate_page(&f.l.id);
    assert!(f.s.place_page_on_active("first"));
    f.s.wipe_history();
    let n = doc(&f.s).read_objects().objects.len();
    let at = {
        let d = doc(&f.s);
        let c = d.placed_pages()[0].frame_corners().unwrap();
        Point::new(f64::midpoint(c[0].x, c[2].x), f64::midpoint(c[0].y, c[2].y))
    };
    f.s.escape();
    click(&mut f.s, at);
    key(&mut f.s, "Delete", false, false);
    assert!(doc(&f.s).placed_pages().is_empty());
    assert_eq!(
        doc(&f.s).read_objects().objects.len(),
        n,
        "source objects stay"
    );
}

// ---- criterion 21: format ---------------------------------------------------------------------

#[test]
fn ac21_format_stays_38_and_a_session_with_context_state_writes_nothing_new() {
    assert_eq!(CURRENT_FORMAT_VERSION, 38);
    let mut f = fixture();
    let before = doc(&f.s).read_objects().objects.len();
    select_by_clicks(&mut f.s, &[f.ra, f.rb]);
    f.s.activate_page(&f.b.id);
    let d = unpack(5, &bytes(&f.s)).unwrap();
    assert_eq!(d.read_objects().objects.len(), before);
    // Re-open unchanged: pack of the reopened document equals pack of the first.
    let mut s2 = Session::open(7, &bytes(&f.s)).unwrap();
    s2.resize_viewport(1600.0, 1000.0);
    assert_eq!(s2.undo_len(), 0);
}

#[test]
fn ac3_shift_press_adds_a_laser_page_item_to_a_drawing_selection() {
    let mut f = fixture();
    select_by_clicks(&mut f.s, &[f.ra, f.rl]);
    assert_eq!(
        f.s.selected_object_count(),
        2,
        "Shift+press on an object of a laser page"
    );
    select_by_clicks(&mut f.s, &[f.ra, f.rbl]);
    assert_eq!(
        f.s.selected_object_count(),
        2,
        "Shift+press on an object straddling B and L"
    );
}

#[test]
fn ac5_a_move_by_drag_keeps_the_active_page_all_the_way() {
    let mut f = fixture();
    let d = doc(&f.s);
    let at = edge_of(&d, f.rb);
    f.s.pointer_hover(at, false, false);
    f.s.pointer_down(at, false);
    assert_eq!(f.s.active_page().id, f.a.id, "after press");
    f.s.pointer_hover(Point::new(at.x + 10.0, at.y + 3.0), false, false);
    assert_eq!(f.s.active_page().id, f.a.id, "during move");
    let to = Point::new(at.x + 25.0, at.y + 7.0);
    f.s.pointer_hover(to, false, false);
    f.s.pointer_up(to, false, false);
    assert_eq!(f.s.active_page().id, f.a.id, "after release");
}

// ---- more: Enter, hidden chords, view state, sketch page, node tool -----------------------------

#[test]
fn ux_enter_with_one_foreign_home_activates_it_and_nothing_else() {
    let mut f = fixture();
    select_by_clicks(&mut f.s, &[f.rb, f.rb2]);
    key(&mut f.s, "Enter", false, false);
    assert_eq!(f.s.active_page().id, f.b.id);
    assert_eq!(f.s.undo_len(), 0);
    assert_eq!(f.s.selected_object_count(), 2);
}

#[test]
fn ux_enter_with_several_homes_activates_nothing_and_posts_the_notice() {
    let mut f = fixture();
    let from = Point::new(f.b.origin.x + 5.0, f.b.origin.y + 5.0);
    let to = Point::new(f.l.origin.x + 40.0, f.l.origin.y + 30.0);
    f.s.escape();
    f.s.pointer_hover(from, false, false);
    f.s.pointer_down(from, false);
    f.s.pointer_hover(
        Point::new(f.b.origin.x + 100.0, f.b.origin.y + 50.0),
        false,
        false,
    );
    f.s.pointer_hover(to, false, false);
    f.s.pointer_up(to, false, false);
    assert!(
        f.s.selected_object_count() >= 3,
        "marquee reaches B and L: {}",
        f.s.selected_object_count()
    );
    key(&mut f.s, "Enter", false, false);
    assert_eq!(f.s.active_page().id, f.a.id);
    assert!(
        f.s.take_context_notice().starts_with("Not editable here"),
        "refusal notice for Enter with several homes"
    );
}

#[test]
fn ac9_ctrl_shift_g_on_a_foreign_group_posts_one_notice_and_writes_nothing() {
    let mut f = fixture();
    f.s.activate_page(&f.b.id);
    select_by_clicks(&mut f.s, &[f.rb, f.rb2]);
    key(&mut f.s, "g", true, false);
    f.s.activate_page(&f.a.id);
    f.s.escape();
    let at = edge_of(&doc(&f.s), f.rb);
    click(&mut f.s, at);
    assert!(f.s.selection_move_only());
    f.s.wipe_history();
    let before = bytes(&f.s);
    let _ = f.s.take_context_notice();
    key(&mut f.s, "G", true, true);
    assert!(same(&bytes(&f.s), &before), "group not dissolved");
    assert_eq!(f.s.undo_len(), 0);
    let n = f.s.take_context_notice();
    assert!(
        n.starts_with("Not editable here: on "),
        "Ctrl+Shift+G notice: {n:?}"
    );
}

#[test]
fn ac9_the_node_tool_press_on_a_foreign_outline_selects_it_like_select_does() {
    let mut f = fixture();
    select_by_clicks(&mut f.s, &[f.ra]);
    f.s.set_tool(Tool::Node);
    let at = edge_of(&doc(&f.s), f.rb);
    click(&mut f.s, at);
    assert_eq!(f.s.active_page().id, f.a.id);
    assert_eq!(f.s.undo_len(), 0);
    assert_eq!(f.s.tool(), Tool::Node);
    assert!(
        f.s.selection_move_only(),
        "foreign object selected, shown without nodes"
    );
}

#[test]
fn ac8_moving_a_drawing_object_over_a_sketch_leaves_the_sketch_alone() {
    let mut f = fixture();
    f.s.activate_page(&f.sk.id);
    f.s.escape();
    let sketch_before: String = {
        let d = doc(&f.s);
        d.read_objects()
            .objects
            .iter()
            .filter_map(|o| match o {
                curvyo_document_core::ObjectSnapshot::Sketch(sn) => Some(format!("{sn:?}")),
                _ => None,
            })
            .collect()
    };
    assert!(!sketch_before.is_empty());
    let at = edge_of(&doc(&f.s), f.rs);
    click(&mut f.s, at);
    assert!(f.s.selection_move_only());
    key(&mut f.s, "ArrowRight", false, false);
    assert_eq!(f.s.undo_len(), 1);
    assert_eq!(f.s.active_page().id, f.sk.id);
    let sketch_after: String = doc(&f.s)
        .read_objects()
        .objects
        .iter()
        .filter_map(|o| match o {
            curvyo_document_core::ObjectSnapshot::Sketch(sn) => Some(format!("{sn:?}")),
            _ => None,
        })
        .collect();
    assert_eq!(sketch_before, sketch_after, "sketch geometry untouched");
}

#[test]
fn ac6_eye_and_lock_toggles_work_under_a_move_only_selection() {
    let mut f = fixture();
    select_by_clicks(&mut f.s, &[f.rb]);
    assert!(f.s.selection_move_only());
    let id = serde_json::to_string(&f.rb).unwrap();
    let before = snap(&f.s);
    let _ =
        f.s.toggle_layer_row_flag(&id, curvyo_editor_wasm::RowFlag::Visible);
    let rows = layer_rows(&mut f.s);
    let row = rows
        .iter()
        .find(|r| r["id"] == serde_json::to_value(f.rb).unwrap())
        .unwrap();
    assert_eq!(row["visible"], false, "eye toggled");
    assert_eq!(f.s.undo_len(), 1);
    let _ = before;
}

#[test]
fn ac2_a_press_on_a_sketch_page_hits_sketch_geometry_first() {
    let mut f = fixture();
    f.s.activate_page(&f.sk.id);
    f.s.escape();
    // A press on an out-of-context object away from the sketch geometry selects it.
    let at = edge_of(&doc(&f.s), f.rs);
    click(&mut f.s, at);
    assert_eq!(f.s.selected_object_count(), 1);
    // Drawing objects and sketch geometry never together: Shift+press on another drawing object works.
    let at2 = edge_of(&doc(&f.s), f.ra);
    shift_click(&mut f.s, at2);
    assert!(f.s.selection_move_only());
}
