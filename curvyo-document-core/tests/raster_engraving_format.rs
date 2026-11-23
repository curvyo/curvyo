//! Format 50 (`specs/0123-raster-engraving` criteria 12, 13, 38, 41, 44, 46, 47; ADR 0069 §7,
//! §8, §10, §13, §14): the goldens `raster_engraving_v50.curvyo` (an Image operation linked to a
//! material, with an image, a linked clone of it, a gradient-filled ellipse and every Image
//! setting; an Engrave holding the image and the ellipse; an Image test grid with Dither Ordered)
//! and `raster_engraving_damaged_v50.curvyo` (a `dither` of `"halftone_x"`, `gamma` 4.01, `dither`
//! on a Cut and on an Engrave, `crosshatch` on an Image) read back as plain values and save back
//! unchanged; an Image settings edit is one step
//! "Edit settings"; two saves are byte-identical; a file of the next format is refused. The laser
//! extension's reading of the same goldens is in
//! `curvyo-laser-core/tests/raster_engraving_goldens.rs`. `CURVYO_WRITE_FIXTURES=1` rewrites the
//! two goldens of this file only, each from the one test that owns it and only when a member other
//! than `document.loro` (its bytes differ per build) changed; every
//! other test only reads them.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeMap;
use std::io::{Cursor, Read, Write};
use std::path::PathBuf;
use std::sync::Arc;

use curvyo_document_core::{
    Affine, CURRENT_FORMAT_VERSION, Color, CornerRadii, DerivedPart, DerivedRow, Document,
    DocumentSize, EllipseFrame, FillPaint, FixtureParams, GradientFill, GridItems, GridStep,
    Length, NewGridCell, NewGridObject, NewImage, NewOperation, NewOutputPage, NodeId, OpenError,
    Point, RectBounds, SettingsStep, SettingsWrite, Shape, Style, StyleEdit, TestGridWrite, Vec2,
    pack, unpack,
};
use serde_json::{Map, Value, json};

const GOLDEN: &str = "raster_engraving_v50.curvyo";
const DAMAGED: &str = "raster_engraving_damaged_v50.curvyo";

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

/// The stored golden `name`; the tests only read it.
fn stored(name: &str) -> Vec<u8> {
    std::fs::read(fixture_path(name)).unwrap()
}

/// Under `CURVYO_WRITE_FIXTURES`, rewrites the golden `name` from `build`, called by the one test
/// that owns the file. `document.loro`'s bytes differ per build, so the file is rewritten
/// only when another member differs, and atomically, so a test reading it in parallel never
/// sees half a file. The stored golden.
fn write_if_asked(name: &str, build: impl Fn() -> Vec<u8>) -> Vec<u8> {
    if std::env::var_os("CURVYO_WRITE_FIXTURES").is_some() {
        let path = fixture_path(name);
        let fresh = build();
        let comparable = |bytes: &[u8]| {
            let mut files = members(bytes);
            files.remove("document.loro");
            files
        };
        let unchanged =
            std::fs::read(&path).is_ok_and(|old| comparable(&old) == comparable(&fresh));
        if !unchanged {
            let temp = path.with_extension(format!("curvyo.{}", std::process::id()));
            std::fs::write(&temp, &fresh).unwrap();
            std::fs::rename(&temp, &path).unwrap();
        }
    }
    stored(name)
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

/// The laser page's origin: the golden's objects stand on the laser page, so they run there.
fn laser_origin(d: &Document, mfg: NodeId) -> Point {
    d.pages()
        .into_iter()
        .find(|p| p.kind.manufacturing() == Some(mfg))
        .unwrap()
        .origin
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

fn options(d: &Document, operation: NodeId, value: &Value) {
    assert!(
        d.write_operation_options(operation, &map(value.clone()))
            .unwrap()
    );
}

fn png() -> Arc<[u8]> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../curvyo-image-core/tests/fixtures/images/png_rgb_8.png");
    Arc::from(std::fs::read(path).unwrap().as_slice())
}

/// Every Image setting away from its default, with the scan keys, as the laser extension writes
/// the whole map of an Image operation (ADR 0069 §7, §13: no Crosshatch).
fn image_options() -> Value {
    json!({
        "bidirectional": true, "scan_angle_deg": 30.0, "overscan_pct": 5,
        "z_offset_mm": 0.0,
        "dither": "grey", "threshold_pct": 60, "brightness_pct": 10, "contrast_pct": -20,
        "gamma": 1.8, "invert": true, "lowest_power_pct": 5.5,
    })
}

/// The whole map of an Engrave with its defaults (no Image key, ADR 0069 §13).
fn engrave_options() -> Value {
    json!({
        "crosshatch": false, "bidirectional": true, "scan_angle_deg": 0.0, "overscan_pct": 2,
        "z_offset_mm": 0.0,
    })
}

/// The options of an Image grid cell: the grid's Dither, everything else its default.
fn cell_options() -> Value {
    json!({
        "bidirectional": true, "scan_angle_deg": 0.0, "overscan_pct": 2,
        "z_offset_mm": 0.0,
        "dither": "ordered", "threshold_pct": 50, "brightness_pct": 0, "contrast_pct": 0,
        "gamma": 1.0, "invert": false, "lowest_power_pct": 0.0,
    })
}

fn image_recipe() -> Value {
    json!({
        "id": "R4ST3", "kind": "image",
        "columns": {"param": "interval", "min": 0.05, "max": 0.1, "count": 2},
        "rows": {"param": "power", "min": 20.0, "max": 100.0, "count": 2},
        "fixed": {"speed_mm_min": 3000, "passes": 1},
        "cell_mm": 10.0, "crosshatch": false, "dither": "ordered",
    })
}

/// The material the Image operation is linked to, and its Image set's numbers (ADR 0069 §14).
fn image_settings() -> Value {
    json!({
        "main": {"power_pct": 45.0, "speed_mm_min": 9000, "passes": 2, "interval_mm": 0.08},
        "material": {"id": "0b6c3f1e-2a4d-4b8e-9c7f-1d2e3f4a5b6c", "name": "Slate tile",
                     "modified": 1_762_948_805_000_u64},
    })
}

fn gradient_style() -> Style {
    let mut style = Style::default();
    style.fill.enabled = true;
    style.stroke.enabled = false;
    style.fill.paint = FillPaint::Gradient;
    style.fill.gradient = Some(GradientFill::first_for(Color::BLACK, 255));
    style
}

/// A 2 x 2 Image grid: Interval on the columns, Power on the rows, Dither Ordered, every cell a
/// Gradient rectangle (criterion 44), as the laser extension resolves it.
fn place_image_grid(d: &Document, mfg: NodeId) -> NodeId {
    let recipe = map(image_recipe());
    let mut items = GridItems::default();
    let mut names = vec!["R4ST3 labels".to_owned()];
    let mut parts = vec![DerivedPart::Labels];
    let mut set = vec![map(
        json!({"main": {"power_pct": 20.0, "speed_mm_min": 3000, "passes": 1, "interval_mm": 0.1}}),
    )];
    let o = laser_origin(d, mfg);
    for row in 0..2_u8 {
        for column in 0..2_u8 {
            let (x, y) = (
                o.x + 20.0 + 12.0 * f64::from(column),
                o.y + 20.0 + 12.0 * f64::from(row),
            );
            items.cells.push(NewGridCell {
                shape: Shape::Rect {
                    bounds: RectBounds::from_corners(
                        Point::new(x, y),
                        Point::new(x + 10.0, y + 10.0),
                    ),
                    corner_radii: CornerRadii::uniform(Length::from_mm(0.0)),
                },
                style: gradient_style(),
                tag: map(json!({"column": column, "row": row})),
                image: None,
            });
            let interval = [0.05, 0.1][usize::from(column)];
            let power = [20.0, 100.0][usize::from(row)];
            names.push(format!("{interval:.2} · {power} % · R4ST3"));
            parts.push(DerivedPart::Cell { column, row });
            set.push(map(json!({"main": {
                "power_pct": power, "speed_mm_min": 3000, "passes": 1, "interval_mm": interval,
            }})));
        }
    }
    let cell_options = map(cell_options());
    let rows: Vec<DerivedRow<'_>> = (0..names.len())
        .map(|n| DerivedRow {
            part: parts[n],
            kind: if n == 0 { "engrave" } else { "image" },
            name: &names[n],
            settings: &set[n],
            options: (n > 0).then_some(&cell_options),
            at: usize::MAX,
        })
        .collect();
    d.place_test_grid(&NewGridObject {
        name: "Test grid R4ST3",
        write: TestGridWrite {
            manufacturing: mfg,
            recipe: &recipe,
            items: &items,
            rows: &rows,
        },
        lasered: None,
        step: GridStep::Add,
    })
    .unwrap()
}

struct Golden {
    document: Document,
    image: NodeId,
    clone: NodeId,
    ellipse: NodeId,
}

/// The golden: an image, a linked clone of it and a gradient-filled ellipse in an Image
/// operation linked to a material, with every Image setting; the image and the ellipse in an
/// Engrave (the frame and the ellipse hatched); an Image test grid. No two adjacent steps share a
/// label (`specs/0020` criterion 36).
fn golden() -> Golden {
    let d = Document::new(1);
    let mfg = laser(&d);
    let o = laser_origin(&d, mfg);
    let image = d
        .add_images(&[NewImage {
            bytes: png(),
            placement: Affine::new(40.0, 0.0, 0.0, 30.0, o.x + 100.0, o.y + 20.0),
            name: "Photo 1".to_owned(),
        }])
        .unwrap()[0];
    let clone = d.linked_clone(&[image], &[]).unwrap()[0];
    d.translate_objects(&[clone], Vec2::new(0.0, 40.0)).unwrap();
    let ellipse = d.create_ellipse(EllipseFrame {
        center: Point::new(o.x + 200.0, o.y + 60.0),
        rx: Length::from_mm(30.0),
        ry: Length::from_mm(20.0),
    });
    d.edit_style(&[ellipse], &StyleEdit::FillPaint(FillPaint::Gradient))
        .unwrap();
    let raster = op(&d, mfg, "image", "Image 1");
    settings(&d, raster, image_settings());
    assert!(
        d.set_operation_members(raster, &[vec![image], vec![clone], vec![ellipse]], true)
            .unwrap()
    );
    options(&d, raster, &image_options());
    let engrave = op(&d, mfg, "engrave", "Engrave 1");
    settings(
        &d,
        engrave,
        json!({"main": {"power_pct": 40.0, "speed_mm_min": 6000, "passes": 1,
                        "interval_mm": 0.1}}),
    );
    assert!(
        d.set_operation_members(engrave, &[vec![image], vec![ellipse]], true)
            .unwrap()
    );
    options(&d, engrave, &engrave_options());
    place_image_grid(&d, mfg);
    Golden {
        document: d,
        image,
        clone,
        ellipse,
    }
}

fn golden_bytes() -> Vec<u8> {
    pack(&golden().document, "0.1.0").unwrap()
}

/// ADR 0069 §10: one operation per damage reason, each a whole map the file keeps as written.
fn damaged_bytes() -> Vec<u8> {
    let d = Document::new(1);
    let mfg = laser(&d);
    for (kind, name, value) in [
        ("image", "Image 1", json!({"dither": "halftone_x"})),
        ("image", "Image 2", json!({"gamma": 4.01})),
        ("cut", "Cut 1", json!({"dither": "ordered"})),
        ("engrave", "Engrave 1", json!({"dither": "ordered"})),
        ("image", "Image 3", json!({"crosshatch": false})),
    ] {
        let id = op(&d, mfg, kind, name);
        options(&d, id, &value);
    }
    pack(&d, "0.1.0").unwrap()
}

/// `document.json` with the format number of a newer build put back to 50.
fn as_format_50(json: &[u8]) -> String {
    String::from_utf8(json.to_vec()).unwrap().replacen(
        "\"format_version\": 54,",
        "\"format_version\": 50,",
        1,
    )
}

fn row(d: &Document, at: usize) -> curvyo_document_core::OperationSnapshot {
    d.operations(d.manufacturings()[0].id).unwrap()[at].clone()
}

#[test]
fn the_format_is_50() {
    // Superseded by 0135 slice 1 (2026-11-19): format 51.
    assert_eq!(CURRENT_FORMAT_VERSION, 54);
}

#[test]
fn the_golden_reads_back_every_image_setting_and_the_gradient_grid() {
    let bytes = write_if_asked(GOLDEN, golden_bytes);
    assert_eq!(format_of(&bytes), 50);
    let g = golden();
    let d = unpack(2, &bytes).unwrap();
    assert_eq!(d.own_step_count(), 0, "opening writes no step");
    let raster = row(&d, 0);
    assert_eq!(
        (raster.kind.as_deref(), raster.name.as_str()),
        (Some("image"), "Image 1")
    );
    assert_eq!(raster.options, Some(image_options()));
    assert_eq!(raster.settings, Some(image_settings()));
    assert_eq!(
        raster.members,
        vec![vec![g.image], vec![g.clone], vec![g.ellipse]]
    );
    let engrave = row(&d, 1);
    assert_eq!(engrave.kind.as_deref(), Some("engrave"));
    assert_eq!(engrave.options, Some(engrave_options()));
    assert_eq!(engrave.members, vec![vec![g.image], vec![g.ellipse]]);
    assert!(
        d.object(g.ellipse)
            .unwrap()
            .style()
            .fill
            .painted_gradient()
            .is_some()
    );
    let ops = d.operations(d.manufacturings()[0].id).unwrap();
    let grid = ops.iter().find_map(|op| d.test_grid_of_derived_row(op.id));
    let grid = d.test_grid(grid.unwrap()).unwrap();
    assert_eq!(grid.recipe, image_recipe());
    assert_eq!(grid.cells.len(), 4);
    for cell in &grid.cells {
        let style = d.object(cell.id).unwrap().style().clone();
        assert_eq!(
            style.fill.painted_gradient(),
            Some(&GradientFill::first_for(Color::BLACK, 255)),
            "every cell is a black-to-white Gradient rectangle"
        );
    }
    let cells: Vec<_> = ops
        .iter()
        .filter(|op| grid.derived.contains(&op.id) && op.name != "R4ST3 labels")
        .collect();
    assert_eq!(cells.len(), 4);
    assert!(cells.iter().all(|op| op.options == Some(cell_options())));
    assert!(cells.iter().all(|op| op.kind.as_deref() == Some("image")));
}

#[test]
fn saving_twice_gives_the_same_bytes_and_the_golden_saves_back_unchanged() {
    let bytes = stored(GOLDEN);
    let d = unpack(2, &bytes).unwrap();
    let first = pack(&d, "0.1.0").unwrap();
    let second = pack(&d, "0.1.0").unwrap();
    assert_eq!(
        first, second,
        "no pixel, dot or row is stored (criterion 38)"
    );
    // Superseded by 0135 slice 1 (2026-11-19): saved by a build of format 51, the same content under its own version.
    assert_eq!(
        as_format_50(&members(&first)["document.json"]),
        as_format_50(&members(&bytes)["document.json"])
    );
    assert_eq!(
        members(&first).keys().collect::<Vec<_>>(),
        members(&bytes).keys().collect::<Vec<_>>(),
        "the image is stored once, in assets/"
    );
}

#[test]
fn the_damaged_golden_opens_and_keeps_every_damaged_map() {
    let bytes = write_if_asked(DAMAGED, damaged_bytes);
    assert_eq!(format_of(&bytes), 50);
    let d = unpack(2, &bytes).unwrap();
    let ops = d.operations(d.manufacturings()[0].id).unwrap();
    let stored: Vec<_> = ops.iter().map(|op| op.options.clone().unwrap()).collect();
    assert_eq!(
        stored,
        [
            json!({"dither": "halftone_x"}),
            json!({"gamma": 4.01}),
            json!({"dither": "ordered"}),
            json!({"dither": "ordered"}),
            json!({"crosshatch": false})
        ]
    );
    let saved = pack(&d, "0.1.0").unwrap();
    // Superseded by 0135 slice 1 (2026-11-19): saved by a build of format 51.
    assert_eq!(
        as_format_50(&members(&saved)["document.json"]),
        as_format_50(&members(&bytes)["document.json"]),
        "never repaired on read"
    );
}

#[test]
fn an_image_settings_edit_is_one_step_edit_settings_and_undo_restores_it() {
    let d = Document::new(1);
    let mfg = laser(&d);
    let raster = op(&d, mfg, "image", "Image 1");
    let before = row(&d, 0).options;
    options(&d, raster, &image_options());
    let steps = d.own_steps();
    assert_eq!(steps.last().unwrap().label, "edit_operation_settings");
    let changed = d.own_step_count();
    d.undo_step(steps.last().unwrap().step).unwrap();
    assert_eq!(row(&d, 0).options, before);
    assert!(d.own_step_count() <= changed);
}

#[test]
fn a_file_of_the_next_format_is_refused() {
    let bytes = stored(GOLDEN);
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
            found: 55,
            supported: 54
        })
    ));
}
