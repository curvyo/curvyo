//! Dimension label placement is independent from solved geometric intent.
#![allow(clippy::unwrap_used)]
use curvyo_document_core::{Document, SketchDelta, Style, pack, unpack};
use curvyo_sketch_core::{
    Constraint, Construction, Expression, Position, Quantity, Relation, Sketch, construct,
};
#[test]
fn label_position_roundtrip_noop_and_constraint_removal_preserve_geometry() {
    let doc = Document::new(66221);
    let graph = construct(
        &Sketch::new("o".into()),
        "line",
        &Construction::Polyline(vec![Position::mm(10.0, 10.0), Position::mm(20.0, 10.0)]),
    )
    .unwrap()
    .intent;
    let id = doc
        .create_page_sketch(&doc.pages()[0].id, &graph, &Style::default())
        .unwrap();
    let constraint = Constraint::driving(Relation::Length {
        curve: "line:line0".into(),
        value: Expression::Literal(Quantity::length(10.0).unwrap()),
    });
    doc.apply_sketch_delta(
        id,
        &SketchDelta {
            constraints: [("length".into(), Some(constraint))].into(),
            ..Default::default()
        },
    )
    .unwrap();
    let before = doc.sketch(id).unwrap();
    let position = Position::mm(15.0, -8.0);
    assert!(
        doc.set_sketch_label(id, &"length".into(), position)
            .unwrap()
    );
    let changed = doc.sketch(id).unwrap();
    assert_eq!(changed.labels[&"length".into()], position);
    assert_eq!(changed.graph, before.graph);
    assert_eq!(changed.checkpoint, before.checkpoint);
    let version = doc.version();
    assert!(
        !doc.set_sketch_label(id, &"length".into(), position)
            .unwrap()
    );
    assert_eq!(doc.version(), version);
    let loaded = unpack(66222, &pack(&doc, "labels").unwrap()).unwrap();
    assert_eq!(loaded.sketch(id).unwrap().labels, changed.labels);
    assert!(
        doc.set_sketch_label(id, &"missing".into(), position)
            .is_err()
    );
    assert_eq!(doc.version(), version);
    doc.apply_sketch_delta(
        id,
        &SketchDelta {
            constraints: [("length".into(), None)].into(),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(doc.sketch(id).unwrap().labels.is_empty());
    doc.undo_step(doc.own_steps().last().unwrap().step).unwrap();
    assert_eq!(doc.sketch(id).unwrap().labels[&"length".into()], position);
}
