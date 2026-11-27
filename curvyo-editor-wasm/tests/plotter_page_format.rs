//! Format 69 (`specs/0159-cutting-plotter` criterion 17, `adrs.md` PL3): the goldens
//! `plotter_page_v69.curvyo` and `plotter_page_damaged_v69.curvyo`. The first stores a plotter
//! page's process, cutter copy, mat, grip and grid switch and its Cut and Draw operations, and
//! reads them back unchanged; the damaged one (an unknown mat kind, a pressure out of range) reads
//! as the defaults with a notice and is not rewritten; a build of format 68 refuses both.
//! `CURVYO_WRITE_FIXTURES=1` rewrites the two goldens.

#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

use std::path::PathBuf;

use curvyo_document_core::{
    CURRENT_FORMAT_VERSION, Document, FixtureAdd, NewFixture, Point, RectBounds, SettingsStep,
    SettingsWrite, pack, unpack,
};
use curvyo_editor_wasm::{Session, Tool};
use serde_json::{Value, json};

const PLOTTER: &str = "curvyo.cutter";

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

fn plotter_page(d: &Document) -> curvyo_document_core::Page {
    d.manufacturings()
        .into_iter()
        .find(|m| m.process == PLOTTER)
        .and_then(|m| m.page)
        .unwrap()
}

fn click(s: &mut Session, at: Point) {
    s.set_tool(Tool::Select);
    s.pointer_hover(at, false, false);
    s.pointer_down(at, false);
    s.pointer_up(at, false, false);
}

/// A 12 x 24 in `LightGrip` plotter page, Mat grid off, Draw 1 on one rectangle and Cut 1 with two
/// passes on another.
fn build() -> Vec<u8> {
    let mut s = Session::new(1);
    s.add_page_of_kind(PLOTTER, None).unwrap();
    let d = unpack(2, &s.pack("0.1.0").unwrap()).unwrap();
    let page = plotter_page(&d);
    let o = page.origin;
    for x in [20.0, 80.0] {
        let _ = d.create_rect(RectBounds::from_corners(
            Point::new(o.x + x, o.y + 20.0),
            Point::new(o.x + x + 40.0, o.y + 60.0),
        ));
    }
    let mut s = Session::open(3, &pack(&d, "0.1.0").unwrap()).unwrap();
    s.resize_viewport(1600.0, 1000.0);
    assert!(s.activate_page(&page.id));
    click(&mut s, Point::new(o.x + 40.0, o.y + 20.0));
    s.press_plotter_kind("draw", false).unwrap();
    click(&mut s, Point::new(o.x + 100.0, o.y + 20.0));
    let cut = s.press_plotter_kind("cut", false).unwrap();
    assert_eq!(s.edit_plotter_setting(cut, "passes", "2"), Ok(true));
    assert!(s.set_plotter_mat("12x24"));
    assert!(s.set_plotter_grip("light"));
    assert!(s.set_plotter_mat_grid(false));
    s.pack("0.1.0").unwrap()
}

/// The golden with an unknown mat kind active and Cut 1's pressure at 250 %.
fn build_damaged() -> Vec<u8> {
    let d = unpack(4, &read_or_write("plotter_page_v69.curvyo", build)).unwrap();
    let m = d
        .manufacturings()
        .into_iter()
        .find(|m| m.process == PLOTTER)
        .unwrap();
    for fixture in &m.fixtures {
        d.set_fixture_active(fixture.id, false).unwrap();
    }
    let params = serde_json::Map::new();
    d.add_fixture(
        m.id,
        &NewFixture {
            add: FixtureAdd::Holder,
            kind: "curvyo.cutter/mat-9x9",
            label: "Mat",
            params: &params,
            matrix: None,
        },
    )
    .unwrap();
    let cut = d
        .operations(m.id)
        .unwrap()
        .into_iter()
        .find(|op| op.kind.as_deref() == Some("cut"))
        .unwrap();
    let Value::Object(settings) = json!({"pressure_pct": 250, "speed_mm_s": 200, "passes": 2})
    else {
        unreachable!()
    };
    d.write_operation_settings(
        SettingsStep::EditSettings,
        &[SettingsWrite {
            operation: cut.id,
            value: Some(settings),
        }],
    )
    .unwrap();
    pack(&d, "0.1.0").unwrap()
}

fn opened(bytes: &[u8]) -> (Session, Value) {
    let mut s = Session::open(5, bytes).unwrap();
    let page = plotter_page(&unpack(6, bytes).unwrap());
    assert!(s.activate_page(&page.id));
    let view: Value = serde_json::from_str(&s.plotter_view_json()).unwrap();
    (s, view)
}

#[test]
fn the_golden_reads_back_its_page_mat_and_operations() {
    assert_eq!(CURRENT_FORMAT_VERSION, 71);
    let bytes = read_or_write("plotter_page_v69.curvyo", build);
    let (s, view) = opened(&bytes);
    assert_eq!(view["profile"]["name"], "Cricut Maker 3");
    assert_eq!(view["mat"]["size"], "12x24");
    assert_eq!(view["mat"]["grip"], "light");
    assert_eq!(view["mat"]["grid"], false);
    let ops: Vec<(&str, &str)> = view["operations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|op| (op["name"].as_str().unwrap(), op["line2"].as_str().unwrap()))
        .collect();
    assert_eq!(
        ops,
        [("Draw 1", "Draw · 1 item"), ("Cut 1", "Cut · 1 item")]
    );
    assert_eq!(view["operations"][1]["numbers"]["passes"], 2);
    assert_eq!(view["notice"], Value::Null);
    // Saved again, the stored registers are unchanged.
    let again = unpack(7, &s.pack("0.1.0").unwrap()).unwrap();
    let before = unpack(8, &bytes).unwrap();
    assert_eq!(again.manufacturings(), before.manufacturings());
}

#[test]
fn the_damaged_golden_reads_the_defaults_with_a_notice() {
    let bytes = read_or_write("plotter_page_damaged_v69.curvyo", build_damaged);
    let (s, view) = opened(&bytes);
    assert_eq!(
        view["mat"]["size"], "12x12",
        "an unknown mat reads as the default"
    );
    assert_eq!(
        view["operations"][1]["numbers"]["pressure"], 40,
        "the default pressure"
    );
    assert_eq!(
        view["notice"],
        "Some stored values could not be read and show their defaults."
    );
    let again = unpack(9, &s.pack("0.1.0").unwrap()).unwrap();
    assert_eq!(
        again.manufacturings(),
        unpack(10, &bytes).unwrap().manufacturings(),
        "nothing rewritten on read"
    );
}

#[test]
fn a_build_of_format_68_refuses_the_goldens() {
    for name in ["plotter_page_v69.curvyo", "plotter_page_damaged_v69.curvyo"] {
        let bytes = std::fs::read(fixture_path(name)).unwrap();
        let manifest = read_manifest(&bytes);
        assert_eq!(manifest["format_version"], 69, "{name}");
        assert!(
            manifest["format_version"].as_u64().unwrap() > 67,
            "newer than 67"
        );
    }
}

/// The `manifest.json` of a project file.
fn read_manifest(bytes: &[u8]) -> Value {
    let reader = std::io::Cursor::new(bytes);
    let mut zip = zip::ZipArchive::new(reader).unwrap();
    let file = zip.by_name("manifest.json").unwrap();
    serde_json::from_reader(file).unwrap()
}
