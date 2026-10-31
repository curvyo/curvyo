//! Polyline mode cycling and arc previews hold for every segment of a long chain.
#![allow(clippy::many_single_char_names)] // Geometric frame letters b, e, n, u, v.
use super::*;
use crate::session::{KeyInput, KeyOutcome};
use curvyo_document_core::Point;
use curvyo_sketch_core::{ContinuationMode, Entity, Position};

fn local_click(s: &mut Session, x: f64, y: f64) {
    let p = s.sketch_placement().unwrap().apply_point(Point::new(x, y));
    s.pointer_hover(p, false, false);
    s.pointer_down(p, false);
    s.pointer_up(p, false, false);
}

fn local_hover(s: &mut Session, x: f64, y: f64) {
    let p = s.sketch_placement().unwrap().apply_point(Point::new(x, y));
    s.pointer_hover(p, false, false);
}

fn alt(s: &mut Session, repeat: bool) -> KeyOutcome {
    s.key_down(KeyInput {
        key: "Alt",
        shift: false,
        ctrl: false,
        alt: true,
        repeat,
        dom_blocked: false,
        mac: false,
        time_ms: 0.0,
    })
}

fn polyline() -> Session {
    let mut s = Session::new(66_400);
    s.set_workbench(Workbench::Sketch);
    s.set_sketch_tool(SketchTool::Polyline);
    s
}

fn preview_arc(s: &Session) -> Option<(f64, f64)> {
    let preview = s.sketch_construction_preview()?;
    preview
        .candidate
        .entities
        .iter()
        .filter(|(id, _)| !preview_base_has(s, id))
        .find_map(|(_, entity)| match entity {
            Entity::Arc {
                start_angle,
                end_angle,
                radius,
                ..
            } => Some((end_angle.0 - start_angle.0, radius.0)),
            _ => None,
        })
}

fn preview_base_has(s: &Session, id: &curvyo_sketch_core::EntityId) -> bool {
    s.sketch
        .preview_base
        .as_ref()
        .is_some_and(|(_, base)| base.entities.contains_key(id))
}

/// The committed end of the chain and its outgoing unit direction.
fn chain_end(s: &Session) -> (Position, (f64, f64)) {
    if s.sketch.chain.is_none() {
        return (s.sketch.picks[0], super::first_segment::START_DIRECTION);
    }
    let snapshot = s.document.sketch(s.active_sketch().unwrap()).unwrap();
    let (_, last) = s.sketch.chain.clone().unwrap();
    let previous = s.sketch.chain_curve.clone().unwrap();
    let tangent =
        curvyo_sketch_core::endpoint_tangent(&snapshot.solution, &previous, &last).unwrap();
    (snapshot.solution.point(&last).unwrap(), tangent)
}

#[test]
fn alt_cycles_every_applicable_mode_on_the_first_segment() {
    let mut s = polyline();
    local_click(&mut s, 10.0, 10.0);
    local_hover(&mut s, 20.0, 14.0);
    assert_eq!(s.sketch.mode, ContinuationMode::FreeLine);
    let mut seen = vec![s.sketch.mode];
    for _ in 0..super::modes::MODES.len() - 1 {
        assert_eq!(alt(&mut s, false), KeyOutcome::Consumed);
        assert!(
            alt(&mut s, true) == KeyOutcome::Ignored,
            "key repeat ignored"
        );
        seen.push(s.sketch.mode);
        assert!(
            s.sketch_construction_preview().is_some(),
            "{:?} previews on segment 1",
            s.sketch.mode
        );
        assert_eq!(s.sketch.picks.len(), 1, "start pick kept");
    }
    let all: Vec<_> = super::modes::MODES.iter().map(|(mode, _)| *mode).collect();
    assert_eq!(seen, all);
    assert_eq!(alt(&mut s, false), KeyOutcome::Consumed);
    assert_eq!(s.sketch.mode, ContinuationMode::FreeLine, "wraps");
}

#[test]
fn first_segment_tangent_arc_commits_from_a_free_start() {
    let mut s = polyline();
    local_click(&mut s, 0.0 + 10.0, 10.0);
    s.set_sketch_mode("tangent-arc");
    local_hover(&mut s, 20.0, 20.0);
    assert!(preview_arc(&s).is_some());
    local_click(&mut s, 20.0, 20.0);
    let graph = s.document.sketch(s.active_sketch().unwrap()).unwrap().graph;
    assert!(
        graph
            .entities
            .values()
            .any(|e| matches!(e, Entity::Arc { .. }))
    );
    // Segment 2 continues from the arc in the same mode.
    assert!(s.sketch.chain_curve.is_some());
    assert_eq!(s.sketch.mode, ContinuationMode::TangentArc);
}

fn assert_arc_both_sides(s: &mut Session, segment: usize) {
    for mode in [
        "tangent-arc",
        "perpendicular-arc-left",
        "perpendicular-arc-right",
    ] {
        assert!(s.set_sketch_mode(mode), "{mode} at segment {segment}");
        let (b, e) = chain_end(s);
        let direction = match mode {
            "perpendicular-arc-left" => (-e.1, e.0),
            "perpendicular-arc-right" => (e.1, -e.0),
            _ => e,
        };
        let n = (-direction.1, direction.0);
        // A continuous swing from one side through to the other.
        for v in [8.0, 4.0, 1.0, 0.2, 0.01, -0.01, -0.2, -1.0, -4.0, -8.0] {
            for u in [6.0, 2.0] {
                let x = b.x.0 + u * direction.0 + v * n.0;
                let y = b.y.0 + u * direction.1 + v * n.1;
                local_hover(s, x, y);
                let arc = preview_arc(s);
                assert!(
                    arc.is_some(),
                    "segment {segment}, {mode}: no arc at u={u} v={v}: {:?}",
                    s.sketch.error
                );
            }
        }
    }
    s.set_sketch_mode("free-line");
}

#[test]
fn arc_preview_exists_on_both_sides_at_segment_1() {
    let mut s = polyline();
    // Off the axes, so axis inference does not move the pointer onto the start direction.
    local_click(&mut s, 5.0, 7.0);
    assert_arc_both_sides(&mut s, 1);
}

#[test]
fn an_arc_end_on_the_start_direction_previews_straight_and_refuses_commit() {
    let mut s = polyline();
    local_click(&mut s, 5.0, 7.0);
    local_click(&mut s, 15.0, 9.0);
    for segment in [2, 1] {
        let (b, e) = chain_end(&s);
        s.set_sketch_mode("tangent-arc");
        let (x, y) = (b.x.0 + 8.0 * e.0, b.y.0 + 8.0 * e.1);
        local_hover(&mut s, x, y);
        assert!(s.sketch.degenerate, "segment {segment}");
        assert!(
            s.sketch_degenerate_annotation()
                .is_some_and(|a| a.preview && a.conflicting),
            "marked degenerate"
        );
        assert!(
            s.sketch_construction_preview().is_some(),
            "straight preview"
        );
        assert!(preview_arc(&s).is_none());
        let history = s.undo_len();
        local_click(&mut s, x, y);
        assert_eq!(s.undo_len(), history, "segment {segment}: no commit");
        assert!(s.sketch.error.is_some());
        local_hover(&mut s, x, y + 3.0);
        assert!(!s.sketch.degenerate);
        assert!(preview_arc(&s).is_some());
        // Back to a fresh free start for the second round.
        s.set_sketch_mode("free-line");
        s.sketch_secondary_press();
        local_click(&mut s, 30.0, 31.0);
    }
}

#[test]
fn arc_preview_exists_on_both_sides_at_segments_2_3_5_8() {
    let mut s = polyline();
    local_click(&mut s, 0.0, 0.0);
    s.set_sketch_mode("free-line");
    local_click(&mut s, 12.0, 2.0);
    for segment in 2..=8 {
        if [2, 3, 5, 8].contains(&segment) {
            assert_arc_both_sides(&mut s, segment);
        }
        let (b, e) = chain_end(&s);
        let n = (-e.1, e.0);
        let side = if segment % 2 == 0 { 1.0 } else { -1.0 };
        if segment >= 4 && segment % 2 == 0 {
            // Later segments follow arcs, alternating their turning side.
            s.set_sketch_mode("tangent-arc");
            local_click(
                &mut s,
                b.x.0 + 6.0 * e.0 + side * 5.0 * n.0,
                b.y.0 + 6.0 * e.1 + side * 5.0 * n.1,
            );
            s.set_sketch_mode("free-line");
        } else {
            local_click(
                &mut s,
                b.x.0 + 9.0 * e.0 + side * 7.0 * n.0,
                b.y.0 + 9.0 * e.1 + side * 7.0 * n.1,
            );
        }
        assert_eq!(s.undo_len(), segment, "segment {segment} committed");
    }
}

#[test]
fn first_segment_line_modes_follow_the_x_axis_and_keep_their_relation() {
    let mut s = polyline();
    local_click(&mut s, 5.0, 7.0);
    s.set_sketch_mode("tangent-line");
    local_click(&mut s, 17.0, 11.0);
    s.set_sketch_mode("perpendicular-line");
    s.sketch_secondary_press();
    local_click(&mut s, 40.0, 7.0);
    s.sketch_complete_numeric("12 mm", "0 deg").unwrap();
    let snapshot = s.document.sketch(s.active_sketch().unwrap()).unwrap();
    let relations: Vec<_> = snapshot
        .graph
        .constraints
        .values()
        .map(|c| c.relation.clone())
        .collect();
    assert!(
        relations
            .iter()
            .any(|r| matches!(r, curvyo_sketch_core::Relation::Horizontal(_)))
    );
    assert!(
        relations
            .iter()
            .any(|r| matches!(r, curvyo_sketch_core::Relation::Vertical(_)))
    );
    let has = |x: f64, y: f64| {
        snapshot.solution.entities.values().any(|e| {
            matches!(e, Entity::Point { position, .. }
                if position.distance(Position::mm(x, y)).0 < 1e-6)
        })
    };
    assert!(
        has(17.0, 7.0),
        "tangent line projected onto the X direction"
    );
    assert!(has(40.0, 19.0), "typed perpendicular line along +Y");
}
