//! Endpoint capture on every point-creating pick of every tool, through pointer and key events
//! (0066 AC36, AC41).
use super::*;
use crate::session::KeyInput;
use curvyo_document_core::Point;
use curvyo_sketch_core::{Construction, Entity, EntityId, Position, Sketch};

fn key(s: &mut Session, key: &str) {
    s.key_down(KeyInput {
        key,
        shift: false,
        ctrl: false,
        alt: key == "Alt",
        repeat: false,
        dom_blocked: false,
        mac: false,
        time_ms: 0.0,
    });
}

fn global(s: &Session, x: f64, y: f64) -> Point {
    s.sketch_placement().unwrap().apply_point(Point::new(x, y))
}

fn hover(s: &mut Session, x: f64, y: f64) {
    let p = global(s, x, y);
    s.pointer_hover(p, false, false);
}

fn click(s: &mut Session, x: f64, y: f64) {
    let p = global(s, x, y);
    s.pointer_hover(p, false, false);
    s.pointer_down(p, false);
    s.pointer_up(p, false, false);
}

fn px(s: &Session) -> f64 {
    1.0 / s.view().scale()
}

fn snapshot(s: &Session) -> curvyo_document_core::SketchSnapshot {
    s.document.sketch(s.active_sketch().unwrap()).unwrap()
}

fn graph(s: &Session) -> Sketch {
    snapshot(s).graph
}

fn point_at(graph: &Sketch, at: Position) -> EntityId {
    graph
        .entities
        .iter()
        .find_map(|(id, e)| match e {
            Entity::Point { position, .. } if position.distance(at).0 < 1e-6 => Some(id.clone()),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no point at {at:?}"))
}

/// Points of the solved sketch within 1e-3 mm of `id`'s solved position, `id` included.
fn points_near(s: &Session, id: &EntityId) -> usize {
    let snapshot = snapshot(s);
    let at = snapshot.solution.point(id).unwrap();
    snapshot
        .graph
        .entities
        .keys()
        .filter(|other| {
            snapshot
                .solution
                .point(other)
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

/// Presses Alt until the mode label names `mode`, the pointer at (60, 60).
fn mode(s: &mut Session, mode: &str) {
    hover(s, 60.0, 60.0);
    for _ in 0..6 {
        if s.sketch_view()["canvas"]["mode_label"]["text"] == mode {
            return;
        }
        key(s, "Alt");
    }
    panic!("no mode {mode}");
}

/// Hovers 2 px off `at` and asserts the capture marker shows on `id` before the click.
fn captured_hover(s: &mut Session, at: Position, id: &EntityId, what: &str) {
    let u = px(s);
    hover(s, at.x.0 + 2.0 * u, at.y.0 - 1.0 * u);
    assert_eq!(s.sketch.capture.as_ref(), Some(id), "{what}: captured");
    assert!(
        s.sketch.hover.unwrap().distance(at).0 < 1e-9,
        "{what}: pulled onto the endpoint"
    );
    assert!(
        s.sketch_view()["annotations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|a| a["text"] == "Endpoint"),
        "{what}: marker before the click"
    );
}

fn captured_click(s: &mut Session, at: Position, id: &EntityId, what: &str) {
    captured_hover(s, at, id, what);
    let u = px(s);
    click(s, at.x.0 + 2.0 * u, at.y.0 - 1.0 * u);
}

#[test]
fn a_centre_first_arc_refills_a_deleted_chain_segment_on_both_free_endpoints() {
    let mut s = Session::new(66_900);
    s.set_workbench(Workbench::Sketch);
    s.set_sketch_tool(SketchTool::Polyline);
    // The customer's chain: polyline arc, tangent line, tangent arc, tangent arc to the start.
    click(&mut s, 20.0, 20.0);
    mode(&mut s, "Tangent arc");
    click(&mut s, 40.0, 40.0);
    mode(&mut s, "Tangent line");
    click(&mut s, 40.0, 70.0);
    mode(&mut s, "Tangent arc");
    click(&mut s, 20.0, 90.0);
    let start = point_at(&graph(&s), Position::mm(20.0, 20.0));
    captured_click(&mut s, Position::mm(20.0, 20.0), &start, "closing");
    assert!(s.sketch.chain.is_none(), "the chain closed");
    let closed = graph(&s);
    assert_eq!(curves_at(&closed, &start), 2);
    // Delete segment 2, the tangent line.
    s.set_sketch_tool(SketchTool::Select);
    let line = closed
        .entities
        .iter()
        .find_map(|(id, e)| matches!(e, Entity::Line { .. }).then(|| id.clone()))
        .unwrap();
    s.sketch_select_entity(&line, false);
    key(&mut s, "Delete");
    let opened = graph(&s);
    let a = point_at(&opened, Position::mm(40.0, 40.0));
    let b = point_at(&opened, Position::mm(40.0, 70.0));
    assert_eq!((curves_at(&opened, &a), curves_at(&opened, &b)), (1, 1));
    // Refill with Arc, centre-first: centre, start on one free end, end on the other. The
    // centre is off the bisector, so the end lies off the start radius.
    s.set_sketch_tool(SketchTool::Arc);
    hover(&mut s, 36.0, 54.0);
    mode(&mut s, "Arc, centre");
    let steps = s.undo_len();
    click(&mut s, 36.0, 54.0);
    captured_click(&mut s, Position::mm(40.0, 40.0), &a, "start pick");
    captured_click(&mut s, Position::mm(40.0, 70.0), &b, "end pick");
    assert_eq!(s.undo_len(), steps + 1, "one history step");
    let after = graph(&s);
    let new_points = after
        .entities
        .iter()
        .filter(|(id, e)| !opened.entities.contains_key(*id) && matches!(e, Entity::Point { .. }))
        .count();
    assert_eq!(new_points, 1, "only the centre is new");
    assert_eq!((curves_at(&after, &a), curves_at(&after, &b)), (2, 2));
    assert_eq!((points_near(&s, &a), points_near(&s, &b)), (1, 1));
    assert_eq!(
        after.constraints.len(),
        opened.constraints.len(),
        "no extra constraints"
    );
    let snap = snapshot(&s);
    assert!(
        !matches!(
            snap.status,
            Some(
                curvyo_sketch_core::SolveStatus::Conflicting
                    | curvyo_sketch_core::SolveStatus::SolverFailure
            )
        ),
        "{:?}",
        snap.status
    );
}

/// One tool or variant: how to select it, the picks before the end pick.
struct Case {
    name: &'static str,
    tool: SketchTool,
    before: &'static [(f64, f64)],
}

const CASES: &[Case] = &[
    Case {
        name: "point",
        tool: SketchTool::Point,
        before: &[],
    },
    Case {
        name: "polyline",
        tool: SketchTool::Polyline,
        before: &[(40.0, 40.0)],
    },
    Case {
        name: "polyline chain",
        tool: SketchTool::Polyline,
        before: &[(40.0, 40.0), (40.0, 25.0)],
    },
    Case {
        name: "arc start-first",
        tool: SketchTool::Arc,
        before: &[(40.0, 40.0), (35.0, 20.0)],
    },
    // Resumes from the open end (50, 40) of the second line.
    Case {
        name: "arc resumed",
        tool: SketchTool::Arc,
        before: &[(50.0, 40.0), (40.0, 20.0)],
    },
    Case {
        name: "arc centre-first",
        tool: SketchTool::ArcCenter,
        before: &[(30.0, 30.0), (40.0, 30.0)],
    },
    Case {
        name: "circle",
        tool: SketchTool::Circle,
        before: &[(40.0, 10.0)],
    },
    Case {
        name: "circle 3 points",
        tool: SketchTool::CircleThreePoints,
        before: &[(40.0, 30.0), (50.0, 20.0)],
    },
    Case {
        name: "ellipse",
        tool: SketchTool::Ellipse,
        before: &[(40.0, 30.0), (55.0, 30.0)],
    },
    Case {
        name: "ellipse 3 points",
        tool: SketchTool::EllipseThreePoints,
        before: &[(10.0, 30.0), (40.0, 30.0)],
    },
    Case {
        name: "rectangle",
        tool: SketchTool::Rectangle,
        before: &[(40.0, 40.0)],
    },
    Case {
        name: "slot",
        tool: SketchTool::Slot,
        before: &[(40.0, 30.0), (60.0, 30.0)],
    },
];

/// Lines (10, 10)-(20, 10) and (40, 40)-(50, 40) and the case's tool.
fn setup(case: &Case) -> (Session, EntityId) {
    let mut s = Session::new(66_901);
    s.set_workbench(Workbench::Sketch);
    for (from, to) in [((10.0, 10.0), (20.0, 10.0)), ((40.0, 40.0), (50.0, 40.0))] {
        s.sketch_construct(&Construction::Polyline(vec![
            Position::mm(from.0, from.1),
            Position::mm(to.0, to.1),
        ]))
        .unwrap();
    }
    let end = point_at(&graph(&s), Position::mm(20.0, 10.0));
    s.set_sketch_tool(case.tool);
    (s, end)
}

#[test]
fn the_end_pick_of_every_tool_and_variant_captures_and_reuses_a_nearby_endpoint() {
    let e = Position::mm(20.0, 10.0);
    for case in CASES {
        let (mut s, end) = setup(case);
        for (x, y) in case.before {
            click(&mut s, *x, *y);
        }
        let steps = s.undo_len();
        let before = graph(&s);
        captured_click(&mut s, e, &end, case.name);
        let after = graph(&s);
        if case.tool == SketchTool::Point {
            assert_eq!(after, before, "a point on an endpoint is that endpoint");
            continue;
        }
        assert!(s.undo_len() > steps, "{}: committed", case.name);
        assert_eq!(
            points_near(&s, &end),
            1,
            "{}: no duplicate point",
            case.name
        );
        let reused = curves_at(&after, &end) > curves_at(&before, &end);
        let point_free = matches!(
            case.tool,
            SketchTool::Circle
                | SketchTool::CircleThreePoints
                | SketchTool::Ellipse
                | SketchTool::EllipseThreePoints
                | SketchTool::Slot
        );
        // Radius, through and width picks create no point: the pick is pulled, nothing joins.
        assert_eq!(reused, !point_free, "{}: identity reused", case.name);
    }
}

#[test]
fn every_pick_of_a_rectangle_and_a_slot_reuses_captured_endpoints() {
    // First and end pick of a rectangle, both cap centres of a slot.
    for (tool, picks) in [
        (SketchTool::Rectangle, vec![(20.0, 10.0), (50.0, 40.0)]),
        (
            SketchTool::Slot,
            vec![(20.0, 10.0), (50.0, 40.0), (40.0, 10.0)],
        ),
    ] {
        let case = Case {
            name: "",
            tool,
            before: &[],
        };
        let (mut s, end) = setup(&case);
        let other = point_at(&graph(&s), Position::mm(50.0, 40.0));
        let before = graph(&s);
        for (x, y) in &picks {
            let at = Position::mm(*x, *y);
            if at.distance(Position::mm(20.0, 10.0)).0 < 1e-9 {
                captured_click(&mut s, at, &end, "first");
            } else if at.distance(Position::mm(50.0, 40.0)).0 < 1e-9 {
                captured_click(&mut s, at, &other, "second");
            } else {
                click(&mut s, *x, *y);
            }
        }
        let after = graph(&s);
        assert!(after.entities.len() > before.entities.len(), "{tool:?}");
        assert_eq!(
            (points_near(&s, &end), points_near(&s, &other)),
            (1, 1),
            "{tool:?}"
        );
        assert!(
            after
                .entities
                .values()
                .any(|e| e.point_ids().contains(&&end)),
            "{tool:?} uses the endpoint"
        );
    }
}

#[test]
fn a_press_drag_line_from_a_captured_endpoint_starts_on_it() {
    let case = Case {
        name: "",
        tool: SketchTool::Polyline,
        before: &[],
    };
    let (mut s, end) = setup(&case);
    let u = px(&s);
    let from = global(&s, 20.0 + 2.0 * u, 10.0);
    let to = global(&s, 30.0, 30.0);
    s.pointer_hover(from, false, false);
    s.pointer_down(from, false);
    s.pointer_hover(to, false, false);
    s.pointer_up(to, false, false);
    assert_eq!(curves_at(&graph(&s), &end), 2);
    assert_eq!(points_near(&s, &end), 1);
    assert_eq!(
        s.sketch.chain.as_ref().map(|(first, _)| first),
        Some(&end),
        "the chain starts at the endpoint"
    );
}
