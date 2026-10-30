use super::*;
use crate::session::{Session, Tool};
use curvyo_document_core::Vec2;
use curvyo_ui_core::PolyStarMode;

fn drawing_cases() -> [(Tool, PolyStarMode); 5] {
    [
        (Tool::Rectangle, PolyStarMode::Polygon),
        (Tool::Ellipse, PolyStarMode::Polygon),
        (Tool::PolygonStar, PolyStarMode::Polygon),
        (Tool::PolygonStar, PolyStarMode::Star),
        (Tool::Pen, PolyStarMode::Polygon),
    ]
}
fn session(tool: Tool, mode: PolyStarMode, enabled: bool) -> Session {
    let mut s = Session::new(30);
    s.set_snapping(DocumentSnapping {
        enabled,
        ..DocumentSnapping::DEFAULT
    });
    s.set_tool(tool);
    s.set_poly_star_mode(mode);
    s
}
#[test]
fn every_drawing_hover_inverts_on_alt_without_pointer_motion_or_writes() {
    for (tool, mode) in drawing_cases() {
        for enabled in [false, true] {
            let mut s = session(tool, mode, enabled);
            s.pointer_hover(Point::new(53.0, 75.0), false, false);
            let version = s.document.version();
            for (ctrl, alt) in [
                (false, true),
                (true, true),
                (false, false),
                (false, true),
                (false, false),
            ] {
                s.modifiers_changed(false, ctrl, alt);
                assert_eq!(
                    !s.snap_result().unwrap().is_empty(),
                    enabled != alt,
                    "{tool:?}/{mode:?} enabled{enabled} alt{alt}"
                );
                assert_eq!(!s.snap_marker_list().triangles.is_empty(), enabled != alt);
                assert_eq!(s.document.version(), version);
                assert_eq!(s.snapping().enabled, enabled);
            }
        }
    }
}
#[test]
fn alt_before_first_press_and_click_uses_effective_setting_without_prior_hover() {
    for (tool, mode) in drawing_cases() {
        for enabled in [false, true] {
            for ctrl in [false, true] {
                let mut s = session(tool, mode, enabled);
                s.modifiers_changed(false, ctrl, true);
                let raw = Point::new(53.0, 75.0);
                s.pointer_down(raw, false);
                s.pointer_up(raw, false, ctrl);
                let expected = if enabled {
                    raw
                } else {
                    Point::new(52.5, 74.25)
                };
                if tool == Tool::Pen {
                    let node = &s.pen.in_progress_nodes().unwrap()[0];
                    assert_eq!(node.point, expected);
                    assert_eq!(node.handle_out, Vec2::ZERO);
                } else {
                    assert_eq!(s.placement().unwrap().a(), expected);
                }
                assert_eq!(s.snapping().enabled, enabled);
            }
        }
    }
}
#[test]
fn alt_recomputes_moving_drawing_endpoint_but_keeps_the_first_anchor() {
    for (tool, mode) in drawing_cases() {
        for enabled in [false, true] {
            for placement in [false, true] {
                if tool == Tool::Pen && !placement {
                    continue;
                }
                let mut s = session(tool, mode, enabled);
                let start = Point::new(53.0, 75.0);
                s.pointer_down(start, false);
                if placement {
                    s.pointer_up(start, false, false);
                }
                s.pointer_hover(Point::new(104.0, 126.0), false, false);
                let version = s.document.version();
                let anchor = if tool == Tool::Pen {
                    s.pen_last_node().unwrap()
                } else if placement {
                    s.placement().unwrap().a()
                } else {
                    s.rectangle
                        .drag_start()
                        .or(s.ellipse.drag_start())
                        .or(s.poly_star.drag_start())
                        .unwrap()
                };
                for (ctrl, alt) in [(false, true), (true, true), (true, false), (false, false)] {
                    s.modifiers_changed(false, ctrl, alt);
                    assert_eq!(
                        !s.snap_result().unwrap().is_empty(),
                        enabled != alt,
                        "{tool:?}/{mode:?} placement{placement} enabled{enabled} alt{alt}"
                    );
                    if tool == Tool::Pen {
                        assert_eq!(s.pen_last_node(), Some(anchor));
                    } else if placement {
                        assert_eq!(s.placement().unwrap().a(), anchor);
                    } else {
                        assert_eq!(
                            s.rectangle
                                .drag_start()
                                .or(s.ellipse.drag_start())
                                .or(s.poly_star.drag_start()),
                            Some(anchor)
                        );
                    }
                    assert_eq!(s.document.version(), version);
                }
            }
        }
    }
}

#[test]
fn toggling_alt_after_press_does_not_relocate_click_anchor_or_pen_node() {
    for (tool, mode) in drawing_cases() {
        for enabled in [false, true] {
            let mut s = session(tool, mode, enabled);
            let raw = Point::new(53.0, 75.0);
            let pinned = if enabled {
                Point::new(52.5, 74.25)
            } else {
                raw
            };
            s.pointer_down(raw, false);
            let version = s.document.version();
            s.modifiers_changed(false, false, true);
            assert_eq!(s.document.version(), version);
            s.pointer_up(raw, false, false);
            if tool == Tool::Pen {
                let node = &s.pen.in_progress_nodes().unwrap()[0];
                assert_eq!(node.point, pinned);
                assert_eq!(node.handle_out, Vec2::ZERO);
            } else {
                assert_eq!(s.placement().unwrap().a(), pinned);
            }
        }
    }
}
#[test]
fn pen_ctrl_constraint_and_frozen_typed_geometry_survive_alt() {
    let mut s = session(Tool::Pen, PolyStarMode::Polygon, false);
    let start = Point::new(10.0, 10.0);
    s.pointer_down(start, false);
    s.pointer_up(start, false, false);
    let raw = Point::new(100.0, 47.0);
    s.pointer_hover(raw, false, true);
    let constrained = curvyo_ui_core::constrain_segment(start, raw);
    // No target kind can pull; Alt must not disable Ctrl's existing geometric constraint.
    s.document.set_snapping(DocumentSnapping {
        objects: false,
        guides: false,
        nodes: false,
        page: false,
        ..s.snapping()
    });
    for alt in [true, false] {
        s.modifiers_changed(false, true, alt);
        assert_eq!(s.pen_rubber_band_end(), Some(constrained));
    }
    let mut s = session(Tool::Rectangle, PolyStarMode::Polygon, false);
    s.pointer_down(start, false);
    s.pointer_up(start, false, false);
    s.pointer_hover(Point::new(53.0, 75.0), false, false);
    s.placement_edit(0, "20");
    s.placement_edit(1, "10");
    let frozen = s.placement().unwrap().pointer();
    let version = s.document.version();
    for alt in [true, false] {
        s.modifiers_changed(false, false, alt);
        assert_eq!(s.placement().unwrap().pointer(), frozen);
        assert_eq!(s.document.version(), version);
        assert!(s.snap_result().is_none());
    }
    assert!(s.place_placement());
    let object = s.document.object(s.document.object_ids()[0]).unwrap();
    assert_eq!(
        curvyo_ui_core::object_bounds(&object),
        (start, Point::new(30.0, 20.0))
    );
}
