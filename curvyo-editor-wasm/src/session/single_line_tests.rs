//! Single-line texts in the session (`specs/0076-single-line-fonts` criteria 6 to 8, 11, 13, 17
//! to 20, 22), with the included fonts listed as `index.json` lists them.

use std::path::PathBuf;

use curvyo_document_core::{
    Color, FontKind, FontRef, Length, NodeKind, ObjectSnapshot, Point, StyleEdit,
};
use curvyo_ui_core::{BooleanOp, BooleanRefusal, FontBlock, StackRefusal, TextInput, WeldRefusal};

use super::boolean::BooleanOutcome;
use super::fracture::StackOutcome;
use super::text_path::ObjectToPathOutcome;
use super::weld::WeldOutcome;
use super::{CombineOutcome, Session, Tool};

const RED: Color = Color { r: 255, g: 0, b: 0 };

fn fonts_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../frontend/public/fonts")
}

fn load_wanted(session: &mut Session) {
    for file in session.wanted_font_files() {
        let bytes = std::fs::read(fonts_dir().join(&file)).unwrap();
        assert!(session.add_font_file(&file, &bytes));
    }
}

/// A session with the included fonts listed with their kinds.
fn session() -> Session {
    let mut session = Session::new(1);
    let index: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(fonts_dir().join("index.json")).unwrap())
            .unwrap();
    let faces = index["faces"]
        .as_array()
        .unwrap()
        .iter()
        .map(|face| {
            let kind = if face["kind"].as_str() == Some("single_line") {
                FontKind::SingleLine
            } else {
                FontKind::Outline
            };
            (
                face["file"].as_str().unwrap().to_owned(),
                face["family"].as_str().unwrap().to_owned(),
                face["style"].as_str().unwrap().to_owned(),
                0,
                kind,
            )
        })
        .collect();
    session.set_font_list_with_kinds(FontBlock::Included, faces, Vec::new());
    load_wanted(&mut session);
    session
}

fn face_index(session: &Session, family: &str) -> usize {
    session
        .fonts
        .faces()
        .iter()
        .position(|f| f.family == family)
        .unwrap()
}

/// A session with one single-line text `string` in EMS Readability at 20 mm, selected.
fn with_text(string: &str) -> Session {
    let mut session = session();
    let index = face_index(&session, "EMS Readability");
    session.pick_font(index);
    load_wanted(&mut session);
    session.set_tool(Tool::Text);
    let at = Point::new(10.0, 40.0);
    session.pointer_down(at, false);
    session.pointer_up(at, false, false);
    for c in string.chars() {
        session.text_input(TextInput::Insert(c.to_string()));
    }
    session.escape();
    session.escape();
    load_wanted(&mut session);
    let id = texts(&session)[0].id;
    session.selection.select_single(id);
    session.set_text_size(20.0);
    session
}

fn texts(session: &Session) -> Vec<curvyo_document_core::TextSnapshot> {
    session
        .objects()
        .iter()
        .filter_map(|object| match object {
            ObjectSnapshot::Text(text) => Some((**text).clone()),
            _ => None,
        })
        .collect()
}

fn add_rectangle(session: &mut Session) {
    session.set_tool(Tool::Rectangle);
    session.pointer_down(Point::new(0.0, 50.0), false);
    session.pointer_up(Point::new(20.0, 60.0), false, false);
    session.set_tool(Tool::Select);
}

fn select_all(session: &mut Session) {
    let ids: Vec<_> = session.objects().iter().map(ObjectSnapshot::id).collect();
    session.selection.clear();
    for id in ids {
        session.selection.toggle(id);
    }
}

/// Criteria 7 and 8: a single-line "O" with fill red on and a black 0.5 mm stroke draws no red
/// at all (never filled), keeps its fill in the document, and the Style tab hides Fill.
#[test]
fn ac07_ac08_stroke_only_and_fill_kept() {
    let mut session = with_text("O");
    let text = texts(&session)[0].clone();
    assert_eq!(text.font.kind, FontKind::SingleLine);
    session
        .document
        .edit_style(&[text.id], &StyleEdit::FillEnabled(true))
        .unwrap();
    session
        .document
        .edit_style(&[text.id], &StyleEdit::FillColor(RED))
        .unwrap();
    session
        .document
        .edit_style(
            &[text.id],
            &StyleEdit::StrokeColor(Color { r: 0, g: 0, b: 0 }),
        )
        .unwrap();
    session
        .document
        .edit_style(&[text.id], &StyleEdit::StrokeWidth(Length::from_mm(0.5)))
        .unwrap();
    let list = session.draw_list();
    assert!(list.triangle_count() > 0, "the stroke is drawn");
    assert!(
        list.triangles
            .iter()
            .all(|v| !(v.color.r > 230 && v.color.g < 25 && v.color.b < 25)),
        "no red pixel anywhere, the centre of the O included"
    );
    let stored = texts(&session)[0].style.clone();
    assert!(stored.fill.enabled && stored.fill.color == RED, "kept");
    session.set_tool(Tool::Select);
    session.selection.select_single(text.id);
    assert!(!session.style_panel_view().fill_shown);
    // Mixed: Fill changes the rectangle only and says so.
    add_rectangle(&mut session);
    select_all(&mut session);
    assert!(session.style_panel_view().fill_shown);
    session.set_fill_paint(false);
    assert!(
        texts(&session)[0].style.fill.enabled,
        "the text's fill is not changed"
    );
    assert_eq!(
        session.take_style_skip_notice(),
        "1 single-line text has no fill and was not changed."
    );
}

/// Criteria 11 and 13: typing and editing work as for any text; Size does not change the
/// stroke width.
#[test]
fn ac11_ac13_editing_and_size() {
    let mut session = with_text("Name");
    let width = texts(&session)[0].style.stroke.width;
    session.set_tool(Tool::Text);
    let start = texts(&session)[0]
        .placement
        .apply_point(Point::new(0.0, -2.0));
    session.pointer_down(start, false);
    session.pointer_up(start, false, false);
    assert!(session.text_editing());
    session.text_input(TextInput::Insert(">".to_owned()));
    session.escape();
    assert_eq!(texts(&session)[0].string, ">Name");
    session.set_tool(Tool::Select);
    session.selection.select_single(texts(&session)[0].id);
    session.set_text_size(40.0);
    assert_eq!(texts(&session)[0].style.stroke.width, width);
}

/// Criteria 17 and 18: Object to path gives a group of open paths, Fill off, one per glyph,
/// nodes on the centre lines; Combine converts the text in its own step.
#[test]
fn ac17_ac18_object_to_path_and_combine() {
    let mut session = with_text("HE");
    let baseline = texts(&session)[0].placement.f;
    assert_eq!(session.object_to_path(), ObjectToPathOutcome::Converted);
    let group = session
        .document
        .read_nodes()
        .into_iter()
        .find(|node| node.kind == NodeKind::Group)
        .unwrap();
    assert_eq!(group.registers.name.as_deref(), Some("HE"));
    let paths: Vec<_> = session
        .objects()
        .iter()
        .filter_map(|object| match object {
            ObjectSnapshot::Path(path) => Some(path.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(paths.len(), 2);
    for path in &paths {
        assert!(!path.style.fill.enabled, "Fill off");
        assert!(
            path.subpaths().all(|outline| !outline.closed),
            "open strokes"
        );
    }
    let top = paths[0]
        .all_anchors()
        .map(|a| a.point.y)
        .fold(f64::INFINITY, f64::min);
    assert!(
        (baseline - top - 20.0).abs() < 0.01,
        "criterion 12 on the path box"
    );
    let mut session = with_text("Hi");
    add_rectangle(&mut session);
    select_all(&mut session);
    assert!(matches!(
        session.apply_combine(),
        CombineOutcome::Applied { .. }
    ));
    assert_eq!(texts(&session).len(), 0);
}

/// Criteria 19 and 20: the Boolean operations, Fracture and Flatten refuse a single-line text
/// with its own reason and change nothing; after Object to path its paths are ordinary paths.
#[test]
fn ac19_ac20_area_commands_refuse() {
    let mut session = with_text("Hi");
    add_rectangle(&mut session);
    select_all(&mut session);
    let steps = session.document.own_steps().len();
    let BooleanOutcome::Refused(BooleanRefusal::Texts(refusal)) =
        session.apply_boolean(BooleanOp::Union)
    else {
        panic!("refused")
    };
    assert!(refusal.no_area && refusal.offenders.len() == 1);
    assert!(matches!(
        session.apply_fracture(),
        StackOutcome::Refused(StackRefusal::Operands(BooleanRefusal::Texts(_)))
    ));
    assert!(matches!(
        session.apply_flatten(),
        StackOutcome::Refused(StackRefusal::Operands(BooleanRefusal::Texts(_)))
    ));
    assert_eq!(session.document.own_steps().len(), steps, "nothing changed");
    let text = texts(&session)[0].id;
    session.selection.select_single(text);
    assert_eq!(session.object_to_path(), ObjectToPathOutcome::Converted);
    select_all(&mut session);
    assert!(!matches!(
        session.apply_boolean(BooleanOp::Union),
        BooleanOutcome::Refused(BooleanRefusal::Texts(_))
    ));
}

/// Criteria 19 and 22: Weld refuses a single-line text like the other area commands, outlined,
/// with nothing changed; a missing face is refused first with the missing-font reason.
#[test]
fn ac19_ac22_weld_refuses() {
    let mut session = with_text("Hi");
    let id = texts(&session)[0].id;
    add_rectangle(&mut session);
    select_all(&mut session);
    let steps = session.document.own_steps().len();
    let WeldOutcome::Refused(WeldRefusal::Texts(refusal)) = session.apply_weld() else {
        panic!("refused")
    };
    assert!(refusal.no_area && refusal.missing_faces.is_empty());
    assert_eq!(refusal.offenders, [id]);
    assert!(session.command_refusal.is_some(), "outlined");
    assert_eq!(session.document.own_steps().len(), steps, "nothing changed");
    let lost = FontRef {
        family: "Lost Stroke".to_owned(),
        style: "Regular".to_owned(),
        face_hash: "c".repeat(64),
        face_index: 0,
        kind: FontKind::SingleLine,
    };
    session.document.set_text_font(&[id], &lost).unwrap();
    select_all(&mut session);
    let WeldOutcome::Refused(WeldRefusal::Texts(refusal)) = session.apply_weld() else {
        panic!("refused")
    };
    assert!(!refusal.no_area && refusal.missing_faces == ["Lost Stroke Regular"]);
}

/// Criterion 22: a missing single-line face is drawn in EMS Readability, stroke only, marked
/// missing; Object to path and the area commands give the missing-face refusal.
#[test]
fn ac22_missing_single_line_face() {
    let mut session = with_text("Name");
    let id = texts(&session)[0].id;
    let lost = FontRef {
        family: "Lost Stroke".to_owned(),
        style: "Regular".to_owned(),
        face_hash: "c".repeat(64),
        face_index: 0,
        kind: FontKind::SingleLine,
    };
    session.document.set_text_font(&[id], &lost).unwrap();
    let text = &texts(&session)[0];
    let outline = text.outline.as_ref().unwrap();
    assert!(outline.substitute);
    assert!(!outline.glyphs.style.fill.enabled);
    assert!(
        outline.glyphs.subpaths().any(|o| !o.closed),
        "drawn as lines"
    );
    assert!(session.text_bar().missing);
    let tab: serde_json::Value = serde_json::from_str(&session.fonts_tab_json()).unwrap();
    assert_eq!(tab["current"]["substitute"], "EMS Readability");
    session.set_tool(Tool::Select);
    session.selection.select_single(id);
    let ObjectToPathOutcome::Refused(refusal) = session.object_to_path() else {
        panic!("refused")
    };
    assert_eq!(refusal.missing_faces, ["Lost Stroke Regular"]);
    add_rectangle(&mut session);
    select_all(&mut session);
    let BooleanOutcome::Refused(BooleanRefusal::Texts(refusal)) =
        session.apply_boolean(BooleanOp::Union)
    else {
        panic!("refused")
    };
    assert!(!refusal.no_area && refusal.missing_faces == ["Lost Stroke Regular"]);
}

/// Criterion 6: a single-line face is marked for the Fonts tab, which draws its row's name from
/// the engine's centre lines.
#[test]
fn ac06_single_line_rows() {
    let session = session();
    let index = face_index(&session, "Hershey Script 1-stroke");
    let tab: serde_json::Value = serde_json::from_str(&session.fonts_tab_json()).unwrap();
    assert_eq!(tab["faces"][index]["singleLine"], true);
    let bytes = std::fs::read(fonts_dir().join("hershey-script-1/HersheyScript1.svg")).unwrap();
    let preview: serde_json::Value = serde_json::from_str(
        &crate::font_file_glue::face_preview_path(&bytes, 0, "Hershey Script 1-stroke"),
    )
    .unwrap();
    assert!(preview["d"].as_str().unwrap().starts_with('M'));
    assert!(
        preview["box"][2].as_f64().unwrap() > 5.0,
        "about 15 em wide"
    );
    assert_eq!(
        crate::font_file_glue::face_preview_path(b"not a font", 0, "x"),
        ""
    );
}
