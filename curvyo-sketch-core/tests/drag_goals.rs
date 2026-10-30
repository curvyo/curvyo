//! Body translation constrains all defining point goals without changing intent.
#![allow(clippy::unwrap_used)]
use curvyo_sketch_core::{
    Constraint, Entity, Expression, Length, Position, Quantity, Relation, Sketch, SolveMode,
    create_ellipse, solve, validate_candidate,
};
use std::collections::BTreeMap;

#[test]
fn ellipse_body_translation_preserves_axes_with_multiple_goals() {
    let graph = create_ellipse(
        &Sketch::new("origin".into()),
        "e",
        Position::mm(20.0, 10.0),
        Position::mm(30.0, 10.0),
        Length(4.0),
    )
    .unwrap();
    let goals = ["e:center", "e:focus"]
        .into_iter()
        .map(|id| {
            let p = graph.point(&id.into()).unwrap();
            (id.into(), Position::mm(p.x.0 + 7.0, p.y.0 - 3.0))
        })
        .collect::<BTreeMap<_, _>>();
    let result = solve(
        &graph,
        &SolveMode::DragPoints {
            goals: goals.clone(),
        },
    )
    .unwrap()
    .candidate
    .unwrap();
    validate_candidate(&graph, &result).unwrap();
    for (id, expected) in goals {
        assert!(result.point(&id).unwrap().distance(expected).0 < 1e-6);
    }
    assert_eq!(
        result.entities[&"e:curve".into()],
        graph.entities[&"e:curve".into()]
    );
}

#[test]
fn unreachable_point_goal_moves_to_closest_feasible_radius_instead_of_freezing() {
    let mut graph = Sketch::new("origin".into());
    graph.entities.insert(
        "p".into(),
        Entity::Point {
            position: Position::mm(10.0, 0.0),
            fixed: false,
        },
    );
    graph.constraints.insert(
        "length".into(),
        Constraint::driving(Relation::Distance {
            a: "origin".into(),
            b: "p".into(),
            value: Expression::Literal(Quantity::length(10.0).unwrap()),
        }),
    );
    let result = solve(
        &graph,
        &SolveMode::DragPoints {
            goals: BTreeMap::from([("p".into(), Position::mm(0.0, 20.0))]),
        },
    )
    .unwrap();
    let candidate = result.candidate.unwrap();
    validate_candidate(&graph, &candidate).unwrap();
    assert!(
        candidate
            .point(&"p".into())
            .unwrap()
            .distance(Position::mm(0.0, 10.0))
            .0
            < 1e-4,
        "actual {:?}",
        candidate.point(&"p".into())
    );
}

#[test]
fn requested_seed_preserves_fixed_targets_and_repeated_curve_branch() {
    let mut graph = Sketch::new("origin".into());
    graph.entities.insert(
        "p".into(),
        Entity::Point {
            position: Position::mm(10.0, 0.0),
            fixed: false,
        },
    );
    graph.constraints.insert(
        "radius".into(),
        Constraint::driving(Relation::Distance {
            a: "origin".into(),
            b: "p".into(),
            value: Expression::Literal(Quantity::length(10.0).unwrap()),
        }),
    );
    let intent = graph.clone();
    for i in 0..20 {
        let angle = std::f64::consts::FRAC_PI_4 + f64::from(i) * 0.01;
        let target = Position::mm(20.0 * angle.cos(), 20.0 * angle.sin());
        graph = curvyo_sketch_core::solve_seeded(
            &intent,
            &graph,
            &SolveMode::DragPoints {
                goals: BTreeMap::from([("p".into(), target)]),
            },
        )
        .unwrap()
        .candidate
        .unwrap();
        validate_candidate(&intent, &graph).unwrap();
        let expected = Position::mm(10.0 * angle.cos(), 10.0 * angle.sin());
        assert!(graph.point(&"p".into()).unwrap().distance(expected).0 < 1e-4);
    }
    let mut fixed = intent;
    fixed.constraints.insert(
        "fixed".into(),
        Constraint::driving(Relation::Fixed("p".into())),
    );
    let result = solve(
        &fixed,
        &SolveMode::DragPoints {
            goals: BTreeMap::from([
                ("p".into(), Position::mm(20.0, 20.0)),
                ("origin".into(), Position::mm(30.0, 30.0)),
            ]),
        },
    )
    .unwrap()
    .candidate
    .unwrap();
    validate_candidate(&fixed, &result).unwrap();
    assert!(
        result
            .point(&"p".into())
            .unwrap()
            .distance(Position::mm(10.0, 0.0))
            .0
            < 1e-6
    );
    assert_eq!(
        result.point(&"origin".into()).unwrap(),
        Position::mm(0.0, 0.0)
    );
}

#[test]
fn ellipse_axis_handles_are_real_axis_points_and_preserve_the_center() {
    use curvyo_sketch_core::{SketchHandle, handle_positions};
    let graph = create_ellipse(
        &Sketch::new("origin".into()),
        "e",
        Position::mm(20.0, 20.0),
        Position::mm(30.0, 20.0),
        Length(4.0),
    )
    .unwrap();
    let handles = handle_positions(&graph, &"e:curve".into()).unwrap();
    assert!(
        handles[&SketchHandle::MajorAxis]
            .distance(Position::mm(30.0, 20.0))
            .0
            < 1e-6
    );
    assert!(
        handles[&SketchHandle::MinorAxis]
            .distance(Position::mm(20.0, 24.0))
            .0
            < 1e-6
    );
    for (handle, target) in [
        (SketchHandle::MajorAxis, Position::mm(34.0, 20.0)),
        (SketchHandle::MinorAxis, Position::mm(20.0, 26.0)),
    ] {
        let candidate = solve(
            &graph,
            &SolveMode::DragHandle {
                entity: "e:curve".into(),
                handle,
                target,
            },
        )
        .unwrap()
        .candidate
        .unwrap();
        validate_candidate(&graph, &candidate).unwrap();
        assert!(
            candidate
                .point(&"e:center".into())
                .unwrap()
                .distance(Position::mm(20.0, 20.0))
                .0
                < 1e-6
        );
        assert!(
            handle_positions(&candidate, &"e:curve".into()).unwrap()[&handle]
                .distance(target)
                .0
                < 1e-4
        );
    }
}
