//! The Text tool in the session (`specs/0074-text-objects` criteria 1, 2, 7, 8, 10, 12, 13, 22,
//! 24, 29), with the included fonts handed in as the host hands them.

use std::path::PathBuf;

use curvyo_document_core::{FontRef, NodeKind, ObjectSnapshot, Point};
use curvyo_ui_core::{FontBlock, TextInput};

use super::keys::{EscapeStep, KeyInput, KeyOutcome};
use super::text_bar::TextSizeOutcome;
use super::text_path::ObjectToPathOutcome;
use super::{Session, Tool};

fn fonts_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../frontend/public/fonts")
}

/// A session with the included fonts listed (as `index.json` lists them) and loaded on request,
/// as the host does after every event.
pub(super) fn session() -> Session {
    let mut session = Session::new(1);
    let index: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(fonts_dir().join("index.json")).unwrap())
            .unwrap();
    let faces = index["faces"]
        .as_array()
        .unwrap()
        .iter()
        .map(|face| {
            (
                face["file"].as_str().unwrap().to_owned(),
                face["family"].as_str().unwrap().to_owned(),
                face["style"].as_str().unwrap().to_owned(),
                0,
            )
        })
        .collect();
    session.set_font_list(FontBlock::Included, faces, Vec::new());
    load_wanted(&mut session);
    session
}

pub(super) fn load_wanted(session: &mut Session) {
    for file in session.wanted_font_files() {
        let bytes = std::fs::read(fonts_dir().join(&file)).unwrap();
        assert!(session.add_font_file(&file, &bytes));
    }
}

fn click(session: &mut Session, at: Point) {
    session.pointer_down(at, false);
    session.pointer_up(at, false, false);
}

fn key(session: &mut Session, key: &str) -> KeyOutcome {
    session.key_down(KeyInput {
        key,
        ..KeyInput::default()
    })
}

fn type_text(session: &mut Session, text: &str) {
    for c in text.chars() {
        session.text_input(TextInput::Insert(c.to_string()));
    }
}

pub(super) fn labels(session: &Session) -> Vec<String> {
    session
        .document
        .own_steps()
        .into_iter()
        .map(|step| step.label)
        .collect()
}

pub(super) fn texts(session: &Session) -> Vec<curvyo_document_core::TextSnapshot> {
    session
        .objects()
        .iter()
        .filter_map(|object| match object {
            ObjectSnapshot::Text(text) => Some((**text).clone()),
            _ => None,
        })
        .collect()
}

/// A session with one text "Name" at (10, 20) in the Text tool, edit mode ended.
pub(super) fn with_text(string: &str) -> Session {
    let mut session = session();
    session.set_tool(Tool::Text);
    click(&mut session, Point::new(10.0, 20.0));
    type_text(&mut session, string);
    assert_eq!(session.escape(), EscapeStep::ClearedState);
    load_wanted(&mut session);
    session
}

/// Criteria 1, 2, 7: T selects the Text tool; a click starts edit mode with nothing written; the
/// session is one step "Add text"; the second Escape goes to the Select tool with the text.
#[test]
fn ac01_ac02_ac07_click_type_escape() {
    let mut session = session();
    assert_eq!(key(&mut session, "t"), KeyOutcome::ToolChanged);
    assert_eq!(session.tool(), Tool::Text);
    click(&mut session, Point::new(10.0, 20.0));
    assert!(session.text_editing());
    type_text(&mut session, "Name");
    assert!(labels(&session).is_empty(), "nothing written while typing");
    assert!(session.draw_list().triangle_count() > 0);
    assert_eq!(session.escape(), EscapeStep::ClearedState);
    assert_eq!(labels(&session), ["add_text"]);
    let text = &texts(&session)[0];
    assert_eq!(text.string, "Name");
    assert_eq!((text.placement.e, text.placement.f), (10.0, 20.0));
    assert_eq!(session.escape(), EscapeStep::LeftTool);
    assert_eq!(session.tool(), Tool::Select);
    assert_eq!(session.selection.ids(), [text.id]);
}

/// Criterion 8: edit mode ended with an empty string creates nothing and writes no step; an
/// existing text emptied is removed in one step "Edit text".
#[test]
fn ac08_empty_strings() {
    let mut session = session();
    session.set_tool(Tool::Text);
    click(&mut session, Point::new(10.0, 20.0));
    session.escape();
    assert_eq!(labels(&session).len(), 0);
    let mut session = with_text("Name");
    click(&mut session, Point::new(12.0, 18.0));
    session.text_input(TextInput::SelectAll);
    session.text_input(TextInput::DeleteBackward);
    session.escape();
    assert_eq!(labels(&session), ["add_text", "edit_text"]);
    assert_eq!(texts(&session).len(), 0);
}

/// Criterion 10: a click on a text edits it at the nearest boundary; a Select double-click and
/// Enter with one text selected do the same.
#[test]
fn ac10_editing_an_existing_text() {
    let mut session = with_text("Name");
    click(&mut session, Point::new(10.2, 18.0));
    assert!(session.text_editing());
    type_text(&mut session, ">");
    session.escape();
    assert_eq!(texts(&session)[0].string, ">Name");
    session.set_tool(Tool::Select);
    session.double_click(Point::new(12.0, 18.0), false, false);
    assert_eq!(session.tool(), Tool::Text);
    assert!(session.text_editing());
    session.escape();
    session.escape();
    assert_eq!(session.tool(), Tool::Select);
    assert_eq!(key(&mut session, "Enter"), KeyOutcome::TextEditStarted);
    type_text(&mut session, "!");
    session.escape();
    assert_eq!(texts(&session)[0].string, ">Name!");
}

/// Criteria 12, 13: Size is held to its range with the notice's outcome; a change on the
/// selected text is one step and sets the next text's size.
#[test]
fn ac12_ac13_size() {
    let mut session = with_text("Name");
    assert_eq!(session.set_text_size(2500.0), TextSizeOutcome::ClampedToMax);
    assert_eq!(texts(&session)[0].size.as_mm(), 2000.0);
    assert_eq!(session.set_text_size(0.01), TextSizeOutcome::ClampedToMin);
    assert_eq!(session.set_text_size(20.0), TextSizeOutcome::Set);
    assert_eq!(
        labels(&session),
        [
            "add_text",
            "change_text_size",
            "change_text_size",
            "change_text_size"
        ]
    );
    session.selection.clear();
    assert_eq!(session.text_bar().size_mm, Some(20.0));
}

/// Criteria 12 and 22: Object to path gives a group named by the text with one compound path per
/// glyph, in one step; the H of a 20 mm text has its top 20.00 mm above the baseline.
#[test]
fn ac12_ac22_object_to_path() {
    let mut session = with_text("H H");
    session.set_text_size(20.0);
    let baseline = texts(&session)[0].placement.f;
    assert_eq!(session.object_to_path(), ObjectToPathOutcome::Converted);
    assert_eq!(labels(&session).last().unwrap(), "convert_to_paths");
    assert_eq!(texts(&session).len(), 0);
    let nodes = session.document.read_nodes();
    let group = nodes
        .iter()
        .find(|node| node.kind == NodeKind::Group)
        .unwrap();
    assert_eq!(group.registers.name.as_deref(), Some("H H"));
    let paths: Vec<_> = session
        .objects()
        .iter()
        .filter_map(|object| match object {
            ObjectSnapshot::Path(path) => Some(path.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(
        paths.len(),
        2,
        "one path per drawn glyph, none for the space"
    );
    let top = paths[0]
        .all_anchors()
        .map(|a| a.point.y)
        .fold(f64::INFINITY, f64::min);
    assert!((baseline - top - 20.0).abs() < 0.01);
    // The group is selected as its leaves, as after Ctrl+G.
    let leaves: Vec<_> = paths.iter().map(|path| path.id).collect();
    assert_eq!(session.selection.ids(), leaves.as_slice());
}

/// Criterion 24 and the architect review: Combine converts a text in the same step; a text
/// without a layout refuses the whole command and writes nothing.
#[test]
fn ac24_path_commands_convert_texts() {
    let mut session = with_text("Hi");
    session.set_tool(Tool::Select);
    session.set_tool(Tool::Rectangle);
    session.pointer_down(Point::new(0.0, 30.0), false);
    session.pointer_up(Point::new(20.0, 40.0), false, false);
    let ids: Vec<_> = session.objects().iter().map(ObjectSnapshot::id).collect();
    for id in &ids {
        if !session.selection.ids().contains(id) {
            session.selection.toggle(*id);
        }
    }
    let steps = labels(&session).len();
    // The face's bytes are gone: nothing is laid out.
    session.fonts = super::text_fonts::TextFonts::default();
    assert!(matches!(
        session.apply_combine(),
        super::CombineOutcome::Refused(curvyo_ui_core::CombineRefusal::Texts(_))
    ));
    assert_eq!(labels(&session).len(), steps);
    let mut session = with_text("Hi");
    session.set_tool(Tool::Rectangle);
    session.pointer_down(Point::new(0.0, 30.0), false);
    session.pointer_up(Point::new(20.0, 40.0), false, false);
    let ids: Vec<_> = session.objects().iter().map(ObjectSnapshot::id).collect();
    for id in &ids {
        if !session.selection.ids().contains(id) {
            session.selection.toggle(*id);
        }
    }
    assert!(matches!(
        session.apply_combine(),
        super::CombineOutcome::Applied { .. }
    ));
    assert_eq!(texts(&session).len(), 0);
    assert_eq!(labels(&session).last().unwrap(), "combine_paths");
}

/// Criteria 29 and 30: a face that is not here is drawn with the included sans and marked; the
/// reference is kept; Object to path is refused naming the face; picking a face replaces it.
#[test]
fn ac29_missing_face() {
    let mut session = with_text("Name");
    let id = texts(&session)[0].id;
    let lost = FontRef {
        family: "Lost Serif".to_owned(),
        style: "Bold".to_owned(),
        face_hash: "c".repeat(64),
        face_index: 0,
        kind: curvyo_document_core::FontKind::Outline,
    };
    session.document.set_text_font(&[id], &lost).unwrap();
    let text = &texts(&session)[0];
    assert!(text.outline.as_ref().unwrap().substitute);
    assert_eq!(text.font, lost);
    assert!(session.text_bar().missing);
    assert_eq!(session.missing_font_marks().len(), 1);
    session.set_tool(Tool::Select);
    session.selection.select_single(id);
    let ObjectToPathOutcome::Refused(refusal) = session.object_to_path() else {
        panic!("refused")
    };
    assert_eq!(refusal.missing_faces, ["Lost Serif Bold"]);
    assert_eq!(texts(&session)[0].font, lost);
    let serif = session
        .fonts
        .faces()
        .iter()
        .position(|f| f.family == "Source Serif 4" && f.style == "Bold")
        .unwrap();
    session.pick_font(serif);
    load_wanted(&mut session);
    assert_eq!(texts(&session)[0].font.family, "Source Serif 4");
    assert!(!texts(&session)[0].outline.as_ref().unwrap().substitute);
}

/// UX notes §1 "Hover": in the Text tool a text under the pointer shows the hover box, and no
/// selected box.
#[test]
fn ux_hover_box_in_the_text_tool() {
    let mut session = with_text("Name");
    session.selection.clear();
    session.pointer_hover(Point::new(12.0, 18.0), false, false);
    let input = session.select_decoration_input();
    assert!(input.hovered.is_some());
    assert_eq!(input.selected.len(), 0);
    session.pointer_hover(Point::new(150.0, 150.0), false, false);
    assert!(session.select_decoration_input().hovered.is_none());
}

/// Criterion 26: the typed size entry (S) resizes a text and the text stays a text. Doubling
/// both rounded prefills is a stretch that is not exactly uniform: it goes into the transform,
/// Size stays as set.
#[test]
fn ac26_typed_size_resizes_a_text() {
    let mut session = with_text("Name");
    session.set_text_size(10.0);
    assert_eq!(session.escape(), EscapeStep::LeftTool);
    assert_eq!(key(&mut session, "s"), KeyOutcome::EntryOpened);
    let chip = session.transform_entry().unwrap();
    let double = |text: &str| format!("{}", text.parse::<f64>().unwrap() * 2.0);
    let (width, height) = (
        double(&chip.fields[0].prefill),
        double(&chip.fields[1].prefill),
    );
    assert_eq!(
        session.commit_transform_entry(&width, &height, 1),
        curvyo_ui_core::EntryOutcome::Committed
    );
    let text = &texts(&session)[0];
    assert_eq!(text.size.as_mm(), 10.0);
    assert!((text.placement.a - 2.0).abs() < 0.01 && (text.placement.d - 2.0).abs() < 0.01);
}
