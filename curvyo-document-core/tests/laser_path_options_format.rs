//! Formats 67 and 68 (`specs/0139-laser-cut-path-options` slices 1 and 2, adrs L2 to L6): the
//! goldens `laser_path_options_v67.curvyo` and `laser_path_options_damaged_v67.curvyo` read back
//! every tab option and every own tabs record, `laser_path_options_v68.curvyo` and
//! `laser_path_options_damaged_v68.curvyo` every path option, own path options and entry, as plain
//! values, and save back unchanged; older files open with no records; a file of the next format
//! is refused. The laser extension's reading of the same goldens is in
//! `curvyo-laser-core/tests/tabs_document_goldens.rs`. `CURVYO_WRITE_FIXTURES=1` rewrites the
//! goldens of the current format only.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeMap;
use std::io::{Cursor, Read, Write};
use std::path::PathBuf;

use curvyo_document_core::{
    ADD_TAB_COMMIT_LABEL, AnchorId, CURRENT_FORMAT_VERSION, Document, DocumentSize, ItemRecord,
    MOVE_START_POINT_COMMIT_LABEL, NewAnchor, NewOperation, NewOutputPage, NodeId,
    OWN_PATH_OPTIONS_COMMIT_LABEL, OpenError, Point, RectBounds, member_key_text, pack, unpack,
};
use loro::{LoroDoc, LoroValue, TreeParentId};
use serde_json::{Map, Value, json};

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn read_or_write(name: &str, build: impl Fn() -> Vec<u8>) -> Vec<u8> {
    if std::env::var_os("CURVYO_WRITE_FIXTURES").is_some() && name.contains("_v68") {
        std::fs::write(fixture_path(name), build()).unwrap();
    }
    std::fs::read(fixture_path(name)).unwrap()
}

fn map(value: Value) -> Map<String, Value> {
    let Value::Object(map) = value else {
        panic!("a map")
    };
    map
}

fn laser(d: &Document) -> NodeId {
    d.add_output_page(NewOutputPage {
        process: "curvyo.laser",
        label: "Laser",
        profile: None,
        work_area: DocumentSize::from_mm(400.0, 300.0),
    })
    .unwrap()
    .kind
    .manufacturing()
    .unwrap()
}

fn op(d: &Document, mfg: NodeId, kind: &str, name: &str, members: &[Vec<NodeId>]) -> NodeId {
    let id = d
        .add_operation(
            mfg,
            &NewOperation {
                kind,
                name,
                settings: None,
            },
            99,
            &[],
        )
        .unwrap();
    if !members.is_empty() {
        d.set_operation_members(id, members, true).unwrap();
    }
    id
}

fn rect(d: &Document, x: f64) -> NodeId {
    d.create_rect(RectBounds::from_corners(
        Point::new(x, 10.0),
        Point::new(x + 20.0, 30.0),
    ))
}

fn tabs(d: &Document, op: NodeId, target: NodeId, record: Value) {
    tabs_as(d, op, target, record, ADD_TAB_COMMIT_LABEL);
}

/// As [`tabs`] with the step label `label`: no two adjacent steps of a golden share a label (the
/// legacy-history oracle of `0020`).
fn tabs_as(d: &Document, op: NodeId, target: NodeId, record: Value, label: &str) {
    let key = member_key_text(&[target]);
    assert!(
        d.write_operation_tabs(op, &key, Some(&map(record)), label)
            .unwrap()
    );
}

/// A Cut with Count 4, Width 2 mm and Skip inner shapes; an own item with own tabs (one with an
/// own width); a group whose record names its member's outline; a Cut-out item with tabs on its
/// contour and a Cut-out whose item has none.
#[allow(clippy::many_single_char_names)] // the items a to e of the golden
fn golden_document() -> Document {
    let d = Document::new(1);
    let mfg = laser(&d);
    let a = rect(&d, 0.0);
    let cut = op(&d, mfg, "cut", "Cut 1", &[vec![a]]);
    assert!(
        d.write_operation_options(
            cut,
            &map(json!({"tab_count": 4, "tab_width_mm": 2.0, "tab_skip_inner": true})),
        )
        .unwrap()
    );
    tabs(
        &d,
        cut,
        a,
        json!({"0": [{"at": 125_000_000, "width_mm": 1.5}, {"at": 625_000_000}]}),
    );
    let b = rect(&d, 40.0);
    let cut_out = op(&d, mfg, "cut_out", "Cut-out 1", &[]);
    let c = rect(&d, 80.0);
    let group = d.group_objects(&[b, c]).unwrap();
    assert!(d.set_operation_members(cut, &[vec![group]], true).unwrap());
    let member = member_key_text(&[c]);
    tabs_as(
        &d,
        cut,
        group,
        json!({ format!("{member}:0"): [] }),
        curvyo_document_core::OWN_TABS_COMMIT_LABEL,
    );
    let e = rect(&d, 120.0);
    assert!(d.set_operation_members(cut_out, &[vec![e]], true).unwrap());
    assert!(
        d.write_operation_options(cut_out, &map(json!({"tab_count": 2})))
            .unwrap()
    );
    tabs(
        &d,
        cut_out,
        e,
        json!({"0": [{"at": 0}, {"at": 500_000_000}]}),
    );
    d
}

/// The damaged records, one operation each: not a map, a fraction outside 0 to 1, a width out
/// of range, an unknown field, overlapping (equal) positions; and damaged tab options.
fn damaged_records() -> Vec<Value> {
    vec![
        json!({"0": 3}),
        json!({"0": [{"at": 1_000_000_000_u64}]}),
        json!({"0": [{"at": 1, "width_mm": 20.01}]}),
        json!({"0": [{"at": 1, "side": "left"}]}),
        json!({"0": [{"at": 5}, {"at": 5}]}),
    ]
}

fn damaged_document() -> Document {
    let d = Document::new(1);
    let mfg = laser(&d);
    for (n, record) in damaged_records().into_iter().enumerate() {
        let x = 30.0 * f64::from(u32::try_from(n).unwrap());
        let item = rect(&d, x);
        let cut = op(&d, mfg, "cut", &format!("Damaged {}", n + 2), &[vec![item]]);
        tabs(&d, cut, item, record);
    }
    let options = op(&d, mfg, "cut", "Damaged options", &[]);
    assert!(
        d.write_operation_options(
            options,
            &map(json!({"tab_count": 21, "tab_width_mm": 0.05}))
        )
        .unwrap()
    );
    let scored = op(&d, mfg, "score", "Damaged kind", &[]);
    assert!(
        d.write_operation_options(scored, &map(json!({"tab_count": 1})))
            .unwrap()
    );
    let item = rect(&d, 300.0);
    op(&d, mfg, "cut", "Damaged 1", &[vec![item]]);
    d
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

/// The damaged golden with "Damaged 1"'s record planted as a text (no map at all).
fn damaged_bytes() -> Vec<u8> {
    let d = damaged_document();
    let bytes = pack(&d, "0.1.0").unwrap();
    let loro = LoroDoc::new();
    loro.import(&members(&bytes)["document.loro"]).unwrap();
    let tree = loro.get_tree("paths");
    let mut stack = tree.children(TreeParentId::Root).unwrap();
    while let Some(node) = stack.pop() {
        let meta = tree.get_meta(node).unwrap();
        if meta.get("name").and_then(|v| v.into_value().ok()) == Some(LoroValue::from("Damaged 1"))
        {
            let mut member = None;
            meta.for_each(|key, _| {
                if let Some(text) = key.strip_prefix("member.") {
                    member = Some(text.to_owned());
                }
            });
            meta.insert(&format!("tabs.{}", member.unwrap()), "on")
                .unwrap();
        }
        stack.extend(tree.children(TreeParentId::Node(node)).unwrap_or_default());
    }
    loro.commit();
    let mut files = members(&bytes);
    files.insert(
        "document.loro".to_owned(),
        loro.export(loro::ExportMode::Snapshot).unwrap(),
    );
    rezip(&files)
}

fn format_of(bytes: &[u8]) -> u64 {
    let json: Value = serde_json::from_slice(&members(bytes)["manifest.json"]).unwrap();
    json["format_version"].as_u64().unwrap()
}

/// The registers of every live node, sorted, as stored.
fn raw(bytes: &[u8]) -> Vec<BTreeMap<String, LoroValue>> {
    let loro = LoroDoc::new();
    loro.import(&members(bytes)["document.loro"]).unwrap();
    let tree = loro.get_tree("paths");
    let mut out = Vec::new();
    let mut stack = tree.children(TreeParentId::Root).unwrap();
    while let Some(node) = stack.pop() {
        let mut registers = BTreeMap::new();
        tree.get_meta(node).unwrap().for_each(|key, value| {
            let plain = match value {
                loro::ValueOrContainer::Value(value) => value,
                container @ loro::ValueOrContainer::Container(_) => container.get_deep_value(),
            };
            registers.insert(key.to_owned(), plain);
        });
        out.push(registers);
        stack.extend(tree.children(TreeParentId::Node(node)).unwrap_or_default());
    }
    out.sort_by_key(|r| format!("{r:?}"));
    out
}

fn open_unchanged(bytes: &[u8]) -> Document {
    let opened = unpack(2, bytes).unwrap();
    assert_eq!(opened.own_step_count(), 0, "opening writes no step");
    let saved = pack(&opened, "0.1.0").unwrap();
    assert_eq!(raw(&saved), raw(bytes), "saved back unchanged");
    opened
}

#[test]
fn the_golden_reads_back_every_tab_option_and_record() {
    let bytes = read_or_write("laser_path_options_v67.curvyo", || {
        pack(&golden_document(), "0.1.0").unwrap()
    });
    assert_eq!(format_of(&bytes), 67);
    let d = open_unchanged(&bytes);
    let ops = d.operations(d.manufacturings()[0].id).unwrap();
    assert_eq!(
        ops[0].options,
        Some(json!({"tab_count": 4, "tab_width_mm": 2.0, "tab_skip_inner": true}))
    );
    let records = d.operation_tabs(ops[0].id).unwrap();
    assert_eq!(records.len(), 2, "the item and the group");
    let item = member_key_text(&ops[0].members[0]);
    assert!(records.contains_key(&item), "{records:?}");
    let cut_out = d.operation_tabs(ops[1].id).unwrap();
    assert_eq!(
        cut_out.values().next(),
        Some(&json!({"0": [{"at": 0}, {"at": 500_000_000}]}))
    );
}

#[test]
fn every_damaged_record_is_kept_and_saved_back_unchanged() {
    let bytes = read_or_write("laser_path_options_damaged_v67.curvyo", damaged_bytes);
    assert_eq!(format_of(&bytes), 67);
    let d = open_unchanged(&bytes);
    let ops = d.operations(d.manufacturings()[0].id).unwrap();
    assert_eq!(ops.len(), 8);
    for (op, record) in ops.iter().zip(damaged_records()) {
        let stored = d.operation_tabs(op.id).unwrap();
        assert_eq!(stored.values().next(), Some(&record), "{}", op.name);
    }
    let planted = ops.iter().find(|o| o.name == "Damaged 1").unwrap();
    assert_eq!(
        d.operation_tabs(planted.id).unwrap().values().next(),
        Some(&json!("on"))
    );
}

#[test]
fn older_files_open_with_no_records_and_a_newer_file_is_refused() {
    for name in [
        "operation_tools_v44.curvyo",
        "generated_part_ops_v65.curvyo",
        "laser_path_options_v67.curvyo",
    ] {
        let bytes = std::fs::read(fixture_path(name)).unwrap();
        let opened = open_unchanged(&bytes);
        for manufacturing in opened.manufacturings() {
            for operation in opened.operations(manufacturing.id).unwrap() {
                if !name.contains("_v67") {
                    assert_eq!(opened.operation_tabs(operation.id).unwrap().len(), 0);
                }
                for family in [ItemRecord::PathOwn, ItemRecord::Entry] {
                    assert!(
                        opened
                            .operation_records(operation.id, family)
                            .unwrap()
                            .is_empty()
                    );
                }
            }
        }
    }
    let bytes = pack(&golden_document(), "0.1.0").unwrap();
    let mut files = members(&bytes);
    let mut manifest: Value = serde_json::from_slice(&files["manifest.json"]).unwrap();
    manifest["format_version"] = json!(CURRENT_FORMAT_VERSION + 1);
    files.insert(
        "manifest.json".to_owned(),
        serde_json::to_vec(&manifest).unwrap(),
    );
    assert!(matches!(
        unpack(2, &rezip(&files)),
        Err(OpenError::FormatTooNew {
            found: 70,
            supported: 69
        })
    ));
}

fn record(d: &Document, op: NodeId, target: NodeId, family: ItemRecord, value: Value, label: &str) {
    let key = member_key_text(&[target]);
    assert!(
        d.write_operation_records(op, &[(family, key, Some(map(value)))], label)
            .unwrap()
    );
}

/// Slice 2: a Cut with Lead-in Arc 2 mm, Lead-out Line and Overcut 1.5 mm; an item with own path
/// options and a set start; an open line with a fixed direction; a group whose record names its
/// member's outline; a Cut-out item with a set start on its contour.
#[allow(clippy::many_single_char_names)] // the items of the golden
fn golden_document_v68() -> Document {
    let d = Document::new(1);
    let mfg = laser(&d);
    let a = rect(&d, 0.0);
    let cut = op(&d, mfg, "cut", "Cut 1", &[vec![a]]);
    assert!(
        d.write_operation_options(
            cut,
            &map(
                json!({"lead_in": "arc", "lead_in_mm": 2.0, "lead_out": "line",
                        "lead_out_mm": 1.0, "overcut_mm": 1.5})
            ),
        )
        .unwrap()
    );
    record(
        &d,
        cut,
        a,
        ItemRecord::PathOwn,
        json!({"lead_in": "line", "lead_in_mm": 0.5, "lead_out": "none", "lead_out_mm": 1.0,
               "overcut_mm": 0.0}),
        OWN_PATH_OPTIONS_COMMIT_LABEL,
    );
    record(
        &d,
        cut,
        a,
        ItemRecord::Entry,
        json!({"0": {"start": 250_000_000}}),
        MOVE_START_POINT_COMMIT_LABEL,
    );
    let l = d.create_path(
        &[
            NewAnchor::corner(AnchorId::new(1, 1), Point::new(0.0, 60.0)),
            NewAnchor::corner(AnchorId::new(1, 2), Point::new(50.0, 60.0)),
        ],
        false,
    );
    assert!(d.set_operation_members(cut, &[vec![l]], true).unwrap());
    record(
        &d,
        cut,
        l,
        ItemRecord::Entry,
        json!({"0": {"direction": "reversed"}}),
        curvyo_document_core::DIRECTION_COMMIT_LABEL,
    );
    let b = rect(&d, 40.0);
    let cut_out = op(&d, mfg, "cut_out", "Cut-out 1", &[]);
    let c = rect(&d, 80.0);
    let group = d.group_objects(&[b, c]).unwrap();
    assert!(d.set_operation_members(cut, &[vec![group]], true).unwrap());
    let member = member_key_text(&[c]);
    record(
        &d,
        cut,
        group,
        ItemRecord::Entry,
        json!({ format!("{member}:0"): {"start": 0} }),
        MOVE_START_POINT_COMMIT_LABEL,
    );
    let e = rect(&d, 120.0);
    assert!(d.set_operation_members(cut_out, &[vec![e]], true).unwrap());
    assert!(
        d.write_operation_options(cut_out, &map(json!({"overcut_mm": 2.0})))
            .unwrap()
    );
    record(
        &d,
        cut_out,
        e,
        ItemRecord::Entry,
        json!({"0": {"start": 500_000_000}}),
        MOVE_START_POINT_COMMIT_LABEL,
    );
    d
}

/// The damaged slice 2 records, one operation each: own path options not a map, out of range,
/// with a missing value; an entry not a map, a start outside 0 to 1, an unknown direction.
fn damaged_records_v68() -> Vec<(ItemRecord, Value)> {
    let own = |overcut: f64| {
        json!({"lead_in": "none", "lead_in_mm": 1.0, "lead_out": "none", "lead_out_mm": 1.0,
               "overcut_mm": overcut})
    };
    vec![
        (ItemRecord::PathOwn, json!({"lead_in": ["arc"]})),
        (ItemRecord::PathOwn, own(10.5)),
        (ItemRecord::PathOwn, json!({"lead_in": "line"})),
        (ItemRecord::Entry, json!({"0": 3})),
        (
            ItemRecord::Entry,
            json!({"0": {"start": 1_000_000_000_u64}}),
        ),
        (ItemRecord::Entry, json!({"0": {"direction": "sideways"}})),
    ]
}

fn damaged_document_v68() -> Document {
    let d = Document::new(1);
    let mfg = laser(&d);
    for (n, (family, value)) in damaged_records_v68().into_iter().enumerate() {
        let x = 30.0 * f64::from(u32::try_from(n).unwrap());
        let item = rect(&d, x);
        let cut = op(&d, mfg, "cut", &format!("Damaged {}", n + 1), &[vec![item]]);
        let label = if n % 2 == 0 {
            OWN_PATH_OPTIONS_COMMIT_LABEL
        } else {
            MOVE_START_POINT_COMMIT_LABEL
        };
        record(&d, cut, item, family, value, label);
    }
    let options = op(&d, mfg, "cut", "Damaged options", &[]);
    assert!(
        d.write_operation_options(
            options,
            &map(json!({"lead_in": "spiral", "overcut_mm": 10.01}))
        )
        .unwrap()
    );
    let scored = op(&d, mfg, "score", "Damaged kind", &[]);
    assert!(
        d.write_operation_options(scored, &map(json!({"lead_out": "line"})))
            .unwrap()
    );
    d
}

#[test]
fn the_v68_golden_reads_back_every_path_option_and_record() {
    let bytes = read_or_write("laser_path_options_v68.curvyo", || {
        pack(&golden_document_v68(), "0.1.0").unwrap()
    });
    assert_eq!(format_of(&bytes), 68);
    assert_eq!(CURRENT_FORMAT_VERSION, 69);
    let d = open_unchanged(&bytes);
    let ops = d.operations(d.manufacturings()[0].id).unwrap();
    assert_eq!(
        ops[0].options,
        Some(
            json!({"lead_in": "arc", "lead_in_mm": 2.0, "lead_out": "line",
                    "lead_out_mm": 1.0, "overcut_mm": 1.5})
        )
    );
    let own = d.operation_records(ops[0].id, ItemRecord::PathOwn).unwrap();
    assert_eq!(own.len(), 1);
    let entries = d.operation_records(ops[0].id, ItemRecord::Entry).unwrap();
    assert_eq!(entries.len(), 3, "the item, the line and the group");
    let line = entries
        .values()
        .find(|v| v["0"].get("direction").is_some())
        .unwrap();
    assert_eq!(line, &json!({"0": {"direction": "reversed"}}));
    let cut_out = d.operation_records(ops[1].id, ItemRecord::Entry).unwrap();
    assert_eq!(
        cut_out.values().next(),
        Some(&json!({"0": {"start": 500_000_000}}))
    );
}

#[test]
fn every_damaged_v68_record_is_kept_and_saved_back_unchanged() {
    let bytes = read_or_write("laser_path_options_damaged_v68.curvyo", || {
        pack(&damaged_document_v68(), "0.1.0").unwrap()
    });
    assert_eq!(format_of(&bytes), 68);
    let d = open_unchanged(&bytes);
    let ops = d.operations(d.manufacturings()[0].id).unwrap();
    assert_eq!(ops.len(), 8);
    for (op, (family, value)) in ops.iter().zip(damaged_records_v68()) {
        let stored = d.operation_records(op.id, family).unwrap();
        assert_eq!(stored.values().next(), Some(&value), "{}", op.name);
    }
}
