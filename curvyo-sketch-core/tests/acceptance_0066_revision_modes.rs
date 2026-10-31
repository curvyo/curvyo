//! Independent continuation witnesses from revision-mode-oracles.md, not solver status alone.
#![allow(clippy::unwrap_used)]
use curvyo_sketch_core::*;
fn near(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-6, "{a} != {b}");
}
fn point(graph: &mut Sketch, id: &str, x: f64, y: f64, fixed: bool) {
    graph.entities.insert(
        id.into(),
        Entity::Point {
            position: Position::mm(x, y),
            fixed,
        },
    );
}
fn base() -> Sketch {
    let mut graph = Sketch::new("origin".into());
    point(&mut graph, "a", -10.0, 0.0, true);
    point(&mut graph, "b", 0.0, 0.0, true);
    graph.entities.insert(
        "previous".into(),
        Entity::Line {
            start: "a".into(),
            end: "b".into(),
        },
    );
    graph
}
fn append(
    graph: &Sketch,
    prefix: &str,
    previous: &str,
    start: &str,
    end: &str,
    target: Position,
    mode: ContinuationMode,
) -> ConstructionResult {
    construct(
        graph,
        prefix,
        &Construction::Continue {
            previous: previous.into(),
            start: start.into(),
            end: end.into(),
            position: target,
            mode,
        },
    )
    .unwrap()
}
fn arc_witness(
    graph: &Sketch,
    id: &str,
    endpoints: (&str, &str),
    center: Position,
    radius: f64,
    tangent: (f64, f64),
    sweep: f64,
) {
    let (join, end) = endpoints;
    let Entity::Arc {
        center: cid,
        start: stored_start,
        end: stored_end,
        radius: r,
        start_angle,
        end_angle,
        ..
    } = &graph.entities[&id.into()]
    else {
        panic!("analytic arc required")
    };
    assert!(
        (stored_start == &join.into() && stored_end == &end.into())
            || (stored_end == &join.into() && stored_start == &end.into())
    );
    let actual = graph.point(cid).unwrap();
    near(actual.x.0, center.x.0);
    near(actual.y.0, center.y.0);
    near(r.0, radius);
    for p in [join, end] {
        near(graph.point(&p.into()).unwrap().distance(actual).0, radius);
    }
    let sign = if stored_start == &join.into() {
        1.0
    } else {
        -1.0
    };
    near(sign * (end_angle.0 - start_angle.0), sweep);
    let p = graph.point(&join.into()).unwrap();
    near(sign * -(p.y.0 - actual.y.0) / r.0, tangent.0);
    near(sign * (p.x.0 - actual.x.0) / r.0, tangent.1);
}
#[test]
fn six_modes_have_shared_identity_and_independent_analytic_witnesses() {
    use ContinuationMode::*;
    let cases = [
        (FreeLine, 6.0, 4.0),
        (PerpendicularLine, 0.0, 8.0),
        (TangentLine, 8.0, 0.0),
        (TangentArc, 6.0, 6.0),
        (PerpendicularArcLeft, 6.0, 6.0),
        (PerpendicularArcRight, 6.0, -6.0),
    ];
    for (mode, x, y) in cases {
        let input = base();
        let result = append(
            &input,
            "new",
            "previous",
            "b",
            "c",
            Position::mm(x, y),
            mode,
        );
        assert_eq!(input, base());
        near(result.candidate.point(&"c".into()).unwrap().x.0, x);
        near(result.candidate.point(&"c".into()).unwrap().y.0, y);
        for id in ["a", "b", "previous"] {
            assert_eq!(
                result.intent.entities[&id.into()],
                input.entities[&id.into()]
            );
        }
        match mode {
            FreeLine | PerpendicularLine | TangentLine => {
                let Entity::Line { start, end } = &result.intent.entities[&"new:line".into()]
                else {
                    panic!("line")
                };
                assert_eq!(start, &"b".into());
                assert_eq!(end, &"c".into());
            }
            TangentArc => arc_witness(
                &result.candidate,
                "new:arc",
                ("b", "c"),
                Position::mm(0.0, 6.0),
                6.0,
                (1.0, 0.0),
                std::f64::consts::FRAC_PI_2,
            ),
            PerpendicularArcLeft => arc_witness(
                &result.candidate,
                "new:arc",
                ("b", "c"),
                Position::mm(6.0, 0.0),
                6.0,
                (0.0, 1.0),
                -std::f64::consts::FRAC_PI_2,
            ),
            PerpendicularArcRight => arc_witness(
                &result.candidate,
                "new:arc",
                ("b", "c"),
                Position::mm(6.0, 0.0),
                6.0,
                (0.0, -1.0),
                std::f64::consts::FRAC_PI_2,
            ),
        }
        if mode == FreeLine {
            assert!(result.intent.constraints.is_empty());
        } else {
            assert!(!result.intent.constraints.is_empty());
        }
    }
}
#[test]
fn reverse_start_and_forward_end_of_lines_and_arcs_use_outward_tangents() {
    let line = base();
    let mut arc = Sketch::new("origin".into());
    point(&mut arc, "center", 0.0, 0.0, true);
    point(&mut arc, "a", 10.0, 0.0, true);
    point(&mut arc, "b", 0.0, 10.0, true);
    arc.entities.insert(
        "previous".into(),
        Entity::Arc {
            center: "center".into(),
            start: "a".into(),
            end: "b".into(),
            radius: Length(10.0),
            start_angle: Angle(0.0),
            end_angle: Angle(std::f64::consts::FRAC_PI_2),
            fixed_shape: true,
        },
    );
    for (graph, endpoint, direction, target) in [
        (&line, "a", (-1.0, 0.0), Position::mm(-18.0, 0.0)),
        (&line, "b", (1.0, 0.0), Position::mm(8.0, 0.0)),
        (&arc, "a", (0.0, -1.0), Position::mm(10.0, -8.0)),
        (&arc, "b", (-1.0, 0.0), Position::mm(-8.0, 10.0)),
    ] {
        let tangent = endpoint_tangent(graph, &"previous".into(), &endpoint.into()).unwrap();
        near(tangent.0, direction.0);
        near(tangent.1, direction.1);
        let result = append(
            graph,
            "next",
            "previous",
            endpoint,
            "c",
            target,
            ContinuationMode::TangentLine,
        );
        let (start, end) = result.candidate.line(&"next:line".into()).unwrap();
        near(end.x.0, target.x.0);
        near(end.y.0, target.y.0);
        near(start.distance(end).0, 8.0);
        let Entity::Line { start, .. } = &result.intent.entities[&"next:line".into()] else {
            panic!("line")
        };
        assert_eq!(start, &endpoint.into());
    }
}
#[test]
fn curved_modes_keep_join_relations_after_radius_dimension_changes() {
    use ContinuationMode::*;
    for (mode, x, y, expected_y, expected_center) in [
        (
            TangentArc,
            4.8,
            2.4,
            10.0_f64 - (100.0_f64 - 4.8 * 4.8).sqrt(),
            Position::mm(0.0, 10.0),
        ),
        (
            PerpendicularArcLeft,
            6.0,
            6.0,
            84.0_f64.sqrt(),
            Position::mm(10.0, 0.0),
        ),
        (
            PerpendicularArcRight,
            6.0,
            -6.0,
            -84.0_f64.sqrt(),
            Position::mm(10.0, 0.0),
        ),
    ] {
        let made = append(
            &base(),
            "curve",
            "previous",
            "b",
            "c",
            Position::mm(x, y),
            mode,
        );
        let mut intent = made.intent;
        let kept = intent.constraints.clone();
        point(&mut intent, "ref0", x, -30.0, true);
        point(&mut intent, "ref1", x, 30.0, true);
        intent.entities.insert(
            "refline".into(),
            Entity::Line {
                start: "ref0".into(),
                end: "ref1".into(),
            },
        );
        intent.constraints.insert(
            "endpoint-x".into(),
            Constraint::driving(Relation::On {
                point: "c".into(),
                curve: "refline".into(),
            }),
        );
        intent.constraints.insert(
            "radius".into(),
            Constraint::driving(Relation::Radius {
                curve: "curve:arc".into(),
                value: Expression::Literal(Quantity::length(10.0).unwrap()),
            }),
        );
        let result = solve(&intent, &SolveMode::Edit).unwrap();
        let candidate = result
            .candidate
            .unwrap_or_else(|| panic!("{mode:?}: {:?}", result.message));
        let endpoint = candidate.point(&"c".into()).unwrap();
        near(endpoint.x.0, x);
        near(endpoint.y.0, expected_y);
        let Entity::Arc { center, radius, .. } = &candidate.entities[&"curve:arc".into()] else {
            panic!("arc")
        };
        near(radius.0, 10.0);
        let center = candidate.point(center).unwrap();
        near(center.x.0, expected_center.x.0);
        near(center.y.0, expected_center.y.0);
        for (id, relation) in kept {
            assert_eq!(intent.constraints[&id], relation);
        }
        assert_eq!(
            candidate.point(&"b".into()).unwrap(),
            Position::mm(0.0, 0.0)
        );
    }
}
#[test]
fn mixed_chain_closes_by_existing_endpoint_identity() {
    let a = append(
        &base(),
        "arc",
        "previous",
        "b",
        "c",
        Position::mm(6.0, 6.0),
        ContinuationMode::TangentArc,
    );
    let b = append(
        &a.intent,
        "line",
        "arc:arc",
        "c",
        "d",
        Position::mm(6.0, 12.0),
        ContinuationMode::TangentLine,
    );
    let closed = append(
        &b.intent,
        "close",
        "line:line",
        "d",
        "a",
        Position::mm(-10.0, 0.0),
        ContinuationMode::FreeLine,
    );
    assert_eq!(closed.intent.entities.len(), b.intent.entities.len() + 1);
    let Entity::Line { start, end } = &closed.intent.entities[&"close:line".into()] else {
        panic!("line")
    };
    assert_eq!(start, &"d".into());
    assert_eq!(end, &"a".into());
    near(closed.candidate.point(&"d".into()).unwrap().x.0, 6.0);
    near(closed.candidate.point(&"d".into()).unwrap().y.0, 12.0);
}
