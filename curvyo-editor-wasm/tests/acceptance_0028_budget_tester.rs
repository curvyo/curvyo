//! Independent budget tests for `specs/0028-material-test-library` criterion 30, thread CPU time
//! (never wall time, CLAUDE.md §5): a 12 x 12 grid made by Add test grid within 50 ms, a field
//! edit of a marked 12 x 12 grid within 30 ms, and the Materials list of 200 records (each with
//! a 40-entry history) opening within 50 ms. Assertions only run in a release build; a debug run
//! executes the scenarios and prints the times.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_possible_truncation,
    missing_docs
)]

mod cpu_time;

use std::path::PathBuf;
use std::time::Duration;

use cpu_time::cpu_time_of;
use curvyo_document_core::{Document, DocumentSize, NewOutputPage, pack};
use curvyo_editor_wasm::Session;
use curvyo_ui_core::FontBlock;
use serde_json::{Value, json};

const TIMED: bool = !cfg!(debug_assertions);

fn fonts_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../frontend/public/fonts")
}

fn session(files: &[(String, String)]) -> Session {
    let d = Document::new(1);
    let made = d
        .add_output_page(NewOutputPage {
            process: "curvyo.laser",
            label: "Laser",
            profile: None,
            work_area: DocumentSize::from_mm(400.0, 300.0),
        })
        .unwrap();
    let mut s = Session::open(3, &pack(&d, "0028-budget-tester").unwrap()).unwrap();
    s.load_materials(files, None);
    assert!(s.activate_page(&made.id));
    let index: Value =
        serde_json::from_str(&std::fs::read_to_string(fonts_dir().join("index.json")).unwrap())
            .unwrap();
    let faces = index["faces"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| {
            (
                f["file"].as_str().unwrap().to_owned(),
                f["family"].as_str().unwrap().to_owned(),
                f["style"].as_str().unwrap().to_owned(),
                0,
            )
        })
        .collect();
    s.set_font_list(FontBlock::Included, faces, Vec::new());
    let _ = s.press_add_test_grid();
    for file in s.wanted_font_files() {
        let bytes = std::fs::read(fonts_dir().join(&file)).unwrap();
        assert!(s.add_font_file(&file, &bytes));
    }
    s
}

fn report(what: &str, took: Duration, budget: Duration) {
    eprintln!("{what}: {took:?} (budget {budget:?})");
    if TIMED {
        assert!(took <= budget, "{what}: {took:?} > {budget:?}");
    }
}

#[test]
fn c30_a_12_by_12_grid_is_made_within_50_ms_and_a_field_edit_within_30_ms() {
    let mut s = session(&[]);
    if s.test_grid_view().is_none() {
        s.press_add_test_grid().unwrap();
    }
    s.set_test_grid_field("cell", "6").unwrap();
    s.set_test_grid_field("gap", "2").unwrap();
    s.set_test_grid_field("columns", "12").unwrap();
    s.set_test_grid_field("rows", "12").unwrap();
    // The next grid takes the remembered 12 x 12 values.
    let (made, took) = cpu_time_of(|| s.press_add_test_grid());
    made.unwrap();
    let view: Value = serde_json::from_str(&s.test_grid_view_json()).unwrap();
    assert_eq!(view["fields"]["columns"], 12);
    assert_eq!(view["fields"]["rows"], 12);
    report("make 12 x 12", took, Duration::from_millis(50));
    assert!(s.press_test_cell(11, 11));
    let (edited, took) = cpu_time_of(|| s.set_test_grid_field("gap", "3"));
    assert!(edited.unwrap());
    report("field edit 12 x 12", took, Duration::from_millis(30));
    let view: Value = serde_json::from_str(&s.test_grid_view_json()).unwrap();
    assert!(!view["worked"].is_null(), "a gap edit keeps the mark");
}

fn record(n: usize) -> (String, String) {
    let id = format!("00000000-0000-4000-8000-{n:012}");
    let history: Vec<Value> = (0..40_u64)
        .map(|h| {
            json!({
                "id": format!("10000000-0000-4000-8000-{:06}{h:06}", n),
                "date": 1_762_948_800_000_u64 + h,
                "event": "edited", "kind": "cut",
                "set": {"power_pct": 50.0, "speed_mm_min": 500, "passes": 1}
            })
        })
        .collect();
    let v = json!({
        "format_version": 1, "id": id, "kind": "laser", "modified": 1_762_948_800_000_u64,
        "name": format!("Material {n:03}"), "thickness_mm": 3.0,
        "machine": {"id": "generic-laser", "name": "Generic laser"},
        "sets": {"cut": {"power_pct": 50.0, "speed_mm_min": 500, "passes": 1, "source": "edited"}},
        "history": history
    });
    (
        format!("{id}.json"),
        serde_json::to_string_pretty(&v).unwrap(),
    )
}

#[test]
fn c30_the_materials_list_with_200_records_opens_within_50_ms() {
    let files: Vec<(String, String)> = (0..200).map(record).collect();
    let s = session(&files);
    let (text, took) = cpu_time_of(|| s.materials_view_json());
    let view: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(view["rows"].as_array().unwrap().len(), 200);
    report("materials list of 200", took, Duration::from_millis(50));
}
