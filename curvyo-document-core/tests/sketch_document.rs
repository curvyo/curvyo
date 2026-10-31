//! Parametric objects retain intent across storage and explicit edits.
#![allow(clippy::unwrap_used)]
use curvyo_document_core::{
    Affine, Document, SketchDelta, SketchEntityRecord, Style, pack, unpack,
};
use curvyo_sketch_core::{Entity, Position, Sketch};

#[test]
fn sketch_roundtrip_keeps_free_point_edit_and_reads_write_nothing() {
    let doc = Document::new(700);
    let mut graph = Sketch::new("origin".into());
    graph.entities.insert(
        "point".into(),
        Entity::Point {
            position: Position::mm(10.0, 20.0),
            fixed: false,
        },
    );
    let id = doc
        .create_sketch(&graph, Affine::IDENTITY, &Style::default())
        .unwrap();
    let mut delta = SketchDelta::default();
    delta.entities.insert(
        "point".into(),
        Some(SketchEntityRecord {
            entity: Entity::Point {
                position: Position::mm(30.0, 40.0),
                fixed: false,
            },
            construction: false,
        }),
    );
    assert!(doc.apply_sketch_delta(id, &delta).unwrap());
    let version = doc.version();
    let snapshot = doc.sketch(id).unwrap();
    assert_eq!(
        snapshot.graph.point(&"point".into()).unwrap(),
        Position::mm(30.0, 40.0)
    );
    assert_eq!(
        snapshot
            .checkpoint
            .unwrap()
            .candidate
            .point(&"point".into())
            .unwrap(),
        Position::mm(30.0, 40.0)
    );
    assert_eq!(doc.version(), version);
    let loaded = unpack(701, &pack(&doc, "test").unwrap()).unwrap();
    assert_eq!(
        loaded.sketch(id).unwrap().graph,
        doc.sketch(id).unwrap().graph
    );
    let step = doc.own_steps().last().unwrap().step;
    doc.undo_step(step).unwrap();
    assert_eq!(
        doc.sketch(id)
            .unwrap()
            .graph
            .point(&"point".into())
            .unwrap(),
        Position::mm(10.0, 20.0)
    );
}

#[test]
fn sketch_clone_chain_moves_and_converts_without_losing_source_history() {
    use curvyo_document_core::{AnchorId, Angle, ObjectSnapshot, Point, Vec2};
    let doc = Document::new(711);
    let graph = curvyo_sketch_core::create_rectangle(
        &Sketch::new("origin".into()),
        "plate",
        Position::mm(0.0, 0.0),
        Position::mm(50.0, 30.0),
    )
    .unwrap();
    let id = doc
        .create_sketch(&graph, Affine::IDENTITY, &Style::default())
        .unwrap();
    let clone = doc.linked_clone(&[id], &[]).unwrap()[0];
    let second = doc.linked_clone(&[clone], &[]).unwrap()[0];
    assert!(matches!(doc.object(id), Some(ObjectSnapshot::Sketch(_))));
    let before = doc.sketch(id).unwrap().graph;
    doc.translate_objects(&[id], Vec2::new(20.0, 40.0)).unwrap();
    let rotated = doc.object(id).unwrap().rotated(
        Point::new(0.0, 0.0),
        Angle::from_radians(std::f64::consts::PI / 6.0),
    );
    doc.rotate_object(&rotated).unwrap();
    assert_eq!(doc.sketch(id).unwrap().graph, before);
    let resolved = doc.object(second).unwrap().drawn_path().unwrap().clone();
    assert_eq!(
        resolved.anchors[0].point,
        doc.sketch(id).unwrap().resolved.anchors[0].point
    );
    let anchor_ids: Vec<_> = (0..doc.sketch(id).unwrap().resolved.all_anchors().count())
        .map(|n| AnchorId::new(711, 1000 + n as u64))
        .collect();
    doc.convert_sketch_to_path(id, &anchor_ids).unwrap();
    assert!(matches!(doc.object(id), Some(ObjectSnapshot::Path(_))));
    assert_eq!(
        resolved.anchors[0].point,
        doc.object(second).unwrap().drawn_path().unwrap().anchors[0].point
    );
    doc.undo_step(doc.own_steps().last().unwrap().step).unwrap();
    assert!(matches!(doc.object(id), Some(ObjectSnapshot::Sketch(_))));
    assert_eq!(doc.sketch(id).unwrap().graph, before);
}

#[test]
fn deleting_sketch_preserves_clone_outline_and_undo_restores_parametric_source() {
    use curvyo_document_core::ObjectSnapshot;
    let doc = Document::new(712);
    let graph = curvyo_sketch_core::create_circle(
        &Sketch::new("origin".into()),
        "hole",
        Position::mm(20.0, 20.0),
        curvyo_sketch_core::Length(5.0),
    )
    .unwrap();
    let id = doc
        .create_sketch(&graph, Affine::IDENTITY, &Style::default())
        .unwrap();
    let clone = doc.linked_clone(&[id], &[]).unwrap()[0];
    let before = doc.object(clone).unwrap().drawn_path().unwrap().anchors[0].point;
    doc.delete_objects(&[id]).unwrap();
    assert!(matches!(doc.object(clone), Some(ObjectSnapshot::Path(_))));
    assert_eq!(
        before,
        doc.object(clone).unwrap().drawn_path().unwrap().anchors[0].point
    );
    doc.undo_step(doc.own_steps().last().unwrap().step).unwrap();
    assert!(matches!(doc.object(id), Some(ObjectSnapshot::Sketch(_))));
    assert!(matches!(doc.object(clone), Some(ObjectSnapshot::Clone(_))));
}

#[test]
fn contradictory_dimension_stays_inspectable_with_last_valid_geometry() {
    use curvyo_sketch_core::{Constraint, Expression, Quantity, Relation};
    let doc = Document::new(713);
    let mut graph = Sketch::new("origin".into());
    graph.entities.insert(
        "circle".into(),
        Entity::Circle {
            center: "origin".into(),
            radius: curvyo_sketch_core::Length(4.0),
            fixed_radius: true,
        },
    );
    let id = doc
        .create_sketch(&graph, Affine::IDENTITY, &Style::default())
        .unwrap();
    let before = doc.sketch(id).unwrap().resolved;
    let mut delta = SketchDelta::default();
    delta.constraints.insert(
        "radius".into(),
        Some(Constraint::driving(Relation::Radius {
            curve: "circle".into(),
            value: Expression::Literal(Quantity::length(5.0).unwrap()),
        })),
    );
    assert!(doc.apply_sketch_delta(id, &delta).unwrap());
    let failed = doc.sketch(id).unwrap();
    assert!(failed.stale);
    assert!(failed.graph.constraints.contains_key(&"radius".into()));
    assert_eq!(failed.resolved, before);
    delta.constraints.insert("radius".into(), None);
    doc.apply_sketch_delta(id, &delta).unwrap();
    assert!(!doc.sketch(id).unwrap().stale);
}

#[test]
fn committed_drag_pose_keeps_dependent_positions_out_of_intent() {
    use curvyo_sketch_core::{Constraint, Expression, Quantity, Relation};
    let doc = Document::new(714);
    let mut graph = Sketch::new("origin".into());
    for (id, x) in [("a", 0.0), ("b", 10.0)] {
        graph.entities.insert(
            id.into(),
            Entity::Point {
                position: Position::mm(x, 0.0),
                fixed: false,
            },
        );
    }
    graph.entities.insert(
        "line".into(),
        Entity::Line {
            start: "a".into(),
            end: "b".into(),
        },
    );
    graph.constraints.insert(
        "horizontal".into(),
        Constraint::driving(Relation::Horizontal("line".into())),
    );
    graph.constraints.insert(
        "length".into(),
        Constraint::driving(Relation::Length {
            curve: "line".into(),
            value: Expression::Literal(Quantity::length(10.0).unwrap()),
        }),
    );
    let id = doc
        .create_sketch(&graph, Affine::IDENTITY, &Style::default())
        .unwrap();
    let mut preview = graph.clone();
    for (point, x) in [("a", 5.0), ("b", 15.0)] {
        preview.entities.insert(
            point.into(),
            Entity::Point {
                position: Position::mm(x, 7.0),
                fixed: false,
            },
        );
    }
    let mut delta = SketchDelta::default();
    delta.entities.insert(
        "a".into(),
        Some(SketchEntityRecord {
            entity: preview.entities[&"a".into()].clone(),
            construction: false,
        }),
    );
    doc.apply_sketch_preview(id, &delta, &preview).unwrap();
    let result = doc.sketch(id).unwrap();
    assert_eq!(
        result.graph.point(&"b".into()).unwrap(),
        Position::mm(10.0, 0.0)
    );
    assert!(
        result
            .checkpoint
            .unwrap()
            .candidate
            .point(&"b".into())
            .unwrap()
            .distance(Position::mm(15.0, 7.0))
            .0
            < 1e-6
    );
}

#[test]
fn scaled_clone_chain_keeps_circle_error_below_point_zero_one_mm() {
    use curvyo_document_core::ObjectSnapshot;
    let doc = Document::new(715);
    let graph = curvyo_sketch_core::create_circle(
        &Sketch::new("origin".into()),
        "circle",
        Position::mm(0.0, 0.0),
        curvyo_sketch_core::Length(4.0),
    )
    .unwrap();
    let id = doc
        .create_sketch(&graph, Affine::IDENTITY, &Style::default())
        .unwrap();
    let first = doc.linked_clone(&[id], &[]).unwrap()[0];
    let ObjectSnapshot::Clone(clone) = doc.object(first).unwrap() else {
        panic!("clone");
    };
    doc.transform_objects(
        &[ObjectSnapshot::Clone(
            clone.mapped_by(Affine::scale(100.0, 100.0)),
        )],
        false,
    )
    .unwrap();
    let second = doc.linked_clone(&[first], &[]).unwrap()[0];
    let ObjectSnapshot::Clone(clone) = doc.object(second).unwrap() else {
        panic!("clone");
    };
    doc.transform_objects(
        &[ObjectSnapshot::Clone(
            clone.mapped_by(Affine::scale(10.0, 10.0)),
        )],
        false,
    )
    .unwrap();
    doc.set_node_visible(&[id, first], false).unwrap();
    let rows = doc.read_nodes();
    let row = rows.iter().find(|row| row.id == second).unwrap();
    assert!(row.visible);
    let object = row.object().unwrap();
    let path = object.drawn_path().unwrap();
    for subpath in path.subpaths() {
        for (a, b) in subpath
            .anchors
            .iter()
            .zip(subpath.anchors.iter().cycle().skip(1))
            .take(subpath.anchors.len())
        {
            for t in [0.25_f64, 0.5, 0.75] {
                let u = 1.0 - t;
                let x = u.powi(3) * a.point.x
                    + 3.0 * u * u * t * (a.point.x + a.handle_out.x)
                    + 3.0 * u * t * t * (b.point.x + b.handle_in.x)
                    + t.powi(3) * b.point.x;
                let y = u.powi(3) * a.point.y
                    + 3.0 * u * u * t * (a.point.y + a.handle_out.y)
                    + 3.0 * u * t * t * (b.point.y + b.handle_in.y)
                    + t.powi(3) * b.point.y;
                assert!(
                    (x.hypot(y) - 4000.0).abs() <= 0.01,
                    "radial error {}",
                    (x.hypot(y) - 4000.0).abs()
                );
            }
        }
    }
}
