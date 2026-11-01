//! The stored shape of the page grids and the grid snapping settings (`specs/0090-grid`
//! criteria 1, 3, 10, 11 and 16, `adrs.md` decisions 1 and 2): per-page `page.<id>.grid_*`
//! registers, defaults on absence, strict validation on open, one labelled commit that writes
//! only the fields that changed, copy on Add page, the golden `grid_v30.curvyo` and one damaged
//! file per register type. `CURVYO_WRITE_FIXTURES=1` rewrites the fixtures.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]

use std::f64::consts::{FRAC_PI_2, FRAC_PI_6, TAU};
use std::io::{Cursor, Read, Write};
use std::ops::ControlFlow;
use std::path::PathBuf;

use curvyo_document_core::{
    Angle, CURRENT_FORMAT_VERSION, CURRENT_LORO_SNAPSHOT_VERSION, Document, DocumentSize,
    DocumentSnapping, GridMode, GridScale, GridSnapPoint, Length, OpenError, OriginCorner,
    PageGrid, PageGridError, Point, Vec2, pack, unpack,
};
use loro::{LoroDoc, LoroValue};

/// The version this slice takes; a rebase onto another bump renumbers it and the fixtures.
const THIS_SLICE_VERSION: u32 = 30;
const GOLDEN: &str = "grid_v30.curvyo";
const DAMAGED: [&str; 7] = [
    "grid_spacing_zero_v30.curvyo",
    "grid_subdivision_one_v30.curvyo",
    "grid_mode_unknown_v30.curvyo",
    "grid_angle_nan_v30.curvyo",
    "grid_shown_type_v30.curvyo",
    "snapping_grid_point_unknown_v30.curvyo",
    "grid_origin_unknown_v30.curvyo",
];

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn loro_of(document: &Document) -> LoroDoc {
    let loro = LoroDoc::new();
    loro.import(&document.export_loro_snapshot().unwrap())
        .unwrap();
    loro
}

fn root_value(document: &Document, key: &str) -> Option<LoroValue> {
    loro_of(document)
        .get_map("root")
        .get(key)
        .map(|value| value.get_deep_value())
}

fn labels(document: &Document) -> Vec<String> {
    let loro = loro_of(document);
    let ids: Vec<loro::ID> = loro.oplog_frontiers().iter().collect();
    let mut out: Vec<(u32, String)> = Vec::new();
    loro.travel_change_ancestors(&ids, &mut |m| {
        out.push((
            m.lamport,
            m.message
                .map(|s| s.split(';').next().unwrap_or_default().to_string())
                .unwrap_or_default(),
        ));
        ControlFlow::Continue(())
    })
    .unwrap();
    out.sort();
    out.into_iter().map(|(_, label)| label).collect()
}

fn container_from_loro(loro_bytes: &[u8]) -> Vec<u8> {
    let manifest = serde_json::json!({
        "format_version": CURRENT_FORMAT_VERSION,
        "loro_snapshot_version": CURRENT_LORO_SNAPSHOT_VERSION,
        "app_version": "tester",
    });
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
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

/// A file whose root map carries the given extra values, as a hand-made or damaged file would.
fn file_with_root(values: &[(&str, LoroValue)]) -> Vec<u8> {
    let loro = loro_of(&Document::new(1));
    loro.set_peer_id(1).unwrap();
    let root = loro.get_map("root");
    for (key, value) in values {
        root.insert(key, value.clone()).unwrap();
    }
    loro.commit();
    container_from_loro(&loro.export(loro::ExportMode::Snapshot).unwrap())
}

fn manifest_version(bytes: &[u8]) -> u32 {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut manifest = Vec::new();
    archive
        .by_name("manifest.json")
        .unwrap()
        .read_to_end(&mut manifest)
        .unwrap();
    let manifest: serde_json::Value = serde_json::from_slice(&manifest).unwrap();
    u32::try_from(manifest["format_version"].as_u64().unwrap()).unwrap()
}

fn damaged_files() -> [(&'static str, Vec<u8>); 7] {
    [
        (
            DAMAGED[0],
            file_with_root(&[("page.first.grid_spacing", LoroValue::Double(0.0))]),
        ),
        (
            DAMAGED[1],
            file_with_root(&[("page.first.grid_subdivision", LoroValue::I64(1))]),
        ),
        (
            DAMAGED[2],
            file_with_root(&[("page.first.grid_mode", LoroValue::from("elastic"))]),
        ),
        (
            DAMAGED[3],
            file_with_root(&[("page.first.grid_angle", LoroValue::Double(f64::NAN))]),
        ),
        (
            DAMAGED[4],
            file_with_root(&[("page.first.grid_shown", LoroValue::from("yes"))]),
        ),
        (
            DAMAGED[5],
            file_with_root(&[("snapping_grid_point", LoroValue::from("corner"))]),
        ),
        (
            DAMAGED[6],
            file_with_root(&[("page.first.grid_origin", LoroValue::from("centre"))]),
        ),
    ]
}

fn rotated_log() -> PageGrid {
    PageGrid {
        shown: true,
        mode: GridMode::Fixed,
        scale: GridScale::Logarithmic,
        spacing: Length::from_mm(10.0),
        subdivision: 10,
        angle: Angle::from_radians(FRAC_PI_6),
        origin: OriginCorner::BottomLeft,
    }
}

fn coarse_linear() -> PageGrid {
    PageGrid {
        shown: true,
        spacing: Length::from_mm(20.0),
        subdivision: 4,
        ..PageGrid::DEFAULT
    }
}

fn golden_document() -> Document {
    let document = Document::new(1);
    let first = document.pages()[0].id.clone();
    assert!(document.set_page_grid(&first, coarse_linear()).unwrap());
    let second = document.add_page();
    assert!(document.set_page_grid(&second.id, rotated_log()).unwrap());
    assert!(document.set_snapping(DocumentSnapping {
        grid: false,
        grid_point: GridSnapPoint::Nodes,
        ..DocumentSnapping::DEFAULT
    }));
    document
}

// ---- criterion 1: defaults and storage -----------------------------------------------

#[test]
fn a_new_document_page_has_the_default_grid_and_stores_no_register() {
    let document = Document::new(1);
    let page = &document.pages()[0];
    assert_eq!(page.grid, PageGrid::DEFAULT);
    assert!(!page.grid.shown);
    assert_eq!(page.grid.mode, GridMode::Adaptive);
    assert_eq!(page.grid.scale, GridScale::Linear);
    assert_eq!(page.grid.spacing.as_mm(), 5.0);
    assert_eq!(page.grid.subdivision, 5);
    assert_eq!(page.grid.angle.as_radians(), 0.0);
    assert_eq!(page.grid.origin, OriginCorner::TopLeft);
    let loro = loro_of(&document);
    loro.get_map("root").for_each(|key, _| {
        assert!(!key.contains("grid"), "{key}");
    });
}

#[test]
fn each_field_writes_only_its_own_register_in_one_labelled_commit() {
    let document = Document::new(1);
    let id = document.pages()[0].id.clone();
    let before = labels(&document).len();
    assert!(
        document
            .set_page_grid(
                &id,
                PageGrid {
                    shown: true,
                    ..PageGrid::DEFAULT
                }
            )
            .unwrap()
    );
    assert_eq!(
        root_value(&document, "page.first.grid_shown"),
        Some(LoroValue::Bool(true))
    );
    assert_eq!(root_value(&document, "page.first.grid_spacing"), None);
    let after = labels(&document);
    assert_eq!(after.len(), before + 1);
    assert_eq!(after.last().unwrap(), "set_page_grid");
    let all = rotated_log();
    assert!(document.set_page_grid(&id, all).unwrap());
    assert_eq!(
        labels(&document).len(),
        before + 2,
        "all fields in one commit"
    );
    assert_eq!(
        root_value(&document, "page.first.grid_mode"),
        Some(LoroValue::from("fixed"))
    );
    assert_eq!(
        root_value(&document, "page.first.grid_scale"),
        Some(LoroValue::from("log"))
    );
    assert_eq!(
        root_value(&document, "page.first.grid_subdivision"),
        Some(LoroValue::I64(10))
    );
    assert_eq!(
        root_value(&document, "page.first.grid_origin"),
        Some(LoroValue::from("bottom_left"))
    );
    assert_eq!(document.pages()[0].grid, all);
}

#[test]
fn an_unchanged_value_writes_nothing() {
    let document = Document::new(1);
    let id = document.pages()[0].id.clone();
    let version = document.version();
    assert!(!document.set_page_grid(&id, PageGrid::DEFAULT).unwrap());
    assert_eq!(document.version(), version);
}

#[test]
fn invalid_values_are_refused_and_write_nothing() {
    let document = Document::new(1);
    let id = document.pages()[0].id.clone();
    let version = document.version();
    let cases = [
        (
            PageGrid {
                spacing: Length::from_mm(0.0),
                ..PageGrid::DEFAULT
            },
            PageGridError::Spacing,
        ),
        (
            PageGrid {
                spacing: Length::from_mm(-1.0),
                ..PageGrid::DEFAULT
            },
            PageGridError::Spacing,
        ),
        (
            PageGrid {
                subdivision: 1,
                ..PageGrid::DEFAULT
            },
            PageGridError::Subdivision,
        ),
        (
            PageGrid {
                spacing: Length::from_mm(f64::NAN),
                ..PageGrid::DEFAULT
            },
            PageGridError::NotFinite,
        ),
        (
            PageGrid {
                angle: Angle::from_radians(f64::INFINITY),
                ..PageGrid::DEFAULT
            },
            PageGridError::NotFinite,
        ),
    ];
    for (grid, error) in cases {
        assert_eq!(document.set_page_grid(&id, grid), Err(error));
    }
    assert_eq!(
        document.set_page_grid("missing", PageGrid::DEFAULT),
        Err(PageGridError::NotFound)
    );
    assert_eq!(document.version(), version);
}

#[test]
fn the_angle_is_stored_modulo_a_full_turn() {
    let document = Document::new(1);
    let id = document.pages()[0].id.clone();
    let set = |radians: f64| {
        let _ = document
            .set_page_grid(
                &id,
                PageGrid {
                    angle: Angle::from_radians(radians),
                    ..PageGrid::DEFAULT
                },
            )
            .unwrap();
        document.pages()[0].grid.angle.as_radians()
    };
    assert!((set(-FRAC_PI_2) - 3.0 * FRAC_PI_2).abs() < 1e-12);
    assert_eq!(set(TAU), 0.0);
    assert!((set(TAU + FRAC_PI_6) - FRAC_PI_6).abs() < 1e-12);
}

#[test]
fn a_stored_angle_outside_a_turn_reads_normalised_and_is_not_damage() {
    let bytes = file_with_root(&[("page.first.grid_angle", LoroValue::Double(TAU + 1.0))]);
    let document = unpack(2, &bytes).unwrap();
    assert!((document.pages()[0].grid.angle.as_radians() - 1.0).abs() < 1e-12);
}

#[test]
fn the_grid_moves_with_its_page_and_survives_a_resize() {
    let document = Document::new(1);
    let page = document.add_page();
    assert!(document.set_page_grid(&page.id, rotated_log()).unwrap());
    document
        .move_page(&page.id, Vec2::new(30.0, -10.0), &[])
        .unwrap();
    document
        .resize_page(&page.id, DocumentSize::from_mm(80.0, 60.0), &[])
        .unwrap();
    let moved = document
        .pages()
        .into_iter()
        .find(|p| p.id == page.id)
        .unwrap();
    assert_eq!(moved.grid, rotated_log());
}

#[test]
fn the_origin_corner_measures_positive_into_the_page() {
    let document = Document::new(1);
    let mut page = document.add_page();
    document
        .resize_page(&page.id, DocumentSize::from_mm(100.0, 50.0), &[])
        .unwrap();
    page = document.pages()[1].clone();
    let (x, y) = (page.origin.x, page.origin.y);
    let inside = Point::new(x + 10.0, y + 20.0);
    for (corner, zero, measured) in [
        (
            OriginCorner::TopLeft,
            Point::new(x, y),
            Vec2::new(10.0, 20.0),
        ),
        (
            OriginCorner::TopRight,
            Point::new(x + 100.0, y),
            Vec2::new(90.0, 20.0),
        ),
        (
            OriginCorner::BottomLeft,
            Point::new(x, y + 50.0),
            Vec2::new(10.0, 30.0),
        ),
        (
            OriginCorner::BottomRight,
            Point::new(x + 100.0, y + 50.0),
            Vec2::new(90.0, 30.0),
        ),
    ] {
        page.grid.origin = corner;
        assert_eq!(page.zero_point(), zero, "{corner:?}");
        let m = page.measured(inside);
        assert!(
            (m.x - measured.x).abs() < 1e-9 && (m.y - measured.y).abs() < 1e-9,
            "{corner:?}: {m:?}"
        );
        let back = page.placed(m);
        assert!((back.x - inside.x).abs() < 1e-9 && (back.y - inside.y).abs() < 1e-9);
    }
}

#[test]
fn add_page_copies_the_grid_of_its_size_source_in_the_same_commit() {
    let document = Document::new(1);
    let first = document.pages()[0].id.clone();
    document.set_page_grid(&first, rotated_log()).unwrap();
    let before = labels(&document).len();
    let added = document.add_page();
    assert_eq!(added.grid, rotated_log());
    assert_eq!(document.pages()[1].grid, rotated_log());
    assert_eq!(labels(&document).len(), before + 1);
    let plain = Document::new(2).add_page();
    assert_eq!(plain.grid, PageGrid::DEFAULT);
}

#[test]
fn pages_keep_independent_grids() {
    let document = golden_document();
    let pages = document.pages();
    assert_eq!(pages[0].grid, coarse_linear());
    assert_eq!(pages[1].grid, rotated_log());
}

// ---- criteria 10 and 11: the snapping registers ----------------------------------------

#[test]
fn grid_snapping_defaults_to_on_and_centre_without_registers() {
    let document = Document::new(1);
    assert!(document.snapping().grid);
    assert_eq!(document.snapping().grid_point, GridSnapPoint::Centre);
    assert_eq!(root_value(&document, "snapping_grid"), None);
    assert_eq!(root_value(&document, "snapping_grid_point"), None);
}

#[test]
fn grid_snapping_registers_write_only_what_changed() {
    let document = Document::new(1);
    assert!(document.set_snapping(DocumentSnapping {
        grid_point: GridSnapPoint::Box,
        ..DocumentSnapping::DEFAULT
    }));
    assert_eq!(
        root_value(&document, "snapping_grid_point"),
        Some(LoroValue::from("box"))
    );
    assert_eq!(root_value(&document, "snapping_grid"), None);
    assert!(document.set_snapping(DocumentSnapping {
        grid: false,
        grid_point: GridSnapPoint::Box,
        ..DocumentSnapping::DEFAULT
    }));
    assert_eq!(
        root_value(&document, "snapping_grid"),
        Some(LoroValue::Bool(false))
    );
}

// ---- save, open, document.json ----------------------------------------------------------

#[test]
fn grids_and_snapping_survive_save_and_open() {
    let document = golden_document();
    let reopened = unpack(2, &pack(&document, "t").unwrap()).unwrap();
    assert_eq!(reopened.pages(), document.pages());
    assert_eq!(reopened.snapping(), document.snapping());
}

#[test]
fn document_json_carries_the_grids_and_the_grid_snapping() {
    let document = golden_document();
    let json: serde_json::Value = serde_json::from_slice(&document.export_json().unwrap()).unwrap();
    let grid = &json["pages"][1]["grid"];
    assert_eq!(grid["shown"], true);
    assert_eq!(grid["mode"], "fixed");
    assert_eq!(grid["scale"], "log");
    assert_eq!(grid["subdivision"], 10);
    assert_eq!(grid["origin"], "bottom_left");
    assert_eq!(json["pages"][0]["grid"]["mode"], "adaptive");
    assert_eq!(json["pages"][0]["grid"]["origin"], "top_left");
    assert_eq!(json["snapping"]["grid"], false);
    assert_eq!(json["snapping"]["grid_point"], "nodes");
}

#[test]
fn this_slice_took_its_format_version() {
    let bytes = pack(&Document::new(1), "t").unwrap();
    assert!(manifest_version(&bytes) >= THIS_SLICE_VERSION);
    assert_eq!(manifest_version(&bytes), CURRENT_FORMAT_VERSION);
}

// ---- strict validation ------------------------------------------------------------------

#[test]
fn each_malformed_register_is_refused_as_damaged() {
    for (name, bytes) in damaged_files() {
        assert!(
            matches!(unpack(2, &bytes), Err(OpenError::Damaged)),
            "{name}"
        );
    }
}

#[test]
fn malformed_registers_of_every_field_and_deleted_pages_are_damage() {
    for (key, value) in [
        ("page.first.grid_scale", LoroValue::from("linear ")),
        ("page.first.grid_spacing", LoroValue::I64(5)),
        ("page.first.grid_spacing", LoroValue::Double(f64::INFINITY)),
        ("page.first.grid_subdivision", LoroValue::Double(5.0)),
        ("page.first.grid_subdivision", LoroValue::I64(1 << 40)),
        ("page.first.grid_origin", LoroValue::from("top")),
        ("page.first.grid_origin", LoroValue::from("Top_left")),
        ("page.first.grid_origin", LoroValue::I64(0)),
        ("page.gone.grid_shown", LoroValue::Double(1.0)),
        ("snapping_grid", LoroValue::from("true")),
    ] {
        assert!(
            matches!(
                unpack(2, &file_with_root(&[(key, value.clone())])),
                Err(OpenError::Damaged)
            ),
            "{key} = {value:?}"
        );
    }
}

#[test]
fn valid_edges_open() {
    for (key, value) in [
        ("page.first.grid_spacing", LoroValue::Double(1e-6)),
        ("page.first.grid_spacing", LoroValue::Double(1e9)),
        ("page.first.grid_subdivision", LoroValue::I64(2)),
        (
            "page.first.grid_subdivision",
            LoroValue::I64(i64::from(u32::MAX)),
        ),
        ("page.first.grid_angle", LoroValue::Double(-100.0)),
        ("page.first.grid_origin", LoroValue::from("top_left")),
        ("page.first.grid_origin", LoroValue::from("top_right")),
        ("page.first.grid_origin", LoroValue::from("bottom_right")),
        ("page.first.grid_colour", LoroValue::from("unknown field")),
        ("snapping_grid_point", LoroValue::from("box")),
    ] {
        assert!(
            unpack(2, &file_with_root(&[(key, value.clone())])).is_ok(),
            "{key} = {value:?}"
        );
    }
}

// ---- golden -------------------------------------------------------------------------------

fn write_fixtures_on_request() {
    if std::env::var_os("CURVYO_WRITE_FIXTURES").is_some() {
        std::fs::write(
            fixture_path(GOLDEN),
            pack(&golden_document(), "0.1.0").unwrap(),
        )
        .unwrap();
        for (name, bytes) in damaged_files() {
            std::fs::write(fixture_path(name), bytes).unwrap();
        }
    }
}

#[test]
fn the_damaged_fixtures_are_refused_as_damaged() {
    write_fixtures_on_request();
    for name in DAMAGED {
        let bytes = std::fs::read(fixture_path(name)).unwrap();
        assert!(
            matches!(unpack(2, &bytes), Err(OpenError::Damaged)),
            "{name}"
        );
    }
}

#[test]
fn the_golden_reads_back_and_opening_it_writes_nothing() {
    write_fixtures_on_request();
    let bytes = std::fs::read(fixture_path(GOLDEN)).unwrap();
    assert_eq!(manifest_version(&bytes), THIS_SLICE_VERSION);
    let document = unpack(2, &bytes).unwrap();
    let pages = document.pages();
    assert_eq!(pages.len(), 2);
    assert_eq!(pages[0].grid, coarse_linear());
    assert_eq!(pages[1].grid, rotated_log());
    assert!(!document.snapping().grid);
    assert_eq!(document.snapping().grid_point, GridSnapPoint::Nodes);
    let mut archive = zip::ZipArchive::new(Cursor::new(&bytes)).unwrap();
    let mut stored_bytes = Vec::new();
    archive
        .by_name("document.loro")
        .unwrap()
        .read_to_end(&mut stored_bytes)
        .unwrap();
    let stored = LoroDoc::new();
    stored.import(&stored_bytes).unwrap();
    assert_eq!(loro_of(&document).oplog_vv(), stored.oplog_vv());
}

#[test]
fn an_older_file_without_grid_registers_opens_with_the_defaults_and_is_not_rewritten() {
    let bytes = std::fs::read(fixture_path("pages_v26.curvyo")).unwrap();
    let document = unpack(2, &bytes).unwrap();
    let version = document.version();
    assert!(
        document
            .pages()
            .iter()
            .all(|page| page.grid == PageGrid::DEFAULT)
    );
    assert!(document.snapping().grid);
    assert_eq!(document.version(), version);
}
