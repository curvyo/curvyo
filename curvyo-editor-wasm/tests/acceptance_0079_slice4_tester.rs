//! Independent tester cases for `specs/0079-text-layout-controls/specification.md` slice 4 at the
//! `Session` level (ADR 0021): the two kerning fields after Character rotation with their ranges
//! and tracks (criterion 26), the character after the caret, a range and the line end (27), all
//! characters outside edit mode and "Clear kerning" in one step (28), Alt+arrows with the readout
//! inside the edit session and a field change as its own step (29), kerns following their
//! characters through typing, deleting and plain paste (30), Undo per character (ADR 0021 §5), a
//! new text kerned before it exists (33), a uniform resize (34), Object to path equals drawing
//! (35) and save and reopen (37).
//!
//! Written from the specification, ADR 0021 and the public names before the implementation was
//! read.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::too_many_lines,
    clippy::float_cmp,
    clippy::assert_is_empty,
    missing_docs
)]

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use curvyo_document_core::{
    CharValues, Document, FontKind, ObjectSnapshot, Point, TextSnapshot, Timestamp, unpack,
};
use curvyo_editor_wasm::{EscapeStep, KeyInput, LayoutOutcome, ObjectToPathOutcome, Session, Tool};
use curvyo_text_core::{CharOffset, Face, FaceStore, Mm};
use curvyo_ui_core::{CaretMove, FontBlock, LayoutField, TextInput};
use serde_json::Value;

static NOW: AtomicU64 = AtomicU64::new(1_990_000_000);
const TOL: f64 = 0.01;
const HK: LayoutField = LayoutField::HorizontalKerning;
const VK: LayoutField = LayoutField::VerticalKerning;

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

fn with_fonts(mut s: Session) -> Session {
    s.set_clock(clock);
    s.resize_viewport(1200.0, 800.0);
    s.set_font_list_with_kinds(FontBlock::Included, included_faces(), Vec::new());
    load(&mut s);
    s
}

fn fresh() -> Session {
    with_fonts(Session::new(79))
}

fn click(s: &mut Session, at: Point) {
    s.pointer_hover(at, false, false);
    s.pointer_down(at, false);
    s.pointer_up(at, false, false);
}

fn start_text(s: &mut Session, at: Point, string: &str) {
    s.set_tool(Tool::Text);
    click(s, at);
    assert!(s.text_editing());
    s.text_input(TextInput::Insert(string.to_owned()));
    load(s);
}

/// A text of Size 10 mm at `at`, committed and left selected.
fn add_text(s: &mut Session, at: Point, string: &str) {
    start_text(s, at, string);
    assert_eq!(s.escape(), EscapeStep::ClearedState);
    load(s);
    s.set_text_size(10.0);
}

/// Edit mode on the selected text (Enter), the caret before character `at`.
fn edit_at(s: &mut Session, at: usize) {
    let _ = s.key_down(KeyInput {
        key: "Enter",
        ..KeyInput::default()
    });
    assert!(s.text_editing(), "Enter enters edit mode");
    caret_to(s, at);
}

fn caret_to(s: &mut Session, at: usize) {
    let (string, _, _) = s.text_sink_state().unwrap();
    let byte = string
        .char_indices()
        .nth(at)
        .map_or(string.len(), |(b, _)| b);
    s.text_input(TextInput::Select {
        anchor: byte,
        caret: byte,
    });
}

fn extend_right(s: &mut Session, n: usize) {
    for _ in 0..n {
        s.text_input(TextInput::Move {
            to: CaretMove::Right,
            extend: true,
        });
    }
}

fn key(s: &mut Session, k: &str, ctrl: bool) {
    let _ = s.key_down(KeyInput {
        key: k,
        ctrl,
        ..KeyInput::default()
    });
}

fn select_all(s: &mut Session) {
    s.set_tool(Tool::Select);
    key(s, "a", true);
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

fn text_named(s: &Session, string: &str) -> TextSnapshot {
    texts(s)
        .into_iter()
        .find(|t| t.string == string)
        .unwrap_or_else(|| panic!("no text {string:?}: {:?}", texts(s)))
}

/// The kerns of `t` in mm: (index, along, across).
fn kerns_mm(t: &TextSnapshot) -> Vec<(usize, f64, f64)> {
    let size = t.size.as_mm();
    t.chars
        .entries()
        .iter()
        .map(|(i, v)| (*i, v.kern_x * size, v.kern_y * size))
        .collect()
}

fn assert_kerns(t: &TextSnapshot, want: &[(usize, f64, f64)]) {
    let got = kerns_mm(t);
    assert_eq!(got.len(), want.len(), "{got:?} vs {want:?}");
    for (g, w) in got.iter().zip(want) {
        assert!(
            g.0 == w.0 && (g.1 - w.1).abs() < 1e-6 && (g.2 - w.2).abs() < 1e-6,
            "{got:?} vs {want:?}"
        );
    }
}

fn labels(s: &Session) -> Vec<String> {
    let l = loro::LoroDoc::new();
    l.import(&doc_of(s).export_loro_snapshot().unwrap())
        .unwrap();
    let ids: Vec<loro::ID> = l.oplog_frontiers().iter().collect();
    let mut changes: Vec<(u32, String)> = Vec::new();
    l.travel_change_ancestors(&ids, &mut |c| {
        changes.push((
            c.lamport,
            c.message
                .map(|m| m.split(';').next().unwrap_or_default().to_string())
                .unwrap_or_default(),
        ));
        std::ops::ControlFlow::Continue(())
    })
    .unwrap();
    changes.sort();
    changes
        .into_iter()
        .map(|(_, m)| m)
        .filter(|m| m != "new_document")
        .collect()
}

fn field(s: &Session, f: LayoutField) -> curvyo_ui_core::LayoutFieldView {
    let block = s.text_layout_block().expect("the block is shown");
    let i = LayoutField::ALL.iter().position(|x| *x == f).unwrap();
    block[i].clone()
}

fn with_sans<R>(f: impl FnOnce(&Face) -> R) -> R {
    let bytes = std::fs::read(fonts_dir().join("source-sans-3/SourceSans3-Regular.otf")).unwrap();
    let mut store = FaceStore::new();
    let key = store.add(bytes, 0).unwrap();
    f(store.get(&key).unwrap())
}

// ---------------------------------------------------------------------------------------------
// Part E
// ---------------------------------------------------------------------------------------------

/// Criteria 26 and 27b: after Character rotation come Horizontal kerning and Vertical kerning, in mm,
/// then Per-character rotation (customer 2026-10-24),
/// default 0, tracks ±0.25 and ±0.5 × Size, typed ranges ±1 and ±2 × Size (a typed value beyond
/// is set to the limit, permit over refuse).
#[test]
fn c26_two_kerning_fields_after_rotation_with_ranges_and_tracks() {
    let order: Vec<LayoutField> = LayoutField::ALL.to_vec();
    let rot = order
        .iter()
        .position(|f| *f == LayoutField::CharacterRotation)
        .unwrap();
    assert_eq!(
        &order[rot + 1..],
        &[LayoutField::Taper, HK, VK, LayoutField::CharRotation]
    );
    let mut s = fresh();
    add_text(&mut s, pt(10.0, 20.0), "Kern");
    for (f, track) in [(HK, 2.5), (VK, 5.0)] {
        let v = field(&s, f);
        assert_eq!((v.text.as_str(), v.mixed), ("0", false), "{f:?} default 0");
        assert!(
            (v.track_min + track).abs() < 1e-9 && (v.track_max - track).abs() < 1e-9,
            "{f:?}: {v:?}"
        );
    }
    assert!(matches!(
        s.set_text_layout_text(HK, "999"),
        LayoutOutcome::Clamped(true, _)
    ));
    assert!(matches!(
        s.set_text_layout_text(VK, "-999"),
        LayoutOutcome::Clamped(false, _)
    ));
    let t = text_named(&s, "Kern");
    assert_kerns(&t, &(0..4).map(|i| (i, 10.0, -20.0)).collect::<Vec<_>>());
    assert_eq!(field(&s, HK).text, "10");
    assert_eq!(field(&s, VK).text, "\u{2212}20");
}

/// Criterion 27: in edit mode the fields show and set the kern of the character after the caret;
/// a range shows its common value or Mixed and a change sets every character of it; at the end of
/// a line the fields show 0 and a change does nothing.
#[test]
fn c27_fields_act_on_the_char_after_the_caret_a_range_and_not_at_line_end() {
    let mut s = fresh();
    add_text(&mut s, pt(10.0, 20.0), "ABCD\nE");
    edit_at(&mut s, 1);
    assert_eq!(s.set_text_layout_text(HK, "2"), LayoutOutcome::Set);
    assert_eq!(s.set_text_layout_text(VK, "-1.5"), LayoutOutcome::Set);
    assert_eq!(field(&s, HK).text, "2");
    caret_to(&mut s, 0);
    assert_eq!(field(&s, HK).text, "0", "A has no kern");
    // A range B..C: B is kerned, C is not: Mixed; a change sets both.
    caret_to(&mut s, 1);
    extend_right(&mut s, 2);
    assert!(field(&s, HK).mixed);
    assert_eq!(s.set_text_layout_text(HK, "0.5"), LayoutOutcome::Set);
    assert!(!field(&s, HK).mixed);
    // End of the first line: nothing after the caret but the line break.
    s.text_input(TextInput::Move {
        to: CaretMove::LineEnd,
        extend: false,
    });
    assert_eq!(field(&s, HK).text, "0");
    let _ = s.set_text_layout_text(HK, "3");
    assert!(s.text_editing());
    assert_eq!(s.escape(), EscapeStep::ClearedState);
    assert_kerns(&text_named(&s, "ABCD\nE"), &[(1, 0.5, -1.5), (2, 0.5, 0.0)]);
}

/// Criterion 28: outside edit mode the fields show the common kern of all characters of the
/// subject texts (0, else Mixed); a change sets every character; the reset clears every manual
/// kern of the texts in one step "Clear kerning".
#[test]
fn c28_outside_edit_mode_all_chars_and_clear_kerning_in_one_step() {
    let mut s = fresh();
    add_text(&mut s, pt(10.0, 20.0), "One");
    add_text(&mut s, pt(10.0, 80.0), "Two");
    edit_at(&mut s, 1);
    s.set_text_layout_text(HK, "1");
    s.escape();
    select_all(&mut s);
    assert!(field(&s, HK).mixed, "one kerned character among six");
    assert_eq!(field(&s, VK).text, "0");
    assert!(field(&s, HK).resettable);
    assert_eq!(s.set_text_layout_text(VK, "0.4"), LayoutOutcome::Set);
    assert_kerns(
        &text_named(&s, "One"),
        &[(0, 0.0, 0.4), (1, 0.0, 0.4), (2, 0.0, 0.4)],
    );
    assert_eq!(field(&s, VK).text, "0.4");
    let n = labels(&s).len();
    assert_eq!(s.reset_text_layout(HK), LayoutOutcome::Set);
    assert_eq!(labels(&s).len(), n + 1, "one step for both texts");
    assert_eq!(labels(&s).last().unwrap(), "clear_kerning");
    assert!(
        texts(&s).iter().all(|t| t.chars.is_empty()),
        "every kern, both keys"
    );
    assert_eq!(s.reset_text_layout(VK), LayoutOutcome::Unchanged);
    assert_eq!(labels(&s).len(), n + 1);
    s.undo();
    assert_eq!(
        kerns_mm(&text_named(&s, "Two")).len(),
        3,
        "Undo brings them back"
    );
}

/// Criterion 29: Alt+Left/Right change the horizontal kern of the character after the caret by
/// ∓0.1 mm, Shift by 1 mm, Alt+Up/Down the vertical kern; the readout is "Kern +0.3, 0.0 mm"; the
/// keys belong to the edit session: Undo inside the session takes the last one back, and the
/// whole session is one step.
#[test]
fn c29_alt_arrows_step_with_readout_inside_the_edit_session() {
    let mut s = fresh();
    add_text(&mut s, pt(10.0, 20.0), "AV");
    edit_at(&mut s, 1);
    let n = labels(&s).len();
    for _ in 0..3 {
        s.text_kern(1, 0, false);
    }
    assert_eq!(s.text_kern(0, 1, false), "Kern +0.3, +0.1 mm");
    assert_eq!(
        s.text_kern(-1, -1, true),
        "Kern \u{2212}0.7, \u{2212}0.9 mm"
    );
    assert_eq!(labels(&s).len(), n, "nothing written during the session");
    // Session undo takes the run of kern keys back (a text field groups a run), Redo
    // brings it again; neither writes a step.
    s.text_input(TextInput::Undo);
    assert_eq!(field(&s, HK).text, "0", "session undo");
    s.text_input(TextInput::Redo);
    assert_eq!(field(&s, HK).text, "\u{2212}0.7", "session redo");
    s.text_kern(1, 0, true);
    s.text_kern(0, 1, true);
    assert_eq!(s.text_kern(0, 0, false), "", "no change, no readout");
    assert_eq!(labels(&s).len(), n);
    assert_eq!(s.escape(), EscapeStep::ClearedState);
    assert_eq!(labels(&s).len(), n + 1, "the session is one step");
    assert_kerns(&text_named(&s, "AV"), &[(1, 0.3, 0.1)]);
    // At the line end nothing changes and the readout is empty.
    edit_at(&mut s, 2);
    assert_eq!(s.text_kern(1, 0, false), "");
}

/// Criterion 29, last sentence: a kerning field change in edit mode is its own step "Change
/// kerning" (as criterion 14 for the other fields), not folded into the typing session.
#[test]
fn c29_kerning_field_in_edit_mode_is_its_own_step() {
    let mut s = fresh();
    add_text(&mut s, pt(10.0, 20.0), "To");
    edit_at(&mut s, 1);
    let n = labels(&s).len();
    s.set_text_layout_text(HK, "-0.8");
    assert_eq!(labels(&s).len(), n + 1, "its own step at once");
    assert_eq!(labels(&s).last().unwrap(), "change_kerning");
    assert!(s.text_editing(), "edit mode stays");
}

/// Criterion 29 with `0074` criterion 9 (white-box, from the plan's dated note): after a kerning
/// field change in edit mode, a Ctrl+Z inside the session that takes back earlier typing leaves
/// the document as the canvas shows it when edit mode ends (the typing before the change may be
/// committed as its own step first, as `0078` criterion 16 does).
#[test]
fn c29_field_change_then_session_undo_commits_what_is_shown() {
    let mut s = fresh();
    add_text(&mut s, pt(10.0, 20.0), "To");
    edit_at(&mut s, 2);
    s.text_input(TextInput::Insert("p".into()));
    caret_to(&mut s, 1);
    s.set_text_layout_text(HK, "-0.8");
    assert_eq!(labels(&s).last().unwrap(), "change_kerning");
    s.text_input(TextInput::Undo);
    let shown = s.text_sink_state().unwrap().0;
    assert_eq!(s.escape(), EscapeStep::ClearedState);
    assert!(
        texts(&s).iter().any(|t| t.string == shown),
        "shown {shown:?}, stored {:?}",
        texts(&s)
            .iter()
            .map(|t| t.string.clone())
            .collect::<Vec<_>>()
    );
}

/// Criterion 30: a kern stays with its character: typing or deleting before it moves it along,
/// a character typed directly before or after it has none, deleting it deletes its kern, and a
/// deleted and retyped character has none; plain copy and paste carries no kern.
#[test]
fn c30_kern_follows_its_char_through_typing_deleting_and_paste() {
    let mut s = fresh();
    add_text(&mut s, pt(10.0, 20.0), "ABC");
    edit_at(&mut s, 1);
    s.set_text_layout_text(HK, "1");
    caret_to(&mut s, 0);
    s.text_input(TextInput::Insert("xx".into())); // "xxABC", B at 3
    caret_to(&mut s, 3);
    s.text_input(TextInput::Insert("y".into())); // directly before B: "xxAyBC"
    caret_to(&mut s, 5);
    s.text_input(TextInput::Insert("z".into())); // directly after B: "xxAyBzC"
    caret_to(&mut s, 1);
    s.text_input(TextInput::DeleteBackward); // "xAyBzC", B at 3
    assert_eq!(s.escape(), EscapeStep::ClearedState);
    assert_kerns(&text_named(&s, "xAyBzC"), &[(3, 1.0, 0.0)]);

    // Copy B (plain text), paste it at the end: no kern on the copy.
    edit_at(&mut s, 3);
    extend_right(&mut s, 1);
    let copied = s.text_selected();
    assert_eq!(copied, "B");
    s.text_input(TextInput::Move {
        to: CaretMove::LineEnd,
        extend: false,
    });
    s.text_input(TextInput::Insert(copied));
    // Delete B and type it again: no kern.
    caret_to(&mut s, 4);
    s.text_input(TextInput::DeleteBackward);
    s.text_input(TextInput::Insert("B".into()));
    assert_eq!(s.escape(), EscapeStep::ClearedState);
    let t = text_named(&s, "xAyBzCB");
    assert!(t.chars.is_empty(), "{:?}", kerns_mm(&t));
}

/// ADR 0021 §5: Undo is per character: undoing a later kern on one character keeps an earlier
/// kern on another; Redo brings it back.
#[test]
fn c30_undo_per_character() {
    let mut s = fresh();
    add_text(&mut s, pt(10.0, 20.0), "WAVE");
    select_all(&mut s);
    edit_at(&mut s, 1);
    s.text_kern(-1, 0, true);
    s.escape();
    edit_at(&mut s, 3);
    s.text_kern(1, 0, true);
    s.escape();
    assert_kerns(&text_named(&s, "WAVE"), &[(1, -1.0, 0.0), (3, 1.0, 0.0)]);
    s.undo();
    assert_kerns(&text_named(&s, "WAVE"), &[(1, -1.0, 0.0)]);
    s.redo();
    assert_kerns(&text_named(&s, "WAVE"), &[(1, -1.0, 0.0), (3, 1.0, 0.0)]);
}

// ---------------------------------------------------------------------------------------------
// Part G
// ---------------------------------------------------------------------------------------------

/// Criterion 33 with ADR 0021 (`create_text_with_chars`): a new text kerned in edit mode before it
/// was ever committed is created with its kern in one step.
#[test]
fn c33_new_text_kerned_before_it_exists() {
    let mut s = fresh();
    let n = labels(&s).len();
    start_text(&mut s, pt(40.0, 40.0), "LT");
    caret_to(&mut s, 1);
    assert!(!s.text_kern(-1, 0, false).is_empty());
    assert_eq!(s.escape(), EscapeStep::ClearedState);
    let t = text_named(&s, "LT");
    assert_eq!(t.chars.entries().len(), 1);
    assert!((kerns_mm(&t)[0].1 + 0.1).abs() < 1e-6);
    assert_eq!(labels(&s).len(), n + 1, "one step");
}

/// Criterion 34: a uniform resize keeps each kern's factor; the mm readout grows with Size.
#[test]
fn c34_uniform_resize_keeps_kern_factors() {
    let mut s = fresh();
    add_text(&mut s, pt(10.0, 20.0), "Hi");
    edit_at(&mut s, 1);
    s.set_text_layout_text(HK, "1.5");
    s.escape();
    let before: Vec<(usize, CharValues)> = text_named(&s, "Hi").chars.entries().to_vec();
    select_all(&mut s);
    s.set_text_size(20.0);
    assert_eq!(text_named(&s, "Hi").chars.entries(), &before[..]);
    edit_at(&mut s, 1);
    assert_eq!(field(&s, HK).text, "3");
}

/// Criterion 35: Object to path of a kerned text matches the drawn outlines within 0.01 mm, one
/// path per glyph.
#[test]
fn c35_object_to_path_equals_the_kerned_drawing() {
    let mut s = fresh();
    add_text(&mut s, pt(30.0, 40.0), "AVA");
    edit_at(&mut s, 1);
    s.set_text_layout_text(HK, "-1.2");
    s.set_text_layout_text(VK, "2");
    s.escape();
    let t = text_named(&s, "AVA");
    let size = t.size.as_mm();
    let offsets: Vec<CharOffset> = t
        .chars
        .entries()
        .iter()
        .map(|(i, v)| CharOffset {
            index: *i,
            along: Mm(v.kern_x * size),
            across: Mm(v.kern_y * size),
            rotation_rad: v.rotation.to_radians(),
        })
        .collect();
    assert_eq!(offsets.len(), 1);
    let want: Vec<Point> = with_sans(|face| {
        let o = curvyo_text_core::LayoutOptions {
            char_offsets: offsets,
            ..curvyo_text_core::LayoutOptions::default()
        };
        curvyo_text_core::layout(face, &t.string, Mm(size), &o)
            .unwrap()
            .glyph_paths(face)
            .iter()
            .flat_map(|p| {
                p.elements()
                    .iter()
                    .filter_map(kurbo::PathEl::end_point)
                    .map(|q| t.placement.apply_point(pt(q.x, q.y)))
                    .collect::<Vec<_>>()
            })
            .collect()
    });
    select_all(&mut s);
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
    assert_eq!(paths.len(), 3, "one path per glyph");
    for p in paths.concat() {
        assert!(
            want.iter()
                .any(|w| (w.x - p.x).abs() < TOL && (w.y - p.y).abs() < TOL),
            "converted anchor {p:?} is not on the kerned outline"
        );
    }
}

/// Criterion 37: save, close and open again: the kerns are there and editable.
#[test]
fn c37_save_and_reopen_keeps_kerns_editable() {
    let mut s = fresh();
    add_text(&mut s, pt(20.0, 20.0), "Keep");
    edit_at(&mut s, 2);
    s.set_text_layout_text(HK, "0.7");
    s.set_text_layout_text(VK, "-0.3");
    s.escape();
    let before = text_named(&s, "Keep");
    let mut s = with_fonts(Session::open(23, &s.pack("0.1.0").unwrap()).unwrap());
    assert_eq!(text_named(&s, "Keep").chars, before.chars);
    select_all(&mut s);
    edit_at(&mut s, 2);
    assert_eq!(field(&s, HK).text, "0.7");
    assert_eq!(field(&s, VK).text, "\u{2212}0.3");
    s.set_text_layout_text(HK, "0");
    s.set_text_layout_text(VK, "0");
    s.escape();
    assert!(text_named(&s, "Keep").chars.is_empty());
}

/// White-box (criterion 12 budget applied to the kerning fields, ADR 0021 §2 cost): outside edit
/// mode a drag frame on Horizontal kerning over a 500-character text, and its release writing a
/// mark on every character, stay within the frame budget; a 10,000-character text (the limit)
/// can be kerned and cleared.
#[test]
fn edge_kerning_every_char_of_long_texts_within_budget() {
    let mut s = fresh();
    let long: String = "Lorem ipsum dolor sit amet "
        .repeat(19)
        .chars()
        .take(500)
        .collect();
    add_text(&mut s, pt(10.0, 20.0), &long);
    let budget = if cfg!(debug_assertions) { 500.0 } else { 50.0 };
    let start = std::time::Instant::now();
    s.preview_text_layout(HK, 0.7, curvyo_ui_core::Grid::Normal);
    let _ = s.text_layout_block();
    let _ = s.draw_list();
    let ms = start.elapsed().as_secs_f64() * 1000.0;
    assert!(ms < budget, "{ms} ms for one drag frame");
    let start = std::time::Instant::now();
    s.commit_text_layout_preview();
    let ms = start.elapsed().as_secs_f64() * 1000.0;
    assert!(ms < budget, "{ms} ms for the release");
    assert_eq!(
        text_named(&s, &long).chars.entries().len(),
        500 - long.matches('\n').count()
    );

    let huge: String = "W".repeat(10_000);
    add_text(&mut s, pt(10.0, 120.0), &huge);
    assert_eq!(s.set_text_layout_text(HK, "-0.5"), LayoutOutcome::Set);
    assert_eq!(text_named(&s, &huge).chars.entries().len(), 10_000);
    assert_eq!(s.reset_text_layout(HK), LayoutOutcome::Set);
    assert!(text_named(&s, &huge).chars.is_empty());
}
