//! The stored shape of layers and node registers (`specs/0050-layers-and-objects-panel` criteria
//! 17, 46, 47 and 54, `adrs.md` decisions 1, 6 and 15, and the note of 2026-10-12 on decision
//! 11): format version 12, the golden `layers_v12.curvyo`, the damaged files, `document.json`, and
//! the two writers that exist carrying hidden and locked nodes.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]

#[path = "support/pre_pages_ids.rs"]
mod pre_pages_ids;

use std::io::{Cursor, Read, Write};
use std::path::PathBuf;

use curvyo_document_core::{
    CURRENT_FORMAT_VERSION, CURRENT_LORO_SNAPSHOT_VERSION, Document, MovePlan, MoveTarget, NodeId,
    NodeKind, Opacity, OpenError, Point, RectBounds, pack, unpack,
};
use loro::{LoroDoc, LoroValue, TreeID, TreeParentId};

/// The version this slice took at its branch point. A rebase onto a merge that took 12 first
/// renumbers this constant, the fixtures and their names.
const THIS_SLICE_VERSION: u32 = 12;
const GOLDEN: &str = "layers_v12.curvyo";
const DAMAGED_OPACITY: &str = "layers_opacity_damaged_v12.curvyo";
const MARKER_IN_GROUP: &str = "layers_marker_in_group_v12.curvyo";
const MARKUP_NAME: &str = "<img src=x onerror=alert(1)>";

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn rect(document: &Document, x: f64) -> NodeId {
    document.create_rect(RectBounds::from_corners(
        Point::new(x, 0.0),
        Point::new(x + 10.0, 10.0),
    ))
}

fn into(document: &Document, nodes: &[NodeId], parent: NodeId) {
    document
        .move_nodes(&MovePlan {
            nodes: nodes.to_vec(),
            target: MoveTarget::FrontOf(Some(parent)),
        })
        .unwrap();
}

/// A layer "Cut" at 50 % holding a group and a hidden rectangle, a locked layer "Engrave" holding
/// a rectangle named with markup, and a loose rectangle on top.
fn golden_document() -> Document {
    // No two adjacent steps share a label: an old file shows those as one row (`0020` c36).
    let document = Document::new(1);
    let cut = document.new_layer(None);
    document.rename_node(cut, "Cut").unwrap();
    let a = rect(&document, 0.0);
    document
        .set_node_opacity(&[a], Opacity::new(0.6).unwrap())
        .unwrap();
    let b = rect(&document, 20.0);
    into(&document, &[a, b], cut);
    let group = document.group_objects(&[a, b]).unwrap();
    document.rename_node(group, "Tabs").unwrap();
    let hidden = rect(&document, 40.0);
    document.set_node_visible(&[hidden], false).unwrap();
    into(&document, &[hidden], cut);
    document
        .set_node_opacity(&[cut], Opacity::new(0.5).unwrap())
        .unwrap();
    let engrave = document.new_layer(Some(cut));
    document.rename_node(engrave, "Engrave").unwrap();
    let marked = rect(&document, 60.0);
    into(&document, &[marked], engrave);
    document.rename_node(marked, MARKUP_NAME).unwrap();
    document.set_node_locked(&[engrave], true).unwrap();
    rect(&document, 80.0);
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

fn loro_of(document: &Document) -> LoroDoc {
    let loro = LoroDoc::new();
    loro.import(&document.export_loro_snapshot().unwrap())
        .unwrap();
    loro.set_peer_id(7).unwrap();
    loro
}

fn file_from_loro(loro: &LoroDoc) -> Vec<u8> {
    loro.commit();
    container_from_loro(&loro.export(loro::ExportMode::Snapshot).unwrap())
}

/// One rectangle whose node carries `key: value`.
fn file_with_register(key: &str, value: LoroValue) -> Vec<u8> {
    let document = Document::new(1);
    rect(&document, 0.0);
    let loro = loro_of(&document);
    let tree = loro.get_tree("paths");
    tree.get_meta(tree.roots()[0])
        .unwrap()
        .insert(key, value)
        .unwrap();
    file_from_loro(&loro)
}

/// A group inside a group whose inner one carries the layer marker.
fn marker_in_group_file() -> Vec<u8> {
    let loro = loro_of(&Document::new(1));
    let tree = loro.get_tree("paths");
    let outer = tree.create(TreeParentId::Root).unwrap();
    let inner = tree.create(TreeParentId::Node(outer)).unwrap();
    for (node, layer) in [(outer, false), (inner, true)] {
        let meta = tree.get_meta(node).unwrap();
        meta.insert("group", true).unwrap();
        if layer {
            meta.insert("layer", true).unwrap();
            meta.insert("name", "Stray").unwrap();
        }
    }
    file_from_loro(&loro)
}

fn layers_file(count: usize) -> Vec<u8> {
    let loro = loro_of(&Document::new(1));
    let tree = loro.get_tree("paths");
    for i in 0..count {
        let node: TreeID = tree.create(TreeParentId::Root).unwrap();
        let meta = tree.get_meta(node).unwrap();
        meta.insert("group", true).unwrap();
        meta.insert("layer", true).unwrap();
        meta.insert("name", format!("Layer {}", i + 1)).unwrap();
    }
    file_from_loro(&loro)
}

fn write_fixtures_on_request() {
    if std::env::var_os("CURVYO_WRITE_FIXTURES").is_some() {
        std::fs::write(
            fixture_path(GOLDEN),
            pack(&golden_document(), "0.1.0").unwrap(),
        )
        .unwrap();
        std::fs::write(
            fixture_path(DAMAGED_OPACITY),
            file_with_register("opacity", LoroValue::Double(1.5)),
        )
        .unwrap();
        std::fs::write(fixture_path(MARKER_IN_GROUP), marker_in_group_file()).unwrap();
    }
}

fn member(bytes: &[u8], name: &str) -> Vec<u8> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut out = Vec::new();
    archive
        .by_name(name)
        .unwrap()
        .read_to_end(&mut out)
        .unwrap();
    out
}

fn manifest_version(bytes: &[u8]) -> u32 {
    let manifest: serde_json::Value =
        serde_json::from_slice(&member(bytes, "manifest.json")).unwrap();
    u32::try_from(manifest["format_version"].as_u64().unwrap()).unwrap()
}

fn json(document: &Document) -> serde_json::Value {
    serde_json::from_slice(&document.export_json().unwrap()).unwrap()
}

#[test]
fn this_slice_took_its_format_version_and_the_writer_declares_it() {
    const _: () = assert!(CURRENT_FORMAT_VERSION >= THIS_SLICE_VERSION);
    let bytes = pack(&Document::new(1), "t").unwrap();
    assert_eq!(manifest_version(&bytes), CURRENT_FORMAT_VERSION);
}

#[test]
fn every_tree_order_name_flag_and_opacity_survives_save_and_open() {
    let document = golden_document();
    let reopened = unpack(2, &pack(&document, "t").unwrap()).unwrap();
    assert_eq!(json(&reopened), json(&document));
    assert_eq!(reopened.read_nodes(), document.read_nodes());
}

#[test]
fn document_json_writes_layers_and_registers_away_from_their_defaults() {
    let view = json(&golden_document());
    let objects = view["objects"].as_array().unwrap();
    assert_eq!(objects.len(), 3);
    let cut = &objects[0];
    assert_eq!(cut["shape"], "layer");
    assert_eq!(cut["name"], "Cut");
    assert_eq!(cut["opacity"], 0.5);
    assert!(cut.get("visible").is_none() && cut.get("locked").is_none());
    let group = &cut["objects"][0];
    assert_eq!(group["shape"], "group");
    assert_eq!(group["name"], "Tabs");
    let keys: Vec<_> = group.as_object().unwrap().keys().collect();
    assert_eq!(keys, ["id", "name", "objects", "shape"]);
    assert_eq!(group["objects"][0]["opacity"], 0.6);
    assert_eq!(cut["objects"][1]["visible"], false);
    let engrave = &objects[1];
    assert_eq!(engrave["shape"], "layer");
    assert_eq!(engrave["locked"], true);
    assert_eq!(engrave["objects"][0]["name"], MARKUP_NAME);
    let loose = &objects[2];
    assert_eq!(loose["shape"], "rect");
    for key in ["name", "visible", "locked", "opacity"] {
        assert!(loose.get(key).is_none(), "{key} at its default is left out");
    }
}

#[test]
fn the_golden_reads_back_and_opening_it_writes_nothing() {
    write_fixtures_on_request();
    let bytes = std::fs::read(fixture_path(GOLDEN)).unwrap();
    assert_eq!(manifest_version(&bytes), THIS_SLICE_VERSION);
    let document = unpack(2, &bytes).unwrap();
    let reads = document.read_nodes();
    let kinds: Vec<&str> = reads
        .iter()
        .map(|read| match read.kind {
            NodeKind::Layer => "layer",
            NodeKind::Group => "group",
            NodeKind::Leaf(_) => "leaf",
        })
        .collect();
    assert_eq!(
        kinds,
        [
            "layer", "group", "leaf", "leaf", "leaf", "layer", "leaf", "leaf"
        ]
    );
    assert_eq!(reads[0].registers.name.as_deref(), Some("Cut"));
    assert!((reads[2].opacity.get() - 0.3).abs() < 1e-12);
    assert!(!reads[4].visible);
    assert!(reads[6].locked);
    assert_eq!(reads[6].registers.name.as_deref(), Some(MARKUP_NAME));
    assert_eq!(
        json(&document),
        pre_pages_ids::value(&json(&golden_document()))
    );
    let stored = LoroDoc::new();
    stored.import(&member(&bytes, "document.loro")).unwrap();
    assert_eq!(loro_of(&document).oplog_vv(), stored.oplog_vv());
}

#[test]
fn a_file_from_the_previous_version_opens_with_no_layers_and_default_registers() {
    let bytes = std::fs::read(fixture_path("groups_v11.curvyo")).unwrap();
    assert_eq!(manifest_version(&bytes), THIS_SLICE_VERSION - 1);
    let document = unpack(2, &bytes).unwrap();
    for read in document.read_nodes() {
        assert_ne!(read.kind, NodeKind::Layer);
        assert!(read.visible && !read.locked);
        assert_eq!(read.opacity.get(), 1.0);
        assert_eq!(read.registers.name, None);
    }
    let stored = LoroDoc::new();
    stored.import(&member(&bytes, "document.loro")).unwrap();
    assert_eq!(
        loro_of(&document).oplog_vv(),
        stored.oplog_vv(),
        "not rewritten"
    );
}

#[test]
fn a_file_of_a_newer_version_is_refused() {
    let document = golden_document();
    let bytes = pack(&document, "t").unwrap();
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&member(&bytes, "manifest.json")).unwrap();
    manifest["format_version"] = (CURRENT_FORMAT_VERSION + 1).into();
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default();
    for (name, data) in [
        ("manifest.json", serde_json::to_vec(&manifest).unwrap()),
        ("document.loro", member(&bytes, "document.loro")),
        ("document.json", member(&bytes, "document.json")),
    ] {
        writer.start_file(name, options).unwrap();
        writer.write_all(&data).unwrap();
    }
    let newer = writer.finish().unwrap().into_inner();
    assert!(matches!(
        unpack(2, &newer),
        Err(OpenError::FormatTooNew { .. })
    ));
}

#[test]
fn mistyped_registers_are_damaged_and_do_not_crash() {
    let cases = [
        ("opacity", LoroValue::Double(1.5)),
        ("opacity", LoroValue::Double(-0.1)),
        ("opacity", LoroValue::Double(f64::INFINITY)),
        ("opacity", LoroValue::from("half")),
        ("visible", LoroValue::I64(0)),
        ("locked", LoroValue::from("no")),
        ("name", LoroValue::Double(3.0)),
        ("layer", LoroValue::Bool(true)),
    ];
    for (key, value) in cases {
        let bytes = file_with_register(key, value.clone());
        assert!(
            matches!(unpack(2, &bytes), Err(OpenError::Damaged)),
            "{key} = {value:?}"
        );
    }
    write_fixtures_on_request();
    let bytes = std::fs::read(fixture_path(DAMAGED_OPACITY)).unwrap();
    assert_eq!(manifest_version(&bytes), THIS_SLICE_VERSION);
    assert!(matches!(unpack(2, &bytes), Err(OpenError::Damaged)));
}

#[test]
fn a_layer_marker_below_the_top_level_reads_as_a_plain_group() {
    write_fixtures_on_request();
    let bytes = std::fs::read(fixture_path(MARKER_IN_GROUP)).unwrap();
    let document = unpack(2, &bytes).unwrap();
    let reads = document.read_nodes();
    assert_eq!(reads.len(), 2);
    assert_eq!(reads[1].kind, NodeKind::Group);
    assert!(!document.is_layer(reads[1].id));
    assert!(document.is_group(reads[1].id));
    assert_eq!(
        json(&document)["objects"][0]["objects"][0]["shape"],
        "group"
    );
}

#[test]
fn a_name_in_a_file_longer_than_128_or_with_bidi_controls_is_cleaned_for_display_only() {
    let raw = format!("\u{202E}{}", "a".repeat(200));
    let bytes = file_with_register("name", LoroValue::from(raw.as_str()));
    let document = unpack(2, &bytes).unwrap();
    let read = &document.read_nodes()[0];
    assert_eq!(read.registers.name, Some("a".repeat(128)));
    let loro = loro_of(&document);
    let tree = loro.get_tree("paths");
    let stored = tree.get_meta(tree.roots()[0]).unwrap().get("name");
    assert!(
        matches!(stored, Some(loro::ValueOrContainer::Value(LoroValue::String(s))) if *s == raw),
        "the file is left as it is"
    );
}

#[test]
fn ten_thousand_layers_open() {
    let bytes = layers_file(10_000);
    let document = unpack(2, &bytes).unwrap();
    assert_eq!(document.read_nodes().len(), 10_000);
    assert!(document.is_layer(document.children(None)[9_999]));
}

/// Criterion 54: the only two writers of the document carry hidden and locked nodes with their
/// registers. A new writer that leaves them out has to be added on purpose.
#[test]
fn export_loro_snapshot_and_export_json_carry_hidden_and_locked_nodes() {
    let document = Document::new(1);
    let layer = document.new_layer(None);
    let inside = rect(&document, 0.0);
    into(&document, &[inside], layer);
    document.set_node_visible(&[layer], false).unwrap();
    let locked = rect(&document, 20.0);
    document.set_node_locked(&[locked], true).unwrap();

    let snapshot = document.export_loro_snapshot().unwrap();
    let reopened = unpack(2, &pack(&document, "t").unwrap()).unwrap();
    assert_ne!(snapshot, Vec::<u8>::new());
    let reads = reopened.read_nodes();
    assert_eq!(reads.len(), 3);
    assert!(!reads[0].registers.visible && !reads[1].visible);
    assert!(reads[2].registers.locked);

    let view = json(&document);
    assert_eq!(view["objects"][0]["visible"], false);
    assert_eq!(view["objects"][0]["objects"][0]["shape"], "rect");
    assert_eq!(view["objects"][1]["locked"], true);
}
