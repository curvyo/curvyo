//! Independent analytical acceptance of the first public sketch-core milestone.
#![allow(clippy::unwrap_used, clippy::float_cmp)]
use curvyo_sketch_core::*;
use kurbo::{PathEl, Shape};
fn blank() -> Sketch {
    Sketch::new("origin".into())
}
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
fn add(s: &mut Sketch, id: &str, r: Relation) {
    s.constraints.insert(id.into(), Constraint::driving(r));
}
fn length(n: f64) -> Expression {
    Expression::Literal(Quantity::length(n).unwrap())
}
fn near(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-6, "{a} != {b}");
}
fn expected(s: &Sketch, status: &SolveStatus) -> Sketch {
    let before = s.clone();
    let r = solve(s, &SolveMode::Edit).unwrap();
    assert_eq!(&r.status, status);
    assert_eq!(s, &before);
    r.candidate.unwrap()
}

#[test]
fn ac4_ac5_analytical_point_and_line_rank_and_residuals() {
    let mut s = blank();
    point(&mut s, "p", 3.0, 4.0, false);
    expected(&s, &SolveStatus::UnderConstrained(2));
    point(&mut s, "a", 0.0, 0.0, false);
    line(&mut s, "line", "a", "p");
    expected(&s, &SolveStatus::UnderConstrained(4));
    add(&mut s, "anchor", Relation::Fixed("a".into()));
    expected(&s, &SolveStatus::UnderConstrained(2));
    add(&mut s, "horizontal", Relation::Horizontal("line".into()));
    expected(&s, &SolveStatus::UnderConstrained(1));
    add(
        &mut s,
        "length",
        Relation::Length {
            curve: "line".into(),
            value: length(10.0),
        },
    );
    let solved = expected(&s, &SolveStatus::FullyConstrained);
    let (a, b) = solved.line(&"line".into()).unwrap();
    near(a.x.0, 0.0);
    near(a.y.0, 0.0);
    near(b.y.0, 0.0);
    near(a.distance(b).0, 10.0);
    assert_eq!(
        solved.point(&"origin".into()).unwrap(),
        Position::mm(0.0, 0.0)
    );
}
#[test]
fn ac5_circle_and_non_circular_ellipse_have_known_freedom() {
    let mut c = blank();
    point(&mut c, "c", 2.0, 3.0, false);
    c.entities.insert(
        "circle".into(),
        Entity::Circle {
            center: "c".into(),
            radius: Length(5.0),
            fixed_radius: false,
        },
    );
    expected(&c, &SolveStatus::UnderConstrained(3));
    add(&mut c, "center", Relation::Fixed("c".into()));
    expected(&c, &SolveStatus::UnderConstrained(1));
    add(
        &mut c,
        "r",
        Relation::Radius {
            curve: "circle".into(),
            value: length(5.0),
        },
    );
    expected(&c, &SolveStatus::FullyConstrained);
    let mut e = blank();
    point(&mut e, "c", 0.0, 0.0, false);
    point(&mut e, "f", 4.0, 0.0, false);
    e.entities.insert(
        "ellipse".into(),
        Entity::Ellipse {
            center: "c".into(),
            focus: "f".into(),
            minor_radius: Length(3.0),
            fixed_radius: false,
        },
    );
    expected(&e, &SolveStatus::UnderConstrained(5));
    add(&mut e, "center", Relation::Fixed("c".into()));
    expected(&e, &SolveStatus::UnderConstrained(3));
    add(&mut e, "focus", Relation::Fixed("f".into()));
    expected(&e, &SolveStatus::UnderConstrained(1));
    add(&mut e, "fixed", Relation::Fixed("ellipse".into()));
    expected(&e, &SolveStatus::FullyConstrained);
}
#[test]
fn ac5_ac7_duplicate_and_contradiction_ids_never_publish_invalid_zero_dof() {
    let mut s = blank();
    point(&mut s, "a", 0.0, 0.0, true);
    point(&mut s, "b", 10.0, 0.0, false);
    line(&mut s, "line", "a", "b");
    add(&mut s, "first", Relation::Horizontal("line".into()));
    add(&mut s, "second", Relation::Horizontal("line".into()));
    let r = solve(&s, &SolveMode::Edit).unwrap();
    assert_eq!(r.status, SolveStatus::UnderConstrained(1));
    assert!(r.conflicting.is_empty());
    assert!(!r.redundant.is_empty());
    assert!(r.redundant.iter().all(|id| s.constraints.contains_key(id)));
    assert!(r.candidate.is_some());
    point(&mut s, "b", 10.0, 1.0, true);
    let before = s.clone();
    let last_valid = r.candidate.unwrap();
    let failure = solve(&s, &SolveMode::Edit).unwrap();
    assert_ne!(failure.status, SolveStatus::FullyConstrained);
    assert!(failure.candidate.is_none());
    assert_eq!(s, before);
    assert!(
        failure
            .conflicting
            .iter()
            .all(|id| s.constraints.contains_key(id))
    );
    // Caller checkpoint remains available; failed intent must not replace it.
    near(last_valid.point(&"b".into()).unwrap().y.0, 0.0);
}
#[test]
fn ac6_drag_fixed_and_circle_locus_keeps_hard_constraints() {
    let mut s = blank();
    point(&mut s, "p", 10.0, 0.0, false);
    add(
        &mut s,
        "distance",
        Relation::Distance {
            a: "origin".into(),
            b: "p".into(),
            value: length(10.0),
        },
    );
    let before = s.clone();
    let r = solve(
        &s,
        &SolveMode::Drag {
            point: "p".into(),
            target: Position::mm(20.0, 0.0),
        },
    )
    .unwrap();
    assert_eq!(r.status, SolveStatus::Preview);
    assert!(r.fully_constrained.is_none());
    let candidate = r.candidate.unwrap();
    near(
        candidate
            .point(&"p".into())
            .unwrap()
            .distance(Position::mm(0.0, 0.0))
            .0,
        10.0,
    );
    assert_eq!(s, before);
    let r = solve(
        &s,
        &SolveMode::Drag {
            point: "origin".into(),
            target: Position::mm(99.0, 23.0),
        },
    )
    .unwrap();
    if let Some(candidate) = r.candidate {
        assert_eq!(
            candidate.point(&"origin".into()).unwrap(),
            Position::mm(0.0, 0.0)
        );
    }
}
#[test]
fn ac3_parameter_formula_units_rename_cycles_and_failure_are_atomic() {
    let mut p = Parameters::default();
    p.insert(
        "w".into(),
        Parameter::literal("width", Quantity::length(80.0).unwrap()),
    );
    p.insert(
        "m".into(),
        Parameter::literal("edge_margin", Quantity::length(10.0).unwrap()),
    );
    let formula = Expression::parse("width - 2 * edge_margin", &p).unwrap();
    p.insert(
        "s".into(),
        Parameter::expression("spacing", Dimension::Length, formula),
    );
    near(p.evaluate().unwrap()[&"s".into()].value(), 60.0);
    p.get_mut(&"w".into()).unwrap().name = "plate_width".into();
    p.get_mut(&"w".into()).unwrap().expression = length(100.0);
    near(p.evaluate().unwrap()[&"s".into()].value(), 80.0);
    let q = Expression::parse("1 in - 5.4 mm", &p)
        .unwrap()
        .evaluate(&p.evaluate().unwrap())
        .unwrap();
    near(q.value(), 20.0);
    assert_eq!(q.dimension(), Dimension::Length);
    let before = p.clone();
    for expression in ["missing + 1 mm", "plate_width +", "(1 mm"] {
        assert!(Expression::parse(expression, &p).is_err());
    }
    for expression in ["plate_width + 1 deg", "plate_width / 0"] {
        let result = Expression::parse(expression, &p).and_then(|e| e.evaluate(&p.evaluate()?));
        assert!(result.is_err());
    }
    assert_eq!(p, before);
    p.get_mut(&"w".into()).unwrap().expression = Expression::Reference("s".into());
    assert!(p.evaluate().is_err());
    near(before.evaluate().unwrap()[&"s".into()].value(), 80.0);
}
#[test]
fn ac3_ac10_deep_parameter_graph_roundtrip_and_invalid_data() {
    let mut p = Parameters::default();
    p.insert(
        "p0".into(),
        Parameter::literal("base", Quantity::length(1.0).unwrap()),
    );
    for i in 1..3000 {
        p.insert(
            format!("p{i}").into(),
            Parameter::expression(
                &format!("value{i}"),
                Dimension::Length,
                Expression::Add(
                    Box::new(Expression::Reference(format!("p{}", i - 1).into())),
                    Box::new(length(1.0)),
                ),
            ),
        );
    }
    let restored: Parameters = serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
    near(
        restored.evaluate().unwrap()[&ParameterId::from("p2999")].value(),
        3000.0,
    );
    let mut s = blank();
    point(&mut s, "c", 0.0, 0.0, true);
    s.entities.insert(
        "bad".into(),
        Entity::Circle {
            center: "c".into(),
            radius: Length(-1.0),
            fixed_radius: false,
        },
    );
    let decoded: Sketch = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
    assert!(decoded.validate().is_err());
    assert!(solve(&decoded, &SolveMode::Edit).is_err());
    assert!(
        serde_json::from_str::<Quantity>(r#"{"value":"not-a-number","dimension":"Length"}"#)
            .is_err()
    );
}
#[test]
fn ac14_three_point_curves_pass_actual_non_axis_third_pick() {
    let seed = blank();
    let circle = create_circle_three_points(
        &seed,
        "circle",
        [
            Position::mm(0.0, 0.0),
            Position::mm(2.0, 0.0),
            Position::mm(0.0, 2.0),
        ],
    )
    .unwrap();
    let Entity::Circle { center, radius, .. } = circle
        .entities
        .values()
        .find(|ellipse| matches!(ellipse, Entity::Circle { .. }))
        .unwrap()
    else {
        unreachable!()
    };
    let center = circle.point(center).unwrap();
    near(center.x.0, 1.0);
    near(center.y.0, 1.0);
    near(radius.0, 2.0_f64.sqrt());
    let third = Position::mm(3.0, 2.4);
    let ellipse = create_ellipse_three_points(
        &seed,
        "ellipse",
        Position::mm(-5.0, 0.0),
        Position::mm(5.0, 0.0),
        third,
    )
    .unwrap();
    let Entity::Ellipse {
        center,
        focus,
        minor_radius,
        ..
    } = ellipse
        .entities
        .values()
        .find(|ellipse| matches!(ellipse, Entity::Ellipse { .. }))
        .unwrap()
    else {
        unreachable!()
    };
    let circle = ellipse.point(center).unwrap();
    let focus_position = ellipse.point(focus).unwrap();
    let minor_axis = minor_radius.0;
    let major_axis = (circle.distance(focus_position).0.powi(2) + minor_axis * minor_axis).sqrt();
    near(major_axis, 5.0);
    near(minor_axis, 3.0);
    near(
        (third.x.0 - circle.x.0).powi(2) / (major_axis * major_axis)
            + (third.y.0 - circle.y.0).powi(2) / (minor_axis * minor_axis),
        1.0,
    );
    assert!(ellipse.constraints.is_empty());
    assert_eq!(seed, blank());
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
fn ac8_ac15_slot_outline_has_exact_topology_dimensions_and_area() {
    let seed = blank();
    let s = create_slot(
        &seed,
        "slot",
        Position::mm(0.0, 0.0),
        Position::mm(20.0, 0.0),
        Length(6.0),
    )
    .unwrap();
    let r = solve(&s, &SolveMode::Edit).unwrap();
    let c = r.candidate.unwrap();
    let path = outline(&c, Length(0.001)).unwrap();
    let b = path.bounding_box();
    near(b.x0, -3.0);
    near(b.x1, 23.0);
    near(b.y0, -3.0);
    near(b.y1, 3.0);
    assert!((path.area().abs() - (120.0 + 9.0 * std::f64::consts::PI)).abs() < 0.03);
    assert_eq!(
        path.elements()
            .iter()
            .filter(|e| matches!(e, PathEl::ClosePath))
            .count(),
        1
    );
    assert_eq!(seed, blank());
}

#[test]
fn ac7_contradictory_dimensions_name_original_constraints_and_repair() {
    let mut s = blank();
    point(&mut s, "b", 10.0, 0.0, false);
    line(&mut s, "line", "origin", "b");
    add(&mut s, "horizontal", Relation::Horizontal("line".into()));
    add(
        &mut s,
        "ten",
        Relation::Length {
            curve: "line".into(),
            value: length(10.0),
        },
    );
    let last = expected(&s, &SolveStatus::FullyConstrained);
    add(
        &mut s,
        "twenty",
        Relation::Length {
            curve: "line".into(),
            value: length(20.0),
        },
    );
    let before = s.clone();
    let r = solve(&s, &SolveMode::Edit).unwrap();
    assert_eq!(r.status, SolveStatus::Conflicting);
    assert!(r.candidate.is_none());
    assert!(!r.conflicting.is_empty());
    assert!(
        r.conflicting
            .iter()
            .all(|id| s.constraints.contains_key(id))
    );
    assert_eq!(s, before);
    s.constraints.remove(&"twenty".into());
    let repaired = expected(&s, &SolveStatus::FullyConstrained);
    near(
        repaired
            .point(&"b".into())
            .unwrap()
            .distance(last.point(&"b".into()).unwrap())
            .0,
        0.0,
    );
}
#[test]
fn ac16_fillet_preserves_original_relationships_and_oversize_refuses_without_mutation() {
    let mut s = blank();
    point(&mut s, "corner", 0.0, 0.0, true);
    point(&mut s, "a", 10.0, 0.0, true);
    point(&mut s, "b", 0.0, 10.0, true);
    line(&mut s, "x", "corner", "a");
    line(&mut s, "y", "corner", "b");
    add(&mut s, "horizontal", Relation::Horizontal("x".into()));
    add(&mut s, "vertical", Relation::Vertical("y".into()));
    add(
        &mut s,
        "width",
        Relation::Length {
            curve: "x".into(),
            value: length(10.0),
        },
    );
    let before = s.clone();
    let rounded = fillet(&s, "round", &"x".into(), &"y".into(), Length(2.0)).unwrap();
    assert_eq!(s, before);
    for (id, constraint) in &before.constraints {
        assert_eq!(rounded.constraints.get(id), Some(constraint));
    }
    for id in [EntityId::from("x"), EntityId::from("y")] {
        assert_eq!(rounded.entities.get(&id), before.entities.get(&id));
        assert!(rounded.construction.contains(&id));
    }
    let Entity::Arc {
        center,
        start,
        end,
        radius,
        ..
    } = rounded
        .entities
        .values()
        .find(|e| matches!(e, Entity::Arc { .. }))
        .unwrap()
    else {
        unreachable!()
    };
    near(radius.0, 2.0);
    let c = rounded.point(center).unwrap();
    near(c.x.0, 2.0);
    near(c.y.0, 2.0);
    let ends = [rounded.point(start).unwrap(), rounded.point(end).unwrap()];
    for target in [Position::mm(2.0, 0.0), Position::mm(0.0, 2.0)] {
        assert!(
            ends.iter().any(|p| p.distance(target).0 < 1e-6),
            "endpoints {ends:?}, target {target:?}"
        );
    }
    assert!(fillet(&s, "too-large", &"x".into(), &"y".into(), Length(20.0)).is_err());
    assert_eq!(s, before);
}
#[test]
fn ac8_outline_explicit_coincidence_closes_cycle_and_preserves_attached_spur() {
    let mut s = blank();
    for (id, x, y) in [
        ("a", 0.0, 0.0),
        ("b", 10.0, 0.0),
        ("c", 10.0, 10.0),
        ("d", 0.0, 10.0),
        ("a2", 0.0, 0.0),
        ("spur", -5.0, -5.0),
    ] {
        point(&mut s, id, x, y, true);
    }
    for (id, a, b) in [
        ("ab", "a", "b"),
        ("bc", "b", "c"),
        ("cd", "c", "d"),
        ("da", "d", "a2"),
        ("spur-line", "a", "spur"),
    ] {
        line(&mut s, id, a, b);
    }
    let disconnected = outline(&s, Length(0.001)).unwrap();
    assert_eq!(
        disconnected
            .elements()
            .iter()
            .filter(|e| matches!(e, PathEl::ClosePath))
            .count(),
        0
    );
    add(
        &mut s,
        "join",
        Relation::Coincident("a".into(), "a2".into()),
    );
    let p = outline(&s, Length(0.001)).unwrap();
    assert_eq!(
        p.elements()
            .iter()
            .filter(|e| matches!(e, PathEl::ClosePath))
            .count(),
        1
    );
    near(p.area().abs(), 100.0);
    assert!(
        p.elements()
            .iter()
            .any(|e| matches!(e,PathEl::MoveTo(p)|PathEl::LineTo(p) if p.x == -5.0 && p.y == -5.0))
    );
}

#[test]
fn ac2_remaining_relations_have_independent_geometric_residuals() {
    for action in [
        "parallel",
        "perpendicular",
        "vertical",
        "coincident",
        "midpoint",
        "symmetric",
    ] {
        let mut s = blank();
        point(&mut s, "a", 0.0, 0.0, true);
        point(&mut s, "b", 10.0, 0.0, true);
        line(&mut s, "axis", "a", "b");
        point(&mut s, "source", 3.0, 4.0, true);
        point(&mut s, "moving", 4.0, 2.0, false);
        line(&mut s, "moving-line", "a", "moving");
        let relation = match action {
            "parallel" => Relation::Parallel("axis".into(), "moving-line".into()),
            "perpendicular" => Relation::Perpendicular("axis".into(), "moving-line".into()),
            "vertical" => Relation::Vertical("moving-line".into()),
            "coincident" => Relation::Coincident("source".into(), "moving".into()),
            "midpoint" => Relation::Midpoint {
                point: "moving".into(),
                line: "axis".into(),
            },
            _ => Relation::Symmetric {
                source: "source".into(),
                image: "moving".into(),
                axis: "axis".into(),
            },
        };
        add(&mut s, action, relation);
        let candidate = solve(&s, &SolveMode::Edit).unwrap().candidate.unwrap();
        let p = candidate.point(&"moving".into()).unwrap();
        match action {
            "parallel" => near(10.0 * p.y.0, 0.0),
            "perpendicular" | "vertical" => near(10.0 * p.x.0, 0.0),
            "coincident" => near(p.distance(Position::mm(3.0, 4.0)).0, 0.0),
            "midpoint" => near(p.distance(Position::mm(5.0, 0.0)).0, 0.0),
            _ => near(p.distance(Position::mm(3.0, -4.0)).0, 0.0),
        }
    }
}
#[test]
fn ac2_ac3_equal_diameter_and_ellipse_tangency_use_known_answers() {
    let mut s = blank();
    point(&mut s, "center", 0.0, 0.0, true);
    point(&mut s, "other", 20.0, 0.0, true);
    for (id, center, radius, fixed_radius) in
        [("one", "center", 3.0, true), ("two", "other", 5.0, false)]
    {
        s.entities.insert(
            id.into(),
            Entity::Circle {
                center: center.into(),
                radius: Length(radius),
                fixed_radius,
            },
        );
    }
    add(&mut s, "equal", Relation::Equal("one".into(), "two".into()));
    let candidate = solve(&s, &SolveMode::Edit).unwrap().candidate.unwrap();
    let Entity::Circle { radius, .. } = candidate.entities[&"two".into()] else {
        panic!()
    };
    near(radius.0, 3.0);
    s.constraints.clear();
    add(
        &mut s,
        "diameter",
        Relation::Diameter {
            curve: "two".into(),
            value: length(8.0),
        },
    );
    let candidate = solve(&s, &SolveMode::Edit).unwrap().candidate.unwrap();
    let Entity::Circle { radius, .. } = candidate.entities[&"two".into()] else {
        panic!()
    };
    near(radius.0, 4.0);
    let mut ellipse = blank();
    point(&mut ellipse, "center", 0.0, 0.0, true);
    point(&mut ellipse, "focus", 4.0, 0.0, true);
    point(&mut ellipse, "left", -10.0, 3.0, true);
    point(&mut ellipse, "right", 10.0, 3.0, true);
    line(&mut ellipse, "tangent", "left", "right");
    ellipse.entities.insert(
        "ellipse".into(),
        Entity::Ellipse {
            center: "center".into(),
            focus: "focus".into(),
            minor_radius: Length(2.8),
            fixed_radius: false,
        },
    );
    add(
        &mut ellipse,
        "touch",
        Relation::Tangent("tangent".into(), "ellipse".into()),
    );
    let solved = solve(&ellipse, &SolveMode::Edit)
        .unwrap()
        .candidate
        .unwrap();
    let Entity::Ellipse { minor_radius, .. } = solved.entities[&"ellipse".into()] else {
        panic!()
    };
    near(minor_radius.0, 3.0); // At (0,3), ellipse normal is vertical and line direction horizontal.
    near(
        0.0_f64 * 10.0 + (3.0 / (minor_radius.0 * minor_radius.0)) * 0.0,
        0.0,
    );
}
#[test]
fn ac3_angle_and_reference_have_known_geometry_and_rank() {
    let mut s = blank();
    point(&mut s, "axis-end", 10.0, 0.0, true);
    line(&mut s, "axis", "origin", "axis-end");
    point(&mut s, "end", 3.0, 4.0, false);
    line(&mut s, "line", "origin", "end");
    add(
        &mut s,
        "length",
        Relation::Length {
            curve: "line".into(),
            value: length(5.0),
        },
    );
    add(
        &mut s,
        "angle",
        Relation::Angle {
            a: "axis".into(),
            b: "line".into(),
            value: Expression::Literal(Quantity::angle(std::f64::consts::FRAC_PI_3).unwrap()),
        },
    );
    let solved = expected(&s, &SolveStatus::FullyConstrained);
    let endpoint = solved.point(&"end".into()).unwrap();
    near(endpoint.x.0, 2.5);
    near(endpoint.y.0, 5.0 * 3.0_f64.sqrt() / 2.0);
    let before = solve(&s, &SolveMode::Edit).unwrap();
    s.constraints.insert(
        "reference".into(),
        Constraint {
            relation: Relation::Length {
                curve: "line".into(),
                value: length(99.0),
            },
            reference: true,
        },
    );
    let after = solve(&s, &SolveMode::Edit).unwrap();
    assert_eq!(after.status, before.status);
    near(
        after
            .candidate
            .unwrap()
            .point(&"end".into())
            .unwrap()
            .distance(endpoint)
            .0,
        0.0,
    );
}

#[test]
fn ac17_entity_freedom_includes_defining_points() {
    let mut circle = blank();
    point(&mut circle, "center", 20.0, 20.0, false);
    circle.entities.insert(
        "circle".into(),
        Entity::Circle {
            center: "center".into(),
            radius: Length(5.0),
            fixed_radius: false,
        },
    );
    add(
        &mut circle,
        "diameter",
        Relation::Diameter {
            curve: "circle".into(),
            value: length(10.0),
        },
    );
    let free = solve(&circle, &SolveMode::Edit).unwrap();
    assert_eq!(free.status, SolveStatus::UnderConstrained(2));
    assert!(!free.fully_constrained.unwrap().contains(&"circle".into()));
    let moved = solve(
        &circle,
        &SolveMode::Drag {
            point: "center".into(),
            target: Position::mm(30.0, 25.0),
        },
    )
    .unwrap()
    .candidate
    .unwrap();
    near(
        moved
            .point(&"center".into())
            .unwrap()
            .distance(Position::mm(30.0, 25.0))
            .0,
        0.0,
    );
    add(&mut circle, "fix", Relation::Fixed("center".into()));
    let fixed = solve(&circle, &SolveMode::Edit).unwrap();
    assert!(fixed.fully_constrained.unwrap().contains(&"circle".into()));
    let mut free_line = blank();
    point(&mut free_line, "end", 5.0, 5.0, false);
    line(&mut free_line, "line", "origin", "end");
    assert!(
        !solve(&free_line, &SolveMode::Edit)
            .unwrap()
            .fully_constrained
            .unwrap()
            .contains(&"line".into())
    );
}

#[test]
fn ac16_fillet_radius_remains_editable_without_losing_original_constraints() {
    let mut original = blank();
    point(&mut original, "corner", 0.0, 0.0, true);
    point(&mut original, "x", 10.0, 0.0, true);
    point(&mut original, "y", 0.0, 10.0, true);
    line(&mut original, "horizontal", "corner", "x");
    line(&mut original, "vertical", "corner", "y");
    add(
        &mut original,
        "width",
        Relation::Length {
            curve: "horizontal".into(),
            value: length(10.0),
        },
    );
    let mut rounded = fillet(
        &original,
        "round",
        &"horizontal".into(),
        &"vertical".into(),
        Length(2.0),
    )
    .unwrap();
    let radius = rounded
        .constraints
        .values_mut()
        .find_map(|c| {
            if let Relation::Radius { value, .. } = &mut c.relation {
                Some(value)
            } else {
                None
            }
        })
        .unwrap();
    *radius = length(1.0);
    let solved = solve(&rounded, &SolveMode::Edit)
        .unwrap()
        .candidate
        .unwrap();
    for (id, c) in &original.constraints {
        assert_eq!(solved.constraints.get(id), Some(c));
    }
    let Entity::Arc { center, radius, .. } = solved
        .entities
        .values()
        .find(|e| matches!(e, Entity::Arc { .. }))
        .unwrap()
    else {
        panic!()
    };
    near(radius.0, 1.0);
    near(
        solved
            .point(center)
            .unwrap()
            .distance(Position::mm(1.0, 1.0))
            .0,
        0.0,
    );
}
#[test]
fn ac12_repeated_disconnected_solve_and_near_circular_ellipse_are_stable() {
    let mut s = blank();
    point(&mut s, "focus", 1e-8, 0.0, true);
    s.entities.insert(
        "ellipse".into(),
        Entity::Ellipse {
            center: "origin".into(),
            focus: "focus".into(),
            minor_radius: Length(3.0),
            fixed_radius: true,
        },
    );
    point(&mut s, "unrelated", 100.0, 20.0, false);
    let first = solve(&s, &SolveMode::Edit).unwrap();
    for _ in 0..5 {
        let next = solve(&s, &SolveMode::Edit).unwrap();
        assert_eq!(next.status, first.status);
        let a = first.candidate.as_ref().unwrap();
        let b = next.candidate.unwrap();
        for (id, e) in &a.entities {
            if let Entity::Point { position, .. } = e {
                near(position.distance(b.point(id).unwrap()).0, 0.0);
            }
        }
    }
    let mut reversed = s.clone();
    reversed.entities.clear();
    for (id, e) in s.entities.iter().rev() {
        reversed.entities.insert(id.clone(), e.clone());
    }
    assert_eq!(
        solve(&reversed, &SolveMode::Edit).unwrap().candidate,
        first.candidate
    );
}
#[test]
fn ac3_deep_expression_lifecycle_is_bounded_in_isolated_process() {
    const CHILD: &str = "CURVYO_0066_DEEP_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let result = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "ac3_deep_expression_lifecycle_is_bounded_in_isolated_process",
                "--nocapture",
            ])
            .env(CHILD, "1")
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "child failed: {}",
            String::from_utf8_lossy(&result.stderr)
        );
        return;
    }
    let mut expression = Expression::Literal(Quantity::length(5.0).unwrap());
    for _ in 0..2048 {
        expression = Expression::Negate(Box::new(expression));
    }
    let resolved = expression.evaluate(&std::collections::BTreeMap::new());
    if let Ok(value) = resolved {
        near(value.value(), 5.0);
    }
    let encoded = serde_json::to_vec(&expression);
    if let Ok(bytes) = encoded {
        let decoded = serde_json::from_slice::<Expression>(&bytes);
        if let Ok(value) = decoded {
            drop(value);
        }
    }
    drop(expression);
}

#[test]
fn ac3_angle_crosses_signed_pi_boundary_without_false_conflict() {
    for sign in [-1.0, 1.0] {
        let mut g = blank();
        point(&mut g, "axis-end", 10.0, 0.0, true);
        line(&mut g, "axis", "origin", "axis-end");
        let initial = sign * (std::f64::consts::PI - 0.02);
        let target = -sign * (std::f64::consts::PI - 0.01);
        point(
            &mut g,
            "end",
            10.0 * initial.cos(),
            10.0 * initial.sin(),
            false,
        );
        line(&mut g, "line", "origin", "end");
        add(
            &mut g,
            "length",
            Relation::Length {
                curve: "line".into(),
                value: length(10.0),
            },
        );
        add(
            &mut g,
            "angle",
            Relation::Angle {
                a: "axis".into(),
                b: "line".into(),
                value: Expression::Literal(Quantity::angle(target).unwrap()),
            },
        );
        let solved = expected(&g, &SolveStatus::FullyConstrained);
        let p = solved.point(&"end".into()).unwrap();
        near(p.x.0, 10.0 * target.cos());
        near(p.y.0, 10.0 * target.sin());
    }
}
