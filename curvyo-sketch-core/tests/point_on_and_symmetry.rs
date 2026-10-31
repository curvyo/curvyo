//! Coincident and Symmetric targets of customer revision 3 (0066 AC37, AC40): solved position
//! within 1e-6 mm and the change of the degrees of freedom.
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

/// Solves `s` before and after adding `relation`; returns the DOF change and the solved sketch.
fn apply(s: &Sketch, relation: Relation) -> (usize, usize, Sketch) {
    let (before, _) = dof(s);
    let mut after = s.clone();
    after
        .constraints
        .insert("new".into(), Constraint::driving(relation));
    let (after_dof, solved) = dof(&after);
    (before, after_dof, solved)
}

fn at(s: &Sketch, id: &str) -> Position {
    s.point(&id.into()).unwrap()
}

/// A sketch with the free point `p` at (20, 3) and the fixed geometry `fixed` adds.
fn with_free_point(fixed: impl FnOnce(&mut Sketch)) -> Sketch {
    let mut s = Sketch::new("origin".into());
    point(&mut s, "p", 20.0, 3.0, false);
    fixed(&mut s);
    s
}

#[test]
fn a_point_on_a_line_lies_on_its_infinite_line_beyond_the_segment() {
    let s = with_free_point(|s| {
        point(s, "a", 0.0, 0.0, true);
        point(s, "b", 10.0, 0.0, true);
        s.entities.insert(
            "l".into(),
            Entity::Line {
                start: "a".into(),
                end: "b".into(),
            },
        );
    });
    let (before, after, solved) = apply(
        &s,
        Relation::PointOn {
            point: "p".into(),
            curve: "l".into(),
        },
    );
    assert_eq!((before, after), (2, 1));
    let p = at(&solved, "p");
    assert!(p.y.0.abs() < 1e-6, "{p:?}");
    assert!(p.x.0 > 10.0 + 1e-3, "the extent is not enforced: {p:?}");
}

#[test]
fn a_point_on_a_circle_arc_or_ellipse_lies_on_the_full_curve() {
    let curves: [(&str, Entity); 3] = [
        (
            "circle",
            Entity::Circle {
                center: "c".into(),
                radius: Length(5.0),
                fixed_radius: true,
            },
        ),
        (
            // A quarter arc from 0 to 90 degrees; the point ends outside that span.
            "arc",
            Entity::Arc {
                center: "c".into(),
                start: "s".into(),
                end: "e".into(),
                radius: Length(5.0),
                start_angle: Angle(0.0),
                end_angle: Angle(std::f64::consts::FRAC_PI_2),
                fixed_shape: true,
            },
        ),
        (
            "ellipse",
            Entity::Ellipse {
                center: "c".into(),
                focus: "f".into(),
                minor_radius: Length(3.0),
                fixed_radius: true,
            },
        ),
    ];
    for (name, curve) in curves {
        let mut s = Sketch::new("origin".into());
        point(&mut s, "c", 10.0, 10.0, true);
        point(&mut s, "s", 15.0, 10.0, true);
        point(&mut s, "e", 10.0, 15.0, true);
        point(&mut s, "f", 14.0, 10.0, true);
        point(&mut s, "p", 4.0, 7.0, false);
        s.entities.insert("k".into(), curve);
        let (before, after, solved) = apply(
            &s,
            Relation::PointOn {
                point: "p".into(),
                curve: "k".into(),
            },
        );
        assert_eq!((before, after), (2, 1), "{name}");
        let p = at(&solved, "p");
        let (dx, dy) = (p.x.0 - 10.0, p.y.0 - 10.0);
        let error = if name == "ellipse" {
            // Major radius 5 from focus distance 4 and minor radius 3, axis along X.
            (dx / 5.0).hypot(dy / 3.0) - 1.0
        } else {
            dx.hypot(dy) - 5.0
        };
        assert!(error.abs() < 1e-6, "{name}: {p:?}");
        assert!(
            dx < 0.0,
            "{name}: stays on the far side, outside the arc span"
        );
    }
}

#[test]
fn a_point_on_an_axis() {
    let s = with_free_point(|_| {});
    for (axis, coordinate) in [(Axis::X, 1), (Axis::Y, 0)] {
        let (before, after, solved) = apply(
            &s,
            Relation::OnAxis {
                point: "p".into(),
                axis,
            },
        );
        assert_eq!((before, after), (2, 1));
        let p = at(&solved, "p");
        let value = if coordinate == 0 { p.x.0 } else { p.y.0 };
        assert!(value.abs() < 1e-6, "{axis:?}: {p:?}");
    }
}

/// A fixed source at (3, 4) and a free image at (8, -1).
fn mirror_pair(extra: impl FnOnce(&mut Sketch)) -> Sketch {
    let mut s = Sketch::new("origin".into());
    point(&mut s, "a", 3.0, 4.0, true);
    point(&mut s, "b", 8.0, -1.0, false);
    extra(&mut s);
    s
}

#[test]
fn two_points_mirror_about_a_point_a_line_and_each_axis() {
    let cases: Vec<(&str, Sketch, Relation, Position)> = vec![
        (
            "point",
            mirror_pair(|s| point(s, "m", 5.0, 5.0, true)),
            Relation::SymmetricPoint {
                source: "a".into(),
                image: "b".into(),
                center: "m".into(),
            },
            Position::mm(7.0, 6.0),
        ),
        (
            "line",
            mirror_pair(|s| {
                point(s, "l0", 6.0, 0.0, true);
                point(s, "l1", 6.0, 10.0, true);
                s.entities.insert(
                    "l".into(),
                    Entity::Line {
                        start: "l0".into(),
                        end: "l1".into(),
                    },
                );
            }),
            Relation::Symmetric {
                source: "a".into(),
                image: "b".into(),
                axis: "l".into(),
            },
            Position::mm(9.0, 4.0),
        ),
        (
            "x axis",
            mirror_pair(|_| {}),
            Relation::SymmetricAxis {
                source: "a".into(),
                image: "b".into(),
                axis: Axis::X,
            },
            Position::mm(3.0, -4.0),
        ),
        (
            "y axis",
            mirror_pair(|_| {}),
            Relation::SymmetricAxis {
                source: "a".into(),
                image: "b".into(),
                axis: Axis::Y,
            },
            Position::mm(-3.0, 4.0),
        ),
    ];
    for (name, s, relation, image) in cases {
        let (before, after, solved) = apply(&s, relation);
        assert_eq!((before, after), (2, 0), "{name}");
        assert!(at(&solved, "b").distance(image).0 < 1e-6, "{name}");
    }
}

#[test]
fn two_points_share_a_y_or_an_x_coordinate() {
    let s = mirror_pair(|_| {});
    let (before, after, solved) = apply(&s, Relation::HorizontalPoints("a".into(), "b".into()));
    assert_eq!((before, after), (2, 1));
    assert!((at(&solved, "b").y.0 - 4.0).abs() < 1e-6);
    let (before, after, solved) = apply(&s, Relation::VerticalPoints("a".into(), "b".into()));
    assert_eq!((before, after), (2, 1));
    assert!((at(&solved, "b").x.0 - 3.0).abs() < 1e-6);
}

#[test]
fn the_new_relations_survive_a_json_round_trip_and_a_namespace_copy() {
    let mut s = mirror_pair(|s| point(s, "m", 5.0, 5.0, true));
    for (id, relation) in [
        (
            "on",
            Relation::PointOn {
                point: "b".into(),
                curve: "missing".into(),
            },
        ),
        (
            "axis",
            Relation::SymmetricAxis {
                source: "a".into(),
                image: "b".into(),
                axis: Axis::Y,
            },
        ),
        ("h", Relation::HorizontalPoints("a".into(), "b".into())),
        ("v", Relation::VerticalPoints("a".into(), "b".into())),
        (
            "centre",
            Relation::SymmetricPoint {
                source: "a".into(),
                image: "b".into(),
                center: "m".into(),
            },
        ),
        (
            "direction",
            Relation::AxisAngle {
                line: "l".into(),
                value: Expression::Reference("turn".into()),
            },
        ),
        (
            "major",
            Relation::EllipseRadius {
                curve: "e".into(),
                axis: EllipseAxis::Major,
                value: Expression::Reference("size".into()),
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
        assert!(
            constraint
                .relation
                .parameter_ids()
                .iter()
                .all(|id| id.0.starts_with("n:"))
        );
    }
}

/// A sketch with a point `p`, a line `l`, a circle `c` and an ellipse `e`.
fn one_of_each() -> Sketch {
    let mut s = Sketch::new("origin".into());
    for (id, x, y) in [
        ("p", 1.0, 1.0),
        ("q", 2.0, 5.0),
        ("l0", 0.0, 0.0),
        ("l1", 9.0, 0.0),
    ] {
        point(&mut s, id, x, y, false);
    }
    point(&mut s, "f", 13.0, 0.0, false);
    s.entities.insert(
        "l".into(),
        Entity::Line {
            start: "l0".into(),
            end: "l1".into(),
        },
    );
    s.entities.insert(
        "c".into(),
        Entity::Circle {
            center: "p".into(),
            radius: Length(2.0),
            fixed_radius: false,
        },
    );
    s.entities.insert(
        "e".into(),
        Entity::Ellipse {
            center: "l1".into(),
            focus: "f".into(),
            minor_radius: Length(3.0),
            fixed_radius: false,
        },
    );
    s
}

#[test]
fn records_naming_the_wrong_kind_of_geometry_are_refused() {
    let length = || Expression::Literal(Quantity::length(5.0).unwrap());
    let cases = [
        (
            "point on a point",
            Relation::PointOn {
                point: "p".into(),
                curve: "q".into(),
            },
        ),
        (
            "mirror across a circle",
            Relation::Symmetric {
                source: "p".into(),
                image: "q".into(),
                axis: "c".into(),
            },
        ),
        (
            "mirror across a point",
            Relation::Symmetric {
                source: "p".into(),
                image: "q".into(),
                axis: "l0".into(),
            },
        ),
        (
            "axis mirror of a line",
            Relation::SymmetricAxis {
                source: "l".into(),
                image: "q".into(),
                axis: Axis::X,
            },
        ),
        (
            "level with a line",
            Relation::HorizontalPoints("p".into(), "l".into()),
        ),
        (
            "plumb with a line",
            Relation::VerticalPoints("l".into(), "q".into()),
        ),
        (
            "centre is a line",
            Relation::SymmetricPoint {
                source: "p".into(),
                image: "q".into(),
                center: "l".into(),
            },
        ),
        (
            "axis angle of a circle",
            Relation::AxisAngle {
                line: "c".into(),
                value: Expression::Literal(Quantity::angle(0.5).unwrap()),
            },
        ),
        (
            "ellipse radius of a circle",
            Relation::EllipseRadius {
                curve: "c".into(),
                axis: EllipseAxis::Minor,
                value: length(),
            },
        ),
        (
            "ellipse radius in degrees",
            Relation::EllipseRadius {
                curve: "e".into(),
                axis: EllipseAxis::Major,
                value: Expression::Literal(Quantity::angle(0.5).unwrap()),
            },
        ),
        (
            "axis angle in millimetres",
            Relation::AxisAngle {
                line: "l".into(),
                value: length(),
            },
        ),
    ];
    let valid = one_of_each();
    valid.validate().unwrap();
    for (name, relation) in cases {
        let mut s = valid.clone();
        s.constraints
            .insert("bad".into(), Constraint::driving(relation));
        // Records load on their own (references may resolve only after a merge); the graph
        // check before every solve refuses them.
        assert!(validate_records(&s).is_ok(), "{name}");
        assert!(s.validate().is_err(), "{name}");
        assert!(solve(&s, &SolveMode::Edit).is_err(), "{name}");
    }
}

#[test]
fn entity_ids_starting_with_at_are_reserved_for_the_axes() {
    let mut s = one_of_each();
    point(&mut s, "@axis-x", 0.0, 0.0, false);
    assert!(validate_records(&s).is_err());
    assert!(s.validate().is_err());
}
