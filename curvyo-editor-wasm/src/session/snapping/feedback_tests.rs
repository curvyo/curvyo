use super::*;
use crate::session::{Session, Tool};
use curvyo_document_core::{RectBounds, Vec2};
#[test]
fn alt_inverts_only_running_move_without_a_document_write() {
    for enabled in [false, true] {
        let mut s = Session::new(19);
        let id = s.document.create_rect(RectBounds::from_corners(
            Point::new(10.0, 10.0),
            Point::new(30.0, 30.0),
        ));
        let _ = s.document.create_rect(RectBounds::from_corners(
            Point::new(100.0, 10.0),
            Point::new(120.0, 30.0),
        ));
        s.set_snapping(DocumentSnapping {
            enabled,
            page: false,
            nodes: false,
            guides: false,
            ..DocumentSnapping::DEFAULT
        });
        s.selection.set(&[id]);
        s.pointer_down(Point::new(20.0, 20.0), false);
        s.pointer_hover(Point::new(89.0, 20.0), false, false);
        let version = s.document.version();
        s.modifiers_changed(false, false, true);
        assert_eq!(s.snap_result().unwrap().is_empty(), enabled);
        s.modifiers_changed(false, true, true);
        assert_eq!(s.snap_result().unwrap().is_empty(), enabled);
        s.modifiers_changed(false, false, false);
        assert_eq!(s.snap_result().unwrap().is_empty(), !enabled);
        assert_eq!(s.document.version(), version);
        assert_eq!(s.snapping().enabled, enabled);
    }
}

#[test]
fn drawing_hover_and_simple_click_resolve_before_any_drag() {
    for tool in [Tool::Rectangle, Tool::Ellipse, Tool::PolygonStar, Tool::Pen] {
        let mut s = Session::new(20);
        s.set_snapping(DocumentSnapping {
            enabled: true,
            ..DocumentSnapping::DEFAULT
        });
        s.set_tool(tool);
        let version = s.document.version();
        s.pointer_hover(Point::new(53.0, 75.0), false, false);
        assert!(!s.snap_result().unwrap().is_empty(), "{tool:?}");
        assert!(!s.snapping_toolbar().has_drag);
        assert_eq!(s.document.version(), version);
        s.pointer_down(Point::new(53.0, 75.0), false);
        s.pointer_up(Point::new(53.0, 75.0), false, false);
        if tool == Tool::Pen {
            assert_eq!(
                s.pen.in_progress_nodes().unwrap()[0].point,
                Point::new(52.5, 74.25)
            );
        } else {
            assert_eq!(s.placement().unwrap().a(), Point::new(52.5, 74.25));
        }
        s.pointer_leave();
        assert!(
            s.snap_result()
                .is_none_or(curvyo_ui_core::SnapResult::is_empty)
        );
    }
}

#[test]
fn placement_follow_and_frozen_typed_size_keep_snapped_anchor() {
    let mut s = Session::new(22);
    s.set_snapping(DocumentSnapping {
        enabled: true,
        ..DocumentSnapping::DEFAULT
    });
    s.set_tool(Tool::Rectangle);
    // No preceding hover is required.
    s.pointer_down(Point::new(53.0, 75.0), false);
    s.pointer_up(Point::new(53.0, 75.0), false, false);
    assert_eq!(s.placement().unwrap().a(), Point::new(52.5, 74.25));
    s.pointer_hover(Point::new(104.0, 149.0), false, false);
    assert_eq!(
        s.placement().unwrap().following().unwrap().0,
        Point::new(105.0, 148.5)
    );
    s.placement_edit(0, "20");
    s.placement_edit(1, "10");
    assert!(s.snap_result().is_none());
    s.pointer_hover(Point::new(160.0, 200.0), false, false);
    assert!(s.place_placement());
    let id = s.document.object_ids()[0];
    assert_eq!(
        curvyo_ui_core::object_bounds(&s.document.object(id).unwrap()),
        (Point::new(52.5, 74.25), Point::new(72.5, 84.25))
    );
}

#[test]
fn hover_axes_follow_history_and_clear_on_escape_and_disable() {
    let mut s = Session::new(23);
    s.set_tool(Tool::Rectangle);
    s.set_snapping(DocumentSnapping {
        enabled: true,
        x: false,
        ..DocumentSnapping::DEFAULT
    });
    s.pointer_hover(Point::new(53.0, 75.0), false, false);
    assert_eq!(s.snap_result().unwrap().offset, Vec2::new(0.0, -0.75));
    s.set_snapping(DocumentSnapping {
        x: true,
        y: false,
        ..s.snapping()
    });
    s.pointer_hover(Point::new(53.0, 75.0), false, false);
    assert_eq!(s.snap_result().unwrap().offset, Vec2::new(-0.5, 0.0));
    s.undo();
    assert!(s.snap_result().is_none());
    assert!(!s.snapping().x && s.snapping().y);
    s.pointer_hover(Point::new(53.0, 75.0), false, false);
    s.escape();
    assert!(s.snap_result().is_none());
    s.pointer_hover(Point::new(53.0, 75.0), false, false);
    s.set_snapping(DocumentSnapping {
        enabled: false,
        ..s.snapping()
    });
    assert!(s.snap_result().is_none());
}

#[test]
fn repeated_raw_placement_click_does_not_confirm_after_peer_disables_snapping() {
    let mut s = Session::new(24);
    s.set_tool(Tool::Rectangle);
    s.set_snapping(DocumentSnapping {
        enabled: true,
        ..DocumentSnapping::DEFAULT
    });
    let raw = Point::new(51.0, 74.0);
    s.pointer_down(raw, false);
    s.pointer_up(raw, false, false);
    assert_eq!(s.placement().unwrap().a(), Point::new(52.5, 74.25));
    s.document.set_snapping(DocumentSnapping {
        enabled: false,
        ..s.snapping()
    });
    let version = s.document.version();
    s.pointer_down(raw, false);
    s.pointer_up(raw, false, false);
    assert!(s.placement().is_some());
    assert_eq!(
        s.document.object_ids(),
        [] as [curvyo_document_core::NodeId; 0]
    );
    assert_eq!(s.document.version(), version);
}
