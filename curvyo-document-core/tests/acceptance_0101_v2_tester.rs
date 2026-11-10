//! Tester acceptance cases for `specs/0101-laser-operations` after the customer's design of
//! 2026-11-10 on the file: no operation colour in any golden (ADR 0046 decision 1, criterion 2),
//! and a format 39 file that still carries the old `op_color` register opens, reads the same
//! operations and saves without error (no colour is read anywhere).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::too_many_lines,
    missing_docs
)]

use std::io::{Cursor, Read, Write};
use std::path::PathBuf;

use curvyo_document_core::{Document, OperationSnapshot, pack, unpack};
use loro::{ExportMode, LoroDoc, LoroValue};
use zip::write::SimpleFileOptions;

const GOLDENS: [&str; 5] = [
    "operations_v39.curvyo",
    "operations_clone_member_v39.curvyo",
    "operations_unknown_kind_v39.curvyo",
    "operations_dangling_v39.curvyo",
    "operations_unreadable_v39.curvyo",
];

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name),
    )
    .unwrap()
}

/// Every member of a zip container, decompressed.
fn members(bytes: &[u8]) -> Vec<(String, Vec<u8>)> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut out = Vec::new();
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();
        let name = file
            .enclosed_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
        let mut data = Vec::new();
        file.read_to_end(&mut data).unwrap();
        out.push((name, data));
    }
    out
}

fn contains(hay: &[u8], needle: &[u8]) -> bool {
    hay.windows(needle.len()).any(|w| w == needle)
}

fn operations_text(d: &Document) -> String {
    let mut parts = Vec::new();
    for m in d.manufacturings() {
        parts.push(format!("{:?}", d.operations(m.id).unwrap()));
    }
    parts.join("|")
}

#[test]
fn c2_no_golden_carries_an_op_color_register_or_colour_text() {
    for name in GOLDENS {
        let bytes = fixture(name);
        for (member, data) in members(&bytes) {
            assert!(
                !contains(&data, b"op_color"),
                "{name} member {member} holds an op_color register"
            );
        }
        let d = unpack(9, &bytes).unwrap();
        let packed = pack(&d, "0101-tester").unwrap();
        for (member, data) in members(&packed) {
            assert!(
                !contains(&data, b"op_color"),
                "a re-saved {name} member {member} holds an op_color register"
            );
        }
    }
}

/// Builds a format 39 file from the `operations_v39` golden with an `op_color` register on every
/// operation node: the register an earlier build wrote (ADR 0046 decision 1, removed 2026-11-10).
fn with_old_colour_register(bytes: &[u8]) -> Vec<u8> {
    let mut entries = members(bytes);
    let loro_index = entries
        .iter()
        .position(|(n, _)| n == "document.loro")
        .unwrap();
    let doc = LoroDoc::new();
    doc.import(&entries[loro_index].1).unwrap();
    let tree = doc.get_tree("paths");
    let mut touched = 0;
    for id in tree.nodes() {
        let meta = tree.get_meta(id).unwrap();
        if meta.get("op_kind").is_some() {
            meta.insert("op_color", LoroValue::String("op-3".into()))
                .unwrap();
            touched += 1;
        }
    }
    assert!(touched > 0, "the golden holds operation nodes");
    doc.commit();
    entries[loro_index].1 = doc.export(ExportMode::Snapshot).unwrap();

    let mut out = Cursor::new(Vec::new());
    {
        let mut writer = zip::ZipWriter::new(&mut out);
        for (name, data) in &entries {
            writer
                .start_file(name.as_str(), SimpleFileOptions::default())
                .unwrap();
            writer.write_all(data).unwrap();
        }
        writer.finish().unwrap();
    }
    out.into_inner()
}

#[test]
fn c2_a_v39_file_with_the_old_colour_register_opens_with_the_same_operations() {
    let golden = fixture("operations_v39.curvyo");
    let with_colour = with_old_colour_register(&golden);

    let plain = unpack(9, &golden).unwrap();
    let old = unpack(9, &with_colour).expect("a file with the old register still opens");

    assert_eq!(
        operations_text(&plain),
        operations_text(&old),
        "the operations read the same with the register present (it is ignored or kept, never read)"
    );
    let saved = pack(&old, "0101-tester").expect("and saves without error");
    let back = unpack(9, &saved).unwrap();
    assert_eq!(operations_text(&plain), operations_text(&back));
}

#[test]
fn c2_the_snapshot_of_an_operation_has_no_colour_field() {
    // Compile-time seam: the snapshot is built from its listed fields only. A colour field would
    // show up here as a new Debug entry on the first operation of the golden.
    let d = unpack(9, &fixture("operations_v39.curvyo")).unwrap();
    let all: Vec<OperationSnapshot> = d
        .manufacturings()
        .into_iter()
        .flat_map(|m| d.operations(m.id).unwrap())
        .collect();
    assert_ne!(all.len(), 0);
    for op in &all {
        let text = format!("{op:?}").to_ascii_lowercase();
        assert!(!text.contains("colo"), "snapshot names a colour: {text}");
    }
}
