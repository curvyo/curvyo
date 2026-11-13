//! Format 46 (`specs/0112-test-grid-and-materials` criteria 15, 29, ADR 0057 §1, §2, §6 to §8):
//! the goldens `test_grid_v46.curvyo`, `test_grid_damaged_v46.curvyo` and
//! `test_grid_orphan_rows_v46.curvyo` read back every recipe, Lasered register, item tag and
//! `op_grid_part` as plain values and save back unchanged; damage is kept and never repaired on
//! read; orphan rows go at the next local step; a format 47 file is refused. The laser
//! extension's reading of the same goldens is in `curvyo-laser-core/tests/test_grid_goldens.rs`.
//! `CURVYO_WRITE_FIXTURES=1` rewrites the three goldens of this file only.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeMap;
use std::io::{Cursor, Read, Write};
use std::path::PathBuf;

use curvyo_document_core::{
    Affine, CURRENT_FORMAT_VERSION, CornerRadii, DerivedPart, DerivedRow, Document, DocumentSize,
    DrawingText, FixtureParams, FontKind, FontRef, GridItems, GridStep, Length, MISSING_FACE_HASH,
    NewGridCell, NewGridLabel, NewGridObject, NewOutputPage, NodeId, OpenError, Point, RectBounds,
    SettingsStep, SettingsWrite, Shape, Style, TestGridWrite, TextChars, TextLayoutValues, Vec2,
    pack, unpack,
};
use loro::{LoroDoc, LoroValue, TreeParentId};
use serde_json::{Map, Value, json};

/// The material of the golden.
const MATERIAL_ID: &str = "0b6c3f1e-2a4d-4b8e-9c7f-1d2e3f4a5b6c";
const LASERED_AT: u64 = 1_762_948_800_000;

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

fn laser(d: &Document) -> NodeId {
    let profile: FixtureParams = map(json!({
        "id": "generic-laser", "name": "Generic laser", "origin": "top_left",
        "max_feed_mm_per_min": 6000, "max_power": 1000, "laser_mode": true,
        "motorised_z": false,
    }));
    d.add_output_page(NewOutputPage {
        process: "curvyo.laser",
        label: "Laser",
        profile: Some(&profile),
        work_area: DocumentSize::from_mm(600.0, 400.0),
    })
    .unwrap()
    .kind
    .manufacturing()
    .unwrap()
}

fn label(text: &str, tag: &str, x: f64, y: f64) -> NewGridLabel {
    NewGridLabel {
        text: DrawingText {
            string: text.to_owned(),
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
        },
        tag: tag.to_owned(),
    }
}

/// A grid as the laser extension resolves it, with plain values (ADR 0057 §1, §2).
struct Grid {
    recipe: Map<String, Value>,
    items: GridItems,
    names: Vec<String>,
    kinds: Vec<&'static str>,
    settings: Vec<Map<String, Value>>,
    options: Option<Map<String, Value>>,
    parts: Vec<DerivedPart>,
}

/// A grid of `count` x `count` cells at `origin`: power on the columns, speed on the rows.
fn grid(id: &str, kind: &'static str, count: u8, origin: (f64, f64)) -> Grid {
    let engrave = kind == "engrave";
    let mut recipe = json!({
        "id": id, "kind": kind,
        "columns": {"param": "power", "min": 20.0, "max": 100.0, "count": count},
        "rows": {"param": "speed", "min": 600, "max": 6000, "count": count},
        "fixed": if engrave { json!({"passes": 1, "interval_mm": 0.08}) } else { json!({"passes": 1}) },
        "cell_mm": 10.0,
    });
    recipe["crosshatch"] = json!(engrave);
    if !engrave {
        recipe["material"] = json!({"id": MATERIAL_ID, "name": "Birch 3 mm"});
    }
    // Whole numbers on every step of the golden counts (2 and 5): power in %, speed in mm/min.
    let step = |n: u8, min: u32, max: u32| min + (max - min) * u32::from(n) / u32::from(count - 1);
    let (x0, y0) = (origin.0 + 12.0, origin.1 + 12.0);
    let mut items = GridItems {
        cells: Vec::new(),
        labels: vec![
            label(id, "id", origin.0, origin.1),
            label("Power %", "column_title", x0, origin.1),
            label("mm/min", "row_title", origin.0, y0),
        ],
    };
    let mut grid = Grid {
        recipe: map(recipe),
        items: GridItems::default(),
        names: vec![format!("{id} labels")],
        kinds: vec!["engrave"],
        settings: vec![map(
            json!({"main": {"power_pct": 20.0, "speed_mm_min": 3000, "passes": 1}}),
        )],
        options: engrave.then(|| map(json!({"crosshatch": true}))),
        parts: vec![DerivedPart::Labels],
    };
    for n in 0..count {
        let at = 12.0 * f64::from(n);
        let power = step(n, 20, 100);
        let speed = step(n, 600, 6000);
        items.labels.push(label(
            &format!("{power}"),
            "column_value",
            x0 + at,
            y0 - 4.0,
        ));
        items
            .labels
            .push(label(&format!("{speed}"), "row_value", x0 - 10.0, y0 + at));
    }
    for row in 0..count {
        for column in 0..count {
            let (x, y) = (x0 + 12.0 * f64::from(column), y0 + 12.0 * f64::from(row));
            items.cells.push(NewGridCell {
                shape: Shape::Rect {
                    bounds: RectBounds::from_corners(
                        Point::new(x, y),
                        Point::new(x + 10.0, y + 10.0),
                    ),
                    corner_radii: CornerRadii::uniform(Length::from_mm(0.0)),
                },
                style: Style::default(),
                tag: map(json!({"column": column, "row": row})),
            });
            let (power, speed) = (step(column, 20, 100), step(row, 600, 6000));
            grid.names.push(format!("{power} % · {speed} · {id}"));
            grid.kinds.push(kind);
            let mut main =
                json!({"power_pct": f64::from(power), "speed_mm_min": speed, "passes": 1});
            if engrave {
                main["interval_mm"] = json!(0.08);
            }
            grid.settings.push(map(json!({ "main": main })));
            grid.parts.push(DerivedPart::Cell { column, row });
        }
    }
    grid.items = items;
    grid
}

fn place(d: &Document, m: NodeId, name: &str, grid: &Grid) -> NodeId {
    let rows: Vec<DerivedRow<'_>> = (0..grid.names.len())
        .map(|n| DerivedRow {
            part: grid.parts[n],
            kind: grid.kinds[n],
            name: &grid.names[n],
            settings: &grid.settings[n],
            options: if n == 0 { None } else { grid.options.as_ref() },
            at: usize::MAX,
        })
        .collect();
    d.place_test_grid(&NewGridObject {
        name,
        write: TestGridWrite {
            manufacturing: m,
            recipe: &grid.recipe,
            items: &grid.items,
            rows: &rows,
        },
        lasered: None,
        step: GridStep::Add,
    })
    .unwrap()
}

/// ADR 0057 §8: a 5 x 5 Cut grid with material, 26 rows, one hand-edited cell row; an Engrave grid
/// with Crosshatch, Lasered.
fn golden_document() -> Document {
    let d = Document::new(1);
    let m = laser(&d);
    place(
        &d,
        m,
        "Test grid K7Q2M",
        &grid("K7Q2M", "cut", 5, (10.0, 10.0)),
    );
    let edited = d.operations(m).unwrap()[7].id;
    let write = [SettingsWrite {
        operation: edited,
        value: Some(map(
            json!({"main": {"power_pct": 55.5, "speed_mm_min": 1700, "passes": 2}}),
        )),
    }];
    assert!(
        d.write_operation_settings(SettingsStep::EditSettings, &write)
            .unwrap()
    );
    let engrave = place(
        &d,
        m,
        "Test grid 8PL4D",
        &grid("8PL4D", "engrave", 2, (200.0, 10.0)),
    );
    d.mark_test_grid_lasered_for_tests(engrave, LASERED_AT)
        .unwrap();
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

/// Rewrites the packed `bytes` by `plant` over every node's name and meta map, raw.
fn planted(bytes: &[u8], plant: impl Fn(&loro::LoroTree, loro::TreeID, &str)) -> Vec<u8> {
    let loro = LoroDoc::new();
    loro.import(&members(bytes)["document.loro"]).unwrap();
    let tree = loro.get_tree("paths");
    let mut stack = tree.children(TreeParentId::Root).unwrap();
    while let Some(node) = stack.pop() {
        if tree.is_node_deleted(&node).unwrap_or(true) {
            continue;
        }
        stack.extend(tree.children(TreeParentId::Node(node)).unwrap_or_default());
        let name = match tree.get_meta(node).unwrap().get("name") {
            Some(loro::ValueOrContainer::Value(LoroValue::String(name))) => name.to_string(),
            _ => String::new(),
        };
        plant(&tree, node, &name);
    }
    loro.commit();
    let mut files = members(bytes);
    files.insert(
        "document.loro".to_owned(),
        loro.export(loro::ExportMode::Snapshot).unwrap(),
    );
    rezip(&files)
}

/// ADR 0057 §8: a bad id, both axes on power, a count of 13, a non-integer Lasered, an
/// `op_grid_part` that is text.
fn damaged_bytes() -> Vec<u8> {
    let d = Document::new(1);
    let m = laser(&d);
    let mut bad_id = grid("K7Q2M", "cut", 2, (10.0, 10.0));
    bad_id.recipe.insert("id".to_owned(), json!("k7q2"));
    place(&d, m, "Bad id", &bad_id);
    let mut both = grid("BOTH1", "cut", 2, (60.0, 10.0));
    both.recipe["rows"] = json!({"param": "power", "min": 20.0, "max": 100.0, "count": 2});
    place(&d, m, "Both power", &both);
    let mut count = grid("CNT13", "cut", 2, (110.0, 10.0));
    count.recipe["columns"]["count"] = json!(13);
    place(&d, m, "Count 13", &count);
    place(
        &d,
        m,
        "Lasered 1.5",
        &grid("LSR15", "cut", 2, (160.0, 10.0)),
    );
    place(&d, m, "Text part", &grid("TXT01", "cut", 2, (210.0, 10.0)));
    planted(&pack(&d, "0.1.0").unwrap(), |tree, node, name| {
        let meta = tree.get_meta(node).unwrap();
        match name {
            "Lasered 1.5" => {
                meta.insert("x.curvyo.laser.test_grid_lasered", 1.5)
                    .unwrap();
            }
            "TXT01 labels" => {
                meta.insert("op_grid_part", "labels").unwrap();
            }
            _ => {}
        }
    })
}

/// ADR 0057 §8: the rows of a grid whose group a merge removed.
fn orphan_bytes() -> Vec<u8> {
    let d = Document::new(1);
    let m = laser(&d);
    place(&d, m, "Gone", &grid("G0NE1", "cut", 2, (10.0, 10.0)));
    place(&d, m, "Kept", &grid("KEPT1", "cut", 2, (60.0, 10.0)));
    planted(&pack(&d, "0.1.0").unwrap(), |tree, node, name| {
        if name == "Gone" {
            tree.delete(node).unwrap();
        }
    })
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

/// Every grid group of the first manufacturing's derived rows, in list order, deduplicated.
fn grids(d: &Document) -> Vec<NodeId> {
    let mut out: Vec<NodeId> = Vec::new();
    for op in d.operations(d.manufacturings()[0].id).unwrap() {
        if let Some(grid) = d.test_grid_of_derived_row(op.id)
            && !out.contains(&grid)
        {
            out.push(grid);
        }
    }
    out
}

#[test]
fn the_test_grid_golden_reads_back_every_recipe_tag_row_and_lasered_register() {
    let bytes = read_or_write("test_grid_v46.curvyo", || {
        pack(&golden_document(), "0.1.0").unwrap()
    });
    assert_eq!(format_of(&bytes), 46);
    assert_eq!(CURRENT_FORMAT_VERSION, 46);
    let d = open_unchanged(&bytes);
    let ops = d.operations(d.manufacturings()[0].id).unwrap();
    assert_eq!(ops.len(), 26 + 5);
    let [cut, engrave] = grids(&d)[..] else {
        panic!("two grids")
    };
    let cut = d.test_grid(cut).unwrap();
    assert_eq!(cut.recipe["id"], "K7Q2M");
    assert_eq!(cut.recipe["material"]["name"], "Birch 3 mm");
    assert_eq!(
        (cut.lasered, cut.cells.len(), cut.labels.len()),
        (None, 25, 13)
    );
    assert_eq!(cut.cells[6].tag, json!({"column": 1, "row": 1}));
    assert_eq!(cut.labels[0].tag, json!("id"));
    assert_eq!(
        cut.derived,
        ops[..26].iter().map(|op| op.id).collect::<Vec<_>>()
    );
    assert_eq!(ops[0].name, "K7Q2M labels");
    assert_eq!(ops[0].members.len(), 13);
    assert_eq!(ops[7].name, "40 % · 1950 · K7Q2M");
    assert_eq!(
        ops[7].settings.as_ref().unwrap()["main"],
        json!({"power_pct": 55.5, "speed_mm_min": 1700, "passes": 2}),
        "the hand-edited cell row"
    );
    assert_eq!(ops[7].grid_part.as_ref().unwrap()["part"], "cell");
    assert_eq!(
        (
            ops[7].grid_part.as_ref().unwrap()["column"].clone(),
            ops[7].grid_part.as_ref().unwrap()["row"].clone()
        ),
        (json!(1), json!(1))
    );
    let engrave = d.test_grid(engrave).unwrap();
    assert_eq!(engrave.lasered, Some(LASERED_AT));
    assert_eq!(engrave.recipe["crosshatch"], true);
    assert_eq!(ops[27].options, Some(json!({"crosshatch": true})));
    assert_eq!(
        ops[27].settings.as_ref().unwrap()["main"]["interval_mm"],
        0.08
    );
    assert!(ops.iter().all(|op| !op.grid && op.kind.is_some()));
}

#[test]
fn every_damaged_grid_opens_as_stored_and_saves_back_unchanged() {
    let bytes = read_or_write("test_grid_damaged_v46.curvyo", damaged_bytes);
    assert_eq!(format_of(&bytes), 46);
    let d = open_unchanged(&bytes);
    let ops = d.operations(d.manufacturings()[0].id).unwrap();
    assert_eq!(ops.len(), 25);
    let found = grids(&d);
    assert_eq!(
        found.len(),
        5,
        "every group keeps its recipe register, read or not"
    );
    let recipes: Vec<Value> = found
        .iter()
        .map(|g| d.test_grid(*g).unwrap().recipe)
        .collect();
    assert_eq!(recipes[0]["id"], "k7q2", "a bad id is kept, not repaired");
    assert_eq!(recipes[1]["rows"]["param"], "power");
    assert_eq!(recipes[2]["columns"]["count"], 13);
    assert_eq!(
        d.test_grid(found[3]).unwrap().lasered,
        None,
        "1.5 reads Not lasered"
    );
    let text = ops.iter().find(|op| op.name == "TXT01 labels").unwrap();
    assert_eq!(text.grid_part, Some(json!("labels")));
    assert_eq!(d.test_grid_of_derived_row(text.id), None, "an ordinary row");
    assert_eq!(d.test_grid(found[4]).unwrap().derived.len(), 4);
}

#[test]
fn orphan_rows_open_kept_and_go_at_the_next_local_step() {
    let bytes = read_or_write("test_grid_orphan_rows_v46.curvyo", orphan_bytes);
    assert_eq!(format_of(&bytes), 46);
    let d = open_unchanged(&bytes);
    let m = d.manufacturings()[0].id;
    let ops = d.operations(m).unwrap();
    assert_eq!(ops.len(), 10, "the orphans are kept on open");
    let orphans: Vec<_> = ops
        .iter()
        .filter(|op| d.test_grid_of_derived_row(op.id).is_none())
        .collect();
    assert_eq!(orphans.len(), 5);
    assert!(
        orphans
            .iter()
            .all(|op| op.grid_part.is_some() && op.members.is_empty())
    );
    let steps = d.own_step_count();
    let _ = d.create_rect(RectBounds::from_corners(
        Point::new(0.0, 0.0),
        Point::new(5.0, 5.0),
    ));
    assert_eq!(d.own_step_count(), steps + 1);
    let left = d.operations(m).unwrap();
    assert_eq!(left.len(), 5, "swept by the next local step");
    assert!(left.iter().all(|op| op.name.contains("KEPT1")));
}

#[test]
fn a_file_of_format_47_is_refused() {
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
            found: 47,
            supported: 46
        })
    ));
}
