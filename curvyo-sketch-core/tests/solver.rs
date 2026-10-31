//! Independent behavior checks for typed sketch inputs.
#![allow(clippy::unwrap_used, clippy::float_cmp)]
use curvyo_sketch_core::{
    Constraint, Entity, EntityId, Expression, Length, Position, Quantity, Relation, Sketch,
    SolveMode, SolveStatus, solve,
};
fn line() -> Sketch {
    let mut s = Sketch::new("origin".into());
    s.entities.insert(
        "end".into(),
        Entity::Point {
            position: Position::mm(8.0, 2.0),
            fixed: false,
        },
    );
    s.entities.insert(
        "line".into(),
        Entity::Line {
            start: "origin".into(),
            end: "end".into(),
        },
    );
    s.constraints.insert(
        "horizontal".into(),
        Constraint::driving(Relation::Horizontal("line".into())),
    );
    s.constraints.insert(
        "length".into(),
        Constraint::driving(Relation::Length {
            curve: "line".into(),
            value: Expression::Literal(Quantity::length(10.0).unwrap()),
        }),
    );
    s
}
#[test]
fn fully_constrained_line_has_known_coordinates() {
    let input = line();
    let result = solve(&input, &SolveMode::Edit).unwrap();
    assert_eq!(result.status, SolveStatus::FullyConstrained);
    let candidate = result.candidate.unwrap();
    let p = candidate.point(&EntityId::from("end")).unwrap();
    assert!((p.x.0 - 10.0).abs() < 1e-6 && p.y.0.abs() < 1e-6);
    assert_eq!(input.point(&"end".into()).unwrap(), Position::mm(8.0, 2.0));
}
#[test]
fn conflicts_return_original_ids_without_candidate() {
    let mut s = line();
    s.constraints.insert(
        "conflict".into(),
        Constraint::driving(Relation::Length {
            curve: "line".into(),
            value: Expression::Literal(Quantity::length(12.0).unwrap()),
        }),
    );
    let result = solve(&s, &SolveMode::Edit).unwrap();
    assert_eq!(result.status, SolveStatus::Conflicting);
    assert!(result.candidate.is_none());
    assert!(result.conflicting.contains(&"length".into()));
    assert!(result.conflicting.contains(&"conflict".into()));
}
#[test]
fn free_circle_has_three_degrees_and_invalid_radius_is_rejected() {
    let mut s = Sketch::new("origin".into());
    s.entities.insert(
        "c".into(),
        Entity::Point {
            position: Position::mm(4.0, 5.0),
            fixed: false,
        },
    );
    s.entities.insert(
        "circle".into(),
        Entity::Circle {
            center: "c".into(),
            radius: Length(3.0),
            fixed_radius: false,
        },
    );
    assert_eq!(
        solve(&s, &SolveMode::Edit).unwrap().status,
        SolveStatus::UnderConstrained(3)
    );
    s.entities.insert(
        "circle".into(),
        Entity::Circle {
            center: "c".into(),
            radius: Length(f64::NAN),
            fixed_radius: false,
        },
    );
    assert!(solve(&s, &SolveMode::Edit).is_err());
}
#[test]
fn drag_cannot_break_hard_dimensions() {
    let s = line();
    let result = solve(
        &s,
        &SolveMode::Drag {
            point: "end".into(),
            target: Position::mm(30.0, 30.0),
        },
    )
    .unwrap();
    let p = result.candidate.unwrap().point(&"end".into()).unwrap();
    assert!((p.x.0 - 10.0).abs() < 1e-6 && p.y.0.abs() < 1e-6);
}

#[test]
fn off_axis_ellipse_circle_tangency_uses_one_contact_and_original_identity() {
    let mut s = Sketch::new("origin".into());
    s.entities.insert(
        "focus".into(),
        Entity::Point {
            position: Position::mm(4.0, 0.0),
            fixed: true,
        },
    );
    s.entities.insert(
        "ellipse".into(),
        Entity::Ellipse {
            center: "origin".into(),
            focus: "focus".into(),
            minor_radius: Length(3.0),
            fixed_radius: true,
        },
    );
    let q = Position::mm(5.0 / 2.0_f64.sqrt(), 3.0 / 2.0_f64.sqrt());
    let normal = (q.x.0 / 25.0, q.y.0 / 9.0);
    let norm = normal.0.hypot(normal.1);
    let center = Position::mm(q.x.0 + 2.0 * normal.0 / norm, q.y.0 + 2.0 * normal.1 / norm);
    s.entities.insert(
        "center".into(),
        Entity::Point {
            position: center,
            fixed: true,
        },
    );
    s.entities.insert(
        "circle".into(),
        Entity::Circle {
            center: "center".into(),
            radius: Length(1.8),
            fixed_radius: false,
        },
    );
    s.constraints.insert(
        "tangent".into(),
        Constraint::driving(Relation::Tangent("ellipse".into(), "circle".into())),
    );
    let result = solve(&s, &SolveMode::Edit).unwrap();
    assert_eq!(result.status, SolveStatus::FullyConstrained);
    let candidate = result.candidate.unwrap();
    assert_eq!(candidate.entities.len(), s.entities.len());
    match candidate.entities.get(&"circle".into()).unwrap() {
        Entity::Circle { radius, .. } => assert!((radius.0 - 2.0).abs() < 1e-6),
        _ => panic!("wrong geometry"),
    }
    assert!(result.redundant.is_empty());
}

#[test]
fn generic_tangent_uses_line_carrier_beyond_visible_segment() {
    let mut s = Sketch::new("origin".into());
    for (id, position) in [
        ("end", Position::mm(1.0, 0.0)),
        ("center", Position::mm(5.0, 2.0)),
    ] {
        s.entities.insert(
            id.into(),
            Entity::Point {
                position,
                fixed: true,
            },
        );
    }
    s.entities.insert(
        "segment".into(),
        Entity::Line {
            start: "origin".into(),
            end: "end".into(),
        },
    );
    s.entities.insert(
        "circle".into(),
        Entity::Circle {
            center: "center".into(),
            radius: Length(1.5),
            fixed_radius: false,
        },
    );
    s.constraints.insert(
        "tangent".into(),
        Constraint::driving(Relation::Tangent("segment".into(), "circle".into())),
    );
    let solved = solve(&s, &SolveMode::Edit).unwrap();
    assert_eq!(solved.status, SolveStatus::FullyConstrained);
    match &solved.candidate.unwrap().entities[&"circle".into()] {
        Entity::Circle { radius, .. } => assert!((radius.0 - 2.0).abs() < 1e-6),
        _ => panic!("not circle"),
    }
}

#[test]
fn fixed_radius_with_free_center_is_not_a_fully_constrained_circle() {
    let mut sketch = Sketch::new("origin".into());
    sketch.entities.insert(
        "center".into(),
        Entity::Point {
            position: Position::mm(10.0, 20.0),
            fixed: false,
        },
    );
    sketch.entities.insert(
        "circle".into(),
        Entity::Circle {
            center: "center".into(),
            radius: Length(6.0),
            fixed_radius: true,
        },
    );
    let result = solve(&sketch, &SolveMode::Edit).unwrap();
    assert_eq!(result.status, SolveStatus::UnderConstrained(2));
    assert!(!result.fully_constrained.unwrap().contains(&"circle".into()));
    sketch.entities.insert(
        "center".into(),
        Entity::Point {
            position: Position::mm(10.0, 20.0),
            fixed: true,
        },
    );
    let result = solve(&sketch, &SolveMode::Edit).unwrap();
    assert!(result.fully_constrained.unwrap().contains(&"circle".into()));
}

#[test]
fn line_classification_requires_both_endpoint_variables_to_be_fixed() {
    let mut sketch = line();
    sketch.entities.insert(
        "origin".into(),
        Entity::Point {
            position: Position::mm(0.0, 0.0),
            fixed: false,
        },
    );
    sketch.constraints.remove(&"horizontal".into());
    let result = solve(&sketch, &SolveMode::Edit).unwrap();
    assert_eq!(result.status, SolveStatus::UnderConstrained(3));
    assert!(!result.fully_constrained.unwrap().contains(&"line".into()));
    let result = solve(&line(), &SolveMode::Edit).unwrap();
    assert_eq!(result.status, SolveStatus::FullyConstrained);
    assert!(result.fully_constrained.unwrap().contains(&"line".into()));
}
