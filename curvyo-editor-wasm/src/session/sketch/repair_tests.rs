//! Workbench navigation never changes the sketch's persistent intent.
use super::*;
use curvyo_sketch_core::{Construction, Position};

#[test]
fn constraints_conflict_inspection_parameter_errors_and_origin_changes_are_undoable() {
    use curvyo_sketch_core::{
        Constraint, Entity, Expression, Length, Parameter, Quantity, Relation, SolveStatus,
    };
    let mut session = Session::new(69);
    session.set_workbench(Workbench::Sketch);
    let id = session.new_sketch().unwrap();
    session
        .sketch_construct(&Construction::Circle {
            center: Position::mm(20.0, 20.0),
            radius: Length(5.0),
        })
        .unwrap();
    let graph = session.document.sketch(id).unwrap().graph;
    let curve = graph
        .entities
        .iter()
        .find_map(|(id, e)| matches!(e, Entity::Circle { .. }).then_some(id.clone()))
        .unwrap();
    session
        .sketch_set_parameter(
            "r".into(),
            Parameter::literal("radius", Quantity::length(7.0).unwrap()),
        )
        .unwrap();
    session
        .sketch_set_constraint(
            "radius".into(),
            Constraint::driving(Relation::Radius {
                curve: curve.clone(),
                value: Expression::Reference("r".into()),
            }),
        )
        .unwrap();
    let before = session.document.sketch(id).unwrap().solution;
    session
        .sketch_set_constraint(
            "conflict".into(),
            Constraint::driving(Relation::Radius {
                curve,
                value: Expression::Literal(Quantity::length(8.0).unwrap()),
            }),
        )
        .unwrap();
    let view = session.document.sketch(id).unwrap();
    assert!(view.stale);
    assert!(matches!(
        view.status,
        Some(SolveStatus::Conflicting | SolveStatus::SolverFailure)
    ));
    assert_eq!(view.solution, before);
    session
        .sketch_remove_constraint(&"conflict".into())
        .unwrap();
    assert!(!session.document.sketch(id).unwrap().stale);
    let version = session.document.version();
    assert!(
        session
            .sketch_set_parameter(
                "r".into(),
                Parameter::expression(
                    "radius",
                    curvyo_sketch_core::Dimension::Length,
                    Expression::Reference("r".into())
                )
            )
            .is_err()
    );
    assert_eq!(session.document.version(), version);
    session.sketch_fix_origin(false).unwrap();
    assert!(matches!(
        session.document.sketch(id).unwrap().graph.entities[&graph.origin],
        Entity::Point { fixed: false, .. }
    ));
    session.undo();
    assert!(matches!(
        session.document.sketch(id).unwrap().graph.entities[&graph.origin],
        Entity::Point { fixed: true, .. }
    ));
}

#[test]
fn fillet_preview_is_cancelable_revision_checked_and_commits_once() {
    use curvyo_sketch_core::Length;
    let mut session = Session::new(75);
    session.set_workbench(Workbench::Sketch);
    let id = session.new_sketch().unwrap();
    session
        .sketch_construct_named(
            "corner",
            &Construction::Polyline(vec![
                Position::mm(0.0, 20.0),
                Position::mm(20.0, 20.0),
                Position::mm(20.0, 40.0),
            ]),
        )
        .unwrap();
    session.sketch.entities = ["corner:line0".into(), "corner:line1".into()].into();
    let history = session.undo_len();
    let before = session.document.sketch(id).unwrap().graph;
    session.sketch_preview_fillet(Length(3.0)).unwrap();
    assert!(session.sketch_construction_preview().is_some());
    assert_eq!(session.document.sketch(id).unwrap().graph, before);
    session.escape();
    assert!(session.sketch_construction_preview().is_none());
    assert_eq!(session.sketch.entities.len(), 2);
    assert_eq!(session.undo_len(), history);
    session.sketch_preview_fillet(Length(3.0)).unwrap();
    session.sketch_commit_preview().unwrap();
    assert_eq!(session.undo_len(), history + 1);
    session.undo();
    assert_eq!(session.document.sketch(id).unwrap().graph, before);
    session.sketch.entities = ["corner:line0".into(), "corner:line1".into()].into();
    session.sketch_preview_fillet(Length(3.0)).unwrap();
    session
        .document
        .apply_sketch_delta(
            id,
            &curvyo_document_core::SketchDelta {
                parameters: [(
                    "width".into(),
                    Some(curvyo_sketch_core::Parameter::literal(
                        "width",
                        curvyo_sketch_core::Quantity::length(10.0).unwrap(),
                    )),
                )]
                .into(),
                ..Default::default()
            },
        )
        .unwrap();
    assert!(session.sketch_construction_preview().is_none());
    assert!(session.sketch_commit_preview().is_err());
}

#[test]
fn literal_parameter_edit_repairs_a_cycle_created_by_peer_merge() {
    use curvyo_document_core::{SketchDelta, pack, unpack};
    use curvyo_sketch_core::{Dimension, Expression, Parameter, Quantity};
    let mut session = Session::new(76);
    session.set_workbench(Workbench::Sketch);
    let id = session.new_sketch().unwrap();
    for name in ["a", "b", "c", "d"] {
        session
            .sketch_set_parameter(
                name.into(),
                Parameter::literal(name, Quantity::length(10.0).unwrap()),
            )
            .unwrap();
    }
    let peer = unpack(77, &pack(&session.document, "test").unwrap()).unwrap();
    session
        .sketch_set_parameter(
            "a".into(),
            Parameter::expression("a", Dimension::Length, Expression::Reference("b".into())),
        )
        .unwrap();
    peer.apply_sketch_delta(
        id,
        &SketchDelta {
            parameters: [(
                "b".into(),
                Some(Parameter::expression(
                    "b",
                    Dimension::Length,
                    Expression::Reference("a".into()),
                )),
            )]
            .into(),
            ..Default::default()
        },
    )
    .unwrap();
    session
        .sketch_set_parameter(
            "c".into(),
            Parameter::expression("c", Dimension::Length, Expression::Reference("d".into())),
        )
        .unwrap();
    peer.apply_sketch_delta(
        id,
        &SketchDelta {
            parameters: [(
                "d".into(),
                Some(Parameter::expression(
                    "d",
                    Dimension::Length,
                    Expression::Reference("c".into()),
                )),
            )]
            .into(),
            ..Default::default()
        },
    )
    .unwrap();
    session
        .document
        .import_updates(&peer.export_updates(&session.document.version()).unwrap())
        .unwrap();
    assert!(parameters_invalid(&session, id));
    session
        .sketch_parameter_text("a".into(), "a", "12", Dimension::Length)
        .unwrap();
    assert!(parameters_invalid(&session, id));
    assert!(session.document.sketch(id).unwrap().stale);
    // A local newly cyclic edit remains refused while the other component is broken.
    assert!(
        session
            .sketch_parameter_text("a".into(), "a", "b", Dimension::Length)
            .is_err()
    );
    session
        .sketch_parameter_text("c".into(), "c", "8", Dimension::Length)
        .unwrap();
    let values = session
        .document
        .sketch(id)
        .unwrap()
        .graph
        .parameters
        .evaluate()
        .unwrap();
    assert_eq!(values[&"a".into()].value(), 12.0);
    assert_eq!(values[&"b".into()].value(), 12.0);
}

#[test]
fn dangling_relationships_created_by_merge_can_be_removed_individually() {
    use curvyo_document_core::{SketchDelta, pack, unpack};
    use curvyo_sketch_core::{Constraint, Relation};
    let mut session = Session::new(81);
    session.set_workbench(Workbench::Sketch);
    let id = session.new_sketch().unwrap();
    for name in ["a", "b"] {
        session
            .sketch_construct_named(name, &Construction::Point(Position::mm(10.0, 20.0)))
            .unwrap();
    }
    let graph = session.document.sketch(id).unwrap().graph;
    let points = graph
        .entities
        .keys()
        .filter(|key| **key != graph.origin)
        .cloned()
        .collect::<Vec<_>>();
    let peer = unpack(82, &pack(&session.document, "test").unwrap()).unwrap();
    let constraints = points
        .iter()
        .enumerate()
        .map(|(index, id)| {
            (
                format!("fixed{index}").into(),
                Some(Constraint::driving(Relation::Fixed(id.clone()))),
            )
        })
        .collect();
    peer.apply_sketch_delta(
        id,
        &SketchDelta {
            constraints,
            ..Default::default()
        },
    )
    .unwrap();
    session
        .sketch_delete_entities(&points.into_iter().collect())
        .unwrap();
    session
        .document
        .import_updates(&peer.export_updates(&session.document.version()).unwrap())
        .unwrap();
    assert!(
        session
            .document
            .sketch(id)
            .unwrap()
            .graph
            .validate()
            .is_err()
    );
    session.sketch_remove_constraint(&"fixed0".into()).unwrap();
    assert!(
        session
            .document
            .sketch(id)
            .unwrap()
            .graph
            .validate()
            .is_err()
    );
    session.sketch_remove_constraint(&"fixed1".into()).unwrap();
    session
        .document
        .sketch(id)
        .unwrap()
        .graph
        .validate()
        .unwrap();
}

fn parameters_invalid(session: &Session, id: curvyo_document_core::NodeId) -> bool {
    session
        .document
        .sketch(id)
        .unwrap()
        .graph
        .parameters
        .evaluate()
        .is_err()
}

#[test]
fn rejected_numeric_fillet_apply_invalidates_preview_without_history() {
    let mut session = Session::new(92);
    session.set_workbench(Workbench::Sketch);
    let id = session.new_sketch().unwrap();
    session
        .sketch_construct_named(
            "corner",
            &Construction::Polyline(vec![
                Position::mm(0.0, 20.0),
                Position::mm(20.0, 20.0),
                Position::mm(20.0, 40.0),
            ]),
        )
        .unwrap();
    session.set_sketch_tool(super::SketchTool::Fillet);
    session.sketch.entities = ["corner:line0".into(), "corner:line1".into()].into();
    let before = session.document.sketch(id).unwrap().graph;
    let steps = session.undo_len();
    for invalid in ["unknown", "2 rad", "-1 mm", ""] {
        session.sketch_preview_fillet_text("2 mm").unwrap();
        assert!(session.sketch_complete_numeric(invalid, "").is_err());
        assert!(session.sketch.error.is_some());
        assert!(session.sketch.preview.is_none());
        assert!(session.sketch.preview_base.is_none());
        assert!(session.sketch_commit_preview().is_err());
        assert_eq!(session.document.sketch(id).unwrap().graph, before);
        assert_eq!(session.undo_len(), steps);
    }
    assert!(session.sketch_complete_numeric("1 mm", "").unwrap());
    assert_eq!(session.undo_len(), steps + 1);
    assert!(session.sketch.error.is_none());
}
