//! Independent tester cases for `specs/0080-text-on-path/specification.md` slice 1 at the
//! `Session` level: the On path button's subjects (criterion 1), pick mode (2), the Select tool
//! route (3), Object to path equals the drawing (29), Duplicate and the Ctrl-copy move (31) and
//! the removal notice of a kept outline (25).
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
    Document, FontKind, NodeId, ObjectSnapshot, Point, TextSnapshot, Timestamp, unpack,
};
use curvyo_editor_wasm::{EscapeStep, KeyInput, ObjectToPathOutcome, Session, Tool};
use curvyo_text_core::FaceStore;
use curvyo_ui_core::{FontBlock, TextInput, lay_out};
use serde_json::Value;

static NOW: AtomicU64 = AtomicU64::new(1_995_000_000);
const TOL: f64 = 0.01;

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
    let mut s = Session::new(80);
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

/// `TextPathPress` is not exported from the crate root: compared by its Debug name.
fn press(s: &mut Session) -> String {
    format!("{:?}", s.text_path_press())
}

fn key(s: &mut Session, k: &str, ctrl: bool) {
    let _ = s.key_down(KeyInput {
        key: k,
        ctrl,
        ..KeyInput::default()
    });
}

fn doc_of(s: &Session) -> Document {
    unpack(77, &s.pack("0.1.0").unwrap()).unwrap()
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

/// A circle of radius 50 around (100, 100), drawn with the Ellipse tool; a text "Ring" at
/// (100, 260) committed and selected with the Select tool.
fn scene(s: &mut Session) -> NodeId {
    s.set_tool(Tool::Ellipse);
    drag(s, pt(50.0, 50.0), pt(150.0, 150.0));
    let circle = ellipses(s)[0];
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
    circle
}

/// A point on the circle's outline (right side), away from any text.
const ON_CIRCLE: Point = Point::new(150.0, 100.0);
const EMPTY: Point = Point::new(400.0, 400.0);

fn on_path(s: &Session) -> Option<TextSnapshot> {
    texts(s)
        .into_iter()
        .find(|t| t.on_path.as_ref().is_some_and(|p| p.curve.is_some()))
}

/// Criteria 1 and 2: with one text the button is enabled; pick mode starts; a click on empty
/// canvas changes nothing and stays; Escape ends and changes nothing; a click on the carrier puts
/// the text in one step and ends pick mode; the carrier is a guide.
#[test]
fn c1_c2_pick_mode_lifecycle() {
    let mut s = fresh();
    let circle = scene(&mut s);
    assert_eq!(s.text_path_view().texts, 1, "one subject text: enabled");
    let steps = s.history_len();

    assert_eq!(press(&mut s), "Picking");
    assert!(s.text_path_picking());
    click(&mut s, EMPTY);
    assert!(
        s.text_path_picking(),
        "a click on nothing stays in pick mode"
    );
    assert_eq!(s.history_len(), steps);
    s.escape();
    assert!(!s.text_path_picking(), "Escape ends");
    assert!(on_path(&s).is_none(), "and changes nothing");
    assert_eq!(s.history_len(), steps);

    assert_eq!(press(&mut s), "Picking");
    assert!(s.cancel_text_path_pick(), "right click ends");
    assert!(on_path(&s).is_none());
    assert_eq!(press(&mut s), "Picking");
    assert_eq!(press(&mut s), "Ended", "a second press ends");
    assert!(on_path(&s).is_none());

    assert_eq!(press(&mut s), "Picking");
    s.pointer_hover(ON_CIRCLE, false, false);
    click(&mut s, ON_CIRCLE);
    assert!(!s.text_path_picking(), "the put ends pick mode");
    let t = on_path(&s).expect("the text is on the circle");
    assert_eq!(t.on_path.as_ref().unwrap().carrier.node, circle);
    assert_eq!(s.history_len(), steps + 1, "one step");
    assert!(guide(&s, circle), "the carrier became a guide");
    assert_eq!(s.text_path_view().on_path, 1, "the On path groups show");
    s.undo();
    assert!(on_path(&s).is_none(), "Undo gives back the straight text");
    assert!(!guide(&s, circle));
}

/// Criterion 1: with two texts the button is disabled (the view counts two subjects) and a press
/// does nothing.
#[test]
fn c1_two_texts_disable_the_button() {
    let mut s = fresh();
    let _ = scene(&mut s);
    s.set_tool(Tool::Text);
    click(&mut s, pt(100.0, 320.0));
    s.text_input(TextInput::Insert("Two".to_owned()));
    load(&mut s);
    s.escape();
    s.set_tool(Tool::Select);
    key(&mut s, "a", true);
    let n = s.selected_object_count();
    assert!(n >= 2);
    let view = s.text_path_view();
    assert_eq!(view.texts, 2);
    assert_eq!(press(&mut s), "Ignored");
    assert!(!s.text_path_picking());
}

/// Criterion 3: text and carrier selected with the Select tool: On path puts at once.
#[test]
fn c3_select_tool_route_puts_without_pick_mode() {
    let mut s = fresh();
    let circle = scene(&mut s);
    shift_click(&mut s, ON_CIRCLE);
    assert_eq!(s.selected_object_count(), 2);
    assert_eq!(press(&mut s), "Put");
    assert!(!s.text_path_picking());
    let t = on_path(&s).unwrap();
    assert_eq!(t.on_path.unwrap().carrier.node, circle);
}

fn put_on_circle(s: &mut Session) -> NodeId {
    let circle = scene(s);
    assert_eq!(press(s), "Picking");
    click(s, ON_CIRCLE);
    assert!(on_path(s).is_some());
    circle
}

/// Criterion 29: Object to path gives one path per letter at the drawn outlines; the carrier
/// stays a guide (criterion 4a).
#[test]
fn c29_object_to_path_equals_the_drawing() {
    let mut s = fresh();
    let circle = put_on_circle(&mut s);
    let t = on_path(&s).unwrap();
    let bytes = std::fs::read(fonts_dir().join("source-sans-3/SourceSans3-Regular.otf")).unwrap();
    let mut store = FaceStore::new();
    let key_ = store.add(bytes, 0).unwrap();
    let face = store.get(&key_).unwrap();
    let drawn = lay_out(&t, face, false).unwrap();
    let want: Vec<Point> = std::iter::once(&drawn.glyphs.anchors)
        .chain(drawn.glyphs.extra_subpaths.iter().map(|p| &p.anchors))
        .flat_map(|a| a.iter().map(|a| a.point))
        .collect();
    assert_eq!(s.object_to_path(), ObjectToPathOutcome::Converted);
    let d = doc_of(&s);
    let paths: Vec<Vec<Point>> = d
        .object_ids()
        .into_iter()
        .filter_map(|id| match d.object(id) {
            Some(ObjectSnapshot::Path(p)) => Some(p.all_anchors().map(|a| a.point).collect()),
            _ => None,
        })
        .collect();
    assert_eq!(paths.len(), 4, "one path per letter of Ring");
    let got = paths.concat();
    assert_eq!(got.len(), want.len());
    for p in got {
        assert!(
            want.iter()
                .any(|w| (w.x - p.x).abs() < TOL && (w.y - p.y).abs() < TOL),
            "converted anchor {p:?} is not on the drawn outline"
        );
    }
    assert!(guide(&s, circle), "the carrier stays a guide");
}

/// Criterion 31: Duplicate of a text on path alone: the copy is on the same carrier with
/// Position + 5 mm.
#[test]
fn c31_duplicate_shifts_five_mm_along_the_carrier() {
    let mut s = fresh();
    let circle = put_on_circle(&mut s);
    let before = on_path(&s)
        .unwrap()
        .on_path
        .unwrap()
        .values
        .position
        .as_mm();
    s.set_tool(Tool::Select);
    click(&mut s, pt(100.0, 46.0));
    assert_eq!(s.selected_object_count(), 1);
    let _ = s.duplicate_layer_target();
    let all: Vec<TextSnapshot> = texts(&s);
    assert_eq!(all.len(), 2, "a copy");
    let mut positions: Vec<f64> = all
        .iter()
        .map(|t| {
            let p = t.on_path.as_ref().expect("both on path");
            assert_eq!(p.carrier.node, circle);
            p.values.position.as_mm()
        })
        .collect();
    positions.sort_by(f64::total_cmp);
    let len = 2.0 * std::f64::consts::PI * 50.0;
    let shift = (positions[1] - positions[0]).rem_euclid(len);
    assert!(
        (shift - 5.0).abs() < 0.02 || (len - shift - 5.0).abs() < 0.02,
        "copy 5 mm along: {positions:?}, original {before}"
    );
}

/// Criterion 31: a Ctrl-copy move of a text on path alone shifts the copy by the moved distance
/// projected on the carrier (here a drag of 20 mm along the circle's top, where the carrier
/// runs in +x).
#[test]
fn c31_ctrl_copy_move_shifts_by_the_projected_drag() {
    let mut s = fresh();
    let _ = put_on_circle(&mut s);
    let t = on_path(&s).unwrap();
    let p0 = t.on_path.as_ref().unwrap().values.position.as_mm();
    // The text sits centred at the top (criterion 5): grab its body just above (100, 50).
    let grab = pt(100.0, 46.0);
    s.set_tool(Tool::Select);
    click(&mut s, grab);
    s.modifiers_changed(false, true, false);
    s.pointer_hover(grab, false, true);
    s.pointer_down(grab, false);
    for i in 1..=10 {
        s.pointer_hover(pt(100.0 + f64::from(i) * 2.0, 46.0), false, true);
    }
    s.pointer_up(pt(120.0, 46.0), false, true);
    s.modifiers_changed(false, false, false);
    let all = texts(&s);
    assert_eq!(all.len(), 2, "Ctrl-move copies");
    let copy = all.iter().find(|x| x.id != t.id).unwrap();
    let p1 = copy.on_path.as_ref().unwrap().values.position.as_mm();
    let orig = all.iter().find(|x| x.id == t.id).unwrap();
    assert!(
        (orig.on_path.as_ref().unwrap().values.position.as_mm() - p0).abs() < 1e-6,
        "the original stays"
    );
    // 20 mm of drag projected on a circle of radius 50 near the top: about 20 mm of arc.
    assert!(
        (p1 - p0 - 20.0).abs() < 2.0,
        "copy moved by the projected drag (~20 mm), got {}",
        p1 - p0
    );
}

/// Criterion 25: deleting the carrier keeps the outline; the notice says "1 text kept its path."
#[test]
fn c25_delete_carrier_notice() {
    let mut s = fresh();
    let circle = put_on_circle(&mut s);
    s.set_tool(Tool::Select);
    click(&mut s, EMPTY);
    click(&mut s, ON_CIRCLE);
    assert_eq!(s.selected_object_count(), 1);
    key(&mut s, "Delete", false);
    assert!(!ellipses(&s).contains(&circle), "carrier removed");
    let notice = s.take_clone_repair_notice();
    assert!(
        notice.contains("1 text kept its path."),
        "notice: {notice:?}"
    );
    let t = on_path(&s).expect("still drawn on the kept outline");
    assert!(t.on_path.unwrap().kept);
}

/// Criterion 21: a move drag of the text's body slides it along the carrier: a 1 mm drag along
/// the top of the circle moves Position by about 1 mm, with no jump at the grab.
#[test]
fn c21_body_drag_slides_without_a_jump() {
    let mut s = fresh();
    let _ = put_on_circle(&mut s);
    let p0 = on_path(&s)
        .unwrap()
        .on_path
        .unwrap()
        .values
        .position
        .as_mm();
    let grab = pt(100.0, 46.0);
    s.set_tool(Tool::Select);
    click(&mut s, grab);
    s.pointer_hover(grab, false, false);
    s.pointer_down(grab, false);
    s.pointer_hover(pt(100.5, 46.0), false, false);
    s.pointer_hover(pt(101.0, 46.0), false, false);
    s.pointer_up(pt(101.0, 46.0), false, false);
    let t = on_path(&s).expect("still on the carrier");
    let p1 = t.on_path.unwrap().values.position.as_mm();
    assert!(
        (p1 - p0 - 1.0).abs() < 0.3,
        "1 mm drag moved Position by {} mm",
        p1 - p0
    );
}
