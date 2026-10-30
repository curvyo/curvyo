//! The stored shape of the guide flag (`specs/0046-document-shapes` criteria 1, 2, 30 and 31,
//! ADR 0019 §2 and §4): the golden file, save and open to the last bit, the damaged file, residue
//! on a group, and older files that open unchanged.

#![allow(clippy::unwrap_used, clippy::expect_used)]

#[path = "support/pre_pages_ids.rs"]
mod pre_pages_ids;

use std::io::{Cursor, Read, Write};
use std::path::PathBuf;

use curvyo_document_core::{
    AnchorId, CURRENT_FORMAT_VERSION, CURRENT_LORO_SNAPSHOT_VERSION, Document, EllipseFrame,
    NewAnchor, NodeId, OpenError, Point, pack, unpack,
};
use loro::{LoroDoc, LoroValue, TreeID};

/// The version this slice takes. A rebase onto another bump renumbers this constant, the
/// fixtures and their names (`specs/README.md`, "`format_version` plan").
const GUIDE_FORMAT_VERSION: u32 = 15;
const GOLDEN: &str = "guide_v15.curvyo";
const DAMAGED: &str = "guide_damaged_v15.curvyo";

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

fn read_member(bytes: &[u8], name: &str) -> Vec<u8> {
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
        serde_json::from_slice(&read_member(bytes, "manifest.json")).unwrap();
    u32::try_from(manifest["format_version"].as_u64().unwrap()).unwrap()
}

fn file_from_loro(loro: &LoroDoc) -> Vec<u8> {
    loro.commit();
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
        (
            "document.loro",
            loro.export(loro::ExportMode::Snapshot).unwrap(),
        ),
        ("document.json", b"{}".to_vec()),
    ] {
        writer.start_file(name, options).unwrap();
        writer.write_all(&bytes).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

fn tree_id(id: NodeId) -> TreeID {
    let json = serde_json::to_value(id).unwrap();
    TreeID::new(
        json["peer"].as_str().unwrap().parse().unwrap(),
        i32::try_from(json["counter"].as_i64().unwrap()).unwrap(),
    )
}

fn square(n: u64, at: f64, size: f64) -> Vec<NewAnchor> {
    [(0.0, 0.0), (size, 0.0), (size, size), (0.0, size)]
        .iter()
        .enumerate()
        .map(|(k, &(dx, dy))| {
            NewAnchor::corner(
                AnchorId::new(1, n * 10 + k as u64),
                Point::new(at + dx, at + dy),
            )
        })
        .collect()
}

/// What the golden holds, by role.
struct Golden {
    document: Document,
    path: NodeId,
    circle: NodeId,
    ring: NodeId,
    clone: NodeId,
    ordinary: NodeId,
}

/// A path, a circle, a compound path with a hole and a clone of the path, all guides but the
/// clone (which reads its origin's flag), plus one ordinary path.
fn golden() -> Golden {
    let d = Document::new(1);
    let path = d.create_path(&square(1, 0.0, 10.0), true);
    let circle = d.create_ellipse(EllipseFrame::from_corners(
        Point::new(20.0, 0.0),
        Point::new(40.0, 20.0),
    ));
    let outer = d.create_path(&square(2, 50.0, 30.0), true);
    let ring = d
        .replace_with_path(
            &[outer],
            outer,
            &[(square(3, 50.0, 30.0), true), (square(4, 60.0, 10.0), true)],
            "combine",
        )
        .unwrap();
    let clone = d.linked_clone(&[path], &[]).unwrap()[0];
    let ordinary = d.create_path(&square(5, 100.0, 10.0), true);
    d.set_guide(&[path, circle, ring], true).unwrap();
    Golden {
        document: d,
        path,
        circle,
        ring,
        clone,
        ordinary,
    }
}

fn guides_of(d: &Document) -> Vec<NodeId> {
    d.read_nodes()
        .into_iter()
        .filter(|read| read.guide)
        .map(|read| read.id)
        .collect()
}

/// A path whose `guide` register holds a number, as no writer of ours stores it.
fn damaged_file() -> Vec<u8> {
    let d = Document::new(1);
    let path = d.create_path(&square(1, 0.0, 10.0), true);
    let loro = loro_of(&d);
    loro.set_peer_id(7).unwrap();
    let meta = loro.get_tree("paths").get_meta(tree_id(path)).unwrap();
    meta.insert("guide", 1_i64).unwrap();
    file_from_loro(&loro)
}

fn write_fixtures_on_request() {
    if std::env::var_os("CURVYO_WRITE_FIXTURES").is_some() {
        std::fs::write(
            fixture_path(GOLDEN),
            pack(&golden().document, "0.1.0").unwrap(),
        )
        .unwrap();
        std::fs::write(fixture_path(DAMAGED), damaged_file()).unwrap();
    }
}

#[test]
fn c2_this_slice_takes_its_format_version() {
    const _: () = assert!(CURRENT_FORMAT_VERSION >= GUIDE_FORMAT_VERSION);
    let bytes = pack(&Document::new(1), "t").unwrap();
    assert_eq!(manifest_version(&bytes), CURRENT_FORMAT_VERSION);
}

#[test]
fn c30_the_golden_reads_back_and_opening_it_writes_nothing() {
    write_fixtures_on_request();
    let bytes = std::fs::read(fixture_path(GOLDEN)).unwrap();
    assert_eq!(manifest_version(&bytes), GUIDE_FORMAT_VERSION);
    let d = unpack(2, &bytes).unwrap();
    let mut g = golden();
    g.path = pre_pages_ids::node(g.path);
    g.clone = pre_pages_ids::node(g.clone);
    g.circle = pre_pages_ids::node(g.circle);
    g.ring = pre_pages_ids::node(g.ring);
    g.ordinary = pre_pages_ids::node(g.ordinary);
    // A clone sits directly after its origin.
    assert_eq!(guides_of(&d), vec![g.path, g.clone, g.circle, g.ring]);
    assert!(!d.node_registers(g.clone).unwrap().guide, "the clone reads");
    assert!(!d.node_registers(g.ordinary).unwrap().guide);
    let stored = LoroDoc::new();
    stored
        .import(&read_member(&bytes, "document.loro"))
        .unwrap();
    assert_eq!(loro_of(&d).oplog_vv(), stored.oplog_vv(), "nothing written");
}

#[test]
fn c30_saving_twice_and_save_open_save_give_the_same_bytes() {
    let g = golden();
    let first = pack(&g.document, "t").unwrap();
    assert_eq!(pack(&g.document, "t").unwrap(), first);
    let reopened = unpack(1, &first).unwrap();
    assert_eq!(
        read_member(&pack(&reopened, "t").unwrap(), "document.json"),
        read_member(&first, "document.json")
    );
    assert_eq!(reopened.read_nodes(), g.document.read_nodes());
}

#[test]
fn c30_the_key_is_written_only_on_guides() {
    let g = golden();
    let json: serde_json::Value =
        serde_json::from_slice(&g.document.export_json().unwrap()).unwrap();
    let flagged: Vec<serde_json::Value> = json["objects"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|o| o.get("guide").is_some())
        .map(|o| o["id"].clone())
        .collect();
    let expected: Vec<serde_json::Value> = [g.path, g.circle, g.ring]
        .iter()
        .map(|id| serde_json::to_value(id).unwrap())
        .collect();
    assert_eq!(flagged, expected);
}

#[test]
fn c31_a_damaged_flag_is_refused_and_a_newer_version_first() {
    write_fixtures_on_request();
    let bytes = std::fs::read(fixture_path(DAMAGED)).unwrap();
    assert_eq!(manifest_version(&bytes), GUIDE_FORMAT_VERSION);
    assert!(matches!(unpack(2, &bytes), Err(OpenError::Damaged)));
    for value in [
        LoroValue::from("true"),
        LoroValue::Double(1.0),
        LoroValue::from(vec![true]),
    ] {
        let d = Document::new(1);
        let path = d.create_path(&square(1, 0.0, 10.0), true);
        let loro = loro_of(&d);
        loro.set_peer_id(7).unwrap();
        loro.get_tree("paths")
            .get_meta(tree_id(path))
            .unwrap()
            .insert("guide", value.clone())
            .unwrap();
        assert!(
            matches!(unpack(2, &file_from_loro(&loro)), Err(OpenError::Damaged)),
            "{value:?}"
        );
    }
}

#[test]
fn c31_a_flag_on_a_group_is_residue() {
    let d = Document::new(1);
    let a = d.create_path(&square(1, 0.0, 10.0), true);
    let b = d.create_path(&square(2, 20.0, 10.0), true);
    let group = d.group_objects(&[a, b]).unwrap();
    let loro = loro_of(&d);
    loro.set_peer_id(7).unwrap();
    loro.get_tree("paths")
        .get_meta(tree_id(group))
        .unwrap()
        .insert("guide", true)
        .unwrap();
    let opened = unpack(2, &file_from_loro(&loro)).unwrap();
    assert_eq!(guides_of(&opened), Vec::<NodeId>::new());
}

#[test]
fn c1_every_older_fixture_opens_unchanged_with_no_guide() {
    for name in [
        "format_version_1.curvyo",
        "paths_v2.curvyo",
        "compound_v8.curvyo",
        "groups_v11.curvyo",
        "layers_v12.curvyo",
        "clone_v13.curvyo",
        "clone_overrides_v14.curvyo",
    ] {
        let bytes = std::fs::read(fixture_path(name)).unwrap();
        let d = unpack(2, &bytes).unwrap_or_else(|e| panic!("{name}: {e:?}"));
        assert_eq!(guides_of(&d), Vec::<NodeId>::new(), "{name}");
        let json: serde_json::Value = serde_json::from_slice(&d.export_json().unwrap()).unwrap();
        assert!(!json.to_string().contains("\"guide\""), "{name}");
        let stored = LoroDoc::new();
        stored
            .import(&read_member(&bytes, "document.loro"))
            .unwrap();
        assert_eq!(
            loro_of(&d).oplog_vv(),
            stored.oplog_vv(),
            "{name}: not rewritten"
        );
    }
}
