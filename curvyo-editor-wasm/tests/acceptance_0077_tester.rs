//! Independent tester cases for `specs/0077-font-preview/specification.md` at the `Session` and
//! glue level: criteria 8 and 9 (the sample follows the one selected text and its first line,
//! `adrs.md` decision 5) and 13 (the tile call draws one line; rewritten for the customer decision
//! of 2026-10-23).
//!
//! Written from the specification, `adrs.md` and the public names before the implementation was
//! read.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::cast_precision_loss,
    clippy::assert_is_empty,
    missing_docs
)]

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use curvyo_document_core::{Point, Timestamp};
use curvyo_editor_wasm::{EscapeStep, PreviewFacesGlue, Session, Tool};
use curvyo_ui_core::{FontBlock, TextInput};
use serde_json::Value;

static NOW: AtomicU64 = AtomicU64::new(1_960_000_000);

fn clock() -> Timestamp {
    Timestamp::from_secs(NOW.fetch_add(3, Ordering::SeqCst))
}

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn fonts_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../frontend/public/fonts")
}

fn included_faces() -> Vec<(String, String, String, u32)> {
    let index: Value =
        serde_json::from_str(&std::fs::read_to_string(fonts_dir().join("index.json")).unwrap())
            .unwrap();
    index["faces"]
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
        .collect()
}

fn load(s: &mut Session) {
    for file in s.wanted_font_files() {
        let bytes = std::fs::read(fonts_dir().join(&file)).unwrap();
        assert!(s.add_font_file(&file, &bytes), "{file} loads");
    }
}

fn fresh() -> Session {
    let mut s = Session::new(77);
    s.set_clock(clock);
    s.resize_viewport(1200.0, 800.0);
    s.set_font_list(FontBlock::Included, included_faces(), Vec::new());
    load(&mut s);
    s
}

fn click(s: &mut Session, at: Point) {
    s.pointer_hover(at, false, false);
    s.pointer_down(at, false);
    s.pointer_up(at, false, false);
}

fn type_text(s: &mut Session, text: &str) {
    s.text_input(TextInput::Insert(text.to_owned()));
    load(s);
}

fn add_text(s: &mut Session, at: Point, string: &str) {
    s.set_tool(Tool::Text);
    click(s, at);
    assert!(s.text_editing(), "a click starts edit mode");
    type_text(s, string);
    assert_eq!(s.escape(), EscapeStep::ClearedState);
    load(s);
}

fn select_at(s: &mut Session, at: Point) {
    s.set_tool(Tool::Select);
    click(s, at);
}

// --- Criteria 8 and 9 ------------------------------------------------------------------------

/// Criterion 9 with `adrs.md` decision 5: selecting another text with the same first line is a
/// selection change the preview sees (the node differs), so a typed sample is replaced again.
#[test]
fn ac09_a_second_text_with_the_same_line_is_a_new_sample_source() {
    let mut s = fresh();
    add_text(&mut s, pt(10.0, 20.0), "Anna & Jonas");
    add_text(&mut s, pt(10.0, 200.0), "Anna & Jonas");
    select_at(&mut s, pt(14.0, 16.0));
    let first = s.text_bar();
    assert_eq!(first.sample, "Anna & Jonas");
    assert!(!first.sample_node.is_empty());
    select_at(&mut s, pt(14.0, 196.0));
    let second = s.text_bar();
    assert_eq!(second.sample, "Anna & Jonas");
    assert!(!second.sample_node.is_empty());
    assert_ne!(first.sample_node, second.sample_node, "another node");
}

/// Criterion 9: while the text stays selected, a change on the canvas reaches the sample, and an
/// undo brings the old first line back.
#[test]
fn ac09_canvas_edits_and_undo_reach_the_sample() {
    let mut s = fresh();
    add_text(&mut s, pt(10.0, 20.0), "Anna\nJonas");
    select_at(&mut s, pt(14.0, 16.0));
    assert_eq!(s.text_bar().sample, "Anna");
    let node = s.text_bar().sample_node;
    s.set_tool(Tool::Text);
    click(&mut s, pt(14.0, 16.0));
    assert!(s.text_editing());
    s.text_input(TextInput::SelectAll);
    type_text(&mut s, "Ben\nJonas");
    assert_eq!(s.text_bar().sample, "Ben", "the edit is followed");
    assert_eq!(s.text_bar().sample_node, node, "the same node");
    assert_eq!(s.escape(), EscapeStep::ClearedState);
    let _ = s.undo();
    load(&mut s);
    select_at(&mut s, pt(14.0, 16.0));
    let bar = s.text_bar();
    assert_eq!(bar.sample_node, node);
    assert_eq!(bar.sample, "Anna", "undo restores the first line");
}

/// Criterion 8: a text object with an empty first line gives an empty sample (the tiles then
/// show family names, criterion 11), not the second line.
#[test]
fn ac08_an_empty_first_line_is_an_empty_sample() {
    let mut s = fresh();
    add_text(&mut s, pt(10.0, 20.0), "\nJonas");
    s.set_tool(Tool::Select);
    s.pointer_hover(pt(0.0, -40.0), false, false);
    s.pointer_down(pt(0.0, -40.0), false);
    s.pointer_hover(pt(100.0, 100.0), false, false);
    s.pointer_hover(pt(300.0, 150.0), false, false);
    s.pointer_up(pt(300.0, 150.0), false, false);
    let bar = s.text_bar();
    assert!(!bar.sample_node.is_empty(), "the text is selected");
    assert_eq!(bar.sample, "");
}

// --- Tiles through the glue (criteria 13, 15, 18) --------------------------------------------

fn glue_with(files: &[&str]) -> PreviewFacesGlue {
    let mut glue = PreviewFacesGlue::default();
    for file in files {
        let bytes = std::fs::read(fonts_dir().join(file)).unwrap();
        assert!(glue.add(file, &bytes, 0), "{file}");
    }
    glue
}

fn tile(glue: &mut PreviewFacesGlue, id: &str, text: &str) -> Value {
    serde_json::from_str(&glue.tile(id, text)).unwrap()
}

/// Criterion 13 (customer decision 2026-10-23): a tile's sample is one line, never wrapped and
/// never ended with an ellipsis: a sample ten times as long is drawn one line high and about ten
/// times as wide, for the tile to clip at its edge.
#[test]
fn ac13_tile_sample_is_one_line_however_long() {
    let id = "source-sans-3/SourceSans3-Regular.otf";
    let mut glue = glue_with(&[id]);
    let one = tile(&mut glue, id, "Anna & Jonas 2026");
    let long = tile(&mut glue, id, &"Anna & Jonas 2026 ".repeat(10));
    let height = |t: &Value| t["box"][3].as_f64().unwrap();
    let width = |t: &Value| t["box"][2].as_f64().unwrap();
    assert!((height(&long) - height(&one)).abs() < 1e-9, "one line high");
    assert!(
        width(&long) > 9.0 * width(&one),
        "the whole line, not cut by the engine"
    );
}
