//! Stroke to path in the session (`specs/0085-stroke-to-path` criteria 1, 15, 17 to 22, 26):
//! the enabled rule, one step that undo and redo take, the selection after, texts, clones,
//! guides and skips, the refusal of a stroke out of range.

use curvyo_document_core::{
    Color, Length, NewAnchor, ObjectSnapshot, Point, RectBounds, StyleEdit,
};

use super::stroke_to_path::StrokeToPathOutcome;
use super::text_tests::{labels, texts, with_text};
use super::{Session, Tool};

fn select(session: &mut Session, ids: &[curvyo_document_core::NodeId]) {
    session.set_tool(Tool::Select);
    session.selection.clear();
    for &id in ids {
        session.selection.add(id);
    }
}

fn rect(session: &Session, x: f64) -> curvyo_document_core::NodeId {
    session.document.create_rect(RectBounds {
        origin: Point::new(x, 0.0),
        width: Length::from_mm(20.0),
        height: Length::from_mm(20.0),
    })
}

fn line(session: &Session, from: Point, to: Point) -> curvyo_document_core::NodeId {
    let mut minter = curvyo_ui_core::AnchorIdMinter::new(9);
    let anchors = [
        NewAnchor::corner(minter.mint(), from),
        NewAnchor::corner(minter.mint(), to),
    ];
    session.document.create_path(&anchors, false)
}

/// Criterion 1: enabled with a stroked object under the Select tool, dimmed otherwise.
#[test]
fn ac01_the_button_needs_a_stroke() {
    let mut session = Session::new(1);
    let a = rect(&session, 0.0);
    select(&mut session, &[]);
    assert!(!session.stroke_to_path_available());
    select(&mut session, &[a]);
    assert!(session.stroke_to_path_available());
    session
        .document
        .edit_style(&[a], &StyleEdit::StrokeEnabled(false))
        .unwrap();
    assert!(!session.stroke_to_path_available());
    assert_eq!(session.apply_stroke_to_path(), StrokeToPathOutcome::Ignored);
}

/// Criteria 14, 19 to 21: a filled rectangle stays a rectangle without stroke, its outline lies
/// above it, both are selected, one step `stroke_to_path`, undo and redo.
#[test]
fn ac19_ac21_one_step_two_objects_selected_undo_and_redo() {
    let mut session = Session::new(1);
    let a = rect(&session, 0.0);
    session
        .document
        .edit_style(&[a], &StyleEdit::FillEnabled(true))
        .unwrap();
    select(&mut session, &[a]);
    let steps = labels(&session).len();
    let outcome = session.apply_stroke_to_path();
    assert_eq!(
        outcome,
        StrokeToPathOutcome::Applied {
            strokes: 1,
            unstroked: 0,
            texts: 0,
            unlinked: 0
        }
    );
    assert_eq!(labels(&session).len(), steps + 1);
    assert_eq!(labels(&session).last().unwrap(), "stroke_to_path");
    assert_eq!(session.tool(), Tool::Select);
    let ids = session.selection.ids().to_vec();
    assert_eq!(ids.len(), 2);
    let objects = session.objects();
    assert!(matches!(objects[0], ObjectSnapshot::Primitive(_)));
    assert!(!objects[0].style().stroke.enabled);
    assert!(matches!(objects[1], ObjectSnapshot::Path(_)));
    assert!(objects[1].style().fill.enabled);
    session.undo();
    assert_eq!(session.objects().len(), 1);
    assert!(session.objects()[0].style().stroke.enabled);
    assert_eq!(session.selection.ids(), [a]);
    session.redo();
    assert_eq!(session.objects().len(), 2);
}

/// Criteria 18 and 22: objects without a stroke and guides are left as they are; the first are
/// counted.
#[test]
fn ac18_unstroked_objects_and_guides_are_skipped() {
    let mut session = Session::new(1);
    let stroked = line(&session, Point::new(0.0, 0.0), Point::new(50.0, 0.0));
    let bare = rect(&session, 60.0);
    let guide = line(&session, Point::new(0.0, 40.0), Point::new(50.0, 40.0));
    session
        .document
        .edit_style(&[bare], &StyleEdit::StrokeEnabled(false))
        .unwrap();
    session.document.set_guide(&[guide], true).unwrap();
    select(&mut session, &[stroked, bare, guide]);
    let StrokeToPathOutcome::Applied {
        strokes, unstroked, ..
    } = session.apply_stroke_to_path()
    else {
        panic!("applied");
    };
    assert_eq!((strokes, unstroked), (1, 1));
    // The open line had Fill off: it became its outline under its own id.
    let objects = session.objects();
    let ObjectSnapshot::Path(path) = objects.iter().find(|o| o.id() == stroked).unwrap() else {
        panic!("a path");
    };
    assert!(path.closed && path.style.fill.enabled);
    let guide_now = objects.iter().find(|o| o.id() == guide).unwrap();
    assert!(guide_now.style().stroke.enabled, "the guide is unchanged");
}

/// Criterion 17: a clone of the converted original is unlinked and keeps its look.
#[test]
fn ac17_clones_of_a_converted_original_are_unlinked() {
    let mut session = Session::new(1);
    let original = line(&session, Point::new(0.0, 0.0), Point::new(50.0, 0.0));
    let clone = session.document.linked_clone(&[original], &[]).unwrap()[0];
    select(&mut session, &[original]);
    let StrokeToPathOutcome::Applied { unlinked, .. } = session.apply_stroke_to_path() else {
        panic!("applied");
    };
    assert_eq!(unlinked, 1);
    let objects = session.objects();
    let ObjectSnapshot::Path(path) = objects.iter().find(|o| o.id() == clone).unwrap() else {
        panic!("the clone is a path now");
    };
    assert!(path.style.stroke.enabled && !path.closed);
}

/// Criterion 15: a text with a stroke is converted and each glyph's stroke outlined, in one step.
#[test]
fn ac15_a_stroked_text_is_converted_in_the_same_step() {
    let mut session = with_text("Hi");
    let id = texts(&session)[0].id;
    session
        .document
        .edit_style(&[id], &StyleEdit::StrokeColor(Color { r: 0, g: 0, b: 200 }))
        .unwrap();
    select(&mut session, &[id]);
    let steps = labels(&session).len();
    let StrokeToPathOutcome::Applied {
        texts: converted,
        strokes,
        ..
    } = session.apply_stroke_to_path()
    else {
        panic!("applied");
    };
    assert_eq!((converted, strokes), (1, 1));
    assert_eq!(labels(&session).len(), steps + 1);
    assert_eq!(texts(&session).len(), 0);
    let outlines = session
        .objects()
        .iter()
        .filter(|o| o.style().fill.enabled && o.style().fill.color == Color { r: 0, g: 0, b: 200 })
        .count();
    assert!(outlines >= 2, "one outline per glyph: {outlines}");
    session.undo();
    assert_eq!(texts(&session).len(), 1, "the text is editable again");
}

/// Criterion 26: a stroke beyond the coordinate limit refuses, outlined, nothing changed.
#[test]
fn ac26_out_of_range_refuses() {
    let mut session = Session::new(1);
    // The kernel's limit is 10 km (`MAX_COORDINATE_MM`).
    let far = 1.0e7 - 0.5;
    let id = line(&session, Point::new(far - 10.0, 0.0), Point::new(far, 0.0));
    select(&mut session, &[id]);
    let steps = labels(&session).len();
    let outcome = session.apply_stroke_to_path();
    assert!(
        matches!(outcome, StrokeToPathOutcome::Refused(_)),
        "{outcome:?}"
    );
    assert_eq!(labels(&session).len(), steps);
    assert!(session.command_refusal.is_some());
}

/// Criterion 25: a text whose face is missing refuses the whole command, nothing changed.
#[test]
fn ac25_a_missing_face_refuses() {
    let mut session = with_text("Name");
    let id = texts(&session)[0].id;
    let lost = curvyo_document_core::FontRef {
        family: "Lost Serif".to_owned(),
        style: "Bold".to_owned(),
        face_hash: "c".repeat(64),
        face_index: 0,
        kind: curvyo_document_core::FontKind::Outline,
    };
    session.document.set_text_fonts(&[(id, lost)]).unwrap();
    let other = rect(&session, 50.0);
    select(&mut session, &[id, other]);
    let steps = labels(&session).len();
    let StrokeToPathOutcome::Refused(curvyo_ui_core::StrokeToPathRefusal::Texts(refusal)) =
        session.apply_stroke_to_path()
    else {
        panic!("refused");
    };
    assert_eq!(refusal.offenders, [id]);
    assert_eq!(labels(&session).len(), steps);
    assert!(session.command_refusal.is_some());
    assert!(session.objects().iter().all(|o| o.style().stroke.enabled));
}
