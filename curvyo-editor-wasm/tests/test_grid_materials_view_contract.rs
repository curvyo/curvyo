//! The shared Rust/TypeScript fixtures of `specs/0112-test-grid-and-materials` (`adrs.md` finding
//! 3, plan C9): each scenario is built through the public session and its `test_grid_view`,
//! `materials_view` or host write list must equal the fixture in `tests/fixtures/`; the frontend
//! decoders read the same files. `CURVYO_WRITE_FIXTURES=1` rewrites them.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;

use curvyo_document_core::{Document, DocumentSize, NewOutputPage, Point, Timestamp, pack};
use curvyo_editor_wasm::{Session, Tool};
use curvyo_ui_core::FontBlock;
use serde_json::Value;

const BIRCH_FULL: &str = "0b6c3f1e-2a4d-4b8e-9c7f-1d2e3f4a5b6c";

fn clock() -> Timestamp {
    Timestamp::from_secs(1_762_948_800)
}

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn material_golden(name: &str) -> String {
    std::fs::read_to_string(
        manifest()
            .join("../curvyo-laser-core/tests/fixtures/materials")
            .join(name),
    )
    .unwrap()
}

/// Compares `value` with the fixture `dir/name`, or writes it with `CURVYO_WRITE_FIXTURES`.
fn pin(dir: &str, name: &str, value: &Value) {
    let path = manifest().join("tests/fixtures").join(dir).join(name);
    if std::env::var_os("CURVYO_WRITE_FIXTURES").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let mut text = serde_json::to_string_pretty(value).unwrap();
        text.push('\n');
        std::fs::write(&path, text).unwrap();
    }
    let text = std::fs::read_to_string(&path).unwrap_or_else(|_| {
        panic!(
            "{} is missing; run with CURVYO_WRITE_FIXTURES=1",
            path.display()
        )
    });
    let fixture: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(&fixture, value, "{dir}/{name}");
}

fn fonts_dir() -> PathBuf {
    manifest().join("../frontend/public/fonts")
}

/// A laser page of 400 x 300 mm on Generic laser with the fonts loaded, a fixed clock and
/// `files` in the library.
fn session(files: &[(String, String)]) -> Session {
    let d = Document::new(1);
    let laser = d
        .add_output_page(NewOutputPage {
            process: "curvyo.laser",
            label: "Laser",
            profile: None,
            work_area: DocumentSize::from_mm(400.0, 300.0),
        })
        .unwrap();
    let mut s = Session::open(5, &pack(&d, "contract").unwrap()).unwrap();
    s.set_clock(clock);
    assert!(s.activate_page(&laser.id));
    let index: Value =
        serde_json::from_str(&std::fs::read_to_string(fonts_dir().join("index.json")).unwrap())
            .unwrap();
    let faces = index["faces"]
        .as_array()
        .unwrap()
        .iter()
        .map(|face| {
            (
                face["file"].as_str().unwrap().to_owned(),
                face["family"].as_str().unwrap().to_owned(),
                face["style"].as_str().unwrap().to_owned(),
                0,
            )
        })
        .collect();
    s.set_font_list(FontBlock::Included, faces, Vec::new());
    s.set_tool(Tool::TestGrid);
    let _ = s.test_grid_press(None);
    for file in s.wanted_font_files() {
        let bytes = std::fs::read(fonts_dir().join(&file)).unwrap();
        assert!(s.add_font_file(&file, &bytes));
    }
    s.set_tool(Tool::Select);
    s.load_materials(files, None);
    s
}

/// Places a grid at the bed's top left and keeps it selected.
fn place(s: &mut Session) {
    s.set_tool(Tool::TestGrid);
    s.test_grid_press(Some(Point::new(1.0, 1.0))).unwrap();
    s.set_tool(Tool::Select);
}

fn grid_view(s: &Session) -> Value {
    serde_json::from_str(&s.test_grid_view_json()).unwrap()
}

fn materials_view(s: &Session) -> Value {
    serde_json::from_str(&s.materials_view_json()).unwrap()
}

fn writes(s: &mut Session) -> Value {
    serde_json::from_str(&s.take_material_writes_json()).unwrap()
}

/// A session with "Birch 3 mm" picked by a placed grid; the material's id and the test id.
fn birch() -> (Session, String, String) {
    let mut s = session(&[]);
    let birch = s.add_material("Birch 3 mm", 3.0, &[3; 16]).unwrap();
    place(&mut s);
    s.pick_test_grid_material(Some(&birch)).unwrap();
    let id = grid_view(&s)["id"].as_str().unwrap().to_owned();
    let _ = writes(&mut s);
    (s, birch, id)
}

/// A PNG header of `width` x `height`.
fn png(width: u32, height: u32) -> Vec<u8> {
    let mut bytes = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
    bytes.extend_from_slice(&width.to_be_bytes());
    bytes.extend_from_slice(&height.to_be_bytes());
    bytes.extend_from_slice(&[8, 6, 0, 0, 0, 0, 0, 0, 0]);
    bytes
}

#[test]
fn test_grid_view_fixtures() {
    let mut s = session(&[]);
    place(&mut s);
    pin("test_grid_view", "no_material.json", &grid_view(&s));
    s.set_test_grid_kind("engrave").unwrap();
    s.set_test_grid_crosshatch(true).unwrap();
    pin("test_grid_view", "engrave_crosshatch.json", &grid_view(&s));
    assert!(s.set_test_grid_field("cell", "60").is_err());
    pin("test_grid_view", "invalid_value.json", &grid_view(&s));

    let (mut s, _, _) = birch();
    let group = serde_json::from_value(grid_view(&s)["group"].clone()).unwrap();
    s.mark_test_grid_lasered_for_tests(group, 1_762_948_800_000)
        .unwrap();
    pin("test_grid_view", "lasered.json", &grid_view(&s));

    // The tool's draft without a grid: a recipe wider than the bed names its cause.
    let mut s = session(&[]);
    s.set_tool(Tool::TestGrid);
    s.set_test_grid_field("cell", "50").unwrap();
    s.set_test_grid_field("columns_count", "12").unwrap();
    let view = grid_view(&s);
    assert!(view["fit"].as_str().unwrap().starts_with("Grid is "));
    pin("test_grid_view", "does_not_fit.json", &view);
}

#[test]
fn materials_view_fixtures() {
    let s = session(&[]);
    pin("materials_view", "empty.json", &materials_view(&s));

    let full = vec![(
        format!("{BIRCH_FULL}.json"),
        material_golden("format2_full.json"),
    )];
    let mut s = session(&full);
    s.unfold_material(Some(BIRCH_FULL));
    s.toggle_material_laser("6f1c2a4e-9b3d-4c1a-8e2f-0a1b2c3d4e5f");
    pin("materials_view", "two_lasers.json", &materials_view(&s));
    s.open_material_test(Some("7K2QX"));
    pin(
        "materials_view",
        "test_with_photo.json",
        &materials_view(&s),
    );

    let mut s = session(&[]);
    let id = s.add_material("Walnut", 4.0, &[4; 16]).unwrap();
    s.add_material_set(&id, "generic-laser", "cut", &[5; 16])
        .unwrap();
    s.edit_material_set(&id, "generic-laser", "cut", "power", 70.0, &[6; 16])
        .unwrap();
    pin("materials_view", "typed_set.json", &materials_view(&s));

    let (mut s, birch, test) = birch();
    s.unfold_material(Some(&birch));
    s.open_material_test(Some(&test));
    pin(
        "materials_view",
        "test_without_photo.json",
        &materials_view(&s),
    );

    let older = vec![(format!("{BIRCH_FULL}.json"), material_golden("full.json"))];
    let s = session(&older);
    pin("materials_view", "format1_record.json", &materials_view(&s));

    let damaged = vec![(
        format!("{BIRCH_FULL}.json"),
        material_golden("interval_on_cut.json"),
    )];
    let s = session(&damaged);
    pin("materials_view", "damaged_record.json", &materials_view(&s));
}

#[test]
fn material_writes_fixtures() {
    let (mut s, birch, test) = birch();
    s.judge_test(&birch, &test, 2, 1, &[8; 16]).unwrap();
    pin("material_writes", "judge.json", &writes(&mut s));
    s.set_test_photo(&birch, &test, &png(600, 400), false)
        .unwrap();
    pin("material_writes", "photo_write.json", &writes(&mut s));
    s.remove_test_photo(&birch, &test).unwrap();
    pin("material_writes", "photo_remove.json", &writes(&mut s));
    s.set_test_photo(&birch, &test, &png(600, 400), false)
        .unwrap();
    let _ = writes(&mut s);
    s.delete_material(&birch).unwrap();
    pin(
        "material_writes",
        "delete_material_retires.json",
        &writes(&mut s),
    );
}
