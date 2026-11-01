//! Revised picking: a point within 10 px wins over a nearer edge (0066 AC50), else the nearest edge.
#![allow(clippy::unwrap_used)]
use curvyo_sketch_core::{Entity, Length, Position, Sketch};
#[test]
fn a_point_within_reach_wins_over_a_nearer_line_and_the_line_wins_beyond() {
    // Superseded by 0066 AC50 (revision 5): the point 5 px away now wins over the line under
    // the pointer; a point 11 px away leaves the line (1 mm = 1 px, edge tolerance 6 px).
    let mut graph = Sketch::new("o".into());
    for (id, x, y) in [
        ("a", 10.0, 10.0),
        ("b", 30.0, 10.0),
        ("p", 20.0, 15.0),
        ("r", 40.0, 30.0),
        ("s", 80.0, 30.0),
        ("q", 60.0, 41.0),
    ] {
        graph.entities.insert(
            id.into(),
            Entity::Point {
                position: Position::mm(x, y),
                fixed: false,
            },
        );
    }
    for (id, start, end) in [("line", "a", "b"), ("far", "r", "s")] {
        graph.entities.insert(
            id.into(),
            Entity::Line {
                start: start.into(),
                end: end.into(),
            },
        );
    }
    let hit = |x: f64, y: f64| {
        curvyo_ui_core::sketch_hit(&graph, &graph, Position::mm(x, y), Length(1.0))
            .unwrap()
            .entity
            .0
    };
    assert_eq!(hit(20.0, 10.0), "p");
    assert_eq!(hit(60.0, 30.0), "far");
}

#[test]
fn crossing_and_containment_use_curve_geometry_not_just_control_points() {
    let mut graph = Sketch::new("o".into());
    for (id, x, y) in [("a", -10.0, 0.0), ("b", 10.0, 0.0), ("c", 0.0, 10.0)] {
        graph.entities.insert(
            id.into(),
            Entity::Point {
                position: Position::mm(x, y),
                fixed: false,
            },
        );
    }
    graph.entities.insert(
        "line".into(),
        Entity::Line {
            start: "a".into(),
            end: "b".into(),
        },
    );
    graph.entities.insert(
        "circle".into(),
        Entity::Circle {
            center: "c".into(),
            radius: Length(5.0),
            fixed_radius: false,
        },
    );
    let crossing = curvyo_ui_core::sketch_marquee(
        &graph,
        &graph,
        Position::mm(-1.0, -1.0),
        Position::mm(1.0, 1.0),
        false,
    );
    assert!(crossing.contains(&"line".into()));
    assert!(!crossing.contains(&"a".into()));
    let contained = curvyo_ui_core::sketch_marquee(
        &graph,
        &graph,
        Position::mm(-1.0, -1.0),
        Position::mm(1.0, 1.0),
        true,
    );
    assert!(!contained.contains(&"line".into()));
    let circle_edge = curvyo_ui_core::sketch_marquee(
        &graph,
        &graph,
        Position::mm(4.0, 9.0),
        Position::mm(6.0, 11.0),
        false,
    );
    assert!(circle_edge.contains(&"circle".into()));
    assert!(!circle_edge.contains(&"c".into()));
}

#[test]
fn equidistant_overlapping_curves_have_stable_cycle_candidates() {
    let mut graph = Sketch::new("o".into());
    for (id, x, y) in [("a", 10.0, 10.0), ("b", 30.0, 10.0)] {
        graph.entities.insert(
            id.into(),
            Entity::Point {
                position: Position::mm(x, y),
                fixed: false,
            },
        );
    }
    for id in ["line-b", "line-a"] {
        graph.entities.insert(
            id.into(),
            Entity::Line {
                start: "a".into(),
                end: "b".into(),
            },
        );
    }
    let hits = curvyo_ui_core::sketch_hits(&graph, &graph, Position::mm(20.0, 10.0), Length(0.5));
    assert_eq!(
        hits.iter().map(|h| h.entity.0.as_str()).collect::<Vec<_>>(),
        ["line-a", "line-b"]
    );
}
