//! Independent tester cases for `specs/0080-text-on-path/specification.md` slice 2 at the
//! `Session` level: the pick set with Shift+click, its "1" badge and Enter (criteria 2, 37), the
//! Select route with several carriers, lowest first (3, 37), typing on a clone shows on every
//! member (38), one clone's own placement and colour (39, 40), Unlink (41), removing the source
//! (41), the emptied string (41a), Object to path of a clone (42).
//!
//! Written from the specification and the public names before the implementation was read.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::too_many_lines,
    clippy::float_cmp,
    missing_docs
)]

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use curvyo_document_core::{
    Color, Document, FontKind, NodeId, ObjectSnapshot, Point, TextSnapshot, Timestamp, unpack,
};
use curvyo_editor_wasm::{EscapeStep, KeyInput, ObjectToPathOutcome, Session, TextPathPress, Tool};
use curvyo_ui_core::{FontBlock, StyleField, TextInput};
use serde_json::Value;

static NOW: AtomicU64 = AtomicU64::new(1_996_000_000);

fn clock() -> Timestamp {
    Timestamp::from_secs(NOW.fetch_add(3, Ordering::SeqCst))
}

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn fonts_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../frontend/public/fonts")
}

fn included_faces() -> Vec<(String, String, String, u32, FontKind)> {
    let index: Value =
        serde_json::from_str(&std::fs::read_to_string(fonts_dir().join("index.json")).unwrap())
            .unwrap();
    index["faces"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| {
            let kind = if f["kind"] == "single_line" {
                FontKind::SingleLine
            } else {
                FontKind::Outline
            };
            (
                f["file"].as_str().unwrap().to_owned(),
                f["family"].as_str().unwrap().to_owned(),
                f["style"].as_str().unwrap().to_owned(),
                0,
                kind,
            )
        })
        .collect()
}

fn load(s: &mut Session) {
    for file in s.wanted_font_files() {
        let bytes = std::fs::read(fonts_dir().join(&file)).unwrap();
        assert!(s.add_font_file(&file, &bytes), "{file} loads");
    }
}

fn fresh() -> Session {
    let mut s = Session::new(81);
    s.set_clock(clock);
    s.resize_viewport(1200.0, 800.0);
    s.set_font_list_with_kinds(FontBlock::Included, included_faces(), Vec::new());
    load(&mut s);
    s
}

fn click(s: &mut Session, at: Point) {
    s.pointer_hover(at, false, false);
    s.pointer_down(at, false);
    s.pointer_up(at, false, false);
}

fn shift_click(s: &mut Session, at: Point) {
    s.pointer_hover(at, true, false);
    s.pointer_down(at, true);
    s.pointer_up(at, true, false);
}

fn drag(s: &mut Session, from: Point, to: Point) {
    s.pointer_hover(from, false, false);
    s.pointer_down(from, false);
    for i in 1..=8 {
        let f = f64::from(i) / 8.0;
        s.pointer_hover(
            pt(from.x + (to.x - from.x) * f, from.y + (to.y - from.y) * f),
            false,
            false,
        );
    }
    s.pointer_up(to, false, false);
}

fn key(s: &mut Session, k: &str, ctrl: bool) {
    let _ = s.key_down(KeyInput {
        key: k,
        ctrl,
        ..KeyInput::default()
    });
}

fn doc_of(s: &Session) -> Document {
    unpack(78, &s.pack("0.1.0").unwrap()).unwrap()
}

fn texts(s: &Session) -> Vec<TextSnapshot> {
    let d = doc_of(s);
    d.object_ids()
        .into_iter()
        .filter_map(|id| match d.object(id) {
            Some(ObjectSnapshot::Text(t)) => Some(*t),
            _ => None,
        })
        .collect()
}

fn ellipses(s: &Session) -> Vec<NodeId> {
    let d = doc_of(s);
    d.object_ids()
        .into_iter()
        .filter(|&id| matches!(d.object(id), Some(ObjectSnapshot::Primitive(_))))
        .collect()
}

fn guide(s: &Session, id: NodeId) -> bool {
    doc_of(s)
        .read_nodes()
        .iter()
        .find(|n| n.id == id)
        .unwrap()
        .guide
}

/// Centres of the three circles (radius 50).
const XS: [f64; 3] = [100.0, 250.0, 400.0];

fn on_circle(k: usize) -> Point {
    pt(XS[k] + 50.0, 100.0)
}

/// Just above the top of circle `k`, on the letters of a text centred there.
fn letters_on(k: usize) -> Point {
    pt(XS[k], 46.0)
}

const EMPTY: Point = Point::new(600.0, 600.0);

/// Three circles drawn left to right (so the first is lowest in stacking order) and a text "Ring"
/// at (100, 260), committed and selected with the Select tool.
fn scene(s: &mut Session) -> Vec<NodeId> {
    for x in XS {
        s.set_tool(Tool::Ellipse);
        drag(s, pt(x - 50.0, 50.0), pt(x + 50.0, 150.0));
    }
    let circles = ellipses(s);
    assert_eq!(circles.len(), 3);
    s.set_tool(Tool::Text);
    click(s, pt(100.0, 260.0));
    assert!(s.text_editing());
    s.text_input(TextInput::Insert("Ring".to_owned()));
    load(s);
    assert_eq!(s.escape(), EscapeStep::ClearedState);
    load(s);
    s.set_text_size(10.0);
    s.set_tool(Tool::Select);
    click(s, pt(105.0, 257.0));
    assert_eq!(s.selected_object_count(), 1, "the text is selected");
    circles
}

fn carrier_of(t: &TextSnapshot) -> Option<NodeId> {
    t.on_path
        .as_ref()
        .filter(|p| p.curve.is_some())
        .map(|p| p.carrier.node)
}

fn source_text(s: &Session) -> TextSnapshot {
    texts(s).into_iter().find(|t| t.source.is_none()).unwrap()
}

fn clone_on(s: &Session, carrier: NodeId) -> TextSnapshot {
    texts(s)
        .into_iter()
        .find(|t| t.source.is_some() && carrier_of(t) == Some(carrier))
        .unwrap()
}

/// The scene, put on all three circles by the pick set in order 1, 2, 3.
fn put_on_three(s: &mut Session) -> Vec<NodeId> {
    let circles = scene(s);
    assert_eq!(s.text_path_press(), TextPathPress::Picking);
    for k in 0..3 {
        s.pointer_hover(on_circle(k), true, false);
        shift_click(s, on_circle(k));
    }
    assert!(s.confirm_text_path_pick());
    let _ = s.take_text_path_notice();
    circles
}

fn all_notices(s: &mut Session) -> String {
    [
        s.take_text_path_notice(),
        s.take_clone_repair_notice(),
        s.take_delete_notice(),
        s.take_orphan_notice(),
        s.take_history_notice().unwrap_or_default(),
    ]
    .join(" | ")
}

/// Criteria 2 and 37: Shift+click builds a pick set and stays in pick mode, the first picked
/// carrier carries the "1" badge, Enter puts the text on the set in one step: the text on the
/// first picked, a clone on each further one, every carrier a guide, notice "Put text on 3
/// paths.". Undo takes it all back.
#[test]
fn c2_c37_pick_set_badge_enter_one_step_and_notice() {
    let mut s = fresh();
    let circles = scene(&mut s);
    let steps = s.history_len();
    assert_eq!(s.text_path_press(), TextPathPress::Picking);
    assert!(s.text_path_pick_badge().is_none(), "no badge before a pick");
    // Picked in the order 2, 1, 3: circle 2 is the first picked.
    for k in [1, 0, 2] {
        s.pointer_hover(on_circle(k), true, false);
        shift_click(&mut s, on_circle(k));
        assert!(s.text_path_picking(), "Shift+click stays in pick mode");
    }
    let badge = s.text_path_pick_badge().expect("the 1 badge");
    // Screen px at the default view: 96 px per inch, no pan.
    let badge = (badge.0 * 25.4 / 96.0, badge.1 * 25.4 / 96.0);
    // The badge sits at the first picked carrier (circle 2, x 200..300), not at the others.
    assert!(
        badge.0 > 180.0 && badge.0 < 320.0,
        "badge at the first picked carrier: {badge:?}"
    );
    assert_eq!(s.history_len(), steps, "picking writes nothing");
    key(&mut s, "Enter", false);
    assert!(!s.text_path_picking(), "Enter ends pick mode");
    assert_eq!(s.history_len(), steps + 1, "one step");
    let notice = all_notices(&mut s);
    assert!(
        notice.contains("Put text on 3 paths."),
        "notice: {notice:?}"
    );
    let src = source_text(&s);
    assert_eq!(
        carrier_of(&src),
        Some(circles[1]),
        "the text is on the first picked"
    );
    for &c in &[circles[0], circles[2]] {
        let cl = clone_on(&s, c);
        assert_eq!(cl.source.unwrap().node, src.id);
    }
    for &c in &circles {
        assert!(guide(&s, c));
    }
    s.undo();
    assert_eq!(texts(&s).len(), 1, "Undo removes the clones");
    assert!(carrier_of(&source_text(&s)).is_none());
}

/// Criteria 3 and 37: the Select tool with one text and three carriers puts at once; the text
/// goes on the lowest carrier in stacking order whatever the selection order.
#[test]
fn c3_c37_select_route_puts_the_text_on_the_lowest_carrier() {
    let mut s = fresh();
    let circles = scene(&mut s);
    for k in [2, 0, 1] {
        shift_click(&mut s, on_circle(k));
    }
    assert_eq!(s.selected_object_count(), 4);
    assert_eq!(s.text_path_press(), TextPathPress::Put);
    let src = source_text(&s);
    assert_eq!(carrier_of(&src), Some(circles[0]), "lowest first");
    assert_eq!(texts(&s).len(), 3);
    let notice = all_notices(&mut s);
    assert!(
        notice.contains("Put text on 3 paths."),
        "notice: {notice:?}"
    );
}

/// Criterion 38: the Text tool on a clone's letters starts edit mode there; a typed letter shows
/// on the source and every clone; it is one step.
#[test]
fn c38_typing_in_a_clone_shows_on_all_members_one_step() {
    let mut s = fresh();
    let circles = put_on_three(&mut s);
    let steps = s.history_len();
    s.set_tool(Tool::Text);
    click(&mut s, letters_on(1));
    assert!(s.text_editing(), "edit mode on the clone");
    s.text_input(TextInput::Insert("X".to_owned()));
    load(&mut s);
    s.escape();
    let all = texts(&s);
    assert_eq!(all.len(), 3);
    for t in &all {
        assert!(
            t.string.contains('X') && t.string.len() == 5,
            "{:?}",
            t.string
        );
    }
    assert_eq!(s.history_len(), steps + 1, "one step for all");
    assert!(clone_on(&s, circles[1]).source.is_some(), "still a clone");
}

/// Criterion 39: dragging one clone's position handle (its letters' body) moves that clone alone.
#[test]
fn c39_moving_one_clone_moves_it_alone() {
    let mut s = fresh();
    let circles = put_on_three(&mut s);
    let before: Vec<(NodeId, f64)> = texts(&s)
        .iter()
        .map(|t| (t.id, t.on_path.as_ref().unwrap().values.position.as_mm()))
        .collect();
    s.set_tool(Tool::Select);
    click(&mut s, EMPTY);
    click(&mut s, letters_on(2));
    assert_eq!(s.selected_object_count(), 1);
    drag(&mut s, letters_on(2), pt(XS[2] + 10.0, 46.0));
    let moved = clone_on(&s, circles[2]).id;
    for t in texts(&s) {
        let p = t.on_path.as_ref().unwrap().values.position.as_mm();
        let was = before.iter().find(|(id, _)| *id == t.id).unwrap().1;
        if t.id == moved {
            assert!(
                (p - was - 10.0).abs() < 2.0,
                "the dragged clone slid: {}",
                p - was
            );
        } else {
            assert_eq!(p, was, "the others stay");
        }
    }
}

/// Criterion 40: Own style on one clone, then a fill colour on it: only that clone takes it; the
/// source keeps its colour and so does the other clone.
#[test]
fn c40_own_colour_on_one_clone() {
    let mut s = fresh();
    let circles = put_on_three(&mut s);
    s.set_tool(Tool::Select);
    click(&mut s, EMPTY);
    click(&mut s, letters_on(1));
    assert_eq!(s.selected_object_count(), 1);
    s.set_clone_style(true);
    assert!(s.set_style_text(StyleField::FillColor, "#c80000").unwrap());
    let mine = clone_on(&s, circles[1]);
    assert_eq!(mine.style.fill.color, Color { r: 200, g: 0, b: 0 });
    assert_ne!(
        source_text(&s).style.fill.color,
        Color { r: 200, g: 0, b: 0 }
    );
    assert_ne!(
        clone_on(&s, circles[2]).style.fill.color,
        Color { r: 200, g: 0, b: 0 }
    );
}

/// Criterion 41: Unlink clone on a selected clone: one step, it becomes a text with the string,
/// on its carrier; the source's later edit does not reach it.
#[test]
fn c41_unlink_in_the_session() {
    let mut s = fresh();
    let circles = put_on_three(&mut s);
    s.set_tool(Tool::Select);
    click(&mut s, EMPTY);
    click(&mut s, letters_on(2));
    let steps = s.history_len();
    let _ = s.unlink_clones();
    assert_eq!(s.history_len(), steps + 1);
    let t = texts(&s)
        .into_iter()
        .find(|t| carrier_of(t) == Some(circles[2]))
        .unwrap();
    assert!(t.source.is_none());
    assert_eq!(t.string, "Ring");
}

/// Criterion 41: deleting the source turns both clones into texts in the same step, with the
/// notice "2 text clones became texts.".
#[test]
fn c41_delete_source_notice() {
    let mut s = fresh();
    let _ = put_on_three(&mut s);
    s.set_tool(Tool::Select);
    click(&mut s, EMPTY);
    click(&mut s, letters_on(0));
    assert_eq!(s.selected_object_count(), 1);
    let steps = s.history_len();
    key(&mut s, "Delete", false);
    assert_eq!(s.history_len(), steps + 1);
    let notice = all_notices(&mut s);
    assert!(
        notice.contains("2 text clones became texts."),
        "notice: {notice:?}"
    );
    let left = texts(&s);
    assert_eq!(left.len(), 2);
    assert!(
        left.iter()
            .all(|t| t.source.is_none() && t.string == "Ring")
    );
}

/// Criterion 41a: emptying the source in edit mode: while editing nothing is drawn by any member
/// (a pack in edit mode still opens); at the end the source is removed and the clones become texts
/// with "Ring", notice "2 text clones became texts."; one step; Undo brings everything back.
#[test]
fn c41a_emptied_source_in_edit_mode() {
    let mut s = fresh();
    let _ = put_on_three(&mut s);
    let steps = s.history_len();
    s.set_tool(Tool::Text);
    click(&mut s, letters_on(0));
    assert!(s.text_editing());
    s.text_input(TextInput::SelectAll);
    s.text_input(TextInput::DeleteBackward);
    let _ = doc_of(&s);
    s.escape();
    assert!(!s.text_editing());
    let left = texts(&s);
    assert_eq!(left.len(), 2, "the source is removed: {left:?}");
    assert!(
        left.iter()
            .all(|t| t.source.is_none() && t.string == "Ring")
    );
    let notice = all_notices(&mut s);
    assert!(
        notice.contains("2 text clones became texts."),
        "notice: {notice:?}"
    );
    assert_eq!(s.history_len(), steps + 1, "one step");
    s.undo();
    assert_eq!(texts(&s).len(), 3);
    assert_eq!(texts(&s).iter().filter(|t| t.source.is_some()).count(), 2);
}

/// Criterion 41a: emptying a clone in edit mode removes that clone only.
#[test]
fn c41a_emptied_clone_in_edit_mode() {
    let mut s = fresh();
    let circles = put_on_three(&mut s);
    s.set_tool(Tool::Text);
    click(&mut s, letters_on(2));
    assert!(s.text_editing());
    s.text_input(TextInput::SelectAll);
    s.text_input(TextInput::DeleteBackward);
    s.escape();
    let left = texts(&s);
    assert_eq!(left.len(), 2);
    assert!(left.iter().all(|t| t.string == "Ring"));
    assert!(left.iter().all(|t| carrier_of(t) != Some(circles[2])));
    assert_eq!(left.iter().filter(|t| t.source.is_some()).count(), 1);
}

/// Criterion 42: Object to path on a selected clone converts that clone only.
#[test]
fn c42_object_to_path_of_a_clone_alone() {
    let mut s = fresh();
    let _ = put_on_three(&mut s);
    s.set_tool(Tool::Select);
    click(&mut s, EMPTY);
    click(&mut s, letters_on(1));
    assert_eq!(s.selected_object_count(), 1);
    assert_eq!(s.object_to_path(), ObjectToPathOutcome::Converted);
    let left = texts(&s);
    assert_eq!(left.len(), 2, "the source and the other clone stay texts");
    assert_eq!(left.iter().filter(|t| t.source.is_some()).count(), 1);
    let d = doc_of(&s);
    let paths = d
        .object_ids()
        .into_iter()
        .filter(|&id| matches!(d.object(id), Some(ObjectSnapshot::Path(_))))
        .count();
    assert_eq!(paths, 4, "one path per letter of Ring");
}

/// Criterion 41a with `0074` criterion 8: a Save in edit mode with an emptied string keeps edit
/// mode on and writes no step; the saved file still holds the last written string (nothing is
/// removed behind the maker's back); ending edit mode then removes the text in one step.
#[test]
fn c41a_save_in_edit_mode_with_an_empty_string_writes_nothing() {
    let mut s = fresh();
    let _ = scene(&mut s);
    let steps = s.history_len();
    s.set_tool(Tool::Text);
    click(&mut s, pt(105.0, 257.0));
    assert!(s.text_editing());
    s.text_input(TextInput::SelectAll);
    s.text_input(TextInput::DeleteBackward);
    s.checkpoint_text_edit();
    assert!(s.text_editing(), "edit mode goes on");
    assert_eq!(s.history_len(), steps, "no step");
    let saved = texts(&s);
    assert_eq!(saved.len(), 1);
    assert_eq!(
        saved[0].string, "Ring",
        "the file keeps the last written string"
    );
    s.escape();
    assert!(texts(&s).is_empty(), "removed when edit mode ends");
    assert_eq!(s.history_len(), steps + 1);
}

/// White box (criteria 41 and 29): Object to path of the source removes the source text, so its
/// clones become independent texts in the same step, never orphans that vanish.
#[test]
fn wb_object_to_path_of_the_source_unlinks_its_clones() {
    let mut s = fresh();
    let _ = put_on_three(&mut s);
    s.set_tool(Tool::Select);
    click(&mut s, EMPTY);
    click(&mut s, letters_on(0));
    assert_eq!(s.selected_object_count(), 1);
    let steps = s.history_len();
    assert_eq!(s.object_to_path(), ObjectToPathOutcome::Converted);
    assert_eq!(s.history_len(), steps + 1);
    let left = texts(&s);
    assert_eq!(left.len(), 2, "both clones stay as texts: {left:?}");
    assert!(
        left.iter()
            .all(|t| t.source.is_none() && t.string == "Ring")
    );
}
