//! Independent snapping acceptance through public session gestures.
#![allow(clippy::unwrap_used)]
use curvyo_document_core::{
    AnchorId, Document, DocumentSize, DocumentSnapping, NewAnchor, Point, RectBounds, StyleEdit,
    Vec2, pack, unpack,
};
use curvyo_editor_wasm::{KeyInput, Session, Tool};
use curvyo_ui_core::{PolyStarMode, object_outline_bounds};
fn opened(s: &Session) -> Document {
    unpack(609, &s.pack("acceptance").unwrap()).unwrap()
}
fn only_bounds(s: &Session) -> (Point, Point) {
    let d = opened(s);
    assert_eq!(d.object_ids().len(), 1);
    object_outline_bounds(&d.object(d.object_ids()[0]).unwrap())
}
fn config() -> DocumentSnapping {
    DocumentSnapping {
        enabled: true,
        objects: false,
        guides: false,
        nodes: false,
        page: true,
        ..DocumentSnapping::DEFAULT
    }
}
fn drag(s: &mut Session, start: Point, end: Point, shift: bool, ctrl: bool) {
    s.pointer_down(start, shift);
    s.pointer_hover(end, shift, ctrl);
    s.pointer_up(end, shift, ctrl);
}
fn close(a: (Point, Point), b: (Point, Point)) {
    for (x, y) in [
        (a.0.x, b.0.x),
        (a.0.y, b.0.y),
        (a.1.x, b.1.x),
        (a.1.y, b.1.y),
    ] {
        assert!((x - y).abs() < 1e-8, "{a:?} != {b:?}");
    }
}

#[test]
fn ac7_creation_tools_use_exact_page_targets_and_suppression_preserves_raw_tool_behavior() {
    for (tool, mode) in [
        (Tool::Rectangle, PolyStarMode::Polygon),
        (Tool::Ellipse, PolyStarMode::Polygon),
        (Tool::PolygonStar, PolyStarMode::Polygon),
        (Tool::PolygonStar, PolyStarMode::Star),
    ] {
        let mut snapped = Session::new(610);
        snapped.set_tool(tool);
        snapped.set_poly_star_mode(mode);
        snapped.set_snapping(config());
        drag(
            &mut snapped,
            Point::new(53.0, 75.0),
            Point::new(104.0, 149.0),
            false,
            false,
        );
        let mut exact = Session::new(611);
        exact.set_tool(tool);
        exact.set_poly_star_mode(mode);
        drag(
            &mut exact,
            Point::new(52.5, 74.25),
            Point::new(105.0, 148.5),
            false,
            false,
        );
        close(only_bounds(&snapped), only_bounds(&exact));
        for shift in [false, true] {
            let mut suppressed = Session::new(612);
            suppressed.set_tool(tool);
            suppressed.set_poly_star_mode(mode);
            suppressed.set_snapping(config());
            suppressed.modifiers_changed(shift, true, true);
            drag(
                &mut suppressed,
                Point::new(53.0, 75.0),
                Point::new(104.0, 149.0),
                shift,
                true,
            );
            let mut raw = Session::new(613);
            raw.set_tool(tool);
            raw.set_poly_star_mode(mode);
            raw.modifiers_changed(shift, true, true);
            drag(
                &mut raw,
                Point::new(53.0, 75.0),
                Point::new(104.0, 149.0),
                shift,
                true,
            );
            close(only_bounds(&suppressed), only_bounds(&raw));
            assert!(suppressed.snapping().enabled);
        }
    }
}

#[test]
fn ac7_ac9_ac12_shift_move_snaps_only_free_axis_commits_once_and_cancels_without_writes() {
    let d = Document::new(614);
    let moving = d.create_rect(RectBounds::from_corners(
        Point::new(10.0, 10.0),
        Point::new(30.0, 30.0),
    ));
    d.edit_style(&[moving], &StyleEdit::FillEnabled(true))
        .unwrap();
    let _ = d.create_rect(RectBounds::from_corners(
        Point::new(100.0, 10.0),
        Point::new(120.0, 30.0),
    ));
    d.set_snapping(DocumentSnapping {
        enabled: true,
        page: false,
        nodes: false,
        guides: false,
        ..DocumentSnapping::DEFAULT
    });
    let mut s = Session::open(615, &pack(&d, "acceptance").unwrap()).unwrap();
    s.set_tool(Tool::Select);
    // Fill makes the interior a stable move hit target, away from handles.
    drag(
        &mut s,
        Point::new(10.0, 20.0),
        Point::new(10.0, 20.0),
        false,
        false,
    );
    assert_eq!(s.selected_object_count(), 1);
    let before = opened(&s);
    let steps = s.undo_len();
    s.pointer_down(Point::new(20.0, 20.0), false);
    s.pointer_hover(Point::new(89.0, 24.0), true, false);
    s.escape();
    s.pointer_up(Point::new(89.0, 24.0), false, false);
    assert_eq!(
        opened(&s).read_objects().objects,
        before.read_objects().objects
    );
    assert_eq!(s.undo_len(), steps);
    // Press first, then hold Shift to lock the gesture; Shift+press toggles selection.
    s.pointer_down(Point::new(20.0, 20.0), false);
    s.pointer_hover(Point::new(89.0, 24.0), true, false);
    s.pointer_up(Point::new(89.0, 24.0), true, false);
    close(
        object_outline_bounds(&opened(&s).object(moving).unwrap()),
        (Point::new(80.0, 10.0), Point::new(100.0, 30.0)),
    );
    assert_eq!(s.undo_len(), steps + 1);
    s.undo();
    assert_eq!(
        opened(&s).read_objects().objects,
        before.read_objects().objects
    );
    s.redo();
    close(
        object_outline_bounds(&opened(&s).object(moving).unwrap()),
        (Point::new(80.0, 10.0), Point::new(100.0, 30.0)),
    );
}

#[test]
fn ac7_pen_first_and_subsequent_clicks_snap_without_accidental_handles() {
    let mut s = Session::new(616);
    s.set_tool(Tool::Pen);
    s.set_snapping(config());
    for p in [Point::new(53.0, 75.0), Point::new(104.0, 149.0)] {
        drag(&mut s, p, p, false, false);
    }
    s.key_down(KeyInput {
        key: "Enter",
        ..KeyInput::default()
    });
    let d = opened(&s);
    assert_eq!(d.object_ids().len(), 1);
    let path = d.path(d.object_ids()[0]).unwrap();
    assert_eq!(path.anchors.len(), 2);
    assert_eq!(path.anchors[0].point, Point::new(52.5, 74.25));
    assert_eq!(path.anchors[1].point, Point::new(105.0, 148.5));
    for anchor in path.anchors {
        assert_eq!(anchor.handle_in, Vec2::ZERO);
        assert_eq!(anchor.handle_out, Vec2::ZERO);
    }
}

#[test]
fn ac7_node_anchor_drag_snaps_to_another_path_and_is_one_undo_step() {
    let d = Document::new(617);
    let source = d.create_path(
        &[
            NewAnchor::corner(AnchorId::new(617, 1), Point::new(10.0, 10.0)),
            NewAnchor::corner(AnchorId::new(617, 2), Point::new(40.0, 10.0)),
        ],
        false,
    );
    let _ = d.create_path(
        &[
            NewAnchor::corner(AnchorId::new(617, 3), Point::new(100.0, 100.0)),
            NewAnchor::corner(AnchorId::new(617, 4), Point::new(130.0, 100.0)),
        ],
        false,
    );
    d.set_snapping(DocumentSnapping {
        enabled: true,
        objects: false,
        guides: false,
        page: false,
        ..DocumentSnapping::DEFAULT
    });
    let mut s = Session::open(618, &pack(&d, "acceptance").unwrap()).unwrap();
    s.set_tool(Tool::Node);
    let steps = s.undo_len();
    drag(
        &mut s,
        Point::new(10.0, 10.0),
        Point::new(99.0, 99.0),
        false,
        false,
    );
    assert_eq!(
        opened(&s).path(source).unwrap().anchors[0].point,
        Point::new(100.0, 100.0)
    );
    assert_eq!(s.undo_len(), steps + 1);
    s.undo();
    assert_eq!(opened(&s).path(source).unwrap(), d.path(source).unwrap());
}

#[test]
fn ac15_inactive_page_targets_follow_its_actual_origin_and_new_size() {
    let d = Document::new(619);
    let p = d.add_page();
    d.resize_page(&p.id, DocumentSize::from_mm(300.0, 120.0), &[])
        .unwrap();
    d.move_page(&p.id, Vec2::new(1000.0 - p.origin.x, 400.0), &[])
        .unwrap();
    d.set_snapping(config());
    let mut s = Session::open(620, &pack(&d, "acceptance").unwrap()).unwrap();
    assert_eq!(s.active_page().id, "first");
    s.set_tool(Tool::Rectangle);
    drag(
        &mut s,
        Point::new(1074.0, 431.0),
        Point::new(1149.0, 459.0),
        false,
        false,
    );
    close(
        only_bounds(&s),
        (Point::new(1075.0, 430.0), Point::new(1150.0, 460.0)),
    );
}

mod cpu_time;
#[test]
fn ac11_ten_thousand_mixed_guides_and_box_edges_are_exact_under_two_ms_per_frame() {
    use curvyo_ui_core::{
        SnapCurve, SnapProbe, SnapSettings, SnapState, SnapTarget, SnapTargets, TargetKind, resolve,
    };
    use std::hint::black_box;
    let mut entries = Vec::new();
    for n in 0..10_000 {
        let x = f64::from(n % 100) * 12.0;
        let y = f64::from(n / 100) * 12.0;
        let curve = SnapCurve {
            start: Point::new(x, y),
            end: Point::new(x + 8.0, y),
            handle_out: Vec2::ZERO,
            handle_in: Vec2::ZERO,
        };
        let mut guide = SnapTarget::point(curve.start, (curve.start, curve.end), TargetKind::Guide);
        guide.curve = Some(curve);
        entries.push(guide);
        entries.push(SnapTarget::vertical(
            x + 4.0,
            (Point::new(x + 4.0, y - 3.0), Point::new(x + 4.0, y + 3.0)),
            TargetKind::Box,
        ));
    }
    let targets = SnapTargets::new(entries);
    let settings = SnapSettings::from_document(config(), 1.0);
    let settings = SnapSettings {
        objects: true,
        guides: true,
        page: false,
        ..settings
    };
    let ((), baseline) = cpu_time::cpu_time_of(|| {
        for frame in 0..1000 {
            let n = (frame * 97) % 10_000;
            black_box(SnapProbe::point(Point::new(
                f64::from(n % 100) * 12.0 + 3.0,
                f64::from(n / 100) * 12.0 + 1.0,
            )));
        }
    });
    let ((), snapped) = cpu_time::cpu_time_of(|| {
        for frame in 0..1000 {
            let n = (frame * 97) % 10_000;
            let x = f64::from(n % 100) * 12.0;
            let y = f64::from(n / 100) * 12.0;
            let raw = Point::new(x + 3.0, y + 1.0);
            let result = resolve(
                &settings,
                &targets,
                &SnapProbe::point(raw),
                &mut SnapState::default(),
            );
            let actual = raw.translated(result.offset);
            assert!((actual.x - (x + 4.0)).abs() < 1e-9 && (actual.y - y).abs() < 1e-9);
            assert_eq!(result.marker.lines.len(), 2);
            black_box(result);
        }
    });
    let incremental = snapped.saturating_sub(baseline).as_secs_f64();
    eprintln!(
        "10k mixed finite guides + 10k box edges: {incremental:.4}ms/frame incremental thread CPU (1000 fresh captures)"
    );
    assert!(
        incremental <= 2.0,
        "incremental {incremental}ms/frame exceeds 2ms"
    );
}

#[test]
fn ac13_dragged_clone_snaps_to_stationary_source_and_sibling_without_losing_overrides() {
    use curvyo_document_core::{Color, ObjectSnapshot};
    for to_sibling in [false, true] {
        let d = Document::new(621);
        let source = d.create_path(
            &[(100.0, 10.0), (120.0, 10.0), (120.0, 30.0), (100.0, 30.0)]
                .into_iter()
                .enumerate()
                .map(|(n, (x, y))| {
                    NewAnchor::corner(AnchorId::new(621, n as u64 + 1), Point::new(x, y))
                })
                .collect::<Vec<_>>(),
            true,
        );
        d.edit_style(&[source], &StyleEdit::FillEnabled(true))
            .unwrap();
        let moving = d.linked_clone(&[source], &[]).unwrap()[0];
        let sibling = d.linked_clone(&[source], &[]).unwrap()[0];
        d.translate_objects(&[moving], Vec2::new(-90.0, 0.0))
            .unwrap();
        d.translate_objects(&[sibling], Vec2::new(0.0, 90.0))
            .unwrap();
        d.set_own_placement(&[moving], true).unwrap();
        d.set_own_style(&[moving], true).unwrap();
        d.edit_style(
            &[moving],
            &StyleEdit::FillColor(Color { r: 255, g: 0, b: 0 }),
        )
        .unwrap();
        d.set_snapping(DocumentSnapping {
            enabled: true,
            nodes: false,
            guides: false,
            page: false,
            ..DocumentSnapping::DEFAULT
        });
        let mut s = Session::open(622, &pack(&d, "acceptance").unwrap()).unwrap();
        s.set_tool(Tool::Select);
        drag(
            &mut s,
            Point::new(20.0, 20.0),
            Point::new(20.0, 20.0),
            false,
            false,
        );
        assert_eq!(s.selected_object_count(), 1);
        let before = opened(&s);
        let steps = s.undo_len();
        let y = if to_sibling { 110.0 } else { 20.0 };
        drag(
            &mut s,
            Point::new(20.0, 20.0),
            Point::new(89.0, y),
            false,
            false,
        );
        let after = opened(&s);
        close(
            object_outline_bounds(&after.object(moving).unwrap()),
            (Point::new(80.0, y - 10.0), Point::new(100.0, y + 10.0)),
        );
        for id in [source, sibling] {
            assert_eq!(after.object(id), before.object(id));
        }
        let ObjectSnapshot::Clone(c) = after.object(moving).unwrap() else {
            panic!("link lost")
        };
        assert_eq!(c.chain.origin, source);
        assert!(c.chain.own_placement && c.chain.own_style);
        let ObjectSnapshot::Clone(old) = before.object(moving).unwrap() else {
            panic!("fixture")
        };
        assert_eq!(c.resolved.style, old.resolved.style);
        assert_eq!(s.undo_len(), steps + 1);
        s.undo();
        assert_eq!(
            opened(&s).read_objects().objects,
            before.read_objects().objects
        );
        s.redo();
        assert_eq!(
            opened(&s).read_objects().objects,
            after.read_objects().objects
        );
    }
}

fn reference_vertices(s: &Session) -> Vec<Point> {
    s.draw_list()
        .triangles
        .into_iter()
        .filter(|v| v.color == curvyo_render_core::RgbaColor::opaque(229, 72, 77))
        .map(|v| v.position)
        .collect()
}

#[test]
fn ac9_alt_recomputes_at_same_pointer_without_writes_and_ctrl_alt_never_copies() {
    for enabled in [false, true] {
        for ctrl in [false, true] {
            let d = Document::new(640);
            let moving = d.create_rect(RectBounds::from_corners(
                Point::new(10.0, 10.0),
                Point::new(30.0, 30.0),
            ));
            d.edit_style(&[moving], &StyleEdit::FillEnabled(true))
                .unwrap();
            let _ = d.create_rect(RectBounds::from_corners(
                Point::new(100.0, 10.0),
                Point::new(120.0, 30.0),
            ));
            d.set_snapping(DocumentSnapping {
                enabled,
                page: false,
                nodes: false,
                guides: false,
                ..DocumentSnapping::DEFAULT
            });
            let mut s = Session::open(641, &pack(&d, "acceptance").unwrap()).unwrap();
            s.set_tool(Tool::Select);
            drag(
                &mut s,
                Point::new(20.0, 20.0),
                Point::new(20.0, 20.0),
                false,
                false,
            );
            let before = opened(&s);
            let steps = s.undo_len();
            s.pointer_down(Point::new(20.0, 20.0), false);
            s.pointer_hover(Point::new(89.0, 20.0), false, false);
            let baseline = reference_vertices(&s);
            s.modifiers_changed(false, ctrl, true);
            let inverted = reference_vertices(&s);
            assert!(!s.move_indicators().copy_badge);
            assert!(!s.live_readout().unwrap().text.contains("Copy"));
            assert_ne!(baseline, inverted, "stored {enabled},ctrl {ctrl}");
            s.modifiers_changed(false, true, false);
            assert!(s.move_indicators().copy_badge);
            assert!(s.live_readout().unwrap().text.contains("Copy"));
            s.modifiers_changed(false, false, false);
            assert_eq!(reference_vertices(&s), baseline);
            assert_eq!(s.undo_len(), steps);
            assert_eq!(
                opened(&s).read_objects().objects,
                before.read_objects().objects
            );
            assert_eq!(s.snapping(), before.snapping());
            s.modifiers_changed(false, ctrl, true);
            assert_eq!(reference_vertices(&s), inverted);
            s.pointer_up(Point::new(89.0, 20.0), false, ctrl);
            let after = opened(&s);
            assert_eq!(after.object_ids().len(), 2);
            let left = if enabled { 79.0 } else { 80.0 };
            close(
                object_outline_bounds(&after.object(moving).unwrap()),
                (Point::new(left, 10.0), Point::new(left + 20.0, 30.0)),
            );
            assert_eq!(s.undo_len(), steps + 1);
            s.undo();
            assert_eq!(
                opened(&s).read_objects().objects,
                before.read_objects().objects
            );
        }
    }
}

#[test]
fn ac17_idle_hover_markers_have_no_drag_history_and_clear_on_lifecycle_events() {
    for (tool, mode) in [
        (Tool::Rectangle, PolyStarMode::Polygon),
        (Tool::Ellipse, PolyStarMode::Polygon),
        (Tool::PolygonStar, PolyStarMode::Polygon),
        (Tool::PolygonStar, PolyStarMode::Star),
        (Tool::Pen, PolyStarMode::Polygon),
    ] {
        let mut s = Session::new(642);
        s.set_tool(tool);
        s.set_poly_star_mode(mode);
        s.set_snapping(config());
        let before = opened(&s);
        let steps = s.undo_len();
        let empty = reference_vertices(&s);
        s.pointer_hover(Point::new(53.0, 75.0), false, false);
        assert!(!s.snapping_toolbar().has_drag);
        assert_ne!(reference_vertices(&s), empty, "{tool:?}");
        assert_eq!(s.undo_len(), steps);
        assert_eq!(
            opened(&s).read_objects().objects,
            before.read_objects().objects
        );
        s.pointer_leave();
        assert_eq!(reference_vertices(&s), empty);
        s.pointer_hover(Point::new(53.0, 75.0), false, false);
        s.escape();
        assert_eq!(reference_vertices(&s), empty);
        s.pointer_hover(Point::new(53.0, 75.0), false, false);
        s.set_snapping(DocumentSnapping {
            x: false,
            y: false,
            ..s.snapping()
        });
        assert_eq!(reference_vertices(&s), empty);
        s.set_snapping(config());
        s.pointer_hover(Point::new(53.0, 75.0), false, false);
        s.set_snapping(DocumentSnapping {
            enabled: false,
            ..s.snapping()
        });
        assert_eq!(reference_vertices(&s), empty);
        s.modifiers_changed(false, false, true);
        s.pointer_hover(Point::new(53.0, 75.0), false, false);
        assert_eq!(
            reference_vertices(&s),
            empty,
            "Alt does not invert idle hover"
        );
        s.modifiers_changed(false, false, false);
        s.set_snapping(config());
        s.pointer_hover(Point::new(53.0, 75.0), false, false);
        s.set_tool(Tool::Select);
        assert_eq!(reference_vertices(&s), empty);
        let reopened = Session::open(643, &s.pack("acceptance").unwrap()).unwrap();
        assert_eq!(reference_vertices(&reopened).len(), 0);
    }
}

#[test]
fn ac16_ac18_all_shape_click_placements_obey_axes_and_preserve_typed_geometry() {
    for (tool, mode) in [
        (Tool::Rectangle, PolyStarMode::Polygon),
        (Tool::Ellipse, PolyStarMode::Polygon),
        (Tool::PolygonStar, PolyStarMode::Polygon),
        (Tool::PolygonStar, PolyStarMode::Star),
    ] {
        for (x, y) in [(true, true), (true, false), (false, true), (false, false)] {
            for hover in [false, true] {
                let mut s = Session::new(644);
                s.set_tool(tool);
                s.set_poly_star_mode(mode);
                s.set_snapping(DocumentSnapping { x, y, ..config() });
                let mut exact = Session::new(645);
                exact.set_tool(tool);
                exact.set_poly_star_mode(mode);
                let raw = Point::new(53.0, 75.0);
                let snapped = Point::new(if x { 52.5 } else { 53.0 }, if y { 74.25 } else { 75.0 });
                if hover {
                    s.pointer_hover(Point::new(104.0, 149.0), false, false);
                }
                let steps = s.undo_len();
                drag(&mut s, raw, raw, false, false);
                drag(&mut exact, snapped, snapped, false, false);
                assert_eq!(s.placement().unwrap().a(), snapped);
                assert_eq!(s.undo_len(), steps);
                assert_eq!(opened(&s).object_ids().len(), 0);
                s.placement_edit(0, "20");
                exact.placement_edit(0, "20");
                s.placement_edit(1, "10");
                exact.placement_edit(1, "10");
                // Typed size freezes the original placement anchor under later hover.
                s.pointer_hover(Point::new(104.0, 149.0), false, false);
                assert!(s.place_placement());
                assert!(exact.place_placement());
                close(only_bounds(&s), only_bounds(&exact));
                assert_eq!(s.undo_len(), steps + 1);
                s.undo();
                assert_eq!(opened(&s).object_ids().len(), 0);
                s.redo();
                close(only_bounds(&s), only_bounds(&exact));
            }
        }
    }
}

#[test]
fn ac18_following_click_placement_and_pen_first_click_resolve_current_raw_coordinates() {
    for (x, y) in [(true, true), (true, false), (false, true), (false, false)] {
        for tool in [Tool::Rectangle, Tool::Ellipse, Tool::PolygonStar, Tool::Pen] {
            let mut s = Session::new(646);
            s.set_tool(tool);
            s.set_snapping(DocumentSnapping { x, y, ..config() });
            let start = Point::new(53.0, 75.0);
            let end = Point::new(104.0, 149.0);
            let a = Point::new(if x { 52.5 } else { 53.0 }, if y { 74.25 } else { 75.0 });
            let b = Point::new(if x { 105.0 } else { 104.0 }, if y { 148.5 } else { 149.0 });
            let steps = s.undo_len();
            // No preceding hover before first press; second press has a real hover.
            drag(&mut s, start, start, false, false);
            s.pointer_hover(end, false, false);
            if tool != Tool::Pen {
                assert_eq!(s.placement().unwrap().following().unwrap().0, b);
            }
            drag(&mut s, end, end, false, false);
            if tool == Tool::Pen {
                s.key_down(KeyInput {
                    key: "Enter",
                    ..KeyInput::default()
                });
            }
            let actual = opened(&s);
            assert_eq!(actual.object_ids().len(), 1);
            assert_eq!(s.undo_len(), steps + 1);
            if tool == Tool::Pen {
                let path = actual.path(actual.object_ids()[0]).unwrap();
                assert_eq!(path.anchors[0].point, a);
                assert_eq!(path.anchors[1].point, b);
                for node in path.anchors {
                    assert_eq!(node.handle_in, Vec2::ZERO);
                    assert_eq!(node.handle_out, Vec2::ZERO);
                }
            } else {
                let mut exact = Session::new(647);
                exact.set_tool(tool);
                drag(&mut exact, a, a, false, false);
                exact.pointer_hover(b, false, false);
                drag(&mut exact, b, b, false, false);
                close(only_bounds(&s), only_bounds(&exact));
            }
        }
    }
}

#[test]
fn ac9_alt_at_initial_press_preserves_lasso_and_remove_badge() {
    let d = Document::new(648);
    let id = d.create_rect(RectBounds::from_corners(
        Point::new(10.0, 10.0),
        Point::new(30.0, 30.0),
    ));
    d.edit_style(&[id], &StyleEdit::FillEnabled(true)).unwrap();
    for ctrl in [false, true] {
        let mut s = Session::open(649, &pack(&d, "acceptance").unwrap()).unwrap();
        s.set_tool(Tool::Select);
        let before = opened(&s);
        let steps = s.undo_len();
        s.modifiers_changed(false, ctrl, true);
        s.pointer_down(Point::new(20.0, 20.0), false);
        s.pointer_hover(Point::new(90.0, 20.0), false, ctrl);
        assert!(!s.move_indicators().copy_badge);
        assert_eq!(s.move_indicators().remove_badge, ctrl);
        s.pointer_up(Point::new(90.0, 20.0), false, ctrl);
        assert_eq!(
            opened(&s).read_objects().objects,
            before.read_objects().objects
        );
        assert_eq!(s.undo_len(), steps);
    }
}

#[test]
fn ac16_aspect_resize_cannot_change_the_disabled_coordinate() {
    let d = Document::new(650);
    let id = d.create_rect(RectBounds::from_corners(
        Point::new(10.0, 10.0),
        Point::new(30.0, 30.0),
    ));
    d.edit_style(&[id], &StyleEdit::FillEnabled(true)).unwrap();
    let _ = d.create_rect(RectBounds::from_corners(
        Point::new(100.0, 100.0),
        Point::new(120.0, 120.0),
    ));
    let resize = |enabled, x, y| {
        let mut s = Session::open(651, &pack(&d, "acceptance").unwrap()).unwrap();
        s.set_tool(Tool::Select);
        s.set_snapping(DocumentSnapping {
            enabled,
            x,
            y,
            page: false,
            nodes: false,
            guides: false,
            ..DocumentSnapping::DEFAULT
        });
        drag(
            &mut s,
            Point::new(20.0, 20.0),
            Point::new(20.0, 20.0),
            false,
            false,
        );
        s.pointer_down(Point::new(30.0, 30.0), false);
        s.pointer_hover(Point::new(99.0, 98.0), false, true);
        s.pointer_up(Point::new(99.0, 98.0), false, true);
        object_outline_bounds(&opened(&s).object(id).unwrap())
    };
    let raw = resize(false, true, true);
    for (x, y) in [(true, false), (false, true), (false, false)] {
        close(resize(true, x, y), raw);
    }
    let snapped = resize(true, true, true);
    close(snapped, (Point::new(10.0, 10.0), Point::new(100.0, 100.0)));
    assert!((raw.1.x - snapped.1.x).abs() > 0.1);
}
