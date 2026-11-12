//! Format 44 (`specs/0111-laser-operation-tools` criterion 25, ADR 0056 §1, §5 to §9): the goldens
//! `operation_tools_v44.curvyo` and `operation_options_damaged_v44.curvyo` read back every
//! `op_options` map, `interval_mm`, profile copy `motorised_z` and group key as plain values and
//! save back unchanged; format-43 files open with no options and are not rewritten; the
//! clone-member keys of `operations_v39.curvyo` and `operations_clone_member_v39.curvyo` are kept;
//! a format 45 file is refused. The laser extension's reading of the same goldens is in
//! `curvyo-laser-core/tests/operation_tools_goldens.rs`. `CURVYO_WRITE_FIXTURES=1` rewrites the
//! two goldens of this file only.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeMap;
use std::io::{Cursor, Read, Write};
use std::path::PathBuf;

use curvyo_document_core::{
    CURRENT_FORMAT_VERSION, Document, DocumentSize, FixtureParams, NewOperation, NewOutputPage,
    NodeId, OpenError, OperationSnapshot, Point, RectBounds, SettingsStep, SettingsWrite, pack,
    unpack,
};
use loro::{LoroDoc, LoroValue, TreeParentId};
use serde_json::{Map, Value, json};

/// The material of the golden.
const MATERIAL_ID: &str = "0b6c3f1e-2a4d-4b8e-9c7f-1d2e3f4a5b6c";
const MODIFIED: u64 = 1_762_862_400_000;

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn read_or_write(name: &str, build: impl Fn() -> Vec<u8>) -> Vec<u8> {
    if std::env::var_os("CURVYO_WRITE_FIXTURES").is_some() {
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

/// A profile copy as the laser extension writes it (ADR 0039 §1, ADR 0056 §7).
fn copy(name: &str, motorised_z: bool) -> FixtureParams {
    map(json!({
        "id": "generic-laser", "name": name, "origin": "top_left",
        "max_feed_mm_per_min": 6000, "max_power": 1000, "laser_mode": true,
        "motorised_z": motorised_z,
    }))
}

fn laser(d: &Document, profile: &FixtureParams) -> NodeId {
    d.add_output_page(NewOutputPage {
        process: "curvyo.laser",
        label: "Laser",
        profile: Some(profile),
        work_area: DocumentSize::from_mm(600.0, 400.0),
    })
    .unwrap()
    .kind
    .manufacturing()
    .unwrap()
}

fn op(d: &Document, mfg: NodeId, kind: &str, name: &str) -> NodeId {
    d.add_operation(mfg, &NewOperation { kind, name }, 99, &[])
        .unwrap()
}

fn settings(d: &Document, operation: NodeId, value: Value) {
    let write = [SettingsWrite {
        operation,
        value: Some(map(value)),
    }];
    assert!(
        d.write_operation_settings(SettingsStep::EditSettings, &write)
            .unwrap()
    );
}

fn options(d: &Document, operation: NodeId, value: Value) {
    assert!(d.write_operation_options(operation, &map(value)).unwrap());
}

fn rect(d: &Document, x: f64) -> NodeId {
    d.create_rect(RectBounds::from_corners(
        Point::new(x, 10.0),
        Point::new(x + 20.0, 30.0),
    ))
}

/// ADR 0056 §9: an Engrave with an interval and every option set, an Engrave + Outline linked to
/// a material with its own interval, a Cut with a Z offset under a copy with `motorised_z` false,
/// a group assigned by decision 1 with one member holding its own key in the Engrave, and a second
/// manufacturing whose copy has `motorised_z` true. No two adjacent steps share a label (the
/// legacy-history oracle of `0020`).
fn golden_document() -> Document {
    let d = Document::new(1);
    let mfg = laser(&d, &copy("Generic laser", false));
    let a = rect(&d, 0.0);
    let engrave = op(&d, mfg, "engrave", "Engrave 1");
    let b = rect(&d, 40.0);
    let group = d.group_objects(&[a, b]).unwrap();
    settings(
        &d,
        engrave,
        json!({"main": {"power_pct": 30.0, "speed_mm_min": 6000, "passes": 1,
                        "interval_mm": 0.08}}),
    );
    assert!(d.set_operation_members(engrave, &[vec![b]], true).unwrap());
    options(
        &d,
        engrave,
        json!({"crosshatch": true, "bidirectional": false, "scan_angle_deg": 15.0,
               "overscan_pct": 10, "z_offset_mm": -1.5}),
    );
    let outline = op(&d, mfg, "engrave_outline", "Engrave + Outline 1");
    settings(
        &d,
        outline,
        json!({
            "main": {"power_pct": 25.0, "speed_mm_min": 6000, "passes": 1, "interval_mm": 0.12},
            "outline": {"power_pct": 15.0, "speed_mm_min": 9000, "passes": 1},
            "material": {"id": MATERIAL_ID, "name": "Birch 3 mm", "outline_from": "engrave",
                         "modified": MODIFIED},
        }),
    );
    let cut = op(&d, mfg, "cut", "Cut 1");
    options(&d, cut, json!({"z_offset_mm": 1.5}));
    // The group key and the key of B, which already holds one (ADR 0056 §1).
    assert!(
        d.set_operation_members(cut, &[vec![group], vec![b]], true)
            .unwrap()
    );
    let second = laser(&d, &copy("Z laser", true));
    let c = rect(&d, 80.0);
    let cut_2 = op(&d, second, "cut", "Cut 1");
    assert!(d.set_operation_members(cut_2, &[vec![c]], true).unwrap());
    options(&d, cut_2, json!({"z_offset_mm": 2.0}));
    d
}

/// The values of the damaged golden, one operation each (ADR 0056 §9), after "Engrave 1" whose
/// `op_options` is no map (planted raw in [`damaged_bytes`]).
fn damaged_values() -> Vec<(&'static str, &'static str, Value)> {
    let main = json!({"power_pct": 30.0, "speed_mm_min": 6000, "passes": 1});
    let mut interval_cut = main.clone();
    interval_cut["interval_mm"] = json!(0.1);
    let mut interval_small = main;
    interval_small["interval_mm"] = json!(0.005);
    vec![
        ("engrave", "options", json!({"dither": true})),
        ("engrave", "options", json!({"scan_angle_deg": 90.1})),
        ("engrave", "options", json!({"overscan_pct": 51})),
        ("cut", "options", json!({"crosshatch": true})),
        ("cut", "settings", json!({"main": interval_cut})),
        ("engrave", "settings", json!({"main": interval_small})),
    ]
}

fn damaged_document() -> Document {
    let d = Document::new(1);
    let mfg = laser(&d, &copy("Generic laser", false));
    for (n, (kind, register, value)) in damaged_values().into_iter().enumerate() {
        let id = op(&d, mfg, kind, &format!("Damaged {}", n + 2));
        match register {
            "options" => options(&d, id, value),
            _ => settings(&d, id, value),
        }
    }
    let first = NewOperation {
        kind: "engrave",
        name: "Damaged 1",
    };
    d.add_operation(mfg, &first, 0, &[]).unwrap();
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

fn damaged_bytes() -> Vec<u8> {
    let bytes = pack(&damaged_document(), "0.1.0").unwrap();
    let loro = LoroDoc::new();
    loro.import(&members(&bytes)["document.loro"]).unwrap();
    let tree = loro.get_tree("paths");
    let mut stack = tree.children(TreeParentId::Root).unwrap();
    while let Some(node) = stack.pop() {
        let meta = tree.get_meta(node).unwrap();
        if meta.get("name").and_then(|v| v.into_value().ok()) == Some(LoroValue::from("Damaged 1"))
        {
            meta.insert("op_options", "on").unwrap();
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

fn operations_of(d: &Document, at: usize) -> Vec<OperationSnapshot> {
    d.operations(d.manufacturings()[at].id).unwrap()
}

#[test]
fn the_operation_tools_golden_reads_back_every_option_interval_copy_and_group_key() {
    let bytes = read_or_write("operation_tools_v44.curvyo", || {
        pack(&golden_document(), "0.1.0").unwrap()
    });
    assert_eq!(format_of(&bytes), 44);
    assert_eq!(CURRENT_FORMAT_VERSION, 44);
    let d = open_unchanged(&bytes);
    let mfgs = d.manufacturings();
    assert_eq!(mfgs.len(), 2);
    assert_eq!(
        mfgs[0].profile.as_ref().unwrap()["motorised_z"],
        json!(false)
    );
    assert_eq!(
        mfgs[1].profile.as_ref().unwrap()["motorised_z"],
        json!(true)
    );
    let ops = operations_of(&d, 0);
    assert_eq!(ops.len(), 3);
    assert_eq!(
        ops[0].settings.as_ref().unwrap()["main"]["interval_mm"],
        0.08
    );
    assert_eq!(
        ops[0].options,
        Some(
            json!({"crosshatch": true, "bidirectional": false, "scan_angle_deg": 15.0,
                    "overscan_pct": 10, "z_offset_mm": -1.5})
        )
    );
    assert_eq!(
        ops[1].settings.as_ref().unwrap()["main"]["interval_mm"],
        0.12
    );
    assert_eq!(ops[1].options, None, "absent: every default");
    assert_eq!(ops[2].options, Some(json!({"z_offset_mm": 1.5})));
    let member = &ops[0].members;
    assert_eq!(member.len(), 1, "B in the Engrave");
    assert_eq!(ops[2].members.len(), 2, "the group and B in the Cut");
    assert!(ops[2].members.contains(&member[0]));
    let second = operations_of(&d, 1);
    assert_eq!(second[0].options, Some(json!({"z_offset_mm": 2.0})));
}

#[test]
fn every_damaged_option_and_interval_is_kept_with_a_readable_kind_and_saved_back_unchanged() {
    let bytes = read_or_write("operation_options_damaged_v44.curvyo", damaged_bytes);
    assert_eq!(format_of(&bytes), 44);
    let d = open_unchanged(&bytes);
    let ops = operations_of(&d, 0);
    assert_eq!(ops.len(), 7);
    assert!(
        ops.iter().all(|o| o.kind.is_some()),
        "Damaged settings is not an Unknown operation"
    );
    assert_eq!(ops[0].options, Some(json!("on")));
    for (op, (kind, register, value)) in ops[1..].iter().zip(damaged_values()) {
        assert_eq!(op.kind.as_deref(), Some(kind));
        let stored = if register == "options" {
            &op.options
        } else {
            &op.settings
        };
        assert_eq!(stored.as_ref(), Some(&value), "{}", op.name);
    }
}

#[test]
fn files_of_format_43_and_older_open_with_no_options_and_are_not_rewritten() {
    for name in [
        "laser_settings_v43.curvyo",
        "laser_settings_damaged_v43.curvyo",
        "fixture_jobs_v41.curvyo",
        "operations_v39.curvyo",
    ] {
        let bytes = std::fs::read(fixture_path(name)).unwrap();
        let opened = open_unchanged(&bytes);
        for manufacturing in opened.manufacturings() {
            for operation in opened.operations(manufacturing.id).unwrap() {
                assert_eq!(operation.options, None, "{name}");
            }
        }
    }
}

#[test]
fn clone_member_keys_of_old_files_are_kept_and_saved_back() {
    // Frage 212 A (ADR 0056 §2): read as legacy keys, never removed on open or save.
    for name in [
        "operations_v39.curvyo",
        "operations_clone_member_v39.curvyo",
    ] {
        let bytes = std::fs::read(fixture_path(name)).unwrap();
        let opened = open_unchanged(&bytes);
        let paths = opened
            .manufacturings()
            .iter()
            .flat_map(|m| opened.operations(m.id).unwrap())
            .flat_map(|op| op.members)
            .filter(|path| path.len() > 1)
            .count();
        if name == "operations_clone_member_v39.curvyo" {
            assert_eq!(paths, 1, "{name} holds its clone-member key");
        }
    }
}

#[test]
fn a_file_of_format_45_is_refused() {
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
            found: 45,
            supported: 44
        })
    ));
}
