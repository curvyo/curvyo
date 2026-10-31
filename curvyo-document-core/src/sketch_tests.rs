//! Sketch storage guards distinguish malformed records from repairable graph merges.
use super::*;
use crate::sketch_codec::{self, KEYS};
use curvyo_sketch_core::{Entity, Position, Sketch};

fn fixture() -> (Document, NodeId) {
    let doc = Document::new(801);
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
    (doc, id)
}

#[test]
fn changed_intent_overrides_old_checkpoint_without_read_writes() {
    let (doc, id) = fixture();
    let meta = doc.sketch_meta(id).unwrap();
    let entities = sketch_codec::collection(&meta, KEYS[1]).unwrap();
    let record = SketchEntityRecord {
        entity: Entity::Point {
            position: Position::mm(42.0, 35.0),
            fixed: false,
        },
        construction: false,
    };
    sketch_codec::write(&entities, "point", &record).unwrap();
    doc.commit_with_label("peer_geometry");
    let before = doc.version();
    let snapshot = doc.sketch(id).unwrap();
    assert!(!snapshot.stale);
    let seed = crate::sketch_read::seed_for(&snapshot.graph, snapshot.checkpoint.as_ref());
    assert_eq!(
        seed.point(&"point".into()).unwrap(),
        Position::mm(42.0, 35.0)
    );
    assert_eq!(before, doc.version());
    let delta = SketchDelta {
        entities: std::collections::BTreeMap::from([(
            "other".into(),
            Some(SketchEntityRecord {
                entity: Entity::Point {
                    position: Position::mm(1.0, 2.0),
                    fixed: false,
                },
                construction: false,
            }),
        )]),
        ..SketchDelta::default()
    };
    doc.apply_sketch_delta(id, &delta).unwrap();
    assert_eq!(
        doc.sketch(id)
            .unwrap()
            .checkpoint
            .unwrap()
            .candidate
            .point(&"point".into())
            .unwrap(),
        Position::mm(42.0, 35.0)
    );
}

#[test]
fn dangling_reference_opens_but_malformed_record_does_not() {
    let (doc, id) = fixture();
    let meta = doc.sketch_meta(id).unwrap();
    let entities = sketch_codec::collection(&meta, KEYS[1]).unwrap();
    let record = SketchEntityRecord {
        entity: Entity::Line {
            start: "missing".into(),
            end: "point".into(),
        },
        construction: false,
    };
    sketch_codec::write(&entities, "line", &record).unwrap();
    doc.commit_with_label("peer_reference");
    let loaded = unpack(802, &pack(&doc, "test").unwrap()).unwrap();
    assert!(loaded.sketch(id).unwrap().stale);
    entities.insert("line", "not a record").unwrap();
    doc.commit_with_label("damaged");
    assert!(unpack(803, &pack(&doc, "test").unwrap()).is_err());
}

#[test]
fn checkpoint_hash_does_not_authorize_invalid_fixed_geometry() {
    let (doc, id) = fixture();
    let meta = doc.sketch_meta(id).unwrap();
    let mut saved: SketchCheckpoint = sketch_codec::atomic(&meta, KEYS[5]).unwrap();
    saved.candidate.entities.insert(
        "origin".into(),
        Entity::Point {
            position: Position::mm(99.0, 0.0),
            fixed: true,
        },
    );
    sketch_codec::write(&meta, KEYS[5], &saved).unwrap();
    doc.commit_with_label("damaged_checkpoint");
    assert!(unpack(803, &pack(&doc, "test").unwrap()).is_err());
}

#[test]
fn cached_rank_is_recomputed_before_display() {
    let (doc, id) = fixture();
    let meta = doc.sketch_meta(id).unwrap();
    let mut saved: SketchCheckpoint = sketch_codec::atomic(&meta, KEYS[5]).unwrap();
    saved.status = curvyo_sketch_core::SolveStatus::FullyConstrained;
    saved.fully_constrained = Some(saved.intent.entities.keys().cloned().collect());
    sketch_codec::write(&meta, KEYS[5], &saved).unwrap();
    doc.commit_with_label("wrong_cached_rank");
    assert_eq!(
        doc.sketch(id).unwrap().status,
        Some(curvyo_sketch_core::SolveStatus::UnderConstrained(2))
    );
}

#[test]
fn peer_entity_edits_survive_local_undo_as_atomic_records() {
    let (a, id) = fixture();
    let b = Document::from_loro_snapshot(802, &a.export_loro_snapshot().unwrap()).unwrap();
    let mut local = SketchDelta::default();
    local.entities.insert(
        "point".into(),
        Some(SketchEntityRecord {
            entity: Entity::Point {
                position: Position::mm(30.0, 40.0),
                fixed: false,
            },
            construction: true,
        }),
    );
    a.apply_sketch_delta(id, &local).unwrap();
    let step = a.own_steps().last().unwrap().step;
    let mut remote = SketchDelta::default();
    remote.entities.insert(
        "peer".into(),
        Some(SketchEntityRecord {
            entity: Entity::Point {
                position: Position::mm(90.0, 80.0),
                fixed: false,
            },
            construction: true,
        }),
    );
    b.apply_sketch_delta(id, &remote).unwrap();
    a.loro().import(&b.export_loro_snapshot().unwrap()).unwrap();
    let before = a.version();
    assert_eq!(
        a.sketch(id).unwrap().graph.point(&"point".into()).unwrap(),
        Position::mm(30.0, 40.0)
    );
    assert_eq!(a.version(), before);
    a.undo_step(step).unwrap();
    let graph = a.sketch(id).unwrap().graph;
    assert_eq!(
        graph.point(&"point".into()).unwrap(),
        Position::mm(10.0, 20.0)
    );
    assert_eq!(
        graph.point(&"peer".into()).unwrap(),
        Position::mm(90.0, 80.0)
    );
    assert!(graph.construction.contains(&"peer".into()));
    assert!(!graph.construction.contains(&"point".into()));
}

#[test]
fn ordinary_path_transform_refuses_sketch_without_writes() {
    let (doc, id) = fixture();
    let before = doc.version();
    let path = doc.sketch(id).unwrap().resolved;
    assert!(
        doc.transform_objects(&[ObjectSnapshot::Path(path)], false)
            .is_err()
    );
    assert_eq!(before, doc.version());
}

#[test]
fn node_lists_and_clone_queries_reuse_original_diagnosis_after_rigid_move() {
    let (doc, id) = fixture();
    let clone = doc.linked_clone(&[id], &[]).unwrap()[0];
    let _ = doc.read_nodes();
    let before = crate::sketch_read::solve_count();
    assert!(doc.object(clone).is_some());
    let _ = doc.read_nodes();
    assert_eq!(crate::sketch_read::solve_count(), before);
    doc.translate_objects(&[id], Vec2::new(10.0, 10.0)).unwrap();
    let after_write = crate::sketch_read::solve_count();
    let _ = doc.read_nodes();
    assert!(doc.object(clone).is_some());
    assert_eq!(crate::sketch_read::solve_count(), after_write);
}
