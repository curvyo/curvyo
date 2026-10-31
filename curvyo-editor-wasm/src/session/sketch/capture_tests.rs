//! Endpoint capture and the Ctrl angle lock act on real pointer events (AC27, AC28).
use super::*;
use curvyo_document_core::Point;
use curvyo_sketch_core::{Construction, Entity, EntityId, Position, Relation, Sketch};

fn global(s: &Session, x: f64, y: f64) -> Point {
    s.sketch_placement().unwrap().apply_point(Point::new(x, y))
}

fn hover(s: &mut Session, x: f64, y: f64, ctrl: bool) {
    let p = global(s, x, y);
    s.pointer_hover(p, false, ctrl);
}

fn click(s: &mut Session, x: f64, y: f64, ctrl: bool) {
    let p = global(s, x, y);
    s.pointer_hover(p, false, ctrl);
    s.pointer_down(p, false);
    s.pointer_up(p, false, ctrl);
}

/// Millimetres per screen pixel at the current zoom.
fn px(s: &Session) -> f64 {
    1.0 / s.view().scale()
}

/// A sketch holding one line from (10, 10) to (20, 10) and the Polyline tool.
fn with_line() -> (Session, EntityId) {
    let mut s = Session::new(66_500);
    s.set_workbench(Workbench::Sketch);
    s.sketch_construct(&Construction::Polyline(vec![
        Position::mm(10.0, 10.0),
        Position::mm(20.0, 10.0),
    ]))
    .unwrap();
    let end = point_at(&graph(&s), Position::mm(20.0, 10.0)).unwrap();
    s.set_sketch_tool(SketchTool::Polyline);
    (s, end)
}

fn graph(s: &Session) -> Sketch {
    s.document.sketch(s.active_sketch().unwrap()).unwrap().graph
}

fn point_at(graph: &Sketch, at: Position) -> Option<EntityId> {
    graph.entities.iter().find_map(|(id, e)| match e {
        Entity::Point { position, .. } if position.distance(at).0 < 1e-6 => Some(id.clone()),
        _ => None,
    })
}

fn points_near(s: &Session, at: Position) -> usize {
    let snapshot = s.document.sketch(s.active_sketch().unwrap()).unwrap();
    snapshot
        .graph
        .entities
        .keys()
        .filter(|id| {
            snapshot
                .solution
                .point(id)
                .is_ok_and(|p| p.distance(at).0 < 1e-3)
        })
        .count()
}

fn curves_at(graph: &Sketch, point: &EntityId) -> usize {
    graph
        .entities
        .values()
        .filter(|e| match e {
            Entity::Line { start, end } | Entity::Arc { start, end, .. } => {
                start == point || end == point
            }
            _ => false,
        })
        .count()
}

#[test]
fn capture_pulls_within_12_px_and_releases_beyond_16_px() {
    let (mut s, end) = with_line();
    let u = px(&s);
    hover(&mut s, 20.0 + 11.0 * u, 10.0, false);
    assert_eq!(s.sketch.capture, Some(end.clone()), "11 px captures");
    assert!(s.sketch.hover.unwrap().distance(Position::mm(20.0, 10.0)).0 < 1e-9);
    hover(&mut s, 20.0 + 15.0 * u, 10.0, false);
    assert_eq!(
        s.sketch.capture,
        Some(end.clone()),
        "15 px keeps the capture"
    );
    hover(&mut s, 20.0 + 17.0 * u, 10.0, false);
    assert_eq!(s.sketch.capture, None, "17 px releases");
    hover(&mut s, 20.0 + 14.0 * u, 10.0, false);
    assert_eq!(s.sketch.capture, None, "14 px does not capture anew");
    let view = s.sketch_view();
    assert!(
        !view["annotations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|a| a["text"] == "Endpoint")
    );
    hover(&mut s, 20.0, 10.0 + 5.0 * u, false);
    let view = s.sketch_view();
    assert!(
        view["annotations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|a| a["text"] == "Endpoint" && a["preview"] == true),
        "the capture marker is labelled"
    );
}

#[test]
fn the_nearest_open_endpoint_wins_and_inner_joints_never_capture() {
    let (mut s, _) = with_line();
    let start = point_at(&graph(&s), Position::mm(10.0, 10.0)).unwrap();
    let u = px(&s);
    // Both ends 5 mm apart on screen at most; the nearer one wins.
    s.sketch_construct(&Construction::Polyline(vec![
        Position::mm(10.0 + 20.0 * u, 10.0),
        Position::mm(10.0 + 20.0 * u, 40.0),
    ]))
    .unwrap();
    s.set_sketch_tool(SketchTool::Polyline);
    hover(&mut s, 10.0 + 8.0 * u, 10.0, false);
    assert_eq!(s.sketch.capture, Some(start));
    // A joint with two adjoining segments is not an open endpoint.
    click(&mut s, 50.0, 50.0, false);
    click(&mut s, 60.0, 50.0, false);
    click(&mut s, 70.0, 60.0, false);
    hover(&mut s, 60.0 + 2.0 * u, 50.0, false);
    assert_eq!(s.sketch.capture, None);
}

#[test]
fn ending_on_a_captured_endpoint_reuses_its_identity() {
    let (mut s, end) = with_line();
    let u = px(&s);
    click(&mut s, 40.0, 40.0, false);
    click(&mut s, 20.0 + 6.0 * u, 10.0 - 4.0 * u, false);
    let after = graph(&s);
    assert_eq!(
        curves_at(&after, &end),
        2,
        "the new segment ends on the endpoint"
    );
    assert_eq!(
        points_near(&s, Position::mm(20.0, 10.0)),
        1,
        "no duplicate point"
    );
    assert!(s.sketch.chain.is_none(), "joining ends the chain");
    assert_eq!(s.undo_len(), 2);
}

#[test]
fn closing_a_chain_by_capture_yields_one_shared_point() {
    let mut s = Session::new(66_501);
    s.set_workbench(Workbench::Sketch);
    s.set_sketch_tool(SketchTool::Polyline);
    let u = px(&s);
    click(&mut s, 10.0, 10.0, false);
    click(&mut s, 40.0, 10.0, false);
    click(&mut s, 40.0, 40.0, false);
    click(&mut s, 10.0 + 9.0 * u, 10.0 + 3.0 * u, false);
    let after = graph(&s);
    let start = point_at(&after, Position::mm(10.0, 10.0)).unwrap();
    assert_eq!(curves_at(&after, &start), 2);
    assert_eq!(points_near(&s, Position::mm(10.0, 10.0)), 1);
    assert!(s.sketch.chain.is_none());
}

#[test]
fn a_three_point_arc_ending_on_a_captured_endpoint_reuses_it() {
    let (mut s, end) = with_line();
    s.set_sketch_tool(SketchTool::Arc);
    let u = px(&s);
    click(&mut s, 40.0, 10.0, false);
    click(&mut s, 30.0, 18.0, false);
    click(&mut s, 20.0 + 7.0 * u, 10.0, false);
    assert_eq!(curves_at(&graph(&s), &end), 2);
    assert_eq!(points_near(&s, Position::mm(20.0, 10.0)), 1);
}

#[test]
fn capture_ignores_the_vector_snapping_settings_and_other_tools() {
    let (mut s, _) = with_line();
    s.set_sketch_tool(SketchTool::Circle);
    let u = px(&s);
    hover(&mut s, 20.0 + 3.0 * u, 10.0, false);
    assert_eq!(s.sketch.capture, None, "circle picks are not captured");
}

fn locked_session(degrees: f64) -> Session {
    let mut s = Session::new(66_510);
    s.set_workbench(Workbench::Sketch);
    s.set_sketch_tool(SketchTool::Polyline);
    click(&mut s, 30.0, 30.0, false);
    // 3 degrees off the stop and 25 mm away; Ctrl locks it.
    let raw = (degrees + 3.0).to_radians();
    click(
        &mut s,
        30.0 + 25.0 * raw.cos(),
        30.0 + 25.0 * raw.sin(),
        true,
    );
    s
}

#[test]
fn ctrl_lock_commits_horizontal_or_vertical_only_at_the_axis_stops() {
    for (degrees, expected) in [
        (0.0, Some("horizontal")),
        (90.0, Some("vertical")),
        (180.0, Some("horizontal")),
        (270.0, Some("vertical")),
        (45.0, None),
    ] {
        let s = locked_session(degrees);
        let snapshot = s.document.sketch(s.active_sketch().unwrap()).unwrap();
        let (line, start, end) = snapshot
            .graph
            .entities
            .iter()
            .find_map(|(id, e)| match e {
                Entity::Line { start, end } => Some((id.clone(), start.clone(), end.clone())),
                _ => None,
            })
            .unwrap();
        let a = snapshot.solution.point(&start).unwrap();
        let b = snapshot.solution.point(&end).unwrap();
        let angle = (b.y.0 - a.y.0).atan2(b.x.0 - a.x.0).to_degrees();
        let diff = (angle - degrees).rem_euclid(360.0);
        assert!(diff.min(360.0 - diff) < 1e-6, "{degrees}: drawn at {angle}");
        assert!((a.distance(b).0 - 25.0).abs() < 1e-6, "length kept");
        let kinds: Vec<_> = snapshot
            .graph
            .constraints
            .values()
            .filter_map(|c| match &c.relation {
                Relation::Horizontal(id) if *id == line => Some("horizontal"),
                Relation::Vertical(id) if *id == line => Some("vertical"),
                _ => None,
            })
            .collect();
        assert_eq!(kinds, expected.into_iter().collect::<Vec<_>>(), "{degrees}");
    }
}

#[test]
fn ctrl_lock_shows_the_pending_relation_and_angle_before_commit_and_releases() {
    let mut s = Session::new(66_511);
    s.set_workbench(Workbench::Sketch);
    s.set_sketch_tool(SketchTool::Polyline);
    click(&mut s, 30.0, 30.0, false);
    hover(&mut s, 55.0, 31.0, true);
    let texts = |s: &Session| {
        s.sketch_view()["annotations"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|a| a["preview"] == true)
            .map(|a| a["text"].as_str().unwrap().to_owned())
            .collect::<Vec<_>>()
    };
    let shown = texts(&s);
    assert!(shown.iter().any(|t| t == "Horizontal"), "{shown:?}");
    assert!(shown.iter().any(|t| t.ends_with(", 0°")), "{shown:?}");
    assert!((s.sketch.hover.unwrap().y.0 - 30.0).abs() < 1e-9);
    s.modifiers_changed(false, false, false);
    assert!(!texts(&s).iter().any(|t| t == "Horizontal"), "released");
    assert!(
        (s.sketch.hover.unwrap().y.0 - 31.0).abs() < 1e-9,
        "free again"
    );
}

#[test]
fn ctrl_has_no_effect_in_fixed_direction_modes_and_capture_wins() {
    let (mut s, end) = with_line();
    click(&mut s, 40.0, 40.0, false);
    // Capture beats the lock.
    let u = px(&s);
    hover(&mut s, 20.0 + 4.0 * u, 10.0 + 2.0 * u, true);
    assert_eq!(s.sketch.capture, Some(end));
    assert!(s.sketch.lock.is_none());
    assert!(s.set_sketch_mode("tangent-line"));
    hover(&mut s, 60.0, 47.0, true);
    assert!(s.sketch.lock.is_none(), "tangent mode ignores Ctrl");
}

#[test]
fn ctrl_lock_on_the_slot_centreline_constrains_its_side() {
    let mut s = Session::new(66_512);
    s.set_workbench(Workbench::Sketch);
    s.set_sketch_tool(SketchTool::Slot);
    click(&mut s, 30.0, 30.0, false);
    click(&mut s, 31.0, 60.0, true);
    click(&mut s, 34.0, 45.0, false);
    let snapshot = s.document.sketch(s.active_sketch().unwrap()).unwrap();
    assert!(
        snapshot
            .graph
            .constraints
            .values()
            .any(|c| matches!(c.relation, Relation::Vertical(_)))
    );
}
