//! The stored shape of a group (`specs/0023-groups` criteria 5, 25 and 27, `adrs.md` "the stored
//! shape of a group"): a tree node with `group: true`, children in z-order, validated on open, the
//! golden `groups_v11.curvyo`, and every older fixture opening unchanged.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]

use std::io::{Cursor, Read, Write};
use std::path::PathBuf;

use curvyo_document_core::{
    AnchorId, CURRENT_FORMAT_VERSION, CURRENT_LORO_SNAPSHOT_VERSION, Document, EllipseFrame,
    Length, MAX_GROUP_DEPTH, NewAnchor, NodeId, OpenError, Point, RectBounds, pack, unpack,
};
use loro::{LoroDoc, LoroValue, TreeID, TreeParentId};

/// The version this slice took at its branch point. A rebase onto a merge that took 11 first
/// renumbers this constant, the fixtures and their names.
const THIS_SLICE_VERSION: u32 = 11;
const GOLDEN: &str = "groups_v11.curvyo";
const TOO_DEEP: &str = "groups_too_deep_v11.curvyo";
const LEAF_CHILD: &str = "groups_leaf_child_v11.curvyo";

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn loro_of(document: &Document) -> LoroDoc {
    let loro = LoroDoc::new();
    loro.import(&document.export_loro_snapshot().unwrap())
        .unwrap();
    loro
}

fn rect(document: &Document, x: f64) -> NodeId {
    document.create_rect(RectBounds {
        origin: Point::new(x, 10.0),
        width: Length::from_mm(20.0),
        height: Length::from_mm(30.0),
    })
}

fn path(document: &Document, n: u64) -> NodeId {
    document.create_path(
        &[
            NewAnchor::corner(AnchorId::new(1, n), Point::new(0.0, 0.0)),
            NewAnchor::corner(AnchorId::new(1, n + 1), Point::new(10.0, 5.0)),
        ],
        false,
    )
}

/// Two nested groups and a mixed leaf set: `rect`, `group(path, group(ellipse, rect))`, `rect`.
fn golden_document() -> Document {
    let document = Document::new(1);
    let first = rect(&document, 0.0);
    let p = path(&document, 1);
    let e = document.create_ellipse(EllipseFrame {
        center: Point::new(60.0, 20.0),
        rx: Length::from_mm(10.0),
        ry: Length::from_mm(8.0),
    });
    let r = rect(&document, 80.0);
    let last = rect(&document, 120.0);
    let inner = document.group_objects(&[e, r]).unwrap();
    let outer = document.group_objects(&[p, inner]).unwrap();
    assert_eq!(document.children(None), vec![first, outer, last]);
    document
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

fn new_loro() -> LoroDoc {
    let loro = loro_of(&Document::new(1));
    loro.set_peer_id(7).unwrap();
    loro
}

fn mark_group(loro: &LoroDoc, node: TreeID) {
    loro.get_tree("paths")
        .get_meta(node)
        .unwrap()
        .insert("group", true)
        .unwrap();
}

/// A chain of `groups` nested groups; the deepest is empty, which is valid on read.
fn chain_file(groups: usize) -> Vec<u8> {
    let loro = new_loro();
    let tree = loro.get_tree("paths");
    let mut parent = TreeParentId::Root;
    for _ in 0..groups {
        let node = tree.create(parent).unwrap();
        mark_group(&loro, node);
        parent = TreeParentId::Node(node);
    }
    file_from_loro(&loro)
}

/// A valid rectangle (no `group` register) that holds a child.
fn leaf_with_child_file() -> Vec<u8> {
    let document = Document::new(1);
    rect(&document, 0.0);
    let loro = loro_of(&document);
    loro.set_peer_id(7).unwrap();
    let tree = loro.get_tree("paths");
    tree.create(tree.roots()[0]).unwrap();
    file_from_loro(&loro)
}

fn write_fixtures_on_request() {
    if std::env::var_os("CURVYO_WRITE_FIXTURES").is_some() {
        std::fs::write(
            fixture_path(GOLDEN),
            pack(&golden_document(), "0.1.0").unwrap(),
        )
        .unwrap();
        std::fs::write(fixture_path(TOO_DEEP), chain_file(MAX_GROUP_DEPTH + 1)).unwrap();
        std::fs::write(fixture_path(LEAF_CHILD), leaf_with_child_file()).unwrap();
    }
}

fn manifest_version(bytes: &[u8]) -> u32 {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut manifest = Vec::new();
    archive
        .by_name("manifest.json")
        .unwrap()
        .read_to_end(&mut manifest)
        .unwrap();
    let manifest: serde_json::Value = serde_json::from_slice(&manifest).unwrap();
    u32::try_from(manifest["format_version"].as_u64().unwrap()).unwrap()
}

fn stored_loro(bytes: &[u8]) -> Vec<u8> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut stored = Vec::new();
    archive
        .by_name("document.loro")
        .unwrap()
        .read_to_end(&mut stored)
        .unwrap();
    stored
}

#[test]
fn this_slice_took_its_format_version_and_the_writer_declares_it() {
    assert_eq!(CURRENT_FORMAT_VERSION, THIS_SLICE_VERSION);
    let bytes = pack(&Document::new(1), "t").unwrap();
    assert_eq!(manifest_version(&bytes), THIS_SLICE_VERSION);
}

#[test]
fn the_tree_the_order_and_every_leaf_survive_save_and_open() {
    let document = golden_document();
    let reopened = unpack(2, &pack(&document, "t").unwrap()).unwrap();
    assert_eq!(
        reopened.export_json().unwrap(),
        document.export_json().unwrap(),
        "document.json shows the same nested tree"
    );
    assert_eq!(reopened.object_ids(), document.object_ids());
    let outer = reopened.children(None)[1];
    assert!(reopened.is_group(outer));
    let inner = reopened.children(Some(outer))[1];
    assert!(reopened.is_group(inner));
    assert_eq!(reopened.children(Some(inner)).len(), 2);
    for id in document.object_ids() {
        assert_eq!(reopened.object(id), document.object(id), "{id:?}");
    }
}

#[test]
fn document_json_nests_a_group_with_its_children_and_nothing_else() {
    let json: serde_json::Value =
        serde_json::from_slice(&golden_document().export_json().unwrap()).unwrap();
    let objects = json["objects"].as_array().unwrap();
    assert_eq!(objects.len(), 3);
    let outer = &objects[1];
    assert_eq!(outer["shape"], "group");
    let keys: Vec<_> = outer.as_object().unwrap().keys().collect();
    assert_eq!(keys, ["id", "objects", "shape"], "no style, no transform");
    assert_eq!(outer["objects"][0]["shape"], "path");
    assert_eq!(outer["objects"][1]["shape"], "group");
}

#[test]
fn the_golden_reads_back_and_opening_it_writes_nothing() {
    write_fixtures_on_request();
    let bytes = std::fs::read(fixture_path(GOLDEN)).unwrap();
    assert_eq!(manifest_version(&bytes), THIS_SLICE_VERSION);
    let document = unpack(2, &bytes).unwrap();
    let outer = document.children(None)[1];
    assert!(document.is_group(outer));
    assert_eq!(document.object_ids().len(), 5);
    assert_eq!(loro_of(&document).oplog_vv(), {
        let stored = LoroDoc::new();
        stored.import(&stored_loro(&bytes)).unwrap();
        stored.oplog_vv()
    });
}

#[test]
fn a_group_nested_thirty_two_deep_opens_and_thirty_three_is_damaged() {
    assert!(unpack(2, &chain_file(MAX_GROUP_DEPTH)).is_ok());
    assert!(matches!(
        unpack(2, &chain_file(MAX_GROUP_DEPTH + 1)),
        Err(OpenError::Damaged)
    ));
}

#[test]
fn the_damaged_fixtures_are_refused_as_damaged_and_do_not_crash() {
    write_fixtures_on_request();
    for name in [TOO_DEEP, LEAF_CHILD] {
        let bytes = std::fs::read(fixture_path(name)).unwrap();
        assert_eq!(manifest_version(&bytes), THIS_SLICE_VERSION, "{name}");
        assert!(
            matches!(unpack(2, &bytes), Err(OpenError::Damaged)),
            "{name}"
        );
    }
    assert!(matches!(
        unpack(2, &leaf_with_child_file()),
        Err(OpenError::Damaged)
    ));
}

#[test]
fn a_very_deep_chain_is_refused_without_a_crash() {
    assert!(matches!(
        unpack(2, &chain_file(2_000)),
        Err(OpenError::Damaged)
    ));
}

#[test]
fn a_group_that_carries_anchors_a_shape_or_a_mistyped_register_is_damaged() {
    let carrying = |edit: &dyn Fn(&LoroDoc, TreeID)| {
        let loro = new_loro();
        let node = loro.get_tree("paths").create(TreeParentId::Root).unwrap();
        mark_group(&loro, node);
        edit(&loro, node);
        file_from_loro(&loro)
    };
    let anchors = carrying(&|loro, node| {
        let meta = loro.get_tree("paths").get_meta(node).unwrap();
        meta.insert_container("anchors", loro::LoroMovableList::new())
            .unwrap();
    });
    let shape = carrying(&|loro, node| {
        loro.get_tree("paths")
            .get_meta(node)
            .unwrap()
            .insert("shape", "rect")
            .unwrap();
    });
    let false_flag = carrying(&|loro, node| {
        loro.get_tree("paths")
            .get_meta(node)
            .unwrap()
            .insert("group", false)
            .unwrap();
    });
    let text_flag = carrying(&|loro, node| {
        loro.get_tree("paths")
            .get_meta(node)
            .unwrap()
            .insert("group", LoroValue::from("yes"))
            .unwrap();
    });
    for (name, bytes) in [
        ("anchors", anchors),
        ("shape", shape),
        ("false", false_flag),
        ("text", text_flag),
    ] {
        assert!(
            matches!(unpack(2, &bytes), Err(OpenError::Damaged)),
            "{name}"
        );
    }
}

#[test]
fn an_empty_group_is_valid_on_read_and_draws_nothing() {
    let loro = new_loro();
    let node = loro.get_tree("paths").create(TreeParentId::Root).unwrap();
    mark_group(&loro, node);
    let document = unpack(2, &file_from_loro(&loro)).unwrap();
    assert_eq!(document.children(None).len(), 1);
    assert_eq!(document.object_ids(), Vec::<NodeId>::new());
}

#[test]
fn every_older_fixture_opens_unchanged_and_writes_nothing() {
    for name in [
        "format_version_1.curvyo",
        "paths_v2.curvyo",
        "primitives_v3.curvyo",
        "rotation_v5.curvyo",
        "legacy_gradient_v7.curvyo",
        "compound_v8.curvyo",
        "dash_v9.curvyo",
        "markers_v9.curvyo",
        "display_unit_in_v7.curvyo",
        "background_v10.curvyo",
    ] {
        let bytes = std::fs::read(fixture_path(name)).unwrap();
        let document = unpack(2, &bytes).unwrap_or_else(|e| panic!("{name}: {e:?}"));
        // An older file is a tree of depth 0: every object is at the root and none is a group.
        assert!(
            document
                .children(None)
                .iter()
                .all(|&id| !document.is_group(id)),
            "{name}"
        );
        assert_eq!(document.children(None), document.object_ids(), "{name}");
        // Opening wrote nothing: the replica holds exactly the operations the file stored, and
        // its operations exported again are the very same bytes.
        let stored = stored_loro(&bytes);
        let reference = LoroDoc::new();
        reference.import(&stored).unwrap();
        assert_eq!(
            loro_of(&document).oplog_vv(),
            reference.oplog_vv(),
            "{name}"
        );
        assert_eq!(
            loro_of(&document)
                .export(loro::ExportMode::all_updates())
                .unwrap(),
            reference.export(loro::ExportMode::all_updates()).unwrap(),
            "{name}: the operations after Open are the operations of the file, byte for byte"
        );
    }
}
