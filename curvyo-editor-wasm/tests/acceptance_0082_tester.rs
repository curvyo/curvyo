//! Independent tester cases for `specs/0082-glyph-table/specification.md` at the `Session` and
//! glue level: criteria 4 and 11 (the subject face, a missing face), 9 (the count is the cells),
//! 12 to 14 (`insert_from_glyph_table` in its three cases, beyond the implementer's unit tests:
//! a selection inside the text, in-session undo per character, the 10,000 limit, a text in
//! another face than the next text's, other objects selected, the next text's face and size,
//! the centre in document coordinates), 16 (the glue's query cut) and 19 (the detail bar's text).
//!
//! Written from the specification, `adrs.md` and the public names before the implementation was
//! read.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    missing_docs
)]
#![cfg(not(target_arch = "wasm32"))]

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use curvyo_document_core::{Document, ObjectSnapshot, Point, TextSnapshot, Timestamp, unpack};
use curvyo_editor_wasm::{EscapeStep, PreviewFacesGlue, Session, Tool};
use curvyo_text_core::FaceStore;
use curvyo_ui_core::{FontBlock, TextInput};
use serde_json::Value;

static NOW: AtomicU64 = AtomicU64::new(1_950_000_000);

const GREAT_VIBES: &str = "great-vibes/GreatVibes-Regular.ttf";
const SANS: &str = "source-sans-3/SourceSans3-Regular.otf";

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
    let mut s = Session::new(82);
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

fn add_text(s: &mut Session, at: Point, string: &str) {
    s.set_tool(Tool::Text);
    click(s, at);
    assert!(s.text_editing());
    s.text_input(TextInput::Insert(string.to_owned()));
    load(s);
    assert_eq!(s.escape(), EscapeStep::ClearedState);
    load(s);
}

fn doc_of(s: &Session) -> Document {
    unpack(82, &s.pack("0.1.0").unwrap()).unwrap()
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

fn select_at(s: &mut Session, at: Point) {
    s.set_tool(Tool::Select);
    click(s, at);
}

fn subject(s: &Session) -> Value {
    serde_json::from_str(&s.glyph_subject_json()).unwrap()
}

fn sink(s: &Session) -> (String, usize, usize) {
    s.text_sink_state().unwrap()
}

// --- Criterion 12 ---------------------------------------------------------------------------

/// The sink after: "Hello" edited at its end, "x" inserted and undone, "el" selected, two
/// characters put in by `put`, one in-session undo; and the steps the session wrote.
fn inner_selection_run(put: fn(&mut Session, &str)) -> Vec<(String, usize, usize)> {
    let mut s = fresh();
    add_text(&mut s, pt(10.0, 20.0), "Hello");
    let steps = s.undo_len();
    select_at(&mut s, pt(12.0, 17.0));
    s.insert_from_glyph_table("x"); // criterion 13 enters edit mode at the end
    s.text_input(TextInput::Undo);
    let mut seen = vec![sink(&s)];
    s.text_input(TextInput::Select {
        anchor: 1,
        caret: 3,
    });
    put(&mut s, "♥");
    seen.push(sink(&s));
    put(&mut s, "→");
    seen.push(sink(&s));
    s.text_input(TextInput::Undo);
    seen.push(sink(&s));
    s.text_input(TextInput::Redo);
    s.escape();
    seen.push((texts(&s)[0].string.clone(), s.undo_len() - steps, 0));
    seen
}

/// Criterion 12: a selection inside the text is replaced, the caret lands after the inserted
/// character, and the in-session undo and the history step are exactly those of typing the
/// same characters.
#[test]
fn ac12_inserts_exactly_as_typed() {
    let inserted = inner_selection_run(|s, c| {
        assert!(s.insert_from_glyph_table(c).editing);
    });
    let typed = inner_selection_run(|s, c| {
        s.text_input(TextInput::Insert(c.to_owned()));
    });
    assert_eq!(inserted, typed);
    assert_eq!(inserted[0].0, "Hello");
    assert_eq!(inserted[1], ("H♥lo".to_owned(), 2, 2));
    assert_eq!(inserted[2], ("H♥→lo".to_owned(), 3, 3));
    assert_eq!(inserted[4], ("H♥→lo".to_owned(), 1, 0), "one history step");
}

/// Criterion 12: the 10,000-character limit holds as for typing; replacing a selected
/// character at the limit still works.
#[test]
fn ac12_the_character_limit_holds() {
    let mut s = fresh();
    s.set_tool(Tool::Text);
    click(&mut s, pt(10.0, 20.0));
    s.text_input(TextInput::Insert("x".repeat(10_000)));
    let _ = s.take_text_limit_notice();
    let out = s.insert_from_glyph_table("♥");
    assert!(out.limited, "the limit notice is asked for");
    assert_eq!(sink(&s).0.chars().count(), 10_000);
    assert!(!sink(&s).0.contains('♥'));
    s.text_input(TextInput::Select {
        anchor: 0,
        caret: 1,
    });
    let out = s.insert_from_glyph_table("♥");
    assert!(!out.limited);
    assert!(sink(&s).0.starts_with('♥'));
    assert_eq!(sink(&s).0.chars().count(), 10_000);
}

/// Criteria 4 and 12: the subject is the edited text's face even when the next-text face
/// differs, and inserting never changes the text's face.
#[test]
fn ac12_the_text_keeps_its_own_face() {
    let mut s = fresh();
    add_text(&mut s, pt(10.0, 20.0), "Ab");
    let sans = texts(&s)[0].font.clone();
    // Nothing selected: a pick sets the next-text face only.
    s.set_tool(Tool::Select);
    click(&mut s, pt(500.0, 500.0));
    s.pick_font_face(GREAT_VIBES, 0);
    load(&mut s);
    assert_eq!(subject(&s)["family"], "Great Vibes", "next-text face");
    select_at(&mut s, pt(12.0, 17.0));
    assert_eq!(s.glyph_subject().unwrap(), sans, "the selected text's face");
    s.insert_from_glyph_table("ä");
    assert_eq!(s.glyph_subject().unwrap(), sans, "the edited text's face");
    s.escape();
    let t = &texts(&s)[0];
    assert_eq!(t.string, "Abä");
    assert_eq!(t.font, sans);
}

// --- Criterion 14 ---------------------------------------------------------------------------

/// Criterion 14: with other objects selected (a rectangle, or a rectangle and a text), a click
/// makes a new text and leaves the selected objects as they were.
#[test]
fn ac14_other_objects_selected_make_a_new_text() {
    let mut s = fresh();
    add_text(&mut s, pt(10.0, 20.0), "One");
    s.set_tool(Tool::Rectangle);
    s.pointer_hover(pt(100.0, 100.0), false, false);
    s.pointer_down(pt(100.0, 100.0), false);
    s.pointer_hover(pt(150.0, 150.0), false, false);
    s.pointer_up(pt(150.0, 150.0), false, false);
    s.set_tool(Tool::Select);
    assert_eq!(s.style_scope().ids.len(), 1, "the rectangle is selected");
    let before = doc_of(&s).object_ids().len();
    let out = s.insert_from_glyph_table("A");
    assert!(out.editing);
    assert_eq!(sink(&s), ("A".to_owned(), 1, 1));
    s.escape();
    let d = doc_of(&s);
    assert_eq!(d.object_ids().len(), before + 1);
    let strings: Vec<String> = texts(&s).into_iter().map(|t| t.string).collect();
    assert_eq!(strings, vec!["One".to_owned(), "A".to_owned()]);
}

/// Criterion 14: the new text is in the next-text face at the Text bar's size, the character's
/// middle (half its advance, half the cap height) on the centre of the canvas, one step that
/// one undo removes.
#[test]
fn ac14_new_text_in_next_face_and_size_centred() {
    let mut s = fresh();
    s.pick_font_face(GREAT_VIBES, 0);
    load(&mut s);
    let _ = s.set_text_size(25.0);
    let steps = s.undo_len();
    let out = s.insert_from_glyph_table("A");
    assert!(out.editing);
    s.escape();
    load(&mut s);
    let t = texts(&s).pop().unwrap();
    assert_eq!(t.string, "A");
    assert_eq!(t.font.family, "Great Vibes");
    assert!((t.size.as_mm() - 25.0).abs() < 1e-9, "{}", t.size.as_mm());
    let mut store = FaceStore::new();
    let key = store
        .add(std::fs::read(fonts_dir().join(GREAT_VIBES)).unwrap(), 0)
        .unwrap();
    let advance = curvyo_text_core::glyph_cell(store.get(&key).unwrap(), 'A')
        .unwrap()
        .advance;
    let centre = s.screen_to_document(600.0, 400.0);
    let mid_x = t.placement.e + advance * 25.0 / 2.0;
    let mid_y = t.placement.f - 25.0 / 2.0;
    assert!((mid_x - centre.x).abs() < 0.01, "{mid_x} vs {}", centre.x);
    assert!((mid_y - centre.y).abs() < 0.01, "{mid_y} vs {}", centre.y);
    assert_eq!(s.undo_len(), steps + 1);
    s.undo();
    assert!(texts(&s).is_empty(), "one undo removes the new text");
}

// --- Criteria 4, 11 -------------------------------------------------------------------------

/// Criterion 11: a text whose font is missing gives a subject marked missing, with its family
/// for the "Lobster is missing" state.
#[test]
fn ac11_a_missing_face_is_marked() {
    let mut s = fresh();
    add_text(&mut s, pt(10.0, 20.0), "Ab");
    let gone: Vec<_> = included_faces()
        .into_iter()
        .filter(|f| !f.0.starts_with("source-sans-3"))
        .collect();
    s.set_font_list(FontBlock::Included, gone, Vec::new());
    load(&mut s);
    select_at(&mut s, pt(12.0, 17.0));
    let v = subject(&s);
    assert_eq!(v["family"], "Source Sans 3");
    assert_eq!(v["missing"], true, "{v}");
}

// --- Glue: criteria 9, 16, 19 ---------------------------------------------------------------

fn glue() -> PreviewFacesGlue {
    let mut g = PreviewFacesGlue::default();
    for file in [SANS, GREAT_VIBES] {
        assert!(g.add(file, &std::fs::read(fonts_dir().join(file)).unwrap(), 0));
    }
    g
}

fn names_bytes() -> Vec<u8> {
    std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../frontend/public/unicode/names-17.0.0.bin"),
    )
    .unwrap()
}

fn table(g: &mut PreviewFacesGlue, id: &str, q: &str) -> Value {
    serde_json::from_str(&g.glyph_table(id, q)).unwrap()
}

/// Criteria 4 and 9: the count is the number of cells, each group's cells in code point order,
/// groups in code point order; with a query, "3 of 412" is `shown` of `total`.
#[test]
fn ac09_count_equals_cells() {
    let mut g = glue();
    let v = table(&mut g, SANS, "");
    let cells: Vec<u64> = v["groups"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|gr| {
            gr["cells"]
                .as_array()
                .unwrap()
                .iter()
                .map(|c| c.as_u64().unwrap())
        })
        .collect();
    assert_eq!(v["total"].as_u64().unwrap(), cells.len() as u64);
    assert_eq!(v["shown"], v["total"]);
    assert!(cells.windows(2).all(|w| w[0] < w[1]));
    assert!(g.set_unicode_names(&names_bytes()));
    let v = table(&mut g, SANS, "heart");
    assert!(v["shown"].as_u64().unwrap() < v["total"].as_u64().unwrap());
}

/// Criterion 16: the glue uses at most 64 characters of the query: 64 "x" and then "heart"
/// finds what 64 "x" find.
#[test]
fn ac16_query_cut_at_64_characters() {
    let mut g = glue();
    assert!(g.set_unicode_names(&names_bytes()));
    let long = format!("{} heart", "x".repeat(64));
    assert_eq!(
        table(&mut g, SANS, &long)["shown"],
        table(&mut g, SANS, &"x".repeat(64))["shown"]
    );
}

/// Criterion 19: the detail bar's name, glyph name and block; the name only after the names
/// file is handed over; a garbage file is refused.
#[test]
fn ac19_detail_bar_text() {
    let mut g = glue();
    let d: Value = serde_json::from_str(&g.glyph_detail(SANS, 0xE4)).unwrap();
    assert!(d["name"].is_null(), "no name before the file: {d}");
    assert_eq!(d["block"], "Latin-1 Supplement");
    assert!(!g.set_unicode_names(b"garbage"));
    assert!(g.set_unicode_names(&names_bytes()));
    let d: Value = serde_json::from_str(&g.glyph_detail(SANS, 0xE4)).unwrap();
    assert_eq!(d["name"], "LATIN SMALL LETTER A WITH DIAERESIS");
    assert_eq!(d["glyph_name"], "adieresis");
    let d: Value = serde_json::from_str(&g.glyph_detail(SANS, 0x20)).unwrap();
    assert_eq!(d["abbreviation"], "SP");
}

/// Open point of the implementer (reported, not a criterion): how many cells a one-letter query
/// lists in Source Sans 3. Printed for the report; asserts only that the exact match is first.
#[test]
fn open_point_one_letter_query() {
    let mut g = glue();
    assert!(g.set_unicode_names(&names_bytes()));
    let v = table(&mut g, SANS, "a");
    println!(
        "query \"a\": {} of {} cells, groups {:?}",
        v["shown"],
        v["total"],
        v["groups"]
            .as_array()
            .unwrap()
            .iter()
            .map(|gr| gr["name"].as_str().unwrap().to_owned())
            .collect::<Vec<_>>()
    );
    assert_eq!(v["groups"][0]["cells"][0], 0x61);
}
