//! Format 43 (`specs/0028-material-test-library` criterion 28, ADR 0054 §1, §4 to §6): the goldens
//! `laser_settings_v43.curvyo`, `laser_settings_damaged_v43.curvyo`,
//! `test_grid_hand_edited_v43.curvyo` and `test_grid_orphan_row_v43.curvyo` read back every
//! settings register, material link, override, marker, grid, mark and row as plain values and save
//! back unchanged; files of format 42 and older open with no settings and no grid; a file of the
//! next format is refused. The laser extension's reading of the same goldens is in
//! `curvyo-laser-core/tests/settings_goldens.rs`. The goldens are frozen.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::cast_precision_loss)]

use std::collections::BTreeMap;
use std::io::{Cursor, Read, Write};
use std::path::PathBuf;

use curvyo_document_core::{
    CURRENT_FORMAT_VERSION, Document, OpenError, OperationSnapshot, pack, unpack,
};
use loro::{LoroDoc, LoroValue, TreeParentId};
use serde_json::{Map, Value, json};

/// The stored recipe of the golden's 3 x 2 Cut grid with labels (format 43).
fn recipe() -> Map<String, Value> {
    let Value::Object(map) = json!({
        "kind": "cut", "columns": 3, "rows": 2,
        "power_from_pct": 20.0, "power_to_pct": 100.0,
        "speed_from_mm_min": 600, "speed_to_mm_min": 6000,
        "cell_mm": 10.0, "gap_mm": 3.0,
        "labels": {"on": true, "kind": "engrave", "height_mm": 3.0, "power_pct": 20.0,
                   "speed_mm_min": 3000, "passes": 1},
    }) else {
        panic!("a map")
    };
    map
}

/// The material of the goldens.
pub const MATERIAL_ID: &str = "0b6c3f1e-2a4d-4b8e-9c7f-1d2e3f4a5b6c";
const MODIFIED: u64 = 1_762_862_400_000;

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

/// The frozen golden `name`: a file of format 43 is never rewritten by a newer build (the
/// commands that made it went with `specs/0112` Part C).
fn golden(name: &str) -> Vec<u8> {
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

fn format_of(bytes: &[u8]) -> u64 {
    let json: Value = serde_json::from_slice(&members(bytes)["manifest.json"]).unwrap();
    json["format_version"].as_u64().unwrap()
}

/// The registers of every live node of the document, sorted, as stored, and the root map.
fn raw(bytes: &[u8]) -> (Vec<BTreeMap<String, LoroValue>>, String) {
    let loro = LoroDoc::new();
    loro.import(&members(bytes)["document.loro"]).unwrap();
    let tree = loro.get_tree("paths");
    let mut out = Vec::new();
    let mut stack: Vec<loro::TreeID> = tree.children(TreeParentId::Root).unwrap();
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
    let root = format!("{:?}", loro.get_map("root").get_deep_value());
    (out, root)
}

fn open_unchanged(bytes: &[u8]) -> Document {
    let opened = unpack(2, bytes).unwrap();
    assert_eq!(opened.own_step_count(), 0, "opening writes no step");
    let saved = pack(&opened, "0.1.0").unwrap();
    assert_eq!(raw(&saved), raw(bytes), "saved back unchanged");
    opened
}

fn operations(d: &Document) -> Vec<OperationSnapshot> {
    d.operations(d.manufacturings()[0].id).unwrap()
}

#[test]
fn the_settings_golden_reads_back_every_register_link_override_and_grid() {
    let bytes = golden("laser_settings_v43.curvyo");
    assert_eq!(format_of(&bytes), 43);
    // Format 44 since `specs/0111-laser-operation-tools` (ADR 0056 §8).
    // Format 46 since `specs/0112-test-grid-and-materials` (ADR 0057 §7).
    assert_eq!(CURRENT_FORMAT_VERSION, 46);
    let d = open_unchanged(&bytes);
    let ops = operations(&d);
    assert_eq!(ops.len(), 5, "four operations and the grid row");
    let settings = |at: usize| ops[at].settings.clone();
    assert_eq!(
        settings(0).unwrap()["overrides"],
        json!({"main": {"power_pct": 35.0}})
    );
    assert_eq!(settings(0).unwrap()["material"]["outline_from"], "score");
    assert_eq!(
        settings(1).unwrap()["material"]["modified"],
        json!(MODIFIED)
    );
    assert!(
        settings(2).unwrap().get("material").is_none(),
        "own numbers"
    );
    assert_eq!(settings(3), None, "no settings");
    assert!(ops[..4].iter().all(|o| !o.grid && o.kind.is_some()));
    let row = &ops[4];
    assert!(row.grid && row.settings.is_none());
    assert_eq!(
        d.material_edit_marker(),
        Some(json!({"id": MATERIAL_ID, "modified": MODIFIED}))
    );
    let group = d.test_grid_of_row(row.id).unwrap();
    let grid = d.test_grid(group).unwrap();
    assert_eq!(grid.recipe, Value::Object(recipe()));
    assert_eq!(grid.worked, Some(json!({"column": 1, "row": 0})));
    assert_eq!((grid.cells.len(), grid.labels.len()), (6, 6));
    assert_eq!(grid.cells[4].tag["power_pct"], json!(60.0));
    assert_eq!(grid.rows, [row.id]);
}

#[test]
fn every_damaged_register_is_kept_with_a_readable_kind_and_saved_back_unchanged() {
    let bytes = golden("laser_settings_damaged_v43.curvyo");
    assert_eq!(format_of(&bytes), 43);
    let d = open_unchanged(&bytes);
    let ops = operations(&d);
    assert_eq!(ops.len(), 9);
    assert!(
        ops.iter().all(|o| o.kind.as_deref() == Some("cut")),
        "Damaged settings is not an Unknown operation"
    );
    assert_eq!(ops[0].settings, Some(json!("60 %")));
    assert!(ops.iter().all(|o| o.settings.is_some()));
}

#[test]
fn a_hand_edited_grid_keeps_its_moved_cell_and_its_missing_label() {
    let bytes = golden("test_grid_hand_edited_v43.curvyo");
    assert_eq!(format_of(&bytes), 43);
    let d = open_unchanged(&bytes);
    let row = operations(&d).remove(0);
    let grid = d.test_grid(d.test_grid_of_row(row.id).unwrap()).unwrap();
    assert_eq!((grid.cells.len(), grid.labels.len()), (6, 5));
}

#[test]
fn a_row_whose_group_is_gone_reads_no_grid_and_is_kept() {
    let bytes = golden("test_grid_orphan_row_v43.curvyo");
    assert_eq!(format_of(&bytes), 43);
    let d = open_unchanged(&bytes);
    let rows = operations(&d);
    assert_eq!(rows.len(), 1);
    assert!(rows[0].grid);
    assert!(rows[0].members.is_empty(), "the key's path is not live");
    assert_eq!(d.test_grid_of_row(rows[0].id), None, "no grid");
}

#[test]
fn files_of_format_42_and_older_open_with_no_settings_and_no_grid() {
    for name in [
        "pen_modes_v42.curvyo",
        "fixture_jobs_v41.curvyo",
        "operations_v39.curvyo",
        "operations_unknown_kind_v39.curvyo",
    ] {
        let bytes = std::fs::read(fixture_path(name)).unwrap();
        let opened = unpack(2, &bytes).unwrap();
        assert_eq!(opened.own_step_count(), 0, "{name}");
        for manufacturing in opened.manufacturings() {
            for operation in opened.operations(manufacturing.id).unwrap() {
                assert_eq!(operation.settings, None, "{name}");
                assert!(!operation.grid, "{name}");
            }
        }
        assert_eq!(opened.material_edit_marker(), None, "{name}");
    }
}

#[test]
fn a_file_of_a_newer_format_is_refused() {
    let bytes = golden("laser_settings_v43.curvyo");
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
            found: 47,
            supported: 46
        })
    ));
}
