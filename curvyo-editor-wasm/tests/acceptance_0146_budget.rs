//! Budgets of `specs/0146-material-test-picks` criterion 24 with round Engrave cells: placing a 12 x
//! 12 Engrave grid within 50 ms, a bar edit within 30 ms and marking a pick within 30 ms, in thread
//! CPU time. Asserted in release builds only; a debug run prints the numbers.
#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

mod cpu_time;

use std::path::PathBuf;
use std::time::Duration;

use cpu_time::cpu_time_of;
use curvyo_document_core::{Document, DocumentSize, NewOutputPage, Point, Timestamp, pack};
use curvyo_editor_wasm::{Session, Tool};
use curvyo_ui_core::FontBlock;
use serde_json::Value;

fn fonts_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../frontend/public/fonts")
}

fn session() -> Session {
    let d = Document::new(1);
    let laser = d
        .add_output_page(NewOutputPage {
            process: "curvyo.laser",
            label: "Laser",
            profile: None,
            work_area: DocumentSize::from_mm(600.0, 400.0),
        })
        .unwrap();
    let mut s = Session::open(5, &pack(&d, "0112-budget").unwrap()).unwrap();
    s.set_clock(|| Timestamp::from_secs(1_762_948_800));
    assert!(s.activate_page(&laser.id));
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
    s.set_tool(Tool::TestGrid);
    let _ = s.test_grid_press(None);
    for file in s.wanted_font_files() {
        let bytes = std::fs::read(fonts_dir().join(&file)).unwrap();
        assert!(s.add_font_file(&file, &bytes));
    }
    s.set_tool(Tool::Select);
    s.load_materials(&[], None);
    s
}

/// A session whose bar is 12 x 12, with no grid on the page (the first grid made the bar 12 x
/// 12 and was undone).
fn bar_12() -> Session {
    let mut s = session();
    let o = s.active_page().origin;
    s.set_tool(Tool::TestGrid);
    s.test_grid_press(Some(Point::new(o.x + 5.0, o.y + 5.0)))
        .unwrap();
    s.set_tool(Tool::Select);
    s.set_test_grid_kind("engrave").unwrap();
    s.set_test_grid_field("columns_count", "12").unwrap();
    s.set_test_grid_field("rows_count", "12").unwrap();
    s.delete_selected();
    s
}

fn check(what: &str, took: Duration, budget_ms: u64) {
    println!("criterion 24 {what}: {took:?} (budget {budget_ms} ms)");
    if !cfg!(debug_assertions) {
        assert!(
            took <= Duration::from_millis(budget_ms),
            "{what}: {took:?} > {budget_ms} ms"
        );
    }
}

#[test]
fn c24_placing_a_12_by_12_engrave_grid_of_circles() {
    let mut s = bar_12();
    let o = s.active_page().origin;
    s.set_tool(Tool::TestGrid);
    let (g, took) = cpu_time_of(|| s.test_grid_press(Some(Point::new(o.x + 5.0, o.y + 5.0))));
    assert!(g.is_ok(), "{g:?}");
    let v: Value = serde_json::from_str(&s.test_grid_view_json()).unwrap();
    assert_eq!(
        (v["kind"].as_str(), v["columns"]["count"].as_u64()),
        (Some("engrave"), Some(12))
    );
    check("placing a 12 x 12 Engrave grid", took, 50);
}

#[test]
fn c24_a_bar_edit_of_a_12_by_12_engrave_grid() {
    let mut s = bar_12();
    let o = s.active_page().origin;
    s.set_tool(Tool::TestGrid);
    s.test_grid_press(Some(Point::new(o.x + 5.0, o.y + 5.0)))
        .unwrap();
    s.set_tool(Tool::Select);
    let (r, took) = cpu_time_of(|| s.set_test_grid_field("cell", "12"));
    assert_eq!(r, Ok(true));
    check("a bar edit regenerating 12 x 12 circles", took, 30);
}

#[test]
fn c24_marking_a_pick_is_one_step_within_30_ms() {
    let mut s = bar_12();
    let o = s.active_page().origin;
    s.set_tool(Tool::TestGrid);
    s.test_grid_press(Some(Point::new(o.x + 5.0, o.y + 5.0)))
        .unwrap();
    s.set_tool(Tool::Select);
    let birch = s.add_material("Birch 3 mm", 3.0, &[3; 16]).unwrap();
    s.pick_test_grid_material(Some(&birch)).unwrap();
    let v: Value = serde_json::from_str(&s.test_grid_view_json()).unwrap();
    let id = v["id"].as_str().unwrap().to_owned();
    let steps = s.undo_len();
    let (r, took) = cpu_time_of(|| s.mark_pick(&birch, &id, "fastest", 11, 11, &[8; 16]));
    assert_eq!(r, Ok(()));
    assert_eq!(s.undo_len(), steps + 1, "one step");
    check("marking a pick", took, 30);
}
