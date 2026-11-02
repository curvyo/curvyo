//! The stored shape of tilings (`specs/0069-tiling` criteria 25 and 42, ADR 0030 §10): the golden
//! `tiling_v33.curvyo` (a grid with hue variance and the source kept, a circle with the source
//! left out, a multi-source tiling with an image copy, a copies tiling), one damaged file per
//! damage rule, and a residue file that opens. `CURVYO_WRITE_FIXTURES=1` rewrites the fixtures.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::large_types_passed_by_value
)]

use std::cell::Cell;
use std::io::{Cursor, Read};
use std::path::PathBuf;
use std::sync::Arc;

use curvyo_document_core::{
    Affine, AnchorId, Angle, CURRENT_FORMAT_VERSION, CellKey, CircleOptions, Document, GridOptions,
    Length, NewAnchor, NewImage, NodeId, OffsetLines, OpenError, Percent, Point, SpacingMode,
    TilingCell, TilingLayout, TilingRecipe, TilingRequest, TilingResult, Variance, Vec2, pack,
    unpack,
};
use loro::{LoroDoc, LoroValue};

/// The version this slice takes; a rebase onto another bump renumbers it and the fixtures.
const THIS_SLICE_VERSION: u32 = 33;
const GOLDEN: &str = "tiling_v33.curvyo";
const RESIDUE: &str = "tiling_residue_v33.curvyo";
const DAMAGED: [&str; 9] = [
    "tiling_on_a_leaf_v33.curvyo",
    "tiling_layout_unknown_v33.curvyo",
    "tiling_result_unknown_v33.curvyo",
    "tiling_count_zero_v33.curvyo",
    "tiling_count_over_limit_v33.curvyo",
    "tiling_number_nan_v33.curvyo",
    "tiling_sources_malformed_v33.curvyo",
    "tiling_cell_malformed_v33.curvyo",
    "tiling_member_malformed_v33.curvyo",
];

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn path(document: &Document, x: f64, n: u64) -> NodeId {
    document.create_path(
        &[
            NewAnchor::corner(AnchorId::new(32, n), Point::new(x, 0.0)),
            NewAnchor::corner(AnchorId::new(32, n + 1), Point::new(x + 10.0, 0.0)),
            NewAnchor::corner(AnchorId::new(32, n + 2), Point::new(x + 10.0, 10.0)),
        ],
        true,
    )
}

fn recipe(layout: TilingLayout, keep_source: bool, result: TilingResult) -> TilingRecipe {
    TilingRecipe {
        layout,
        grid: GridOptions {
            columns: 3,
            rows: 2,
            spacing_x: Length::from_mm(2.0),
            spacing_y: Length::from_mm(3.0),
            spacing_mode: SpacingMode::Gap,
            mirror_rows: true,
            mirror_columns: false,
            offset: OffsetLines::Rows,
            shift: Percent::new(50.0),
        },
        circle: CircleOptions {
            count: 4,
            centre: Vec2::new(-20.0, 0.0),
            radius: Length::from_mm(20.0),
            start: Angle::from_radians(0.0),
            sweep: Angle::from_radians(std::f64::consts::TAU),
            face_centre: true,
        },
        variance: Variance::default(),
        seed: 4_000_000_000,
        result,
        keep_source,
    }
}

fn cells(recipe: &TilingRecipe, y: f64) -> Vec<TilingCell> {
    let (rows, columns) = match recipe.layout {
        TilingLayout::Grid => (recipe.grid.rows, recipe.grid.columns),
        TilingLayout::Circle => (1, recipe.circle.count),
    };
    (0..rows)
        .flat_map(|row| (0..columns).map(move |column| CellKey { row, column }))
        .filter(|key| !(recipe.keep_source && *key == CellKey::ORIGIN))
        .map(|key| TilingCell {
            key,
            affine: Affine::translation(Vec2::new(
                f64::from(key.column) * 14.0,
                f64::from(key.row).mul_add(14.0, y),
            )),
            opacity: Percent::new(-10.0),
            hue: Angle::from_radians(f64::from(key.column)),
        })
        .collect()
}

fn mint() -> impl FnMut() -> AnchorId {
    let next = Cell::new(5000_u64);
    move || {
        next.set(next.get() + 1);
        AnchorId::new(32, next.get())
    }
}

fn tile(document: &Document, sources: Vec<NodeId>, recipe: TilingRecipe, y: f64) -> NodeId {
    let request = TilingRequest {
        tiling: None,
        sources,
        cells: cells(&recipe, y),
        recipe,
        conversions: Vec::new(),
    };
    document.apply_tiling(&request, &mut mint()).unwrap().tiling
}

fn golden_document() -> Document {
    let document = Document::new(1);
    let mut hue = recipe(TilingLayout::Grid, true, TilingResult::Clones);
    hue.variance.hue.random = Angle::from_radians(0.5);
    hue.variance.opacity.per_column = Percent::new(-10.0);
    tile(&document, vec![path(&document, 0.0, 1)], hue, 0.0);
    let circle = recipe(TilingLayout::Circle, false, TilingResult::Clones);
    tile(&document, vec![path(&document, 100.0, 10)], circle, 50.0);
    let image = document
        .add_images(&[NewImage {
            bytes: Arc::from(&b"tiling golden picture"[..]),
            placement: Affine::new(10.0, 0.0, 0.0, 10.0, 200.0, 0.0),
            name: "picture.png".to_owned(),
        }])
        .unwrap()[0];
    let multi = recipe(TilingLayout::Grid, true, TilingResult::Clones);
    tile(
        &document,
        vec![path(&document, 190.0, 20), image],
        multi,
        100.0,
    );
    let copies = recipe(TilingLayout::Grid, true, TilingResult::Copies);
    tile(&document, vec![path(&document, 300.0, 30)], copies, 150.0);
    document
}

fn loro_of(document: &Document) -> LoroDoc {
    let loro = LoroDoc::new();
    loro.import(&document.export_loro_snapshot().unwrap())
        .unwrap();
    loro.set_peer_id(9).unwrap();
    loro
}

fn replace_loro(container: &[u8], loro: &[u8]) -> Vec<u8> {
    use std::io::Write;
    let mut archive = zip::ZipArchive::new(Cursor::new(container)).unwrap();
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default();
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();
        let name = file.name().unwrap().to_string();
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes).unwrap();
        writer.start_file(&name, options).unwrap();
        writer
            .write_all(if name == "document.loro" {
                loro
            } else {
                &bytes
            })
            .unwrap();
    }
    writer.finish().unwrap().into_inner()
}

/// A saved document with one multi-source tiling whose nodes `edit` changes through the raw tree.
fn edited(edit: impl Fn(&loro::LoroTree, &[loro::TreeID])) -> Vec<u8> {
    let document = Document::new(1);
    let first = path(&document, 0.0, 1);
    // A step between the two paths: no two history rows in a row carry one label.
    let _ = document.create_rect(curvyo_document_core::RectBounds {
        origin: Point::new(0.0, 40.0),
        width: Length::from_mm(5.0),
        height: Length::from_mm(5.0),
    });
    let sources = vec![first, path(&document, 20.0, 10)];
    tile(
        &document,
        sources,
        recipe(TilingLayout::Grid, true, TilingResult::Clones),
        0.0,
    );
    let loro = loro_of(&document);
    let tree = loro.get_tree("paths");
    let nodes: Vec<loro::TreeID> = tree.get_nodes(false).into_iter().map(|n| n.id).collect();
    edit(&tree, &nodes);
    loro.commit();
    let bytes = pack(&document, "t").unwrap();
    replace_loro(&bytes, &loro.export(loro::ExportMode::Snapshot).unwrap())
}

fn find(tree: &loro::LoroTree, nodes: &[loro::TreeID], key: &str) -> loro::LoroMap {
    nodes
        .iter()
        .filter_map(|n| tree.get_meta(*n).ok())
        .find(|meta| meta.get(key).is_some())
        .unwrap()
}

/// The recipe register with `field` of `section` (or of the root) replaced by `value`.
fn recipe_with(meta: &loro::LoroMap, section: Option<&str>, field: &str, value: LoroValue) {
    let LoroValue::Map(root) = meta.get("tiling").unwrap().get_deep_value() else {
        panic!("a map")
    };
    let mut root: std::collections::HashMap<String, LoroValue> =
        root.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
    match section {
        None => {
            root.insert(field.to_owned(), value);
        }
        Some(section) => {
            let LoroValue::Map(inner) = root[section].clone() else {
                panic!("a map")
            };
            let mut inner: std::collections::HashMap<String, LoroValue> =
                inner.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
            inner.insert(field.to_owned(), value);
            root.insert(section.to_owned(), LoroValue::Map(inner.into()));
        }
    }
    meta.insert("tiling", LoroValue::Map(root.into())).unwrap();
}

fn damaged_files() -> Vec<(&'static str, Vec<u8>)> {
    let recipe_edit = |section: Option<&'static str>, field: &'static str, value: LoroValue| {
        edited(move |tree, nodes| {
            recipe_with(&find(tree, nodes, "tiling"), section, field, value.clone());
        })
    };
    vec![
        (
            DAMAGED[0],
            edited(|tree, nodes| {
                let recipe = find(tree, nodes, "tiling")
                    .get("tiling")
                    .unwrap()
                    .get_deep_value();
                find(tree, nodes, "tiling_member")
                    .insert("tiling", recipe)
                    .unwrap();
            }),
        ),
        (DAMAGED[1], recipe_edit(None, "layout", "spiral".into())),
        (DAMAGED[2], recipe_edit(None, "result", "stamps".into())),
        (
            DAMAGED[3],
            recipe_edit(Some("circle"), "count", 0_i64.into()),
        ),
        (
            DAMAGED[4],
            recipe_edit(Some("grid"), "columns", 100_001_i64.into()),
        ),
        (
            DAMAGED[5],
            recipe_edit(Some("grid"), "spacing_x", f64::NAN.into()),
        ),
        (
            DAMAGED[6],
            edited(|tree, nodes| {
                find(tree, nodes, "tiling_sources")
                    .insert("tiling_sources", "not a list")
                    .unwrap();
            }),
        ),
        (
            DAMAGED[7],
            edited(|tree, nodes| {
                let cell = LoroValue::List(vec![LoroValue::from(1_i64)].into());
                find(tree, nodes, "tiling_cell")
                    .insert("tiling_cell", cell)
                    .unwrap();
            }),
        ),
        (
            DAMAGED[8],
            edited(|tree, nodes| {
                find(tree, nodes, "tiling_member")
                    .insert("tiling_member", 5_i64)
                    .unwrap();
            }),
        ),
    ]
}

/// Dangling source id, a duplicate cell key and a missing cell (merge residue, ADR 0030 §6).
fn residue_file() -> Vec<u8> {
    edited(|tree, nodes| {
        let group = find(tree, nodes, "tiling_sources");
        let mut sources = match group.get("tiling_sources").unwrap().get_deep_value() {
            LoroValue::List(list) => list.to_vec(),
            _ => panic!("a list"),
        };
        sources.push("424242:7".into());
        group.insert("tiling_sources", sources).unwrap();
        let keyed: Vec<loro::LoroMap> = nodes
            .iter()
            .filter_map(|n| tree.get_meta(*n).ok())
            .filter(|meta| meta.get("tiling_cell").is_some())
            .collect();
        let first = keyed[0].get("tiling_cell").unwrap().get_deep_value();
        keyed[1].insert("tiling_cell", first).unwrap();
        keyed[2].delete("tiling_cell").unwrap();
    })
}

#[test]
fn write_fixtures_on_request() {
    if std::env::var_os("CURVYO_WRITE_FIXTURES").is_some() {
        std::fs::write(
            fixture_path(GOLDEN),
            pack(&golden_document(), "0.1.0").unwrap(),
        )
        .unwrap();
        std::fs::write(fixture_path(RESIDUE), residue_file()).unwrap();
        for (name, bytes) in damaged_files() {
            std::fs::write(fixture_path(name), bytes).unwrap();
        }
    }
}

#[test]
fn this_slice_took_its_format_version() {
    const _: () = assert!(CURRENT_FORMAT_VERSION >= THIS_SLICE_VERSION);
    let bytes = pack(&Document::new(1), "t").unwrap();
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut manifest = Vec::new();
    archive
        .by_name("manifest.json")
        .unwrap()
        .read_to_end(&mut manifest)
        .unwrap();
    let manifest: serde_json::Value = serde_json::from_slice(&manifest).unwrap();
    assert_eq!(
        manifest["format_version"].as_u64(),
        Some(u64::from(CURRENT_FORMAT_VERSION))
    );
}

#[test]
fn every_tiling_survives_save_and_open_and_opening_writes_nothing() {
    let document = golden_document();
    let reopened = unpack(2, &pack(&document, "t").unwrap()).unwrap();
    let tilings = |d: &Document| -> Vec<TilingRecipe> {
        d.children(None)
            .into_iter()
            .flat_map(|layer| d.children(Some(layer)))
            .chain(d.children(None))
            .filter_map(|id| d.tiling(id).map(|t| t.recipe))
            .collect()
    };
    assert_eq!(tilings(&document).len(), 4);
    assert_eq!(tilings(&reopened), tilings(&document));
}

#[test]
fn the_golden_opens_with_its_four_tilings() {
    let bytes = std::fs::read(fixture_path(GOLDEN)).unwrap();
    let document = unpack(2, &bytes).unwrap();
    let version = document.version();
    let count = document
        .children(None)
        .into_iter()
        .flat_map(|layer| document.children(Some(layer)))
        .chain(document.children(None))
        .filter(|id| document.tiling(*id).is_some())
        .count();
    assert_eq!(count, 4);
    assert_eq!(document.version(), version);
}

#[test]
fn each_damage_rule_refuses_the_file_and_the_fixtures_agree() {
    for (name, bytes) in damaged_files() {
        assert!(
            matches!(unpack(2, &bytes), Err(OpenError::Damaged)),
            "{name}"
        );
        let stored = std::fs::read(fixture_path(name)).unwrap();
        assert!(
            matches!(unpack(2, &stored), Err(OpenError::Damaged)),
            "fixture {name}"
        );
    }
}

#[test]
fn the_residue_file_opens_and_keeps_the_dangling_source() {
    let stored = std::fs::read(fixture_path(RESIDUE)).unwrap();
    let document = unpack(2, &stored).unwrap();
    assert!(unpack(2, &residue_file()).is_ok());
    let group = document
        .children(None)
        .into_iter()
        .flat_map(|layer| document.children(Some(layer)))
        .chain(document.children(None))
        .find(|id| document.tiling(*id).is_some())
        .unwrap();
    assert!(
        !document.tiling(group).unwrap().sources_live,
        "the dangling id is kept"
    );
}

#[test]
fn the_next_apply_keeps_the_lowest_id_of_a_duplicate_key_and_restores_a_missing_one() {
    let bytes = edited(|tree, nodes| {
        let keyed: Vec<loro::LoroMap> = nodes
            .iter()
            .filter_map(|n| tree.get_meta(*n).ok())
            .filter(|meta| meta.get("tiling_cell").is_some())
            .collect();
        let first = keyed[0].get("tiling_cell").unwrap().get_deep_value();
        keyed[1].insert("tiling_cell", first).unwrap();
    });
    let document = unpack(2, &bytes).unwrap();
    let group = document
        .children(None)
        .into_iter()
        .flat_map(|layer| document.children(Some(layer)))
        .chain(document.children(None))
        .find(|id| document.tiling(*id).is_some())
        .unwrap();
    let stored = document.tiling(group).unwrap();
    let before = document.children(Some(group)).len();
    let request = TilingRequest {
        tiling: Some(group),
        sources: Vec::new(),
        cells: cells(&stored.recipe, 0.0),
        recipe: stored.recipe,
        conversions: Vec::new(),
    };
    let outcome = document.apply_tiling(&request, &mut mint()).unwrap();
    assert!(outcome.written);
    // Two sources plus five instances: the duplicate went, the missing cell came back.
    assert_eq!(before, 7);
    assert_eq!(document.children(Some(group)).len(), 7);
}
