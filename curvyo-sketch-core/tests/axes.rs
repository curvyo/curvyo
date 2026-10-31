//! Fixed local axes remain independent of the editable origin entity.
#![allow(clippy::unwrap_used)]
use curvyo_sketch_core::*;
#[test]
fn local_axis_relations_solve_without_following_moved_origin() {
    for axis in [Axis::X, Axis::Y] {
        let mut graph = Sketch::new("origin".into());
        graph.entities.insert(
            "origin".into(),
            Entity::Point {
                position: Position::mm(30.0, 40.0),
                fixed: true,
            },
        );
        graph.entities.insert(
            "p".into(),
            Entity::Point {
                position: Position::mm(3.0, 4.0),
                fixed: false,
            },
        );
        graph.constraints.insert(
            "axis".into(),
            Constraint::driving(Relation::OnAxis {
                point: "p".into(),
                axis,
            }),
        );
        let solved = solve(&graph, &SolveMode::Edit).unwrap().candidate.unwrap();
        let p = solved.point(&"p".into()).unwrap();
        assert!(match axis {
            Axis::X => p.y.0.abs() < 1e-6,
            Axis::Y => p.x.0.abs() < 1e-6,
        });
        assert_eq!(
            solved.point(&"origin".into()).unwrap(),
            Position::mm(30.0, 40.0)
        );
        validate_candidate(&graph, &solved).unwrap();
    }
}

#[test]
fn intersection_has_two_constraints_and_axis_records_reject_reference_mode() {
    let mut graph = Sketch::new("o".into());
    graph.entities.insert(
        "p".into(),
        Entity::Point {
            position: Position::mm(3.0, 4.0),
            fixed: false,
        },
    );
    for (id, axis) in [("x", Axis::X), ("y", Axis::Y)] {
        graph.constraints.insert(
            id.into(),
            Constraint::driving(Relation::OnAxis {
                point: "p".into(),
                axis,
            }),
        );
    }
    let result = solve(&graph, &SolveMode::Edit).unwrap();
    assert_eq!(result.status, SolveStatus::FullyConstrained);
    assert!(
        result
            .candidate
            .unwrap()
            .point(&"p".into())
            .unwrap()
            .distance(Position::mm(0.0, 0.0))
            .0
            < 1e-6
    );
    graph.constraints.get_mut(&"x".into()).unwrap().reference = true;
    assert!(graph.validate().is_err());
}

#[test]
fn automatic_axis_implication_uses_constraints_never_free_seed_alignment() {
    let mut graph = Sketch::new("o".into());
    graph.entities.insert(
        "p".into(),
        Entity::Point {
            position: Position::mm(10.0, 0.0),
            fixed: false,
        },
    );
    graph.entities.insert(
        "line".into(),
        Entity::Line {
            start: "o".into(),
            end: "p".into(),
        },
    );
    assert!(!curvyo_sketch_core::axis_is_implied(
        &graph,
        &"p".into(),
        Axis::X
    ));
    graph.constraints.insert(
        "horizontal".into(),
        Constraint::driving(Relation::Horizontal("line".into())),
    );
    assert!(curvyo_sketch_core::axis_is_implied(
        &graph,
        &"p".into(),
        Axis::X
    ));
    assert!(!curvyo_sketch_core::axis_is_implied(
        &graph,
        &"p".into(),
        Axis::Y
    ));
    let before = graph.clone();
    assert!(curvyo_sketch_core::axis_is_implied(
        &graph,
        &"p".into(),
        Axis::X
    ));
    assert_eq!(graph, before);
}
