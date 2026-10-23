//! Weld in the session (`specs/0068-laserable-lettering` criteria 1, 11, 19, 20 and 23): one
//! text becomes one path in one step that undo takes back; a missing face refuses.

use curvyo_document_core::{FontKind, FontRef, Length, ObjectSnapshot, Point, RectBounds, unpack};
use curvyo_ui_core::{WeldAvailability, WeldRefusal};

use super::text_tests::{labels, texts, with_text};
use super::weld::WeldOutcome;
use super::{Session, Tool};

fn select_all(session: &mut Session) {
    session.set_tool(Tool::Select);
    let ids: Vec<_> = session.objects().iter().map(ObjectSnapshot::id).collect();
    session.selection.clear();
    for id in ids {
        session.selection.add(id);
    }
}

/// Criterion 1: nothing selected, or another tool, dims the button and a press does nothing; one
/// selected object is enough.
#[test]
fn ac01_weld_needs_the_select_tool_and_one_object() {
    let mut session = with_text("Hi");
    session.set_tool(Tool::Select);
    session.selection.clear();
    assert_eq!(session.weld_availability(), WeldAvailability::NeedsOne);
    assert_eq!(session.apply_weld(), WeldOutcome::Ignored);
    select_all(&mut session);
    assert_eq!(session.weld_availability(), WeldAvailability::Ready);
    session.set_tool(Tool::Rectangle);
    assert_eq!(session.weld_availability(), WeldAvailability::NeedsOne);
    assert_eq!(session.apply_weld(), WeldOutcome::Ignored);
}

/// Criteria 11, 19 and 20: one text welds in one step "weld" into one selected compound path;
/// undo gives the editable text back.
#[test]
fn ac11_ac20_one_text_welds_in_one_step_and_undo_restores_it() {
    let mut session = with_text("Name");
    select_all(&mut session);
    let steps = labels(&session).len();
    let outcome = session.apply_weld();
    let WeldOutcome::Applied {
        objects,
        texts_only,
        compound,
        length_mm,
        ..
    } = outcome
    else {
        panic!("{outcome:?}");
    };
    assert_eq!((objects, texts_only, compound), (1, true, true));
    assert!(length_mm > 0.0);
    assert_eq!(labels(&session).len(), steps + 1);
    assert_eq!(labels(&session).last().unwrap(), "weld");
    assert_eq!(texts(&session).len(), 0);
    let result = session.selection.ids()[0];
    assert_eq!(session.selection.ids().len(), 1);
    assert_eq!(session.tool(), Tool::Select);
    let objects = session.objects();
    let Some(ObjectSnapshot::Path(path)) = objects.iter().find(|o| o.id() == result) else {
        panic!("a path");
    };
    assert!(!path.style.fill.enabled, "line art");
    session.undo();
    assert_eq!(texts(&session).len(), 1);
    assert_eq!(texts(&session)[0].string, "Name");
}

/// Criterion 23: a text with a missing face refuses Weld, outlined, with nothing changed.
#[test]
fn ac23_a_missing_face_refuses() {
    let mut session = with_text("Name");
    let id = texts(&session)[0].id;
    let lost = FontRef {
        family: "Lost Serif".to_owned(),
        style: "Bold".to_owned(),
        face_hash: "c".repeat(64),
        face_index: 0,
        kind: FontKind::Outline,
    };
    session.document.set_text_font(&[id], &lost).unwrap();
    session.set_tool(Tool::Rectangle);
    session.pointer_down(Point::new(0.0, 30.0), false);
    session.pointer_up(Point::new(20.0, 40.0), false, false);
    select_all(&mut session);
    let steps = labels(&session).len();
    let WeldOutcome::Refused(WeldRefusal::Texts(refusal)) = session.apply_weld() else {
        panic!("refused");
    };
    assert_eq!(refusal.missing_faces, ["Lost Serif Bold"]);
    assert_eq!(refusal.offenders, [id]);
    assert_eq!(labels(&session).len(), steps);
    assert!(session.command_refusal.is_some());
    assert_eq!(texts(&session).len(), 1);
}

/// Criterion 22: a welded result saved and reopened keeps its outlines, open or closed state,
/// node order and style.
#[test]
fn ac22_a_welded_result_round_trips() {
    let mut session = Session::new(1);
    for x in [0.0, 10.0] {
        let _ = session.document.create_rect(RectBounds {
            origin: Point::new(x, x),
            width: Length::from_mm(20.0),
            height: Length::from_mm(20.0),
        });
    }
    select_all(&mut session);
    assert!(matches!(session.apply_weld(), WeldOutcome::Applied { .. }));
    let shape = |object: &ObjectSnapshot| {
        let ObjectSnapshot::Path(path) = object else {
            panic!("a path");
        };
        let outlines: Vec<(Vec<Point>, bool)> = path
            .subpaths()
            .map(|s| (s.anchors.iter().map(|a| a.point).collect(), s.closed))
            .collect();
        (outlines, path.style.clone())
    };
    let live = shape(&session.objects()[0]);
    assert_eq!(
        live.0.iter().filter(|o| !o.1).count(),
        1,
        "the trim edge is open"
    );
    let reopened = unpack(5, &session.pack("0.1.0").unwrap()).unwrap();
    let ids = reopened.object_ids();
    assert_eq!(shape(&reopened.object(ids[0]).unwrap()), live);
}
