use super::*;
use crate::session::{Session, Tool};
use curvyo_document_core::{RectBounds, Vec2};
#[test]
fn move_preview_matches_snapped_commit_and_suspension_does_not_copy() {
    let mut s = Session::new(1);
    let id = s.document.create_rect(RectBounds::from_corners(
        Point::new(10.0, 10.0),
        Point::new(30.0, 30.0),
    ));
    let _ = s.document.create_rect(RectBounds::from_corners(
        Point::new(100.0, 10.0),
        Point::new(120.0, 30.0),
    ));
    s.set_snapping(DocumentSnapping {
        enabled: true,
        nodes: false,
        guides: false,
        page: false,
        ..DocumentSnapping::DEFAULT
    });
    s.set_tool(Tool::Select);
    s.selection.set(&[id]);
    s.pointer_down(Point::new(20.0, 20.0), false);
    s.pointer_hover(Point::new(89.0, 20.0), false, false);
    let live = s.select_live_edit_in(&s.objects()).unwrap();
    assert_eq!(curvyo_ui_core::object_bounds(&live.objects[0]).1.x, 100.0);
    s.pointer_up(Point::new(89.0, 20.0), false, false);
    assert_eq!(
        curvyo_ui_core::object_bounds(&s.document.object(id).unwrap())
            .1
            .x,
        100.0
    );
    s.pointer_down(Point::new(90.0, 20.0), false);
    s.modifiers_changed(false, true, true);
    s.pointer_hover(Point::new(140.0, 60.0), false, true);
    let live = s.select_live_edit_in(&s.objects()).unwrap();
    assert!(!live.copy);
    s.pointer_up(Point::new(140.0, 60.0), false, true);
    assert_eq!(s.document.object_ids().len(), 2);
    assert_eq!(
        curvyo_ui_core::object_bounds(&s.document.object(id).unwrap()).0,
        Point::new(80.0, 10.0).translated(Vec2::new(50.0, 40.0))
    );
}
#[test]
fn settings_normalize_and_refresh_after_history() {
    let mut s = Session::new(1);
    assert!(s.set_snapping(DocumentSnapping {
        enabled: true,
        capture_px: 25.0,
        release_px: 0.0,
        ..DocumentSnapping::DEFAULT
    }));
    assert_eq!(s.snapping_settings().release_px, 25.0);
    s.undo();
    assert_eq!(s.snapping(), DocumentSnapping::DEFAULT);
    s.redo();
    assert!(s.snapping_settings().enabled);
    assert_eq!(s.snapping_settings().release_px, 25.0);
}

#[test]
fn creation_snaps_first_press_and_release_to_page_lines() {
    let mut s = Session::new(1);
    s.set_snapping(DocumentSnapping {
        enabled: true,
        ..DocumentSnapping::DEFAULT
    });
    s.set_tool(Tool::Rectangle);
    s.pointer_down(Point::new(53.0, 75.0), false);
    s.pointer_hover(Point::new(104.0, 149.0), false, false);
    s.pointer_up(Point::new(104.0, 149.0), false, false);
    let id = s.document.object_ids()[0];
    let bounds = curvyo_ui_core::object_bounds(&s.document.object(id).unwrap());
    assert_eq!(bounds, (Point::new(52.5, 74.25), Point::new(105.0, 148.5)));
}

#[test]
fn pen_snapped_click_has_no_accidental_handle_from_double_offset() {
    let mut s = Session::new(1);
    s.set_snapping(DocumentSnapping {
        enabled: true,
        ..DocumentSnapping::DEFAULT
    });
    s.set_tool(Tool::Pen);
    s.pointer_down(Point::new(53.0, 75.0), false);
    s.pointer_up(Point::new(53.0, 75.0), false, false);
    let nodes = s.pen.in_progress_nodes().unwrap();
    assert_eq!(nodes[0].point, Point::new(52.5, 74.25));
    assert_eq!(nodes[0].handle_out, Vec2::ZERO);
    s.pointer_hover(Point::new(104.0, 149.0), false, false);
    assert_eq!(s.pen_rubber_band_end(), Some(Point::new(105.0, 148.5)));
    s.pointer_down(Point::new(104.0, 149.0), false);
    s.pointer_up(Point::new(104.0, 149.0), false, false);
    assert_eq!(
        s.pen.in_progress_nodes().unwrap()[1].point,
        Point::new(105.0, 148.5)
    );
}

#[test]
fn a_node_anchor_preview_and_release_match_the_target() {
    use curvyo_document_core::{AnchorId, NewAnchor};
    let mut s = Session::new(1);
    let id = s.document.create_path(
        &[
            NewAnchor::corner(AnchorId::new(1, 1), Point::new(10.0, 10.0)),
            NewAnchor::corner(AnchorId::new(1, 2), Point::new(40.0, 10.0)),
        ],
        false,
    );
    let _ = s.document.create_path(
        &[
            NewAnchor::corner(AnchorId::new(1, 3), Point::new(100.0, 100.0)),
            NewAnchor::corner(AnchorId::new(1, 4), Point::new(130.0, 100.0)),
        ],
        false,
    );
    s.set_snapping(DocumentSnapping {
        enabled: true,
        objects: false,
        guides: false,
        page: false,
        ..DocumentSnapping::DEFAULT
    });
    s.set_tool(Tool::Node);
    s.pointer_down(Point::new(10.0, 10.0), false);
    s.pointer_hover(Point::new(99.0, 99.0), false, false);
    let paths = s.live_node_drag_paths_in(&s.objects());
    assert_eq!(
        paths.iter().find(|p| p.id == id).unwrap().anchors[0].point,
        Point::new(100.0, 100.0)
    );
    s.pointer_up(Point::new(99.0, 99.0), false, false);
    assert_eq!(
        s.document.path(id).unwrap().anchors[0].point,
        Point::new(100.0, 100.0)
    );
}

#[test]
fn resize_uses_the_moving_corner_for_preview_and_commit() {
    let mut s = Session::new(1);
    let id = s.document.create_rect(RectBounds::from_corners(
        Point::new(10.0, 10.0),
        Point::new(30.0, 30.0),
    ));
    let _ = s.document.create_rect(RectBounds::from_corners(
        Point::new(100.0, 100.0),
        Point::new(120.0, 120.0),
    ));
    s.set_snapping(DocumentSnapping {
        enabled: true,
        nodes: false,
        guides: false,
        page: false,
        ..DocumentSnapping::DEFAULT
    });
    s.set_tool(Tool::Select);
    s.selection.set(&[id]);
    s.pointer_down(Point::new(30.0, 30.0), false);
    s.pointer_hover(Point::new(99.0, 99.0), false, false);
    let live = s.select_live_edit_in(&s.objects()).unwrap();
    assert_eq!(
        curvyo_ui_core::object_bounds(&live.objects[0]),
        (Point::new(10.0, 10.0), Point::new(100.0, 100.0))
    );
    s.pointer_up(Point::new(99.0, 99.0), false, false);
    assert_eq!(
        curvyo_ui_core::object_bounds(&s.document.object(id).unwrap()),
        (Point::new(10.0, 10.0), Point::new(100.0, 100.0))
    );
}

#[test]
fn changed_geometry_and_target_flags_refresh_during_a_drag() {
    let mut s = Session::new(1);
    let id = s.document.create_rect(RectBounds::from_corners(
        Point::new(10.0, 10.0),
        Point::new(30.0, 30.0),
    ));
    let target = s.document.create_rect(RectBounds::from_corners(
        Point::new(100.0, 10.0),
        Point::new(120.0, 30.0),
    ));
    s.set_snapping(DocumentSnapping {
        enabled: true,
        objects: false,
        nodes: false,
        guides: false,
        page: false,
        ..DocumentSnapping::DEFAULT
    });
    s.set_tool(Tool::Select);
    s.selection.set(&[id]);
    s.pointer_down(Point::new(20.0, 20.0), false);
    s.pointer_hover(Point::new(89.0, 20.0), false, false);
    assert!(s.snap_result().unwrap().is_empty());
    // Direct document writes model a peer merge, which is not a gated local panel command.
    s.document.set_snapping(DocumentSnapping {
        objects: true,
        ..s.document.snapping()
    });
    s.document
        .translate_objects(&[target], Vec2::new(20.0, 0.0))
        .unwrap();
    s.pointer_hover(Point::new(109.0, 20.0), false, false);
    assert_eq!(s.snap_result().unwrap().offset.x, 1.0);
    s.modifiers_changed(false, true, true);
    assert!(s.snap_result().unwrap().is_empty());
    s.escape();
    assert!(s.snap_result().is_none());
}

#[test]
fn node_click_does_not_move_but_handle_drag_uses_nodes_only() {
    use curvyo_document_core::{AnchorId, AnchorKind, NewAnchor};
    let mut s = Session::new(1);
    let id = s.document.create_path(
        &[
            NewAnchor {
                id: AnchorId::new(1, 1),
                point: Point::new(10.0, 10.0),
                handle_in: Vec2::new(-5.0, 0.0),
                handle_out: Vec2::new(5.0, 0.0),
                kind: AnchorKind::Corner,
            },
            NewAnchor::corner(AnchorId::new(1, 2), Point::new(40.0, 10.0)),
        ],
        false,
    );
    let _ = s.document.create_path(
        &[
            NewAnchor::corner(AnchorId::new(1, 3), Point::new(100.0, 100.0)),
            NewAnchor::corner(AnchorId::new(1, 4), Point::new(130.0, 100.0)),
        ],
        false,
    );
    let _ = s.document.create_rect(RectBounds::from_corners(
        Point::new(99.0, 99.0),
        Point::new(120.0, 120.0),
    ));
    s.set_snapping(DocumentSnapping {
        enabled: true,
        page: false,
        ..DocumentSnapping::DEFAULT
    });
    s.set_tool(Tool::Node);
    s.pointer_down(Point::new(10.0, 10.0), false);
    s.pointer_up(Point::new(10.0, 10.0), false, false);
    assert_eq!(
        s.document.path(id).unwrap().anchors[0].point,
        Point::new(10.0, 10.0)
    );
    s.pointer_down(Point::new(15.0, 10.0), false);
    s.pointer_hover(Point::new(99.0, 99.0), false, false);
    s.pointer_up(Point::new(99.0, 99.0), false, false);
    assert_eq!(
        s.document.path(id).unwrap().anchors[0].handle_out,
        Vec2::new(90.0, 90.0)
    );
}

#[test]
fn mirrored_aspect_resize_preserves_valid_diagonal_and_preview() {
    let mut s = Session::new(1);
    let id = s.document.create_rect(RectBounds::from_corners(
        Point::new(50.0, 50.0),
        Point::new(70.0, 70.0),
    ));
    let _ = s.document.create_rect(RectBounds::from_corners(
        Point::new(18.0, 82.0),
        Point::new(28.0, 92.0),
    ));
    s.set_snapping(DocumentSnapping {
        enabled: true,
        page: false,
        nodes: false,
        guides: false,
        ..DocumentSnapping::DEFAULT
    });
    s.set_tool(Tool::Select);
    s.selection.set(&[id]);
    s.pointer_down(Point::new(70.0, 70.0), false);
    s.pointer_hover(Point::new(19.0, 81.0), false, true);
    let live = s.select_live_edit_in(&s.objects()).unwrap();
    let bounds = curvyo_ui_core::object_bounds(&live.objects[0]);
    assert_eq!(bounds, (Point::new(18.0, 50.0), Point::new(50.0, 82.0)));
    s.pointer_up(Point::new(19.0, 81.0), false, true);
    assert_eq!(
        curvyo_ui_core::object_bounds(&s.document.object(id).unwrap()),
        bounds
    );
}

#[test]
fn rotated_mirrored_aspect_resize_reaches_a_node_on_its_allowed_ray() {
    use curvyo_document_core::{AnchorId, Angle, NewAnchor};
    let mut s = Session::new(1);
    let center = Point::new(60.0, 60.0);
    let angle = Angle::from_radians(30.0_f64.to_radians());
    let turn = |p: Point| p.rotated_around(center, angle);
    let id = s.document.create_rect(RectBounds::from_corners(
        Point::new(50.0, 50.0),
        Point::new(70.0, 70.0),
    ));
    let rotated = s.document.object(id).unwrap().rotated(center, angle);
    s.document.rotate_object(&rotated).unwrap();
    let target = turn(Point::new(18.0, 82.0));
    let _ = s.document.create_path(
        &[
            NewAnchor::corner(AnchorId::new(1, 90), target),
            NewAnchor::corner(AnchorId::new(1, 91), Point::new(300.0, 300.0)),
        ],
        false,
    );
    s.set_snapping(DocumentSnapping {
        enabled: true,
        page: false,
        objects: false,
        guides: false,
        ..DocumentSnapping::DEFAULT
    });
    s.set_tool(Tool::Select);
    s.selection.set(&[id]);
    s.pointer_down(turn(Point::new(70.0, 70.0)), false);
    let raw = turn(Point::new(19.0, 81.0));
    s.pointer_hover(raw, false, true);
    let live = s.select_live_edit_in(&s.objects()).unwrap();
    let bounds = curvyo_ui_core::object_outline_bounds(&live.objects[0]);
    assert!(!s.snap_result().unwrap().is_empty());
    let expected = [
        Point::new(18.0, 50.0),
        Point::new(50.0, 50.0),
        Point::new(18.0, 82.0),
        Point::new(50.0, 82.0),
    ]
    .map(turn);
    assert!(
        (bounds.0.x - expected.iter().map(|p| p.x).fold(f64::INFINITY, f64::min)).abs() < 1e-8,
        "bounds {bounds:?}, expected {expected:?}, snap {:?}, live {:?}",
        s.snap_result(),
        live.objects[0]
    );
    assert!(
        (bounds.1.y
            - expected
                .iter()
                .map(|p| p.y)
                .fold(f64::NEG_INFINITY, f64::max))
        .abs()
            < 1e-8
    );
    s.pointer_up(raw, false, true);
    assert_eq!(
        curvyo_ui_core::object_outline_bounds(&s.document.object(id).unwrap()),
        bounds
    );
}
