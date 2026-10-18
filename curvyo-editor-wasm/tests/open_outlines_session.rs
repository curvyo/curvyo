//! Compound paths with open outlines in the editor session (`specs/0054-combine-open-paths`
//! criteria 4, 6, 18, 21 and 31): a click hits the open outline's stroke and not its closing
//! line, and Combine, Break apart and a Boolean operation are one undo step each that restores
//! every operand.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use curvyo_document_core::{AnchorId, Document, NewAnchor, Point, pack, unpack};
use curvyo_editor_wasm::{BooleanOutcome, BreakApartOutcome, CombineOutcome, Session, Tool};
use curvyo_ui_core::BooleanOp;

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn anchors(first: u64, points: &[(f64, f64)]) -> Vec<NewAnchor> {
    points
        .iter()
        .enumerate()
        .map(|(k, &(x, y))| NewAnchor::corner(AnchorId::new(1, first + k as u64), pt(x, y)))
        .collect()
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

fn select_all(session: &mut Session) {
    let (from, to) = (pt(-50.0, -50.0), pt(200.0, 200.0));
    session.pointer_hover(from, false, false);
    session.pointer_down(from, false);
    session.pointer_hover(to, false, false);
    session.pointer_up(to, false, false);
}

fn json(session: &Session) -> Vec<u8> {
    doc(session).export_json().unwrap()
}

fn doc(session: &Session) -> Document {
    unpack(3, &session.pack("0.1.0").unwrap()).unwrap()
}

/// An open U (criterion 2), a far open line and a far closed square.
fn three_paths() -> Document {
    let document = Document::new(1);
    let _ = document.create_path(
        &anchors(10, &[(0.0, 0.0), (0.0, 10.0), (10.0, 10.0), (10.0, 0.0)]),
        false,
    );
    let _ = document.create_path(&anchors(20, &[(50.0, 0.0), (80.0, 0.0)]), false);
    let _ = document.create_path(
        &anchors(
            30,
            &[(100.0, 0.0), (120.0, 0.0), (120.0, 20.0), (100.0, 20.0)],
        ),
        true,
    );
    document
}

/// Criteria 4 and 6: after Combine, a click on the U's stroke hits the compound path; a click on
/// its closing line does not (fill off).
#[test]
fn c4_the_stroke_is_hit_and_the_closing_line_is_not() {
    let mut session = session_of(&three_paths());
    select_all(&mut session);
    assert!(matches!(
        session.apply_combine(),
        CombineOutcome::Applied { open: 2, .. }
    ));
    click(&mut session, pt(150.0, 150.0), false);
    assert_eq!(session.selected_object_count(), 0);
    click(&mut session, pt(5.0, 0.0), false);
    assert_eq!(
        session.selected_object_count(),
        0,
        "the closing line is not stroked"
    );
    click(&mut session, pt(5.0, 10.0), false);
    assert_eq!(session.selected_object_count(), 1, "the U's stroke");
}

/// Criterion 31: Combine, Break apart and a Boolean operation are one undo step each; undo
/// restores every operand with its ids, nodes, direction and open or closed state.
#[test]
fn c31_each_command_is_one_undo_step() {
    let mut session = session_of(&three_paths());
    let original = json(&session);

    select_all(&mut session);
    let before = session.undo_len();
    assert!(matches!(
        session.apply_combine(),
        CombineOutcome::Applied { .. }
    ));
    assert_eq!(session.undo_len(), before + 1);
    let combined = json(&session);
    assert!(matches!(
        session.apply_break_apart(),
        BreakApartOutcome::Applied { pieces: 3, .. }
    ));
    assert_eq!(session.undo_len(), before + 2);
    let _ = session.undo();
    assert_eq!(json(&session), combined, "Break apart undone");
    let _ = session.undo();
    assert_eq!(json(&session), original, "Combine undone");

    // The U and the square; the straight line alone would have no area (criterion 22).
    let mut session = session_of(&three_paths());
    click(&mut session, pt(150.0, 150.0), false);
    click(&mut session, pt(5.0, 10.0), false);
    click(&mut session, pt(100.0, 10.0), true);
    assert_eq!(session.selected_object_count(), 2);
    let union = session.apply_boolean(BooleanOp::Union);
    assert!(
        matches!(
            union,
            BooleanOutcome::Applied {
                closed_open: 1,
                removed_open: 0,
                ..
            }
        ),
        "{union:?}"
    );
    let _ = session.undo();
    assert_eq!(json(&session), original, "Union undone");
}
