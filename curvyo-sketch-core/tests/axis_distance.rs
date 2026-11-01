//! Horizontal and vertical distance dimensions of customer revision 4 (0066 AC44-AC46): the
//! projection on the measured axis equals the value within 1e-6 mm, the other axis stays free,
//! the side is kept and a negative value is refused.
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

fn dof(s: &Sketch) -> (usize, Sketch) {
    let result = solve(s, &SolveMode::Edit).unwrap();
    let dof = match result.status {
        SolveStatus::UnderConstrained(n) => n,
        SolveStatus::FullyConstrained => 0,
        other => panic!("{other:?}: {:?}", result.message),
    };
    (dof, result.candidate.unwrap())
}

fn at(s: &Sketch, id: &str) -> Position {
    s.point(&id.into()).unwrap()
}

fn mm(value: f64) -> Expression {
    Expression::Literal(Quantity::length(value).unwrap())
}

/// A diagonal line from the fixed point `a` (2, 3) to the free point `b` (12, 9).
fn diagonal() -> Sketch {
    let mut s = Sketch::new("origin".into());
    point(&mut s, "a", 2.0, 3.0, true);
    point(&mut s, "b", 12.0, 9.0, false);
    s.entities.insert(
        "l".into(),
        Entity::Line {
            start: "a".into(),
            end: "b".into(),
        },
    );
    s
}

fn with(s: &Sketch, relation: Relation) -> Sketch {
    let mut s = s.clone();
    s.constraints
        .insert("d".into(), Constraint::driving(relation));
    s
}

#[test]
fn a_horizontal_distance_sets_the_x_projection_and_leaves_y_free() {
    let s = diagonal();
    let (before, _) = dof(&s);
    let (after, solved) = dof(&with(
        &s,
        Relation::HorizontalDistance {
            a: "a".into(),
            b: "b".into(),
            value: mm(30.0),
        },
    ));
    assert_eq!(before - after, 1);
    let (a, b) = (at(&solved, "a"), at(&solved, "b"));
    assert!((b.x.0 - a.x.0 - 30.0).abs() < 1e-6, "{a:?} {b:?}");
    assert!(
        (b.y.0 - 9.0).abs() < 1e-6,
        "vertical component unchanged: {b:?}"
    );
}

#[test]
fn a_vertical_distance_sets_the_y_projection_and_leaves_x_free() {
    let s = diagonal();
    let (before, _) = dof(&s);
    let (after, solved) = dof(&with(
        &s,
        Relation::VerticalDistance {
            a: "a".into(),
            b: "b".into(),
            value: mm(30.0),
        },
    ));
    assert_eq!(before - after, 1);
    let b = at(&solved, "b");
    assert!((b.y.0 - 33.0).abs() < 1e-6, "{b:?}");
    assert!((b.x.0 - 12.0).abs() < 1e-6, "{b:?}");
}

#[test]
fn the_side_is_kept_and_zero_aligns_the_points() {
    // `b` left of `a`: stored with the left point first, a new value keeps `b` on the left.
    let mut s = Sketch::new("origin".into());
    point(&mut s, "a", 20.0, 0.0, true);
    point(&mut s, "b", 5.0, 4.0, false);
    let relation = |value| Relation::HorizontalDistance {
        a: "b".into(),
        b: "a".into(),
        value: mm(value),
    };
    let (_, solved) = dof(&with(&s, relation(8.0)));
    assert!((at(&solved, "b").x.0 - 12.0).abs() < 1e-6);
    let (_, solved) = dof(&with(&s, relation(0.0)));
    assert!((at(&solved, "b").x.0 - 20.0).abs() < 1e-6);
}

#[test]
fn a_negative_value_is_refused() {
    let s = with(
        &diagonal(),
        Relation::VerticalDistance {
            a: "a".into(),
            b: "b".into(),
            value: mm(-1.0),
        },
    );
    assert!(s.validate().is_err());
}

#[test]
fn a_point_measured_to_itself_is_refused() {
    for relation in [
        Relation::HorizontalDistance {
            a: "a".into(),
            b: "a".into(),
            value: mm(0.0),
        },
        Relation::VerticalDistance {
            a: "b".into(),
            b: "b".into(),
            value: mm(3.0),
        },
    ] {
        assert!(with(&diagonal(), relation).validate().is_err());
    }
}

#[test]
fn a_point_to_the_origin_measures_its_coordinate() {
    let mut s = Sketch::new("origin".into());
    point(&mut s, "p", 7.0, 4.0, false);
    let relation = Relation::HorizontalDistance {
        a: "origin".into(),
        b: "p".into(),
        value: mm(15.0),
    };
    assert!((measure(&s, &relation).unwrap().value() - 7.0).abs() < 1e-12);
    let (dof_after, solved) = dof(&with(&s, relation));
    assert_eq!(dof_after, 1);
    assert!((at(&solved, "p").x.0 - 15.0).abs() < 1e-6);
}

#[test]
fn axis_distances_survive_a_json_round_trip_and_a_namespace_copy() {
    let mut s = diagonal();
    for (id, relation) in [
        (
            "h",
            Relation::HorizontalDistance {
                a: "a".into(),
                b: "b".into(),
                value: mm(4.0),
            },
        ),
        (
            "v",
            Relation::VerticalDistance {
                a: "a".into(),
                b: "b".into(),
                value: Expression::Reference("rise".into()),
            },
        ),
    ] {
        s.constraints
            .insert(id.into(), Constraint::driving(relation));
    }
    let text = serde_json::to_string(&s).unwrap();
    assert_eq!(serde_json::from_str::<Sketch>(&text).unwrap(), s);
    let copy = s.with_fresh_namespace("n");
    for constraint in copy.constraints.values() {
        assert!(
            constraint
                .relation
                .entity_ids()
                .iter()
                .all(|id| id.0.starts_with("n:"))
        );
    }
    assert!(
        copy.constraints.values().any(|c| c
            .relation
            .parameter_ids()
            .iter()
            .any(|p| p.0 == "n:rise"))
    );
}
