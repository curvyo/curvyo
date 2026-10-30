//! Reach certification preserves the complete graph rather than assuming a test-shaped chain.
#![allow(clippy::unwrap_used)]
use curvyo_sketch_core::{
    Constraint, Entity, Expression, Position, Quantity, Relation, Sketch, SolveMode, solve,
    validate_candidate,
};

fn graph() -> Sketch {
    let mut graph = Sketch::new("fixed".into());
    for (id, position) in [
        ("joint", Position::mm(3.0, 4.0)),
        ("tip", Position::mm(6.0, 8.0)),
        ("unrelated", Position::mm(99.0, 101.0)),
    ] {
        graph.entities.insert(
            id.into(),
            Entity::Point {
                position,
                fixed: false,
            },
        );
    }
    for (id, a, b) in [("a", "fixed", "joint"), ("b", "joint", "tip")] {
        graph.constraints.insert(
            id.into(),
            Constraint::driving(Relation::Distance {
                a: a.into(),
                b: b.into(),
                value: Expression::Literal(Quantity::length(5.0).unwrap()),
            }),
        );
    }
    graph
}
#[test]
fn unreachable_chain_attains_proven_bound_without_moving_disconnected_point() {
    let graph = graph();
    let candidate = solve(
        &graph,
        &SolveMode::Drag {
            point: "tip".into(),
            target: Position::mm(20.0, 0.0),
        },
    )
    .unwrap()
    .candidate
    .unwrap();
    validate_candidate(&graph, &candidate).unwrap();
    assert!(
        candidate
            .point(&"tip".into())
            .unwrap()
            .distance(Position::mm(10.0, 0.0))
            .0
            < 1e-6
    );
    assert!(
        candidate
            .point(&"joint".into())
            .unwrap()
            .distance(Position::mm(5.0, 0.0))
            .0
            < 1e-6
    );
    assert_eq!(
        candidate.entities[&"unrelated".into()],
        graph.entities[&"unrelated".into()]
    );
}
#[test]
fn extra_fixed_joint_cannot_be_moved_by_chain_certificate() {
    let mut graph = graph();
    graph.constraints.insert(
        "joint-fixed".into(),
        Constraint::driving(Relation::Fixed("joint".into())),
    );
    let candidate = solve(
        &graph,
        &SolveMode::Drag {
            point: "tip".into(),
            target: Position::mm(20.0, 0.0),
        },
    )
    .unwrap()
    .candidate
    .unwrap();
    validate_candidate(&graph, &candidate).unwrap();
    assert_eq!(
        candidate.point(&"joint".into()).unwrap(),
        Position::mm(3.0, 4.0)
    );
    assert!(
        (candidate
            .point(&"tip".into())
            .unwrap()
            .distance(Position::mm(3.0, 4.0))
            .0
            - 5.0)
            .abs()
            < 1e-6
    );
}
#[test]
fn extra_horizontal_relationship_refuses_incompatible_reach_pose() {
    let mut graph = graph();
    graph.entities.insert(
        "tip".into(),
        Entity::Point {
            position: Position::mm(8.0, 4.0),
            fixed: false,
        },
    );
    graph.entities.insert(
        "edge".into(),
        Entity::Line {
            start: "joint".into(),
            end: "tip".into(),
        },
    );
    graph.constraints.insert(
        "joint-fixed".into(),
        Constraint::driving(Relation::Fixed("joint".into())),
    );
    graph.constraints.insert(
        "horizontal".into(),
        Constraint::driving(Relation::Horizontal("edge".into())),
    );
    let candidate = solve(
        &graph,
        &SolveMode::Drag {
            point: "tip".into(),
            target: Position::mm(20.0, 0.0),
        },
    )
    .unwrap()
    .candidate
    .unwrap();
    validate_candidate(&graph, &candidate).unwrap();
    assert!(
        candidate
            .point(&"tip".into())
            .unwrap()
            .distance(Position::mm(8.0, 4.0))
            .0
            < 1e-6
    );
}
