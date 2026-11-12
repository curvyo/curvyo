//! Format 43 (`specs/0028-material-test-library` criterion 28, ADR 0054 §1, §4 to §6): the goldens
//! `laser_settings_v43.curvyo`, `laser_settings_damaged_v43.curvyo`,
//! `test_grid_hand_edited_v43.curvyo` and `test_grid_orphan_row_v43.curvyo` read back every
//! settings register, material link, override, marker, grid, mark and row as plain values and save
//! back unchanged; files of format 42 and older open with no settings and no grid; a format 44
//! file is refused. The laser extension's reading of the same goldens is in
//! `curvyo-laser-core/tests/settings_goldens.rs`. `CURVYO_WRITE_FIXTURES=1` rewrites the goldens.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::cast_precision_loss)]

use std::collections::BTreeMap;
use std::io::{Cursor, Read, Write};
use std::path::PathBuf;

use curvyo_document_core::{
    Affine, CURRENT_FORMAT_VERSION, CornerRadii, Document, DocumentSize, DrawingText, FontKind,
    FontRef, GridItems, Length, MISSING_FACE_HASH, MaterialStep, NewGridCell, NewGridLabel,
    NewOperation, NewOutputPage, NewTestGrid, NodeId, OpenError, OperationSnapshot, Point,
    RectBounds, SettingsStep, SettingsWrite, Shape, Style, TextChars, TextLayoutValues, Vec2, pack,
    unpack,
};
use loro::{LoroDoc, LoroValue, TreeParentId};
use serde_json::{Map, Value, json};

/// The material of the goldens.
pub const MATERIAL_ID: &str = "0b6c3f1e-2a4d-4b8e-9c7f-1d2e3f4a5b6c";
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

fn laser(d: &Document) -> NodeId {
    d.add_output_page(NewOutputPage {
        process: "curvyo.laser",
        label: "Laser",
        profile: None,
        work_area: DocumentSize::from_mm(600.0, 400.0),
    })
    .unwrap()
    .kind
    .manufacturing()
    .unwrap()
}

fn map(value: Value) -> Map<String, Value> {
    let Value::Object(map) = value else {
        panic!("a map")
    };
    map
}

fn op(d: &Document, mfg: NodeId, kind: &str, name: &str) -> NodeId {
    d.add_operation(mfg, &NewOperation { kind, name }, 99, &[])
        .unwrap()
}

fn set(d: &Document, operation: NodeId, value: Value) {
    let write = [SettingsWrite {
        operation,
        value: Some(map(value)),
    }];
    assert!(
        d.write_operation_settings(SettingsStep::EditSettings, &write)
            .unwrap()
    );
}

fn link() -> Value {
    json!({"id": MATERIAL_ID, "name": "Birch 3 mm", "modified": MODIFIED})
}

/// The recipe of a 3 x 2 Cut grid with labels on a machine of 6000 mm/min.
fn recipe() -> Map<String, Value> {
    map(json!({
        "kind": "cut", "columns": 3, "rows": 2,
        "power_from_pct": 20.0, "power_to_pct": 100.0,
        "speed_from_mm_min": 600, "speed_to_mm_min": 6000,
        "cell_mm": 10.0, "gap_mm": 3.0,
        "labels": {"on": true, "kind": "engrave", "height_mm": 3.0, "power_pct": 20.0,
                   "speed_mm_min": 3000, "passes": 1},
    }))
}

fn text(string: &str, x: f64, y: f64) -> DrawingText {
    DrawingText {
        string: string.to_owned(),
        font: FontRef {
            family: "Sans".into(),
            style: "Regular".into(),
            face_hash: MISSING_FACE_HASH.to_owned(),
            face_index: 0,
            kind: FontKind::Outline,
        },
        size: Length::from_mm(3.0),
        layout: TextLayoutValues::DEFAULT,
        chars: TextChars::new([]),
        placement: Affine::translation(Vec2::new(x, y)),
        style: Style::default(),
    }
}

/// The items of [`recipe`] laid out with a speed lane of 8 mm and a unit line of 30 mm, as the
/// laser extension's layout gives them (cells from 9, 8 at a pitch of 13 mm).
fn grid_items() -> GridItems {
    let (powers, speeds) = ([20.0, 60.0, 100.0], [600, 6000]);
    let mut cells = Vec::new();
    for (row, speed) in speeds.iter().enumerate() {
        for (column, power) in powers.iter().enumerate() {
            let (x, y) = (9.0 + 13.0 * column as f64, 8.0 + 13.0 * row as f64);
            cells.push(NewGridCell {
                shape: Shape::Rect {
                    bounds: RectBounds::from_corners(
                        Point::new(x, y),
                        Point::new(x + 10.0, y + 10.0),
                    ),
                    corner_radii: CornerRadii::uniform(Length::from_mm(0.0)),
                },
                style: Style::default(),
                tag: map(json!({"column": column, "row": row, "power_pct": power,
                                "speed_mm_min": speed})),
            });
        }
    }
    let label = |string: &str, x: f64, y: f64, tag: &str| NewGridLabel {
        text: text(string, x, y),
        tag: tag.to_owned(),
    };
    let mut labels = vec![label("% power / mm/min", 0.0, 3.0, "unit")];
    for (column, power) in ["20", "60", "100"].iter().enumerate() {
        labels.push(label(power, 9.0 + 13.0 * column as f64, 7.0, "power"));
    }
    for (row, speed) in ["600", "6000"].iter().enumerate() {
        labels.push(label(speed, 0.0, 14.5 + 13.0 * row as f64, "speed"));
    }
    GridItems { cells, labels }
}

fn add_grid(d: &Document, mfg: NodeId) -> (NodeId, NodeId) {
    let made = d
        .add_test_grid(&NewTestGrid {
            manufacturing: mfg,
            name: "Test grid 1",
            kind: "cut",
            recipe: &recipe(),
            items: grid_items(),
            row_index: 99,
        })
        .unwrap();
    (made.group, made.row)
}

/// ADR 0054 §6: an Engrave + Outline linked with a power override on the hatch, a linked Cut, an
/// own-numbers Score, an operation with no settings, a `material_edit` marker, and a 3 x 2 Cut grid
/// with labels, a mark and its row.
fn golden_document() -> Document {
    let d = Document::new(1);
    let mfg = laser(&d);
    let engrave = op(&d, mfg, "engrave_outline", "Engrave + Outline 1");
    let mut hatch_link = link();
    hatch_link["outline_from"] = json!("score");
    set(
        &d,
        engrave,
        json!({
            "main": {"power_pct": 30.0, "speed_mm_min": 6000, "passes": 1},
            "outline": {"power_pct": 15.0, "speed_mm_min": 9000, "passes": 1},
            "material": hatch_link,
            "overrides": {"main": {"power_pct": 35.0}},
        }),
    );
    let cut = op(&d, mfg, "cut", "Cut 1");
    set(
        &d,
        cut,
        json!({"main": {"power_pct": 60.0, "speed_mm_min": 300, "passes": 2}, "material": link()}),
    );
    let score = op(&d, mfg, "score", "Score 1");
    set(
        &d,
        score,
        json!({"main": {"power_pct": 15.0, "speed_mm_min": 9000, "passes": 1}}),
    );
    op(&d, mfg, "engrave", "Engrave 1");
    d.commit_material_edit(MaterialStep::EditMaterial, MATERIAL_ID, MODIFIED, &[])
        .unwrap();
    let (group, _) = add_grid(&d, mfg);
    d.set_test_grid_mark(group, Some(&map(json!({"column": 1, "row": 0}))))
        .unwrap();
    d
}

/// Criterion 28 and ADR 0054 §1: one Cut operation per damage.
fn damaged_document() -> Document {
    let d = Document::new(1);
    let mfg = laser(&d);
    let main = json!({"power_pct": 60.0, "speed_mm_min": 3000, "passes": 1});
    let values = [
        json!({"main": {"power_pct": 0.0, "speed_mm_min": 3000, "passes": 1}}),
        json!({"main": {"power_pct": 60.0, "speed_mm_min": 3000, "passes": 21}}),
        json!({"main": main, "material": {"name": "Birch 3 mm", "modified": MODIFIED}}),
        json!({"main": main, "material": {"id": MATERIAL_ID, "name": "Birch 3 mm"}}),
        json!({"main": main, "outline": main}),
        json!({"main": main, "overrides": {"main": {"passes": 2}}}),
        json!({"main": main, "material": link(), "overrides": {}}),
        json!({"main": main, "material": link(), "overrides": {"outline": {"passes": 2}}}),
    ];
    for (n, value) in values.into_iter().enumerate() {
        let id = op(&d, mfg, "cut", &format!("Cut {}", n + 2));
        set(&d, id, value);
    }
    // "Cut 1", first in the list, gets a register that is not a map in `damaged_bytes`; it is
    // added last so no two adjacent steps share a label (the legacy-history oracle of `0020`).
    let first = NewOperation {
        kind: "cut",
        name: "Cut 1",
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

/// `bytes` with its document edited raw by `edit`, without any command (a merge leftover).
fn edited(bytes: &[u8], edit: impl Fn(&loro::LoroTree)) -> Vec<u8> {
    let loro = LoroDoc::new();
    loro.import(&members(bytes)["document.loro"]).unwrap();
    edit(&loro.get_tree("paths"));
    loro.commit();
    let mut files = members(bytes);
    files.insert(
        "document.loro".to_owned(),
        loro.export(loro::ExportMode::Snapshot).unwrap(),
    );
    rezip(&files)
}

fn nodes_where(tree: &loro::LoroTree, key: &str, value: &LoroValue) -> Vec<loro::TreeID> {
    let mut stack: Vec<loro::TreeID> = tree.children(TreeParentId::Root).unwrap();
    let mut out = Vec::new();
    while let Some(node) = stack.pop() {
        let meta = tree.get_meta(node).unwrap();
        if meta.get(key).and_then(|v| v.into_value().ok()).as_ref() == Some(value) {
            out.push(node);
        }
        stack.extend(tree.children(TreeParentId::Node(node)).unwrap_or_default());
    }
    out
}

fn damaged_bytes() -> Vec<u8> {
    edited(&pack(&damaged_document(), "0.1.0").unwrap(), |tree| {
        let first = nodes_where(tree, "name", &LoroValue::from("Cut 1"));
        tree.get_meta(first[0])
            .unwrap()
            .insert("op_settings", "60 %")
            .unwrap();
    })
}

fn hand_edited_bytes() -> Vec<u8> {
    let d = Document::new(1);
    let mfg = laser(&d);
    let (group, _) = add_grid(&d, mfg);
    let grid = d.test_grid(group).unwrap();
    d.translate_objects(&[grid.cells[4].id], Vec2::new(0.5, 0.0))
        .unwrap();
    d.delete_objects(&[grid.labels[2].id]).unwrap();
    pack(&d, "0.1.0").unwrap()
}

fn orphan_row_bytes() -> Vec<u8> {
    let d = Document::new(1);
    let mfg = laser(&d);
    add_grid(&d, mfg);
    edited(&pack(&d, "0.1.0").unwrap(), |tree| {
        let named = nodes_where(tree, "name", &LoroValue::from("Test grid 1"));
        let group = named
            .into_iter()
            .find(|&n| {
                tree.get_meta(n)
                    .unwrap()
                    .get("x.curvyo.laser.test_grid")
                    .is_some()
            })
            .unwrap();
        tree.delete(group).unwrap();
    })
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
    let bytes = read_or_write("laser_settings_v43.curvyo", || {
        pack(&golden_document(), "0.1.0").unwrap()
    });
    assert_eq!(format_of(&bytes), 43);
    // Format 43 since `specs/0028-material-test-library` (ADR 0054 §6).
    assert_eq!(CURRENT_FORMAT_VERSION, 43);
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
    let bytes = read_or_write("laser_settings_damaged_v43.curvyo", damaged_bytes);
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
    let bytes = read_or_write("test_grid_hand_edited_v43.curvyo", hand_edited_bytes);
    assert_eq!(format_of(&bytes), 43);
    let d = open_unchanged(&bytes);
    let row = operations(&d).remove(0);
    let grid = d.test_grid(d.test_grid_of_row(row.id).unwrap()).unwrap();
    assert_eq!((grid.cells.len(), grid.labels.len()), (6, 5));
}

#[test]
fn a_row_whose_group_is_gone_reads_no_grid_and_is_kept() {
    let bytes = read_or_write("test_grid_orphan_row_v43.curvyo", orphan_row_bytes);
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
fn a_file_of_format_44_is_refused() {
    let bytes = read_or_write("laser_settings_v43.curvyo", || {
        pack(&golden_document(), "0.1.0").unwrap()
    });
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
            found: 44,
            supported: 43
        })
    ));
}
