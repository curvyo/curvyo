//! Width editing on the canvas (`specs/0110` criterion 4): diamonds on the centre line, a click
//! adds a point, a drag along moves it, a drag across sets the width live, Delete, Escape and
//! Done; every gesture is one undo step.

#![allow(clippy::assert_is_empty)]

use super::*;
use curvyo_document_core::NewAnchor;

/// A 40 mm line from (10, 10) with a 2 mm stroke, selected, width editing on.
fn editing(seed: u64) -> (Session, NodeId) {
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
    // The setup's own steps are not the maker's: put them behind the history's mark.
    session.recording_unguarded(|_| ());
    assert!(session.begin_width_edit());
    (session, id)
}

fn drag(session: &mut Session, from: Point, to: Point) {
    session.pointer_down(from, false);
    session.pointer_hover(
        Point::new(f64::midpoint(from.x, to.x), f64::midpoint(from.y, to.y)),
        false,
        false,
    );
    session.pointer_hover(to, false, false);
    session.pointer_up(to, false, false);
}

#[test]
fn a_click_on_the_line_adds_a_point_as_one_undo_step() {
    let (mut session, id) = editing(80);
    let steps = session.undo_len();
    session.pointer_down(Point::new(30.0, 10.0), false);
    session.pointer_up(Point::new(30.0, 10.0), false, false);
    let points = session.document.width_points(id).unwrap();
    assert_eq!(points.len(), 1);
    assert!((points[0].position - 0.5).abs() < 1e-6);
    assert!((points[0].multiplier - 1.0).abs() < 1e-9);
    assert_eq!(session.undo_len(), steps + 1);
    session.undo();
    assert!(session.document.width_points(id).unwrap().is_empty());
}

#[test]
fn a_sideways_drag_sets_the_width_live_and_once() {
    let (mut session, id) = editing(81);
    let steps = session.undo_len();
    session.pointer_down(Point::new(30.0, 10.0), false);
    session.pointer_hover(Point::new(30.2, 12.0), false, false);
    // Live and relative to the press: the 2 mm stroke grows by twice the 2 mm moved, nothing is
    // written yet.
    assert!(session.document.width_points(id).unwrap().is_empty());
    let readout = session.live_readout().unwrap();
    assert_eq!(readout.text, "Width 6 mm");
    assert!((session.width_point_mm().unwrap() - 6.0).abs() < 1e-9);
    session.pointer_hover(Point::new(30.4, 13.0), false, false);
    session.pointer_up(Point::new(30.4, 13.0), false, false);
    let points = session.document.width_points(id).unwrap();
    assert_eq!(points.len(), 1);
    assert!(
        (points[0].multiplier - 4.0).abs() < 1e-9,
        "8 mm on a 2 mm stroke"
    );
    assert!(
        (points[0].position - 0.5).abs() < 1e-6,
        "the axis stays across"
    );
    assert_eq!(session.undo_len(), steps + 1);
    // A second drag is a second step.
    drag(&mut session, Point::new(30.0, 10.0), Point::new(30.0, 12.5));
    assert_eq!(session.undo_len(), steps + 2);
    session.undo();
    assert!((session.document.width_points(id).unwrap()[0].multiplier - 4.0).abs() < 1e-9);
}

#[test]
fn a_drag_along_the_line_moves_the_point_and_keeps_its_width() {
    let (mut session, id) = editing(82);
    session
        .document
        .set_width_point(id, AnchorId::new(82, 9), 0.5, 2.0)
        .unwrap();
    drag(&mut session, Point::new(30.0, 10.0), Point::new(40.0, 10.3));
    let points = session.document.width_points(id).unwrap();
    assert_eq!(points.len(), 1);
    assert!((points[0].position - 0.75).abs() < 1e-6);
    assert!((points[0].multiplier - 2.0).abs() < 1e-9);
}

#[test]
fn a_press_on_a_diamond_without_movement_selects_it_and_writes_nothing() {
    let (mut session, id) = editing(83);
    session
        .document
        .set_width_point(id, AnchorId::new(83, 9), 0.5, 2.0)
        .unwrap();
    let version = session.document.version();
    session.pointer_down(Point::new(30.0, 10.0), false);
    session.pointer_up(Point::new(30.0, 10.0), false, false);
    assert_eq!(session.document.version(), version);
    assert!((session.width_point_mm().unwrap() - 4.0).abs() < 1e-9);
}

#[test]
fn the_numeric_field_sets_the_selected_point_as_one_step() {
    let (mut session, id) = editing(84);
    session
        .document
        .set_width_point(id, AnchorId::new(84, 9), 0.5, 2.0)
        .unwrap();
    assert!(!session.set_width_point_mm(3.0), "no point selected");
    session.pointer_down(Point::new(30.0, 10.0), false);
    session.pointer_up(Point::new(30.0, 10.0), false, false);
    let steps = session.undo_len();
    assert!(session.set_width_point_mm(5.0));
    assert!((session.document.width_points(id).unwrap()[0].multiplier - 2.5).abs() < 1e-9);
    assert_eq!(session.undo_len(), steps + 1);
    assert!(!session.set_width_point_mm(-1.0));
    assert!(!session.set_width_point_mm(f64::NAN));
}

#[test]
fn delete_removes_the_selected_point_as_one_step() {
    let (mut session, id) = editing(85);
    session
        .document
        .set_width_point(id, AnchorId::new(85, 9), 0.5, 2.0)
        .unwrap();
    session
        .document
        .set_width_point(id, AnchorId::new(85, 10), 0.9, 1.0)
        .unwrap();
    session.pointer_down(Point::new(30.0, 10.0), false);
    session.pointer_up(Point::new(30.0, 10.0), false, false);
    let steps = session.undo_len();
    session.delete_selected();
    let points = session.document.width_points(id).unwrap();
    assert_eq!(points.len(), 1);
    assert_eq!(points[0].id, AnchorId::new(85, 10));
    assert_eq!(session.undo_len(), steps + 1);
    assert!(session.document.path(id).is_some(), "the path stays");
}

#[test]
fn escape_cancels_a_drag_first_then_leaves_and_done_leaves() {
    let (mut session, id) = editing(86);
    session.pointer_down(Point::new(30.0, 10.0), false);
    session.pointer_hover(Point::new(30.0, 13.0), false, false);
    session.escape();
    assert!(session.width_edit_active());
    session.pointer_up(Point::new(30.0, 13.0), false, false);
    assert!(session.document.width_points(id).unwrap().is_empty());
    session.escape();
    assert!(!session.width_edit_active());
    assert!(session.begin_width_edit());
    session.end_width_edit();
    assert!(!session.width_edit_active());
}

#[test]
fn hovering_the_line_shows_a_ghost_diamond_and_away_hides_it() {
    let (mut session, _) = editing(87);
    let plain = session.width_handle_overlay().triangle_count();
    session.pointer_hover(Point::new(25.0, 10.1), false, false);
    let ghost = session.width_handle_overlay().triangle_count();
    assert!(ghost > plain);
    session.pointer_hover(Point::new(25.0, 30.0), false, false);
    assert_eq!(session.width_handle_overlay().triangle_count(), plain);
}

#[test]
fn a_drag_is_cancelled_when_the_path_changes_under_it() {
    let (mut session, id) = editing(88);
    session.pointer_down(Point::new(30.0, 10.0), false);
    session
        .document
        .move_anchors(&[(id, AnchorId::new(88, 2), Point::new(60.0, 10.0))])
        .unwrap();
    session.pointer_hover(Point::new(30.0, 13.0), false, false);
    session.pointer_up(Point::new(30.0, 13.0), false, false);
    assert!(session.document.width_points(id).unwrap().is_empty());
    assert!(session.take_width_edit_notice().contains("changed"));
}

#[test]
fn a_spiro_path_takes_its_width_points_on_the_evaluated_curve() {
    let mut session = Session::new(89);
    let controls = [
        NewAnchor::corner(AnchorId::new(89, 1), Point::new(10.0, 10.0)),
        NewAnchor::corner(AnchorId::new(89, 2), Point::new(50.0, 10.0)),
    ];
    let id = session
        .document
        .create_editable_curve(&controls, false, curvyo_document_core::CurveKind::Spiro)
        .unwrap();
    session.selection.set(&[id]);
    assert!(session.begin_width_edit());
    drag(&mut session, Point::new(30.0, 10.0), Point::new(30.0, 12.0));
    assert_eq!(session.document.width_points(id).unwrap().len(), 1);
    assert!(session.frozen_width_artwork().contains_key(&id));
    assert!(matches!(
        session.export_svg(curvyo_ui_core::SvgExportOptions::default()),
        super::super::svg_export::SvgExportResult::Written { .. }
    ));
}

#[test]
fn width_editing_explains_what_it_needs() {
    let mut session = Session::new(90);
    assert!(!session.begin_width_edit());
    assert!(
        session
            .take_unsupported_width_notice()
            .contains("Select one path")
    );
    let id = session.document.create_path(
        &[
            NewAnchor::corner(AnchorId::new(90, 1), Point::new(10.0, 10.0)),
            NewAnchor::corner(AnchorId::new(90, 2), Point::new(50.0, 10.0)),
        ],
        false,
    );
    session
        .document
        .edit_style(
            &[id],
            &curvyo_document_core::StyleEdit::StrokeEnabled(false),
        )
        .unwrap();
    session.selection.set(&[id]);
    assert!(!session.begin_width_edit());
    assert!(session.take_unsupported_width_notice().contains("stroke"));
}

#[test]
fn a_width_out_of_range_is_refused_with_a_notice() {
    let (mut session, id) = editing(91);
    drag(
        &mut session,
        Point::new(30.0, 10.0),
        Point::new(30.0, 1.0e9),
    );
    assert!(session.document.width_points(id).unwrap().is_empty());
    assert!(session.take_width_edit_notice().contains("out of range"));
}

/// UX review U2: while width editing is on, no resize, rotate, skew or move handle is drawn or
/// hovered, so none covers a diamond.
#[test]
fn width_editing_hides_the_select_handles() {
    let (mut session, _) = editing(90);
    assert!(
        session
            .select_transform_decoration_input()
            .handles
            .is_empty(),
        "no handles while editing width"
    );
    session.pointer_hover(Point::new(10.0, 10.0), false, false);
    assert!(session.select_hovered_handle(&session.objects()).is_none());
    session.end_width_edit();
    assert!(
        !session
            .select_transform_decoration_input()
            .handles
            .is_empty(),
        "the handles are back after Done"
    );
}
