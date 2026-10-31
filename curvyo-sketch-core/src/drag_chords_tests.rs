//! Independent chord witnesses cover exact goals and safe refusal of uncertified poses.
#![allow(clippy::unwrap_used)]
use super::*;
use crate::{Constraint, Entity, Expression, Quantity, Relation};
fn graph() -> (Sketch, Vec<EntityId>) {
    let mut s = Sketch::new("a".into());
    let points = [
        ("a", 0.0, 0.0),
        ("b", 3.0, 4.0),
        ("c", 6.0, 4.0),
        ("d", 9.0, 0.0),
        ("e", 12.0, 0.0),
    ];
    for (id, x, y) in points {
        s.entities.insert(
            id.into(),
            Entity::Point {
                position: Position::mm(x, y),
                fixed: id == "a",
            },
        );
    }
    s.entities.insert(
        "unrelated".into(),
        Entity::Point {
            position: Position::mm(80.0, 90.0),
            fixed: false,
        },
    );
    for (index, pair) in points.windows(2).enumerate() {
        s.constraints.insert(
            format!("distance{index}").into(),
            Constraint::driving(Relation::Distance {
                a: pair[0].0.into(),
                b: pair[1].0.into(),
                value: Expression::Literal(
                    Quantity::length(
                        s.point(&pair[0].0.into())
                            .unwrap()
                            .distance(s.point(&pair[1].0.into()).unwrap())
                            .0,
                    )
                    .unwrap(),
                ),
            }),
        );
    }
    (s, points.iter().map(|p| p.0.into()).collect())
}
#[test]
fn both_branches_small_steps_and_reversal_reach_goal_without_disconnected_motion() {
    for mirror in [1.0, -1.0] {
        let (mut s, ids) = graph();
        for e in s.entities.values_mut() {
            if let Entity::Point { position, .. } = e {
                position.y.0 *= mirror;
            }
        }
        let original = s.clone();
        for i in (1..11).chain((1..10).rev()) {
            let goal = Position::mm(12.0 - f64::from(i) * 0.03, f64::from(i) * 0.04 * mirror);
            let next = candidate(&s, &ids, goal).unwrap();
            crate::validate_candidate(&original, &next).unwrap();
            assert!(next.point(&"e".into()).unwrap().distance(goal).0 <= 1e-6);
            assert_eq!(
                next.entities[&"unrelated".into()],
                original.entities[&"unrelated".into()]
            );
            for id in &ids {
                assert!(next.point(id).unwrap().distance(s.point(id).unwrap()).0 < 0.15);
            }
            assert_eq!(next.constraints, original.constraints);
            s = next;
        }
    }
}
#[test]
fn fixed_interior_and_extra_equations_refuse_candidate() {
    let (mut s, ids) = graph();
    let target = Position::mm(11.0, 1.0);
    s.constraints.insert(
        "fixed".into(),
        Constraint::driving(Relation::Fixed("b".into())),
    );
    assert!(candidate(&s, &ids, target).is_none());
    s.constraints.remove(&"fixed".into());
    s.constraints.insert(
        "axis".into(),
        Constraint::driving(Relation::OnAxis {
            point: "e".into(),
            axis: crate::Axis::X,
        }),
    );
    assert!(candidate(&s, &ids, target).is_none());
}
#[test]
fn unattainable_tangent_and_coincident_circles_fall_back() {
    let (s, ids) = graph();
    assert!(candidate(&s, &ids, Position::mm(100.0, 0.0)).is_none());
    assert!(candidate(&s, &ids, Position::mm(0.0, 0.0)).is_none());
    let r = s
        .point(&"c".into())
        .unwrap()
        .distance(Position::mm(0.0, 0.0))
        .0;
    assert!(candidate(&s, &ids, Position::mm(2.0 * r, 0.0)).is_none());
    assert!(candidate(&s, &ids[..1], Position::mm(1.0, 0.0)).is_none());
}

#[test]
fn certificate_eligibility_preserves_multigoal_branch_and_parameter_rules() {
    use crate::{Parameter, SolveMode};
    use std::collections::BTreeMap;
    let (mut s, _) = graph();
    let target = Position::mm(11.8, 0.2);
    let mode = SolveMode::Drag {
        point: "e".into(),
        target,
    };
    s.parameters.insert(
        "length".into(),
        Parameter::literal("length", Quantity::length(5.0).unwrap()),
    );
    if let Relation::Distance { value, .. } =
        &mut s.constraints.get_mut(&"distance0".into()).unwrap().relation
    {
        *value = Expression::Reference("length".into());
    }
    assert!(crate::drag_reach::candidate(&s, &mode).is_some());
    let goals = BTreeMap::from([("d".into(), target), ("e".into(), target)]);
    assert!(crate::drag_reach::candidate(&s, &SolveMode::DragPoints { goals }).is_none());
    assert!(crate::drag_reach::candidate(&s, &SolveMode::Edit).is_none());
    let length = s
        .point(&"c".into())
        .unwrap()
        .distance(s.point(&"unrelated".into()).unwrap())
        .0;
    s.constraints.insert(
        "branch".into(),
        Constraint::driving(Relation::Distance {
            a: "c".into(),
            b: "unrelated".into(),
            value: Expression::Literal(Quantity::length(length).unwrap()),
        }),
    );
    assert!(crate::drag_reach::candidate(&s, &mode).is_none());
    s.constraints.remove(&"branch".into());
    s.parameters.insert(
        "length".into(),
        Parameter::literal("length", Quantity::length(6.0).unwrap()),
    );
    assert!(
        crate::drag_reach::candidate(&s, &mode).is_none(),
        "current seed does not satisfy changed parameter"
    );
}

#[test]
fn zero_chord_and_reflection_cannot_be_accepted_as_rigid_rotation() {
    let (mut s, ids) = graph();
    let original = s.clone();
    if let Entity::Point { position, .. } = s.entities.get_mut(&"c".into()).unwrap() {
        *position = Position::mm(0.0, 0.0);
    }
    assert!(candidate(&s, &ids, Position::mm(11.0, 1.0)).is_none());
    let next = candidate(&original, &ids, Position::mm(11.8, 0.2)).unwrap();
    let area = |s: &Sketch| {
        let a = s.point(&"a".into()).unwrap();
        let b = s.point(&"b".into()).unwrap();
        let c = s.point(&"c".into()).unwrap();
        (b.x.0 - a.x.0) * (c.y.0 - a.y.0) - (b.y.0 - a.y.0) * (c.x.0 - a.x.0)
    };
    assert!((area(&original) - area(&next)).abs() < 1e-9);
}

#[test]
fn analytic_five_five_witness_selects_least_displacement_branch() {
    for side in [1.0, -1.0] {
        let mut s = Sketch::new("a".into());
        for (id, x, y) in [("b", 4.0, side * 3.0), ("c", 8.0, 0.0)] {
            s.entities.insert(
                id.into(),
                Entity::Point {
                    position: Position::mm(x, y),
                    fixed: false,
                },
            );
        }
        for (id, a, b) in [("ab", "a", "b"), ("bc", "b", "c")] {
            s.constraints.insert(
                id.into(),
                Constraint::driving(Relation::Distance {
                    a: a.into(),
                    b: b.into(),
                    value: Expression::Literal(Quantity::length(5.0).unwrap()),
                }),
            );
        }
        let original = s.clone();
        let ids = vec!["a".into(), "b".into(), "c".into()];
        for (x, expected) in [
            (6.0, Position::mm(3.0, side * 4.0)),
            (8.0, Position::mm(4.0, side * 3.0)),
        ] {
            s = candidate(&s, &ids, Position::mm(x, 0.0)).unwrap();
            crate::validate_candidate(&original, &s).unwrap();
            assert!(s.point(&"b".into()).unwrap().distance(expected).0 < 1e-9);
        }
    }
}

#[test]
fn coherent_suffix_reaches_every_goal_and_rejects_gaps() {
    use crate::SolveMode;
    use std::collections::BTreeMap;
    let (s, _) = graph();
    let original = s.clone();
    for ids in [vec!["d", "e"], vec!["c", "d", "e"]] {
        let goals: BTreeMap<_, _> = ids
            .iter()
            .map(|id| {
                let p = s.point(&(*id).into()).unwrap();
                ((*id).into(), Position::mm(p.x.0 - 0.2, p.y.0 + 0.2))
            })
            .collect();
        let result = crate::drag_reach::candidate(
            &s,
            &SolveMode::DragPoints {
                goals: goals.clone(),
            },
        )
        .unwrap();
        crate::validate_candidate(&s, &result).unwrap();
        for (id, goal) in goals {
            assert!(result.point(&id).unwrap().distance(goal).0 <= 1e-6);
        }
        assert_eq!(
            result.entities[&"unrelated".into()],
            s.entities[&"unrelated".into()]
        );
    }
    for ids in [vec!["c", "e"], vec!["unrelated", "e"]] {
        let goals = ids
            .iter()
            .map(|id| {
                let p = s.point(&(*id).into()).unwrap();
                ((*id).into(), Position::mm(p.x.0 - 0.2, p.y.0 + 0.2))
            })
            .collect();
        assert!(crate::drag_reach::candidate(&s, &SolveMode::DragPoints { goals }).is_none());
    }
    assert_eq!(s, original);
}

#[test]
fn translated_suffix_analytic_branch_reversal_and_guard_witnesses() {
    use crate::SolveMode;
    use std::collections::BTreeMap;
    for side in [1.0, -1.0] {
        let (mut s, _) = graph();
        for (id, x, y) in [
            ("b", 4.0, 3.0),
            ("c", 8.0, 0.0),
            ("d", 11.0, 4.0),
            ("e", 14.0, 0.0),
        ] {
            if let Entity::Point { position, .. } = s.entities.get_mut(&id.into()).unwrap() {
                *position = Position::mm(x, side * y);
            }
        }
        for c in s.constraints.values_mut() {
            if let Relation::Distance { value, .. } = &mut c.relation {
                *value = Expression::Literal(Quantity::length(5.0).unwrap());
            }
        }
        let original = s.clone();
        let goals = BTreeMap::from([
            ("c".into(), Position::mm(6.0, 0.0)),
            ("d".into(), Position::mm(9.0, side * 4.0)),
            ("e".into(), Position::mm(12.0, 0.0)),
        ]);
        let mode = SolveMode::DragPoints {
            goals: goals.clone(),
        };
        let result = crate::drag_reach::candidate(&s, &mode).unwrap();
        assert!(
            result
                .point(&"b".into())
                .unwrap()
                .distance(Position::mm(3.0, side * 4.0))
                .0
                < 1e-9
        );
        let reverse = goals
            .keys()
            .map(|id| (id.clone(), s.point(id).unwrap()))
            .collect();
        let reversed =
            crate::drag_reach::candidate(&result, &SolveMode::DragPoints { goals: reverse })
                .unwrap();
        for id in ["a", "b", "c", "d", "e"] {
            assert!(
                reversed
                    .point(&id.into())
                    .unwrap()
                    .distance(s.point(&id.into()).unwrap())
                    .0
                    < 1e-9
            );
        }
        let mut incoherent = goals;
        incoherent.insert("d".into(), Position::mm(10.0, side * 4.0));
        assert!(
            crate::drag_reach::candidate(&s, &SolveMode::DragPoints { goals: incoherent })
                .is_none()
        );
        s.constraints.insert(
            "fixed".into(),
            Constraint::driving(Relation::Fixed("d".into())),
        );
        assert!(crate::drag_reach::candidate(&s, &mode).is_none());
        s.constraints.remove(&"fixed".into());
        s.constraints.insert(
            "cross".into(),
            Constraint::driving(Relation::Distance {
                a: "a".into(),
                b: "d".into(),
                value: Expression::Literal(
                    Quantity::length(
                        s.point(&"d".into())
                            .unwrap()
                            .distance(Position::mm(0.0, 0.0))
                            .0,
                    )
                    .unwrap(),
                ),
            }),
        );
        assert!(crate::drag_reach::candidate(&s, &mode).is_none());
        s.constraints.remove(&"cross".into());
        assert_eq!(s, original);
    }
}

#[test]
fn malformed_first_or_later_suffix_goal_cannot_be_certified() {
    use crate::SolveMode;
    use std::collections::BTreeMap;
    let (s, _) = graph();
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        for id in ["d", "e"] {
            for axis in [0, 1] {
                let mut goals = BTreeMap::from([
                    ("d".into(), Position::mm(8.8, 0.2)),
                    ("e".into(), Position::mm(11.8, 0.2)),
                ]);
                let point = goals.get_mut(&EntityId::from(id)).unwrap();
                if axis == 0 {
                    point.x.0 = bad;
                } else {
                    point.y.0 = bad;
                }
                assert!(
                    crate::drag_reach::candidate(&s, &SolveMode::DragPoints { goals }).is_none()
                );
            }
        }
    }
}
