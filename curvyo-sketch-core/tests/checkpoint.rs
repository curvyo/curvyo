//! Checkpoints preserve authoritative intent while seeding free geometry.
#![allow(clippy::unwrap_used)]
use curvyo_sketch_core::{
    Constraint, Entity, Position, Relation, Sketch, SolveMode, solve_seeded, validate_candidate,
};

fn graph() -> Sketch {
    let mut graph = Sketch::new("origin".into());
    graph.entities.insert(
        "free".into(),
        Entity::Point {
            position: Position::mm(10.0, 2.0),
            fixed: false,
        },
    );
    graph
}

#[test]
fn checkpoint_validation_rejects_modified_intent_and_fixed_geometry() {
    let intent = graph();
    let mut candidate = intent.clone();
    candidate.entities.insert(
        "origin".into(),
        Entity::Point {
            position: Position::mm(1.0, 0.0),
            fixed: true,
        },
    );
    assert!(validate_candidate(&intent, &candidate).is_err());
    candidate = intent.clone();
    candidate.construction.insert("free".into());
    assert!(validate_candidate(&intent, &candidate).is_err());
    candidate = intent.clone();
    candidate.entities.insert(
        "free".into(),
        Entity::Point {
            position: Position::mm(30.0, 4.0),
            fixed: false,
        },
    );
    assert!(validate_candidate(&intent, &candidate).is_ok());
}

#[test]
fn checkpoint_seeds_free_geometry_without_replacing_fixed_targets() {
    let mut intent = graph();
    intent.constraints.insert(
        "fixed".into(),
        Constraint::driving(Relation::Fixed("free".into())),
    );
    let mut seed = intent.clone();
    seed.entities.insert(
        "free".into(),
        Entity::Point {
            position: Position::mm(90.0, 60.0),
            fixed: false,
        },
    );
    seed.entities.insert(
        "origin".into(),
        Entity::Point {
            position: Position::mm(50.0, 50.0),
            fixed: true,
        },
    );
    let result = solve_seeded(&intent, &seed, &SolveMode::Edit).unwrap();
    let candidate = result.candidate.unwrap();
    assert!(
        candidate
            .point(&"free".into())
            .unwrap()
            .distance(Position::mm(10.0, 2.0))
            .0
            < 1e-6
    );
    assert_eq!(
        candidate.point(&"origin".into()).unwrap(),
        Position::mm(0.0, 0.0)
    );
    intent.constraints.clear();
    let result = solve_seeded(&intent, &seed, &SolveMode::Edit).unwrap();
    assert!(
        result
            .candidate
            .unwrap()
            .point(&"free".into())
            .unwrap()
            .distance(Position::mm(90.0, 60.0))
            .0
            < 1e-6
    );
}
