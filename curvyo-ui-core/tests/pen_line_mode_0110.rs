//! Line mode uses corner nodes even when the pointer is dragged.
#![allow(clippy::unwrap_used, clippy::assert_is_empty)]

use curvyo_document_core::{AnchorKind, Document, Length, Point, Vec2};
use curvyo_ui_core::{AnchorIdMinter, PenMode, PenTool};

#[test]
fn line_mode_drag_keeps_preview_and_committed_node_straight() {
    let document = Document::new(1);
    let mut minter = AnchorIdMinter::new(1);
    let mut pen = PenTool::new();
    assert_eq!(pen.mode(), PenMode::Bezier);
    assert!(pen.set_mode(PenMode::Lines));
    pen.pointer_down(Point::new(1.0, 2.0), Length::from_mm(0.1));
    let preview = pen
        .pending_anchor(minter.peek(), Point::new(8.0, 9.0), Length::from_mm(0.1))
        .unwrap();
    assert_eq!(preview.kind, AnchorKind::Corner);
    assert_eq!(preview.handle_out, Vec2::ZERO);
    assert!(!pen.set_mode(PenMode::Bezier));
    pen.pointer_up(
        &mut minter,
        &document,
        Point::new(8.0, 9.0),
        Length::from_mm(0.1),
    );
    assert_eq!(pen.in_progress_nodes().unwrap()[0], preview);
}

#[test]
fn freehand_press_drag_release_creates_one_path() {
    let document = Document::new(1);
    let mut minter = AnchorIdMinter::new(1);
    let mut pen = PenTool::new();
    assert!(pen.set_mode(PenMode::Freehand));
    assert!(pen.set_freehand_smoothing(Length::from_mm(0.1)));
    pen.pointer_down(Point::new(0.0, 0.0), Length::from_mm(0.1));
    for x in [1.0, 2.0, 3.0] {
        pen.freehand_move(&mut minter, Point::new(x, 0.0));
    }
    pen.pointer_up(
        &mut minter,
        &document,
        Point::new(4.0, 0.0),
        Length::from_mm(0.1),
    );
    assert!(!pen.is_placing());
    let ids = document.object_ids();
    assert_eq!(ids.len(), 1);
    let path = document.path(ids[0]).unwrap();
    assert_eq!(path.anchors.len(), 2);
    assert_eq!(path.anchors.first().unwrap().point, Point::new(0.0, 0.0));
    assert_eq!(path.anchors.last().unwrap().point, Point::new(4.0, 0.0));
}

#[test]
fn freehand_click_without_motion_finishes_without_a_path() {
    let document = Document::new(1);
    let mut minter = AnchorIdMinter::new(1);
    let mut pen = PenTool::new();
    assert!(pen.set_mode(PenMode::Freehand));
    let point = Point::new(10.0, 10.0);
    pen.pointer_down(point, Length::from_mm(0.1));
    pen.pointer_up(&mut minter, &document, point, Length::from_mm(0.1));
    assert!(!pen.is_placing());
    assert!(document.object_ids().is_empty());
}

#[test]
fn freehand_smoothing_preserves_a_deviation_over_tolerance() {
    let document = Document::new(1);
    let mut minter = AnchorIdMinter::new(1);
    let mut pen = PenTool::new();
    assert!(pen.set_mode(PenMode::Freehand));
    assert!(!pen.set_freehand_smoothing(Length::from_mm(f64::NAN)));
    assert!(pen.set_freehand_smoothing(Length::from_mm(0.1)));
    pen.pointer_down(Point::new(0.0, 0.0), Length::from_mm(0.1));
    pen.freehand_move(&mut minter, Point::new(1.0, 1.0));
    pen.pointer_up(
        &mut minter,
        &document,
        Point::new(2.0, 0.0),
        Length::from_mm(0.1),
    );
    assert_eq!(
        document
            .path(document.object_ids()[0])
            .unwrap()
            .anchors
            .len(),
        3
    );
}

#[test]
fn bspline_pen_commits_editable_controls_in_one_step() {
    use curvyo_document_core::CurveKind;
    let document = Document::new(1);
    let mut minter = AnchorIdMinter::new(1);
    let mut pen = PenTool::new();
    assert!(pen.set_mode(PenMode::BSpline));
    for point in [
        Point::new(0.0, 0.0),
        Point::new(10.0, 5.0),
        Point::new(20.0, 0.0),
    ] {
        pen.pointer_down(point, Length::from_mm(0.1));
        pen.pointer_up(&mut minter, &document, point, Length::from_mm(0.1));
    }
    let id = pen.finish(&document).unwrap();
    let path = document.path(id).unwrap();
    assert_eq!(path.curve_kind, CurveKind::BSpline);
    assert_eq!(path.anchors.len(), 3);
    assert!(path.anchors.iter().all(|a| a.kind == AnchorKind::Symmetric));
    let step = document.own_steps().last().unwrap().step;
    document.undo_step(step).unwrap();
    assert!(document.path(id).is_none());
}
