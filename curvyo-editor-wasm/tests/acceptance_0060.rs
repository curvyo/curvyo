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
