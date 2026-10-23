//! Independent tester cases for `specs/0078-font-style-buttons/specification.md` at the `Session`
//! level: criteria 8 (included Source Sans 3), 10 (one step, undo), 11 (several texts, counts),
//! 12 (Italic on a mixed selection), 13 (Bold, Italic, Light), 14 (what a press keeps), 15 (next
//! text), 16 (edit mode stays), 17 (a face not loaded yet), 22 and 23 (family pick keeps the
//! style), 24 (tile face), 26 (save and reopen).
//!
//! Written from the specification, `adrs.md` and the public names before the implementation was
//! read.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::too_many_lines,
    clippy::float_cmp,
    missing_docs
)]

use std::io::{Cursor, Read};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use curvyo_document_core::{
    CURRENT_FORMAT_VERSION, Document, FontKind, ObjectSnapshot, Point, TextSnapshot, Timestamp,
    unpack,
};
use curvyo_editor_wasm::{EscapeStep, Session, Tool};
use curvyo_text_core::WeightBand;
use curvyo_ui_core::{FontBlock, Greyed, Pressed, TextInput};
use serde_json::Value;

static NOW: AtomicU64 = AtomicU64::new(1_970_000_000);

fn clock() -> Timestamp {
    Timestamp::from_secs(NOW.fetch_add(3, Ordering::SeqCst))
}

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn fonts_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../frontend/public/fonts")
}

type Listed = (String, String, String, u32, FontKind);

fn included_faces() -> Vec<Listed> {
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

fn load(s: &mut Session) -> usize {
    let wanted = s.wanted_font_files();
    for file in &wanted {
        let bytes = std::fs::read(fonts_dir().join(file)).unwrap();
        assert!(s.add_font_file(file, &bytes), "{file} loads");
    }
    wanted.len()
}

fn with_fonts(mut s: Session) -> Session {
    s.set_clock(clock);
    s.resize_viewport(1200.0, 800.0);
    s.set_font_list_with_kinds(FontBlock::Included, included_faces(), Vec::new());
    load(&mut s);
    s
}

fn fresh() -> Session {
    with_fonts(Session::new(78))
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

fn marquee(s: &mut Session, a: Point, b: Point) {
    s.set_tool(Tool::Select);
    s.pointer_hover(a, false, false);
    s.pointer_down(a, false);
    s.pointer_hover(
        pt(f64::midpoint(a.x, b.x), f64::midpoint(a.y, b.y)),
        false,
        false,
    );
    s.pointer_hover(b, false, false);
    s.pointer_up(b, false, false);
}

fn select_at(s: &mut Session, at: Point) {
    s.set_tool(Tool::Select);
    click(s, at);
}

fn face_row(s: &Session, family: &str, style: &str) -> (String, u32) {
    let tab: Value = serde_json::from_str(&s.fonts_tab_json()).unwrap();
    let f = tab["faces"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["family"] == family && f["style"] == style)
        .unwrap_or_else(|| panic!("{family} {style}"))
        .clone();
    (
        f["id"].as_str().unwrap().to_owned(),
        u32::try_from(f["faceIndex"].as_u64().unwrap()).unwrap(),
    )
}

fn pick_index(s: &Session, family: &str, style: &str) -> usize {
    let tab: Value = serde_json::from_str(&s.fonts_tab_json()).unwrap();
    usize::try_from(
        tab["faces"]
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["family"] == family && f["style"] == style)
            .unwrap()["index"]
            .as_u64()
            .unwrap(),
    )
    .unwrap()
}

/// Criteria 8 and 10: on Source Sans 3 Regular, Extra Bold is greyed; Bold is one
/// `change_font` step and Undo brings Regular back in one step.
#[test]
fn ac08_ac10_bold_is_one_step_and_undoes() {
    let mut s = fresh();
    add_text(&mut s, pt(10.0, 20.0), "Plate");
    select_at(&mut s, pt(14.0, 16.0));
    let bar = s.text_bar();
    assert_eq!(bar.style.weights[2].pressed, Pressed::On);
    assert_eq!(
        bar.style.weights[4].greyed,
        Some(Greyed::NotIn("Source Sans 3".into()))
    );
    let before = labels(&s).len();
    let out = s.press_weight(WeightBand::Bold);
    load(&mut s);
    assert!(out.is_none() || out.unwrap().left == 0);
    assert_eq!(text_named(&s, "Plate").font.style, "Bold");
    assert_eq!(labels(&s).len(), before + 1);
    assert_eq!(labels(&s).last().unwrap(), "change_font");
    assert_eq!(s.text_bar().font, "Source Sans 3 Bold");
    let _ = s.undo();
    load(&mut s);
    assert_eq!(text_named(&s, "Plate").font.style, "Regular");
}

/// Criterion 6: a press on a greyed button changes nothing and writes no step.
#[test]
fn ac06_greyed_press_does_nothing() {
    let mut s = fresh();
    add_text(&mut s, pt(10.0, 20.0), "Plate");
    select_at(&mut s, pt(14.0, 16.0));
    let before = labels(&s).len();
    let out = s.press_weight(WeightBand::ExtraBold);
    load(&mut s);
    assert!(out.is_none(), "no notice for a greyed press: {out:?}");
    assert_eq!(labels(&s).len(), before);
    assert_eq!(text_named(&s, "Plate").font.style, "Regular");
}

/// Criterion 11: two of three texts get Bold, one step, the outcome counts 2 set and 1 left.
#[test]
fn ac11_several_texts_one_step_and_counts() {
    let mut s = fresh();
    add_text(&mut s, pt(10.0, 20.0), "One");
    add_text(&mut s, pt(10.0, 80.0), "Two");
    s.escape();
    s.escape();
    s.pick_font(pick_index(&s, "Great Vibes", "Regular"));
    load(&mut s);
    add_text(&mut s, pt(10.0, 140.0), "Three");
    marquee(&mut s, pt(-20.0, -20.0), pt(300.0, 200.0));
    assert_eq!(s.style_scope().ids.len(), 3);
    let before = labels(&s).len();
    let out = s.press_weight(WeightBand::Bold).unwrap();
    load(&mut s);
    assert_eq!((out.set, out.left, out.upright), (2, 1, false));
    assert_eq!(labels(&s).len(), before + 1);
    assert_eq!(text_named(&s, "One").font.style, "Bold");
    assert_eq!(text_named(&s, "Two").font.style, "Bold");
    assert_eq!(text_named(&s, "Three").font.family, "Great Vibes");
    let _ = s.undo();
    load(&mut s);
    assert!(
        texts(&s).iter().all(|t| t.font.style == "Regular"),
        "one undo step"
    );
}

/// Criteria 12 and 13: Italic on a mixed selection keeps italic texts and makes the others
/// italic; Bold, Italic, Light gives Light Italic.
#[test]
fn ac12_ac13_italic_mixed_and_composition() {
    let mut s = fresh();
    add_text(&mut s, pt(10.0, 20.0), "One");
    add_text(&mut s, pt(10.0, 80.0), "Two");
    select_at(&mut s, pt(14.0, 16.0));
    s.press_weight(WeightBand::Bold);
    load(&mut s);
    s.press_italic();
    load(&mut s);
    assert_eq!(text_named(&s, "One").font.style, "Bold Italic");
    s.press_weight(WeightBand::Light);
    load(&mut s);
    assert_eq!(text_named(&s, "One").font.style, "Light Italic");
    marquee(&mut s, pt(-20.0, -20.0), pt(300.0, 140.0));
    assert_eq!(s.text_bar().style.italic.pressed, Pressed::Mixed);
    let out = s.press_italic().unwrap();
    load(&mut s);
    assert_eq!((out.set, out.left, out.upright), (2, 0, false));
    assert_eq!(text_named(&s, "One").font.style, "Light Italic");
    assert!(text_named(&s, "Two").font.style.ends_with("Italic"));
    let out = s.press_italic().unwrap();
    load(&mut s);
    assert!(out.upright);
    assert_eq!(text_named(&s, "One").font.style, "Light");
}

/// Criterion 14: a press keeps the string, size, placement and style.
#[test]
fn ac14_press_keeps_everything_but_the_face() {
    let mut s = fresh();
    add_text(&mut s, pt(10.0, 20.0), "Hag");
    select_at(&mut s, pt(14.0, 16.0));
    s.set_text_size(20.0);
    let before = text_named(&s, "Hag");
    s.press_weight(WeightBand::Light);
    load(&mut s);
    let after = text_named(&s, "Hag");
    assert_eq!(after.font.style, "Light");
    assert_eq!(after.size.as_mm(), 20.0);
    assert_eq!(after.placement, before.placement);
    assert_eq!(after.style, before.style);
    assert_eq!(after.id, before.id);
}

/// Criterion 15: nothing selected, Text tool: the next text's face changes, no history step,
/// and the states follow the next face.
#[test]
fn ac15_next_text_face() {
    let mut s = fresh();
    s.set_tool(Tool::Text);
    let before = labels(&s).len();
    s.press_weight(WeightBand::Bold);
    load(&mut s);
    s.press_italic();
    load(&mut s);
    assert_eq!(labels(&s).len(), before, "no history step");
    let bar = s.text_bar();
    assert_eq!(bar.style.weights[3].pressed, Pressed::On);
    assert_eq!(bar.style.italic.pressed, Pressed::On);
    add_text(&mut s, pt(10.0, 20.0), "Next");
    assert_eq!(text_named(&s, "Next").font.style, "Bold Italic");
}

/// Criterion 16: in edit mode on a new text a press keeps edit mode and typing continues.
#[test]
fn ac16_edit_mode_stays() {
    let mut s = fresh();
    s.set_tool(Tool::Text);
    click(&mut s, pt(10.0, 20.0));
    s.text_input(TextInput::Insert("Live".into()));
    load(&mut s);
    s.press_weight(WeightBand::Bold);
    load(&mut s);
    assert!(s.text_editing(), "edit mode stays");
    s.text_input(TextInput::Insert("ly".into()));
    load(&mut s);
    assert_eq!(s.escape(), EscapeStep::ClearedState);
    let strings: Vec<String> = texts(&s).into_iter().map(|t| t.string).collect();
    assert_eq!(strings, ["Lively"], "typing continues at the caret");
    let t = text_named(&s, "Lively");
    assert_eq!(t.font.style, "Bold");
    // A text still being created takes its face with "add_text", as a Fonts tab pick does
    // (`0074`); the separate step is checked on an existing text below.
    assert_eq!(labels(&s), ["add_text"]);
}

/// Criterion 16 on an existing text: edit mode stays, the face change is its own step.
#[test]
fn ac16_existing_text_edit_mode_separate_step() {
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
    let before = labels(&s);
    s.press_weight(WeightBand::Bold);
    load(&mut s);
    assert!(s.text_editing(), "edit mode stays");
    s.text_input(TextInput::Insert("ly".into()));
    load(&mut s);
    assert_eq!(s.escape(), EscapeStep::ClearedState);
    assert_eq!(text_named(&s, "Lively").font.style, "Bold");
    let after = labels(&s);
    assert_eq!(
        after.len(),
        before.len() + 2,
        "face change and edit are two steps"
    );
    assert!(after[before.len()..].iter().any(|l| l == "change_font"));
}

/// Criterion 17: a press on a face whose bytes are not loaded asks for them and applies once
/// they arrive.
#[test]
fn ac17_face_loaded_first() {
    let mut s = fresh();
    add_text(&mut s, pt(10.0, 20.0), "Slow");
    select_at(&mut s, pt(14.0, 16.0));
    s.press_weight(WeightBand::ExtraLight);
    let wanted = s.wanted_font_files();
    assert!(
        wanted.iter().any(|f| f.contains("ExtraLight")),
        "the Extra Light face is asked for: {wanted:?}"
    );
    load(&mut s);
    assert_eq!(text_named(&s, "Slow").font.style, "Extra Light");
}

/// Criteria 22 and 23: a family row pick keeps each text's style where the family has it.
#[test]
fn ac22_ac23_family_pick_keeps_style() {
    let mut s = fresh();
    add_text(&mut s, pt(10.0, 20.0), "One");
    add_text(&mut s, pt(10.0, 80.0), "Two");
    select_at(&mut s, pt(14.0, 16.0));
    s.press_weight(WeightBand::Bold);
    load(&mut s);
    select_at(&mut s, pt(14.0, 76.0));
    s.press_weight(WeightBand::Light);
    load(&mut s);
    marquee(&mut s, pt(-20.0, -20.0), pt(300.0, 140.0));
    let before = labels(&s).len();
    let (id, index) = face_row(&s, "Source Serif 4", "Regular");
    s.pick_font_family(&id, index);
    load(&mut s);
    assert_eq!(labels(&s).len(), before + 1, "one step");
    let one = text_named(&s, "One");
    let two = text_named(&s, "Two");
    assert_eq!(one.font.family, "Source Serif 4");
    assert_eq!(one.font.style, "Bold");
    assert_eq!(two.font.family, "Source Serif 4");
    assert_eq!(two.font.style, "Regular", "no Light in Source Serif 4");
}

/// Criterion 24: a tile for a Bold Italic text shows the family's Bold Italic face.
#[test]
fn ac24_tile_face_follows_style() {
    let mut s = fresh();
    add_text(&mut s, pt(10.0, 20.0), "One");
    select_at(&mut s, pt(14.0, 16.0));
    s.press_weight(WeightBand::Bold);
    load(&mut s);
    s.press_italic();
    load(&mut s);
    let (id, index) = face_row(&s, "Source Sans 3", "Regular");
    let (shown, _) = s.family_tile_face(&id, index).unwrap();
    assert!(shown.contains("BoldIt"), "{shown}");
    let (vid, vindex) = face_row(&s, "Great Vibes", "Regular");
    let (shown, _) = s.family_tile_face(&vid, vindex).unwrap();
    assert!(shown.contains("GreatVibes"), "{shown}");
}

/// Criterion 26: saved and reopened, the face stays; the format version does not change.
#[test]
fn ac26_save_and_reopen() {
    let mut s = fresh();
    add_text(&mut s, pt(10.0, 20.0), "Keep");
    select_at(&mut s, pt(14.0, 16.0));
    s.press_weight(WeightBand::Light);
    load(&mut s);
    s.press_italic();
    load(&mut s);
    let font = text_named(&s, "Keep").font;
    let bytes = s.pack("0.1.0").unwrap();
    let s2 = with_fonts(Session::open(79, &bytes).unwrap());
    assert_eq!(text_named(&s2, "Keep").font, font);
    assert!(!s2.text_bar().missing);
    let mut archive = zip::ZipArchive::new(Cursor::new(&bytes)).unwrap();
    let mut out = String::new();
    archive
        .by_name("manifest.json")
        .unwrap()
        .read_to_string(&mut out)
        .unwrap();
    let manifest: Value = serde_json::from_str(&out).unwrap();
    assert_eq!(
        manifest["format_version"].as_u64(),
        Some(u64::from(CURRENT_FORMAT_VERSION))
    );
}

/// Criterion 9: a text whose face is missing uses the listed family's faces, its stored style
/// is pressed, and a press replaces the missing reference.
#[test]
fn ac09_missing_face_press_replaces_it() {
    let mut s = fresh();
    add_text(&mut s, pt(10.0, 20.0), "Gone");
    select_at(&mut s, pt(14.0, 16.0));
    s.press_weight(WeightBand::Light);
    load(&mut s);
    let bytes = s.pack("0.1.0").unwrap();
    let mut s = Session::open(80, &bytes).unwrap();
    s.set_clock(clock);
    s.resize_viewport(1200.0, 800.0);
    let without_light: Vec<Listed> = included_faces()
        .into_iter()
        .filter(|f| !(f.1 == "Source Sans 3" && f.2 == "Light"))
        .collect();
    s.set_font_list_with_kinds(FontBlock::Included, without_light, Vec::new());
    load(&mut s);
    select_at(&mut s, pt(14.0, 16.0));
    let bar = s.text_bar();
    assert!(bar.missing, "Light is not listed");
    assert_eq!(
        bar.style.weights[1].pressed,
        Pressed::On,
        "stored style pressed"
    );
    assert!(bar.style.weights[3].greyed.is_none());
    assert!(bar.style.italic.greyed.is_none(), "Light Italic is listed");
    s.press_weight(WeightBand::Bold);
    load(&mut s);
    let t = text_named(&s, "Gone");
    assert_eq!(
        (t.font.family.as_str(), t.font.style.as_str()),
        ("Source Sans 3", "Bold")
    );
    assert!(!s.text_bar().missing);
}

/// Criterion 3 and the radio rule: a press on the pressed weight changes nothing, writes no
/// step and gives no notice.
#[test]
fn pressed_weight_press_is_a_no_op() {
    let mut s = fresh();
    add_text(&mut s, pt(10.0, 20.0), "Same");
    select_at(&mut s, pt(14.0, 16.0));
    let before = labels(&s).len();
    assert!(s.press_weight(WeightBand::Regular).is_none());
    load(&mut s);
    assert_eq!(labels(&s).len(), before);
}

/// Criteria 12 and 6: Italic on an all-italic text whose family has no upright face is greyed
/// and a press does nothing (the implementer's reading of the 5/6 contradiction).
#[test]
fn italic_pressed_without_upright_press_does_nothing() {
    let mut s = fresh();
    s.set_font_list_with_kinds(
        FontBlock::Included,
        included_faces()
            .into_iter()
            .filter(|f| !(f.1 == "Source Sans 3" && f.2 == "Regular"))
            .collect(),
        Vec::new(),
    );
    load(&mut s);
    s.pick_font(pick_index(&s, "Source Sans 3", "Italic"));
    load(&mut s);
    add_text(&mut s, pt(10.0, 20.0), "Slanted");
    select_at(&mut s, pt(14.0, 16.0));
    let bar = s.text_bar();
    assert_eq!(bar.style.italic.pressed, Pressed::On);
    assert_eq!(
        bar.style.italic.greyed,
        Some(Greyed::NotInAsUpright("Source Sans 3".into()))
    );
    let before = labels(&s).len();
    assert!(s.press_italic().is_none());
    assert_eq!(labels(&s).len(), before);
}
