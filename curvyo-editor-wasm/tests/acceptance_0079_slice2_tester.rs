//! Independent tester cases for `specs/0079-text-layout-controls/specification.md` slice 2 (the
//! taper) at the `Session` level: the Taper field in the block after Character rotation, typed
//! values, limits, drag preview, release, Escape, reset, Mixed and undo (criterion 16), edit mode
//! (14), the next text (33), a uniform resize and a stretch (34), Object to path equals drawing
//! for outline and single-line faces (35), a missing face (36) and save and reopen (37).
//!
//! Written from the specification, `adrs.md` and the public names before the implementation was
//! read; the harness is the one of `acceptance_0079_tester.rs`.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::too_many_lines,
    clippy::float_cmp,
    clippy::many_single_char_names,
    clippy::assert_is_empty,
    dead_code,
    unused_imports,
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

/// The engine options for `t`, computed here from the spec (factor x Size, the taper as its
/// fraction), not by the implementation's mapping.
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
        taper: t.layout.taper,
        ..curvyo_text_core::LayoutOptions::default()
    }
}

fn engine(t: &TextSnapshot, face: &Face) -> curvyo_text_core::TextLayout {
    curvyo_text_core::layout(face, &t.string, Mm(t.size.as_mm()), &mm_options(t)).unwrap()
}

fn field_text(s: &Session, f: LayoutField) -> (String, bool) {
    let block = s.text_layout_block().expect("the block is shown");
    let i = LayoutField::ALL.iter().position(|x| *x == f).unwrap();
    (block[i].text.clone(), block[i].mixed)
}

const TAPER: LayoutField = LayoutField::Taper;

fn taper_of(s: &Session, string: &str) -> f64 {
    text_named(s, string).layout.taper
}

/// Criterion 16: the Taper field sits directly after Character rotation, starts at 0 % with no
/// reset; typed "20" and "20 %" set 20 %; past the range the value is held at 50 % or -50 % and
/// the outcome carries the notice's value; text that is not a number writes nothing; each change
/// is one step "Change taper"; undo takes it back.
#[test]
fn c16_taper_field_typed_values_limits_and_steps() {
    let order = LayoutField::ALL;
    let rot = order
        .iter()
        .position(|f| *f == LayoutField::CharacterRotation)
        .unwrap();
    assert_eq!(order[rot + 1], TAPER, "directly after Character rotation");
    let mut s = fresh();
    add_text(&mut s, pt(10.0, 20.0), "Ring");
    let (text, mixed) = field_text(&s, TAPER);
    assert_eq!((text.as_str(), mixed), ("0", false));
    let block = s.text_layout_block().unwrap();
    assert!(!block[rot + 1].resettable);
    let n = labels(&s).len();
    assert_eq!(s.set_text_layout_text(TAPER, "20"), LayoutOutcome::Set);
    assert!((taper_of(&s, "Ring") - 0.2).abs() < 1e-12);
    assert_eq!(labels(&s).last().unwrap(), "change_taper");
    assert_eq!(s.set_text_layout_text(TAPER, "12.5 %"), LayoutOutcome::Set);
    assert!((taper_of(&s, "Ring") - 0.125).abs() < 1e-12);
    assert_eq!(field_text(&s, TAPER).0, "12.5");
    assert!(s.text_layout_block().unwrap()[rot + 1].resettable);
    assert_eq!(
        s.set_text_layout_text(TAPER, "80"),
        LayoutOutcome::Clamped(true, 50.0)
    );
    assert!((taper_of(&s, "Ring") - 0.5).abs() < 1e-12);
    assert_eq!(
        s.set_text_layout_text(TAPER, "-75 %"),
        LayoutOutcome::Clamped(false, -50.0)
    );
    assert!((taper_of(&s, "Ring") + 0.5).abs() < 1e-12);
    assert_eq!(labels(&s).len(), n + 4);
    assert_eq!(
        s.set_text_layout_text(TAPER, "pointy"),
        LayoutOutcome::Invalid
    );
    assert_eq!(labels(&s).len(), n + 4, "not a number: no step");
    s.undo();
    assert!(
        (taper_of(&s, "Ring") - 0.5).abs() < 1e-12,
        "undo: one step back"
    );
}

/// Criterion 16 (with 12): a drag previews live without writing, the release is one step; Escape
/// during a drag writes nothing; arrows step 1 %, Shift 10 %, Ctrl 0.1 % and a held arrow is one
/// step; the reset returns to 0 % in one step and a reset at 0 % writes nothing; the canvas
/// follows the preview.
#[test]
fn c16_drag_escape_arrows_and_reset() {
    let mut s = fresh();
    add_text(&mut s, pt(10.0, 20.0), "Drag");
    let n = labels(&s).len();
    let before = s.draw_list();
    for p in [0.6, 0.7, 0.8] {
        s.preview_text_layout(TAPER, p, Grid::Normal);
    }
    assert_eq!(labels(&s).len(), n, "a drag writes nothing");
    assert_eq!(taper_of(&s, "Drag"), 0.0);
    assert_ne!(s.draw_list(), before, "live on the canvas while dragged");
    assert_ne!(field_text(&s, TAPER).0, "0", "the field shows the preview");
    s.commit_text_layout_preview();
    assert_eq!(labels(&s).len(), n + 1);
    assert_eq!(labels(&s).last().unwrap(), "change_taper");
    let dragged = taper_of(&s, "Drag");
    assert!(dragged > 0.0 && dragged <= 0.5, "{dragged}");

    s.preview_text_layout(TAPER, 0.05, Grid::Normal);
    s.cancel_text_layout_preview();
    let _ = s.commit_text_layout_preview();
    assert_eq!(labels(&s).len(), n + 1, "Escape: nothing written");
    assert_eq!(taper_of(&s, "Drag"), dragged);

    assert_eq!(s.set_text_layout_text(TAPER, "10"), LayoutOutcome::Set);
    let m = labels(&s).len();
    s.step_text_layout(TAPER, 1, Grid::Normal);
    s.step_text_layout(TAPER, 1, Grid::Normal);
    s.commit_text_layout_preview();
    assert_eq!(labels(&s).len(), m + 1, "held arrow: one step");
    assert!((taper_of(&s, "Drag") - 0.12).abs() < 1e-9);
    assert_eq!(s.set_text_layout_text(TAPER, "30"), LayoutOutcome::Set);
    s.step_text_layout(TAPER, -1, Grid::Coarse);
    s.commit_text_layout_preview();
    assert!((taper_of(&s, "Drag") - 0.2).abs() < 1e-9, "Shift: 10 %");
    s.step_text_layout(TAPER, 1, Grid::Fine);
    s.commit_text_layout_preview();
    assert!((taper_of(&s, "Drag") - 0.201).abs() < 1e-9, "Ctrl: 0.1 %");

    let m = labels(&s).len();
    s.reset_text_layout(TAPER);
    assert_eq!(labels(&s).len(), m + 1);
    assert_eq!(taper_of(&s, "Drag"), 0.0);
    s.reset_text_layout(TAPER);
    assert_eq!(labels(&s).len(), m + 1, "reset at 0 %: nothing");
    assert_eq!(s.draw_list(), before, "back at 0 % draws as before");
}

/// Criterion 16 (with 13): differing tapers read Mixed; a change sets every subject text in one
/// step; texts of different Size get the same percentage.
#[test]
fn c16_several_texts_mixed_and_one_step() {
    let mut s = fresh();
    add_text(&mut s, pt(10.0, 20.0), "One");
    s.set_text_size(5.0);
    s.set_text_layout_text(TAPER, "30");
    add_text(&mut s, pt(10.0, 90.0), "Two");
    s.set_text_size(40.0);
    select_all(&mut s);
    assert!(field_text(&s, TAPER).1, "Mixed");
    let n = labels(&s).len();
    assert_eq!(s.set_text_layout_text(TAPER, "-20"), LayoutOutcome::Set);
    assert_eq!(labels(&s).len(), n + 1, "one step for both");
    assert!((taper_of(&s, "One") + 0.2).abs() < 1e-12);
    assert!((taper_of(&s, "Two") + 0.2).abs() < 1e-12);
    assert_eq!(field_text(&s, TAPER), ("\u{2212}20".to_owned(), false));
    // The range is the same on every text: past it, both are held and the outcome names the
    // limit (the notice "Taper set to 50 %, the largest taper."), not "limited on 2 of 2".
    assert_eq!(
        s.set_text_layout_text(TAPER, "60"),
        LayoutOutcome::Clamped(true, 50.0)
    );
    assert!((taper_of(&s, "One") - 0.5).abs() < 1e-12);
    assert!((taper_of(&s, "Two") - 0.5).abs() < 1e-12);
}

/// Criterion 14 for the Taper: in edit mode the change keeps edit mode and the caret, applies to
/// the whole text and is its own step.
#[test]
fn c14_taper_in_edit_mode() {
    let mut s = fresh();
    add_text(&mut s, pt(10.0, 20.0), "Edit");
    s.escape();
    s.set_tool(Tool::Text);
    click(&mut s, pt(14.0, 16.0));
    assert!(s.text_editing());
    let caret = s.text_sink_state().unwrap().1;
    let n = labels(&s).len();
    assert_eq!(s.set_text_layout_text(TAPER, "15"), LayoutOutcome::Set);
    assert!(s.text_editing(), "edit mode stays");
    assert_eq!(s.text_sink_state().unwrap().1, caret, "the caret stays");
    assert_eq!(labels(&s).len(), n + 1, "its own step");
    assert_eq!(labels(&s).last().unwrap(), "change_taper");
    assert!((taper_of(&s, "Edit") - 0.15).abs() < 1e-12);
}

/// Criterion 33: a new text starts untapered even after a tapered one was made.
#[test]
fn c33_next_text_starts_at_zero() {
    let mut s = fresh();
    add_text(&mut s, pt(10.0, 20.0), "First");
    s.set_text_layout_text(TAPER, "40");
    deselect(&mut s);
    assert!(
        s.text_layout_block().is_none(),
        "no block without a subject"
    );
    add_text(&mut s, pt(10.0, 90.0), "Second");
    assert_eq!(taper_of(&s, "Second"), 0.0);
    assert_eq!(field_text(&s, TAPER).0, "0");
    assert!((taper_of(&s, "First") - 0.4).abs() < 1e-12);
}

/// Criterion 34: a uniform corner resize changes Size and keeps the taper; the field still reads
/// the same percentage. A non-uniform stretch keeps the taper too.
#[test]
fn c34_uniform_resize_and_stretch_keep_the_taper() {
    let mut s = fresh();
    add_text(&mut s, pt(20.0, 20.0), "Sign");
    s.set_text_size(10.0);
    s.set_text_layout_text(TAPER, "-25");
    select_all(&mut s);
    key(&mut s, "s", false);
    let chip = s.transform_entry().expect("S shows the box");
    let w: f64 = chip.fields[0].prefill.parse().unwrap();
    let h: f64 = chip.fields[1].prefill.parse().unwrap();
    s.cancel_transform_entry();
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
    assert_eq!(after.layout.taper, -0.25);
    assert_eq!(field_text(&s, TAPER).0, "\u{2212}25");

    // A stretch: the right edge's middle handle dragged out (no proportional modifier).
    let chip = {
        key(&mut s, "s", false);
        let c = s.transform_entry().unwrap();
        s.cancel_transform_entry();
        c
    };
    let w: f64 = chip.fields[0].prefill.parse().unwrap();
    let edge = pt(chip.center.x + w / 2.0, chip.center.y);
    let to = pt(edge.x + w / 2.0, edge.y);
    s.pointer_hover(edge, false, false);
    s.pointer_down(edge, false);
    s.pointer_hover(pt(edge.x + w / 4.0, edge.y), false, false);
    s.pointer_hover(to, false, false);
    s.pointer_up(to, false, false);
    let stretched = text_named(&s, "Sign");
    assert_eq!(stretched.layout.taper, -0.25, "a stretch keeps every value");
}

/// Criterion 35: Object to path of a tapered, turned text gives one path per glyph whose anchors
/// lie on the engine's tapered outline within 0.01 mm.
#[test]
fn c35_object_to_path_of_a_tapered_text() {
    let mut s = fresh();
    add_text(&mut s, pt(30.0, 40.0), "Ring S");
    s.set_text_size(15.0);
    s.set_text_layout_text(TAPER, "35");
    s.set_text_layout_text(LayoutField::CharacterRotation, "-20");
    let t = text_named(&s, "Ring S");
    let want: Vec<Point> = with_sans(|face| {
        engine(&t, face)
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
    let glyphs = with_sans(|face| engine(&t, face).glyph_paths(face).len());
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
    assert_eq!(paths.len(), glyphs, "one path per glyph");
    for p in paths.concat() {
        assert!(
            want.iter()
                .any(|w| (w.x - p.x).abs() < TOL && (w.y - p.y).abs() < TOL),
            "converted anchor {p:?} is not on the tapered outline"
        );
    }
}

/// Criterion 35 with `0076`: a tapered single-line text converts to open paths.
#[test]
fn c35_single_line_tapered_text_gives_open_paths() {
    let mut s = fresh();
    add_text(&mut s, pt(30.0, 40.0), "HLT");
    s.pick_font_face("hershey-sans-1/HersheySans1.svg", 0);
    load(&mut s);
    s.set_text_layout_text(TAPER, "30");
    assert!((taper_of(&s, "HLT") - 0.3).abs() < 1e-12);
    assert_eq!(s.object_to_path(), ObjectToPathOutcome::Converted);
    let d = doc_of(&s);
    let paths: Vec<_> = d
        .object_ids()
        .into_iter()
        .filter_map(|id| match d.object(id) {
            Some(ObjectSnapshot::Path(p)) => Some(p),
            _ => None,
        })
        .collect();
    assert_eq!(paths.len(), 3, "one path per glyph");
    for p in &paths {
        assert!(!p.closed, "open strokes");
        assert!(p.extra_subpaths.iter().all(|sp| !sp.closed));
    }
}

/// Criterion 36: a text whose face is missing keeps its taper through open, draw and save.
#[test]
fn c36_missing_face_keeps_the_taper() {
    let d = Document::new(5);
    let values = TextLayoutValues {
        taper: -0.4,
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
        "Lost",
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
    let n = labels(&s).len();
    assert_eq!(text_named(&s, "Lost").layout.taper, -0.4);
    assert_eq!(labels(&s).len(), n, "drawing writes nothing");
    assert_eq!(field_text(&s, TAPER).0, "\u{2212}40");
    let saved = unpack(4, &s.pack("0.1.0").unwrap()).unwrap();
    let t = saved
        .object_ids()
        .into_iter()
        .find_map(|id| saved.text(id))
        .unwrap();
    assert_eq!(t.layout.taper, -0.4);
}

/// Criterion 37: save, close and open: the taper is there, shown and editable.
#[test]
fn c37_save_and_reopen_keeps_the_taper() {
    let mut s = fresh();
    add_text(&mut s, pt(20.0, 20.0), "Keep");
    s.set_text_layout_text(TAPER, "-12.5");
    s.set_text_layout_text(LayoutField::CharacterRotation, "10");
    let before = text_named(&s, "Keep");
    let draw = s.draw_list();
    let mut s = with_fonts(Session::open(23, &s.pack("0.1.0").unwrap()).unwrap());
    select_all(&mut s);
    assert_eq!(text_named(&s, "Keep").layout, before.layout);
    assert_eq!(field_text(&s, TAPER).0, "\u{2212}12.5");
    let _ = draw;
    assert_eq!(s.set_text_layout_text(TAPER, "0"), LayoutOutcome::Set);
    assert_eq!(taper_of(&s, "Keep"), 0.0);
}
