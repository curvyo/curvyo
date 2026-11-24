//! Tester acceptance cases for `specs/0150-operation-always-has-numbers` through the public session,
//! written from the specification and the UX notes before the implementation was read: a new
//! operation has the default numbers in its one "Add operation" step (criteria 4, 20), the Default
//! power and Default speed of the Machine block (1, 2, 22), the material pick, "Own numbers" and
//! undo (9 to 14), the old file derived on open with no step and no modified mark (15, 16, 21),
//! damaged settings never filled (17), the numbers at open surviving a later default edit (W12),
//! and the one-step Max feed clamp (2). Black box.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::too_many_lines,
    clippy::float_cmp,
    clippy::needless_pass_by_value,
    missing_docs
)]

use std::collections::BTreeMap;
use std::io::{Cursor, Read};
use std::path::PathBuf;

use curvyo_document_core::{
    Direction, Document, DocumentSize, NewOperation, NewOutputPage, NodeId, OperationSnapshot,
    Point, RectBounds, pack, unpack,
};
use curvyo_editor_wasm::{Session, Tool};
use serde_json::{Value, json};

const LASER: &str = "curvyo.laser";
const BIRCH: &str = "22222222-2222-4222-8222-222222222222";
const CARD: &str = "11111111-1111-4111-8111-111111111111";

fn fixture(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../curvyo-document-core/tests/fixtures")
        .join(name);
    std::fs::read(path).unwrap()
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

/// A material record of format 2 for Generic laser with the given sets.
fn record_text(id: &str, name: &str, sets: Value) -> String {
    let mut with_sources = serde_json::Map::new();
    for (kind, set) in sets.as_object().unwrap() {
        let mut s = set.as_object().unwrap().clone();
        if kind == "engrave" {
            s.insert("interval_mm".into(), json!(0.1));
        }
        s.insert("source".into(), json!("typed"));
        with_sources.insert(kind.clone(), Value::Object(s));
    }
    serde_json::to_string_pretty(&json!({
        "format_version": 2, "id": id, "kind": "laser", "modified": 1_762_948_800_000_u64,
        "name": name, "thickness_mm": 3.0,
        "lasers": [{"machine": {"id": "generic-laser", "name": "Generic laser"},
            "sets": with_sources}],
        "tests": [], "history": []
    }))
    .unwrap()
}

fn materials() -> Vec<(String, String)> {
    vec![
        (
            format!("{BIRCH}.json"),
            record_text(
                BIRCH,
                "Birch",
                json!({
                    "cut": {"power_pct": 60.0, "speed_mm_min": 300, "passes": 2},
                    "engrave": {"power_pct": 35.0, "speed_mm_min": 6000, "passes": 1}
                }),
            ),
        ),
        (
            format!("{CARD}.json"),
            record_text(
                CARD,
                "Card",
                json!({"cut": {"power_pct": 40.0, "speed_mm_min": 900, "passes": 1}}),
            ),
        ),
    ]
}

/// A laser page of Generic laser with one rectangle, and one operation per kind in `kinds`.
fn fresh(kinds: &[&str]) -> (Session, String, NodeId, Vec<NodeId>) {
    let d = Document::new(1);
    let made = d
        .add_output_page(NewOutputPage {
            process: LASER,
            label: "Laser",
            profile: None,
            work_area: DocumentSize::from_mm(400.0, 300.0),
        })
        .unwrap();
    let page = d.pages().into_iter().find(|p| p.id == made.id).unwrap();
    let r = d.create_rect(RectBounds::from_corners(
        Point::new(page.origin.x + 10.0, page.origin.y + 10.0),
        Point::new(page.origin.x + 30.0, page.origin.y + 20.0),
    ));
    let m = made.kind.manufacturing().unwrap();
    let ids = kinds
        .iter()
        .enumerate()
        .map(|(i, kind)| {
            let name = format!("{kind} {}", i + 1);
            d.add_operation(
                m,
                &NewOperation {
                    kind,
                    name: &name,
                    settings: None,
                },
                i,
                &[vec![r]],
            )
            .unwrap()
        })
        .collect();
    let mut s = Session::open(3, &pack(&d, "0150-tester").unwrap()).unwrap();
    s.resize_viewport(1600.0, 1000.0);
    s.set_tool(Tool::Select);
    s.load_materials(&materials(), None);
    assert!(s.activate_page(&made.id));
    let at = Point::new(page.origin.x + 20.0, page.origin.y + 10.0);
    s.pointer_hover(at, false, false);
    s.pointer_down(at, false);
    s.pointer_up(at, false, false);
    s.wipe_history();
    (s, made.id, m, ids)
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

fn main_run(s: &Session, id: NodeId) -> (f64, u64) {
    let run = &row(s, id)["settings"]["sets"][0]["run"];
    (
        run["power_pct"].as_f64().unwrap(),
        run["speed_mm_min"].as_u64().unwrap(),
    )
}

/// The stored operation of `id` in the saved project, and the copy of manufacturing `m`.
fn undo_name(s: &mut Session) -> String {
    s.history_target_name(Direction::Undo).unwrap_or_default()
}

fn no_settings_text(s: &Session) -> bool {
    let v = s.operations_view_json();
    v.contains("No settings") || v.contains("no_settings")
}

// ---- criteria 4, 20: a new operation has its numbers in the one add step ----------------------

#[test]
fn c4_a_new_cut_has_the_built_in_numbers_at_once_as_own() {
    let (mut s, _, _, _) = fresh(&[]);
    let before = s.undo_len();
    let id = s
        .press_operation_kind("cut", false)
        .expect("a Cut is added");
    assert_eq!(s.undo_len(), before + 1, "one step");
    assert!(
        undo_name(&mut s).starts_with("Assign to "),
        "build label: {}",
        undo_name(&mut s)
    );
    assert_eq!(main_run(&s, id), (10.0, 1_000));
    assert_eq!(row(&s, id)["settings"]["state"], "own");
    assert!(!no_settings_text(&s), "no No settings anywhere");
}

#[test]
fn c4_a_new_engrave_outline_has_outline_equal_to_main() {
    let (mut s, _, _, _) = fresh(&[]);
    let id = s.press_operation_kind("engrave_outline", false).unwrap();
    assert_eq!(main_run(&s, id), (10.0, 1_000));
    assert_eq!(row(&s, id)["settings"]["state"], "own");
    assert!(!no_settings_text(&s));
}

#[test]
fn c20_undo_after_add_operation_removes_the_whole_operation() {
    let (mut s, _, _, _) = fresh(&[]);
    let id = s.press_operation_kind("cut", false).unwrap();
    s.undo();
    let v: Value = serde_json::from_str(&s.operations_view_json()).unwrap();
    let want = serde_json::to_value(id).unwrap();
    assert!(
        v["rows"]
            .as_array()
            .unwrap()
            .iter()
            .all(|r| r["id"] != want),
        "the operation is gone, no empty one remains"
    );
}

#[test]
fn c20_undo_after_an_edit_returns_to_the_previous_numbers_never_empty() {
    let (mut s, _, _, _) = fresh(&[]);
    let id = s.press_operation_kind("cut", false).unwrap();
    s.edit_operation_setting(id, "main", "power", 40.0).unwrap();
    assert_eq!(main_run(&s, id).0, 40.0);
    s.undo();
    assert_eq!(
        main_run(&s, id),
        (10.0, 1_000),
        "back to the numbers of the add step"
    );
}

// ---- criteria 1, 8, 22: the Machine block defaults --------------------------------------------

#[test]
fn c1_c22_default_power_edit_is_one_step_and_changes_no_existing_operation() {
    let (mut s, _, m, _) = fresh(&[]);
    let id = s.press_operation_kind("cut", false).unwrap();
    s.wipe_history();
    let before = s.undo_len();
    assert_eq!(
        s.set_machine_default_power(25.0),
        None,
        "in range: no notice"
    );
    assert_eq!(s.undo_len(), before + 1, "one undo step");
    assert_eq!(undo_name(&mut s), "Edit default power");
    assert_eq!(
        main_run(&s, id),
        (10.0, 1_000),
        "an existing operation keeps its numbers"
    );
    let newly = s.press_operation_kind("score", false).unwrap();
    assert_eq!(
        main_run(&s, newly).0,
        25.0,
        "the next operation takes the new pair"
    );
    s.undo();
    let _ = m;
}

#[test]
fn c1_default_speed_out_of_range_is_limited_with_a_notice_in_one_step() {
    let (mut s, _, _, _) = fresh(&[]);
    let before = s.undo_len();
    let notice = s.set_machine_default_speed(9_000.0).expect("a notice");
    assert_eq!(notice, "Default speed limited to 6,000 mm/min.");
    assert_eq!(s.undo_len(), before + 1);
    assert_eq!(undo_name(&mut s), "Edit default speed");
}

#[test]
fn c2_lowering_max_feed_below_default_speed_is_one_step_and_clamps_the_default() {
    let (mut s, _, _, _) = fresh(&[]);
    assert_eq!(s.set_machine_default_speed(5_000.0), None);
    s.wipe_history();
    let before = s.undo_len();
    let notice = s
        .set_machine_max_feed(4_000.0)
        .expect("a notice names Default speed");
    assert_eq!(notice, "Default speed limited to 4,000 mm/min.");
    assert_eq!(s.undo_len(), before + 1, "one step for both fields");
    let bytes = s.pack("0150-tester").unwrap();
    let d = unpack(9, &bytes).unwrap();
    let copy = serde_json::to_value(d.manufacturings()[0].profile.clone()).unwrap();
    assert_eq!(copy["max_feed_mm_per_min"], json!(4_000));
    assert_eq!(
        copy["default_speed_mm_min"],
        json!(4_000),
        "never above Max feed"
    );
    s.undo();
    let back = serde_json::to_value(
        unpack(9, &s.pack("0150-tester").unwrap())
            .unwrap()
            .manufacturings()[0]
            .profile
            .clone(),
    )
    .unwrap();
    assert_eq!(
        back["default_speed_mm_min"],
        json!(5_000),
        "Ctrl+Z restores both"
    );
    assert_eq!(back["max_feed_mm_per_min"], json!(6_000));
}

// ---- criteria 9 to 14: material pick, Own numbers, undo ---------------------------------------

#[test]
fn c9_picking_a_material_on_a_default_operation_is_one_step_and_undo_gives_the_default_back() {
    let (mut s, _, _, _) = fresh(&[]);
    let id = s.press_operation_kind("cut", false).unwrap();
    s.wipe_history();
    assert!(s.use_material_on(&[id], BIRCH));
    assert_eq!(undo_name(&mut s), "Use material");
    assert_eq!(main_run(&s, id), (60.0, 300), "the material's numbers");
    s.undo();
    assert_eq!(
        main_run(&s, id),
        (10.0, 1_000),
        "the default numbers the row had"
    );
    assert_eq!(row(&s, id)["settings"]["state"], "own");
}

#[test]
fn c10_picking_another_material_and_undo_restores_the_previous_material() {
    let (mut s, _, _, ops) = fresh(&["cut"]);
    let id = ops[0];
    assert!(s.use_material_on(&[id], BIRCH));
    assert!(s.use_material_on(&[id], CARD));
    assert_eq!(main_run(&s, id), (40.0, 900));
    s.undo();
    assert_eq!(main_run(&s, id), (60.0, 300), "Birch and its numbers again");
}

#[test]
fn c11_own_numbers_after_a_material_keeps_the_numbers_that_ran() {
    let (mut s, _, _, ops) = fresh(&["cut"]);
    let id = ops[0];
    assert!(s.use_material_on(&[id], BIRCH));
    assert!(s.use_own_numbers_on(&[id]));
    assert_eq!(row(&s, id)["settings"]["state"], "own");
    assert_eq!(
        main_run(&s, id),
        (60.0, 300),
        "the numbers that ran stay, not the pair"
    );
    assert!(!no_settings_text(&s));
}

#[test]
fn c14_a_typed_number_on_a_linked_operation_is_an_override_and_reset_removes_it() {
    let (mut s, _, _, ops) = fresh(&["cut"]);
    let id = ops[0];
    assert!(s.use_material_on(&[id], BIRCH));
    s.edit_operation_setting(id, "main", "power", 65.0).unwrap();
    assert_eq!(main_run(&s, id).0, 65.0);
    assert!(s.reset_operation_to_material(id));
    assert_eq!(
        main_run(&s, id),
        (60.0, 300),
        "the material's number shows again"
    );
}

#[test]
fn c13_a_linked_operation_with_its_material_gone_keeps_the_stored_numbers_not_the_pair() {
    let (mut s, _, _, ops) = fresh(&["cut"]);
    let id = ops[0];
    assert!(s.use_material_on(&[id], BIRCH));
    s.load_materials(&[], None);
    assert_eq!(row(&s, id)["settings"]["state"], "stored");
    assert_eq!(
        main_run(&s, id),
        (60.0, 300),
        "the stored numbers, never the defaults"
    );
}

// ---- criteria 15, 16, 21: old files ----------------------------------------------------------

fn open_old(name: &str) -> (Session, NodeId) {
    let bytes = fixture(name);
    let d = unpack(2, &bytes).unwrap();
    let m = d.manufacturings()[0].id;
    let page = d.manufacturings()[0].page.as_ref().unwrap().id.clone();
    let mut s = Session::open(4, &bytes).unwrap();
    s.resize_viewport(1600.0, 1000.0);
    assert!(s.activate_page(&page));
    (s, m)
}

#[test]
fn c15_an_old_file_opens_with_derived_numbers_no_step_and_no_modified_mark() {
    let bytes = fixture("laser_settings_v43.curvyo");
    let d = unpack(2, &bytes).unwrap();
    let m = d.manufacturings()[0].id;
    let op = d
        .operations(m)
        .unwrap()
        .into_iter()
        .find(|o| o.settings.is_none() && !o.grid)
        .expect("the golden holds an operation without numbers");
    let (mut s, _) = open_old("laser_settings_v43.curvyo");
    assert_eq!(s.undo_len(), 0, "opening adds no history step");
    assert!(!s.is_dirty(), "no modified mark");
    assert_eq!(row(&s, op.id)["settings"]["state"], "own");
    let copy_pair = copy_pair_of(&d, m);
    assert_eq!(
        main_run(&s, op.id),
        copy_pair,
        "the copy's pair, 10 % and 1,000 mm/min if none"
    );
    assert!(!no_settings_text(&s));
    let _ = &mut s;
}

fn copy_pair_of(d: &Document, m: NodeId) -> (f64, u64) {
    let copy = serde_json::to_value(d.manufacturing(m).unwrap().profile).unwrap();
    let power = copy["default_power_pct"].as_f64().unwrap_or(10.0);
    let speed = copy["default_speed_mm_min"]
        .as_u64()
        .unwrap_or(1_000)
        .min(copy["max_feed_mm_per_min"].as_u64().unwrap_or(6_000));
    (power, speed)
}

#[test]
fn c16_a_plain_save_of_an_old_file_writes_no_numbers_and_keeps_the_document_bytes() {
    let original = fixture("laser_settings_v43.curvyo");
    let d = unpack(2, &original).unwrap();
    let m = d.manufacturings()[0].id;
    let (mut s, _) = open_old("laser_settings_v43.curvyo");
    let saved = s.pack_for_save().unwrap();
    let op = d
        .operations(m)
        .unwrap()
        .into_iter()
        .find(|o| o.settings.is_none() && !o.grid)
        .unwrap();
    let again = unpack(57, &saved).unwrap();
    let kept = again
        .operations(m)
        .unwrap()
        .into_iter()
        .find(|o| o.id == op.id)
        .unwrap();
    assert_eq!(
        kept.settings, None,
        "no write-back on a plain open and save"
    );
    assert_eq!(
        members(&saved)["document.loro"],
        members(&original)["document.loro"],
        "the document bytes are unchanged by a plain save"
    );
}

#[test]
fn c16_a_changing_save_stores_the_derived_numbers_of_the_open_time() {
    let original = fixture("laser_settings_v43.curvyo");
    let d = unpack(2, &original).unwrap();
    let m = d.manufacturings()[0].id;
    let op = d
        .operations(m)
        .unwrap()
        .into_iter()
        .find(|o| o.settings.is_none() && !o.grid)
        .unwrap();
    let (mut s, _) = open_old("laser_settings_v43.curvyo");
    let at_open = main_run(&s, op.id);
    // W12: the pair is edited before the first save; the numbers shown at open stay.
    assert!(s.set_machine_default_power(30.0).is_none());
    assert!(s.set_machine_default_speed(2_000.0).is_none());
    assert_eq!(
        main_run(&s, op.id),
        at_open,
        "the row does not move under the maker"
    );
    s.rename_operation(op.id, "Renamed");
    let saved = s.pack_for_save().unwrap();
    let again = unpack(57, &saved).unwrap();
    let kept = again
        .operations(m)
        .unwrap()
        .into_iter()
        .find(|o| o.id == op.id)
        .unwrap();
    let stored_main = &kept.settings.as_ref().expect("the numbers are stored")["main"];
    assert_eq!(stored_main["power_pct"].as_f64().unwrap(), at_open.0);
    assert_eq!(stored_main["speed_mm_min"].as_u64().unwrap(), at_open.1);
}

#[test]
fn c17_a_damaged_register_is_not_filled_and_keeps_its_warning() {
    let original = fixture("laser_settings_damaged_v43.curvyo");
    let d = unpack(2, &original).unwrap();
    let m = d.manufacturings()[0].id;
    let before: Vec<OperationSnapshot> = d.operations(m).unwrap();
    let (mut s, _) = open_old("laser_settings_damaged_v43.curvyo");
    assert_eq!(s.undo_len(), 0);
    let damaged_rows: Vec<&OperationSnapshot> = before
        .iter()
        .filter(|o| o.settings.as_ref().is_some_and(Value::is_string))
        .collect();
    assert!(
        !damaged_rows.is_empty(),
        "the golden has a damaged register"
    );
    for o in &damaged_rows {
        assert_eq!(row(&s, o.id)["settings"]["state"], "damaged");
    }
    s.rename_operation(before[0].id, "Renamed");
    let saved = s.pack_for_save().unwrap();
    let again = unpack(57, &saved).unwrap();
    for o in &before {
        let after = again
            .operations(m)
            .unwrap()
            .into_iter()
            .find(|x| x.id == o.id)
            .unwrap();
        if o.settings.as_ref().is_some_and(Value::is_string) {
            assert_eq!(
                after.settings, o.settings,
                "a damaged register is kept as it is"
            );
        }
    }
}

#[test]
fn c4_edit_settings_on_an_own_operation_after_open_is_one_edit_step() {
    let (mut s, _, _, ops) = fresh(&["cut"]);
    let id = ops[0];
    let before = s.undo_len();
    s.edit_operation_setting(id, "main", "power", 22.0).unwrap();
    assert_eq!(s.undo_len(), before + 1);
    assert_eq!(undo_name(&mut s), "Edit settings");
    assert_eq!(main_run(&s, id).0, 22.0);
}

// ---- criterion 27 and the row: never empty, never "No settings" ------------------------------

#[test]
fn c27_no_row_of_a_laser_page_reads_no_settings_after_a_mix_of_kinds() {
    let (s, _, _, ops) = fresh(&["cut", "score", "engrave", "engrave_outline", "cut_out"]);
    assert!(!no_settings_text(&s));
    for id in ops {
        let (power, speed) = main_run(&s, id);
        assert!(power > 0.0 && speed > 0, "a number on every row");
    }
}

// ---- criteria 23, 26: the export is not blocked by numbers, and default equals typed ----------

#[test]
fn c23_a_cut_on_default_numbers_is_not_blocked_by_the_export_gate() {
    let (mut s, _, _, _) = fresh(&[]);
    let _ = s.press_operation_kind("cut", false).unwrap();
    assert!(
        s.export_job("Untitled.curvyo").is_some(),
        "no has no power and speed block"
    );
}

#[test]
fn c26_the_gcode_of_default_numbers_equals_the_gcode_of_typed_numbers() {
    let (mut own_default, _, _, _) = fresh(&[]);
    let _ = own_default.press_operation_kind("cut", false).unwrap();
    let default_bytes = own_default
        .export_job("Untitled.curvyo")
        .expect("export allowed")
        .bytes;

    let (mut typed, _, _, _) = fresh(&[]);
    let id = typed.press_operation_kind("cut", false).unwrap();
    typed
        .edit_operation_setting(id, "main", "power", 10.0)
        .unwrap();
    typed
        .edit_operation_setting(id, "main", "speed", 1000.0)
        .unwrap();
    let typed_bytes = typed
        .export_job("Untitled.curvyo")
        .expect("export allowed")
        .bytes;

    assert_ne!(default_bytes.len(), 0);
    assert_eq!(
        default_bytes, typed_bytes,
        "the job file of default numbers is the same as typed numbers"
    );
}

// ---- criteria 5, 6: the interval and the Image kind keep their own defaults -------------------

fn stored_op(s: &mut Session, m: NodeId, id: NodeId) -> OperationSnapshot {
    let bytes = s.pack("0150-tester").unwrap();
    unpack(9, &bytes)
        .unwrap()
        .operations(m)
        .unwrap()
        .into_iter()
        .find(|o| o.id == id)
        .unwrap()
}

#[test]
fn c5_a_new_engrave_keeps_its_interval_default_and_writes_no_options() {
    let (mut s, _, m, _) = fresh(&[]);
    let id = s.press_operation_kind("engrave", false).unwrap();
    assert_eq!(
        main_run(&s, id),
        (10.0, 1_000),
        "the numbers come from the defaults"
    );
    assert_eq!(
        row(&s, id)["settings"]["interval_mm"],
        0.1,
        "the interval keeps 0.10"
    );
    assert_eq!(
        main_run(&s, id).0,
        10.0,
        "the interval does not touch power"
    );
    assert!(
        stored_op(&mut s, m, id).options.is_none(),
        "no op_options are added by the story"
    );
}

#[test]
fn c6_a_new_image_takes_power_and_speed_from_the_defaults() {
    let (mut s, _, _, _) = fresh(&[]);
    let id = s
        .press_operation_kind("image", false)
        .expect("an Image is added");
    assert_eq!(main_run(&s, id), (10.0, 1_000));
    assert_eq!(row(&s, id)["settings"]["state"], "own");
    assert!(!no_settings_text(&s));
}

// ---- criterion 24: damaged settings are the built warning and run nothing -------------------

#[test]
fn c24_damaged_settings_report_the_built_warning_and_run_nothing() {
    let mut s = Session::open(4, &fixture("laser_settings_damaged_v43.curvyo")).unwrap();
    s.resize_viewport(1600.0, 1000.0);
    let page = unpack(2, &fixture("laser_settings_damaged_v43.curvyo"))
        .unwrap()
        .manufacturings()[0]
        .page
        .as_ref()
        .unwrap()
        .id
        .clone();
    assert!(s.activate_page(&page));
    let v: Value = serde_json::from_str(&s.operations_view_json()).unwrap();
    let damaged: Vec<&Value> = v["rows"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["settings"]["state"] == "damaged")
        .collect();
    assert!(!damaged.is_empty(), "the golden has damaged rows");
    for r in &damaged {
        assert!(r["run"].is_null(), "a damaged row runs nothing");
    }
    let out: Value = serde_json::from_str(&s.output_group_json()).unwrap();
    let warns: Vec<String> = out["checks"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["severity"] == "warn")
        .map(|c| c["text"].as_str().unwrap().to_owned())
        .collect();
    let want = format!(
        "{} operations have damaged settings and do not run.",
        damaged.len()
    );
    assert!(warns.contains(&want), "warning {want:?} in {warns:?}");
    let blocks: Vec<String> = out["checks"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["severity"] == "block")
        .map(|c| c["text"].as_str().unwrap().to_owned())
        .collect();
    assert!(
        blocks.iter().all(|t| !t.contains("damaged")),
        "damaged settings is a warning, not a block: {blocks:?}"
    );
}

// ---- criterion 25: the frame of an operation-only job is the typed-numbers frame -----------

#[test]
fn c25_the_frame_of_default_numbers_equals_the_frame_of_typed_numbers() {
    let (mut default, _, _, _) = fresh(&[]);
    let _ = default.press_operation_kind("cut", false).unwrap();
    let (mut typed, _, _, _) = fresh(&[]);
    let id = typed.press_operation_kind("cut", false).unwrap();
    typed
        .edit_operation_setting(id, "main", "power", 10.0)
        .unwrap();
    typed
        .edit_operation_setting(id, "main", "speed", 1000.0)
        .unwrap();
    assert_eq!(
        format!("{:?}", default.frame_draw_list()),
        format!("{:?}", typed.frame_draw_list()),
        "the frame traces the same outline"
    );
}

// ---- criterion 21: an old History step shows the derived numbers, adds no step -------------

#[test]
fn c21_restoring_an_old_step_without_numbers_shows_the_derived_pair_and_adds_no_step() {
    // A document whose first operation was added with no numbers (format 58, no op_settings).
    let d = Document::new(1);
    let made = d
        .add_output_page(NewOutputPage {
            process: LASER,
            label: "Laser",
            profile: None,
            work_area: DocumentSize::from_mm(400.0, 300.0),
        })
        .unwrap();
    let m = made.kind.manufacturing().unwrap();
    let op = d
        .add_operation(
            m,
            &NewOperation {
                kind: "cut",
                name: "Cut 1",
                settings: None,
            },
            0,
            &[],
        )
        .unwrap();
    let _later = d
        .add_operation(
            m,
            &NewOperation {
                kind: "score",
                name: "Score 1",
                settings: None,
            },
            1,
            &[],
        )
        .unwrap();
    let mut s = Session::open(5, &pack(&d, "0150-tester").unwrap()).unwrap();
    s.resize_viewport(1600.0, 1000.0);
    assert!(s.activate_page(&made.id));
    assert_eq!(
        s.history_len(),
        3,
        "opening adds no step (the fill is derived, not written)"
    );
    assert_eq!(
        main_run(&s, op),
        (10.0, 1_000),
        "the derived pair before any restore"
    );
    let _ = s.apply_history_item(false, 1);
    assert_eq!(
        main_run(&s, op),
        (10.0, 1_000),
        "the restored step shows the derived pair"
    );
    assert_eq!(row(&s, op)["settings"]["state"], "own");
}

// ---- criterion 20: one step, and Ctrl+Z after an edit gives back the derived numbers ---------

#[test]
fn c20_one_add_step_then_edit_then_undo_gives_the_derived_numbers_not_an_empty_state() {
    let (mut s, _, _, _) = fresh(&[]);
    let before = s.undo_len();
    let id = s.press_operation_kind("cut", false).unwrap();
    assert_eq!(s.undo_len(), before + 1, "the add is one step");
    s.edit_operation_setting(id, "main", "power", 33.0).unwrap();
    assert_eq!(undo_name(&mut s), "Edit settings");
    s.undo();
    assert_eq!(
        main_run(&s, id),
        (10.0, 1_000),
        "the derived numbers, never empty"
    );
    assert_eq!(row(&s, id)["settings"]["state"], "own");
}
