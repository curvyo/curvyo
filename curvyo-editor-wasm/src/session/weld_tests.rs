//! Weld in the session (`specs/0068-laserable-lettering` criteria 1, 12, 19, 21, 22, 25, 27):
//! one text becomes one path in one step that undo takes back; a missing face refuses.

use curvyo_document_core::{FontRef, ObjectSnapshot, Point};
use curvyo_ui_core::WeldRefusal;

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

/// Criterion 1: nothing selected, or another tool, does nothing.
#[test]
fn ac01_weld_needs_the_select_tool_and_a_selection() {
    let mut session = with_text("Hi");
    session.set_tool(Tool::Select);
    session.selection.clear();
    assert_eq!(session.apply_weld(0.1), WeldOutcome::Ignored);
    select_all(&mut session);
    session.set_tool(Tool::Rectangle);
    assert_eq!(session.apply_weld(0.1), WeldOutcome::Ignored);
}

/// Criteria 12, 19, 21, 22 and 25: one text welds in one step "weld" into one selected path named
/// by the text; undo gives the editable text back.
#[test]
fn ac12_ac22_one_text_welds_in_one_step_and_undo_restores_it() {
    let mut session = with_text("Name");
    select_all(&mut session);
    let steps = labels(&session).len();
    let outcome = session.apply_weld(0.1);
    let WeldOutcome::Applied {
        objects,
        texts_only,
        compound,
        pieces,
        ..
    } = outcome
    else {
        panic!("{outcome:?}");
    };
    assert_eq!((objects, texts_only, compound), (1, true, true));
    assert_eq!(pieces, 4, "N, a, m, e");
    assert_eq!(labels(&session).len(), steps + 1);
    assert_eq!(labels(&session).last().unwrap(), "weld");
    assert_eq!(texts(&session).len(), 0);
    let result = session.selection.ids()[0];
    assert_eq!(session.selection.ids().len(), 1);
    assert_eq!(session.tool(), Tool::Select);
    assert!(matches!(
        session.objects().iter().find(|o| o.id() == result),
        Some(ObjectSnapshot::Path(_))
    ));
    assert_eq!(
        session
            .document
            .node_registers(result)
            .unwrap()
            .name
            .as_deref(),
        Some("Name")
    );
    session.undo();
    assert_eq!(texts(&session).len(), 1);
    assert_eq!(texts(&session)[0].string, "Name");
}

/// Criterion 27: a text with a missing face refuses Weld, outlined, with nothing changed.
#[test]
fn ac27_a_missing_face_refuses() {
    let mut session = with_text("Name");
    let id = texts(&session)[0].id;
    let lost = FontRef {
        family: "Lost Serif".to_owned(),
        style: "Bold".to_owned(),
        face_hash: "c".repeat(64),
        face_index: 0,
    };
    session.document.set_text_font(&[id], &lost).unwrap();
    session.set_tool(Tool::Rectangle);
    session.pointer_down(Point::new(0.0, 30.0), false);
    session.pointer_up(Point::new(20.0, 40.0), false, false);
    select_all(&mut session);
    let steps = labels(&session).len();
    let WeldOutcome::Refused(WeldRefusal::Texts(refusal)) = session.apply_weld(0.1) else {
        panic!("refused");
    };
    assert_eq!(refusal.missing_faces, ["Lost Serif Bold"]);
    assert_eq!(refusal.offenders, [id]);
    assert_eq!(labels(&session).len(), steps);
    assert!(session.command_refusal.is_some());
    assert_eq!(texts(&session).len(), 1);
}
