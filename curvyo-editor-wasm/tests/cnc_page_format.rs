//! Format 71 (`specs/0161-cnc-output` criterion 20, `adrs.md` CN2): the goldens
//! `cnc_page_v71.curvyo` and `cnc_page_damaged_v71.curvyo`. The first stores a CNC page's process,
//! profile copy, stock, tool list and its Profile outside and inside operations, and reads them
//! back unchanged; the damaged one (tool diameter 0, a depth beyond the stock) reads as the
//! defaults with a notice and is not rewritten; a build of format 70 refuses both.
//! `CURVYO_WRITE_FIXTURES=1` rewrites the goldens.

#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

use std::path::PathBuf;

use curvyo_document_core::{CURRENT_FORMAT_VERSION, Document, Point, RectBounds, pack, unpack};
use curvyo_editor_wasm::{Session, Tool};
use serde_json::{Value, json};

const CNC: &str = "curvyo.cnc";

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

fn cnc_page(d: &Document) -> curvyo_document_core::Page {
    d.manufacturings()
        .into_iter()
        .find(|m| m.process == CNC)
        .and_then(|m| m.page)
        .unwrap()
}

fn click(s: &mut Session, at: Point) {
    s.set_tool(Tool::Select);
    s.pointer_hover(at, false, false);
    s.pointer_down(at, false);
    s.pointer_up(at, false, false);
}

/// Stock 8 mm, Profile 1 (outside, 6 mm, climb) on one rectangle and Profile 2 (inside) on
/// another.
fn build() -> Vec<u8> {
    let mut s = Session::new(1);
    s.add_page_of_kind(CNC, None).unwrap();
    let d = unpack(2, &s.pack("0.1.0").unwrap()).unwrap();
    let page = cnc_page(&d);
    let o = page.origin;
    for x in [20.0, 100.0] {
        let _ = d.create_rect(RectBounds::from_corners(
            Point::new(o.x + x, o.y + 20.0),
            Point::new(o.x + x + 50.0, o.y + 40.0),
        ));
    }
    let mut s = Session::open(3, &pack(&d, "0.1.0").unwrap()).unwrap();
    s.resize_viewport(1600.0, 1000.0);
    assert!(s.activate_page(&page.id));
    assert_eq!(s.edit_cnc_stock("thickness_mm", "8"), Ok(true));
    click(&mut s, Point::new(o.x + 40.0, o.y + 20.0));
    let outside = s.press_cnc_profile("outside", false).unwrap();
    assert_eq!(s.edit_cnc_setting(outside, "depth_mm", "6"), Ok(true));
    assert_eq!(s.edit_cnc_setting(outside, "direction", "climb"), Ok(true));
    click(&mut s, Point::new(o.x + 120.0, o.y + 20.0));
    s.press_cnc_profile("inside", false).unwrap();
    s.pack("0.1.0").unwrap()
}

/// The golden with the tool's diameter at 0 and Profile 1's depth at 25 mm on the 8 mm stock.
fn build_damaged() -> Vec<u8> {
    let d = unpack(4, &read_or_write("cnc_page_v71.curvyo", build)).unwrap();
    let m = d
        .manufacturings()
        .into_iter()
        .find(|m| m.process == CNC)
        .unwrap();
    let tools = json!([{
        "id": "default", "kind": "flat", "diameter_mm": 0.0, "flutes": 2, "feed_mm_min": 800.0,
        "plunge_mm_min": 300.0, "rpm": 12000, "step_down_mm": 1.0
    }]);
    d.set_process_register(m.id, "tools", Some(&tools), "edit_tool")
        .unwrap();
    let op = d.operations(m.id).unwrap().into_iter().next().unwrap();
    let mut options = op.options.unwrap().as_object().unwrap().clone();
    options.insert("depth_mm".into(), Value::from(25.0));
    d.write_operation_options(op.id, &options).unwrap();
    pack(&d, "0.1.0").unwrap()
}

fn opened(bytes: &[u8]) -> (Session, Value) {
    let mut s = Session::open(5, bytes).unwrap();
    let page = cnc_page(&unpack(6, bytes).unwrap());
    assert!(s.activate_page(&page.id));
    let view: Value = serde_json::from_str(&s.cnc_view_json()).unwrap();
    (s, view)
}

#[test]
fn the_golden_reads_back_its_page_stock_tools_and_operations() {
    assert_eq!(CURRENT_FORMAT_VERSION, 72);
    let bytes = read_or_write("cnc_page_v71.curvyo", build);
    let (s, view) = opened(&bytes);
    assert_eq!(view["profile"]["name"], "Generic GRBL router");
    assert_eq!(view["stock"]["thickness"], 8.0);
    assert_eq!(view["tools"][0]["name"], "Flat 3.175 mm");
    let ops: Vec<(&str, &str, &str)> = view["operations"]
        .as_array()
        .unwrap()
        .iter()
        .map(|op| {
            (
                op["name"].as_str().unwrap(),
                op["numbers"]["side"].as_str().unwrap(),
                op["numbers"]["direction"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        ops,
        [
            ("Profile 1", "outside", "climb"),
            ("Profile 2", "inside", "conventional")
        ]
    );
    assert_eq!(view["operations"][0]["numbers"]["depth_mm"], 6.0);
    assert_eq!(view["notice"], Value::Null);
    let again = unpack(7, &s.pack("0.1.0").unwrap()).unwrap();
    assert_eq!(
        again.manufacturings(),
        unpack(8, &bytes).unwrap().manufacturings()
    );
}

#[test]
fn the_damaged_golden_reads_the_defaults_with_a_notice() {
    let bytes = read_or_write("cnc_page_damaged_v71.curvyo", build_damaged);
    let (s, view) = opened(&bytes);
    assert_eq!(
        view["tools"][0]["numbers"]["diameter_mm"], 3.175,
        "the default diameter"
    );
    assert_eq!(
        view["operations"][0]["numbers"]["depth_mm"], 8.0,
        "a depth beyond the stock reads as the stock thickness"
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
fn a_build_of_format_70_refuses_the_goldens() {
    for name in ["cnc_page_v71.curvyo", "cnc_page_damaged_v71.curvyo"] {
        let bytes = std::fs::read(fixture_path(name)).unwrap();
        let manifest = read_manifest(&bytes);
        assert_eq!(manifest["format_version"], 71, "{name}");
        assert!(manifest["format_version"].as_u64().unwrap() > 70);
    }
}

/// The `manifest.json` of a project file.
fn read_manifest(bytes: &[u8]) -> Value {
    let reader = std::io::Cursor::new(bytes);
    let mut zip = zip::ZipArchive::new(reader).unwrap();
    let file = zip.by_name("manifest.json").unwrap();
    serde_json::from_reader(file).unwrap()
}
