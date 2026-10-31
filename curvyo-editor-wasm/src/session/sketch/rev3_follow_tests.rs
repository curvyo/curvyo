//! Click-to-follow creation below 4 screen pixels and press-drag-release above (0066 AC38, AC41).
use super::*;
use crate::session::KeyInput;
use curvyo_document_core::Point;
use curvyo_sketch_core::{Entity, Position, Relation, Sketch};

fn global(s: &Session, x: f64, y: f64) -> Point {
    s.sketch_placement().unwrap().apply_point(Point::new(x, y))
}

fn click(s: &mut Session, x: f64, y: f64) {
    let p = global(s, x, y);
    s.pointer_hover(p, false, false);
    s.pointer_down(p, false);
    s.pointer_up(p, false, false);
}

fn drag(s: &mut Session, from: (f64, f64), to: (f64, f64), ctrl: bool) {
    let (a, b) = (global(s, from.0, from.1), global(s, to.0, to.1));
    s.pointer_hover(a, false, false);
    s.pointer_down(a, false);
    s.pointer_hover(b, false, ctrl);
    s.pointer_up(b, false, ctrl);
}

fn escape(s: &mut Session) {
    s.key_down(KeyInput {
        key: "Escape",
        shift: false,
        ctrl: false,
        alt: false,
        repeat: false,
        dom_blocked: false,
        mac: false,
        time_ms: 0.0,
    });
}

fn session(tool: SketchTool) -> Session {
    let mut s = Session::new(66_910);
    s.set_workbench(Workbench::Sketch);
    s.set_sketch_tool(tool);
    s
}

fn graph(s: &Session) -> Option<Sketch> {
    s.active_sketch()
        .map(|id| s.document.sketch(id).unwrap().solution)
}

/// Solved point positions, sorted, so two paths compare without their minted identities.
fn shape(s: &Session) -> Vec<String> {
    let mut points: Vec<_> = graph(s)
        .unwrap()
        .entities
        .values()
        .filter_map(|e| match e {
            Entity::Point { position, .. } => {
                Some(format!("{:.6} {:.6}", position.x.0, position.y.0))
            }
            _ => None,
        })
        .collect();
    points.sort_unstable();
    points
}

const TOOLS: &[(SketchTool, &[(f64, f64)])] = &[
    (SketchTool::Polyline, &[(20.0, 20.0), (45.0, 32.0)]),
    (SketchTool::Rectangle, &[(20.0, 20.0), (45.0, 32.0)]),
    (SketchTool::Circle, &[(20.0, 20.0), (32.0, 25.0)]),
    (
        SketchTool::Slot,
        &[(20.0, 20.0), (45.0, 20.0), (40.0, 25.0)],
    ),
    (
        SketchTool::Ellipse,
        &[(20.0, 20.0), (35.0, 20.0), (28.0, 27.0)],
    ),
    (SketchTool::Arc, &[(20.0, 20.0), (30.0, 28.0), (45.0, 20.0)]),
    (
        SketchTool::ArcCenter,
        &[(20.0, 20.0), (30.0, 20.0), (20.0, 30.0)],
    ),
];

#[test]
fn click_to_follow_and_press_drag_give_identical_geometry_for_every_tool() {
    for (tool, picks) in TOOLS {
        let mut clicked = session(*tool);
        for (x, y) in *picks {
            clicked.pointer_hover(global(&clicked, *x, *y), false, false);
            assert_eq!(
                clicked.undo_len(),
                0,
                "{tool:?}: nothing commits while following"
            );
            click(&mut clicked, *x, *y);
        }
        let mut dragged = session(*tool);
        drag(&mut dragged, picks[0], picks[1], false);
        for (x, y) in &picks[2..] {
            click(&mut dragged, *x, *y);
        }
        assert_eq!(clicked.undo_len(), 1, "{tool:?}");
        assert_eq!(dragged.undo_len(), 1, "{tool:?}");
        assert_eq!(shape(&clicked), shape(&dragged), "{tool:?}");
    }
}

#[test]
fn a_release_below_4_px_follows_and_from_4_px_commits() {
    let mut s = session(SketchTool::Polyline);
    let u = 1.0 / s.view().scale();
    drag(&mut s, (20.0, 20.0), (20.0 + 3.9 * u, 20.0), false);
    assert_eq!(
        s.sketch.picks.len(),
        1,
        "3.9 px is a click: the shape follows"
    );
    assert!(graph(&s).is_none());
    s.pointer_hover(global(&s, 40.0, 35.0), false, false);
    assert!(
        s.sketch_construction_preview().is_some(),
        "the preview follows the pointer"
    );
    escape(&mut s);
    assert!(s.sketch.picks.is_empty() && s.sketch_construction_preview().is_none());
    assert_eq!(s.undo_len(), 0, "Escape in the follow state adds no step");
    let mut s = session(SketchTool::Polyline);
    drag(
        &mut s,
        (20.0, 20.0),
        (20.0 + 4.0 * u, 20.0 + 0.0001 * u),
        false,
    );
    assert_eq!(s.undo_len(), 1, "4 px is a drag: the release commits");
}

#[test]
fn ctrl_lock_and_alt_work_in_the_press_drag_and_the_follow_state() {
    // Ctrl at the release of a drag locks 3 degrees off horizontal and persists Horizontal.
    let mut s = session(SketchTool::Polyline);
    let raw = 3.0_f64.to_radians();
    drag(
        &mut s,
        (20.0, 20.0),
        (20.0 + 25.0 * raw.cos(), 20.0 + 25.0 * raw.sin()),
        true,
    );
    let id = s.active_sketch().unwrap();
    assert!(
        s.document
            .sketch(id)
            .unwrap()
            .graph
            .constraints
            .values()
            .any(|c| matches!(c.relation, Relation::Horizontal(_)))
    );
    // Alt in the follow state switches the next segment's mode before the click.
    let mut s = session(SketchTool::Polyline);
    click(&mut s, 20.0, 20.0);
    s.pointer_hover(global(&s, 40.0, 30.0), false, false);
    s.key_down(KeyInput {
        key: "Alt",
        shift: false,
        ctrl: false,
        alt: true,
        repeat: false,
        dom_blocked: false,
        mac: false,
        time_ms: 0.0,
    });
    assert_eq!(
        s.sketch_view()["canvas"]["mode_label"]["text"],
        "Perpendicular line"
    );
}

#[test]
fn a_drag_whose_press_and_release_capture_one_point_follows_like_a_click() {
    for start in [(0.0, 0.0), (20.0, 20.0)] {
        let mut s = session(SketchTool::Polyline);
        drag(&mut s, (20.0, 20.0), (45.0, 32.0), false);
        escape(&mut s);
        s.set_sketch_tool(SketchTool::Rectangle);
        let steps = s.undo_len();
        let u = 1.0 / s.view().scale();
        // A 5 px drag (not a click) whose press and release both land on the same point: the
        // origin by axis inference, a line end by capture.
        drag(
            &mut s,
            (start.0 + u, start.1 + u),
            (start.0 + 5.0 * u, start.1 + 4.0 * u),
            false,
        );
        assert_eq!(s.undo_len(), steps, "{start:?}: nothing commits yet");
        assert_eq!(s.sketch.error, None, "{start:?}: no refusal");
        assert_eq!(
            s.sketch.picks,
            [Position::mm(start.0, start.1)],
            "{start:?}: the captured point is the first pick and the shape follows"
        );
        click(&mut s, start.0 + 30.0, start.1 + 15.0);
        assert_eq!(s.undo_len(), steps + 1, "{start:?}: the next click commits");
    }
}
