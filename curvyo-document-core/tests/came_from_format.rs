//! The stored Came from record and the page list of a saved container (`specs/0137` criteria 2,
//! 6 and 9, ADR 0084 §1, §7, §8): the golden files of format 58, the damaged record that opens
//! and reads as absent, the older-build refusal, a file of 57 that opens unchanged, and the
//! `library.json` member with and without.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]

use std::io::{Cursor, Read, Write};
use std::path::PathBuf;

use curvyo_document_core::{
    AnchorId, CURRENT_FORMAT_VERSION, CameFrom, Document, FragmentTarget, LIBRARY_MEMBER,
    LibrarySummary, NewAnchor, NodeId, OpenError, Point, UnixMillis, Vec2, pack, unpack,
};

const GOLDEN: &str = "came_from_v58.curvyo";
const DAMAGED: &str = "came_from_damaged_v58.curvyo";
const OLDER: &str = "rounded_ellipse_v57.curvyo";

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn record() -> CameFrom {
    CameFrom {
        document: "Box job".into(),
        page: "Lid".into(),
        page_id: "11-4".into(),
        item: None,
        at: UnixMillis::new(1_764_000_000_000),
    }
}

fn anchors(peer: u64, n: usize) -> Vec<AnchorId> {
    (0..n as u64)
        .map(|i| AnchorId::new(peer, 100 + i))
        .collect()
}

fn square(d: &Document, x: f64, first: u64) -> NodeId {
    let ids = anchors(9, 4);
    let _ = first;
    d.create_path(
        &[
            NewAnchor::corner(ids[0], Point::new(x, 0.0)),
            NewAnchor::corner(ids[1], Point::new(x + 10.0, 0.0)),
            NewAnchor::corner(ids[2], Point::new(x + 10.0, 10.0)),
            NewAnchor::corner(ids[3], Point::new(x, 10.0)),
        ],
        true,
    )
}

/// An imported page holding a square, and an imported group of two squares on the first page.
fn golden_document() -> Document {
    let source = Document::new(11);
    let a = square(&source, 0.0, 0);
    let b = square(&source, 20.0, 4);
    let group = source.group_objects(&[a, b]).unwrap();
    source.rename_node(group, "Handle").unwrap();
    let d = Document::new(1);
    let page = source.extract_fragment(&[group], Some("first")).unwrap();
    d.insert_fragment(
        &page,
        FragmentTarget::Page,
        &record(),
        &anchors(1, page.anchor_count()),
    )
    .unwrap();
    let elements = source.extract_fragment(&[group], None).unwrap();
    d.insert_fragment(
        &elements,
        FragmentTarget::Elements {
            page: "first",
            offset: Vec2::new(0.0, 50.0),
        },
        &record(),
        &(200..200 + elements.anchor_count() as u64)
            .map(|i| AnchorId::new(1, i))
            .collect::<Vec<_>>(),
    )
    .unwrap();
    d
}

/// A golden with a record of the wrong type on its imported group.
fn damaged_document() -> Vec<u8> {
    let d = golden_document();
    let loro = loro::LoroDoc::new();
    loro.import(&d.export_loro_snapshot().unwrap()).unwrap();
    loro.set_peer_id(7).unwrap();
    let tree = loro.get_tree("paths");
    let mut stack = tree.roots();
    while let Some(node) = stack.pop() {
        let meta = tree.get_meta(node).unwrap();
        if meta.get("came_from").is_some() {
            meta.insert("came_from", "Box job").unwrap();
        }
        stack.extend(
            tree.children(loro::TreeParentId::Node(node))
                .unwrap_or_default(),
        );
    }
    loro.commit();
    let mut files = members(&pack(&d, "0.1.0").unwrap());
    files.retain(|(name, _)| name != "document.loro");
    files.push((
        "document.loro".into(),
        loro.export(loro::ExportMode::Snapshot).unwrap(),
    ));
    zip_of(&files)
}

fn members(bytes: &[u8]) -> Vec<(String, Vec<u8>)> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    (0..archive.len())
        .map(|i| {
            let mut file = archive.by_index(i).unwrap();
            let mut content = Vec::new();
            file.read_to_end(&mut content).unwrap();
            (file.name().unwrap().to_string(), content)
        })
        .collect()
}

fn zip_of(files: &[(String, Vec<u8>)]) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default();
    for (name, content) in files {
        writer.start_file(name.as_str(), options).unwrap();
        writer.write_all(content).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

fn stored(name: &str) -> Vec<u8> {
    if std::env::var_os("CURVYO_WRITE_FIXTURES").is_some() {
        match name {
            GOLDEN => std::fs::write(
                fixture_path(GOLDEN),
                pack(&golden_document(), "0.1.0").unwrap(),
            )
            .unwrap(),
            DAMAGED => std::fs::write(fixture_path(DAMAGED), damaged_document()).unwrap(),
            _ => {}
        }
    }
    std::fs::read(fixture_path(name)).unwrap()
}

fn records(d: &Document) -> Vec<Option<CameFrom>> {
    let mut found = Vec::new();
    for page in d.pages() {
        found.push(d.page_came_from(&page.id));
    }
    for id in d.children(None) {
        for child in d.children(Some(id)) {
            found.push(d.came_from(child));
        }
        found.push(d.came_from(id));
    }
    found
}

fn manifest_format(bytes: &[u8]) -> u64 {
    let files = members(bytes);
    let manifest = &files.iter().find(|(n, _)| n == "manifest.json").unwrap().1;
    serde_json::from_slice::<serde_json::Value>(manifest).unwrap()["format_version"]
        .as_u64()
        .unwrap()
}

/// Criterion 9: the golden is format 58; its page and its two group copies carry the record, and
/// it saves back unchanged.
#[test]
fn the_golden_reads_back_every_record() {
    const { assert!(CURRENT_FORMAT_VERSION >= 58) };
    let bytes = stored(GOLDEN);
    assert_eq!(manifest_format(&bytes), 58);
    let d = unpack(2, &bytes).unwrap();
    let pages = d.pages();
    assert_eq!(pages.len(), 2);
    assert_eq!(d.page_came_from(&pages[0].id), None);
    assert_eq!(d.page_came_from(&pages[1].id), Some(record()));
    let handle = record().with_item(Some("Handle".into()));
    let with_record: Vec<_> = records(&d).into_iter().flatten().collect();
    assert_eq!(with_record, [record(), handle.clone(), handle]);
    let reopened = unpack(3, &pack(&d, "0.1.0").unwrap()).unwrap();
    assert_eq!(records(&reopened), records(&d));
}

/// Criterion 9: a record of the wrong type opens, reads as absent and is kept untouched.
#[test]
fn a_damaged_record_opens_and_reads_as_absent() {
    let bytes = stored(DAMAGED);
    let d = unpack(2, &bytes).unwrap();
    let pages = d.pages();
    assert_eq!(d.page_came_from(&pages[1].id), Some(record()));
    let objects: Vec<_> = records(&d)[pages.len()..]
        .iter()
        .flatten()
        .cloned()
        .collect();
    assert!(objects.is_empty(), "{objects:?}");
    let loro = loro::LoroDoc::new();
    loro.import(&d.export_loro_snapshot().unwrap()).unwrap();
    let tree = loro.get_tree("paths");
    let kept = tree.get_nodes(false).into_iter().any(|node| {
        tree.get_meta(node.id)
            .ok()
            .and_then(|meta| meta.get("came_from"))
            .is_some_and(|value| value.get_deep_value() == loro::LoroValue::from("Box job"))
    });
    assert!(kept, "the value is kept");
}

/// Criterion 9: a build before this one refuses the file by version; a file of 57 opens.
#[test]
fn an_older_build_refuses_the_file_and_a_file_of_57_opens() {
    let bytes = stored(GOLDEN);
    let mut files = members(&bytes);
    for (name, content) in &mut files {
        if name == "manifest.json" {
            let mut json: serde_json::Value = serde_json::from_slice(content).unwrap();
            json["format_version"] = serde_json::json!(CURRENT_FORMAT_VERSION + 1);
            *content = serde_json::to_vec(&json).unwrap();
        }
    }
    assert!(matches!(
        unpack(2, &zip_of(&files)),
        Err(OpenError::FormatTooNew { found, .. }) if found == CURRENT_FORMAT_VERSION + 1
    ));
    let older = stored(OLDER);
    assert_eq!(manifest_format(&older), 57);
    assert!(unpack(2, &older).is_ok());
}

/// Criterion 2: a save writes the page list; a file of 57 has none and the reader says so.
#[test]
fn the_page_list_is_in_a_saved_file_and_absent_from_an_older_one() {
    let golden = members(&stored(GOLDEN));
    let list = &golden.iter().find(|(n, _)| n == LIBRARY_MEMBER).unwrap().1;
    let summary = LibrarySummary::from_bytes(list).unwrap();
    let names: Vec<_> = summary.pages.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, ["Page 1", "Page 1"]);
    let older = members(&stored(OLDER));
    assert!(older.iter().all(|(n, _)| n != LIBRARY_MEMBER));
}
