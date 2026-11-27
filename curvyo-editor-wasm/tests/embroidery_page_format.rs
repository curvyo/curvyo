//! Format 70 (`specs/0160-embroidery` criterion 20, `adrs.md` EM5): the goldens
//! `embroidery_page_v70.curvyo` and `embroidery_page_damaged_v70.curvyo`. The first stores an
//! embroidery page's process, profile copy, active hoop and its Running stitch and Tatami fill
//! operations with their numbers and threads, and reads them back unchanged; the damaged one (an
//! unknown hoop kind, a row spacing out of range) reads as the defaults with a notice and is not
//! rewritten; a build of format 68 refuses both. `CURVYO_WRITE_FIXTURES=1` rewrites the goldens.

#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

use std::path::PathBuf;

use curvyo_document_core::{
    CURRENT_FORMAT_VERSION, Document, FixtureAdd, NewFixture, Point, RectBounds, SettingsStep,
    SettingsWrite, pack, unpack,
};
use curvyo_editor_wasm::{Session, Tool};
use serde_json::{Value, json};

const EMBROIDERY: &str = "curvyo.embroidery";

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

fn embroidery_page(d: &Document) -> curvyo_document_core::Page {
    d.manufacturings()
        .into_iter()
        .find(|m| m.process == EMBROIDERY)
        .and_then(|m| m.page)
        .unwrap()
}

fn click(s: &mut Session, at: Point) {
    s.set_tool(Tool::Select);
    s.pointer_hover(at, false, false);
    s.pointer_down(at, false);
    s.pointer_up(at, false, false);
}

/// The 100 x 100 mm hoop active, Running stitch 1 (triple) on one rectangle and Tatami fill 1 at
/// 45 degrees in red on another.
fn build() -> Vec<u8> {
    let mut s = Session::new(1);
    s.add_page_of_kind(EMBROIDERY, None).unwrap();
    let d = unpack(2, &s.pack("0.1.0").unwrap()).unwrap();
    let page = embroidery_page(&d);
    let o = page.origin;
    for x in [65.0, 100.0] {
        let _ = d.create_rect(RectBounds::from_corners(
            Point::new(o.x + x, o.y + 130.0),
            Point::new(o.x + x + 25.0, o.y + 160.0),
        ));
    }
    let mut s = Session::open(3, &pack(&d, "0.1.0").unwrap()).unwrap();
    s.resize_viewport(1600.0, 1000.0);
    assert!(s.activate_page(&page.id));
    click(&mut s, Point::new(o.x + 75.0, o.y + 130.0));
    let running = s.press_embroidery_kind("running", false).unwrap();
    assert_eq!(s.edit_embroidery_setting(running, "repeats", "3"), Ok(true));
    click(&mut s, Point::new(o.x + 110.0, o.y + 130.0));
    let tatami = s.press_embroidery_kind("tatami", false).unwrap();
    assert_eq!(
        s.edit_embroidery_setting(tatami, "angle_deg", "45"),
        Ok(true)
    );
    assert!(s.set_embroidery_thread(tatami, "#C8102E"));
    assert!(s.set_embroidery_hoop("100x100"));
    s.pack("0.1.0").unwrap()
}

/// The golden with an unknown hoop kind active and Tatami fill 1's row spacing at 5 mm.
fn build_damaged() -> Vec<u8> {
    let d = unpack(4, &read_or_write("embroidery_page_v70.curvyo", build)).unwrap();
    let m = d
        .manufacturings()
        .into_iter()
        .find(|m| m.process == EMBROIDERY)
        .unwrap();
    for fixture in &m.fixtures {
        d.set_fixture_active(fixture.id, false).unwrap();
    }
    let params = serde_json::Map::new();
    d.add_fixture(
        m.id,
        &NewFixture {
            add: FixtureAdd::Holder,
            kind: "curvyo.embroidery/hoop-9x9",
            label: "Hoop",
            params: &params,
            matrix: None,
        },
    )
    .unwrap();
    let tatami = d
        .operations(m.id)
        .unwrap()
        .into_iter()
        .find(|op| op.kind.as_deref() == Some("tatami"))
        .unwrap();
    let Value::Object(settings) = json!({
        "angle_deg": 45, "row_spacing_mm": 5.0, "max_stitch_mm": 3.0, "thread_colour": "#C8102E"
    }) else {
        unreachable!()
    };
    d.write_operation_settings(
        SettingsStep::EditSettings,
        &[SettingsWrite {
            operation: tatami.id,
            value: Some(settings),
        }],
    )
    .unwrap();
    pack(&d, "0.1.0").unwrap()
}

fn opened(bytes: &[u8]) -> (Session, Value) {
    let mut s = Session::open(5, bytes).unwrap();
    let page = embroidery_page(&unpack(6, bytes).unwrap());
    assert!(s.activate_page(&page.id));
    let view: Value = serde_json::from_str(&s.embroidery_view_json()).unwrap();
    (s, view)
}

#[test]
fn the_golden_reads_back_its_page_hoop_and_operations() {
    assert_eq!(CURRENT_FORMAT_VERSION, 70);
    let bytes = read_or_write("embroidery_page_v70.curvyo", build);
    let (s, view) = opened(&bytes);
    assert_eq!(view["profile"]["name"], "W6 EU7");
    assert_eq!(view["hoop"]["key"], "100x100");
    let ops: Vec<(&str, &str, &str)> = view["operations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|op| {
            (
                op["name"].as_str().unwrap(),
                op["line3"].as_str().unwrap(),
                op["thread"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        ops,
        [
            ("Running stitch 1", "2.5 mm, 3 ×", "#000000"),
            ("Tatami fill 1", "45°, 0.25 mm, 3 mm", "#C8102E")
        ]
    );
    assert_eq!(view["notice"], Value::Null);
    let again = unpack(7, &s.pack("0.1.0").unwrap()).unwrap();
    let before = unpack(8, &bytes).unwrap();
    assert_eq!(again.manufacturings(), before.manufacturings());
}

#[test]
fn the_damaged_golden_reads_the_defaults_with_a_notice() {
    let bytes = read_or_write("embroidery_page_damaged_v70.curvyo", build_damaged);
    let (s, view) = opened(&bytes);
    assert_eq!(
        view["hoop"]["key"], "150x250",
        "an unknown hoop reads as the first"
    );
    assert_eq!(
        view["operations"][1]["numbers"]["row_spacing_mm"], 0.25,
        "the default row spacing"
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
    for name in [
        "embroidery_page_v70.curvyo",
        "embroidery_page_damaged_v70.curvyo",
    ] {
        let bytes = std::fs::read(fixture_path(name)).unwrap();
        let manifest = read_manifest(&bytes);
        assert_eq!(manifest["format_version"], 70, "{name}");
        assert!(
            manifest["format_version"].as_u64().unwrap() > 68,
            "newer than 68"
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
