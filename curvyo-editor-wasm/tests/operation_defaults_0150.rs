//! `specs/0150-operation-always-has-numbers` through the public session: the Machine block's
//! Default power and Default speed (criteria 1, 2, 22), a new operation with its numbers in its
//! add step (4, 20, W11), a pair edit that changes no operation (8, W12), and the old file that
//! opens without a change and is filled only by a changing save (15, 16).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    missing_docs
)]

use std::collections::BTreeMap;
use std::io::{Cursor, Read};
use std::path::PathBuf;

use curvyo_document_core::{
    Direction, Document, DocumentSize, NewOperation, NewOutputPage, NodeId, Point, RectBounds,
    pack, unpack,
};
use curvyo_editor_wasm::{Session, Tool};
use serde_json::Value;

struct Fx {
    s: Session,
    m: NodeId,
    rect: NodeId,
    /// An operation stored without `op_settings`, as a file of format 56 holds it.
    old: NodeId,
}

/// A laser page with one rectangle and a Cut without `op_settings`.
fn fx() -> Fx {
    let d = Document::new(1);
    let made = d
        .add_output_page(NewOutputPage {
            process: "curvyo.laser",
            label: "Laser",
            profile: None,
            work_area: DocumentSize::from_mm(400.0, 300.0),
        })
        .unwrap();
    let page = d.pages().into_iter().find(|p| p.id == made.id).unwrap();
    let rect = d.create_rect(RectBounds::from_corners(
        Point::new(page.origin.x + 10.0, page.origin.y + 10.0),
        Point::new(page.origin.x + 30.0, page.origin.y + 20.0),
    ));
    let m = made.kind.manufacturing().unwrap();
    let old = d
        .add_operation(
            m,
            &NewOperation {
                kind: "cut",
                name: "Cut 1",
                settings: None,
            },
            0,
            &[vec![rect]],
        )
        .unwrap();
    let mut s = Session::open(3, &pack(&d, "0150").unwrap()).unwrap();
    s.resize_viewport(1600.0, 1000.0);
    s.set_tool(Tool::Select);
    assert!(s.activate_page(&made.id));
    s.wipe_history();
    Fx { s, m, rect, old }
}

fn copy(s: &Session) -> Value {
    let v: Value = serde_json::from_str(&s.manufacturing_view_json()).unwrap();
    v["machine"]["copy"].clone()
}

fn row(s: &Session, id: NodeId) -> Value {
    let want = serde_json::to_value(id).unwrap();
    let v: Value = serde_json::from_str(&s.operations_view_json()).unwrap();
    v["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == want)
        .cloned()
        .unwrap()
}

fn line3(s: &Session, id: NodeId) -> String {
    row(s, id)["numbers"][0]["text"]
        .as_str()
        .unwrap()
        .to_owned()
}

fn undo_name(s: &mut Session) -> String {
    s.history_target_name(Direction::Undo).unwrap_or_default()
}

fn stored(s: &Session, m: NodeId, id: NodeId) -> Option<Value> {
    let d = unpack(9, &s.pack("0.1.0").unwrap()).unwrap();
    d.operations(m)
        .unwrap()
        .into_iter()
        .find(|op| op.id == id)
        .unwrap()
        .settings
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

fn fixture(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../curvyo-document-core/tests/fixtures")
        .join(name);
    std::fs::read(path).unwrap()
}

#[test]
fn criterion_1_the_machine_block_has_the_pair_with_limits_and_one_step_each() {
    let mut f = fx();
    let c = copy(&f.s);
    assert_eq!(
        (c["default_power"].as_f64(), c["default_speed"].as_u64()),
        (Some(10.0), Some(1000))
    );
    let steps = f.s.undo_len();
    assert_eq!(
        f.s.set_machine_default_power(12.34).as_deref(),
        Some("Default power rounded to 12.3 %.")
    );
    assert_eq!(f.s.undo_len(), steps + 1);
    assert_eq!(undo_name(&mut f.s), "Edit default power");
    assert_eq!(copy(&f.s)["default_power"], 12.3);
    assert_eq!(
        f.s.set_machine_default_speed(7000.0).as_deref(),
        Some("Default speed limited to 6,000 mm/min.")
    );
    assert_eq!(undo_name(&mut f.s), "Edit default speed");
    assert_eq!(copy(&f.s)["default_speed"], 6000);
    assert_eq!(
        f.s.set_machine_default_speed(1250.0),
        None,
        "no success notice"
    );
    let _ = f.s.undo();
    assert_eq!(
        copy(&f.s)["default_speed"],
        6000,
        "Ctrl+Z puts the previous default back"
    );
}

#[test]
fn criterion_2_a_lower_max_feed_lowers_the_default_speed_in_the_same_step() {
    let mut f = fx();
    f.s.set_machine_default_speed(5000.0);
    let steps = f.s.undo_len();
    assert_eq!(
        f.s.set_machine_max_feed(4000.0).as_deref(),
        Some("Default speed limited to 4,000 mm/min.")
    );
    assert_eq!(f.s.undo_len(), steps + 1, "one step");
    assert_eq!(undo_name(&mut f.s), "Edit max feed");
    assert_eq!(
        (
            copy(&f.s)["max_feed"].as_u64(),
            copy(&f.s)["default_speed"].as_u64()
        ),
        (Some(4000), Some(4000))
    );
    let _ = f.s.undo();
    assert_eq!(
        (
            copy(&f.s)["max_feed"].as_u64(),
            copy(&f.s)["default_speed"].as_u64()
        ),
        (Some(6000), Some(5000))
    );
}

#[test]
fn criteria_4_20_a_new_operation_has_numbers_in_its_add_step_and_redo_replays_them() {
    let mut f = fx();
    f.s.set_machine_default_power(20.0);
    assert!(f.s.select_output_items(&[f.rect]));
    let steps = f.s.undo_len();
    let cut = f.s.press_operation_kind("cut", true).unwrap();
    assert_eq!(
        f.s.undo_len(),
        steps + 1,
        "one step, no second one for the numbers"
    );
    assert_eq!(line3(&f.s, cut), "20 %, 1,000 mm/min, 1 ×");
    let settings = row(&f.s, cut)["settings"].clone();
    assert_eq!(settings["state"], "own");
    assert_eq!(settings["sets"][0]["run"]["power_pct"], 20.0);
    let outline = f.s.press_operation_kind("engrave_outline", true).unwrap();
    let sets = row(&f.s, outline)["settings"]["sets"].clone();
    assert_eq!(sets[1]["role"], "outline");
    assert_eq!(sets[1]["run"], sets[0]["run"], "the outline equals main");
    let _ = f.s.undo();
    let _ = f.s.undo();
    let names: Vec<String> =
        serde_json::from_str::<Value>(&f.s.operations_view_json()).unwrap()["rows"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r["name"].as_str().unwrap().to_owned())
            .collect();
    assert_eq!(names, ["Cut 1"], "undo removes the whole operation");
    let _ = f.s.redo();
    let _ = f.s.redo();
    assert_eq!(
        stored(&f.s, f.m, cut).unwrap()["main"]["power_pct"],
        20.0,
        "a redo replays the step's numbers (W11)"
    );
}

#[test]
fn criteria_8_22_a_pair_edit_changes_no_operation_and_undo_returns_both() {
    let mut f = fx();
    assert_eq!(line3(&f.s, f.old), "10 %, 1,000 mm/min, 1 ×");
    assert!(stored(&f.s, f.m, f.old).is_none());
    let steps = f.s.undo_len();
    f.s.set_machine_default_power(25.0);
    assert_eq!(f.s.undo_len(), steps + 1, "one step");
    assert_eq!(
        line3(&f.s, f.old),
        "10 %, 1,000 mm/min, 1 ×",
        "the row keeps its numbers (W12)"
    );
    assert_eq!(stored(&f.s, f.m, f.old).unwrap()["main"]["power_pct"], 10.0);
    f.s.set_machine_default_speed(800.0);
    assert_eq!(line3(&f.s, f.old), "10 %, 1,000 mm/min, 1 ×");
    let _ = f.s.undo();
    let _ = f.s.undo();
    assert!(
        stored(&f.s, f.m, f.old).is_none(),
        "undo returns copy and register together"
    );
    assert_eq!(line3(&f.s, f.old), "10 %, 1,000 mm/min, 1 ×");
}

#[test]
fn criteria_9_11_material_and_own_numbers_never_return_to_an_empty_state() {
    let mut f = fx();
    let typed =
        f.s.edit_operation_setting(f.old, "main", "power", 40.0)
            .unwrap();
    assert!(typed, "a typed number on a derived operation is one step");
    assert_eq!(undo_name(&mut f.s), "Edit settings");
    assert_eq!(line3(&f.s, f.old), "40 %, 1,000 mm/min, 1 ×");
    let _ = f.s.undo();
    assert_eq!(
        line3(&f.s, f.old),
        "10 %, 1,000 mm/min, 1 ×",
        "never an empty state"
    );
    assert_eq!(row(&f.s, f.old)["settings"]["state"], "own");
}

#[test]
fn criteria_15_16_an_old_file_opens_unchanged_and_only_a_changing_save_fills_it() {
    let bytes = fixture("laser_settings_v43.curvyo");
    let mut s = Session::open(4, &bytes).unwrap();
    let d = unpack(9, &bytes).unwrap();
    let m = d.manufacturings()[0].clone();
    assert!(s.activate_page(&m.page.as_ref().unwrap().id));
    let ops = d.operations(m.id).unwrap();
    let old = ops[3].id;
    assert!(ops[3].settings.is_none());
    assert_eq!(row(&s, old)["settings"]["state"], "own");
    assert!(
        line3(&s, old).starts_with("10 %, 1,000 mm/min, 1 ×"),
        "{}",
        line3(&s, old)
    );
    assert!(!s.is_dirty(), "no modified mark");
    assert_eq!(s.undo_len(), 0, "no step");
    let saved = s.pack_for_save().unwrap();
    assert!(!s.is_dirty());
    let reread = unpack(9, &saved).unwrap();
    assert!(reread.operations(m.id).unwrap()[3].settings.is_none());
    assert_eq!(
        members(&saved)["document.loro"],
        members(&bytes)["document.loro"]
    );
    // Any change: the save stores the derived numbers.
    assert!(s.rename_operation(ops[0].id, "Renamed"));
    let saved = s.pack_for_save().unwrap();
    let reread = unpack(9, &saved).unwrap();
    let filled = reread.operations(m.id).unwrap()[3]
        .settings
        .clone()
        .unwrap();
    assert_eq!(filled["main"]["power_pct"], 10.0);
    assert_eq!(filled["main"]["speed_mm_min"], 1000);
    assert_eq!(filled["main"]["passes"], 1);
    assert_eq!(undo_name(&mut s), "Rename operation", "the fill is no step");
}

#[test]
fn criterion_15_each_page_reads_its_own_copy_pair() {
    let bytes = fixture("operation_defaults_v57.curvyo");
    let mut s = Session::open(4, &bytes).unwrap();
    let d = unpack(9, &bytes).unwrap();
    let pages: Vec<_> = d.manufacturings();
    assert!(s.activate_page(&pages[0].page.as_ref().unwrap().id));
    let first = d.operations(pages[0].id).unwrap();
    assert_eq!(line3(&s, first[0].id), "12.5 %, 1,500 mm/min, 1 ×");
    assert_eq!(
        row(&s, first[5].id)["settings"]["state"],
        "damaged",
        "damage stays damage"
    );
    assert!(
        row(&s, first[6].id)["settings"].is_null(),
        "an unknown kind is never read"
    );
    assert!(s.activate_page(&pages[1].page.as_ref().unwrap().id));
    let second = d.operations(pages[1].id).unwrap();
    assert_eq!(line3(&s, second[0].id), "10 %, 600 mm/min, 1 ×");
    assert!(!s.is_dirty());
}

#[test]
fn criterion_15_a_legacy_test_grid_row_is_never_read_or_filled() {
    let bytes = fixture("laser_settings_v43.curvyo");
    let mut s = Session::open(4, &bytes).unwrap();
    let d = unpack(9, &bytes).unwrap();
    let m = d.manufacturings()[0].clone();
    assert!(s.activate_page(&m.page.as_ref().unwrap().id));
    let grid = d
        .operations(m.id)
        .unwrap()
        .into_iter()
        .find(|op| op.grid)
        .expect("the format-43 golden holds a test grid row");
    assert!(grid.settings.is_none());
    let before = row(&s, grid.id);
    assert!(before["numbers"].is_null(), "no derived line 3: {before}");
    assert!(before["settings"].is_null());
    assert!(s.set_machine_default_power(55.0).is_none());
    assert_eq!(
        row(&s, grid.id),
        before,
        "a pair edit leaves the row as it was"
    );
    assert!(
        stored(&s, m.id, grid.id).is_none(),
        "no fill writes into it"
    );
    assert!(s.rename_operation(grid.id, "Renamed grid"));
    let saved = s.pack_for_save().unwrap();
    let reread = unpack(9, &saved).unwrap();
    let after = reread
        .operations(m.id)
        .unwrap()
        .into_iter()
        .find(|op| op.id == grid.id)
        .unwrap();
    assert!(after.grid);
    assert!(
        after.settings.is_none(),
        "a changing save writes nothing into it"
    );
}
