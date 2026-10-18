//! Compound paths with open, zero-length and crossing outlines in the file
//! (`specs/0054-combine-open-paths` criteria 28 to 30): the golden file
//! `compound_open.curvyo`, the damaged `closed` register and an outline of a
//! single node from a foreign file.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::cast_precision_loss)]

use std::io::{Cursor, Read, Write};
use std::path::PathBuf;

use curvyo_document_core::{
    AnchorId, CURRENT_FORMAT_VERSION, CURRENT_LORO_SNAPSHOT_VERSION, Document, NewAnchor, NodeId,
    OpenError, PathSnapshot, Point, pack, unpack,
};
use loro::{LoroDoc, LoroMap, LoroMovableList};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

/// The committed golden. Regenerate with
/// `CURVYO_WRITE_FIXTURES=1 cargo test -p curvyo-document-core --test compound_open_0054`.
const GOLDEN: &str = "compound_open.curvyo";

fn id(n: u64) -> AnchorId {
    AnchorId::new(7, n)
}

fn outline(first: u64, points: &[(f64, f64)], closed: bool) -> (Vec<NewAnchor>, bool) {
    let anchors = points
        .iter()
        .enumerate()
        .map(|(k, &(x, y))| NewAnchor::corner(id(first + k as u64), Point::new(x, y)))
        .collect();
    (anchors, closed)
}

/// Criterion 29: a closed square, an open line, a zero-length open outline and
/// a closed square crossing the first, in that order.
fn golden_outlines() -> Vec<(Vec<NewAnchor>, bool)> {
    vec![
        outline(
            100,
            &[(0.0, 0.0), (20.0, 0.0), (20.0, 20.0), (0.0, 20.0)],
            true,
        ),
        outline(200, &[(-10.0, 10.0), (40.0, 10.0)], false),
        outline(300, &[(5.0, 5.0), (5.0, 5.0)], false),
        outline(
            400,
            &[(10.0, 10.0), (30.0, 10.0), (30.0, 30.0), (10.0, 30.0)],
            true,
        ),
    ]
}

fn golden_document() -> (Document, NodeId) {
    let document = Document::new(1);
    let base = document.create_path(
        &[
            NewAnchor::corner(id(1), Point::new(0.0, 0.0)),
            NewAnchor::corner(id(2), Point::new(5.0, 0.0)),
        ],
        false,
    );
    let node = document
        .replace_with_path(&[base], base, &golden_outlines(), "combine_paths")
        .unwrap();
    (document, node)
}

/// Every outline of `path` as (anchors, closed), in order.
fn outlines_of(path: &PathSnapshot) -> Vec<(Vec<NewAnchor>, bool)> {
    path.subpaths()
        .map(|subpath| (subpath.anchors.to_vec(), subpath.closed))
        .collect()
}

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn member(container: &[u8], name: &str) -> Vec<u8> {
    let mut archive = ZipArchive::new(Cursor::new(container)).expect("zip");
    let mut bytes = Vec::new();
    archive
        .by_name(name)
        .expect("member")
        .read_to_end(&mut bytes)
        .expect("read");
    bytes
}

/// Criteria 28 and 29: the golden reopens with every outline's state, nodes,
/// direction and order, is not rewritten on open, and saving the document
/// gives the same outlines again.
#[test]
fn c28_c29_the_golden_keeps_open_zero_length_and_crossing_outlines() {
    if std::env::var_os("CURVYO_WRITE_FIXTURES").is_some() {
        let (document, _) = golden_document();
        std::fs::write(fixture_path(GOLDEN), pack(&document, "0.1.0").unwrap()).unwrap();
    }
    let golden = std::fs::read(fixture_path(GOLDEN)).unwrap();
    let document = unpack(2, &golden).unwrap();
    let ids = document.object_ids();
    assert_eq!(ids.len(), 1);
    let path = document.path(ids[0]).unwrap();
    assert_eq!(outlines_of(&path), golden_outlines());

    let stored = LoroDoc::new();
    stored.import(&member(&golden, "document.loro")).unwrap();
    let reread = LoroDoc::new();
    reread
        .import(&document.export_loro_snapshot().unwrap())
        .unwrap();
    assert_eq!(
        reread.get_deep_value(),
        stored.get_deep_value(),
        "opening must not rewrite the document"
    );

    let again = unpack(2, &pack(&document, "0.1.0").unwrap()).unwrap();
    assert_eq!(outlines_of(&again.path(ids[0]).unwrap()), golden_outlines());
}

/// The format version this slice writes; 0054 does not bump it (criterion 29). The golden stays
/// as this build wrote it when a later slice bumps the version.
const GOLDEN_FORMAT_VERSION: u32 = 14;

/// Criterion 29: no `format_version` bump; the golden declares the version of its build.
#[test]
fn c29_the_golden_declares_the_version_of_its_build() {
    let golden = std::fs::read(fixture_path(GOLDEN)).unwrap();
    let manifest: serde_json::Value =
        serde_json::from_slice(&member(&golden, "manifest.json")).unwrap();
    assert_eq!(manifest["format_version"], GOLDEN_FORMAT_VERSION);
}

/// Criterion 30: the `document.json` view lists the open or closed state of
/// every outline.
#[test]
fn c30_the_json_view_lists_the_state_of_every_outline() {
    let (document, _) = golden_document();
    let bytes = pack(&document, "0.1.0").unwrap();
    let json: serde_json::Value = serde_json::from_slice(&member(&bytes, "document.json")).unwrap();
    let object = &json["objects"][0];
    assert_eq!(object["closed"], true);
    let extras: Vec<Option<bool>> = object["extra_subpaths"]
        .as_array()
        .unwrap()
        .iter()
        .map(|extra| extra["closed"].as_bool())
        .collect();
    assert_eq!(extras, vec![Some(false), Some(false), Some(true)]);
}

fn container(loro_bytes: &[u8]) -> Vec<u8> {
    let manifest = serde_json::json!({
        "format_version": CURRENT_FORMAT_VERSION,
        "loro_snapshot_version": CURRENT_LORO_SNAPSHOT_VERSION,
        "app_version": "0.1.0",
    });
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
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

/// Opens the golden document with `edit` applied to its path's meta map.
fn open_golden_with(edit: impl FnOnce(&LoroMap)) -> Result<Document, OpenError> {
    let (document, _) = golden_document();
    let loro = LoroDoc::new();
    loro.set_peer_id(1).unwrap();
    loro.import(&document.export_loro_snapshot().unwrap())
        .unwrap();
    let tree = loro.get_tree("paths");
    edit(&tree.get_meta(tree.roots()[0]).unwrap());
    loro.commit();
    unpack(
        2,
        &container(&loro.export(loro::ExportMode::Snapshot).unwrap()),
    )
}

/// The map of extra outline `index` of `meta`.
fn extra(meta: &LoroMap, index: usize) -> LoroMap {
    match meta.get("extra_subpaths") {
        Some(loro::ValueOrContainer::Container(loro::Container::MovableList(list))) => {
            match list.get(index) {
                Some(loro::ValueOrContainer::Container(loro::Container::Map(map))) => map,
                _ => panic!("no extra outline {index}"),
            }
        }
        _ => panic!("the golden has extra_subpaths"),
    }
}

/// Criterion 30: an extra outline without an open or closed state, or with a
/// state that is not a boolean, is damage, refused without a crash.
#[test]
fn c30_a_missing_or_wrong_closed_state_is_damage() {
    type Edit = Box<dyn FnOnce(&LoroMap)>;
    assert!(
        open_golden_with(|_| {}).is_ok(),
        "the unedited golden opens"
    );
    let damaged: Vec<(&str, Edit)> = vec![
        (
            "no closed",
            Box::new(|meta| extra(meta, 0).delete("closed").unwrap()),
        ),
        (
            "closed is a string",
            Box::new(|meta| extra(meta, 1).insert("closed", "yes").unwrap()),
        ),
        (
            "closed is a number",
            Box::new(|meta| extra(meta, 2).insert("closed", 1.0).unwrap()),
        ),
    ];
    for (what, edit) in damaged {
        assert!(
            matches!(open_golden_with(edit), Err(OpenError::Damaged)),
            "{what}"
        );
    }
}

/// Criterion 30: an outline of a single node (only a foreign or hand-edited
/// file holds one) opens and reads as that one node.
#[test]
fn c30_an_outline_of_a_single_node_opens() {
    let document = open_golden_with(|meta| {
        let anchors: LoroMovableList = match extra(meta, 0).get("anchors") {
            Some(loro::ValueOrContainer::Container(loro::Container::MovableList(list))) => list,
            _ => panic!("anchors"),
        };
        anchors.delete(1, 1).unwrap();
    })
    .unwrap();
    let path = document.path(document.object_ids()[0]).unwrap();
    let outlines = outlines_of(&path);
    assert_eq!(outlines.len(), 4);
    assert_eq!(outlines[1].0.len(), 1);
    assert!(!outlines[1].1);
}
