//! Format 39 (`specs/0101-laser-operations` criteria 25 and 26, ADR 0046 §4): the goldens
//! `operations_v39.curvyo`, `operations_unknown_kind_v39.curvyo` and
//! `operations_unreadable_v39.curvyo` read back every operation, switch, membership and Cuts last
//! identical, an unknown or unreadable record saves back unchanged register by register, and a
//! format-38 file opens unchanged. `CURVYO_WRITE_FIXTURES=1` rewrites the goldens.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::assert_is_empty,
    clippy::many_single_char_names
)]

use std::collections::BTreeMap;
use std::io::{Cursor, Read, Write};
use std::path::PathBuf;

use curvyo_document_core::{
    Affine, AnchorId, CURRENT_FORMAT_VERSION, Document, DocumentSize, Length, NewAnchor,
    NewOperation, NewOutputPage, NodeId, OpenError, OperationSnapshot, OperationSwitch, Page,
    Point, RectBounds, Vec2, pack, unpack,
};
use loro::{LoroDoc, LoroValue, TreeParentId};

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn laser(d: &Document) -> (Page, NodeId) {
    let page = d
        .add_output_page(NewOutputPage {
            process: "curvyo.laser",
            label: "Laser",
            profile: None,
            work_area: DocumentSize::from_mm(400.0, 300.0),
        })
        .unwrap();
    let mfg = page.kind.manufacturing().unwrap();
    (page, mfg)
}

/// Adds an operation, then assigns `members` in a step of its own, so no two adjacent steps of a
/// golden share a label (the legacy-history oracle of `0020`).
fn add(d: &Document, mfg: NodeId, kind: &str, name: &str, members: &[Vec<NodeId>]) -> NodeId {
    let id = d
        .add_operation(mfg, &NewOperation { kind, name }, usize::MAX, &[])
        .unwrap();
    d.set_operation_members(id, members, true).unwrap();
    id
}

/// Two laser pages. The first: the five kinds in a stored order with a cutting operation above
/// marking ones, Cuts last off, Show off on Score, Output off on Engrave, a Cut-out margin, and
/// memberships on an own item, an own group and its member, a linked clone, a placed page and an
/// inner row (a source object of the placed drawing page). The second: one Cut, Cuts last on.
fn golden_document() -> Document {
    let d = Document::new(1);
    let drawing = d.pages()[0].clone();
    let (first, mfg) = laser(&d);
    let to = |page: &Page| {
        Vec2::new(
            page.origin.x - drawing.origin.x + 20.0,
            page.origin.y - drawing.origin.y + 20.0,
        )
    };
    let c = |k: u64, x: f64, y: f64| NewAnchor::corner(AnchorId::new(1, k), Point::new(x, y));
    let source = d.create_rect(RectBounds::from_corners(
        Point::new(0.0, 0.0),
        Point::new(20.0, 10.0),
    ));
    let a = d.create_path(&[c(1, 60.0, 0.0), c(2, 70.0, 0.0), c(3, 70.0, 10.0)], true);
    let own = d.create_rect(RectBounds::from_corners(
        Point::new(0.0, 50.0),
        Point::new(30.0, 70.0),
    ));
    d.translate_objects(&[own], to(&first)).unwrap();
    let b = d.create_path(&[c(4, 80.0, 0.0), c(5, 90.0, 5.0)], false);
    d.translate_objects(&[a, b], to(&first)).unwrap();
    let group = d.group_objects(&[a, b]).unwrap();
    let clone = d.place_linked_clones(&[source], to(&first)).unwrap()[0];
    let placement = d
        .place_page(
            &first.id,
            &drawing.id,
            Affine::translation(Vec2::new(150.0, 10.0)),
        )
        .unwrap();
    let cut = add(&d, mfg, "cut", "Cut 1", &[vec![own], vec![source]]);
    let engrave = add(&d, mfg, "engrave", "Engrave 1", &[vec![group]]);
    let score = add(&d, mfg, "score", "Score 1", &[vec![clone], vec![b]]);
    add(
        &d,
        mfg,
        "engrave_outline",
        "Engrave + Outline 1",
        &[vec![placement]],
    );
    let cut_out = add(&d, mfg, "cut_out", "Cut-out 1", &[vec![own]]);
    d.set_operation_switch(score, OperationSwitch::Show, false)
        .unwrap();
    d.set_operation_switch(engrave, OperationSwitch::Output, false)
        .unwrap();
    d.set_operation_margin(cut_out, Length::from_mm(1.5))
        .unwrap();
    d.rename_operation(cut, "Outer cut").unwrap();
    d.set_cuts_last(mfg, false, &[]).unwrap();
    let (second, mfg2) = laser(&d);
    let own2 = d.create_rect(RectBounds::from_corners(
        Point::new(0.0, 0.0),
        Point::new(5.0, 5.0),
    ));
    d.translate_objects(&[own2], to(&second)).unwrap();
    add(&d, mfg2, "cut", "Cut 1", &[vec![own2]]);
    d.set_cuts_last(mfg2, true, &[]).unwrap();
    d
}

/// One operation of a kind this build does not know, holding one member, beside a Cut.
fn unknown_kind_document() -> Document {
    let d = Document::new(1);
    let (page, mfg) = laser(&d);
    let x = d.create_rect(RectBounds::from_corners(
        Point::new(page.origin.x + 10.0, page.origin.y + 10.0),
        Point::new(page.origin.x + 30.0, page.origin.y + 20.0),
    ));
    add(&d, mfg, "dot_mode", "Dots", &[vec![x]]);
    add(&d, mfg, "cut", "Cut 1", &[vec![x]]);
    d
}

fn read_or_write(name: &str, build: impl Fn() -> Vec<u8>) -> Vec<u8> {
    if std::env::var_os("CURVYO_WRITE_FIXTURES").is_some() {
        std::fs::write(fixture_path(name), build()).unwrap();
    }
    std::fs::read(fixture_path(name)).unwrap()
}

fn members(bytes: &[u8]) -> BTreeMap<String, Vec<u8>> {
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

fn rezip(files: &BTreeMap<String, Vec<u8>>) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, content) in files {
        writer
            .start_file(name.as_str(), zip::write::SimpleFileOptions::default())
            .unwrap();
        writer.write_all(content).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

fn loro_of(bytes: &[u8]) -> LoroDoc {
    let loro = LoroDoc::new();
    loro.import(&members(bytes)["document.loro"]).unwrap();
    loro
}

/// Every operation node's registers as stored, in tree order.
fn raw_operations(bytes: &[u8]) -> Vec<BTreeMap<String, LoroValue>> {
    let loro = loro_of(bytes);
    let tree = loro.get_tree("paths");
    let mut found = Vec::new();
    for top in tree.children(TreeParentId::Root).unwrap() {
        for child in tree.children(TreeParentId::Node(top)).unwrap_or_default() {
            let meta = tree.get_meta(child).unwrap();
            if meta.get("shape").and_then(|v| v.into_value().ok())
                == Some(LoroValue::from("operation"))
            {
                let mut registers = BTreeMap::new();
                meta.for_each(|key, value| {
                    registers.insert(key.to_owned(), value.into_value().unwrap());
                });
                found.push(registers);
            }
        }
    }
    found
}

fn format_of(bytes: &[u8]) -> u64 {
    let json: serde_json::Value = serde_json::from_slice(&members(bytes)["manifest.json"]).unwrap();
    json["format_version"].as_u64().unwrap()
}

/// The golden with its unknown operation's kind made a number and its Show switch a text: a
/// record no build can read.
fn unreadable_bytes() -> Vec<u8> {
    let bytes = pack(&unknown_kind_document(), "0.1.0").unwrap();
    let loro = loro_of(&bytes);
    let tree = loro.get_tree("paths");
    for top in tree.children(TreeParentId::Root).unwrap() {
        for child in tree.children(TreeParentId::Node(top)).unwrap_or_default() {
            let meta = tree.get_meta(child).unwrap();
            if meta.get("op_kind").and_then(|v| v.into_value().ok())
                == Some(LoroValue::from("dot_mode"))
            {
                meta.insert("op_kind", 7_i64).unwrap();
                meta.insert("op_show", "yes").unwrap();
                meta.insert("op_margin", -2.0).unwrap();
            }
        }
    }
    loro.commit();
    let mut files = members(&bytes);
    files.insert(
        "document.loro".to_owned(),
        loro.export(loro::ExportMode::Snapshot).unwrap(),
    );
    rezip(&files)
}

/// One Cut of a rectangle, and a membership key whose path names nodes that are not in the file
/// (a merge leftover, ADR 0046 §2 (d)): it reads as nothing and is kept.
fn dangling_bytes() -> Vec<u8> {
    let d = Document::new(1);
    let (_, mfg) = laser(&d);
    let x = d.create_rect(RectBounds::from_corners(
        Point::new(0.0, 0.0),
        Point::new(20.0, 10.0),
    ));
    add(&d, mfg, "cut", "Cut 1", &[vec![x]]);
    let bytes = pack(&d, "0.1.0").unwrap();
    let loro = loro_of(&bytes);
    let tree = loro.get_tree("paths");
    for top in tree.children(TreeParentId::Root).unwrap() {
        for child in tree.children(TreeParentId::Node(top)).unwrap_or_default() {
            let meta = tree.get_meta(child).unwrap();
            if meta.get("op_kind").and_then(|v| v.into_value().ok()) == Some(LoroValue::from("cut"))
            {
                meta.insert("member.9:999/9:998", true).unwrap();
            }
        }
    }
    loro.commit();
    let mut files = members(&bytes);
    files.insert(
        "document.loro".to_owned(),
        loro.export(loro::ExportMode::Snapshot).unwrap(),
    );
    rezip(&files)
}

#[test]
fn a_dangling_key_reads_as_nothing_and_is_saved_back_unchanged() {
    let bytes = read_or_write("operations_dangling_v39.curvyo", dangling_bytes);
    assert_eq!(format_of(&bytes), 39);
    let opened = unpack(2, &bytes).unwrap();
    assert_eq!(opened.own_step_count(), 0, "opening writes no step");
    let list = &all_operations(&opened)[0];
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].members.len(), 1, "only the live key");
    let saved = pack(&opened, "0.1.0").unwrap();
    assert_eq!(raw_operations(&saved), raw_operations(&bytes));
}

fn manufacturing_ids(d: &Document) -> Vec<NodeId> {
    d.manufacturings().iter().map(|m| m.id).collect()
}

fn all_operations(d: &Document) -> Vec<Vec<OperationSnapshot>> {
    manufacturing_ids(d)
        .into_iter()
        .map(|m| d.operations(m).unwrap())
        .collect()
}

#[test]
fn the_golden_reads_back_every_operation_switch_membership_and_cuts_last() {
    let expected = golden_document();
    let bytes = read_or_write("operations_v39.curvyo", || {
        pack(&golden_document(), "0.1.0").unwrap()
    });
    assert_eq!(format_of(&bytes), 39);
    // Format 41 since `specs/0103-fixture-jobs` (ADR 0052 §6).
    // Format 42 since `specs/0110-pen-modes-variable-stroke` (ADR 0053).
    // Format 44 since `specs/0111-laser-operation-tools` (ADR 0056 §8).
    assert_eq!(CURRENT_FORMAT_VERSION, 44);
    let opened = unpack(2, &bytes).unwrap();
    assert_eq!(opened.own_step_count(), 0, "opening writes no step");
    let lists = all_operations(&opened);
    assert_eq!(lists, all_operations(&expected));
    let first = &lists[0];
    let kinds: Vec<_> = first.iter().map(|o| o.kind.clone().unwrap()).collect();
    assert_eq!(
        kinds,
        ["cut", "engrave", "score", "engrave_outline", "cut_out"]
    );
    let names: Vec<_> = first.iter().map(|o| o.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "Outer cut",
            "Engrave 1",
            "Score 1",
            "Engrave + Outline 1",
            "Cut-out 1"
        ]
    );
    let switches: Vec<_> = first.iter().map(|o| (o.show, o.output)).collect();
    assert_eq!(
        switches,
        [
            (true, true),
            (true, false),
            (false, true),
            (true, true),
            (true, true)
        ]
    );
    assert_eq!(first[4].margin.as_mm(), 1.5);
    assert_eq!(first[0].members.len(), 2, "own item and inner row");
    assert_eq!(first[2].members.len(), 2, "clone and a group member");
    assert!(
        opened.is_placement(first[3].members[0][0]),
        "a placed page's own record"
    );
    assert_eq!(lists[1].len(), 1);
    let cuts_last = |d: &Document| -> Vec<serde_json::Value> {
        d.manufacturings()
            .iter()
            .map(|m| m.process_values["x.curvyo.laser.cuts_last"].clone())
            .collect()
    };
    assert_eq!(cuts_last(&opened), [false, true]);
    let saved = pack(&opened, "0.1.0").unwrap();
    assert_eq!(
        raw_operations(&saved),
        raw_operations(&bytes),
        "round trip identical"
    );
    assert_eq!(all_operations(&unpack(3, &saved).unwrap()), lists);
}

#[test]
fn a_clone_member_key_is_a_path_and_reads_back_unchanged() {
    // Frage 212 A (ADR 0056 §2): the legacy key is read, kept and saved back unchanged; the laser
    // rules ignore it.
    let bytes = std::fs::read(fixture_path("operations_clone_member_v39.curvyo")).unwrap();
    assert_eq!(format_of(&bytes), 39);
    let opened = unpack(2, &bytes).unwrap();
    assert_eq!(opened.own_step_count(), 0, "opening writes no step");
    let list = &all_operations(&opened)[0];
    assert_eq!(list[0].members.len(), 1);
    assert_eq!(list[0].members[0].len(), 2, "the clone, then its member");
    let raw = raw_operations(&bytes);
    let keys: Vec<&String> = raw
        .iter()
        .flat_map(|op| op.keys())
        .filter(|key| key.starts_with("member."))
        .collect();
    assert_eq!(keys.len(), 1);
    assert_eq!(keys[0].matches('/').count(), 1, "a path of two ids");
    let saved = pack(&opened, "0.1.0").unwrap();
    assert_eq!(raw_operations(&saved), raw_operations(&bytes));
}

#[test]
fn an_unknown_kind_is_kept_with_its_members_and_saved_back_unchanged() {
    let bytes = read_or_write("operations_unknown_kind_v39.curvyo", || {
        pack(&unknown_kind_document(), "0.1.0").unwrap()
    });
    assert_eq!(format_of(&bytes), 39);
    let opened = unpack(2, &bytes).unwrap();
    assert_eq!(opened.own_step_count(), 0);
    let list = &all_operations(&opened)[0];
    assert_eq!(list.len(), 2);
    assert_eq!(list[0].kind.as_deref(), Some("dot_mode"));
    assert_eq!(list[0].name, "Dots");
    assert_eq!(list[0].members, list[1].members);
    assert_eq!(list[0].members.len(), 1);
    let saved = pack(&opened, "0.1.0").unwrap();
    assert_eq!(raw_operations(&saved), raw_operations(&bytes));
}

#[test]
fn an_unreadable_record_reads_with_no_kind_and_is_saved_back_unchanged() {
    let bytes = read_or_write("operations_unreadable_v39.curvyo", unreadable_bytes);
    assert_eq!(format_of(&bytes), 39);
    let raw = raw_operations(&bytes);
    assert_eq!(
        raw[0]["op_kind"],
        LoroValue::I64(7),
        "the golden holds the damage"
    );
    assert_eq!(raw[0]["op_show"], LoroValue::from("yes"));
    let opened = unpack(2, &bytes).unwrap();
    assert_eq!(opened.own_step_count(), 0);
    let list = &all_operations(&opened)[0];
    assert_eq!(list[0].kind, None, "an Unknown operation");
    assert_eq!(list[0].name, "Dots");
    assert_eq!(list[0].members.len(), 1, "kept with its memberships");
    assert_eq!(
        list[1].kind.as_deref(),
        Some("cut"),
        "the readable one beside it"
    );
    let saved = pack(&opened, "0.1.0").unwrap();
    assert_eq!(raw_operations(&saved), raw);
}

#[test]
fn a_build_of_format_38_refuses_the_golden_as_saved_by_a_newer_version() {
    // A format-38 build checks `format_version` against its own 38 and refuses the golden's 39
    // before it could read an operation node as damage. This build plays the check one up.
    let mut files = members(&read_or_write("operations_v39.curvyo", || {
        pack(&golden_document(), "0.1.0").unwrap()
    }));
    let mut manifest: serde_json::Value = serde_json::from_slice(&files["manifest.json"]).unwrap();
    manifest["format_version"] = serde_json::json!(CURRENT_FORMAT_VERSION + 1);
    files.insert(
        "manifest.json".to_owned(),
        serde_json::to_vec(&manifest).unwrap(),
    );
    let found_newer = matches!(
        unpack(2, &rezip(&files)),
        Err(OpenError::FormatTooNew { found, supported })
            if found == CURRENT_FORMAT_VERSION + 1 && supported == CURRENT_FORMAT_VERSION
    );
    assert!(found_newer);
}

#[test]
fn a_format_38_golden_opens_unchanged_and_is_not_rewritten() {
    let bytes = std::fs::read(fixture_path("bring_content_in_v38.curvyo")).unwrap();
    assert_eq!(format_of(&bytes), 38);
    let opened = unpack(2, &bytes).unwrap();
    assert_eq!(opened.own_step_count(), 0);
    assert!(all_operations(&opened).iter().all(Vec::is_empty));
}
