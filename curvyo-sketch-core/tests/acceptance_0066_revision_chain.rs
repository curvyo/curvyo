//! Independent analytic chain witnesses written before reading the shortcut implementation.
#![allow(clippy::unwrap_used)]
use curvyo_sketch_core::*;
use std::collections::BTreeMap;
fn seed(points: &[(f64, f64)]) -> Sketch {
    let mut g = Sketch::new("origin".into());
    for (i, &(x, y)) in points.iter().enumerate() {
        let id = format!("p{i}");
        g.entities.insert(
            id.clone().into(),
            Entity::Point {
                position: Position::mm(x, y),
                fixed: i == 0,
            },
        );
        if i > 0 {
            g.entities.insert(
                format!("l{i}").into(),
                Entity::Line {
                    start: format!("p{}", i - 1).into(),
                    end: id.clone().into(),
                },
            );
            g.constraints.insert(
                format!("d{i}").into(),
                Constraint::driving(Relation::Distance {
                    a: format!("p{}", i - 1).into(),
                    b: id.into(),
                    value: Expression::Literal(Quantity::length(5.0).unwrap()),
                }),
            );
        }
    }
    g
}
fn near(a: f64, b: f64) {
    assert!((a - b).abs() <= 1e-6, "{a} != {b}");
}
fn valid(g: &Sketch, n: usize) {
    near(
        g.point(&"p0".into())
            .unwrap()
            .distance(Position::mm(0.0, 0.0))
            .0,
        0.0,
    );
    for i in 1..n {
        near(
            g.point(&format!("p{i}").into())
                .unwrap()
                .distance(g.point(&format!("p{}", i - 1).into()).unwrap())
                .0,
            5.0,
        );
    }
}
fn drag(g: &Sketch, goals: &[(usize, f64, f64)]) -> Sketch {
    let before = g.clone();
    let result = solve(
        g,
        &SolveMode::DragPoints {
            goals: goals
                .iter()
                .map(|&(i, x, y)| (format!("p{i}").into(), Position::mm(x, y)))
                .collect(),
        },
    )
    .unwrap();
    assert_eq!(g, &before);
    let candidate = result.candidate.unwrap();
    assert_eq!(candidate.constraints, g.constraints);
    candidate
}
#[test]
fn two_link_branch_continuity_reversal_and_global_limit() {
    for sign in [-1.0, 1.0] {
        let initial = seed(&[(0.0, 0.0), (4.0, sign * 3.0), (8.0, 0.0)]);
        let moved = drag(&initial, &[(2, 6.0, 0.0)]);
        valid(&moved, 3);
        let position = moved.point(&"p1".into()).unwrap();
        near(position.x.0, 3.0);
        near(position.y.0, sign * 4.0);
        let returned = drag(&moved, &[(2, 8.0, 0.0)]);
        valid(&returned, 3);
        let position = returned.point(&"p1".into()).unwrap();
        near(position.x.0, 4.0);
        near(position.y.0, sign * 3.0);
        let limited = drag(&initial, &[(2, 20.0, 20.0)]);
        valid(&limited, 3);
        let position = limited.point(&"p2".into()).unwrap();
        near(position.x.0, 10.0 / 2.0_f64.sqrt());
        near(position.y.0, 10.0 / 2.0_f64.sqrt());
    }
}
#[test]
fn coherent_suffix_checks_every_goal_and_whole_pose_branch() {
    for sign in [-1.0, 1.0] {
        let g = seed(&[
            (0.0, 0.0),
            (4.0, sign * 3.0),
            (8.0, 0.0),
            (11.0, sign * 4.0),
            (14.0, 0.0),
        ]);
        let goals = [(2, 6.0, 0.0), (3, 9.0, sign * 4.0), (4, 12.0, 0.0)];
        let a = drag(&g, &goals);
        valid(&a, 5);
        for &(i, x, y) in &goals {
            near(
                a.point(&format!("p{i}").into())
                    .unwrap()
                    .distance(Position::mm(x, y))
                    .0,
                0.0,
            );
        }
        let p = a.point(&"p1".into()).unwrap();
        near(p.x.0, 3.0);
        near(p.y.0, sign * 4.0);
        let b = drag(&a, &[(4, 14.0, 0.0), (3, 11.0, sign * 4.0), (2, 8.0, 0.0)]);
        valid(&b, 5);
        for i in 0..5 {
            near(
                b.point(&format!("p{i}").into())
                    .unwrap()
                    .distance(g.point(&format!("p{i}").into()).unwrap())
                    .0,
                0.0,
            );
        }
    }
}
#[test]
fn invalid_first_and_later_raw_goals_never_produce_a_candidate() {
    let g = seed(&[(0.0, 0.0), (4.0, 3.0), (8.0, 0.0), (11.0, 4.0), (14.0, 0.0)]);
    let before = g.clone();
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        for bad in [2, 4] {
            for axis in [0, 1] {
                let mut goals: BTreeMap<_, _> = [
                    ("p2".into(), Position::mm(6.0, 0.0)),
                    ("p3".into(), Position::mm(9.0, 4.0)),
                    ("p4".into(), Position::mm(12.0, 0.0)),
                ]
                .into();
                let target = goals.get_mut(&format!("p{bad}").into()).unwrap();
                if axis == 0 {
                    target.x = Length(value);
                } else {
                    target.y = Length(value);
                }
                if let Ok(result) = solve(&g, &SolveMode::DragPoints { goals }) {
                    assert!(
                        result.candidate.is_none(),
                        "invalid {bad}/{axis}/{value} accepted"
                    );
                }
                assert_eq!(g, before);
            }
        }
    }
}
#[test]
fn suffix_guards_preserve_fixed_and_extra_equations() {
    let original = seed(&[(0.0, 0.0), (4.0, 3.0), (8.0, 0.0), (11.0, 4.0), (14.0, 0.0)]);
    for kind in ["fixed", "crosslink", "gap", "incoherent"] {
        let mut g = original.clone();
        if kind == "fixed" {
            let Entity::Point { fixed, .. } = g.entities.get_mut(&"p3".into()).unwrap() else {
                panic!()
            };
            *fixed = true;
        }
        if kind == "crosslink" {
            g.constraints.insert(
                "cross".into(),
                Constraint::driving(Relation::Distance {
                    a: "p0".into(),
                    b: "p3".into(),
                    value: Expression::Literal(Quantity::length(137.0_f64.sqrt()).unwrap()),
                }),
            );
        }
        let mut goals = vec![(2, 6.0, 0.0), (3, 9.0, 4.0), (4, 12.0, 0.0)];
        if kind == "gap" {
            goals.remove(1);
        }
        if kind == "incoherent" {
            goals[1].1 = 10.0;
        }
        let before = g.clone();
        let result = solve(
            &g,
            &SolveMode::DragPoints {
                goals: goals
                    .iter()
                    .map(|&(i, x, y)| (format!("p{i}").into(), Position::mm(x, y)))
                    .collect(),
            },
        );
        assert_eq!(g, before);
        if let Ok(SolveResult {
            candidate: Some(c), ..
        }) = result
        {
            valid(&c, 5);
            assert_eq!(c.constraints, g.constraints);
            if kind == "fixed" {
                assert_eq!(c.entities[&"p3".into()], g.entities[&"p3".into()]);
            }
            if kind == "crosslink" {
                near(
                    c.point(&"p3".into())
                        .unwrap()
                        .distance(Position::mm(0.0, 0.0))
                        .0,
                    137.0_f64.sqrt(),
                );
            }
            if kind == "incoherent" {
                assert!(goals.iter().any(|&(i, x, y)| {
                    c.point(&format!("p{i}").into())
                        .unwrap()
                        .distance(Position::mm(x, y))
                        .0
                        > 1e-6
                }));
            }
        }
    }
}
#[test]
fn four_link_proper_rotations_and_degenerate_certificates_are_finite() {
    let g = seed(&[(0.0, 0.0), (3.0, 4.0), (6.0, 0.0), (9.0, 4.0), (12.0, 0.0)]);
    let c = drag(&g, &[(4, 8.0, 0.0)]);
    valid(&c, 5);
    let h = 20.0_f64.sqrt();
    let poses = [1.0, -1.0].map(|s| {
        [
            (0.0, 0.0),
            (2.0 - 2.0 * s * h / 3.0, 8.0 / 3.0 + s * h / 2.0),
            (4.0, s * h),
            (6.0 + 2.0 * s * h / 3.0, 8.0 / 3.0 + s * h / 2.0),
            (8.0, 0.0),
        ]
    });
    let costs = poses.map(|points| {
        points
            .iter()
            .enumerate()
            .map(|(i, &(x, y))| {
                g.point(&format!("p{i}").into())
                    .unwrap()
                    .distance(Position::mm(x, y))
                    .0
                    .powi(2)
            })
            .sum::<f64>()
    });
    let expected = poses[usize::from(costs[1] < costs[0])];
    for (i, (x, y)) in expected.into_iter().enumerate() {
        near(
            c.point(&format!("p{i}").into())
                .unwrap()
                .distance(Position::mm(x, y))
                .0,
            0.0,
        );
    }
    let folded = seed(&[(0.0, 0.0), (5.0, 0.0), (0.0, 0.0)]);
    let first = drag(&folded, &[(2, 6.0, 0.0)]);
    let second = drag(&folded, &[(2, 6.0, 0.0)]);
    assert_eq!(first, second);
    valid(&first, 3);
    near(first.point(&"p1".into()).unwrap().x.0, 3.0);
    near(first.point(&"p1".into()).unwrap().y.0.abs(), 4.0);
    let tangent = drag(&first, &[(2, 10.0, 0.0)]);
    println!(
        "tangent endpoint={:?} joint={:?}",
        tangent.point(&"p2".into()).unwrap(),
        tangent.point(&"p1".into()).unwrap()
    );
    valid(&tangent, 3);
    let end = tangent.point(&"p2".into()).unwrap();
    near(end.distance(Position::mm(10.0, 0.0)).0, 0.0);
    // At this double root, raw-goal tolerance propagates as sqrt(error), not linearly.
    let d = end.x.0.hypot(end.y.0);
    let h = (25.0 - d * d / 4.0).max(0.0).sqrt();
    let sign = first.point(&"p1".into()).unwrap().y.0.signum();
    let expected = Position::mm(
        end.x.0 / 2.0 - sign * h * end.y.0 / d,
        end.y.0 / 2.0 + sign * h * end.x.0 / d,
    );
    near(
        tangent.point(&"p1".into()).unwrap().distance(expected).0,
        0.0,
    );
}
