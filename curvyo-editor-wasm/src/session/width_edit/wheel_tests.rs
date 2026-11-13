//! `specs/0114` criteria 5 to 7: the hit area of a width diamond, the typed width a double-click
//! opens and the wheel over a width point.

use super::*;
use curvyo_document_core::NewAnchor;

/// A 40 mm line from (10, 10) with a 2 mm stroke and a width point in its middle, width
/// editing on, the point not selected.
fn with_point(seed: u64) -> (Session, NodeId) {
    let mut session = Session::new(seed);
    let id = session.document.create_path(
        &[
            NewAnchor::corner(AnchorId::new(seed, 1), Point::new(10.0, 10.0)),
            NewAnchor::corner(AnchorId::new(seed, 2), Point::new(50.0, 10.0)),
        ],
        false,
    );
    session
        .document
        .edit_style(
            &[id],
            &curvyo_document_core::StyleEdit::StrokeWidth(Length::from_mm(2.0)),
        )
        .unwrap();
    session.selection.set(&[id]);
    session.recording_unguarded(|_| ());
    assert!(session.begin_width_edit());
    session.pointer_down(Point::new(30.0, 10.0), false);
    session.pointer_up(Point::new(30.0, 10.0), false, false);
    session.recording_unguarded(|_| ());
    if let Some(state) = &mut session.width_edit {
        state.selected = None;
    }
    (session, id)
}

fn width_mm(session: &Session, id: NodeId) -> f64 {
    2.0 * session.document.width_points(id).unwrap()[0].multiplier
}

#[test]
fn a_diamond_takes_the_pointer_beyond_a_node_tolerance() {
    let (session, _) = with_point(90);
    let px = 1.0 / session.view().scale();
    assert!(session.width_point_under(Point::new(30.0 + 10.0 * px, 10.0)));
    assert!(!session.width_point_under(Point::new(30.0 + 30.0 * px, 10.0)));
}

#[test]
fn a_wheel_burst_over_a_point_changes_its_width_live_and_once() {
    let (mut session, id) = with_point(91);
    let steps = session.undo_len();
    let at = Point::new(30.0, 10.0);
    for _ in 0..3 {
        assert!(session.width_wheel(at, 1.0, false));
    }
    assert!(
        (width_mm(&session, id) - 2.0).abs() < 1e-9,
        "nothing written yet"
    );
    assert_eq!(session.live_readout().unwrap().text, "Width 3.5 mm");
    assert!(session.width_wheel(at, -1.0, true));
    assert!(session.end_width_wheel());
    assert!((width_mm(&session, id) - 3.4).abs() < 1e-9);
    assert_eq!(session.undo_len(), steps + 1, "one burst, one step");
    session.undo();
    assert!((width_mm(&session, id) - 2.0).abs() < 1e-9);
}

#[test]
fn the_wheel_never_goes_below_zero_and_elsewhere_is_not_taken() {
    let (mut session, id) = with_point(92);
    assert!(
        !session.width_wheel(Point::new(20.0, 30.0), 1.0, false),
        "not over a point"
    );
    assert!(session.width_wheel(Point::new(30.0, 10.0), -40.0, false));
    session.pointer_down(Point::new(20.0, 30.0), false);
    session.pointer_up(Point::new(20.0, 30.0), false, false);
    assert!(
        width_mm(&session, id).abs() < 1e-9,
        "a press ends the burst"
    );
}

#[test]
fn a_double_click_on_a_point_opens_the_typed_width_without_a_step() {
    let (mut session, id) = with_point(93);
    let steps = session.undo_len();
    let at = Point::new(30.0, 10.0);
    session.pointer_down(at, false);
    session.pointer_up(at, false, false);
    assert_eq!(
        session.double_click_hint(at, false, false),
        crate::session::DoubleClickHint::WidthEntry
    );
    assert_eq!(session.undo_len(), steps, "opening changes nothing");
    assert!((session.width_point_mm().unwrap() - 2.0).abs() < 1e-9);
    assert!(session.set_width_point_mm(5.25));
    assert!((width_mm(&session, id) - 5.25).abs() < 1e-9);
    assert_eq!(session.undo_len(), steps + 1);
}

#[test]
fn an_undo_during_a_burst_takes_the_burst_back() {
    let (mut session, id) = with_point(94);
    let steps = session.undo_len();
    assert!(session.width_wheel(Point::new(30.0, 10.0), 4.0, false));
    session.undo();
    assert!(
        (width_mm(&session, id) - 2.0).abs() < 1e-9,
        "the width before the wheel"
    );
    assert_eq!(session.undo_len(), steps);
    assert_eq!(session.take_width_edit_notice(), "");
}

/// UX review 2026-11-12: the typed width and the wheel readout sit beside the diamond, not the
/// pointer, so the host asks where the selected point's diamond is, live while the wheel turns.
#[test]
fn the_selected_point_says_where_its_diamond_is_and_its_live_width() {
    let (mut session, _) = with_point(94);
    assert_eq!(session.width_point_screen(), None, "nothing selected");
    let px = 1.0 / session.view().scale();
    assert!(session.width_wheel(Point::new(30.0 + 5.0 * px, 10.0), 2.0, false));
    let (x, y) = session.width_point_screen().unwrap();
    let expected = session.view().document_to_screen(Point::new(30.0, 10.0));
    assert!((x - expected.0).abs() < 0.01 && (y - expected.1).abs() < 0.01);
    assert!(
        (session.width_point_mm().unwrap() - 3.0).abs() < 1e-9,
        "live width while it turns"
    );
}
