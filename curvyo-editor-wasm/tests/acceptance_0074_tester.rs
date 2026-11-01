//! Independent tester cases for `specs/0074-text-objects/specification.md` at the `Session`
//! level, for the criteria the implementer's tests leave open or cover thinly: 3 (tabs in a
//! paste), 7 (tool switch and a click outside end edit mode), 11 (the Text bar), 13 (several
//! texts in one step, the next text's face), 14 (first-use face, size and style), 15 (the Fonts
//! tab is always there), 18 (the current face mark), 24 (Boolean, Fracture, Flatten convert
//! texts), 25 (hit and marquee by the line box), 27 (Layers row name), 28 (reopened text stays
//! editable), 33 (face bytes once per session).
//!
//! Written from the specification and the public names before the implementation was read.

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

use curvyo_document_core::{Document, NodeId, ObjectSnapshot, Point, Timestamp, unpack};
use curvyo_editor_wasm::{
    BooleanOutcome, EscapeStep, KeyInput, ObjectToPathOutcome, Session, StackOutcome, Tool,
};
use curvyo_ui_core::{BooleanOp, FontBlock, TextInput};
use serde_json::Value;

static NOW: AtomicU64 = AtomicU64::new(1_900_000_000);

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

/// Hands every wanted face over, as the host does after an event; returns how many.
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
    s.set_font_list(FontBlock::Included, included_faces(), Vec::new());
    load(&mut s);
    s
}

fn fresh() -> Session {
    with_fonts(Session::new(21))
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

/// A new text typed at `at` with the Text tool, edit mode ended by Escape.
fn add_text(s: &mut Session, at: Point, string: &str) {
    s.set_tool(Tool::Text);
    click(s, at);
    assert!(s.text_editing(), "a click starts edit mode");
    type_text(s, string);
    assert_eq!(s.escape(), EscapeStep::ClearedState);
    load(s);
}

fn doc_of(s: &Session) -> Document {
    unpack(77, &s.pack("0.1.0").unwrap()).expect("the session output reopens")
}

fn texts(d: &Document) -> Vec<curvyo_document_core::TextSnapshot> {
    d.object_ids()
        .into_iter()
        .filter_map(|id| match d.object(id) {
            Some(ObjectSnapshot::Text(t)) => Some(*t),
            _ => None,
        })
        .collect()
}

fn kinds(d: &Document) -> Vec<&'static str> {
    d.object_ids()
        .into_iter()
        .map(|id| match d.object(id).unwrap() {
            ObjectSnapshot::Sketch(_) => unreachable!("no sketch in this test"),
            ObjectSnapshot::Image(_) => unreachable!("no image in this test"),
            ObjectSnapshot::Dimension(_) => unreachable!("no dimension in this test"),
            ObjectSnapshot::Text(_) => "text",
            ObjectSnapshot::Path(_) => "path",
            ObjectSnapshot::Primitive(_) => "primitive",
            ObjectSnapshot::Clone(_) => "clone",
        })
        .collect()
}

/// The labels of all commits after the new document's own, oldest first.
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

fn selected(s: &Session) -> Vec<NodeId> {
    s.style_scope().ids
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

fn rectangle(s: &mut Session, a: Point, b: Point) {
    s.set_tool(Tool::Rectangle);
    s.pointer_hover(a, false, false);
    s.pointer_down(a, false);
    s.pointer_hover(b, false, false);
    s.pointer_up(b, false, false);
}

fn fonts_tab(s: &Session) -> Value {
    serde_json::from_str(&s.fonts_tab_json()).unwrap()
}

fn layer_row_name(s: &mut Session, id: NodeId) -> String {
    let view: Value = serde_json::from_str(&s.layers_view_json(0, 500)).unwrap();
    let want = serde_json::to_value(id).unwrap();
    view["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == want)
        .unwrap_or_else(|| panic!("no row for {id:?}"))["name"]
        .as_str()
        .unwrap()
        .to_owned()
}

// ---------------------------------------------------------------------------------------------
// Part A
// ---------------------------------------------------------------------------------------------

/// Criterion 3: a pasted tab is kept as four spaces (Proposal accepted with the spec).
#[test]
fn ac03_a_pasted_tab_becomes_four_spaces() {
    let mut s = fresh();
    add_text(&mut s, pt(10.0, 20.0), "a\tb");
    assert_eq!(texts(&doc_of(&s))[0].string, "a    b");
}

/// Criterion 7: a tool switch ends edit mode as one step "Add text"; a click outside with the
/// Text tool ends the session and starts the next text at once.
#[test]
fn ac07_a_tool_switch_and_a_click_outside_end_edit_mode() {
    let mut s = fresh();
    s.set_tool(Tool::Text);
    click(&mut s, pt(10.0, 20.0));
    type_text(&mut s, "One");
    click(&mut s, pt(10.0, 80.0));
    assert!(
        s.text_editing(),
        "the click outside starts the next text at once"
    );
    assert_eq!(labels(&s), ["add_text"]);
    type_text(&mut s, "Two");
    s.set_tool(Tool::Select);
    assert!(!s.text_editing(), "a tool switch ends edit mode");
    assert_eq!(labels(&s), ["add_text", "add_text"]);
    let strings: Vec<_> = texts(&doc_of(&s)).into_iter().map(|t| t.string).collect();
    assert_eq!(strings.len(), 2);
    assert!(strings.contains(&"One".to_owned()) && strings.contains(&"Two".to_owned()));
    // The global undo takes the second session back in one step.
    s.undo();
    let strings: Vec<_> = texts(&doc_of(&s)).into_iter().map(|t| t.string).collect();
    assert_eq!(strings, ["One"]);
}

/// Criterion 9: Ctrl+Z inside the session undoes typing only; the session still writes one step.
#[test]
fn ac09_in_session_undo_does_not_touch_the_document() {
    let mut s = fresh();
    add_text(&mut s, pt(10.0, 20.0), "Base");
    s.set_tool(Tool::Text);
    click(&mut s, pt(10.5, 17.0));
    assert!(s.text_editing());
    s.text_input(TextInput::Move {
        to: curvyo_ui_core::CaretMove::LineEnd,
        extend: false,
    });
    type_text(&mut s, "X");
    type_text(&mut s, "Y");
    for _ in 0..3 {
        s.text_input(TextInput::Undo);
    }
    assert_eq!(
        s.text_sink_state().unwrap().0,
        "Base",
        "typing undone in the session"
    );
    assert_eq!(
        labels(&s),
        ["add_text"],
        "nothing is undone outside the session"
    );
    s.text_input(TextInput::Redo);
    let redone = s.text_sink_state().unwrap().0;
    assert!(
        redone.len() > 4 && "BaseXY".starts_with(&redone),
        "{redone}"
    );
    s.escape();
    assert_eq!(labels(&s), ["add_text", "edit_text"]);
    assert_eq!(texts(&doc_of(&s))[0].string, redone);
}

// ---------------------------------------------------------------------------------------------
// Part B
// ---------------------------------------------------------------------------------------------

/// Criteria 11 and 14: the Text bar is shown in the Text tool with the face named "Family
/// Style" and the size in mm; on first use that is Source Sans 3 Regular at 10 mm; the Select bar
/// shows it only for a selection that holds a text.
#[test]
fn ac11_ac14_text_bar_and_first_use_default() {
    let mut s = fresh();
    s.set_tool(Tool::Text);
    let bar = s.text_bar();
    assert!(bar.shown);
    assert_eq!(bar.font, "Source Sans 3 Regular");
    assert_eq!(bar.size_mm, Some(10.0));
    assert!(!bar.missing);
    rectangle(&mut s, pt(50.0, 50.0), pt(70.0, 70.0));
    s.set_tool(Tool::Select);
    click(&mut s, pt(60.0, 60.0));
    assert_eq!(selected(&s).len(), 1);
    assert!(!s.text_bar().shown, "a rectangle alone shows no Text group");
    add_text(&mut s, pt(10.0, 20.0), "Hi");
    let t = &texts(&doc_of(&s))[0];
    assert_eq!(
        (t.font.family.as_str(), t.font.style.as_str()),
        ("Source Sans 3", "Regular")
    );
    assert_eq!(t.size.as_mm(), 10.0);
    s.escape();
    assert_eq!(s.tool(), Tool::Select);
    assert!(
        s.text_bar().shown,
        "the Select bar's Text group with a text selected"
    );
    assert!(s.select_bar_state().object_to_path);
}

/// Criterion 14: a new text gets the fill and stroke a new rectangle gets; the Style tab shows
/// no stroke markers for it.
#[test]
fn ac14_new_text_style_is_the_rectangle_default_and_no_markers() {
    let mut s = fresh();
    rectangle(&mut s, pt(50.0, 50.0), pt(70.0, 70.0));
    add_text(&mut s, pt(10.0, 20.0), "Hi");
    let d = doc_of(&s);
    let rect_style = d
        .object_ids()
        .into_iter()
        .find_map(|id| match d.object(id) {
            Some(ObjectSnapshot::Primitive(p)) => Some(p.style.clone()),
            _ => None,
        })
        .unwrap();
    assert_eq!(texts(&d)[0].style, rect_style);
    s.escape();
    assert_eq!(selected(&s).len(), 1);
    assert!(!s.style_panel_view().markers_shown, "no markers for a text");
}

/// Criterion 13: Size on several selected texts is one step; Font likewise ("Change font");
/// with nothing selected both set the next text's face and size and write nothing.
#[test]
fn ac13_several_texts_change_in_one_step_and_next_text_state() {
    let mut s = fresh();
    add_text(&mut s, pt(10.0, 20.0), "One");
    add_text(&mut s, pt(10.0, 80.0), "Two");
    marquee(&mut s, pt(0.0, 0.0), pt(200.0, 150.0));
    assert_eq!(selected(&s).len(), 2);
    let before = labels(&s).len();
    s.set_text_size(30.0);
    assert_eq!(labels(&s).len(), before + 1);
    assert_eq!(labels(&s).last().unwrap(), "change_text_size");
    assert!(texts(&doc_of(&s)).iter().all(|t| t.size.as_mm() == 30.0));
    let tab = fonts_tab(&s);
    let bold = tab["faces"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["family"] == "Source Serif 4" && f["style"] == "Bold")
        .unwrap()["index"]
        .as_u64()
        .unwrap();
    s.pick_font(usize::try_from(bold).unwrap());
    load(&mut s);
    assert_eq!(labels(&s).len(), before + 2);
    assert_eq!(labels(&s).last().unwrap(), "change_font");
    assert!(
        texts(&doc_of(&s))
            .iter()
            .all(|t| t.font.family == "Source Serif 4" && t.font.style == "Bold")
    );
    // Nothing selected: the next text.
    s.escape();
    assert!(selected(&s).is_empty());
    let steps = labels(&s).len();
    s.set_text_size(12.5);
    let regular = fonts_tab(&s)["faces"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["family"] == "Great Vibes")
        .unwrap()["index"]
        .as_u64()
        .unwrap();
    s.pick_font(usize::try_from(regular).unwrap());
    load(&mut s);
    assert_eq!(
        labels(&s).len(),
        steps,
        "the next text's state writes nothing"
    );
    assert_eq!(fonts_tab(&s)["next"], true);
    add_text(&mut s, pt(10.0, 140.0), "Three");
    let three = texts(&doc_of(&s))
        .into_iter()
        .find(|t| t.string == "Three")
        .unwrap();
    assert_eq!(three.font.family, "Great Vibes");
    assert_eq!(three.size.as_mm(), 12.5);
}

// ---------------------------------------------------------------------------------------------
// Part C
// ---------------------------------------------------------------------------------------------

/// Criterion 15: Fonts is a tab of its own, present and not dimmed with nothing selected, with
/// the tooltip naming Shift+Ctrl+T, and the shortcut activates it.
#[test]
fn ac15_the_fonts_tab_is_always_there() {
    let mut s = fresh();
    let v = s.panel_view();
    let fonts = v.tabs.iter().find(|t| t.name == "fonts").unwrap();
    assert_eq!(fonts.label, "Fonts");
    assert_eq!(fonts.tooltip, "Fonts (Shift+Ctrl+T)");
    assert!(fonts.enabled, "never dimmed");
    assert!(!fonts.selected, "never active by itself");
    s.panel_shortcut_fonts();
    let v = s.panel_view();
    assert!(v.tabs.iter().find(|t| t.name == "fonts").unwrap().selected);
    // Selecting a text does not switch away, and the tab stays with a text selected.
    add_text(&mut s, pt(10.0, 20.0), "Hi");
    s.escape();
    assert!(
        s.panel_view()
            .tabs
            .iter()
            .any(|t| t.name == "fonts" && t.enabled)
    );
}

/// Criterion 18: the current face is marked; with a text selected it is that text's face, with
/// nothing selected the next text's face ("next").
#[test]
fn ac18_current_face_mark() {
    let mut s = fresh();
    let tab = fonts_tab(&s);
    assert_eq!(tab["next"], true);
    assert_eq!(tab["current"]["family"], "Source Sans 3");
    assert_eq!(tab["current"]["style"], "Regular");
    let idx = tab["current"]["index"].as_u64().unwrap();
    let face = &tab["faces"][usize::try_from(idx).unwrap()];
    assert_eq!(
        (face["family"].as_str(), face["style"].as_str()),
        (Some("Source Sans 3"), Some("Regular"))
    );
    add_text(&mut s, pt(10.0, 20.0), "Hi");
    s.escape();
    let serif = fonts_tab(&s)["faces"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["family"] == "Source Serif 4" && f["style"] == "Italic")
        .unwrap()["index"]
        .as_u64()
        .unwrap();
    s.pick_font(usize::try_from(serif).unwrap());
    load(&mut s);
    let tab = fonts_tab(&s);
    assert_eq!(tab["next"], false);
    assert_eq!(tab["current"]["index"].as_u64(), Some(serif));
    assert_eq!(tab["mixed"], false);
}

/// Criterion 13 and the Fonts tab subject: two texts with different faces read as mixed, no
/// row is marked and the Font button names no face.
#[test]
fn ac18_mixed_faces_mark_nothing() {
    let mut s = fresh();
    add_text(&mut s, pt(10.0, 20.0), "One");
    s.escape();
    s.escape();
    assert!(selected(&s).is_empty());
    let vibes = fonts_tab(&s)["faces"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["family"] == "Great Vibes")
        .unwrap()["index"]
        .as_u64()
        .unwrap();
    s.pick_font(usize::try_from(vibes).unwrap());
    load(&mut s);
    add_text(&mut s, pt(10.0, 80.0), "Two");
    marquee(&mut s, pt(0.0, 0.0), pt(200.0, 150.0));
    assert_eq!(selected(&s).len(), 2);
    let families: Vec<_> = texts(&doc_of(&s))
        .into_iter()
        .map(|t| t.font.family)
        .collect();
    assert!(families.contains(&"Source Sans 3".to_owned()));
    assert!(families.contains(&"Great Vibes".to_owned()));
    let tab = fonts_tab(&s);
    assert_eq!(tab["mixed"], true);
    assert!(tab["current"].is_null());
    assert_eq!(s.text_bar().font, "");
    assert_eq!(s.text_bar().size_mm, Some(10.0));
}

// ---------------------------------------------------------------------------------------------
// Part D
// ---------------------------------------------------------------------------------------------

fn text_and_rect(s: &mut Session) {
    add_text(s, pt(10.0, 20.0), "Hi");
    s.set_text_size(20.0);
    rectangle(s, pt(5.0, 10.0), pt(40.0, 25.0));
    marquee(s, pt(0.0, -20.0), pt(100.0, 60.0));
    assert_eq!(selected(s).len(), 2);
}

/// Criterion 24: Union with a text converts it in the same step; no text is left.
#[test]
fn ac24_boolean_converts_texts_in_its_own_step() {
    let mut s = fresh();
    text_and_rect(&mut s);
    let steps = s.undo_len();
    assert!(matches!(
        s.apply_boolean(BooleanOp::Union),
        BooleanOutcome::Applied { .. }
    ));
    assert_eq!(s.undo_len(), steps + 1);
    assert!(texts(&doc_of(&s)).is_empty());
}

/// Criterion 24: Fracture and Flatten convert texts in the same step.
#[test]
fn ac24_fracture_and_flatten_convert_texts() {
    for flatten in [false, true] {
        let mut s = fresh();
        text_and_rect(&mut s);
        let steps = s.undo_len();
        let out = if flatten {
            s.apply_flatten()
        } else {
            s.apply_fracture()
        };
        assert!(matches!(out, StackOutcome::Applied { .. }), "{out:?}");
        assert_eq!(s.undo_len(), steps + 1, "flatten={flatten}");
        assert!(texts(&doc_of(&s)).is_empty(), "flatten={flatten}");
        let k = kinds(&doc_of(&s));
        // Flatten keeps the top-most rectangle whole (`0037`); Fracture replaces both.
        if !flatten {
            assert!(k.iter().all(|k| *k == "path"), "{k:?}");
        }
    }
}

/// Criterion 24: two texts in one Combine, Union and Split; a text alone in Break apart, Cut
/// and Offset. Each command converts the texts in its own one step and leaves no text.
#[test]
fn ac24_every_path_command_takes_texts() {
    let two_texts = |s: &mut Session| {
        add_text(s, pt(10.0, 20.0), "Ab");
        add_text(s, pt(12.0, 40.0), "Cd");
        marquee(s, pt(0.0, -20.0), pt(200.0, 60.0));
        assert_eq!(selected(s).len(), 2);
        s.undo_len()
    };
    let one_text = |s: &mut Session| {
        add_text(s, pt(10.0, 20.0), "Hi o");
        s.escape();
        assert_eq!(selected(s).len(), 1);
        s.undo_len()
    };
    let check = |s: &Session, steps: usize, what: &str| {
        assert_eq!(s.undo_len(), steps + 1, "{what}: one step");
        assert!(texts(&doc_of(s)).is_empty(), "{what}: no text left");
        assert!(!doc_of(s).object_ids().is_empty(), "{what}: a result");
    };

    let mut s = fresh();
    let steps = two_texts(&mut s);
    let out = s.apply_combine();
    assert!(
        matches!(out, curvyo_editor_wasm::CombineOutcome::Applied { .. }),
        "{out:?}"
    );
    check(&s, steps, "combine two texts");

    let mut s = fresh();
    let steps = two_texts(&mut s);
    let out = s.apply_boolean(BooleanOp::Union);
    assert!(matches!(out, BooleanOutcome::Applied { .. }), "{out:?}");
    check(&s, steps, "union of two texts");

    let mut s = fresh();
    let steps = two_texts(&mut s);
    let out = s.apply_split();
    assert!(
        matches!(out, curvyo_editor_wasm::SplitOutcome::Applied { .. }),
        "{out:?}"
    );
    check(&s, steps, "split two texts");

    let mut s = fresh();
    let steps = one_text(&mut s);
    let out = s.apply_break_apart();
    assert!(
        matches!(out, curvyo_editor_wasm::BreakApartOutcome::Applied { .. }),
        "{out:?}"
    );
    check(&s, steps, "break apart a text");
    assert!(
        doc_of(&s).object_ids().len() >= 3,
        "H, i, the dot and o come apart"
    );

    // Cut refuses compound paths (`0048`), so the cut text is one single-outline glyph.
    let mut s = fresh();
    add_text(&mut s, pt(10.0, 20.0), "I");
    s.set_text_size(20.0);
    rectangle(&mut s, pt(5.0, 10.0), pt(40.0, 12.0));
    marquee(&mut s, pt(0.0, -20.0), pt(100.0, 60.0));
    let steps = s.undo_len();
    let out = s.apply_cut();
    assert!(
        matches!(out, curvyo_editor_wasm::CutOutcome::Applied { .. }),
        "{out:?}"
    );
    assert_eq!(s.undo_len(), steps + 1, "cut: one step");
    assert!(texts(&doc_of(&s)).is_empty(), "cut: no text left");

    let mut s = fresh();
    let steps = one_text(&mut s);
    s.set_tool(Tool::Offset);
    assert!(s.set_offset_distance(0.5));
    assert!(s.open_offset_chip(), "a text is an Offset operand");
    let out = s.apply_offset();
    assert!(
        matches!(out, curvyo_editor_wasm::OffsetOutcome::Applied { .. }),
        "{out:?}"
    );
    assert_eq!(s.undo_len(), steps + 1, "offset: one step");
}

/// Criterion 22: several texts in one Object to path are one step, each its own group.
#[test]
fn ac22_two_texts_one_step() {
    let mut s = fresh();
    add_text(&mut s, pt(10.0, 20.0), "Ab");
    add_text(&mut s, pt(10.0, 80.0), "Cd");
    marquee(&mut s, pt(0.0, 0.0), pt(200.0, 150.0));
    let steps = s.undo_len();
    assert_eq!(s.object_to_path(), ObjectToPathOutcome::Converted);
    assert_eq!(s.undo_len(), steps + 1);
    let d = doc_of(&s);
    assert!(texts(&d).is_empty());
    assert_eq!(
        kinds(&d),
        ["path", "path", "path", "path"],
        "one path per glyph"
    );
    // The result is selected as a grouped selection is everywhere else (Ctrl+G selects the
    // group's leaves): the Style tab sees the four paths and a nudge moves them.
    assert_eq!(
        selected(&s).len(),
        4,
        "the Style tab sees the converted result"
    );
    let before: Vec<_> = d.object_ids();
    let x_of = |d: &Document| -> Vec<f64> {
        d.object_ids()
            .into_iter()
            .map(|id| match d.object(id).unwrap() {
                ObjectSnapshot::Path(p) => p
                    .all_anchors()
                    .map(|a| a.point.x)
                    .fold(f64::INFINITY, f64::min),
                _ => f64::NAN,
            })
            .collect()
    };
    let x0 = x_of(&d);
    let _ = s.key_down(KeyInput {
        key: "ArrowRight",
        ..KeyInput::default()
    });
    let d = doc_of(&s);
    assert_eq!(d.object_ids(), before);
    let x1 = x_of(&d);
    assert!(
        x0.iter().zip(&x1).all(|(a, b)| b > a),
        "a nudge moves the result: {x0:?} {x1:?}"
    );
}

// ---------------------------------------------------------------------------------------------
// Part E
// ---------------------------------------------------------------------------------------------

/// Criterion 25: the text is hit anywhere inside its line box, also in the gap between two
/// letters; a marquee selects it only when the box is inside.
#[test]
fn ac25_hit_and_marquee_by_the_line_box() {
    let mut s = fresh();
    add_text(&mut s, pt(10.0, 20.0), "I I");
    s.set_text_size(20.0);
    s.escape();
    s.escape();
    s.escape();
    assert!(selected(&s).is_empty());
    s.set_tool(Tool::Select);
    // Inside the space between the two I, at mid cap height: no ink there, but in the box.
    click(&mut s, pt(14.0, 12.0));
    assert_eq!(
        selected(&s).len(),
        1,
        "a click in the box between letters hits"
    );
    s.escape();
    // A marquee that covers only half of the text does not select it.
    marquee(&mut s, pt(0.0, -20.0), pt(13.0, 40.0));
    assert!(selected(&s).is_empty());
    marquee(&mut s, pt(0.0, -20.0), pt(100.0, 40.0));
    assert_eq!(selected(&s).len(), 1);
}

/// Criterion 27: the Layers row is named by the first 24 characters with an ellipsis, a line
/// break as a space, and follows a change of the string.
#[test]
fn ac27_layers_row_name_follows_the_string() {
    let mut s = fresh();
    add_text(&mut s, pt(10.0, 20.0), "Short\nline");
    let id = texts(&doc_of(&s))[0].id;
    assert_eq!(layer_row_name(&mut s, id), "Short line");
    s.set_tool(Tool::Text);
    click(&mut s, pt(10.3, 17.0));
    assert!(s.text_editing());
    s.text_input(TextInput::SelectAll);
    type_text(&mut s, "abcdefghijklmnopqrstuvwxyz0123");
    s.escape();
    let name = layer_row_name(&mut s, id);
    assert!(name.starts_with("abcdefghijklmnopqrstuvwx"), "{name}");
    assert!(name.ends_with('…'), "{name}");
    assert_eq!(name.chars().count(), 25, "{name}");
}

/// Criterion 28: a saved and reopened project keeps the text editable in the same face.
#[test]
fn ac28_reopened_text_is_editable() {
    let mut s = fresh();
    add_text(&mut s, pt(10.0, 20.0), "Plate");
    s.set_text_size(20.0);
    let bytes = s.pack("0.1.0").unwrap();
    let mut s = with_fonts(Session::open(22, &bytes).unwrap());
    let before = texts(&doc_of(&s))[0].clone();
    assert_eq!(before.string, "Plate");
    assert_eq!(before.size.as_mm(), 20.0);
    assert!(!s.text_bar().missing);
    s.set_tool(Tool::Text);
    click(&mut s, pt(10.3, 10.0));
    assert!(s.text_editing(), "a reopened text is editable");
    s.text_input(TextInput::Move {
        to: curvyo_ui_core::CaretMove::LineEnd,
        extend: false,
    });
    type_text(&mut s, "!");
    s.escape();
    let after = texts(&doc_of(&s))[0].clone();
    assert_eq!(after.string, "Plate!");
    assert_eq!(after.font, before.font);
}

// ---------------------------------------------------------------------------------------------
// Part F
// ---------------------------------------------------------------------------------------------

/// Criterion 33: each face's bytes are asked for once per session: after a face was handed over
/// no further frames, edits or texts in it ask for it again.
#[test]
fn ac33_face_bytes_once_per_face() {
    let mut s = fresh();
    add_text(&mut s, pt(10.0, 20.0), "One");
    assert_eq!(load(&mut s), 0);
    add_text(&mut s, pt(10.0, 80.0), "Two");
    let _ = s.draw_list();
    assert!(s.wanted_font_files().is_empty(), "nothing asked again");
}

/// Criterion 31 at the session: a paste past the limit stops at 10,000 characters and raises the
/// notice once.
#[test]
fn ac31_paste_past_the_limit() {
    let mut s = fresh();
    s.set_tool(Tool::Text);
    click(&mut s, pt(10.0, 20.0));
    s.text_input(TextInput::Insert("x".repeat(10_050)));
    assert!(s.take_text_limit_notice());
    assert!(!s.take_text_limit_notice(), "taken once");
    s.escape();
    load(&mut s);
    assert_eq!(texts(&doc_of(&s))[0].string.chars().count(), 10_000);
}

/// Hostile: a key in the Text tool with no edit mode and no text does not panic, and Enter with
/// nothing selected starts a text at the viewport centre (criterion 10).
#[test]
fn ac10_enter_with_nothing_selected_starts_a_text() {
    let mut s = fresh();
    s.set_tool(Tool::Text);
    let _ = s.key_down(KeyInput {
        key: "Enter",
        ..KeyInput::default()
    });
    assert!(s.text_editing());
    type_text(&mut s, "C");
    s.escape();
    assert_eq!(texts(&doc_of(&s)).len(), 1);
}

/// Criteria 7 and 9 (UX notes §2, History tab in edit mode): Undo pressed while a text is edited
/// commits the session first, so it takes the session back, never the step before it, and the
/// typed characters are not lost.
#[test]
fn ac09_history_undo_in_edit_mode_takes_the_session_back() {
    let mut s = fresh();
    add_text(&mut s, pt(10.0, 20.0), "Base");
    s.set_tool(Tool::Text);
    click(&mut s, pt(10.5, 17.0));
    assert!(s.text_editing());
    type_text(&mut s, "X");
    s.undo();
    let strings: Vec<_> = texts(&doc_of(&s)).into_iter().map(|t| t.string).collect();
    assert_eq!(
        strings,
        ["Base"],
        "Undo takes back the edit session, not \"Add text\""
    );
    s.redo();
    s.escape();
    let strings: Vec<_> = texts(&doc_of(&s)).into_iter().map(|t| t.string).collect();
    assert_eq!(strings.len(), 1, "the typed text is not lost: {strings:?}");
    assert!(
        strings[0].contains('X') && strings[0].contains("Base"),
        "{strings:?}"
    );
}
