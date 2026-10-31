//! Construction intent stays separate from dependent solved geometry.
#![allow(clippy::unwrap_used)]
use curvyo_sketch_core::{Construction, Entity, Length, Position, Sketch, construct};

#[test]
fn construction_retains_explicit_intent_and_shared_segment_endpoints() {
    let seed = Sketch::new("origin".into());
    let first = construct(
        &seed,
        "first",
        &Construction::Segment {
            start: "origin".into(),
            end: "joint".into(),
            end_position: Some(Position::mm(10.0, 0.0)),
        },
    )
    .unwrap();
    let second = construct(
        &first.candidate,
        "second",
        &Construction::Segment {
            start: "joint".into(),
            end: "last".into(),
            end_position: Some(Position::mm(10.0, 10.0)),
        },
    )
    .unwrap();
    assert_eq!(
        second.intent.entities[&"second:line".into()],
        Entity::Line {
            start: "joint".into(),
            end: "last".into(),
        }
    );
    assert_eq!(second.intent.entities.len(), 5);
    let close = construct(
        &second.candidate,
        "close",
        &Construction::Segment {
            start: "last".into(),
            end: "origin".into(),
            end_position: None,
        },
    )
    .unwrap();
    assert_eq!(close.intent.entities.len(), 6);
    assert_eq!(
        close.intent.entities[&"origin".into()],
        seed.entities[&"origin".into()]
    );
}

#[test]
fn construction_intent_is_available_before_solver_changes_existing_geometry() {
    use curvyo_sketch_core::{Constraint, Expression, Quantity, Relation};
    let mut seed = Sketch::new("origin".into());
    seed.entities.insert(
        "dependent".into(),
        Entity::Point {
            position: Position::mm(10.0, 0.0),
            fixed: false,
        },
    );
    seed.constraints.insert(
        "distance".into(),
        Constraint::driving(Relation::Distance {
            a: "origin".into(),
            b: "dependent".into(),
            value: Expression::Literal(Quantity::length(20.0).unwrap()),
        }),
    );
    let operation = construct(
        &seed,
        "circle",
        &Construction::Circle {
            center: Position::mm(40.0, 10.0),
            radius: Length(5.0),
        },
    )
    .unwrap();
    assert_eq!(
        operation.intent.point(&"dependent".into()).unwrap(),
        Position::mm(10.0, 0.0)
    );
    assert!(
        (operation
            .candidate
            .point(&"dependent".into())
            .unwrap()
            .distance(Position::mm(0.0, 0.0))
            .0
            - 20.0)
            .abs()
            < 1e-6
    );
}

#[test]
fn deleting_geometry_removes_only_its_reference_dependants() {
    use curvyo_sketch_core::{Constraint, Relation, remove_entities};
    use std::collections::BTreeSet;
    let seed = curvyo_sketch_core::create_rectangle(
        &Sketch::new("origin".into()),
        "r",
        Position::mm(0.0, 0.0),
        Position::mm(10.0, 10.0),
    )
    .unwrap();
    let mut graph = seed;
    graph.constraints.insert(
        "keep".into(),
        Constraint::driving(Relation::Fixed("origin".into())),
    );
    let result = remove_entities(&graph, &BTreeSet::from(["r:p0".into()])).unwrap();
    assert!(!result.entities.contains_key(&"r:line0".into()));
    assert!(!result.entities.contains_key(&"r:line3".into()));
    assert!(result.entities.contains_key(&"r:line1".into()));
    assert!(result.constraints.contains_key(&"keep".into()));
    assert!(!result.constraints.contains_key(&"r:axis0".into()));
    assert!(result.constraints.contains_key(&"r:axis1".into()));
    assert!(remove_entities(&graph, &BTreeSet::from([graph.origin.clone()])).is_err());
    result.validate().unwrap();
}

#[test]
fn displayed_formula_uses_current_parameter_names_and_round_trips_units() {
    use curvyo_sketch_core::{Expression, Parameter, Parameters, Quantity};
    let mut parameters = Parameters::default();
    parameters.insert(
        "width".into(),
        Parameter::literal("plate_width", Quantity::length(80.0).unwrap()),
    );
    let expression = Expression::parse("plate_width / 2 + 3 mm", &parameters).unwrap();
    parameters.get_mut(&"width".into()).unwrap().name = "width".into();
    let text = expression.display_text(&parameters).unwrap();
    assert!(text.contains("width"));
    assert!(!text.contains("plate_width"));
    assert_eq!(
        Expression::parse(&text, &parameters)
            .unwrap()
            .evaluate(&parameters.evaluate().unwrap())
            .unwrap()
            .value(),
        43.0
    );
}

#[test]
fn ellipse_variants_preserve_tall_and_near_circular_pick_geometry() {
    for second_axis in [3.0, 2.0, 2.0 + 1e-10] {
        let seed = Sketch::new("origin".into());
        for command in [
            Construction::Ellipse {
                center: Position::mm(0.0, 0.0),
                axis_end: Position::mm(2.0, 0.0),
                second_axis: Length(second_axis),
            },
            Construction::EllipseThreePoints([
                Position::mm(-2.0, 0.0),
                Position::mm(2.0, 0.0),
                Position::mm(0.0, second_axis),
            ]),
        ] {
            let result = construct(&seed, "ellipse", &command).unwrap();
            let Entity::Ellipse {
                center,
                focus,
                minor_radius,
                ..
            } = &result.candidate.entities[&"ellipse:curve".into()]
            else {
                panic!("ellipse");
            };
            let center = result.candidate.point(center).unwrap();
            let focus = result.candidate.point(focus).unwrap();
            let focal = center.distance(focus).0;
            let major = focal.hypot(minor_radius.0);
            assert!((major - second_axis.max(2.0)).abs() < 1e-8);
            assert!((minor_radius.0 - second_axis.min(2.0)).abs() < 1e-8);
            if second_axis > 2.0 + 1e-9 {
                assert!(focus.x.0.abs() < 1e-8);
                assert!((focus.y.0 - 5.0_f64.sqrt()).abs() < 1e-8);
            }
        }
    }
}
