//! Hostile sizes in a file (`specs/0049-linked-clone` criterion 39): 100,000 clones of one original,
//! a chain of 10,000 and a ring of 10,000 open without a hang, a crash or recursion that grows with
//! the chain. Full sizes in a release build (the release job of `ci.yml`), a tenth in debug, as
//! `compound_write_cost.rs` does. The recursion-free walk itself is pinned fast, without Loro, by
//! the unit tests of `clone_read.rs`.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::HashMap;
use std::io::{Cursor, Write};

use curvyo_document_core::{
    AnchorId, CURRENT_FORMAT_VERSION, CURRENT_LORO_SNAPSHOT_VERSION, Document, NewAnchor, NodeId,
    ObjectSnapshot, Point, unpack,
};
use loro::{LoroDoc, LoroValue, TreeID, TreeParentId};

/// Clones of one original: 100,000 in release, 10,000 in debug.
const CLONES: usize = if cfg!(debug_assertions) {
    10_000
} else {
    100_000
};
/// The length of the chain and of the ring: 10,000 in release, 1,000 in debug.
const RUN: usize = if cfg!(debug_assertions) {
    1_000
} else {
    10_000
};

fn loro_of(document: &Document) -> LoroDoc {
    let loro = LoroDoc::new();
    loro.import(&document.export_loro_snapshot().unwrap())
        .unwrap();
    loro
}

fn square(document: &Document, n: u64, at: Point) -> NodeId {
    let corner = |k: u64, dx: f64, dy: f64| {
        NewAnchor::corner(
            AnchorId::new(1, n * 10 + k),
            Point::new(at.x + dx, at.y + dy),
        )
    };
    document.create_path(
        &[
            corner(0, 0.0, 0.0),
            corner(1, 10.0, 0.0),
            corner(2, 10.0, 10.0),
            corner(3, 0.0, 10.0),
        ],
        true,
    )
}

/// Writes a clone link straight into a Loro document, as another tool or a hand edit could.
fn raw_clone(loro: &LoroDoc, origin: &str, matrix: LoroValue) -> TreeID {
    let tree = loro.get_tree("paths");
    let node = tree.create(TreeParentId::Root).unwrap();
    let meta = tree.get_meta(node).unwrap();
    meta.insert("shape", "clone").unwrap();
    meta.insert(
        "clone_link",
        LoroValue::from(HashMap::from([
            ("origin".to_owned(), LoroValue::from(origin)),
            ("matrix".to_owned(), matrix),
        ])),
    )
    .unwrap();
    node
}

fn text(id: TreeID) -> String {
    format!("{}:{}", id.peer, id.counter)
}

fn identity() -> LoroValue {
    LoroValue::from(vec![1.0, 0.0, 0.0, 1.0, 0.0, 0.0])
}

fn container_from_loro(loro_bytes: &[u8]) -> Vec<u8> {
    let manifest = serde_json::json!({
        "format_version": CURRENT_FORMAT_VERSION,
        "loro_snapshot_version": CURRENT_LORO_SNAPSHOT_VERSION,
        "app_version": "tester",
    });
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    for (name, bytes) in [
        ("manifest.json", serde_json::to_vec(&manifest).unwrap()),
        ("document.loro", loro_bytes.to_vec()),
        ("document.json", b"{}".to_vec()),
    ] {
        writer.start_file(name, options).unwrap();
        writer.write_all(&bytes).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

fn file_from_loro(loro: &LoroDoc) -> Vec<u8> {
    loro.commit();
    container_from_loro(&loro.export(loro::ExportMode::Snapshot).unwrap())
}

/// The peer of a node id, read through its `document.json` form.
fn peer_of(id: NodeId) -> u64 {
    serde_json::to_value(id).unwrap()["peer"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap()
}

/// The counter of a node id, read through its `document.json` form.
fn counter_of(id: NodeId) -> i32 {
    i32::try_from(
        serde_json::to_value(id).unwrap()["counter"]
            .as_i64()
            .unwrap(),
    )
    .unwrap()
}

/// The text form `<peer>:<counter>` the link register stores.
fn link_text(id: NodeId) -> String {
    format!("{}:{}", peer_of(id), counter_of(id))
}

#[test]
fn ac39_hostile_sizes_open_without_recursion() {
    // Many clones of one original.
    let d = Document::new(1);
    let root = square(&d, 1, Point::new(0.0, 0.0));
    let loro = loro_of(&d);
    loro.set_peer_id(7).unwrap();
    let root_text = link_text(root);
    for _ in 0..CLONES {
        raw_clone(&loro, &root_text, identity());
    }
    let document = unpack(2, &file_from_loro(&loro)).unwrap();
    let read = document.read_objects();
    assert_eq!(read.objects.len(), CLONES + 1);
    assert_eq!(read.clones_of[&root].len(), CLONES);

    // A chain, each following the one made before it, and a ring of the same length.
    let loro = loro_of(&d);
    loro.set_peer_id(8).unwrap();
    let mut previous = root_text;
    for _ in 0..RUN {
        previous = text(raw_clone(&loro, &previous, identity()));
    }
    let first = raw_clone(&loro, "0:0", identity());
    let mut last = first;
    for _ in 1..RUN {
        last = raw_clone(&loro, &text(last), identity());
    }
    loro.get_tree("paths")
        .get_meta(first)
        .unwrap()
        .insert(
            "clone_link",
            LoroValue::from(HashMap::from([
                ("origin".to_owned(), LoroValue::from(text(last))),
                ("matrix".to_owned(), identity()),
            ])),
        )
        .unwrap();
    let document = unpack(2, &file_from_loro(&loro)).unwrap();
    let read = document.read_objects();
    assert_eq!(read.orphans.len(), RUN, "the ring");
    assert_eq!(read.objects.len(), RUN + 1, "the root and the whole chain");
    assert_eq!(
        read.objects
            .iter()
            .filter_map(|o| match o {
                ObjectSnapshot::Clone(c) => Some(c.chain.depth),
                _ => None,
            })
            .max(),
        Some(RUN),
        "no depth bound when reading"
    );
}
