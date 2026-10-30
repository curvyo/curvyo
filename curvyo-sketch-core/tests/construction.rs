//! Analytic construction retains topology through constrained edits.
#![allow(clippy::unwrap_used)]
use curvyo_sketch_core::{Entity, Length, Position, Sketch, create_slot, fillet, outline};
use kurbo::Shape;
#[test]
fn slot_is_closed_and_has_expected_area() {
    let seed = Sketch::new("origin".into());
    let slot = create_slot(
        &seed,
        "slot",
        Position::mm(0.0, 0.0),
        Position::mm(10.0, 0.0),
        Length(4.0),
    )
    .unwrap();
    let path = outline(&slot, Length(0.001)).unwrap();
    assert!((path.area().abs() - (40.0 + 4.0 * std::f64::consts::PI)).abs() < 0.01);
    assert!(matches!(
        path.elements().last(),
        Some(kurbo::PathEl::ClosePath)
    ));
}
#[test]
fn fillet_keeps_virtual_corner_and_rejects_radius_beyond_segments() {
    let mut seed = Sketch::new("origin".into());
    for (id, pos) in [
        ("left", Position::mm(10.0, 0.0)),
        ("up", Position::mm(0.0, 10.0)),
    ] {
        seed.entities.insert(
            id.into(),
            Entity::Point {
                position: pos,
                fixed: true,
            },
        );
    }
    seed.entities.insert(
        "a".into(),
        Entity::Line {
            start: "left".into(),
            end: "origin".into(),
        },
    );
    seed.entities.insert(
        "b".into(),
        Entity::Line {
            start: "origin".into(),
            end: "up".into(),
        },
    );
    let rounded = fillet(&seed, "round", &"a".into(), &"b".into(), Length(2.0)).unwrap();
    assert_eq!(
        rounded.point(&"origin".into()).unwrap(),
        Position::mm(0.0, 0.0)
    );
    assert!(rounded.construction.contains(&"a".into()));
    assert!(rounded.entities.contains_key(&"round:arc".into()));
    assert!(fillet(&seed, "too_big", &"a".into(), &"b".into(), Length(20.0)).is_err());
    assert!(!seed.construction.contains(&"a".into()));
}

#[test]
fn three_point_circle_and_ellipse_pass_through_the_picks() {
    use curvyo_sketch_core::{create_circle_three_points, create_ellipse_three_points};
    let seed = Sketch::new("origin".into());
    let circle = create_circle_three_points(
        &seed,
        "c",
        [
            Position::mm(2.0, 0.0),
            Position::mm(0.0, 2.0),
            Position::mm(-2.0, 0.0),
        ],
    )
    .unwrap();
    match &circle.entities[&"c:curve".into()] {
        Entity::Circle { radius, center, .. } => {
            assert!((radius.0 - 2.0).abs() < 1e-9);
            assert!(
                circle
                    .point(center)
                    .unwrap()
                    .distance(Position::mm(0.0, 0.0))
                    .0
                    < 1e-9
            );
        }
        _ => panic!("not circle"),
    }
    let ellipse = create_ellipse_three_points(
        &seed,
        "e",
        Position::mm(-5.0, 0.0),
        Position::mm(5.0, 0.0),
        Position::mm(3.0, 2.4),
    )
    .unwrap();
    match &ellipse.entities[&"e:curve".into()] {
        Entity::Ellipse { minor_radius, .. } => assert!((minor_radius.0 - 3.0).abs() < 1e-9),
        _ => panic!("not ellipse"),
    }
    assert!(
        create_circle_three_points(
            &seed,
            "bad",
            [
                Position::mm(0.0, 0.0),
                Position::mm(1.0, 0.0),
                Position::mm(2.0, 0.0)
            ]
        )
        .is_err()
    );
}

#[test]
fn rectangle_and_three_point_arc_keep_analytic_geometry() {
    use curvyo_sketch_core::{create_arc_three_points, create_rectangle};
    let seed = Sketch::new("origin".into());
    let rectangle =
        create_rectangle(&seed, "r", Position::mm(0.0, 0.0), Position::mm(10.0, 5.0)).unwrap();
    assert!((outline(&rectangle, Length(0.001)).unwrap().area().abs() - 50.0).abs() < 1e-9);
    let arc = create_arc_three_points(
        &seed,
        "a",
        [
            Position::mm(2.0, 0.0),
            Position::mm(0.0, 2.0),
            Position::mm(-2.0, 0.0),
        ],
    )
    .unwrap();
    match &arc.entities[&"a:curve".into()] {
        Entity::Arc { radius, .. } => assert!((radius.0 - 2.0).abs() < 1e-9),
        _ => panic!("not arc"),
    }
}

#[test]
fn explicit_coincidence_closes_loops_even_with_an_attached_spur() {
    use curvyo_sketch_core::{Constraint, Relation, SolveMode, create_polyline, solve};
    let seed = Sketch::new("origin".into());
    let mut s = create_polyline(
        &seed,
        "loop",
        &[
            Position::mm(0.0, 0.0),
            Position::mm(10.0, 0.0),
            Position::mm(10.0, 10.0),
            Position::mm(0.0, 10.0),
            Position::mm(0.0, 0.0),
        ],
        false,
    )
    .unwrap();
    s.constraints.insert(
        "join".into(),
        Constraint::driving(Relation::Coincident("loop:p4".into(), "loop:p0".into())),
    );
    s.entities.insert(
        "spur_end".into(),
        Entity::Point {
            position: Position::mm(-4.0, 0.0),
            fixed: false,
        },
    );
    s.entities.insert(
        "0_spur".into(),
        Entity::Line {
            start: "loop:p0".into(),
            end: "spur_end".into(),
        },
    );
    let solved = solve(&s, &SolveMode::Edit).unwrap().candidate.unwrap();
    let path = outline(&solved, Length(0.001)).unwrap();
    assert_eq!(
        path.elements()
            .iter()
            .filter(|e| matches!(e, kurbo::PathEl::ClosePath))
            .count(),
        1
    );
    let mut without_coincidence = solved;
    without_coincidence.constraints.clear();
    assert_eq!(
        outline(&without_coincidence, Length(0.001))
            .unwrap()
            .elements()
            .iter()
            .filter(|e| matches!(e, kurbo::PathEl::ClosePath))
            .count(),
        0
    );
}
