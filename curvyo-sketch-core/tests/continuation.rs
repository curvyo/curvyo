//! Mixed analytic chains preserve shared endpoints and explicit directional intent.
#![allow(clippy::unwrap_used)]
use curvyo_sketch_core::*;
#[test]
fn line_tangent_arc_tangent_line_share_ids_and_keep_endpoint_directions() {
    let base = construct(
        &Sketch::new("o".into()),
        "first",
        &Construction::Polyline(vec![Position::mm(0.0, 0.0), Position::mm(10.0, 0.0)]),
    )
    .unwrap()
    .candidate;
    let arc = construct(
        &base,
        "turn",
        &Construction::Continue {
            previous: "first:line0".into(),
            start: "first:p1".into(),
            end: "turn:end".into(),
            position: Position::mm(20.0, 10.0),
            mode: ContinuationMode::TangentArc,
        },
    )
    .unwrap();
    let Entity::Arc { start, end, .. } = &arc.candidate.entities[&"turn:arc".into()] else {
        panic!("arc")
    };
    assert!(start == &EntityId::from("first:p1") || end == &EntityId::from("first:p1"));
    validate_candidate(&arc.intent, &arc.candidate).unwrap();
    let line = construct(
        &arc.candidate,
        "next",
        &Construction::Continue {
            previous: "turn:arc".into(),
            start: "turn:end".into(),
            end: "next:end".into(),
            position: Position::mm(20.0, 20.0),
            mode: ContinuationMode::TangentLine,
        },
    )
    .unwrap();
    assert!(
        line.candidate
            .point(&"next:end".into())
            .unwrap()
            .distance(Position::mm(20.0, 20.0))
            .0
            < 1e-6
    );
    validate_candidate(&line.intent, &line.candidate).unwrap();
}

#[test]
fn both_perpendicular_arc_branches_keep_endpoint_normal_relationship() {
    let base = construct(
        &Sketch::new("o".into()),
        "first",
        &Construction::Polyline(vec![Position::mm(0.0, 0.0), Position::mm(10.0, 0.0)]),
    )
    .unwrap()
    .candidate;
    for mode in [
        ContinuationMode::PerpendicularArcLeft,
        ContinuationMode::PerpendicularArcRight,
    ] {
        let arc = construct(
            &base,
            "turn",
            &Construction::Continue {
                previous: "first:line0".into(),
                start: "first:p1".into(),
                end: "turn:end".into(),
                position: Position::mm(20.0, 10.0),
                mode,
            },
        )
        .unwrap();
        assert!(
            arc.intent
                .constraints
                .values()
                .any(|c| matches!(c.relation, Relation::PerpendicularAt { .. }))
        );
        validate_candidate(&arc.intent, &arc.candidate).unwrap();
        let tangent =
            endpoint_tangent(&arc.candidate, &"turn:arc".into(), &"first:p1".into()).unwrap();
        assert!(tangent.0.abs() < 1e-6);
    }
}

#[test]
fn perpendicular_arc_pair_survives_radius_edit_and_remapping() {
    let base = construct(
        &Sketch::new("o".into()),
        "first",
        &Construction::Polyline(vec![Position::mm(0.0, 0.0), Position::mm(10.0, 0.0)]),
    )
    .unwrap()
    .candidate;
    let first = construct(
        &base,
        "turn",
        &Construction::Continue {
            previous: "first:line0".into(),
            start: "first:p1".into(),
            end: "turn:end".into(),
            position: Position::mm(20.0, 10.0),
            mode: ContinuationMode::TangentArc,
        },
    )
    .unwrap()
    .candidate;
    let mut next = construct(
        &first,
        "next",
        &Construction::Continue {
            previous: "turn:arc".into(),
            start: "turn:end".into(),
            end: "next:end".into(),
            position: Position::mm(30.0, 20.0),
            mode: ContinuationMode::PerpendicularArcLeft,
        },
    )
    .unwrap()
    .intent;
    next.constraints.insert(
        "radius".into(),
        Constraint::driving(Relation::Radius {
            curve: "next:arc".into(),
            value: Expression::Literal(Quantity::length(12.0).unwrap()),
        }),
    );
    let result = solve(&next, &SolveMode::Edit).unwrap();
    validate_candidate(&next, &result.candidate.unwrap()).unwrap();
    let copied = next.with_fresh_namespace("copy");
    copied.validate().unwrap();
    let json = serde_json::to_string(&copied).unwrap();
    let roundtrip: Sketch = serde_json::from_str(&json).unwrap();
    assert_eq!(copied, roundtrip);
    let result = solve(&roundtrip, &SolveMode::Edit).unwrap();
    validate_candidate(&roundtrip, &result.candidate.unwrap()).unwrap();
}

#[test]
fn typed_arc_radius_retains_start_and_tangent_on_both_branches() {
    let base = construct(
        &Sketch::new("o".into()),
        "first",
        &Construction::Polyline(vec![Position::mm(10.0, 10.0), Position::mm(20.0, 10.0)]),
    )
    .unwrap()
    .candidate;
    for toward in [Position::mm(30.0, 20.0), Position::mm(30.0, 0.0)] {
        let end =
            arc_radius_endpoint(Position::mm(20.0, 10.0), (1.0, 0.0), Length(7.0), toward).unwrap();
        let result = construct(
            &base,
            "turn",
            &Construction::Continue {
                previous: "first:line0".into(),
                start: "first:p1".into(),
                end: "turn:end".into(),
                position: end,
                mode: ContinuationMode::TangentArc,
            },
        )
        .unwrap();
        let Entity::Arc { radius, .. } = result.candidate.entities[&"turn:arc".into()] else {
            panic!("arc")
        };
        assert!((radius.0 - 7.0).abs() < 1e-6);
        assert!(
            result
                .candidate
                .point(&"first:p1".into())
                .unwrap()
                .distance(Position::mm(20.0, 10.0))
                .0
                < 1e-6
        );
        validate_candidate(&result.intent, &result.candidate).unwrap();
    }
}

#[test]
fn closing_arc_reuses_existing_endpoint_without_creating_a_duplicate_point() {
    let base = construct(
        &Sketch::new("o".into()),
        "first",
        &Construction::Polyline(vec![
            Position::mm(10.0, 10.0),
            Position::mm(20.0, 10.0),
            Position::mm(20.0, 20.0),
        ]),
    )
    .unwrap()
    .candidate;
    let result = construct(
        &base,
        "close",
        &Construction::Continue {
            previous: "first:line1".into(),
            start: "first:p2".into(),
            end: "first:p0".into(),
            position: Position::mm(10.0, 10.0),
            mode: ContinuationMode::TangentArc,
        },
    )
    .unwrap();
    assert_eq!(result.intent.entities.len(), base.entities.len() + 2);
    let Entity::Arc { start, end, .. } = &result.intent.entities[&"close:arc".into()] else {
        panic!("arc")
    };
    assert!(start == &EntityId::from("first:p0") || end == &EntityId::from("first:p0"));
    validate_candidate(&result.intent, &result.candidate).unwrap();
}

#[test]
fn centre_arc_projects_only_final_direction_onto_pinned_radius() {
    let result = construct(
        &Sketch::new("o".into()),
        "arc",
        &Construction::ArcCenter {
            center: Position::mm(10.0, 20.0),
            start: Position::mm(15.0, 20.0),
            end: Position::mm(10.0, 30.0),
        },
    )
    .unwrap();
    assert!(
        result
            .candidate
            .point(&"arc:center".into())
            .unwrap()
            .distance(Position::mm(10.0, 20.0))
            .0
            < 1e-6
    );
    assert!(
        result
            .candidate
            .point(&"arc:start".into())
            .unwrap()
            .distance(Position::mm(15.0, 20.0))
            .0
            < 1e-6
    );
    assert!(
        result
            .candidate
            .point(&"arc:end".into())
            .unwrap()
            .distance(Position::mm(10.0, 25.0))
            .0
            < 1e-6
    );
    validate_candidate(&result.intent, &result.candidate).unwrap();
}

#[test]
fn three_point_arc_resumes_existing_endpoint_in_either_canonical_direction() {
    let base = construct(
        &Sketch::new("o".into()),
        "line",
        &Construction::Polyline(vec![Position::mm(10.0, 10.0), Position::mm(20.0, 10.0)]),
    )
    .unwrap()
    .candidate;
    for through in [Position::mm(25.0, 15.0), Position::mm(25.0, 5.0)] {
        let result = construct(
            &base,
            "arc",
            &Construction::ArcFrom {
                start: "line:p1".into(),
                through,
                end: Position::mm(30.0, 10.0),
            },
        )
        .unwrap();
        assert_eq!(result.intent.entities.len(), base.entities.len() + 3);
        let Entity::Arc { start, end, .. } = &result.intent.entities[&"arc:curve".into()] else {
            panic!("arc")
        };
        assert!(start == &EntityId::from("line:p1") || end == &EntityId::from("line:p1"));
        assert!(
            result
                .candidate
                .point(&"line:p1".into())
                .unwrap()
                .distance(Position::mm(20.0, 10.0))
                .0
                < 1e-6
        );
        validate_candidate(&result.intent, &result.candidate).unwrap();
    }
}
