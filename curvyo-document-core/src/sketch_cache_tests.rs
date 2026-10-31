//! Revision caching never hides uncommitted analytic record changes.
use crate::{Affine, Document, Style};
use curvyo_sketch_core::{Entity, Position, Sketch};

#[test]
fn pending_entity_writes_bypass_committed_snapshot_cache() {
    let document = Document::new(66);
    let mut graph = Sketch::new("origin".into());
    graph.entities.insert(
        "point".into(),
        Entity::Point {
            position: Position::mm(10.0, 20.0),
            fixed: false,
        },
    );
    let id = document
        .create_sketch(&graph, Affine::IDENTITY, &Style::default())
        .unwrap();
    assert_eq!(
        document
            .sketch(id)
            .unwrap()
            .graph
            .point(&"point".into())
            .unwrap(),
        Position::mm(10.0, 20.0)
    );
    graph.entities.insert(
        "point".into(),
        Entity::Point {
            position: Position::mm(30.0, 40.0),
            fixed: false,
        },
    );
    crate::sketch_codec::write_graph(&document.sketch_meta(id).unwrap(), &graph).unwrap();
    assert!(document.loro.get_pending_txn_len() > 0);
    assert_eq!(
        document
            .sketch(id)
            .unwrap()
            .graph
            .point(&"point".into())
            .unwrap(),
        Position::mm(30.0, 40.0)
    );
    document.commit_with_label("pending_test");
    assert_eq!(
        document
            .sketch(id)
            .unwrap()
            .graph
            .point(&"point".into())
            .unwrap(),
        Position::mm(30.0, 40.0)
    );
}
