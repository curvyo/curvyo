//! The Parameters override of sketch clones (`specs/0099-sketch-pages` criteria 24 and 28): the
//! rows, a typed value with its clamp notice, Original's, Own and Back.
#![allow(clippy::unwrap_used, missing_docs)]

use curvyo_document_core::{Affine, Document, ExposedKey, NodeId, ObjectSnapshot, Style};
use curvyo_sketch_core as sk;
use curvyo_ui_core::instance_sketch::{change, own_all, plan, reset, view};

fn graph() -> sk::Sketch {
    let mut s = sk::Sketch::new("origin".into());
    s.entities.insert(
        "end".into(),
        sk::Entity::Point {
            position: sk::Position::mm(60.0, 0.0),
            fixed: false,
        },
    );
    s.entities.insert(
        "line".into(),
        sk::Entity::Line {
            start: "origin".into(),
            end: "end".into(),
        },
    );
    for (id, value) in [("width", 60.0), ("height", 20.0)] {
        s.parameters.insert(
            id.into(),
            sk::Parameter::literal(id, sk::Quantity::length(value).unwrap()),
        );
    }
    s.constraints.insert(
        "horizontal".into(),
        sk::Constraint::driving(sk::Relation::Horizontal("line".into())),
    );
    s.constraints.insert(
        "length".into(),
        sk::Constraint::driving(sk::Relation::Length {
            curve: "line".into(),
            value: sk::Expression::Reference("width".into()),
        }),
    );
    s
}

fn setup() -> (Document, NodeId, NodeId, NodeId) {
    let d = Document::new(1);
    let sketch = d
        .create_sketch(&graph(), Affine::IDENTITY, &Style::default())
        .unwrap();
    d.expose_parameter(sketch, &ExposedKey::parameter(&"width".into()), None)
        .unwrap();
    d.expose_parameter(sketch, &ExposedKey::parameter(&"height".into()), None)
        .unwrap();
    d.set_exposed_limits(
        sketch,
        &ExposedKey::parameter(&"width".into()),
        Some(20.0),
        Some(100.0),
        None,
    )
    .unwrap();
    let a = d.linked_clone(&[sketch], &[]).unwrap()[0];
    let b = d.linked_clone(&[a], &[]).unwrap()[0];
    (d, sketch, a, b)
}

fn span(d: &Document, id: NodeId) -> f64 {
    let read = d.read_objects();
    let ObjectSnapshot::Clone(clone) = read.get(id).unwrap() else {
        panic!("clone")
    };
    let xs: Vec<f64> = clone.resolved.anchors.iter().map(|a| a.point.x).collect();
    xs.iter().copied().fold(f64::MIN, f64::max) - xs.iter().copied().fold(f64::MAX, f64::min)
}

fn none() -> [(NodeId, curvyo_document_core::ValueSet); 0] {
    []
}

#[test]
fn rows_follow_the_exposed_list_with_the_value_a_clone_shows_and_where_it_comes_from() {
    let (d, _, a, b) = setup();
    let shown = view(&d, &[a], &[]).unwrap();
    assert_eq!(shown.state, "original");
    assert_eq!(shown.rows.len(), 2);
    assert_eq!(shown.rows[0].label, "width");
    assert_eq!(shown.rows[0].origin, "sketch");
    assert_eq!(shown.rows[0].value, Some(60.0));
    assert_eq!(
        (
            shown.rows[0].track_min,
            shown.rows[0].track_max,
            shown.rows[0].track_log
        ),
        (20.0, 100.0, false)
    );
    assert!(
        shown.rows[1].track_log,
        "no limits: a log track from 0 to 4 x the value"
    );
    change(&d, &[a], "p:width", 80.0, &none()).unwrap();
    let chained = view(&d, &[b], &[]).unwrap();
    assert_eq!(chained.state, "original", "B holds nothing itself");
    assert_eq!(chained.rows[0].origin, "inherited");
    assert_eq!(chained.rows[0].value, Some(80.0));
    assert!(chained.rows[0].chained);
    let both = view(&d, &[a, b], &[]).unwrap();
    assert_eq!(both.state, "mixed");
    assert_eq!(both.rows[0].origin, "mixed");
    assert!(!both.rows[0].mixed, "the same value 80");
}

#[test]
fn a_typed_value_reaches_the_limit_and_says_so() {
    let (d, _, a, _) = setup();
    let changed = change(&d, &[a], "p:width", 150.0, &none()).unwrap();
    assert!(changed.written);
    assert_eq!(changed.notices, ["width set to 100 mm, the maximum."]);
    assert!((span(&d, a) - 100.0).abs() < 1e-6);
    let low = change(&d, &[a], "p:width", 5.0, &none()).unwrap();
    assert_eq!(low.notices, ["width set to 20 mm, the minimum."]);
    let same = change(&d, &[a], "p:width", 20.0, &none()).unwrap();
    assert!(!same.written, "an equal value writes nothing");
}

#[test]
fn a_plan_shows_the_clone_without_writing() {
    let (d, _, a, _) = setup();
    let steps = d.history_len();
    let planned = plan(&d, a, "p:width", 90.0).unwrap();
    let preview = planned.preview.unwrap();
    let xs: Vec<f64> = preview.resolved.anchors.iter().map(|p| p.point.x).collect();
    assert!(((xs[1] - xs[0]).abs() - 90.0).abs() < 1e-6);
    assert_eq!(d.history_len(), steps, "nothing is written");
    assert!((span(&d, a) - 60.0).abs() < 1e-6);
}

#[test]
fn own_writes_the_shown_values_so_nothing_moves_and_originals_drops_them() {
    let (d, _, a, _) = setup();
    assert!(own_all(&d, &[a]).unwrap());
    assert!((span(&d, a) - 60.0).abs() < 1e-6);
    let own = view(&d, &[a], &[]).unwrap();
    assert_eq!(own.state, "own");
    assert!(own.rows.iter().all(|row| row.origin == "own"));
    change(&d, &[a], "p:width", 80.0, &none()).unwrap();
    assert!(reset(&d, &[a], None).unwrap());
    assert!((span(&d, a) - 60.0).abs() < 1e-6);
    assert_eq!(view(&d, &[a], &[]).unwrap().state, "original");
    assert!(
        !reset(&d, &[a], None).unwrap(),
        "nothing to drop writes nothing"
    );
}

#[test]
fn back_drops_only_this_clones_value_in_a_chain() {
    let (d, _, a, b) = setup();
    change(&d, &[a], "p:width", 80.0, &none()).unwrap();
    change(&d, &[b], "p:width", 40.0, &none()).unwrap();
    assert!((span(&d, b) - 40.0).abs() < 1e-6);
    reset(&d, &[b], Some("p:width")).unwrap();
    assert!(
        (span(&d, b) - 80.0).abs() < 1e-6,
        "back to the original's value"
    );
    assert!((span(&d, a) - 80.0).abs() < 1e-6);
}

#[test]
fn clones_of_two_sketches_show_the_mixed_pill_and_no_fields() {
    let (d, _, a, _) = setup();
    let other = d
        .create_sketch(&graph(), Affine::IDENTITY, &Style::default())
        .unwrap();
    let c = d.linked_clone(&[other], &[]).unwrap()[0];
    let shown = view(&d, &[a, c], &[]).unwrap();
    assert_eq!(shown.state, "mixed");
    assert!(shown.rows.is_empty());
}
