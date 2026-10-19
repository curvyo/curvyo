//! Fracture and Flatten in the editor session (`specs/0037-fracture-and-flatten` criteria 2, 6,
//! 8, 9, 11, 12 and 14): one commit with the command's label, pieces at their owner's place with
//! its style, the selection afterwards, and refusals that change nothing.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use curvyo_document_core::{
    Color, Document, Length, NodeId, Point, RectBounds, StyleEdit, pack, unpack,
};
use curvyo_editor_wasm::{Session, StackOutcome, Tool};
use curvyo_ui_core::{BooleanRefusal, StackRefusal};

fn rect(document: &Document, x: f64, y: f64, side: f64) -> NodeId {
    document.create_rect(RectBounds {
        origin: Point::new(x, y),
        width: Length::from_mm(side),
        height: Length::from_mm(side),
    })
}

fn session_of(document: &Document) -> Session {
    let mut session = Session::open(2, &pack(document, "0.1.0").unwrap()).unwrap();
    session.set_tool(Tool::Select);
    session.resize_viewport(1200.0, 800.0);
    session
}

fn click(session: &mut Session, at: Point, shift: bool) {
    session.pointer_hover(at, shift, false);
    session.pointer_down(at, shift);
    session.pointer_up(at, shift, false);
}

const RED: Color = Color { r: 255, g: 0, b: 0 };
const BLUE: Color = Color { r: 0, g: 0, b: 255 };

fn commit_labels(session: &Session) -> Vec<String> {
    let loro = loro::LoroDoc::new();
    loro.import(&doc(session).export_loro_snapshot().unwrap())
        .unwrap();
    let ids: Vec<loro::ID> = loro.oplog_frontiers().iter().collect();
    let mut changes: Vec<(u32, String)> = Vec::new();
    loro.travel_change_ancestors(&ids, &mut |change| {
        changes.push((
            change.lamport,
            change
                .message
                .map(|m| m.split(';').next().unwrap_or_default().to_string())
                .unwrap_or_default(),
        ));
        std::ops::ControlFlow::Continue(())
    })
    .unwrap();
    changes.sort();
    changes.into_iter().map(|(_, label)| label).collect()
}

fn doc(session: &Session) -> Document {
    unpack(3, &session.pack("0.1.0").unwrap()).unwrap()
}

/// A bottom square at the origin, an unselected one between, a top square at (10, 10), all of
/// side 20 mm; the bottom one red, the top one blue. Bottom and top are selected.
fn stack() -> (Session, Document) {
    let document = Document::new(1);
    let bottom = rect(&document, 0.0, 0.0, 20.0);
    let _between = rect(&document, 100.0, 0.0, 20.0);
    let top = rect(&document, 10.0, 10.0, 20.0);
    for (id, colour) in [(bottom, RED), (top, BLUE)] {
        document
            .edit_style(&[id], &StyleEdit::FillColor(colour))
            .unwrap();
    }
    let mut session = session_of(&document);
    click(&mut session, Point::new(30.0, 20.0), false); // the top square's right edge
    click(&mut session, Point::new(0.0, 10.0), true); // the bottom square's left edge
    assert_eq!(session.selected_object_count(), 2);
    (session, document)
}

/// Criteria 6, 8 and 9: three pieces in one commit `fracture`, at their owners' places with their
/// styles, all selected; the object between keeps its place.
#[test]
fn fracture_writes_the_pieces_at_their_owners_places() {
    let (mut session, _) = stack();
    let before = commit_labels(&session).len();
    let outcome = session.apply_fracture();
    let StackOutcome::Applied {
        removed, pieces, ..
    } = outcome
    else {
        panic!("applied expected: {outcome:?}");
    };
    assert_eq!((removed, pieces), (2, 3));
    let document = doc(&session);
    let labels = commit_labels(&session);
    assert_eq!(labels.len(), before + 1);
    assert_eq!(labels.last().map(String::as_str), Some("fracture"));
    let colours: Vec<Color> = document
        .object_ids()
        .into_iter()
        .map(|id| document.object(id).unwrap().style().fill.color)
        .collect();
    let grey = colours[1];
    assert_eq!(
        colours,
        vec![RED, grey, BLUE, BLUE],
        "the bottom L, the object between, then the overlap and the top L"
    );
    assert_eq!(session.selected_object_count(), 3);
    assert_eq!(session.tool(), Tool::Select);
}

/// Criteria 11, 12 and 14: a square covered completely is removed in one commit `flatten`; the
/// one above stays selected.
#[test]
fn flatten_removes_a_hidden_object() {
    let document = Document::new(1);
    let _small = rect(&document, 5.0, 5.0, 10.0);
    let large = rect(&document, 0.0, 0.0, 20.0);
    let mut session = session_of(&document);
    click(&mut session, Point::new(20.0, 10.0), false);
    click(&mut session, Point::new(15.0, 10.0), true);
    assert_eq!(session.selected_object_count(), 2);
    let outcome = session.apply_flatten();
    assert!(
        matches!(
            outcome,
            StackOutcome::Applied {
                operands: 2,
                removed: 1,
                pieces: 0,
                hidden: 1,
                ..
            }
        ),
        "{outcome:?}"
    );
    let after = doc(&session);
    assert_eq!(after.object_ids(), vec![large]);
    assert_eq!(
        commit_labels(&session).last().map(String::as_str),
        Some("flatten")
    );
    assert_eq!(session.selected_object_count(), 1);
}

/// Criterion 2 and 5: a group in the selection is refused and nothing changes.
#[test]
fn a_group_is_refused() {
    let (mut session, _) = stack();
    let before = doc(&session).export_json().unwrap();
    session.group_selection();
    click(&mut session, Point::new(130.0, 10.0), true);
    let grouped = doc(&session).export_json().unwrap();
    assert_ne!(before, grouped);
    let outcome = session.apply_fracture();
    assert!(
        matches!(
            outcome,
            StackOutcome::Refused(StackRefusal::Operands(BooleanRefusal::Groups { .. }))
        ),
        "{outcome:?}"
    );
    assert_eq!(doc(&session).export_json().unwrap(), grouped);
}
