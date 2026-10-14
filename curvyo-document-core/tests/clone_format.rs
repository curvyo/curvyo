//! The stored shape of a linked clone (`specs/0049-linked-clone` criteria 35 read side, 37 to 40,
//! `adrs.md` decisions 1 and 11): save and open to the last bit, the golden file, damaged files,
//! orphans and rings, hostile sizes, and the `document.json` entry.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]

use std::collections::HashMap;
use std::io::{Cursor, Read, Write};
use std::path::PathBuf;

use curvyo_document_core::{
    Affine, AnchorId, CURRENT_FORMAT_VERSION, CURRENT_LORO_SNAPSHOT_VERSION, Document, Length,
    NewAnchor, NodeId, ObjectSnapshot, OpenError, Point, RectBounds, Vec2, outline_of_rotated,
    pack, unpack,
};
use loro::{LoroDoc, LoroValue, TreeID, TreeParentId};

/// The version this slice takes. A rebase onto `0050` (which takes 12) renumbers this constant,
/// the fixtures and their names (`specs/0049-linked-clone/plan.md`, "Format version").
const CLONE_FORMAT_VERSION: u32 = 13;
const GOLDEN: &str = "clone_v13.curvyo";
const DAMAGED: &str = "clone_matrix_five_v13.curvyo";
const ORPHANS: &str = "clone_orphan_and_ring_v13.curvyo";

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

fn ring_outline(n: u64, size: f64, at: Point) -> Vec<NewAnchor> {
    let pts = [(0.0, 0.0), (size, 0.0), (size, size), (0.0, size)];
    pts.iter()
        .enumerate()
        .map(|(k, (x, y))| {
            NewAnchor::corner(
                AnchorId::new(2, n * 10 + k as u64),
                Point::new(at.x + x, at.y + y),
            )
        })
        .collect()
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

fn new_loro() -> LoroDoc {
    let loro = loro_of(&Document::new(1));
    loro.set_peer_id(7).unwrap();
    loro
}

/// A root, two clones of it, a chain of three, a compound-path root with a clone, and a rectangle
/// converted by making its clone. Every matrix differs from the identity except the first.
fn golden_document() -> Document {
    let d = Document::new(1);
    let root = square(&d, 1, Point::new(0.0, 0.0));
    let twins = d.linked_clone(&[root, root], &[]).unwrap();
    assert_eq!(twins.len(), 1);
    // Never two adjacent steps with one label: an older file shows those as one row.
    let nudge = |id: NodeId| d.translate_objects(&[id], Vec2::new(1.0, 0.0)).unwrap();
    nudge(root);
    let second = d.linked_clone(&[root], &[]).unwrap()[0];
    let chain_root = square(&d, 2, Point::new(40.0, 0.0));
    let c1 = d.linked_clone(&[chain_root], &[]).unwrap()[0];
    nudge(chain_root);
    let c2 = d.linked_clone(&[c1], &[]).unwrap()[0];
    nudge(chain_root);
    let c3 = d.linked_clone(&[c2], &[]).unwrap()[0];
    let outer = square(&d, 3, Point::new(80.0, 0.0));
    let compound = d
        .replace_with_path(
            &[outer],
            outer,
            &[
                (ring_outline(1, 20.0, Point::new(80.0, 0.0)), true),
                (
                    ring_outline(2, 10.0, Point::new(85.0, 5.0))
                        .into_iter()
                        .rev()
                        .collect(),
                    true,
                ),
            ],
            "combine",
        )
        .unwrap();
    let compound_clone = d.linked_clone(&[compound], &[]).unwrap()[0];
    let rect = d.create_rect(RectBounds {
        origin: Point::new(120.0, 0.0),
        width: Length::from_mm(15.0),
        height: Length::from_mm(10.0),
    });
    let ObjectSnapshot::Primitive(primitive) = d.object(rect).unwrap() else {
        panic!("a rectangle")
    };
    let anchors = outline_of_rotated(&primitive.shape, primitive.rotation)
        .into_iter()
        .enumerate()
        .map(|(k, a)| NewAnchor {
            id: AnchorId::new(3, k as u64),
            point: a.point,
            handle_in: a.handle_in,
            handle_out: a.handle_out,
            kind: a.kind,
        })
        .collect();
    let rect_clone = d.linked_clone(&[rect], &[(rect, anchors)]).unwrap()[0];
    // Matrices that are not the identity, written as a file another build would hold. The
    // command that writes them is milestone 2's; the stored register is the same.
    let loro = loro_of(&d);
    let tree = loro.get_tree("paths");
    for (clone, origin, m) in [
        (second, root, [0.0, 1.0, -1.0, 0.0, 30.0, 0.1]),
        (c1, chain_root, [1.0, 0.0, 0.0, 1.0, 0.0, 30.0]),
        (c2, c1, [2.0, 0.0, 0.0, 2.0, 0.0, 0.0]),
        (c3, c2, [1.0, 0.0, 0.5, 1.0, 1.0 / 3.0, -7.25]),
        (compound_clone, compound, [-1.0, 0.0, 0.0, 1.0, 200.0, 0.0]),
        (rect_clone, rect, [1.0, 0.0, 0.0, 1.0, 0.0, 40.0]),
    ] {
        let meta = tree
            .get_meta(TreeID::new(peer_of(clone), counter_of(clone)))
            .unwrap();
        meta.insert(
            "clone_link",
            LoroValue::from(HashMap::from([
                (
                    "origin".to_owned(),
                    LoroValue::from(format!("{}:{}", peer_of(origin), counter_of(origin))),
                ),
                ("matrix".to_owned(), LoroValue::from(m.to_vec())),
            ])),
        )
        .unwrap();
    }
    unpack(1, &file_from_loro(&loro)).unwrap()
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

fn damaged_file() -> Vec<u8> {
    let loro = new_loro();
    let root = loro.get_tree("paths").create(TreeParentId::Root).unwrap();
    let meta = loro.get_tree("paths").get_meta(root).unwrap();
    meta.insert("closed", true).unwrap();
    meta.insert_container("anchors", loro::LoroMovableList::new())
        .unwrap();
    raw_clone(
        &loro,
        &text(root),
        LoroValue::from(vec![1.0, 0.0, 0.0, 1.0, 0.0]),
    );
    file_from_loro(&loro)
}

/// A good clone, an orphan naming nothing, a clone of that orphan, and a ring of three.
fn orphans_file() -> (Vec<u8>, usize) {
    let d = Document::new(1);
    let root = square(&d, 1, Point::new(0.0, 0.0));
    d.linked_clone(&[root], &[]).unwrap();
    let loro = loro_of(&d);
    loro.set_peer_id(7).unwrap();
    let orphan = raw_clone(&loro, "999:999", identity());
    raw_clone(&loro, &text(orphan), identity());
    let a = raw_clone(&loro, "0:0", identity());
    let b = raw_clone(&loro, &text(a), identity());
    let c = raw_clone(&loro, &text(b), identity());
    let meta = loro.get_tree("paths").get_meta(a).unwrap();
    meta.insert(
        "clone_link",
        LoroValue::from(HashMap::from([
            ("origin".to_owned(), LoroValue::from(text(c))),
            ("matrix".to_owned(), identity()),
        ])),
    )
    .unwrap();
    (file_from_loro(&loro), 5)
}

fn write_fixtures_on_request() {
    if std::env::var_os("CURVYO_WRITE_FIXTURES").is_some() {
        std::fs::write(
            fixture_path(GOLDEN),
            pack(&golden_document(), "0.1.0").unwrap(),
        )
        .unwrap();
        std::fs::write(fixture_path(DAMAGED), damaged_file()).unwrap();
        std::fs::write(fixture_path(ORPHANS), orphans_file().0).unwrap();
    }
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

fn clones(document: &Document) -> Vec<curvyo_document_core::CloneSnapshot> {
    document
        .read_objects()
        .objects
        .iter()
        .filter_map(|object| match object {
            ObjectSnapshot::Clone(clone) => Some(clone.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn ac37_this_slice_takes_its_format_version() {
    const _: () = assert!(CURRENT_FORMAT_VERSION >= CLONE_FORMAT_VERSION);
    let bytes = pack(&Document::new(1), "t").unwrap();
    assert_eq!(manifest_version(&bytes), CURRENT_FORMAT_VERSION);
}

#[test]
fn ac37_save_and_open_keep_every_link_to_the_last_bit() {
    let document = golden_document();
    let reopened = unpack(2, &pack(&document, "t").unwrap()).unwrap();
    assert_eq!(reopened.object_ids(), document.object_ids());
    assert_eq!(reopened.read_objects(), document.read_objects());
    let stored = clones(&reopened);
    assert_eq!(stored.len(), 7);
    let c3 = stored.iter().find(|c| c.chain.depth == 3).unwrap();
    assert_eq!(
        c3.chain.matrix.to_array().map(f64::to_bits),
        [1.0, 0.0, 0.5, 1.0, 1.0 / 3.0, -7.25].map(f64::to_bits)
    );
    assert_eq!(
        reopened.export_json().unwrap(),
        document.export_json().unwrap()
    );
}

/// What `clone_v13.curvyo` stores, as literals: clone counter, origin counter (peer 1), the six
/// matrix entries in SVG order.
const GOLDEN_LINKS: [(i32, i32, [f64; 6]); 7] = [
    (32, 3, [1.0, 0.0, 0.0, 1.0, 0.0, 0.0]),
    (39, 3, [0.0, 1.0, -1.0, 0.0, 30.0, 0.1]),
    (72, 43, [1.0, 0.0, 0.0, 1.0, 0.0, 30.0]),
    (79, 72, [2.0, 0.0, 0.0, 2.0, 0.0, 0.0]),
    (86, 79, [1.0, 0.0, 0.5, 1.0, 1.0 / 3.0, -7.25]),
    (176, 118, [-1.0, 0.0, 0.0, 1.0, 200.0, 0.0]),
    (220, 179, [1.0, 0.0, 0.0, 1.0, 0.0, 40.0]),
];

fn close(a: Point, x: f64, y: f64) -> bool {
    (a.x - x).abs() < 1e-9 && (a.y - y).abs() < 1e-9
}

#[test]
fn ac37_the_golden_reads_back_and_opening_it_writes_nothing() {
    write_fixtures_on_request();
    let bytes = std::fs::read(fixture_path(GOLDEN)).unwrap();
    assert_eq!(manifest_version(&bytes), CLONE_FORMAT_VERSION);
    let document = unpack(2, &bytes).unwrap();
    let read = document.read_objects();
    assert_eq!(read.orphans, Vec::<NodeId>::new());
    let stored = clones(&document);
    let mut links: Vec<(i32, i32, [u64; 6])> = stored
        .iter()
        .map(|c| {
            assert_eq!(peer_of(c.id()), 1);
            assert_eq!(peer_of(c.chain.origin), 1);
            (
                counter_of(c.id()),
                counter_of(c.chain.origin),
                c.chain.matrix.to_array().map(f64::to_bits),
            )
        })
        .collect();
    links.sort_unstable();
    let expected: Vec<(i32, i32, [u64; 6])> = GOLDEN_LINKS
        .iter()
        .map(|&(id, origin, m)| (id, origin, m.map(f64::to_bits)))
        .collect();
    assert_eq!(
        links, expected,
        "every origin and every matrix bit as stored"
    );
    let by_counter = |n: i32| stored.iter().find(|c| counter_of(c.id()) == n).unwrap();
    // `second`: a quarter turn and (30, 0.1) on the root square nudged to x = 1.
    assert!(close(by_counter(39).resolved.anchors[0].point, 30.0, 1.1));
    // `c3`, depth 3: M3 · M2 · M1 composed.
    let c3 = by_counter(86);
    assert_eq!(c3.chain.depth, 3);
    let composed = c3.chain.composed.to_array();
    for (got, want) in composed
        .iter()
        .zip([2.0, 0.0, 1.0, 2.0, 30.0 + 1.0 / 3.0, 52.75])
    {
        assert!((got - want).abs() < 1e-12, "{composed:?}");
    }
    assert!(close(
        c3.resolved.anchors[0].point,
        114.0 + 1.0 / 3.0,
        52.75
    ));
    // The compound clone, mirrored about x = 100: its hole starts at (115, 15).
    let compound = by_counter(176);
    assert_eq!(compound.resolved.extra_subpaths.len(), 1);
    assert!(close(
        compound.resolved.extra_subpaths[0].anchors[0].point,
        115.0,
        15.0
    ));
    assert!(close(compound.resolved.anchors[0].point, 120.0, 0.0));
    let stored_loro = LoroDoc::new();
    stored_loro
        .import(&read_member(&bytes, "document.loro"))
        .unwrap();
    assert_eq!(loro_of(&document).oplog_vv(), stored_loro.oplog_vv());
}

#[test]
fn one_object_read_equals_its_entry_in_the_whole_read() {
    write_fixtures_on_request();
    for name in [GOLDEN, ORPHANS] {
        let document = unpack(2, &std::fs::read(fixture_path(name)).unwrap()).unwrap();
        let read = document.read_objects();
        for id in document.object_ids() {
            let entry = read.objects.iter().find(|o| o.id() == id).cloned();
            assert_eq!(document.object(id), entry, "{name}: {id:?}");
        }
    }
}

/// Residue (ADR 0015 §4): a `clone_link` on a path is ignored.
#[test]
fn ac38_a_link_on_a_path_is_ignored_residue() {
    let d = Document::new(1);
    let target = square(&d, 1, Point::new(0.0, 0.0));
    let p = square(&d, 2, Point::new(20.0, 0.0));
    let loro = loro_of(&d);
    loro.set_peer_id(7).unwrap();
    loro.get_tree("paths")
        .get_meta(TreeID::new(peer_of(p), counter_of(p)))
        .unwrap()
        .insert(
            "clone_link",
            LoroValue::from(HashMap::from([
                ("origin".to_owned(), LoroValue::from(link_text(target))),
                ("matrix".to_owned(), identity()),
            ])),
        )
        .unwrap();
    let document = unpack(2, &file_from_loro(&loro)).unwrap();
    assert!(matches!(document.object(p), Some(ObjectSnapshot::Path(_))));
    assert_eq!(clones(&document), Vec::new());
    let json: serde_json::Value = serde_json::from_slice(&document.export_json().unwrap()).unwrap();
    assert_eq!(json["objects"][1]["shape"], "path");
    assert!(json["objects"][1].get("origin").is_none());
}

/// Residue (ADR 0015 §4): anchors, style, opacity and rotation keys on a clone are ignored.
#[test]
fn ac38_outline_and_style_keys_on_a_clone_are_ignored_residue() {
    let d = Document::new(1);
    let root = square(&d, 1, Point::new(0.0, 0.0));
    let clone = d.linked_clone(&[root], &[]).unwrap()[0];
    let loro = loro_of(&d);
    loro.set_peer_id(7).unwrap();
    let meta = loro
        .get_tree("paths")
        .get_meta(TreeID::new(peer_of(clone), counter_of(clone)))
        .unwrap();
    let anchors = meta
        .insert_container("anchors", loro::LoroMovableList::new())
        .unwrap();
    let anchor = anchors.push_container(loro::LoroMap::new()).unwrap();
    anchor.insert("point", vec![500.0, 500.0]).unwrap();
    meta.insert("closed", false).unwrap();
    meta.insert("stroke_width", 9.0).unwrap();
    meta.insert("opacity", 0.25).unwrap();
    meta.insert("rotation", 1.0).unwrap();
    let document = unpack(2, &file_from_loro(&loro)).unwrap();
    let read = clones(&document);
    assert_eq!(read.len(), 1);
    let root_path = match document.object(root).unwrap() {
        ObjectSnapshot::Path(path) => path,
        other => panic!("{other:?}"),
    };
    assert_eq!(
        read[0].resolved.anchors, root_path.anchors,
        "the root's outline"
    );
    assert_eq!(read[0].resolved.style, root_path.style, "the root's style");
    let json: serde_json::Value = serde_json::from_slice(&document.export_json().unwrap()).unwrap();
    // `document.json` mirrors the storage: the node registers of `0050` (here the residue
    // `opacity`) are listed, but no outline or style.
    let keys: Vec<_> = json["objects"][1]
        .as_object()
        .unwrap()
        .keys()
        .filter(|key| !["name", "visible", "locked", "opacity"].contains(&key.as_str()))
        .collect();
    assert_eq!(keys, ["id", "matrix", "origin", "shape"]);
    assert_eq!(
        document.read_nodes()[1].opacity,
        curvyo_document_core::Opacity::OPAQUE,
        "the residue opacity of a clone never counts: it shows its root's"
    );
}

#[test]
fn ac38_a_damaged_link_is_refused_and_does_not_crash() {
    write_fixtures_on_request();
    let bytes = std::fs::read(fixture_path(DAMAGED)).unwrap();
    assert_eq!(manifest_version(&bytes), CLONE_FORMAT_VERSION);
    assert!(matches!(unpack(2, &bytes), Err(OpenError::Damaged)));
    for matrix in [
        LoroValue::from(vec![1.0, 0.0, 0.0, 1.0, 0.0, f64::NAN]),
        LoroValue::from(vec![1.0, 0.0, 0.0, 1.0, 0.0, 1.5e7]),
        LoroValue::from("1,0,0,1,0,0"),
    ] {
        let loro = new_loro();
        raw_clone(&loro, "7:0", matrix);
        assert!(matches!(
            unpack(2, &file_from_loro(&loro)),
            Err(OpenError::Damaged)
        ));
    }
    let loro = new_loro();
    raw_clone(&loro, "not an id", identity());
    assert!(matches!(
        unpack(2, &file_from_loro(&loro)),
        Err(OpenError::Damaged)
    ));
    let loro = new_loro();
    let clone = raw_clone(&loro, "7:0", identity());
    loro.get_tree("paths").create(clone).unwrap();
    assert!(
        matches!(unpack(2, &file_from_loro(&loro)), Err(OpenError::Damaged)),
        "a clone with a child"
    );
}

#[test]
fn ac35_38_orphans_and_rings_open_and_are_listed_not_drawn() {
    write_fixtures_on_request();
    let bytes = std::fs::read(fixture_path(ORPHANS)).unwrap();
    let document = unpack(2, &bytes).unwrap();
    let read = document.read_objects();
    assert_eq!(read.orphans.len(), orphans_file().1);
    assert_eq!(read.objects.len(), 2, "the root and its good clone");
    for orphan in &read.orphans {
        assert_eq!(document.object(*orphan), None, "an orphan is not an object");
    }
}

/// Criterion 38 as amended 2026-10-14: a clone with no link at all and no outline is an orphan,
/// not damage, and the repair deletes it.
#[test]
fn ac38_a_clone_without_link_or_outline_is_an_orphan_the_repair_deletes() {
    let loro = new_loro();
    let node = loro.get_tree("paths").create(TreeParentId::Root).unwrap();
    loro.get_tree("paths")
        .get_meta(node)
        .unwrap()
        .insert("shape", "clone")
        .unwrap();
    let document = unpack(2, &file_from_loro(&loro)).expect("not damaged");
    assert_eq!(document.read_objects().orphans.len(), 1);
    assert_eq!(document.remove_orphan_clones(), 1);
    assert_eq!(document.take_clone_repair().unlinked, 0);
    assert_eq!(document.object_ids(), Vec::<NodeId>::new());
}

/// Criterion 38 as amended 2026-10-14 (`adrs.md` decisions 1 and 6): a clone with no link but a
/// valid stored outline, the residue of a merge, is an orphan until the repair, which turns it
/// into that path with its style. A clone behind it then follows a path and resolves.
#[test]
fn ac38_a_clone_without_link_but_with_an_outline_becomes_that_path() {
    let d = Document::new(1);
    let p = square(&d, 1, Point::new(0.0, 0.0));
    let before = match d.object(p) {
        Some(ObjectSnapshot::Path(path)) => path,
        other => panic!("{other:?}"),
    };
    let loro = loro_of(&d);
    loro.set_peer_id(7).unwrap();
    loro.get_tree("paths")
        .get_meta(TreeID::new(peer_of(p), counter_of(p)))
        .unwrap()
        .insert("shape", "clone")
        .unwrap();
    raw_clone(
        &loro,
        &format!("{}:{}", peer_of(p), counter_of(p)),
        LoroValue::from(vec![1.0, 0.0, 0.0, 1.0, 30.0, 0.0]),
    );
    let document = unpack(2, &file_from_loro(&loro)).expect("not damaged");
    let read = document.read_objects();
    assert_eq!(read.orphans.len(), 2, "the node and the clone behind it");
    assert_eq!(read.objects.len(), 0, "an orphan is not drawn");
    assert_eq!(document.remove_orphan_clones(), 0, "nothing is deleted");
    assert_eq!(document.take_clone_repair().unlinked, 1);
    assert_eq!(document.object(p), Some(ObjectSnapshot::Path(before)));
    let read = document.read_objects();
    assert_eq!(read.orphans, Vec::<NodeId>::new());
    assert_eq!(clones(&document).len(), 1, "the clone behind it resolves");
    assert_eq!(clones(&document)[0].chain.root, p);
}

/// The same residue with an outline that does not read as a path is deleted like any orphan.
#[test]
fn ac38_a_clone_without_link_and_with_a_broken_outline_is_deleted() {
    let loro = new_loro();
    let node = loro.get_tree("paths").create(TreeParentId::Root).unwrap();
    let meta = loro.get_tree("paths").get_meta(node).unwrap();
    meta.insert("shape", "clone").unwrap();
    meta.insert("anchors", "not a list").unwrap();
    let document = unpack(2, &file_from_loro(&loro)).expect("not damaged");
    assert_eq!(document.remove_orphan_clones(), 1);
    assert_eq!(document.object_ids(), Vec::<NodeId>::new());
}

#[test]
fn ac38_a_clone_of_a_group_or_a_primitive_is_an_orphan_not_damage() {
    let d = Document::new(1);
    let a = square(&d, 1, Point::new(0.0, 0.0));
    let b = square(&d, 2, Point::new(0.0, 0.0));
    let group = d.group_objects(&[a, b]).unwrap();
    let rect = d.create_rect(RectBounds {
        origin: Point::new(0.0, 0.0),
        width: Length::from_mm(1.0),
        height: Length::from_mm(1.0),
    });
    let loro = loro_of(&d);
    loro.set_peer_id(7).unwrap();
    for target in [group, rect] {
        raw_clone(
            &loro,
            &format!("{}:{}", peer_of(target), counter_of(target)),
            identity(),
        );
    }
    let document = unpack(2, &file_from_loro(&loro)).unwrap();
    assert_eq!(document.read_objects().orphans.len(), 2);
}

#[test]
fn ac40_document_json_lists_a_clone_with_its_origin_and_matrix_only() {
    let d = Document::new(1);
    let root = square(&d, 1, Point::new(0.0, 0.0));
    d.linked_clone(&[root], &[]).unwrap();
    let json: serde_json::Value = serde_json::from_slice(&d.export_json().unwrap()).unwrap();
    let clone = &json["objects"][1];
    assert_eq!(clone["shape"], "clone");
    let keys: Vec<_> = clone.as_object().unwrap().keys().collect();
    assert_eq!(keys, ["id", "matrix", "origin", "shape"], "no outline");
    assert_eq!(clone["origin"], json["objects"][0]["id"]);
    assert_eq!(
        clone["matrix"],
        serde_json::json!([1.0, 0.0, 0.0, 1.0, 0.0, 0.0])
    );
    assert_eq!(
        Affine::IDENTITY.to_array().to_vec(),
        vec![1.0, 0.0, 0.0, 1.0, 0.0, 0.0]
    );
}

#[test]
fn ac37_every_older_fixture_opens_with_no_clone() {
    for name in [
        "format_version_1.curvyo",
        "paths_v2.curvyo",
        "primitives_v3.curvyo",
        "compound_v8.curvyo",
        "markers_v9.curvyo",
        "background_v10.curvyo",
        "groups_v11.curvyo",
    ] {
        let bytes = std::fs::read(fixture_path(name)).unwrap();
        let document = unpack(2, &bytes).unwrap_or_else(|e| panic!("{name}: {e:?}"));
        let read = document.read_objects();
        assert_eq!(clones(&document), Vec::new(), "{name}");
        assert_eq!(read.orphans, Vec::<NodeId>::new(), "{name}");
        assert_eq!(read.objects.len(), document.object_ids().len(), "{name}");
    }
}
