//! Independent revised Session lifecycle and local-axis acceptance, without host routing claims.
#![allow(clippy::unwrap_used)]
use curvyo_document_core::{Document, Point, Vec2, pack, unpack};
use curvyo_editor_wasm::{Session, SketchTool, Workbench};
use curvyo_sketch_core::{Axis, Entity, Relation, Sketch};
fn setup() -> (Session, Point) {
    let saved_document = Document::new(901);
    let page = saved_document.pages()[0].clone();
    saved_document
        .move_page(&page.id, Vec2::new(300.0, -40.0), &[])
        .unwrap();
    let center = Point::new(
        300.0 + page.size.width.as_mm() / 2.0,
        -40.0 + page.size.height.as_mm() / 2.0,
    );
    let mut s = Session::open(902, &pack(&saved_document, "revision").unwrap()).unwrap();
    s.set_workbench(Workbench::Sketch);
    (s, center)
}
fn document(s: &Session) -> Document {
    unpack(903, &s.pack("revision").unwrap()).unwrap()
}
fn graph(s: &Session) -> Sketch {
    document(s)
        .sketch(s.active_sketch().unwrap())
        .unwrap()
        .graph
}
fn click(s: &mut Session, p: Point) {
    s.pointer_down(p, false);
    s.pointer_up(p, false, false);
}
fn at(c: Point, x: f64, y: f64) -> Point {
    Point::new(c.x + x, c.y + y)
}
fn assert_point(geometry: &Sketch, x: f64, y: f64) {
    assert!(geometry.entities.iter().any(|(id,e)| id!=&geometry.origin && matches!(e,Entity::Point {position,..} if (position.x.0-x).abs()<1e-6 && (position.y.0-y).abs()<1e-6)),"expected({x},{y}), {geometry:?}");
}
#[test]
fn r01_empty_entry_hover_render_and_cancel_write_nothing() {
    let (mut s, c) = setup();
    let before = document(&s).version();
    let count = s.undo_len();
    assert_eq!(s.active_sketch(), None);
    s.set_sketch_tool(SketchTool::Arc);
    click(&mut s, at(c, 20.0, 20.0));
    s.pointer_hover(at(c, 30.0, 35.0), false, false);
    std::hint::black_box(s.draw_list());
    std::hint::black_box(s.sketch_view_json());
    assert!(s.sketch_secondary_press());
    assert!(!s.sketch_secondary_press());
    s.set_workbench(Workbench::Vector);
    s.set_workbench(Workbench::Sketch);
    assert_eq!(document(&s).version(), before);
    assert_eq!(s.undo_len(), count);
    assert_eq!(s.active_sketch(), None);
}
#[test]
fn r02_numeric_point_and_picked_circle_use_local_center_frame() {
    let (mut s, c) = setup();
    s.set_sketch_tool(SketchTool::Point);
    s.pointer_hover(at(c, 10.0, 20.0), false, false);
    s.sketch_complete_numeric("10 mm", "20 mm").unwrap();
    assert_point(&graph(&s), 10.0, 20.0);
    assert_eq!(s.undo_len(), 1);
    s.set_sketch_tool(SketchTool::Circle);
    click(&mut s, at(c, 30.0, 40.0));
    s.sketch_complete_numeric("5 mm", "").unwrap();
    let geometry = graph(&s);
    assert!(geometry.entities.values().any(|e|matches!(e,Entity::Circle {center,radius,..} if (radius.0-5.0).abs()<1e-6 && geometry.point(center).is_ok_and(|p|(p.x.0-30.0).abs()<1e-6&&(p.y.0-40.0).abs()<1e-6))));
    assert_eq!(s.undo_len(), 2);
    let saved_document = document(&s);
    let snap = saved_document.sketch(s.active_sketch().unwrap()).unwrap();
    // Standalone points are editable analytic geometry, not exported contour anchors.
    let local = snap
        .solution
        .entities
        .values()
        .find_map(|entity| match entity {
            Entity::Point { position, .. }
                if (position.x.0 - 10.0).abs() < 1e-6 && (position.y.0 - 20.0).abs() < 1e-6 =>
            {
                Some(*position)
            }
            _ => None,
        })
        .unwrap();
    let world = snap.placement.apply_point(Point::new(local.x.0, local.y.0));
    assert!((world.x - c.x - 10.0).abs() < 1e-6);
    assert!((world.y - c.y - 20.0).abs() < 1e-6);
}
#[test]
fn r08_secondary_and_escape_cancel_each_arc_pick_phase_without_creation() {
    for picks in [1, 2] {
        for secondary in [false, true] {
            let (mut s, c) = setup();
            s.set_sketch_tool(SketchTool::Arc);
            for p in [at(c, 20.0, 20.0), at(c, 30.0, 35.0)]
                .into_iter()
                .take(picks)
            {
                click(&mut s, p);
            }
            let steps = s.undo_len();
            if secondary {
                assert!(s.sketch_secondary_press());
                assert!(!s.sketch_secondary_press());
            } else {
                s.escape();
            }
            assert_eq!(s.active_sketch(), None);
            assert_eq!(s.undo_len(), steps);
            click(&mut s, at(c, 40.0, 20.0));
            assert_eq!(
                s.active_sketch(),
                None,
                "right/escape must not supply an arc pick"
            );
            s.escape();
            assert_eq!(s.undo_len(), steps);
        }
    }
}
#[test]
fn r10_vector_settings_and_masks_do_not_change_creation_or_move() {
    for enabled in [false, true] {
        for x in [false, true] {
            for y in [false, true] {
                let (mut s, c) = setup();
                let mut settings = s.snapping();
                settings.enabled = enabled;
                settings.x = x;
                settings.y = y;
                s.set_snapping(settings);
                let baseline = s.undo_len();
                s.set_sketch_tool(SketchTool::Point);
                click(&mut s, at(c, 20.0, 30.0));
                assert_point(&graph(&s), 20.0, 30.0);
                assert_eq!(s.undo_len(), baseline + 1);
                s.set_sketch_tool(SketchTool::Select);
                s.pointer_down(at(c, 20.0, 30.0), false);
                s.pointer_hover(at(c, 29.3, 42.7), false, false);
                s.pointer_up(at(c, 29.3, 42.7), false, false);
                assert_point(&graph(&s), 29.3, 42.7);
                assert_eq!(s.snapping(), settings);
            }
        }
    }
}
#[test]
fn r11_axes_infer_in_screen_band_at_two_zooms_and_persist() {
    for zoom in [0.0, -400.0] {
        for (dx, dy, want) in [
            (30.0, 0.0, vec![Axis::X]),
            (0.0, 30.0, vec![Axis::Y]),
            (0.0, 0.0, vec![Axis::X, Axis::Y]),
        ] {
            let (mut s, c) = setup();
            s.wheel(0.0, zoom, 400.0, 300.0, false, true);
            let mm = (s.screen_to_document(1.0, 0.0).x - s.screen_to_document(0.0, 0.0).x).abs();
            let p = at(
                c,
                if dx == 0.0 { 4.0 * mm } else { dx },
                if dy == 0.0 { 4.0 * mm } else { dy },
            );
            s.set_sketch_tool(SketchTool::Point);
            s.pointer_hover(p, false, false);
            let before = s.undo_len();
            std::hint::black_box(s.draw_list());
            assert_eq!(s.undo_len(), before);
            assert_eq!(s.active_sketch(), None);
            click(&mut s, p);
            let geometry = graph(&s);
            assert_point(&geometry, dx, dy);
            let axes: Vec<_> = geometry
                .constraints
                .values()
                .filter_map(|c| match c.relation {
                    Relation::OnAxis { axis, .. } => Some(axis),
                    _ => None,
                })
                .collect();
            assert_eq!(axes.len(), want.len());
            for axis in want {
                assert!(axes.contains(&axis));
            }
            assert_eq!(s.undo_len(), before + 1);
            let saved_document = document(&s);
            let reopen = Session::open(904, &pack(&saved_document, "revision").unwrap()).unwrap();
            assert_eq!(
                document(&reopen)
                    .sketch(s.active_sketch().unwrap())
                    .unwrap()
                    .graph,
                geometry
            );
        }
    }
    for zoom in [0.0, -400.0] {
        let (mut s, c) = setup();
        s.wheel(0.0, zoom, 400.0, 300.0, false, true);
        let mm = (s.screen_to_document(1.0, 0.0).x - s.screen_to_document(0.0, 0.0).x).abs();
        s.set_sketch_tool(SketchTool::Point);
        click(&mut s, at(c, 8.0 * mm, 30.0));
        let geometry = graph(&s);
        assert_point(&geometry, 8.0 * mm, 30.0);
        assert!(geometry.constraints.is_empty());
    }
}

#[test]
fn r19_page_switch_cancels_pending_input_without_cross_page_commit() {
    let (mut session, center) = setup();
    let first = session.active_page().id;
    session.set_sketch_tool(SketchTool::Point);
    click(&mut session, at(center, 10.0, 20.0));
    let owner = session.active_sketch().unwrap();
    let original = graph(&session);
    let second = session.add_page();
    let second_frame = session.active_page();
    assert_eq!(second_frame.id, second);
    assert_eq!(session.active_sketch(), None);
    session.set_sketch_tool(SketchTool::Arc);
    click(
        &mut session,
        Point::new(second_frame.origin.x + 30.0, second_frame.origin.y + 40.0),
    );
    let count = session.undo_len();
    assert!(session.activate_page(&first));
    assert_eq!(session.active_sketch(), Some(owner));
    assert_eq!(graph(&session), original);
    assert_eq!(session.undo_len(), count);
    assert!(!session.sketch_secondary_press());
    assert!(session.activate_page(&second));
    assert_eq!(session.active_sketch(), None);
    assert!(!session.sketch_secondary_press());
    assert_eq!(session.undo_len(), count);
    assert_eq!(document(&session).page_sketch(&second), None);
}
#[test]
fn r09_clone_entry_returns_to_original_source_page_without_history() {
    let document = Document::new(911);
    let source_page = document.pages()[0].id.clone();
    let mut shape = Sketch::new("origin".into());
    shape.entities.insert(
        "point".into(),
        Entity::Point {
            position: curvyo_sketch_core::Position::mm(20.0, 30.0),
            fixed: false,
        },
    );
    let original = document
        .create_page_sketch(
            &source_page,
            &shape,
            &curvyo_document_core::Style::default(),
        )
        .unwrap();
    let composition = document.add_page();
    let clone = document.linked_clone(&[original], &[]).unwrap()[0];
    let mut session = Session::open(912, &pack(&document, "revision").unwrap()).unwrap();
    session.activate_page(&composition.id);
    session.set_workbench(Workbench::Sketch);
    let count = session.undo_len();
    assert!(session.enter_sketch(clone));
    assert_eq!(session.active_page().id, source_page);
    assert_eq!(session.active_sketch(), Some(original));
    assert_eq!(graph(&session), shape);
    assert_eq!(session.undo_len(), count);
    let saved = unpack(913, &session.pack("revision").unwrap()).unwrap();
    assert_eq!(saved.page_sketch(&composition.id), None);
    assert_eq!(saved.sketch_page(clone), None);
}
#[test]
fn r03_first_parameter_materializes_once_and_invalid_first_input_is_inert() {
    let (mut session, _) = setup();
    let version = document(&session).version();
    assert!(
        session
            .sketch_add_parameter(
                "width",
                "unknown + 1 mm",
                curvyo_sketch_core::Dimension::Length
            )
            .is_err()
    );
    assert_eq!(session.active_sketch(), None);
    assert_eq!(session.undo_len(), 0);
    assert_eq!(document(&session).version(), version);
    let id = session
        .sketch_add_parameter("width", "80 mm", curvyo_sketch_core::Dimension::Length)
        .unwrap();
    assert_eq!(session.undo_len(), 1);
    let owner = session.active_sketch().unwrap();
    assert_eq!(
        document(&session).page_sketch(&session.active_page().id),
        Some(owner)
    );
    assert!((graph(&session).parameters.evaluate().unwrap()[&id].value() - 80.0).abs() < 1e-6);
    let reopened = Session::open(914, &session.pack("revision").unwrap()).unwrap();
    assert!(
        (document(&reopened)
            .sketch(owner)
            .unwrap()
            .graph
            .parameters
            .evaluate()
            .unwrap()[&id]
            .value()
            - 80.0)
            .abs()
            < 1e-6
    );
}

fn selected(session: &Session) -> Vec<String> {
    let view: serde_json::Value = serde_json::from_str(&session.sketch_view_json()).unwrap();
    view["entities"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| e["selected"] == true)
        .map(|e| e["id"].as_str().unwrap().to_owned())
        .collect()
}
fn point_id(geometry: &Sketch, x: f64, y: f64) -> curvyo_sketch_core::EntityId {
    geometry.entities.iter().find_map(|(id,e)|matches!(e,Entity::Point {position,..} if (position.x.0-x).abs()<1e-6&&(position.y.0-y).abs()<1e-6).then_some(id.clone())).unwrap()
}
#[test]
fn r13_typed_mixed_chain_alt_repeat_and_closure_preserve_shared_vertices() {
    use curvyo_editor_wasm::KeyInput;
    let (mut session, center) = setup();
    session.set_sketch_tool(SketchTool::Polyline);
    click(&mut session, at(center, 20.0, 20.0));
    click(&mut session, at(center, 30.0, 20.0));
    let first = point_id(&graph(&session), 20.0, 20.0);
    let join = point_id(&graph(&session), 30.0, 20.0);
    let steps = session.undo_len();
    session.key_down(KeyInput {
        key: "Alt",
        alt: true,
        ..KeyInput::default()
    });
    let view =
        |s: &Session| serde_json::from_str::<serde_json::Value>(&s.sketch_view_json()).unwrap();
    assert_eq!(view(&session)["mode"], "perpendicular-line");
    for event in [
        KeyInput {
            key: "Alt",
            repeat: true,
            alt: true,
            ..KeyInput::default()
        },
        KeyInput {
            key: "Alt",
            dom_blocked: true,
            alt: true,
            ..KeyInput::default()
        },
        KeyInput {
            key: "Alt",
            ctrl: true,
            alt: true,
            ..KeyInput::default()
        },
    ] {
        session.key_down(event);
        assert_eq!(view(&session)["mode"], "perpendicular-line");
    }
    assert_eq!(session.undo_len(), steps);
    assert!(session.set_sketch_mode("tangent-arc"));
    session.pointer_hover(at(center, 36.0, 26.0), false, false);
    session.sketch_complete_numeric("6 mm", "").unwrap();
    let geometry = graph(&session);
    let end = point_id(&geometry, 36.0, 26.0);
    assert!(geometry.entities.values().any(|e|matches!(e,Entity::Arc {start,end:other,radius,..} if (start==&join&&other==&end||other==&join&&start==&end)&&(radius.0-6.0).abs()<1e-6)));
    assert!(session.set_sketch_mode("tangent-line"));
    session.sketch_complete_numeric("8 mm", "").unwrap();
    assert_point(&graph(&session), 36.0, 34.0);
    assert!(session.set_sketch_mode("free-line"));
    click(&mut session, at(center, 20.0, 20.0));
    let final_graph = graph(&session);
    assert_eq!(
        final_graph
            .entities
            .keys()
            .filter(|id| **id == first)
            .count(),
        1
    );
    assert_eq!(
        final_graph
            .entities
            .values()
            .filter(|e| matches!(e,Entity::Line {start,end} if start==&first||end==&first))
            .count(),
        2
    );
    assert_eq!(session.undo_len(), steps + 3);
}
#[test]
fn r13_standalone_arc_from_line_then_arc_reuses_explicit_endpoint() {
    let (mut session, center) = setup();
    session.set_sketch_tool(SketchTool::Polyline);
    click(&mut session, at(center, 20.0, 20.0));
    click(&mut session, at(center, 30.0, 20.0));
    session.escape();
    for start_x in [30.0, 40.0] {
        let before = graph(&session);
        let shared = point_id(&before, start_x, 20.0);
        let steps = session.undo_len();
        session.set_sketch_tool(SketchTool::Arc);
        click(&mut session, at(center, start_x, 20.0));
        click(&mut session, at(center, start_x + 5.0, 25.0));
        click(&mut session, at(center, start_x + 10.0, 20.0));
        let after = graph(&session);
        let end_id = point_id(&after, start_x + 10.0, 20.0);
        assert!(after.entities.values().any(|e|matches!(e,Entity::Arc {start,end,center:cid,radius,..} if (start==&shared&&end==&end_id||end==&shared&&start==&end_id)&&(radius.0-5.0).abs()<1e-6&&after.point(cid).is_ok_and(|p|(p.x.0-start_x-5.0).abs()<1e-6&&(p.y.0-20.0).abs()<1e-6))));
        assert_eq!(session.undo_len(), steps + 1);
        let outcome = session.key_down(curvyo_editor_wasm::KeyInput {
            key: "z",
            ctrl: true,
            ..Default::default()
        });
        if outcome == curvyo_editor_wasm::KeyOutcome::UndoBusy {
            session.undo();
        }
        assert_eq!(graph(&session), before);
        let outcome = session.key_down(curvyo_editor_wasm::KeyInput {
            key: "y",
            ctrl: true,
            ..Default::default()
        });
        if outcome == curvyo_editor_wasm::KeyOutcome::RedoBusy {
            session.redo();
        }
        assert_eq!(graph(&session), after);
    }
}
#[test]
fn r22_point_edge_hits_shift_toggle_and_ctrl_marquee_do_not_drag_geometry() {
    let (mut session, center) = setup();
    session.set_sketch_tool(SketchTool::Polyline);
    click(&mut session, at(center, 20.0, 20.0));
    click(&mut session, at(center, 50.0, 20.0));
    session.escape();
    session.set_sketch_tool(SketchTool::Select);
    let geometry = graph(&session);
    let left = point_id(&geometry, 20.0, 20.0);
    let right = point_id(&geometry, 50.0, 20.0);
    let hits = curvyo_ui_core::sketch_hits(
        &geometry,
        &geometry,
        curvyo_sketch_core::Position::mm(20.0, 20.0),
        curvyo_sketch_core::Length(1.0),
    );
    assert_eq!(hits[0].entity, left);
    assert!(hits[0].point);
    let hits = curvyo_ui_core::sketch_hits(
        &geometry,
        &geometry,
        curvyo_sketch_core::Position::mm(35.0, 20.0),
        curvyo_sketch_core::Length(1.0),
    );
    assert!(!hits[0].point);
    let steps = session.undo_len();
    click(&mut session, at(center, 20.0, 20.0));
    assert_eq!(selected(&session), vec![left.0.clone()]);
    session.modifiers_changed(true, false, false);
    session.pointer_down(at(center, 50.0, 20.0), true);
    session.pointer_up(at(center, 50.0, 20.0), true, false);
    assert_eq!(selected(&session).len(), 2);
    session.modifiers_changed(false, true, false);
    session.pointer_down(at(center, 15.0, 15.0), false);
    session.pointer_hover(at(center, 25.0, 25.0), false, true);
    session.pointer_up(at(center, 25.0, 25.0), false, true);
    assert_eq!(selected(&session), vec![right.0]);
    assert_eq!(graph(&session), geometry);
    assert_eq!(session.undo_len(), steps);
}
