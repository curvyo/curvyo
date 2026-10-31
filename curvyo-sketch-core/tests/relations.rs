//! Every user relation has an independent analytic outcome.
#![allow(clippy::unwrap_used)]
use curvyo_sketch_core::*;
fn point(s: &mut Sketch, id: &str, x: f64, y: f64, fixed: bool) {
    s.entities.insert(
        id.into(),
        Entity::Point {
            position: Position::mm(x, y),
            fixed,
        },
    );
}
fn line(s: &mut Sketch, id: &str, a: &str, b: &str) {
    s.entities.insert(
        id.into(),
        Entity::Line {
            start: a.into(),
            end: b.into(),
        },
    );
}
fn length(value: f64) -> Expression {
    Expression::Literal(Quantity::length(value).unwrap())
}
fn add(s: &mut Sketch, id: &str, relation: Relation) {
    s.constraints
        .insert(id.into(), Constraint::driving(relation));
}
fn assert_point(s: &Sketch, id: &str, x: f64, y: f64) {
    assert!(s.point(&id.into()).unwrap().distance(Position::mm(x, y)).0 < 1e-6);
}
#[test]
fn geometric_relation_matrix() {
    for kind in [
        "coincident",
        "horizontal",
        "vertical",
        "parallel",
        "perpendicular",
        "angle",
        "equal",
        "fixed",
        "symmetric",
        "midpoint",
        "distance",
    ] {
        let mut s = Sketch::new("origin".into());
        point(&mut s, "p", 8.0, 2.0, false);
        point(&mut s, "ref", 10.0, 0.0, true);
        line(&mut s, "a", "origin", "ref");
        line(&mut s, "b", "origin", "p");
        let (relation, expected) = match kind {
            "coincident" => (
                Relation::Coincident("origin".into(), "p".into()),
                (0.0, 0.0),
            ),
            "horizontal" => (Relation::Horizontal("b".into()), (10.0, 0.0)),
            "vertical" => (Relation::Vertical("b".into()), (0.0, 10.0)),
            "parallel" => (Relation::Parallel("a".into(), "b".into()), (10.0, 0.0)),
            "perpendicular" => (Relation::Perpendicular("a".into(), "b".into()), (0.0, 10.0)),
            "angle" => (
                Relation::Angle {
                    a: "a".into(),
                    b: "b".into(),
                    value: Expression::Literal(
                        Quantity::angle(std::f64::consts::FRAC_PI_4).unwrap(),
                    ),
                },
                (50.0_f64.sqrt(), 50.0_f64.sqrt()),
            ),
            "equal" => {
                add(&mut s, "h", Relation::Horizontal("b".into()));
                (Relation::Equal("a".into(), "b".into()), (10.0, 0.0))
            }
            "fixed" => (Relation::Fixed("p".into()), (8.0, 2.0)),
            "symmetric" => {
                point(&mut s, "source", 3.0, 2.0, true);
                (
                    Relation::Symmetric {
                        source: "source".into(),
                        image: "p".into(),
                        axis: "a".into(),
                    },
                    (3.0, -2.0),
                )
            }
            "midpoint" => (
                Relation::Midpoint {
                    point: "p".into(),
                    line: "a".into(),
                },
                (5.0, 0.0),
            ),
            _ => {
                add(&mut s, "h", Relation::Horizontal("b".into()));
                (
                    Relation::Distance {
                        a: "origin".into(),
                        b: "p".into(),
                        value: length(10.0),
                    },
                    (10.0, 0.0),
                )
            }
        };
        if [
            "horizontal",
            "vertical",
            "parallel",
            "perpendicular",
            "angle",
        ]
        .contains(&kind)
        {
            add(
                &mut s,
                "length",
                Relation::Length {
                    curve: "b".into(),
                    value: length(10.0),
                },
            );
        }
        add(&mut s, "relation", relation);
        let solved = solve(&s, &SolveMode::Edit).unwrap();
        assert_eq!(
            solved.status,
            SolveStatus::FullyConstrained,
            "{kind}: {:?}",
            solved.message
        );
        assert_point(&solved.candidate.unwrap(), "p", expected.0, expected.1);
    }
}
#[test]
fn radius_diameter_reference_and_duplicate_diagnostics() {
    for diameter in [false, true] {
        let mut s = Sketch::new("origin".into());
        s.entities.insert(
            "circle".into(),
            Entity::Circle {
                center: "origin".into(),
                radius: Length(3.0),
                fixed_radius: false,
            },
        );
        let relation = if diameter {
            Relation::Diameter {
                curve: "circle".into(),
                value: length(10.0),
            }
        } else {
            Relation::Radius {
                curve: "circle".into(),
                value: length(5.0),
            }
        };
        add(&mut s, "dimension", relation.clone());
        add(&mut s, "duplicate", relation.clone());
        let solved = solve(&s, &SolveMode::Edit).unwrap();
        assert_eq!(solved.status, SolveStatus::FullyConstrained);
        assert_eq!(solved.redundant.len(), 1);
        assert!(
            (measure(&solved.candidate.unwrap(), &relation)
                .unwrap()
                .value()
                - if diameter { 10.0 } else { 5.0 })
            .abs()
                < 1e-6
        );
        s.constraints.clear();
        s.constraints.insert(
            "reference".into(),
            Constraint {
                relation: relation.clone(),
                reference: true,
            },
        );
        let solved = solve(&s, &SolveMode::Edit).unwrap();
        assert_eq!(solved.status, SolveStatus::UnderConstrained(1));
        assert!(
            (measure(&solved.candidate.unwrap(), &relation)
                .unwrap()
                .value()
                - if diameter { 6.0 } else { 3.0 })
            .abs()
                < 1e-6
        );
    }
}
#[test]
fn named_parameter_edit_drives_geometry_without_reparsing() {
    let mut s = create_rectangle(
        &Sketch::new("origin".into()),
        "r",
        Position::mm(0.0, 0.0),
        Position::mm(8.0, 4.0),
    )
    .unwrap();
    s.parameters.insert(
        "width-id".into(),
        Parameter::literal("width", Quantity::length(20.0).unwrap()),
    );
    let expression = Expression::parse("width", &s.parameters).unwrap();
    add(
        &mut s,
        "width",
        Relation::Length {
            curve: "r:line0".into(),
            value: expression,
        },
    );
    let solved = solve(&s, &SolveMode::Edit).unwrap().candidate.unwrap();
    assert!(
        (solved
            .point(&"r:p0".into())
            .unwrap()
            .distance(solved.point(&"r:p1".into()).unwrap())
            .0
            - 20.0)
            .abs()
            < 1e-6
    );
}
