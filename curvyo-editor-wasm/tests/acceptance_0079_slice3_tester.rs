//! Independent tester cases for `specs/0079-text-layout-controls/specification.md` slice 3 at the
//! `Session` level: the pressed writing mode and orientation, one step each, Undo and the text in
//! place (criterion 20), the orientation kept on a horizontal text (21), caret keys and typing in
//! a column (24), the next text (33), Object to path equals drawing in the CJK fixture (35), a
//! missing face (36), save and reopen (37) and the `0082` glyph insert into a vertical text.
//!
//! Written from the specification, `adrs.md` and the public names before the implementation was
//! read.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::too_many_lines,
    clippy::float_cmp,
    clippy::many_single_char_names,
    missing_docs
)]

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use curvyo_document_core::{
    Affine, Document, FontKind, FontRef, Length, ObjectSnapshot, Point, TextAlign,
    TextGlyphOrientation, TextLayoutValues, TextSnapshot, TextWritingMode, Timestamp, Vec2, pack,
    unpack,
};
use curvyo_editor_wasm::{EscapeStep, KeyInput, ObjectToPathOutcome, Session, Tool};
use curvyo_text_core::{Face, FaceStore, GlyphOrientation, LayoutOptions, Mm, WritingMode};
use curvyo_ui_core::{CaretMove, FontBlock, TextInput};
use serde_json::Value;

static NOW: AtomicU64 = AtomicU64::new(1_990_000_000);
const TOL: f64 = 0.01;
const D: TextLayoutValues = TextLayoutValues::DEFAULT;
const NOTO: &str = "tester-vertical/NotoSansJP-Subset.otf";

fn clock() -> Timestamp {
    Timestamp::from_secs(NOW.fetch_add(3, Ordering::SeqCst))
}

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn fonts_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../frontend/public/fonts")
}

fn noto_bytes() -> Vec<u8> {
    std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../curvyo-text-core/tests/fixtures/vertical/NotoSansJP-Subset.otf"),
    )
    .unwrap()
}

fn included_faces() -> Vec<(String, String, String, u32, FontKind)> {
    let index: Value =
        serde_json::from_str(&std::fs::read_to_string(fonts_dir().join("index.json")).unwrap())
            .unwrap();
    let mut faces: Vec<_> = index["faces"]
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
        .collect();
    faces.push((
        NOTO.to_owned(),
        "Noto Sans JP".to_owned(),
        "Regular".to_owned(),
        0,
        FontKind::Outline,
    ));
    faces
}

fn load(s: &mut Session) {
    for file in s.wanted_font_files() {
        let bytes = if file == NOTO {
            noto_bytes()
        } else {
            std::fs::read(fonts_dir().join(&file)).unwrap()
        };
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
    with_fonts(Session::new(793))
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

fn add_text(s: &mut Session, at: Point, string: &str) {
    start_text(s, at, string);
    assert_eq!(s.escape(), EscapeStep::ClearedState);
    load(s);
}

fn select_all(s: &mut Session) {
    s.set_tool(Tool::Select);
    let _ = s.key_down(KeyInput {
        key: "a",
        ctrl: true,
        ..KeyInput::default()
    });
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

fn with_face<R>(t: &TextSnapshot, f: impl FnOnce(&Face) -> R) -> R {
    let bytes = if t.font.family == "Noto Sans JP" {
        noto_bytes()
    } else {
        assert_eq!(
            (t.font.family.as_str(), t.font.style.as_str()),
            ("Source Sans 3", "Regular")
        );
        std::fs::read(fonts_dir().join("source-sans-3/SourceSans3-Regular.otf")).unwrap()
    };
    let mut store = FaceStore::new();
    let key = store.add(bytes, 0).unwrap();
    f(store.get(&key).unwrap())
}

/// The engine options for `t`, mapped here from the spec, not by the implementation.
fn mm_options(t: &TextSnapshot) -> LayoutOptions {
    let size = t.size.as_mm();
    LayoutOptions {
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
        writing_mode: match t.layout.writing_mode {
            TextWritingMode::Horizontal => WritingMode::Horizontal,
            TextWritingMode::VerticalRl => WritingMode::VerticalRl,
            TextWritingMode::VerticalLr => WritingMode::VerticalLr,
        },
        orientation: match t.layout.orientation {
            TextGlyphOrientation::Auto => GlyphOrientation::Auto,
            TextGlyphOrientation::Upright => GlyphOrientation::Upright,
            TextGlyphOrientation::Sideways => GlyphOrientation::Sideways,
        },
        ..LayoutOptions::default()
    }
}

fn engine(t: &TextSnapshot, face: &Face) -> curvyo_text_core::TextLayout {
    curvyo_text_core::layout(face, &t.string, Mm(t.size.as_mm()), &mm_options(t)).unwrap()
}

/// The document point of the first line's start (extent start on its baseline, through the
/// frame): where criterion 20 keeps the text.
fn first_start(t: &TextSnapshot) -> Point {
    with_face(t, |face| {
        let l = engine(t, face);
        let p = l.frame() * kurbo::Point::new(l.lines[0].extent.start.0, l.lines[0].baseline.0);
        t.placement.apply_point(pt(p.x, p.y))
    })
}

fn near(a: Point, b: Point) -> bool {
    (a.x - b.x).abs() < TOL && (a.y - b.y).abs() < TOL
}

fn caret(s: &Session) -> usize {
    let (_, a, b) = s.text_sink_state().unwrap();
    assert_eq!(a, b, "no range");
    b
}

fn mv(s: &mut Session, to: CaretMove) -> usize {
    s.text_input(TextInput::Move { to, extend: false });
    caret(s)
}

fn place(s: &mut Session, at: usize) {
    s.text_input(TextInput::Select {
        anchor: at,
        caret: at,
    });
}

/// Criterion 20: Horizontal and Auto are the defaults; a press on selected texts is one step
/// "Change writing mode" / "Change glyph orientation" for all of them; a press on the pressed
/// button writes nothing; differing texts press none; Undo restores values and place.
#[test]
fn ac20_presses_one_step_each_and_undo() {
    let mut s = fresh();
    assert_eq!(s.text_writing_mode(), Some(TextWritingMode::Horizontal));
    assert_eq!(s.text_glyph_orientation(), Some(TextGlyphOrientation::Auto));
    add_text(&mut s, pt(20.0, 20.0), "AB\nCD");
    add_text(&mut s, pt(20.0, 120.0), "Wide line\nx");
    select_all(&mut s);
    let before = texts(&s);
    let n = labels(&s).len();
    s.set_text_writing_mode(TextWritingMode::VerticalRl);
    assert_eq!(labels(&s).len(), n + 1, "one step for two texts");
    assert_eq!(labels(&s).last().unwrap(), "change_writing_mode");
    assert!(
        texts(&s)
            .iter()
            .all(|t| t.layout.writing_mode == TextWritingMode::VerticalRl)
    );
    assert_eq!(s.text_writing_mode(), Some(TextWritingMode::VerticalRl));
    s.set_text_writing_mode(TextWritingMode::VerticalRl);
    assert_eq!(labels(&s).len(), n + 1, "the pressed one writes nothing");
    s.set_text_glyph_orientation(TextGlyphOrientation::Upright);
    assert_eq!(labels(&s).len(), n + 2);
    assert_eq!(labels(&s).last().unwrap(), "change_glyph_orientation");
    assert_eq!(
        s.text_glyph_orientation(),
        Some(TextGlyphOrientation::Upright)
    );
    s.undo();
    assert!(
        texts(&s)
            .iter()
            .all(|t| t.layout.orientation == TextGlyphOrientation::Auto)
    );
    s.undo();
    for (b, u) in before.iter().zip(&texts(&s)) {
        assert_eq!(u.layout, b.layout);
        assert_eq!(u.placement, b.placement);
    }
    // Differing texts: none pressed.
    deselect(&mut s);
    add_text(&mut s, pt(150.0, 20.0), "Third");
    s.set_text_writing_mode(TextWritingMode::VerticalLr);
    select_all(&mut s);
    assert_eq!(s.text_writing_mode(), None);
}

/// Criterion 20: a change keeps the text in place: the first column's start is where the first
/// line's start was, for every alignment, and an orientation change keeps the column's start.
#[test]
fn ac20_text_stays_in_place() {
    for align in [TextAlign::Left, TextAlign::Centre, TextAlign::Right] {
        let mut s = fresh();
        s.set_text_alignment(align);
        add_text(&mut s, pt(40.0, 40.0), "Ab cd\nxyz");
        let a = text_named(&s, "Ab cd\nxyz");
        s.set_text_writing_mode(TextWritingMode::VerticalRl);
        let b = text_named(&s, "Ab cd\nxyz");
        assert!(near(first_start(&a), first_start(&b)), "{align:?}");
        s.set_text_glyph_orientation(TextGlyphOrientation::Upright);
        let c = text_named(&s, "Ab cd\nxyz");
        assert!(near(first_start(&b), first_start(&c)), "{align:?}");
        s.set_text_writing_mode(TextWritingMode::VerticalLr);
        let d = text_named(&s, "Ab cd\nxyz");
        assert!(near(first_start(&c), first_start(&d)), "{align:?}");
        s.set_text_writing_mode(TextWritingMode::Horizontal);
        let e = text_named(&s, "Ab cd\nxyz");
        assert!(near(first_start(&a), first_start(&e)), "{align:?} back");
    }
}

/// Criterion 21: on a horizontal text the stored orientation is kept, shown as pressed, does not
/// change the drawing, and takes effect when the text is set vertical.
#[test]
fn ac21_orientation_kept_on_horizontal_text() {
    let mut s = fresh();
    add_text(&mut s, pt(30.0, 30.0), "Keep");
    s.set_text_glyph_orientation(TextGlyphOrientation::Sideways);
    let t = text_named(&s, "Keep");
    assert_eq!(t.layout.writing_mode, TextWritingMode::Horizontal);
    assert_eq!(t.layout.orientation, TextGlyphOrientation::Sideways);
    assert_eq!(
        s.text_glyph_orientation(),
        Some(TextGlyphOrientation::Sideways)
    );
    // Saved and reopened, still kept.
    let mut r = with_fonts(Session::open(9, &s.pack("0.1.0").unwrap()).unwrap());
    select_all(&mut r);
    assert_eq!(
        r.text_glyph_orientation(),
        Some(TextGlyphOrientation::Sideways)
    );
    r.set_text_writing_mode(TextWritingMode::VerticalRl);
    let v = text_named(&r, "Keep");
    assert_eq!(
        (v.layout.writing_mode, v.layout.orientation),
        (TextWritingMode::VerticalRl, TextGlyphOrientation::Sideways)
    );
}

/// Criterion 24: in a column Up and Down move along it, Left and Right to the previous or next
/// column in reading order (right to left: Left is the next line), Home and End to the top and
/// bottom of the column.
#[test]
fn ac24_caret_keys_in_columns() {
    let mut s = fresh();
    s.set_text_writing_mode(TextWritingMode::VerticalRl);
    start_text(&mut s, pt(80.0, 30.0), "abc\ndef");
    assert_eq!(caret(&s), 7, "caret after the typed text");
    assert_eq!(
        mv(&mut s, CaretMove::LineStart),
        4,
        "Home: top of the column"
    );
    assert_eq!(mv(&mut s, CaretMove::Down), 5, "Down along the column");
    assert_eq!(mv(&mut s, CaretMove::Down), 6);
    assert_eq!(mv(&mut s, CaretMove::Up), 5, "Up along the column");
    assert_eq!(
        mv(&mut s, CaretMove::LineEnd),
        7,
        "End: bottom of the column"
    );
    place(&mut s, 1);
    assert_eq!(
        mv(&mut s, CaretMove::Left),
        5,
        "RL: Left goes to the next line"
    );
    assert_eq!(mv(&mut s, CaretMove::Right), 1, "RL: Right goes back");
    assert_eq!(s.escape(), EscapeStep::ClearedState);

    let mut s = fresh();
    s.set_text_writing_mode(TextWritingMode::VerticalLr);
    start_text(&mut s, pt(80.0, 30.0), "abc\ndef");
    place(&mut s, 2);
    assert_eq!(
        mv(&mut s, CaretMove::Right),
        6,
        "LR: Right goes to the next line"
    );
    assert_eq!(mv(&mut s, CaretMove::Left), 2, "LR: Left goes back");
}

/// Criterion 24 with `0074`: typing in a vertical text keeps the anchor (the column's top stays at
/// the click) and the caret at the typed position; the string grows down the column.
#[test]
fn ac24_typing_in_a_column() {
    let mut s = fresh();
    s.set_text_writing_mode(TextWritingMode::VerticalRl);
    start_text(&mut s, pt(60.0, 40.0), "AB");
    s.text_input(TextInput::Insert("CD".into()));
    assert_eq!(caret(&s), 4);
    place(&mut s, 1);
    s.text_input(TextInput::Insert("x".into()));
    assert_eq!(caret(&s), 2);
    assert_eq!(s.escape(), EscapeStep::ClearedState);
    let t = text_named(&s, "AxBCD");
    assert_eq!(t.layout.writing_mode, TextWritingMode::VerticalRl);
    with_face(&t, |face| {
        let l = engine(&t, face);
        let b = l.line_box();
        let top = t.placement.apply_point(pt(b.center().x, b.y0));
        assert!(
            (top.x - 60.0).abs() < 0.5 && (top.y - 40.0).abs() < 0.5,
            "the column's top centre is the click: {top:?}"
        );
        assert!(b.height() > b.width(), "a column: {b:?}");
    });
    // Undo removes the typing as `0074` does (one "Add text" step for a new text).
    let n = labels(&s).len();
    s.undo();
    assert!(
        texts(&s).is_empty(),
        "{n} steps, the text is gone after Undo"
    );
}

/// Criterion 33: with no subject the presses set the next text's values, write no step, and the
/// next text uses them; the old text keeps its own.
#[test]
fn ac33_next_text_writing_values() {
    let mut s = fresh();
    add_text(&mut s, pt(10.0, 20.0), "Old");
    deselect(&mut s);
    let n = labels(&s).len();
    s.set_text_writing_mode(TextWritingMode::VerticalLr);
    s.set_text_glyph_orientation(TextGlyphOrientation::Upright);
    assert_eq!(labels(&s).len(), n, "no step");
    assert_eq!(s.text_writing_mode(), Some(TextWritingMode::VerticalLr));
    assert_eq!(
        s.text_glyph_orientation(),
        Some(TextGlyphOrientation::Upright)
    );
    assert_eq!(text_named(&s, "Old").layout, D);
    add_text(&mut s, pt(100.0, 100.0), "New");
    let t = text_named(&s, "New");
    assert_eq!(
        (t.layout.writing_mode, t.layout.orientation),
        (TextWritingMode::VerticalLr, TextGlyphOrientation::Upright)
    );
}

/// Criterion 35 with 23: Object to path of a vertical CJK and Latin text (the fixture face, Auto)
/// gives one path per glyph whose anchors lie on the laid-out outlines.
#[test]
fn ac35_object_to_path_in_columns() {
    let mut s = fresh();
    s.set_text_writing_mode(TextWritingMode::VerticalRl);
    start_text(&mut s, pt(80.0, 40.0), "日本A");
    s.pick_font_family(NOTO, 0);
    load(&mut s);
    s.text_input(TextInput::Insert("\n語「x」".into()));
    assert_eq!(s.escape(), EscapeStep::ClearedState);
    load(&mut s);
    let t = text_named(&s, "日本A\n語「x」");
    assert_eq!(t.font.family, "Noto Sans JP");
    assert_eq!(t.layout.writing_mode, TextWritingMode::VerticalRl);
    let want: Vec<Point> = with_face(&t, |face| {
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
    assert_eq!(paths.len(), 7, "one path per glyph");
    for p in paths.concat() {
        assert!(
            want.iter().any(|w| near(*w, p)),
            "converted anchor {p:?} is not on the laid-out outline"
        );
    }
}

/// Criterion 36: a text whose face is missing keeps its writing values; drawing never rewrites
/// them and the pressed state shows them.
#[test]
fn ac36_missing_face_keeps_writing_values() {
    let d = Document::new(5);
    let values = TextLayoutValues {
        writing_mode: TextWritingMode::VerticalLr,
        orientation: TextGlyphOrientation::Upright,
        ..D
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
    let n = labels(&s).len();
    assert_eq!(text_named(&s, "Lost\nface").layout, values);
    assert_eq!(labels(&s).len(), n);
    assert_eq!(s.text_writing_mode(), Some(TextWritingMode::VerticalLr));
    assert_eq!(
        s.text_glyph_orientation(),
        Some(TextGlyphOrientation::Upright)
    );
    s.set_text_writing_mode(TextWritingMode::VerticalRl);
    assert_eq!(
        text_named(&s, "Lost\nface").layout.writing_mode,
        TextWritingMode::VerticalRl
    );
}

/// Criterion 37: a saved and reopened project shows the same writing values, editable.
#[test]
fn ac37_save_and_reopen_in_the_session() {
    let mut s = fresh();
    add_text(&mut s, pt(20.0, 20.0), "Keep\nme");
    s.set_text_writing_mode(TextWritingMode::VerticalRl);
    s.set_text_glyph_orientation(TextGlyphOrientation::Sideways);
    let before = text_named(&s, "Keep\nme");
    let mut r = with_fonts(Session::open(11, &s.pack("0.1.0").unwrap()).unwrap());
    let after = text_named(&r, "Keep\nme");
    assert_eq!(after.layout, before.layout);
    assert_eq!(after.placement, before.placement);
    select_all(&mut r);
    assert_eq!(r.text_writing_mode(), Some(TextWritingMode::VerticalRl));
    assert_eq!(
        r.text_glyph_orientation(),
        Some(TextGlyphOrientation::Sideways)
    );
    r.set_text_glyph_orientation(TextGlyphOrientation::Auto);
    assert_eq!(
        text_named(&r, "Keep\nme").layout.orientation,
        TextGlyphOrientation::Auto
    );
}

/// Regression `0082`: a glyph from the glyph table goes in at the caret of a vertical text in
/// edit mode, as typed; the text stays vertical.
#[test]
fn regression_0082_glyph_insert_into_a_vertical_text() {
    let mut s = fresh();
    s.set_text_writing_mode(TextWritingMode::VerticalRl);
    start_text(&mut s, pt(50.0, 50.0), "AC");
    place(&mut s, 1);
    assert!(s.insert_from_glyph_table("B").editing);
    assert_eq!(caret(&s), 2);
    assert_eq!(s.escape(), EscapeStep::ClearedState);
    let t = text_named(&s, "ABC");
    assert_eq!(t.layout.writing_mode, TextWritingMode::VerticalRl);
    // With the text selected: edit mode at its end, then the glyph.
    select_all(&mut s);
    assert!(s.insert_from_glyph_table("D").editing);
    assert_eq!(s.escape(), EscapeStep::ClearedState);
    assert_eq!(
        text_named(&s, "ABCD").layout.writing_mode,
        TextWritingMode::VerticalRl
    );
}

/// Criterion 20 in edit mode (subject = the edited text, as `0079` criterion 14): a press keeps
/// edit mode and the caret, the edited text takes the value, for a new text and an existing one.
#[test]
fn ac20_press_in_edit_mode() {
    let mut s = fresh();
    start_text(&mut s, pt(60.0, 40.0), "New");
    s.set_text_writing_mode(TextWritingMode::VerticalLr);
    assert!(s.text_editing(), "edit mode stays");
    assert_eq!(caret(&s), 3);
    assert_eq!(s.text_writing_mode(), Some(TextWritingMode::VerticalLr));
    s.text_input(TextInput::Insert("er".into()));
    assert_eq!(s.escape(), EscapeStep::ClearedState);
    assert_eq!(
        text_named(&s, "Newer").layout.writing_mode,
        TextWritingMode::VerticalLr
    );
    // An existing text in edit mode.
    deselect(&mut s);
    s.set_text_writing_mode(TextWritingMode::Horizontal);
    add_text(&mut s, pt(150.0, 40.0), "Old");
    let at = text_named(&s, "Old");
    let p = at.placement.apply_point(pt(2.0, -2.0));
    s.set_tool(Tool::Text);
    click(&mut s, p);
    assert!(s.text_editing());
    place(&mut s, 1);
    s.set_text_glyph_orientation(TextGlyphOrientation::Upright);
    s.set_text_writing_mode(TextWritingMode::VerticalRl);
    assert!(s.text_editing(), "edit mode stays");
    assert_eq!(caret(&s), 1, "the caret stays");
    s.text_input(TextInput::Insert("x".into()));
    assert_eq!(s.escape(), EscapeStep::ClearedState);
    let t = text_named(&s, "Oxld");
    assert_eq!(
        (t.layout.writing_mode, t.layout.orientation),
        (TextWritingMode::VerticalRl, TextGlyphOrientation::Upright)
    );
}
