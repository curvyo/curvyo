//! Independent tester cases for `specs/0079-text-layout-controls/specification.md` slice 1 at the
//! `Session` level: the pressed alignment and Mixed (criterion 1), an alignment change in place
//! in one step (4), typing in a centred text (5), the block only with a subject (6), typed values,
//! limits and the wrap (7), drag preview, release, Escape, click and reset (12), several texts
//! and Mixed (13), edit mode (14), the next text (33), a uniform resize (34), Object to path
//! equals drawing (35), a missing face (36) and save and reopen (37).
//!
//! Written from the specification, `adrs.md` and the public names before the implementation was
//! read.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::too_many_lines,
    clippy::float_cmp,
    clippy::many_single_char_names,
    clippy::assert_is_empty,
    missing_docs
)]

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use curvyo_document_core::{
    Affine, Document, FontKind, FontRef, Length, ObjectSnapshot, Point, TextAlign,
    TextLayoutValues, TextSnapshot, Timestamp, Vec2, pack, unpack,
};
use curvyo_editor_wasm::{EscapeStep, KeyInput, LayoutOutcome, ObjectToPathOutcome, Session, Tool};
use curvyo_text_core::{Face, FaceStore, Mm};
use curvyo_ui_core::{FontBlock, Grid, LayoutField, TextInput};
use serde_json::Value;

static NOW: AtomicU64 = AtomicU64::new(1_980_000_000);
const TOL: f64 = 0.01;
const D: TextLayoutValues = TextLayoutValues::DEFAULT;

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

/// A text at `at` from the Text tool, left in edit mode.
fn start_text(s: &mut Session, at: Point, string: &str) {
    s.set_tool(Tool::Text);
    click(s, at);
    assert!(s.text_editing());
    s.text_input(TextInput::Insert(string.to_owned()));
    load(s);
}

/// A text at `at`, committed and left selected.
fn add_text(s: &mut Session, at: Point, string: &str) {
    start_text(s, at, string);
    assert_eq!(s.escape(), EscapeStep::ClearedState);
    load(s);
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

fn deselect(s: &mut Session) {
    s.set_tool(Tool::Select);
    while !s.style_scope().ids.is_empty() {
        s.escape();
    }
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
    texts(s).into_iter().find(|t| t.string == string).unwrap()
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

fn with_sans<R>(f: impl FnOnce(&Face) -> R) -> R {
    let bytes = std::fs::read(fonts_dir().join("source-sans-3/SourceSans3-Regular.otf")).unwrap();
    let mut store = FaceStore::new();
    let key = store.add(bytes, 0).unwrap();
    f(store.get(&key).unwrap())
}

/// The engine options for `t`, computed here from the spec (factor x Size), not by the
/// implementation's mapping.
fn mm_options(t: &TextSnapshot) -> curvyo_text_core::LayoutOptions {
    let size = t.size.as_mm();
    curvyo_text_core::LayoutOptions {
        align: match t.layout.align {
            TextAlign::Left => curvyo_text_core::Align::Left,
            TextAlign::Centre => curvyo_text_core::Align::Centre,
            TextAlign::Right => curvyo_text_core::Align::Right,
            TextAlign::Justify => curvyo_text_core::Align::Justify,
        },
        letter_spacing: Mm(t.layout.letter_spacing * size),
        word_spacing: Mm(t.layout.word_spacing * size),
        line_spacing: t.layout.line_spacing.map(|f| Mm(f * size)),
        rotation_rad: t.layout.rotation.as_radians(),
        ..curvyo_text_core::LayoutOptions::default()
    }
}

fn engine(t: &TextSnapshot, face: &Face) -> curvyo_text_core::TextLayout {
    assert_eq!(t.font.family, "Source Sans 3");
    assert_eq!(t.font.style, "Regular");
    curvyo_text_core::layout(face, &t.string, Mm(t.size.as_mm()), &mm_options(t)).unwrap()
}

fn field_text(s: &Session, f: LayoutField) -> (String, bool) {
    let block = s.text_layout_block().expect("the block is shown");
    let i = LayoutField::ALL.iter().position(|x| *x == f).unwrap();
    (block[i].text.clone(), block[i].mixed)
}

// ---------------------------------------------------------------------------------------------
// Part A
// ---------------------------------------------------------------------------------------------

/// Criterion 1: the pressed alignment is the subject texts' common one, none when they differ;
/// a new text is left-aligned.
#[test]
fn ac01_pressed_alignment_and_mixed() {
    let mut s = fresh();
    assert_eq!(s.text_alignment(), Some(TextAlign::Left), "next text: left");
    add_text(&mut s, pt(10.0, 20.0), "One");
    assert_eq!(text_named(&s, "One").layout.align, TextAlign::Left);
    s.set_text_alignment(TextAlign::Centre);
    assert_eq!(s.text_alignment(), Some(TextAlign::Centre));
    add_text(&mut s, pt(10.0, 80.0), "Two");
    assert_eq!(
        s.text_alignment(),
        Some(TextAlign::Left),
        "the new text is selected"
    );
    select_all(&mut s);
    assert_eq!(s.style_scope().ids.len(), 2);
    assert_eq!(s.text_alignment(), None, "differing texts: none pressed");
}

/// Criterion 4: an alignment change keeps the widest line where it is, in one step "Change text
/// alignment" for every selected text; a press on the pressed alignment writes nothing; Undo
/// restores alignment and place.
#[test]
fn ac04_alignment_change_in_place_one_step() {
    let mut s = fresh();
    add_text(&mut s, pt(20.0, 20.0), "AA\nAAAAAA\nA");
    add_text(&mut s, pt(20.0, 120.0), "Wide line\nx");
    select_all(&mut s);
    let before: Vec<TextSnapshot> = texts(&s);
    let n = labels(&s).len();
    s.set_text_alignment(TextAlign::Right);
    assert_eq!(labels(&s).len(), n + 1, "one step for two texts");
    assert_eq!(labels(&s).last().unwrap(), "change_text_alignment");
    let after = texts(&s);
    with_sans(|face| {
        for (b, a) in before.iter().zip(&after) {
            assert_eq!(a.layout.align, TextAlign::Right);
            let (lb, la) = (engine(b, face), engine(a, face));
            let widest = |l: &curvyo_text_core::TextLayout| {
                (0..l.lines.len())
                    .max_by(|&i, &j| {
                        let w = |k: usize| l.lines[k].extent.end.0 - l.lines[k].extent.start.0;
                        w(i).total_cmp(&w(j))
                    })
                    .unwrap()
            };
            let i = widest(&lb);
            let p = b
                .placement
                .apply_point(pt(lb.lines[i].extent.start.0, lb.lines[i].baseline.0));
            let q = a
                .placement
                .apply_point(pt(la.lines[i].extent.start.0, la.lines[i].baseline.0));
            assert!(
                (p.x - q.x).abs() < TOL && (p.y - q.y).abs() < TOL,
                "{p:?} {q:?}"
            );
            // Every other line now ends where the widest one ends.
            for line in &la.lines {
                let e = a
                    .placement
                    .apply_point(pt(line.extent.end.0, line.baseline.0));
                let w = a
                    .placement
                    .apply_point(pt(la.lines[i].extent.end.0, la.lines[i].baseline.0));
                assert!((e.x - w.x).abs() < TOL, "right-aligned: {e:?} {w:?}");
            }
        }
    });
    s.set_text_alignment(TextAlign::Right);
    assert_eq!(
        labels(&s).len(),
        n + 1,
        "pressing the pressed one writes nothing"
    );
    s.undo();
    let undone = texts(&s);
    for (b, u) in before.iter().zip(&undone) {
        assert_eq!(u.layout.align, TextAlign::Left);
        assert_eq!(u.placement, b.placement);
    }
}

/// Criterion 5: in edit mode with Centre, typing grows the line to both sides of the anchor and
/// the caret stays at the typed position; one keystroke in a 500-character text stays within the
/// typing budget.
#[test]
fn ac05_centre_typing_grows_both_sides() {
    let mut s = fresh();
    s.set_text_alignment(TextAlign::Centre);
    start_text(&mut s, pt(50.0, 50.0), "H");
    s.text_input(TextInput::Insert("HHH".into()));
    let (_, caret, _) = s.text_sink_state().unwrap();
    assert_eq!(caret, 4, "caret after the typed text");
    assert_eq!(s.escape(), EscapeStep::ClearedState);
    let t = text_named(&s, "HHHH");
    assert_eq!(t.layout.align, TextAlign::Centre);
    with_sans(|face| {
        let l = engine(&t, face);
        let mid = f64::midpoint(l.lines[0].extent.start.0, l.lines[0].extent.end.0);
        let m = t.placement.apply_point(pt(mid, 0.0));
        assert!((m.x - 50.0).abs() < 0.5, "centred on the click: {m:?}");
    });
    // The anchor stays while typing: a second session typing one more H keeps the centre.
    let mut s = fresh();
    s.set_text_alignment(TextAlign::Centre);
    let long: String = "Lorem ipsum dolor sit amet "
        .repeat(19)
        .chars()
        .take(499)
        .collect();
    start_text(&mut s, pt(50.0, 50.0), &long);
    let start = Instant::now();
    s.text_input(TextInput::Insert("x".into()));
    let ms = start.elapsed().as_secs_f64() * 1000.0;
    let budget = if cfg!(debug_assertions) { 500.0 } else { 50.0 };
    assert!(ms < budget, "{ms} ms for one keystroke in 500 characters");
}

// ---------------------------------------------------------------------------------------------
// Part B
// ---------------------------------------------------------------------------------------------

/// Criterion 6: the block is there only with a text selected or in edit mode.
#[test]
fn ac06_block_only_with_a_text_subject() {
    let mut s = fresh();
    assert!(
        s.text_layout_block().is_none(),
        "nothing selected: no block"
    );
    s.set_tool(Tool::Text);
    assert!(
        s.text_layout_block().is_none(),
        "Text tool before the first click: no block"
    );
    start_text(&mut s, pt(10.0, 20.0), "Edit");
    assert_eq!(
        s.text_layout_block().unwrap().len(),
        8,
        "edit mode: the block"
    );
    s.escape();
    assert!(s.text_layout_block().is_some(), "selected text: the block");
    deselect(&mut s);
    assert!(s.text_layout_block().is_none(), "cleared: no block");
    s.set_tool(Tool::Rectangle);
    s.pointer_hover(pt(100.0, 100.0), false, false);
    s.pointer_down(pt(100.0, 100.0), false);
    s.pointer_hover(pt(140.0, 130.0), false, false);
    s.pointer_up(pt(140.0, 130.0), false, false);
    s.set_tool(Tool::Select);
    assert!(!s.style_scope().ids.is_empty());
    assert!(
        s.text_layout_block().is_none(),
        "a rectangle alone: no block"
    );
}

/// Criterion 7: the four fields in order with their defaults; typed values past the range are set
/// to the limit (the notice's value), a rotation wraps, text that is not a number is refused
/// without a step.
#[test]
fn ac07_fields_defaults_limits_and_wrap() {
    let mut s = fresh();
    add_text(&mut s, pt(10.0, 20.0), "Plate");
    s.set_text_size(20.0);
    let block = s.text_layout_block().unwrap();
    assert_eq!(block.len(), 8);
    assert_eq!(block[0].text, "0");
    assert_eq!(block[1].text, "0");
    assert!(block[2].auto && !block[2].resettable);
    assert_eq!(block[3].text, "0");
    let n = labels(&s).len();
    assert_eq!(
        s.set_text_layout_text(LayoutField::LetterSpacing, "100"),
        LayoutOutcome::Clamped(true, 40.0)
    );
    assert_eq!(text_named(&s, "Plate").layout.letter_spacing, 2.0);
    assert_eq!(field_text(&s, LayoutField::LetterSpacing).0, "40");
    assert_eq!(
        s.set_text_layout_text(LayoutField::WordSpacing, "-100"),
        LayoutOutcome::Clamped(false, -20.0)
    );
    assert_eq!(
        s.set_text_layout_text(LayoutField::CharacterRotation, "270"),
        LayoutOutcome::Set
    );
    let r = text_named(&s, "Plate")
        .layout
        .rotation
        .as_radians()
        .to_degrees();
    assert!((r + 90.0).abs() < 1e-9, "{r}");
    assert_eq!(labels(&s).len(), n + 3);
    assert_eq!(
        s.set_text_layout_text(LayoutField::LineSpacing, "wide"),
        LayoutOutcome::Invalid
    );
    assert_eq!(labels(&s).len(), n + 3, "not a number: no step");
    assert_eq!(
        s.set_text_layout_text(LayoutField::LineSpacing, "15 mm"),
        LayoutOutcome::Set
    );
    assert_eq!(text_named(&s, "Plate").layout.line_spacing, Some(0.75));
}

/// Criterion 10: Auto shows the natural spacing and follows a face change; a set value is kept
/// across a face change; the reset returns to Auto.
#[test]
fn ac10_line_spacing_auto_and_set_across_a_face_change() {
    let mut s = fresh();
    add_text(&mut s, pt(10.0, 20.0), "A\nB");
    let auto = s.text_layout_block().unwrap()[2].clone();
    assert!(auto.auto);
    s.pick_font_face("source-serif-4/SourceSerif4-Regular.ttf", 0);
    load(&mut s);
    let auto2 = s.text_layout_block().unwrap()[2].clone();
    assert!(auto2.auto);
    assert!(
        (auto.value - auto2.value).abs() > 0.01,
        "Auto follows the face: {} {}",
        auto.value,
        auto2.value
    );
    assert_eq!(
        s.set_text_layout_text(LayoutField::LineSpacing, "15"),
        LayoutOutcome::Set
    );
    s.pick_font_face("source-sans-3/SourceSans3-Regular.otf", 0);
    load(&mut s);
    let set = s.text_layout_block().unwrap()[2].clone();
    assert!(!set.auto && set.resettable && set.text == "15", "{set:?}");
    let n = labels(&s).len();
    s.reset_text_layout(LayoutField::LineSpacing);
    assert_eq!(labels(&s).len(), n + 1);
    assert_eq!(labels(&s).last().unwrap(), "change_line_spacing");
    assert_eq!(text_named(&s, "A\nB").layout.line_spacing, None);
}

/// Criterion 12: a drag previews without writing, the release is one step with the field's label,
/// Escape during the drag writes nothing; a click, an arrow, a typed value and the reset are one
/// step each; several texts change in one step.
#[test]
fn ac12_preview_release_escape_and_one_step() {
    let mut s = fresh();
    add_text(&mut s, pt(10.0, 20.0), "First");
    add_text(&mut s, pt(10.0, 80.0), "Second");
    select_all(&mut s);
    let n = labels(&s).len();
    for p in [0.3, 0.5, 0.7, 0.9] {
        s.preview_text_layout(LayoutField::LetterSpacing, p, Grid::Normal);
    }
    assert_eq!(labels(&s).len(), n, "a drag writes nothing");
    assert!(texts(&s).iter().all(|t| t.layout.letter_spacing == 0.0));
    s.commit_text_layout_preview();
    assert_eq!(labels(&s).len(), n + 1, "release: one step for two texts");
    assert_eq!(labels(&s).last().unwrap(), "change_letter_spacing");
    let spaced: Vec<f64> = texts(&s).iter().map(|t| t.layout.letter_spacing).collect();
    assert!(spaced.iter().all(|f| *f > 0.0), "{spaced:?}");

    // Escape during a drag: nothing written, the view shows the stored value again.
    let shown = field_text(&s, LayoutField::WordSpacing).0;
    s.preview_text_layout(LayoutField::WordSpacing, 0.95, Grid::Normal);
    assert_ne!(
        field_text(&s, LayoutField::WordSpacing).0,
        shown,
        "the preview shows"
    );
    s.cancel_text_layout_preview();
    assert_eq!(field_text(&s, LayoutField::WordSpacing).0, shown);
    let _ = s.commit_text_layout_preview();
    assert_eq!(
        labels(&s).len(),
        n + 1,
        "the release after Escape writes nothing"
    );
    assert!(texts(&s).iter().all(|t| t.layout.word_spacing == 0.0));

    // A click (one preview, then release), an arrow (key-down, key-up), the reset.
    for (field, label) in [
        (LayoutField::WordSpacing, "change_word_spacing"),
        (LayoutField::LineSpacing, "change_line_spacing"),
        (LayoutField::CharacterRotation, "change_character_rotation"),
    ] {
        let m = labels(&s).len();
        s.preview_text_layout(field, 0.8, Grid::Normal);
        s.commit_text_layout_preview();
        assert_eq!(labels(&s).len(), m + 1, "{field:?} click");
        assert_eq!(labels(&s).last().unwrap(), label);
        s.step_text_layout(field, 1, Grid::Normal);
        s.step_text_layout(field, 1, Grid::Normal);
        s.commit_text_layout_preview();
        assert_eq!(labels(&s).len(), m + 2, "{field:?} held arrow: one step");
        s.reset_text_layout(field);
        assert_eq!(labels(&s).len(), m + 3, "{field:?} reset");
    }
    let t = texts(&s);
    assert!(
        t.iter()
            .all(|t| t.layout.word_spacing == 0.0 && t.layout.line_spacing.is_none())
    );
    // A reset of a field already at default writes nothing.
    let m = labels(&s).len();
    s.reset_text_layout(LayoutField::WordSpacing);
    assert_eq!(labels(&s).len(), m);
    // Undo takes back one step for both texts.
    s.undo();
    assert!(
        texts(&s)
            .iter()
            .all(|t| t.layout.rotation.as_radians() != 0.0),
        "undo restores both"
    );
}

/// Criterion 13: texts of different Size with the same mm read as one value; different values
/// read Mixed; a typed value is set in mm on each, held per text and counted.
#[test]
fn ac13_several_texts_mixed_and_limited() {
    let mut s = fresh();
    add_text(&mut s, pt(10.0, 20.0), "Small");
    s.set_text_size(10.0);
    add_text(&mut s, pt(10.0, 80.0), "Large");
    s.set_text_size(20.0);
    select_all(&mut s);
    assert_eq!(
        s.set_text_layout_text(LayoutField::LetterSpacing, "2"),
        LayoutOutcome::Set
    );
    assert_eq!(text_named(&s, "Small").layout.letter_spacing, 0.2);
    assert_eq!(text_named(&s, "Large").layout.letter_spacing, 0.1);
    assert_eq!(
        field_text(&s, LayoutField::LetterSpacing),
        ("2".to_owned(), false)
    );
    assert_eq!(
        s.set_text_layout_text(LayoutField::LetterSpacing, "30"),
        LayoutOutcome::Limited { limited: 1, of: 2 }
    );
    assert_eq!(text_named(&s, "Small").layout.letter_spacing, 2.0);
    assert_eq!(text_named(&s, "Large").layout.letter_spacing, 1.5);
    let (text, mixed) = field_text(&s, LayoutField::LetterSpacing);
    assert!(mixed && text.is_empty(), "{text:?}");
    // The track uses the smallest Size: at Size 10, 1 x Size = 10 mm.
    let v = &s.text_layout_block().unwrap()[0];
    assert!((v.track_max - 10.0).abs() < 1e-9, "{v:?}");
}

/// Criterion 14: in edit mode a field change keeps edit mode, applies to the whole text, keeps the
/// caret, and is its own step.
#[test]
fn ac14_edit_mode_stays_and_own_step() {
    let mut s = fresh();
    add_text(&mut s, pt(10.0, 20.0), "Live");
    s.escape();
    s.set_tool(Tool::Text);
    click(&mut s, pt(14.0, 16.0));
    assert!(s.text_editing());
    s.text_input(TextInput::Move {
        to: curvyo_ui_core::CaretMove::LineEnd,
        extend: false,
    });
    let caret = s.text_sink_state().unwrap().1;
    let n = labels(&s).len();
    s.preview_text_layout(LayoutField::LetterSpacing, 0.6, Grid::Normal);
    s.commit_text_layout_preview();
    assert!(s.text_editing(), "edit mode stays");
    assert_eq!(s.text_sink_state().unwrap().1, caret, "caret where it was");
    assert_eq!(labels(&s).len(), n + 1, "its own step");
    assert_eq!(labels(&s).last().unwrap(), "change_letter_spacing");
    s.set_text_alignment(TextAlign::Centre);
    assert!(s.text_editing());
    s.text_input(TextInput::Insert("ly".into()));
    assert_eq!(s.escape(), EscapeStep::ClearedState);
    let t = text_named(&s, "Lively");
    assert!(t.layout.letter_spacing > 0.0);
    assert_eq!(t.layout.align, TextAlign::Centre);
    let after = labels(&s);
    assert_eq!(
        after.len(),
        n + 3,
        "spacing, alignment and the edit: three steps: {after:?}"
    );
}

// ---------------------------------------------------------------------------------------------
// Part G
// ---------------------------------------------------------------------------------------------

/// Criterion 33: with no text selected and no edit mode the alignment is the next text's, no step;
/// the next text uses it; the block's values start at their defaults.
#[test]
fn ac33_next_text_alignment() {
    let mut s = fresh();
    add_text(&mut s, pt(10.0, 20.0), "Old");
    deselect(&mut s);
    let n = labels(&s).len();
    s.set_text_alignment(TextAlign::Right);
    assert_eq!(labels(&s).len(), n, "no step");
    assert_eq!(s.text_alignment(), Some(TextAlign::Right));
    assert_eq!(
        text_named(&s, "Old").layout.align,
        TextAlign::Left,
        "the old text keeps its own"
    );
    start_text(&mut s, pt(100.0, 100.0), "New");
    let block = s.text_layout_block().unwrap();
    assert_eq!(block[0].text, "0");
    assert!(block[2].auto);
    s.escape();
    let t = text_named(&s, "New");
    assert_eq!(
        t.layout,
        TextLayoutValues {
            align: TextAlign::Right,
            ..D
        }
    );
    // The click point is the anchor: a right-aligned line ends there.
    with_sans(|face| {
        let l = engine(&t, face);
        let end = t.placement.apply_point(pt(l.lines[0].extent.end.0, 0.0));
        assert!((end.x - 100.0).abs() < 0.5, "{end:?}");
    });
}

/// Criterion 34: a uniform resize by the S entry changes Size; every factor stays, so the mm
/// readouts grow by the same factor; the rotation stays.
#[test]
fn ac34_uniform_resize_keeps_the_factors() {
    let mut s = fresh();
    add_text(&mut s, pt(20.0, 20.0), "Sign");
    s.set_text_size(10.0);
    s.set_text_layout_text(LayoutField::LetterSpacing, "1.5");
    s.set_text_layout_text(LayoutField::CharacterRotation, "20");
    let before = text_named(&s, "Sign");
    select_all(&mut s);
    key(&mut s, "s", false);
    let chip = s.transform_entry().expect("S shows the box");
    let w: f64 = chip.fields[0].prefill.parse().unwrap();
    let h: f64 = chip.fields[1].prefill.parse().unwrap();
    s.cancel_transform_entry();
    // A corner drag with the proportional modifier, doubling about the opposite corner.
    let corner = pt(chip.center.x + w / 2.0, chip.center.y + h / 2.0);
    let to = pt(corner.x + w, corner.y + h);
    s.pointer_hover(corner, false, true);
    s.pointer_down(corner, false);
    s.pointer_hover(pt(corner.x + w / 2.0, corner.y + h / 2.0), false, true);
    s.pointer_hover(to, false, true);
    s.pointer_up(to, false, true);
    let after = text_named(&s, "Sign");
    assert!(
        (after.size.as_mm() - 20.0).abs() < 0.2,
        "Size doubles: {}",
        after.size.as_mm()
    );
    assert_eq!(after.layout, before.layout, "factors and rotation stay");
    assert_eq!(field_text(&s, LayoutField::LetterSpacing).0, "3");
    assert_eq!(field_text(&s, LayoutField::CharacterRotation).0, "20");
}

/// Criterion 35: Object to path of a text with every value gives one path per glyph that match the
/// drawn outlines within 0.01 mm.
#[test]
fn ac35_object_to_path_equals_the_layout() {
    let mut s = fresh();
    add_text(&mut s, pt(30.0, 40.0), "Ab c\nde");
    s.set_text_size(12.0);
    s.set_text_alignment(TextAlign::Justify);
    s.set_text_layout_text(LayoutField::LetterSpacing, "1");
    s.set_text_layout_text(LayoutField::WordSpacing, "2.5");
    s.set_text_layout_text(LayoutField::LineSpacing, "20");
    s.set_text_layout_text(LayoutField::CharacterRotation, "-30");
    let t = text_named(&s, "Ab c\nde");
    assert!(!t.layout.is_default());
    let want: Vec<Vec<Point>> = with_sans(|face| {
        engine(&t, face)
            .glyph_paths(face)
            .iter()
            .map(|p| {
                p.elements()
                    .iter()
                    .filter_map(kurbo::PathEl::end_point)
                    .map(|q| t.placement.apply_point(pt(q.x, q.y)))
                    .collect()
            })
            .collect()
    });
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
    assert_eq!(paths.len(), want.len(), "one path per glyph");
    let all_want: Vec<Point> = want.concat();
    for p in paths.concat() {
        assert!(
            all_want
                .iter()
                .any(|w| (w.x - p.x).abs() < TOL && (w.y - p.y).abs() < TOL),
            "converted anchor {p:?} is not on the laid-out outline"
        );
    }
}

/// Criterion 36: a text whose face is missing keeps its values; opening, drawing and saving do
/// not rewrite them; a change by the maker is written.
#[test]
fn ac36_missing_face_keeps_the_values() {
    let d = Document::new(5);
    let values = TextLayoutValues {
        align: TextAlign::Centre,
        letter_spacing: 0.12,
        word_spacing: 0.3,
        line_spacing: Some(1.7),
        rotation: curvyo_document_core::Angle::from_radians(0.25),
        ..TextLayoutValues::DEFAULT
    };
    let missing = FontRef {
        family: "Nowhere Sans".to_owned(),
        style: "Regular".to_owned(),
        face_hash: "d".repeat(64),
        face_index: 0,
        kind: FontKind::Outline,
    };
    d.create_text(
        "Lost\nface",
        &missing,
        Length::from_mm(10.0),
        Affine::translation(Vec2::new(30.0, 30.0)),
        &values,
    )
    .unwrap();
    let mut s = with_fonts(Session::open(9, &pack(&d, "0.1.0").unwrap()).unwrap());
    let _ = s.draw_list();
    select_all(&mut s);
    load(&mut s);
    let _ = s.draw_list();
    assert!(s.text_bar().missing, "the face is missing");
    let n = labels(&s).len();
    let t = text_named(&s, "Lost\nface");
    assert_eq!(t.layout, values, "drawing never rewrites the values");
    assert_eq!(labels(&s).len(), n);
    assert_eq!(field_text(&s, LayoutField::LetterSpacing).0, "1.2");
    assert_eq!(
        s.set_text_layout_text(LayoutField::LetterSpacing, "2"),
        LayoutOutcome::Set
    );
    assert_eq!(text_named(&s, "Lost\nface").layout.letter_spacing, 0.2);
    assert_eq!(
        text_named(&s, "Lost\nface").font,
        missing,
        "the reference stays"
    );
}

/// Criterion 37: a saved and reopened project shows the same values in the block, editable.
#[test]
fn ac37_save_and_reopen_in_the_session() {
    let mut s = fresh();
    add_text(&mut s, pt(20.0, 20.0), "Keep\nme");
    s.set_text_alignment(TextAlign::Centre);
    s.set_text_layout_text(LayoutField::LetterSpacing, "0.5");
    s.set_text_layout_text(LayoutField::LineSpacing, "18");
    s.set_text_layout_text(LayoutField::CharacterRotation, "-7.5");
    let views = s.text_layout_block().unwrap();
    let before = text_named(&s, "Keep\nme");
    let mut s = with_fonts(Session::open(23, &s.pack("0.1.0").unwrap()).unwrap());
    select_all(&mut s);
    assert_eq!(text_named(&s, "Keep\nme").layout, before.layout);
    assert_eq!(s.text_alignment(), Some(TextAlign::Centre));
    let again = s.text_layout_block().unwrap();
    for (a, b) in views.iter().zip(&again) {
        assert_eq!(a.text, b.text);
    }
    assert_eq!(
        s.set_text_layout_text(LayoutField::CharacterRotation, "0"),
        LayoutOutcome::Set
    );
    assert_eq!(text_named(&s, "Keep\nme").layout.rotation.as_radians(), 0.0);
}

/// UX notes §5 (white-box): a drag in flight when the subject changes is dropped as by Escape;
/// the release then writes nothing to the new subject.
#[test]
fn edge_subject_change_mid_drag_writes_nothing() {
    let mut s = fresh();
    add_text(&mut s, pt(10.0, 20.0), "First");
    add_text(&mut s, pt(10.0, 120.0), "Second");
    s.set_tool(Tool::Select);
    click(&mut s, pt(14.0, 16.0));
    assert_eq!(s.style_scope().ids.len(), 1);
    s.preview_text_layout(LayoutField::LetterSpacing, 0.9, Grid::Normal);
    deselect(&mut s);
    click(&mut s, pt(14.0, 116.0));
    assert_eq!(s.style_scope().ids.len(), 1);
    assert_eq!(
        field_text(&s, LayoutField::LetterSpacing).0,
        "0",
        "the new subject shows its own value, not the old drag's"
    );
    let n = labels(&s).len();
    let _ = s.commit_text_layout_preview();
    assert_eq!(labels(&s).len(), n, "the release writes nothing");
    assert!(texts(&s).iter().all(|t| t.layout.letter_spacing == 0.0));
}
