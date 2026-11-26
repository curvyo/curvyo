//! `specs/0141-parametric-generators` criteria 6 and 7 at the document: Convert to paths is one
//! undo step that gives the generated object back, a saved and reopened generated object keeps
//! its kind, parameters and outline exactly, and an unknown kind opens as stored and can still be
//! converted.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use curvyo_document_core::{
    Affine, AnchorId, Document, GeneratedRecord, NewAnchor, NodeId, Point, pack, unpack,
};
use serde_json::json;

fn square(x: f64, side: f64) -> Vec<f64> {
    let corner = |px: f64, py: f64| [px, py, px, py, px, py];
    let mut numbers = vec![x, 0.0];
    for (px, py) in [(x + side, 0.0), (x + side, side), (x, side), (x, 0.0)] {
        numbers.extend(corner(px, py));
    }
    numbers
}

fn record(kind: &str) -> GeneratedRecord {
    let mut params = serde_json::Map::new();
    params.insert("module".into(), json!({"mm": 2.5}));
    params.insert("teeth".into(), json!({"count": 17}));
    params.insert("web".into(), json!({"choice": "spokes"}));
    params.insert("keyway".into(), json!({"on": true}));
    GeneratedRecord {
        kind: kind.into(),
        seed: 77,
        params,
        parts: vec![vec![square(0.0, 10.0)], vec![square(20.0, 6.0)]],
        part_keys: Vec::new(),
    }
}

fn plates(first: u64) -> Vec<Vec<(Vec<NewAnchor>, bool)>> {
    let corners = |base: u64, side: f64| -> Vec<NewAnchor> {
        [(0.0, 0.0), (side, 0.0), (side, side), (0.0, side)]
            .iter()
            .zip(0_u64..)
            .map(|((x, y), k)| NewAnchor::corner(AnchorId::new(9, base + k), Point::new(*x, *y)))
            .collect()
    };
    vec![
        vec![(corners(first, 10.0), true)],
        vec![(corners(first + 10, 6.0), true)],
    ]
}

fn only(d: &Document) -> NodeId {
    d.read_objects().objects.first().unwrap().id()
}

#[test]
fn convert_to_paths_is_one_undo_step_that_gives_the_generated_object_back() {
    let d = Document::new(1);
    let frame = Affine::new(0.0, 1.0, -1.0, 0.0, 30.0, 40.0);
    let id = d
        .create_generated(&record("curvyo.generators/gear"), frame)
        .unwrap();
    let stored = d.generated(id).unwrap();
    let before = d.own_step_count();
    d.convert_generated(id, &plates(100)).unwrap();
    assert_eq!(d.own_step_count(), before + 1, "exactly one step");
    assert!(d.generated(id).is_none(), "its parameters are gone");
    assert_eq!(d.read_objects().objects.len(), 2, "two plates as paths");
    let step = d.own_steps().last().unwrap().step;
    d.undo_step(step).unwrap();
    assert_eq!(
        d.generated(id).unwrap(),
        stored,
        "undo restores kind, params, outline and frame"
    );
}

#[test]
fn a_saved_and_reopened_generated_object_keeps_kind_parameters_and_outline() {
    let d = Document::new(1);
    let frame = Affine::new(1.0, 0.0, 0.0, -1.0, 12.5, 7.25);
    let id = d
        .create_generated(&record("curvyo.generators/box"), frame)
        .unwrap();
    let stored = d.generated(id).unwrap();
    let reopened = unpack(2, &pack(&d, "0.1.0").unwrap()).unwrap();
    let again = reopened.generated(only(&reopened)).unwrap();
    assert_eq!(
        again.record, stored.record,
        "kind, seed, parameters and outline"
    );
    assert_eq!(again.record.kind, "curvyo.generators/box");
    assert_eq!(again.record.params["teeth"], json!({"count": 17}));
    assert_eq!(again.frame, frame);
    assert_eq!(reopened.own_step_count(), 0, "opening writes nothing");
    // A second save and open changes nothing either.
    let twice = unpack(3, &pack(&reopened, "0.1.0").unwrap()).unwrap();
    assert_eq!(twice.generated(only(&twice)).unwrap(), again);
}

#[test]
fn an_unknown_kind_opens_as_stored_is_not_converted_on_open_and_converts_on_request() {
    let d = Document::new(1);
    let id = d
        .create_generated(&record("future.generators/spiral"), Affine::IDENTITY)
        .unwrap();
    let stored = d.generated(id).unwrap();
    let reopened = unpack(2, &pack(&d, "0.1.0").unwrap()).unwrap();
    let id = only(&reopened);
    assert_eq!(
        reopened.generated(id).unwrap(),
        stored,
        "as stored, not rebuilt"
    );
    assert_eq!(reopened.own_step_count(), 0, "not converted on open");
    // It still draws as an outline of its stored numbers.
    assert!(reopened.object(id).is_some());
    reopened.convert_generated(id, &plates(500)).unwrap();
    assert!(reopened.generated(id).is_none());
}
