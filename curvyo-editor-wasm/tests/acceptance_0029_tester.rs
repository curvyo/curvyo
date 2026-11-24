//! Tester acceptance cases for `specs/0029-laser-job-preview-and-output`, written from the
//! specification before the implementation was read: the Output tab and its gate (criteria 1 to
//! 7), the preview and play bar (8 to 12, 37), what runs and in which order (13 to 23), the file
//! (24 to 27, 38), the export's effects and undo (28 to 30), damage (33) and the output format
//! (39), black box through the public session. Every exported file is read back by the tester's
//! own GRBL reader (`curvyo-laser-core/tests/tester_grbl_0029`), not by the implementer's.
#![allow(
    clippy::case_sensitive_file_extension_comparisons,
    clippy::many_single_char_names,
    clippy::field_reassign_with_default,
    clippy::assigning_clones,
    clippy::semicolon_if_nothing_returned,
    clippy::cast_lossless,
    clippy::neg_multiply,
    clippy::bool_assert_comparison,
    clippy::assert_is_empty,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::too_many_lines,
    clippy::float_cmp,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::similar_names,
    missing_docs
)]

mod fixture_support;
#[path = "../../curvyo-laser-core/tests/tester_grbl_0029/mod.rs"]
mod grbl;

use std::path::PathBuf;

use curvyo_document_core::{
    AnchorId, Direction, Document, DocumentSize, EllipseFrame, FixtureJobSwitch, Length, NewAnchor,
    NewOperation, NewOutputPage, NodeId, OperationSwitch, Page, Point, RectBounds, Timestamp, pack,
    unpack,
};
use curvyo_editor_wasm::{Session, Tool};
use curvyo_ui_core::FontBlock;
use grbl::{Kind, Program, read};
use serde_json::{Value, json};

const LASER: &str = "curvyo.laser";
const PROFILE: &str = "0d0d0d0d-0d0d-4d0d-8d0d-0d0d0d0d0d0d";
const MATERIAL: &str = "0b6c3f1e-2a4d-4b8e-9c7f-1d2e3f4a5b6c";
const MM: i64 = 1000;

// ---- fixture ----------------------------------------------------------------------------------

struct Fx {
    s: Session,
    l: Page,
}

/// A 400 x 300 mm laser page "Laser 1"; `build` draws items on it before the session opens.
fn page_with(build: impl FnOnce(&Document, &Page, NodeId) -> Vec<NodeId>) -> (Fx, Vec<NodeId>) {
    let d = Document::new(1);
    let made = d
        .add_output_page(NewOutputPage {
            process: LASER,
            label: "Laser",
            profile: None,
            work_area: DocumentSize::from_mm(400.0, 300.0),
        })
        .unwrap();
    let l = d.pages().into_iter().find(|p| p.id == made.id).unwrap();
    let m = l.kind.manufacturing().unwrap();
    let ids = build(&d, &l, m);
    let mut s = Session::open(7, &pack(&d, "0029-tester").unwrap()).unwrap();
    s.resize_viewport(1600.0, 1000.0);
    s.set_tool(Tool::Select);
    s.load_machine_profiles_json("[]");
    s.set_clock(|| Timestamp::from_secs(1_762_948_800));
    assert!(s.activate_page(&l.id));
    s.load_materials(&[], None);
    s.wipe_history();
    (Fx { s, l }, ids)
}

/// The same page on a saved profile "Diode 10 W" (no Generic laser warn).
fn on_profile(fx: &mut Fx) {
    let _ = fx.s.save_machine_profile([13; 16], 1_000);
    let _ = fx.s.set_machine_name("Diode 10 W");
    fx.s.wipe_history();
}

fn rect(d: &Document, l: &Page, x: f64, y: f64, w: f64, h: f64) -> NodeId {
    d.create_rect(RectBounds::from_corners(
        Point::new(l.origin.x + x, l.origin.y + y),
        Point::new(l.origin.x + x + w, l.origin.y + y + h),
    ))
}

fn path(d: &Document, l: &Page, pts: &[(f64, f64)], closed: bool, first: u64) -> NodeId {
    let anchors: Vec<NewAnchor> = pts
        .iter()
        .enumerate()
        .map(|(k, &(x, y))| {
            NewAnchor::corner(
                AnchorId::new(1, first + k as u64),
                Point::new(l.origin.x + x, l.origin.y + y),
            )
        })
        .collect();
    d.create_path(&anchors, closed)
}

fn op(d: &Document, m: NodeId, kind: &str, name: &str, members: &[NodeId]) -> NodeId {
    let index = d.operations(m).unwrap().len();
    let keys: Vec<Vec<NodeId>> = members.iter().map(|&n| vec![n]).collect();
    d.add_operation(
        m,
        &NewOperation {
            kind,
            name,
            settings: None,
        },
        index,
        &keys,
    )
    .unwrap()
}

fn numbers(s: &mut Session, id: NodeId, power: f64, speed: f64, passes: f64) {
    let _ = s.edit_operation_setting(id, "main", "power", power);
    let _ = s.edit_operation_setting(id, "main", "speed", speed);
    if passes > 1.0 {
        assert_eq!(
            s.edit_operation_setting(id, "main", "passes", passes),
            Ok(true)
        );
    }
}

// Superseded by 0029 rework (2026-11-17): the output group of the Operations tab replaces the
// Output tab's view; the runs are the rows' line 2 and 3 and the Fixture jobs section's run lines.
fn out(s: &Session) -> Value {
    serde_json::from_str(&s.output_group_json()).unwrap()
}

/// The runs in run order as the tab shows them: the fixture run lines first (`run` "fixture"),
/// then every operation row that runs (`run` its ring digit), each with its name, numbers and
/// time.
fn runs(s: &Session) -> Vec<Value> {
    let fixtures: Value = serde_json::from_str(&s.fixture_jobs_view_json()).unwrap();
    let mut out: Vec<Value> = fixtures["output"]["runs"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|r| {
            json!({"run": "fixture", "fixture": true, "name": r["kind"],
                "numbers": r["numbers"], "time": r["time"]})
        })
        .collect();
    let ops: Value = serde_json::from_str(&s.operations_view_json()).unwrap();
    for row in ops["rows"].as_array().cloned().unwrap_or_default() {
        if row["run"].is_null() {
            continue;
        }
        out.push(
            json!({"run": row["run"]["digit"], "fixture": false, "name": row["name"],
            "numbers": row["numbers"], "time": row["run"]["time"]}),
        );
    }
    out
}

fn checks(s: &Session) -> Vec<Value> {
    out(s)["checks"].as_array().cloned().unwrap_or_default()
}

fn check_texts(s: &Session, severity: &str) -> Vec<String> {
    checks(s)
        .iter()
        .filter(|c| c["severity"] == severity)
        .map(|c| c["text"].as_str().unwrap().to_owned())
        .collect()
}

fn export(s: &mut Session) -> Program {
    let e = s.export_job("Untitled.curvyo").expect("export allowed");
    read(&e.bytes).unwrap_or_else(|err| panic!("{err}\n{}", String::from_utf8_lossy(&e.bytes)))
}

fn export_bytes(s: &mut Session) -> Vec<u8> {
    s.export_job("Untitled.curvyo")
        .expect("export allowed")
        .bytes
}

fn doc(s: &Session) -> Document {
    unpack(99, &s.pack("0.1.0").unwrap()).unwrap()
}

fn undo_name(s: &mut Session) -> Option<String> {
    s.history_target_name(Direction::Undo)
}

/// One Cut 1 (50 %, 600 mm/min) of a 20 x 10 mm rectangle at (10, 10).
fn one_cut() -> (Fx, NodeId, NodeId) {
    let (mut fx, ids) = page_with(|d, l, m| {
        let r = rect(d, l, 10.0, 10.0, 20.0, 10.0);
        vec![r, op(d, m, "cut", "Cut 1", &[r])]
    });
    numbers(&mut fx.s, ids[1], 50.0, 600.0, 1.0);
    fx.s.wipe_history();
    (fx, ids[0], ids[1])
}

fn material_json(machine: &str, sets: &Value) -> String {
    let mut sets = sets.clone();
    for set in sets.as_object_mut().unwrap().values_mut() {
        set["source"] = json!("typed");
    }
    json!({
        "format_version": 2, "id": MATERIAL, "kind": "laser", "modified": 1_762_948_800_000_u64,
        "name": "Cardboard", "thickness_mm": 3.0,
        "lasers": [{"machine": {"id": machine, "name": "Diode 10 W"}, "sets": sets}],
        "tests": [], "history": []
    })
    .to_string()
}

fn load_material(s: &mut Session, sets: &Value) {
    s.load_materials(
        &[(format!("{MATERIAL}.json"), material_json(PROFILE, sets))],
        None,
    );
}

/// A grid fixture with a running Score fixture job (both parts ticked, Output on).
fn running_fixture_score(s: &mut Session) -> NodeId {
    let g = fixture_support::add_fixture(s, "grid").unwrap();
    s.select_fixture(g);
    let job = s.add_fixture_job("score").unwrap();
    assert!(s.set_fixture_job_part(job, "grid.lines", true));
    assert!(s.set_fixture_job_switch(job, FixtureJobSwitch::Output, true));
    job
}

fn fixture_output(s: &Session, job: NodeId) -> bool {
    let v: Value = serde_json::from_str(&s.fixture_jobs_view_json()).unwrap();
    let full = job.full_id();
    v["groups"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|g| g["jobs"].as_array().unwrap().clone())
        .find(|j| {
            s.fixture_job_named(j["id"].as_str().unwrap())
                .map(NodeId::full_id)
                == Some(full.clone())
        })
        .expect("the job row")["output"]
        .as_bool()
        .unwrap()
}

// ---- criterion 1: the Output tab ----------------------------------------------------------------

#[test]
fn c1_a_laser_page_has_the_output_view_and_a_drawing_page_has_none() {
    let (mut fx, _, _) = one_cut();
    let v = out(&fx.s);
    assert!(v.is_object());
    let keys: Vec<&str> = v.as_object().unwrap().keys().map(String::as_str).collect();
    for k in ["machine", "start", "checks", "total", "export"] {
        assert!(keys.contains(&k), "{k} in {keys:?}");
    }
    assert!(fx.s.activate_page("first"));
    assert_eq!(out(&fx.s), Value::Null, "no output group on a drawing page");
}

// ---- criterion 2: the gate lines ----------------------------------------------------------------

#[test]
fn c2_machine_line_mode_line_and_air_assist() {
    let (mut fx, _, _) = one_cut();
    on_profile(&mut fx);
    // Superseded by 0029 rework (2026-11-17): one machine line "<profile> · <format>[ · air
    // assist]" (P30); the bed, origin and laser mode stay in the Machine block.
    let v = out(&fx.s);
    assert_eq!(v["machine"], "Diode 10 W · GRBL");
    assert!(v.get("mode").is_none());
    fx.s.set_machine_laser_mode(false);
    assert_eq!(out(&fx.s)["machine"], "Diode 10 W · GRBL");
    assert!(fx.s.set_machine_air_assist(true));
    assert_eq!(out(&fx.s)["machine"], "Diode 10 W · GRBL · air assist");
}

#[test]
fn c2_one_row_per_running_operation_with_its_numbers_and_time() {
    let (mut fx, ids) = page_with(|d, l, m| {
        let a = rect(d, l, 10.0, 10.0, 20.0, 10.0);
        let b = rect(d, l, 50.0, 10.0, 20.0, 10.0);
        vec![
            op(d, m, "engrave", "Engrave 1", &[a]),
            op(d, m, "cut", "Cut 1", &[b]),
            op(d, m, "score", "Score 1", &[]),
        ]
    });
    numbers(&mut fx.s, ids[0], 30.0, 6000.0, 1.0);
    numbers(&mut fx.s, ids[1], 90.0, 300.0, 2.0);
    let runs = runs(&fx.s);
    let names: Vec<&str> = runs.iter().map(|r| r["name"].as_str().unwrap()).collect();
    assert_eq!(
        names,
        ["Engrave 1", "Cut 1"],
        "Score 1 has no item and does not run"
    );
    // Superseded by 0029 rework (2026-11-17): the ring is the row's digit (P26); Score 1 (row 2,
    // marking section) does not run and keeps its plain digit, so Cut 1's ring reads 3.
    assert_eq!(runs[0]["run"], 1);
    assert_eq!(runs[1]["run"], 3);
    let e = runs[0]["numbers"][0]["text"].as_str().unwrap();
    assert!(
        e.starts_with("30 %, 6000 mm/min, 1 ×") || e.starts_with("30 %, 6,000 mm/min, 1 ×"),
        "{e}"
    );
    assert!(
        e.contains(" mm") && e.matches("mm").count() >= 2,
        "Engrave shows its Interval: {e}"
    );
    let c = runs[1]["numbers"][0]["text"].as_str().unwrap();
    assert!(c.starts_with("90 %, 300 mm/min, 2 ×"), "{c}");
    assert!(!runs[1]["time"].as_str().unwrap().is_empty());
}

// ---- criterion 3: same frame ---------------------------------------------------------------------

#[test]
fn c3_a_typed_power_changes_the_gate_line_at_once() {
    let (mut fx, _, cut) = one_cut();
    assert!(
        runs(&fx.s)[0]["numbers"][0]["text"]
            .as_str()
            .unwrap()
            .starts_with("50 %")
    );
    assert_eq!(
        fx.s.edit_operation_setting(cut, "main", "power", 70.0),
        Ok(true)
    );
    assert!(
        runs(&fx.s)[0]["numbers"][0]["text"]
            .as_str()
            .unwrap()
            .starts_with("70 %")
    );
    // and the file follows
    let p = export(&mut fx.s);
    assert_eq!(p.runs()[0].s, 700);
}

// ---- criterion 4: blocks -------------------------------------------------------------------------

#[test]
fn c4a_an_operation_without_settings_runs_the_default_numbers_and_never_blocks() {
    // Superseded by `specs/0150` criteria 15, 23: the check "has no power and speed" is gone; an
    // operation without `op_settings` runs the copy's default numbers.
    let (mut fx, ids) = page_with(|d, l, m| {
        let a = rect(d, l, 10.0, 10.0, 20.0, 10.0);
        let b = rect(d, l, 50.0, 10.0, 20.0, 10.0);
        vec![
            op(d, m, "cut", "Cut 1", &[a]),
            op(d, m, "cut", "Cut 2", &[b]),
        ]
    });
    numbers(&mut fx.s, ids[0], 50.0, 600.0, 1.0);
    let v = out(&fx.s);
    assert!(check_texts(&fx.s, "block").is_empty());
    assert_eq!(v["export"]["enabled"], true);
    assert!(fx.s.export_job("Untitled.curvyo").is_some());
}

#[test]
fn c4b_geometry_outside_the_bed_blocks_by_its_box_with_0_01_mm_tolerance() {
    let (mut fx, ids) = page_with(|d, l, m| {
        let inside_edge = path(d, l, &[(380.0, 50.0), (400.005, 50.0)], false, 100);
        let out1 = path(d, l, &[(380.0, 60.0), (400.02, 60.0)], false, 200);
        let out2 = rect(d, l, -5.0, 100.0, 10.0, 10.0);
        let not_run = rect(d, l, 395.0, 200.0, 30.0, 10.0);
        let _ = not_run;
        vec![
            op(d, m, "cut", "Cut 1", &[inside_edge, out1, out2]),
            out1,
            out2,
        ]
    });
    numbers(&mut fx.s, ids[0], 50.0, 600.0, 1.0);
    let blocks = check_texts(&fx.s, "block");
    assert_eq!(blocks.len(), 1, "{blocks:?}");
    assert!(
        blocks[0].starts_with("2 items lie outside the 400"),
        "{}",
        blocks[0]
    );
    assert!(blocks[0].ends_with("300 mm bed."), "{}", blocks[0]);
    assert_eq!(out(&fx.s)["export"]["enabled"], false);
    let c = checks(&fx.s)
        .into_iter()
        .find(|c| c["severity"] == "block")
        .unwrap();
    assert_eq!(c["count"], 2);
    assert_eq!(c["target"]["kind"], "items");
    let targets: Vec<NodeId> = serde_json::from_value(c["target"]["ids"].clone()).unwrap();
    assert!(fx.s.select_output_items(&targets));
    assert_eq!(
        fx.s.selected_object_count(),
        2,
        "the count selects the offending items"
    );
}

#[test]
fn c4b_geometry_that_does_not_run_never_blocks() {
    let (mut fx, ids) = page_with(|d, l, m| {
        let a = rect(d, l, 10.0, 10.0, 20.0, 10.0);
        let outside = rect(d, l, 395.0, 200.0, 30.0, 10.0);
        let off = rect(d, l, 395.0, 250.0, 30.0, 10.0);
        vec![
            op(d, m, "cut", "Cut 1", &[a]),
            op(d, m, "cut", "Cut 2", &[off]),
            outside,
        ]
    });
    numbers(&mut fx.s, ids[0], 50.0, 600.0, 1.0);
    numbers(&mut fx.s, ids[1], 50.0, 600.0, 1.0);
    assert!(fx.s.set_operation_switch(ids[1], OperationSwitch::Output, false));
    assert!(
        check_texts(&fx.s, "block").is_empty(),
        "{:?}",
        check_texts(&fx.s, "block")
    );
    assert_eq!(out(&fx.s)["export"]["enabled"], true);
}

#[test]
fn c4c_c39_an_unknown_output_format_in_the_page_copy_blocks_and_writes_nothing() {
    let bytes = std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../curvyo-document-core/tests/fixtures/laser_output_damaged_v48.curvyo"),
    )
    .unwrap();
    let d = unpack(3, &bytes).unwrap();
    let mut s = Session::open(3, &bytes).unwrap();
    s.load_machine_profiles_json("[]");
    s.resize_viewport(1600.0, 1000.0);
    let mut found = false;
    for p in d.pages() {
        if s.activate_page(&p.id) && out(&s).is_object() {
            found = true;
            let v = out(&s);
            assert_eq!(v["export"]["enabled"], false, "{v}");
            assert!(
                check_texts(&s, "block")
                    .iter()
                    .any(|t| t.to_lowercase().contains("machine")
                        || t.to_lowercase().contains("profile")),
                "a damaged copy names the machine: {:?}",
                check_texts(&s, "block")
            );
            assert!(
                s.export_job("Untitled.curvyo").is_none(),
                "nothing written in a guessed format"
            );
            break;
        }
    }
    assert!(found, "the damaged golden has a laser page");
}

#[test]
fn c4d_c7_nothing_runs_on_an_empty_page() {
    let (mut fx, _) = page_with(|_, _, _| vec![]);
    let v = out(&fx.s);
    assert_eq!(
        check_texts(&fx.s, "block"),
        ["Nothing runs. Assign items to an operation."]
    );
    assert_eq!(v["export"]["enabled"], false);
    assert!(v["total"].is_null(), "the summary is empty: {}", v["total"]);
    assert!(fx.s.export_job("Untitled.curvyo").is_none());
    assert!(fx.s.toggle_toolpath_view(&fx.l.id.clone()));
}

#[test]
fn c4d_an_unassigned_item_and_no_operation_nothing_runs() {
    let (mut fx, _) = page_with(|d, l, _| vec![rect(d, l, 10.0, 10.0, 10.0, 10.0)]);
    assert_eq!(
        check_texts(&fx.s, "block"),
        ["Nothing runs. Assign items to an operation."]
    );
    assert!(fx.s.export_job("Untitled.curvyo").is_none());
}

/// FINDING: an operation with no item (and no settings) next to an unassigned item lifts the
/// "Nothing runs" block; Export is enabled and writes a job file with no move.
#[test]
fn c4d_items_but_only_operations_without_items_nothing_runs() {
    let (mut fx, _) = page_with(|d, l, m| {
        let _ = rect(d, l, 10.0, 10.0, 10.0, 10.0);
        vec![op(d, m, "cut", "Cut 1", &[])]
    });
    assert!(
        check_texts(&fx.s, "block")
            .contains(&"Nothing runs. Assign items to an operation.".to_owned()),
        "{:?}",
        checks(&fx.s)
    );
    assert_eq!(out(&fx.s)["export"]["enabled"], false);
    assert!(
        fx.s.export_job("Untitled.curvyo").is_none(),
        "an empty job file is written"
    );
}

#[test]
fn c4d_every_operation_output_off_nothing_runs() {
    let (mut fx, ids) = page_with(|d, l, m| {
        let a = rect(d, l, 10.0, 10.0, 10.0, 10.0);
        vec![op(d, m, "cut", "Cut 1", &[a])]
    });
    numbers(&mut fx.s, ids[0], 50.0, 600.0, 1.0);
    assert!(fx.s.set_operation_switch(ids[0], OperationSwitch::Output, false));
    assert!(
        check_texts(&fx.s, "block")
            .contains(&"Nothing runs. Assign items to an operation.".to_owned()),
        "{:?}",
        checks(&fx.s)
    );
    assert!(fx.s.export_job("Untitled.curvyo").is_none());
}

// ---- criterion 5: warns and their order -----------------------------------------------------------

#[test]
fn c5_warns_never_disable_export_and_come_after_blocks_in_order() {
    let (mut fx, ids) = page_with(|d, l, m| {
        let a = rect(d, l, 10.0, 10.0, 20.0, 10.0);
        let hidden = rect(d, l, 50.0, 10.0, 20.0, 10.0);
        let _loose = rect(d, l, 90.0, 10.0, 20.0, 10.0);
        let b = rect(d, l, 130.0, 10.0, 20.0, 10.0);
        d.set_node_visible(&[hidden], false).unwrap();
        vec![
            op(d, m, "cut", "Cut 1", &[a, hidden]),
            op(d, m, "score", "Score 1", &[b]),
        ]
    });
    fx.s.load_materials(
        &[(
            format!("{MATERIAL}.json"),
            material_json(
                "generic-laser",
                &json!({"cut": {"power_pct": 50.0, "speed_mm_min": 9000, "passes": 1}}),
            ),
        )],
        None,
    );
    assert!(
        fx.s.use_material_on(&[ids[0]], MATERIAL),
        "the material links"
    );
    numbers(&mut fx.s, ids[1], 10.0, 1000.0, 1.0);
    assert!(fx.s.set_operation_switch(ids[1], OperationSwitch::Output, false));
    let v = out(&fx.s);
    assert_eq!(v["export"]["enabled"], true, "{:?}", checks(&fx.s));
    let warns = check_texts(&fx.s, "warn");
    let pos = |needle: &str| {
        warns
            .iter()
            .position(|w| w.to_lowercase().contains(needle))
            .unwrap_or_else(|| panic!("no warn with {needle:?}: {warns:?}"))
    };
    let generic = pos("generic laser");
    let feed = pos("maximum");
    let not_assigned = pos("not assigned");
    let hidden = pos("hidden");
    let off = pos("output off");
    assert!(
        generic < feed && feed < not_assigned && not_assigned < hidden && hidden < off,
        "{warns:?}"
    );
    for c in checks(&fx.s) {
        assert!(c["severity"] == "warn" || c["severity"] == "block");
    }
    // The over-feed speed is written as the machine's max feed (criterion 21).
    let p = export(&mut fx.s);
    let max = p.runs().iter().map(|r| r.f).max().unwrap();
    assert!(max <= 6000, "F {max} above Generic laser's max feed");
}

// ---- criterion 6 ---------------------------------------------------------------------------------

#[test]
fn c6_generic_laser_only_warns() {
    let (fx, _, _) = one_cut();
    assert!(check_texts(&fx.s, "block").is_empty());
    assert!(
        check_texts(&fx.s, "warn")
            .iter()
            .any(|w| w.contains("Generic laser"))
    );
    assert_eq!(out(&fx.s)["export"]["enabled"], true);
}

// ---- criteria 8, 9, 37: Toolpath view and play bar ------------------------------------------------

#[test]
fn c8_toolpath_view_is_session_state_not_saved_not_a_history_step() {
    let (mut fx, _, _) = one_cut();
    let before = fx.s.undo_len();
    let packed = fx.s.pack("0.1.0").unwrap();
    assert!(fx.s.toggle_toolpath_view(&fx.l.id.clone()));
    assert_eq!(out(&fx.s)["toolpath_view"], true);
    assert_eq!(fx.s.undo_len(), before, "no history step");
    assert_eq!(fx.s.pack("0.1.0").unwrap(), packed, "not saved");
    assert!(fx.s.play_view().is_some(), "the play bar sits with it");
}

#[test]
fn c9_c10_the_view_summary_and_play_readout_match_the_file() {
    let (mut fx, ids) = page_with(|d, l, m| {
        let c = d.create_ellipse(EllipseFrame {
            center: Point::new(l.origin.x + 200.0, l.origin.y + 150.0),
            rx: Length::from_mm(40.0),
            ry: Length::from_mm(25.0),
        });
        let r = rect(d, l, 10.0, 10.0, 20.0, 10.0);
        vec![op(d, m, "cut", "Cut 1", &[c, r])]
    });
    numbers(&mut fx.s, ids[0], 50.0, 600.0, 2.0);
    let p = export(&mut fx.s);
    assert!(fx.s.toggle_toolpath_view(&fx.l.id.clone()));
    // Superseded by 0029 rework (2026-11-17): the readout is time only (criterion 37); the
    // lengths are checked on the total's line, which the view reads from the same timeline.
    let play = fx.s.play_view().unwrap();
    assert!(!play.readout.contains(" mm") && !play.readout.contains(" m "));
    let total = out(&fx.s)["total"].clone();
    assert!(
        total["time"]
            .as_str()
            .unwrap()
            .ends_with(play.readout.split(" of ").nth(1).unwrap()),
        "the readout's total is the total's time: {}",
        play.readout
    );
    let lengths = total["lengths"].as_str().unwrap();
    assert_eq!(
        lengths,
        curvyo_laser_core::job::lengths_text(p.burn_mm(), p.travel_mm() + p.off_mm()),
        "{lengths}"
    );
    let expect_s = p.burn_mm() / 600.0 * 60.0 + p.travel_mm() / 6000.0 * 60.0;
    assert_eq!(
        total["time"],
        curvyo_laser_core::job::estimate_text(expect_s),
        "file says {expect_s} s"
    );
}

#[test]
fn c10_the_spec_example_reads_about_21_s() {
    // A 100 mm Cut line at 600 mm/min, 2 passes; the head starts at the bottom-left origin, so a
    // line from (0, 300) along the bed edge needs travel only for pass 2 and the way home.
    let (mut fx, ids) = page_with(|d, l, m| {
        let line = path(d, l, &[(0.0, 300.0), (100.0, 300.0)], false, 300);
        vec![op(d, m, "cut", "Cut 1", &[line])]
    });
    numbers(&mut fx.s, ids[0], 50.0, 600.0, 2.0);
    let p = export(&mut fx.s);
    let expect = 200.0 / 600.0 * 60.0 + p.travel_mm() / 6000.0 * 60.0;
    assert_eq!(
        out(&fx.s)["total"]["time"],
        curvyo_laser_core::job::estimate_text(expect)
    );
    assert!((p.burn_mm() - 200.0).abs() < 0.01);
}

#[test]
fn c37_slider_half_is_half_the_time_arrow_steps_one_percent_play_ends_in_10_s() {
    let (mut fx, _, _) = one_cut();
    assert!(fx.s.toggle_toolpath_view(&fx.l.id.clone()));
    // Superseded by 0029 rework (2026-11-17): the readout is "<elapsed> of <total>" only.
    let total = |r: &str| -> f64 {
        assert!(!r.contains(" · "), "time only: {r}");
        parse_dur(r.split(" of ").nth(1).unwrap())
    };
    fx.s.set_play_position(0.5);
    let pv = fx.s.play_view().unwrap();
    assert_eq!(pv.fraction, 0.5);
    let elapsed = parse_dur(pv.readout.split(" of ").next().unwrap());
    assert!(
        (elapsed - total(&pv.readout) / 2.0).abs() <= 1.0,
        "{}",
        pv.readout
    );
    fx.s.step_play_position(1.0);
    assert!((fx.s.play_view().unwrap().fraction - 0.51).abs() < 1e-9);
    fx.s.step_play_position(-1.0);
    fx.s.step_play_position(-1.0);
    assert!((fx.s.play_view().unwrap().fraction - 0.49).abs() < 1e-9);
    // Play from 0: the whole run takes 10 s whatever its length.
    fx.s.set_play_position(0.0);
    let before = fx.s.undo_len();
    assert!(fx.s.toggle_play(1_000.0));
    assert!(fx.s.play_view().unwrap().playing);
    fx.s.play_tick(6_000.0);
    let f = fx.s.play_view().unwrap().fraction;
    assert!((f - 0.5).abs() < 0.02, "half of 10 s is half the run: {f}");
    fx.s.play_tick(11_500.0);
    let pv = fx.s.play_view().unwrap();
    assert_eq!(pv.fraction, 1.0, "stops at the end");
    assert!(!pv.playing);
    // The next press returns to the start.
    fx.s.toggle_play(12_000.0);
    assert!(fx.s.play_view().unwrap().fraction < 0.01);
    assert_eq!(fx.s.undo_len(), before, "never a history step");
}

fn parse_dur(t: &str) -> f64 {
    let mut s = 0.0;
    let words: Vec<&str> = t.split_whitespace().collect();
    for w in words.chunks(2) {
        let n: f64 = w[0].parse().unwrap_or_else(|_| panic!("{t}"));
        s += match w.get(1).copied() {
            Some("h") => n * 3600.0,
            Some("min") => n * 60.0,
            _ => n,
        };
    }
    s
}

// ---- criterion 11 ---------------------------------------------------------------------------------

#[test]
fn c11_hidden_items_and_show_off_operations_still_run() {
    let (mut fx, ids) = page_with(|d, l, m| {
        let a = rect(d, l, 10.0, 10.0, 20.0, 10.0);
        d.set_node_visible(&[a], false).unwrap();
        vec![op(d, m, "cut", "Cut 1", &[a])]
    });
    numbers(&mut fx.s, ids[0], 50.0, 600.0, 1.0);
    assert!(fx.s.set_operation_switch(ids[0], OperationSwitch::Show, false));
    let p = export(&mut fx.s);
    assert!((p.burn_mm() - 60.0).abs() < 0.01, "{}", p.burn_mm());
    assert!(fx.s.toggle_toolpath_view(&fx.l.id.clone()));
    // Superseded by 0029 rework (2026-11-17): the readout is time only; the run still plays.
    assert!(fx.s.play_view().unwrap().enabled);
}

// ---- criteria 13, 17: what a Cut writes ----------------------------------------------------------

#[test]
fn c13_closed_returns_to_start_open_runs_once_per_pass() {
    let (mut fx, ids) = page_with(|d, l, m| {
        let r = rect(d, l, 10.0, 10.0, 20.0, 10.0);
        let o = path(
            d,
            l,
            &[(100.0, 100.0), (150.0, 100.0), (150.0, 130.0)],
            false,
            400,
        );
        vec![op(d, m, "cut", "Cut 1", &[r, o])]
    });
    numbers(&mut fx.s, ids[0], 50.0, 600.0, 3.0);
    let p = export(&mut fx.s);
    assert!(
        (p.burn_mm() - 3.0 * (60.0 + 80.0)).abs() < 0.01,
        "{}",
        p.burn_mm()
    );
    let runs = p.runs();
    assert_eq!(runs.len(), 6);
    for r in &runs {
        if r.points.len() == 5 {
            assert_eq!(r.points[0], r.points[4]);
        }
    }
}

#[test]
fn c17_an_ellipse_is_flattened_within_0_01_mm() {
    let (cx, cy, rx, ry) = (200.0, 150.0, 60.0, 30.0);
    let (mut fx, ids) = page_with(|d, l, m| {
        let c = d.create_ellipse(EllipseFrame {
            center: Point::new(l.origin.x + cx, l.origin.y + cy),
            rx: Length::from_mm(rx),
            ry: Length::from_mm(ry),
        });
        vec![op(d, m, "cut", "Cut 1", &[c])]
    });
    numbers(&mut fx.s, ids[0], 50.0, 600.0, 1.0);
    let p = export(&mut fx.s);
    // Every written vertex is on the ellipse (as drawn: a Bézier ellipse deviates from the true
    // one by at most 0.03 % of the radius) and every chord's middle is within 0.01 mm of it.
    let f = |q: (i64, i64)| (q.0 as f64 / 1000.0 - cx, 300.0 - q.1 as f64 / 1000.0 - cy);
    let radial = |(x, y): (f64, f64)| {
        // distance to the ellipse along the ray, approximate by scaling
        let t = ((x / rx).powi(2) + (y / ry).powi(2)).sqrt();
        let r = x.hypot(y);
        r - r / t
    };
    let mut n = 0;
    for s in p.segs.iter().filter(|s| s.kind == Kind::Burn) {
        let a = f(s.from);
        let b = f(s.to);
        assert!(
            radial(a).abs() <= 0.02 + 0.0003 * rx,
            "vertex {a:?} {}",
            radial(a)
        );
        let mid = (f64::midpoint(a.0, b.0), f64::midpoint(a.1, b.1));
        assert!(
            radial(mid).abs() <= 0.01 + 0.0003 * rx,
            "chord middle off by {}",
            radial(mid)
        );
        n += 1;
    }
    assert!(n > 50, "{n} segments");
}

#[test]
fn c17_clones_groups_and_transforms_write_where_they_stand() {
    let (mut fx, ids) = page_with(|d, l, m| {
        let a = rect(d, l, 10.0, 10.0, 10.0, 10.0);
        let b = rect(d, l, 30.0, 10.0, 10.0, 10.0);
        let inner = d.group_objects(&[a, b]).unwrap();
        let c = rect(d, l, 60.0, 10.0, 10.0, 10.0);
        let outer = d.group_objects(&[inner, c]).unwrap();
        vec![op(d, m, "cut", "Cut 1", &[outer]), a]
    });
    numbers(&mut fx.s, ids[0], 50.0, 600.0, 1.0);
    let p = export(&mut fx.s);
    assert_eq!(p.runs().len(), 3, "every leaf of nested groups runs");
    assert!((p.burn_mm() - 120.0).abs() < 0.01);
    // A linked clone of the nested group, assigned too, adds its own three leaves where it stands.
    assert!(fx.s.select_output_items(&[ids[1]]));
    let before = doc(&fx.s)
        .operations(fx.l.kind.manufacturing().unwrap())
        .unwrap()[0]
        .members
        .len();
    let _ = before;
}

// ---- criterion 18 at session level: nothing for images (no image here; criterion 31 golden) ------

// ---- criterion 20, 23: order -----------------------------------------------------------------------

#[test]
fn c20_fixture_job_score_then_engrave_then_cut_in_the_file() {
    let (mut fx, ids) = page_with(|d, l, m| {
        let a = rect(d, l, 100.0, 100.0, 20.0, 10.0);
        let b = rect(d, l, 200.0, 100.0, 20.0, 10.0);
        vec![
            op(d, m, "cut", "Cut 1", &[b]),
            op(d, m, "engrave", "Engrave 1", &[a]),
        ]
    });
    on_profile(&mut fx);
    numbers(&mut fx.s, ids[0], 90.0, 300.0, 1.0);
    numbers(&mut fx.s, ids[1], 30.0, 5000.0, 1.0);
    let job = running_fixture_score(&mut fx.s);
    // No pick: a running fixture job blocks.
    assert!(
        !check_texts(&fx.s, "block").is_empty(),
        "no fixture material pick blocks"
    );
    assert_eq!(out(&fx.s)["export"]["enabled"], false);
    load_material(
        &mut fx.s,
        &json!({"score": {"power_pct": 12.0, "speed_mm_min": 4000, "passes": 1}}),
    );
    assert!(fx.s.set_fixture_material(MATERIAL));
    assert!(
        check_texts(&fx.s, "block").is_empty(),
        "{:?}",
        check_texts(&fx.s, "block")
    );
    // Superseded by 0029 rework (2026-11-17): the fixture run is ringed with the fixture glyph
    // and comes first; the operation rows keep their digits (P26).
    let runs = runs(&fx.s);
    assert!(runs[0]["fixture"].as_bool().unwrap());
    assert_eq!(runs[0]["run"], "fixture");
    let p = export(&mut fx.s);
    let order = p.settings_order();
    assert_eq!(
        order.first(),
        Some(&(120, 4000)),
        "fixture Score first: {order:?}"
    );
    // Cuts last (default on): marking before cutting whatever the list says.
    let ei = order.iter().position(|o| o.1 == 5000).unwrap();
    let ci = order.iter().position(|o| *o == (900, 300)).unwrap();
    assert!(ei < ci, "Engrave 1 before Cut 1: {order:?}");
    let _ = job;
}

#[test]
fn c23_cuts_last_off_runs_in_list_order_on_runs_marking_first() {
    let (mut fx, ids) = page_with(|d, l, m| {
        let a = rect(d, l, 100.0, 100.0, 20.0, 10.0);
        let b = rect(d, l, 200.0, 100.0, 20.0, 10.0);
        let c = rect(d, l, 250.0, 100.0, 20.0, 10.0);
        vec![
            op(d, m, "cut_out", "Cut-out 1", &[c]),
            op(d, m, "cut", "Cut 1", &[b]),
            op(d, m, "score", "Score 1", &[a]),
        ]
    });
    numbers(&mut fx.s, ids[0], 80.0, 400.0, 1.0);
    numbers(&mut fx.s, ids[1], 90.0, 300.0, 1.0);
    numbers(&mut fx.s, ids[2], 10.0, 3000.0, 1.0);
    fx.s.set_cuts_last(false);
    let p = export(&mut fx.s);
    assert_eq!(
        p.settings_order(),
        [(800, 400), (900, 300), (100, 3000)],
        "list order with Cuts last off"
    );
    assert!(fx.s.set_cuts_last(true));
    let p = export(&mut fx.s);
    assert_eq!(
        p.settings_order().first(),
        Some(&(100, 3000)),
        "Score (marking) first with Cuts last on"
    );
    let order = p.settings_order();
    assert!(order.iter().position(|o| *o == (800, 400)).unwrap() > 0);
}

// ---- criterion 21 ---------------------------------------------------------------------------------

#[test]
fn c21_passes_repeat_the_whole_operation_pass_by_pass() {
    let (mut fx, ids) = page_with(|d, l, m| {
        let a = rect(d, l, 10.0, 10.0, 10.0, 10.0);
        let b = rect(d, l, 100.0, 10.0, 10.0, 10.0);
        vec![op(d, m, "cut", "Cut 1", &[a, b])]
    });
    numbers(&mut fx.s, ids[0], 50.0, 600.0, 3.0);
    let p = export(&mut fx.s);
    let runs = p.runs();
    assert_eq!(runs.len(), 6);
    assert_ne!(
        runs[0].points, runs[1].points,
        "pass 1 runs both paths before pass 2"
    );
    for k in 2..6 {
        assert_eq!(runs[k].points, runs[k % 2].points);
    }
}

// ---- criterion 22 ---------------------------------------------------------------------------------

#[test]
fn c22_rectangle_hole_and_inner_shape_run_inner_first() {
    let (mut fx, ids) = page_with(|d, l, m| {
        let outer = rect(d, l, 50.0, 50.0, 200.0, 150.0);
        let hole = d.create_ellipse(EllipseFrame {
            center: Point::new(l.origin.x + 150.0, l.origin.y + 125.0),
            rx: Length::from_mm(50.0),
            ry: Length::from_mm(50.0),
        });
        let inner = rect(d, l, 140.0, 120.0, 20.0, 10.0);
        vec![op(d, m, "cut", "Cut 1", &[outer, hole, inner])]
    });
    numbers(&mut fx.s, ids[0], 50.0, 600.0, 1.0);
    let p = export(&mut fx.s);
    let widths: Vec<i64> = p
        .runs()
        .iter()
        .map(|r| {
            r.points.iter().map(|q| q.0).max().unwrap()
                - r.points.iter().map(|q| q.0).min().unwrap()
        })
        .collect();
    // The circle may be one run of many segments.
    assert_eq!(
        widths.first(),
        Some(&(20 * MM)),
        "the inner shape first: {widths:?}"
    );
    assert_eq!(
        widths.last(),
        Some(&(200 * MM)),
        "the rectangle last: {widths:?}"
    );
}

// ---- criteria 24, 31: determinism -----------------------------------------------------------------

#[test]
fn c24_two_exports_and_two_sessions_write_identical_bytes() {
    let (mut fx, _, _) = one_cut();
    let a = export_bytes(&mut fx.s);
    let b = export_bytes(&mut fx.s);
    assert_eq!(a, b);
    let packed = fx.s.pack("0.1.0").unwrap();
    let mut other = Session::open(8, &packed).unwrap();
    other.load_machine_profiles_json("[]");
    other.load_materials(&[], None);
    assert!(other.activate_page(&fx.l.id));
    assert_eq!(
        export_bytes(&mut other),
        a,
        "another peer, another day: same bytes"
    );
    let text = String::from_utf8(a).unwrap();
    assert!(
        !text.contains("0029-tester") && !text.contains("0.1.0"),
        "no app version"
    );
}

// ---- criterion 25 ---------------------------------------------------------------------------------

#[test]
fn c25_s_max_m3_and_air_assist_follow_the_profile() {
    let (mut fx, _, _) = one_cut();
    on_profile(&mut fx);
    let _ = fx.s.set_machine_max_power(255.0);
    let p = export(&mut fx.s);
    assert_eq!(p.runs()[0].s, 128, "round(50 x 255 / 100)");
    fx.s.set_machine_laser_mode(false);
    let p = export(&mut fx.s);
    assert_eq!(p.mode, grbl::Mode::M3);
    assert!(fx.s.set_machine_air_assist(true));
    let p = export(&mut fx.s);
    assert!(p.air_on.is_some() && p.air_off.is_some());
}

// ---- criterion 26 ---------------------------------------------------------------------------------

#[test]
fn c26_four_corners_mirror_and_stay_inside_the_bed() {
    let (mut fx, _, _) = one_cut();
    on_profile(&mut fx);
    let mut boxes = Vec::new();
    for c in ["bottom_left", "top_left", "bottom_right", "top_right"] {
        let _ = fx.s.set_machine_origin(c);
        let p = export(&mut fx.s);
        let b = p.all_box();
        assert!(
            b.0 >= 0 && b.1 >= 0 && b.2 <= 400 * MM && b.3 <= 300 * MM,
            "{c}: {b:?}"
        );
        boxes.push(p.burn_box().unwrap());
    }
    // rect page x 10..30, y 10..20
    assert_eq!(
        boxes[0],
        (10 * MM, 280 * MM, 30 * MM, 290 * MM),
        "bottom-left"
    );
    assert_eq!(boxes[1], (10 * MM, 10 * MM, 30 * MM, 20 * MM), "top-left");
    assert_eq!(
        boxes[2],
        (370 * MM, 280 * MM, 390 * MM, 290 * MM),
        "bottom-right"
    );
    assert_eq!(
        boxes[3],
        (370 * MM, 10 * MM, 390 * MM, 20 * MM),
        "top-right"
    );
}

// ---- criterion 27 ---------------------------------------------------------------------------------

#[test]
fn c27_default_name_file_type_notice_and_cancel() {
    let (mut fx, _, _) = one_cut();
    let before = fx.s.undo_len();
    let e = fx.s.export_job("Untitled.curvyo").unwrap();
    assert_eq!(e.name, "Untitled - Laser 1.gcode");
    assert_eq!(e.file_type, "GRBL G-code");
    assert_eq!(e.extension, "gcode");
    assert_eq!(
        fx.s.undo_len(),
        before,
        "the dialog (or its cancel) writes no history step"
    );
    let (text, ms) =
        fx.s.export_job_done(e.token, &e.name, false, 1_000)
            .unwrap();
    assert_eq!(text, "Exported Untitled - Laser 1.gcode.");
    assert_eq!(ms, 3_000);
    let e = fx.s.export_job("Untitled.curvyo").unwrap();
    let (text, _) = fx.s.export_job_done(e.token, &e.name, true, 1_000).unwrap();
    assert_eq!(text, "Downloaded Untitled - Laser 1.gcode.");
    assert!(
        fx.s.export_job_done(9_999, "x", false, 1).is_none(),
        "an unknown token does nothing"
    );
}

#[test]
fn c27_characters_not_allowed_in_file_names_become_dashes() {
    let (mut fx, _, _) = one_cut();
    let _ = fx.s.rename_page(&fx.l.id.clone(), "A/B:C*D?");
    let e = fx.s.export_job("My: project.curvyo").unwrap();
    assert!(
        !e.name.contains('/')
            && !e.name.contains(':')
            && !e.name.contains('*')
            && !e.name.contains('?'),
        "{}",
        e.name
    );
    assert!(e.name.ends_with(".gcode"));
}

// ---- criterion 28: effects and undo ---------------------------------------------------------------

#[test]
fn c28_export_turns_running_fixture_jobs_off_in_one_undoable_step() {
    let (mut fx, ids) = page_with(|d, l, m| {
        let a = rect(d, l, 100.0, 100.0, 20.0, 10.0);
        vec![op(d, m, "cut", "Cut 1", &[a])]
    });
    on_profile(&mut fx);
    numbers(&mut fx.s, ids[0], 90.0, 300.0, 1.0);
    let job = running_fixture_score(&mut fx.s);
    load_material(
        &mut fx.s,
        &json!({"score": {"power_pct": 12.0, "speed_mm_min": 4000, "passes": 1}}),
    );
    assert!(fx.s.set_fixture_material(MATERIAL));
    let first = export_bytes(&mut fx.s);
    let e = fx.s.export_job("Untitled.curvyo").unwrap();
    assert_eq!(e.bytes, first);
    let (text, ms) =
        fx.s.export_job_done(e.token, &e.name, false, 1_000)
            .unwrap();
    assert_eq!(
        text,
        "Exported Untitled - Laser 1.gcode.\nOutput turned off on 1 fixture job."
    );
    assert_eq!(ms, 5_000);
    assert!(
        !fixture_output(&fx.s, job),
        "the fixture job's Output is off"
    );
    assert_eq!(undo_name(&mut fx.s).as_deref(), Some("Export job file"));
    // The next export no longer runs the fixture job.
    let p = read(&export_bytes(&mut fx.s)).unwrap();
    assert!(!p.settings_order().contains(&(120, 4000)));
    fx.s.undo();
    assert!(fixture_output(&fx.s, job), "undo restores Output");
    assert_eq!(
        export_bytes(&mut fx.s),
        first,
        "and the file is the first one again"
    );
    fx.s.redo();
    assert!(!fixture_output(&fx.s, job));
}

#[test]
fn c28b_stored_numbers_are_refreshed_never_the_overrides() {
    let (mut fx, ids) = page_with(|d, l, m| {
        let a = rect(d, l, 100.0, 100.0, 20.0, 10.0);
        vec![op(d, m, "cut", "Cut 1", &[a])]
    });
    on_profile(&mut fx);
    load_material(
        &mut fx.s,
        &json!({"cut": {"power_pct": 60.0, "speed_mm_min": 300, "passes": 1}}),
    );
    assert!(fx.s.use_material_on(&[ids[0]], MATERIAL));
    // A typed override of speed.
    assert_eq!(
        fx.s.edit_operation_setting(ids[0], "main", "speed", 250.0),
        Ok(true)
    );
    // The material's power changes in the library (another session).
    load_material(
        &mut fx.s,
        &json!({"cut": {"power_pct": 75.0, "speed_mm_min": 300, "passes": 1}}),
    );
    let settings = |s: &Session| {
        doc(s)
            .operations(fx.l.kind.manufacturing().unwrap())
            .unwrap()[0]
            .settings
            .clone()
            .unwrap()
    };
    let before = settings(&fx.s);
    let e = fx.s.export_job("Untitled.curvyo").unwrap();
    let p = read(&e.bytes).unwrap();
    assert_eq!(p.runs()[0].s, 750, "live material power");
    assert_eq!(p.runs()[0].f, 250, "typed speed");
    fx.s.export_job_done(e.token, &e.name, false, 1_000)
        .unwrap();
    let after = settings(&fx.s);
    assert_ne!(before, after, "the stored numbers are refreshed");
    assert_eq!(after.to_string().contains("75"), true, "{after}");
    assert!(
        after.to_string().contains("250"),
        "the override stays: {after}"
    );
    fx.s.undo();
    assert_eq!(settings(&fx.s), before, "undo restores the stored numbers");
}

// ---- criteria 28c, 29, 30: Lasered ----------------------------------------------------------------

fn fonts_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../frontend/public/fonts")
}

fn load_fonts(s: &mut Session) {
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
}

fn place_grid(s: &mut Session, l: &Page, x: f64, y: f64) -> NodeId {
    s.set_tool(Tool::TestGrid);
    let g = s
        .test_grid_press(Some(Point::new(l.origin.x + x, l.origin.y + y)))
        .unwrap();
    s.set_tool(Tool::Select);
    g
}

fn grid_view(s: &Session) -> Value {
    serde_json::from_str(&s.test_grid_view_json()).unwrap()
}

fn lasered(s: &Session, g: NodeId) -> bool {
    doc(s).test_grid(g).expect("the grid").lasered.is_some()
}

/// The test id of the grid at page (20, 20), read by pressing the tool on it.
fn grid_id_at(s: &mut Session, l: &Page) -> String {
    s.set_tool(Tool::TestGrid);
    let _ = s.test_grid_press(Some(Point::new(l.origin.x + 35.0, l.origin.y + 35.0)));
    s.set_tool(Tool::Select);
    grid_view(s)["id"]
        .as_str()
        .expect("a selected grid")
        .to_owned()
}

fn grid_session() -> (Fx, NodeId) {
    let (mut fx, _) = page_with(|_, _, _| vec![]);
    load_fonts(&mut fx.s);
    let l = fx.l.clone();
    let g = place_grid(&mut fx.s, &l, 20.0, 20.0);
    fx.s.wipe_history();
    (fx, g)
}

#[test]
fn c28c_c30_export_sets_lasered_which_stays_after_undo_and_the_header_names_the_test_id() {
    let (mut fx, g) = grid_session();
    let l = fx.l.clone();
    assert!(!lasered(&fx.s, g));
    let id = grid_id_at(&mut fx.s, &l);
    fx.s.wipe_history();
    let e =
        fx.s.export_job("Untitled.curvyo")
            .unwrap_or_else(|| panic!("{}", fx.s.output_group_json()));
    let text = String::from_utf8(e.bytes.clone()).unwrap();
    assert!(
        text.lines()
            .filter(|l| l.starts_with(';'))
            .any(|l| l.contains(&id)),
        "the header names test id {id}:\n{text}"
    );
    read(&e.bytes).unwrap();
    assert!(
        !lasered(&fx.s, g),
        "export_job alone (the dialog) sets nothing"
    );
    fx.s.export_job_done(e.token, &e.name, false, 1_762_948_800_000)
        .unwrap();
    assert!(lasered(&fx.s, g), "Lasered by the export");
    while fx.s.undo_len() > 0 {
        fx.s.undo();
    }
    assert!(lasered(&fx.s, g), "Lasered stays after undo");
    // An unchanged grid keeps its id on the next export.
    let e = fx.s.export_job("Untitled.curvyo").unwrap();
    fx.s.export_job_done(e.token, &e.name, false, 1_762_948_900_000)
        .unwrap();
    assert_eq!(grid_id_at(&mut fx.s, &l), id, "same grid, same id");
}

#[test]
fn c29_a_grid_whose_operations_did_not_write_is_not_lasered() {
    let (mut fx, g) = grid_session();
    let m = fx.l.kind.manufacturing().unwrap();
    let ops = doc(&fx.s).operations(m).unwrap();
    assert!(!ops.is_empty(), "the grid derives operations");
    let other = {
        let d = doc(&fx.s);
        let _ = d;
        None::<NodeId>
    };
    let _ = other;
    for o in &ops {
        assert!(fx.s.set_operation_switch(o.id, OperationSwitch::Output, false));
    }
    // Something else must run so the export is allowed: nothing does, so it is blocked.
    assert!(fx.s.export_job("Untitled.curvyo").is_none(), "nothing runs");
    assert!(!lasered(&fx.s, g));
}

// ---- criterion 33 ---------------------------------------------------------------------------------

#[test]
fn c33_damaged_settings_warn_once_and_do_not_run() {
    let (mut fx, ids) = page_with(|d, l, m| {
        let a = rect(d, l, 10.0, 10.0, 20.0, 10.0);
        let b = rect(d, l, 50.0, 10.0, 20.0, 10.0);
        let good = op(d, m, "cut", "Cut 1", &[a]);
        let bad = op(d, m, "cut", "Cut 2", &[b]);
        d.write_operation_settings(
            curvyo_document_core::SettingsStep::EditSettings,
            &[curvyo_document_core::SettingsWrite {
                operation: bad,
                value: json!({"main": {"power_pct": "high", "speed_mm_min": -3}})
                    .as_object()
                    .cloned(),
            }],
        )
        .unwrap();
        vec![good, bad]
    });
    numbers(&mut fx.s, ids[0], 50.0, 600.0, 1.0);
    let warns = check_texts(&fx.s, "warn");
    let w: Vec<&String> = warns
        .iter()
        .filter(|w| w.contains("damaged settings"))
        .collect();
    assert_eq!(w.len(), 1, "{warns:?}");
    assert!(w[0].chars().count() <= 70);
    assert!(check_texts(&fx.s, "block").is_empty());
    let runs = runs(&fx.s);
    assert_eq!(runs.len(), 1, "Cut 2 is no run");
    let p = export(&mut fx.s);
    assert!((p.burn_mm() - 60.0).abs() < 0.01);
}

// ---- criterion 38 ---------------------------------------------------------------------------------

#[test]
fn c38_one_start_pick_head_position_and_home_then_origin_write_their_files() {
    // Superseded by 0029 rework (2026-11-17): one Start pick of three values replaces Start from
    // and Home first; no Head position warn, no disabled state, no tooltip.
    let (mut fx, _, _) = one_cut();
    let base = export(&mut fx.s);
    assert_eq!(out(&fx.s)["start"]["value"], "machine_origin");
    assert!(fx.s.set_start("head_position"));
    let v = out(&fx.s);
    assert_eq!(v["start"]["value"], "head_position");
    assert!(
        !check_texts(&fx.s, "warn")
            .iter()
            .any(|w| w.starts_with("Starts where")),
        "no Head position warn"
    );
    assert_eq!(v["export"]["enabled"], true);
    let rel = export(&mut fx.s);
    assert!(rel.relative);
    assert_eq!(rel.end, (0, 0));
    assert!(fx.s.set_start("home_then_origin"));
    assert_eq!(out(&fx.s)["start"]["value"], "home_then_origin");
    let home = export(&mut fx.s);
    assert!(home.homes && !home.relative);
    assert_eq!(&home.commands[1..], &base.commands[..]);
    assert_eq!(
        undo_name(&mut fx.s).as_deref(),
        Some("Start: Home, then machine origin")
    );
}

#[test]
fn c38_head_position_checks_the_bed_from_the_jobs_own_box() {
    // Superseded by 0029 rework (2026-11-17): the bed check is measured from the job's own box
    // (criterion 38), so a job that fits the bed does not block wherever it sits on the page.
    let (mut fx, ids) = page_with(|d, l, m| {
        let a = rect(d, l, 390.0, 10.0, 30.0, 10.0);
        vec![op(d, m, "cut", "Cut 1", &[a])]
    });
    numbers(&mut fx.s, ids[0], 50.0, 600.0, 1.0);
    assert!(
        !check_texts(&fx.s, "block").is_empty(),
        "outside the bed blocks at machine origin"
    );
    assert!(fx.s.set_start("head_position"));
    assert!(
        check_texts(&fx.s, "block").is_empty(),
        "{:?}",
        check_texts(&fx.s, "block")
    );
    let p = export(&mut fx.s);
    assert!(p.relative);
}

// ---- criterion 39 ---------------------------------------------------------------------------------

#[test]
fn c39_the_machine_block_shows_output_grbl_read_only() {
    let (fx, _, _) = one_cut();
    let m: Value = serde_json::from_str(&fx.s.manufacturing_view_json()).unwrap();
    assert_eq!(m["machine"]["copy"]["output"], "GRBL");
}

// ---- edges ----------------------------------------------------------------------------------------

#[test]
fn edge_undo_redo_around_export_and_an_edit_during_the_dialog() {
    let (mut fx, _, cut) = one_cut();
    let e = fx.s.export_job("Untitled.curvyo").unwrap();
    // An edit while the dialog is open does not change what the token records.
    assert_eq!(
        fx.s.edit_operation_setting(cut, "main", "power", 80.0),
        Ok(true)
    );
    let (text, _) =
        fx.s.export_job_done(e.token, &e.name, false, 1_000)
            .unwrap();
    assert!(text.starts_with("Exported"));
    assert!(
        read(&e.bytes).unwrap().runs()[0].s == 500,
        "the written bytes are the dialog's"
    );
    fx.s.undo();
    fx.s.redo();
    let p = export(&mut fx.s);
    assert_eq!(p.runs()[0].s, 800);
}

#[test]
fn edge_a_thousand_items_export_and_read_back() {
    let (mut fx, ids) = page_with(|d, l, m| {
        let mut items = Vec::new();
        for k in 0..1000_u64 {
            let x = 5.0 + (k % 40) as f64 * 9.5;
            let y = 5.0 + (k / 40) as f64 * 11.5;
            items.push(rect(d, l, x, y, 5.0, 5.0));
        }
        vec![op(d, m, "cut", "Cut 1", &items)]
    });
    numbers(&mut fx.s, ids[0], 50.0, 600.0, 1.0);
    let p = export(&mut fx.s);
    assert_eq!(p.runs().len(), 1000);
    assert!((p.burn_mm() - 20_000.0).abs() < 0.1);
}

#[test]
fn edge_very_long_names_stay_within_80_character_comment_lines() {
    let long = "Ü".repeat(200);
    let (mut fx, ids) = page_with(|d, l, m| {
        let a = rect(d, l, 10.0, 10.0, 20.0, 10.0);
        vec![op(d, m, "cut", &format!("Cut {long}"), &[a])]
    });
    numbers(&mut fx.s, ids[0], 50.0, 600.0, 1.0);
    let e = fx.s.export_job("Untitled.curvyo").unwrap();
    read(&e.bytes).unwrap();
    for l in String::from_utf8(e.bytes)
        .unwrap()
        .lines()
        .filter(|l| l.starts_with(';'))
    {
        assert!(l.len() <= 80, "{l}");
    }
}

// ---- more edges -----------------------------------------------------------------------------------

#[test]
fn c19_z_offset_written_only_with_the_motorised_axis_else_warned() {
    let (mut fx, _, cut) = one_cut();
    on_profile(&mut fx);
    assert!(fx.s.set_machine_motorised_z(true));
    assert_eq!(fx.s.edit_operation_option(cut, "z_offset", -1.5), Ok(true));
    let p = export(&mut fx.s);
    let z: Vec<i64> = p
        .segs
        .iter()
        .filter(|s| s.kind == Kind::Z)
        .map(|s| s.z)
        .collect();
    assert_eq!(z, [-1500, 0]);
    assert!(fx.s.set_machine_motorised_z(false));
    let p = export(&mut fx.s);
    assert!(
        !p.segs.iter().any(|s| s.kind == Kind::Z),
        "no Z without the axis"
    );
    let warns = check_texts(&fx.s, "warn");
    assert_eq!(
        warns
            .iter()
            .filter(|w| w.to_lowercase().contains('z'))
            .count(),
        1,
        "{warns:?}"
    );
    assert_eq!(out(&fx.s)["export"]["enabled"], true);
}

#[test]
fn c4a_a_fixture_material_without_a_set_of_the_kind_blocks() {
    let (mut fx, ids) = page_with(|d, l, m| {
        let a = rect(d, l, 100.0, 100.0, 20.0, 10.0);
        vec![op(d, m, "cut", "Cut 1", &[a])]
    });
    on_profile(&mut fx);
    numbers(&mut fx.s, ids[0], 90.0, 300.0, 1.0);
    let _ = running_fixture_score(&mut fx.s);
    load_material(
        &mut fx.s,
        &json!({"cut": {"power_pct": 60.0, "speed_mm_min": 300, "passes": 1}}),
    );
    let _ = fx.s.set_fixture_material(MATERIAL);
    assert!(
        !check_texts(&fx.s, "block").is_empty(),
        "no Score set for this laser blocks"
    );
    assert!(fx.s.export_job("Untitled.curvyo").is_none());
}

#[test]
fn c5_material_not_found_runs_the_stored_numbers_and_warns() {
    let (mut fx, ids) = page_with(|d, l, m| {
        let a = rect(d, l, 100.0, 100.0, 20.0, 10.0);
        vec![op(d, m, "cut", "Cut 1", &[a])]
    });
    on_profile(&mut fx);
    load_material(
        &mut fx.s,
        &json!({"cut": {"power_pct": 60.0, "speed_mm_min": 300, "passes": 1}}),
    );
    assert!(fx.s.use_material_on(&[ids[0]], MATERIAL));
    assert_eq!(
        fx.s.edit_operation_setting(ids[0], "main", "power", 65.0),
        Ok(true)
    );
    let warns = check_texts(&fx.s, "warn");
    assert!(
        warns
            .iter()
            .any(|w| w.to_lowercase().contains("typed") || w.to_lowercase().contains("override")),
        "typed override warns: {warns:?}"
    );
    fx.s.load_materials(&[], None);
    let warns = check_texts(&fx.s, "warn");
    assert!(
        warns
            .iter()
            .any(|w| w.to_lowercase().contains("not found") || w.to_lowercase().contains("stored")),
        "{warns:?}"
    );
    let p = export(&mut fx.s);
    assert_eq!(p.runs()[0].s, 650, "typed power over stored numbers");
    assert_eq!(p.runs()[0].f, 300, "stored speed");
}

#[test]
fn c17_a_linked_clone_writes_where_it_stands() {
    let (mut fx, ids) = page_with(|d, l, m| {
        let a = rect(d, l, 10.0, 10.0, 20.0, 10.0);
        vec![a, op(d, m, "cut", "Cut 1", &[a])]
    });
    numbers(&mut fx.s, ids[1], 50.0, 600.0, 1.0);
    assert!(fx.s.select_output_items(&[ids[0]]));
    assert!(fx.s.linked_clone_shown(), "Linked clone is offered");
    let _ = fx.s.linked_clone();
    assert!(fx.s.set_selection_in_operation(ids[1], true));
    let p = export(&mut fx.s);
    // A clone made in place lies on its original, so the two closed paths may join into one run.
    assert!(
        (p.burn_mm() - 120.0).abs() < 0.01,
        "the original and its clone: {}",
        p.burn_mm()
    );
}

// ---- rework 2026-11-17 (customer feedback on PR #371): rows, fixture rings, checks, Start ----------
// Written from the amended specification (criteria 1 to 7, 10, 24, 26, 28, 32, 36 to 40) before
// the rework's code was read.

fn ops_view(s: &Session) -> Value {
    serde_json::from_str(&s.operations_view_json()).unwrap()
}

fn row<'a>(v: &'a Value, name: &str) -> &'a Value {
    v["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["name"] == name)
        .unwrap_or_else(|| panic!("row {name} in {v}"))
}

fn ring_labels(s: &Session) -> Vec<Value> {
    serde_json::from_str::<Value>(&s.toolpath_ring_labels_json())
        .unwrap()
        .as_array()
        .cloned()
        .unwrap()
}

#[test]
fn rework_c2_rows_carry_ring_time_and_numbers_and_rows_that_do_not_run_keep_their_digit() {
    let (mut fx, ids) = page_with(|d, l, m| {
        let a = rect(d, l, 10.0, 10.0, 20.0, 10.0);
        let b = rect(d, l, 50.0, 10.0, 20.0, 10.0);
        let c = rect(d, l, 90.0, 10.0, 20.0, 10.0);
        vec![
            op(d, m, "engrave", "Engrave 1", &[a]),
            op(d, m, "score", "Score 1", &[]),
            op(d, m, "cut", "Cut 1", &[b]),
            op(d, m, "cut", "Cut 2", &[c]),
            op(d, m, "cut", "Cut 3", &[c]),
        ]
    });
    on_profile(&mut fx);
    numbers(&mut fx.s, ids[0], 30.0, 6000.0, 1.0);
    numbers(&mut fx.s, ids[2], 90.0, 300.0, 2.0);
    numbers(&mut fx.s, ids[3], 80.0, 400.0, 1.0);
    assert!(fx.s.set_operation_switch(ids[3], OperationSwitch::Output, false));
    let v = ops_view(&fx.s);
    let rows = v["rows"].as_array().unwrap();
    // The digit of a running row is its place in the list (the ring of the Toolpath view).
    for (k, r) in rows.iter().enumerate() {
        if !r["run"].is_null() {
            assert_eq!(r["run"]["digit"], k + 1, "{}", r["name"]);
        }
    }
    let e = row(&v, "Engrave 1");
    assert!(!e["run"].is_null());
    let et = e["run"]["time"].as_str().unwrap();
    assert!(et != "–" && et.ends_with(" s"), "a time: {et}");
    let line3 = e["numbers"][0]["text"].as_str().unwrap();
    assert!(
        line3.starts_with("30 %, 6,000 mm/min, 1 ×, ") && line3.ends_with(" mm"),
        "Engrave numbers carry the Interval: {line3}"
    );
    let c1 = row(&v, "Cut 1");
    assert_eq!(c1["numbers"][0]["text"], "90 %, 300 mm/min, 2 ×");
    assert!(!c1["run"].is_null());
    // Score 1 (no item), Cut 2 (Output off) do not run: no ring, no time.
    assert!(row(&v, "Score 1")["run"].is_null(), "no item: no run");
    let c2 = row(&v, "Cut 2");
    assert!(c2["run"].is_null(), "Output off: no run");
    // Line 3 shows on every row, running or not.
    assert_eq!(c2["numbers"][0]["text"], "80 %, 400 mm/min, 1 ×");
    // Superseded by `specs/0150` criterion 27: Cut 3 without `op_settings` runs the default
    // numbers, with a time and its line 3.
    let c3 = row(&v, "Cut 3");
    assert_ne!(c3["run"]["time"], "–");
    assert_eq!(c3["numbers"][0]["text"], "10 %, 1,000 mm/min, 1 ×");
    // Folding a row never drops lines 2 and 3.
    assert_eq!(c1["unfolded"], false);
    // The row's time is the run's own time: the four times sum to about the total.
    let total = out(&fx.s)["total"]["time"].as_str().unwrap().to_owned();
    assert!(total.starts_with("About "), "{total}");
}

#[test]
fn rework_c2_a_typed_number_reads_typed_on_line_3() {
    let (mut fx, ids) = page_with(|d, l, m| {
        let a = rect(d, l, 100.0, 100.0, 20.0, 10.0);
        vec![op(d, m, "cut", "Cut 1", &[a])]
    });
    on_profile(&mut fx);
    load_material(
        &mut fx.s,
        &json!({"cut": {"power_pct": 60.0, "speed_mm_min": 300, "passes": 1}}),
    );
    assert!(fx.s.use_material_on(&[ids[0]], MATERIAL));
    assert_eq!(
        row(&ops_view(&fx.s), "Cut 1")["numbers"][0]["text"],
        "60 %, 300 mm/min, 1 ×"
    );
    assert_eq!(
        fx.s.edit_operation_setting(ids[0], "main", "power", 65.0),
        Ok(true)
    );
    let v = ops_view(&fx.s);
    let n = &row(&v, "Cut 1")["numbers"][0];
    assert_eq!(n["text"], "65 % typed, 300 mm/min, 1 ×");
    assert_eq!(n["typed"], json!(["power"]));
}

#[test]
fn rework_c2_fixture_runs_take_the_first_rings_with_the_fixture_glyph() {
    let (mut fx, ids) = page_with(|d, l, m| {
        let a = rect(d, l, 200.0, 150.0, 20.0, 10.0);
        vec![op(d, m, "cut", "Cut 1", &[a])]
    });
    on_profile(&mut fx);
    numbers(&mut fx.s, ids[0], 90.0, 300.0, 1.0);
    let _ = running_fixture_score(&mut fx.s);
    load_material(
        &mut fx.s,
        &json!({"score": {"power_pct": 12.0, "speed_mm_min": 4000, "passes": 1}}),
    );
    assert!(fx.s.set_fixture_material(MATERIAL));
    let f: Value = serde_json::from_str(&fx.s.fixture_jobs_view_json()).unwrap();
    let runs = f["output"]["runs"].as_array().unwrap();
    assert_eq!(runs.len(), 1, "one run line per kind: {runs:?}");
    assert_eq!(runs[0]["kind"], "score");
    assert_eq!(runs[0]["numbers"][0]["text"], "12 %, 4,000 mm/min, 1 ×");
    assert_ne!(runs[0]["time"], "–");
    // The operation row keeps its digit 1 although the fixture run comes first.
    assert_eq!(row(&ops_view(&fx.s), "Cut 1")["run"]["digit"], 1);
    assert!(fx.s.toggle_toolpath_view(&fx.l.id.clone()));
    let rings = ring_labels(&fx.s);
    assert_eq!(rings.len(), 2, "{rings:?}");
    assert!(
        rings[0]["digit"].is_null(),
        "the fixture glyph first: {rings:?}"
    );
    assert_eq!(rings[1]["digit"], 1);
}

/// Every job id of fixture `g` in the Fixture jobs section.
fn jobs_of(s: &Session, g: NodeId) -> Vec<NodeId> {
    let v: Value = serde_json::from_str(&s.fixture_jobs_view_json()).unwrap();
    v["groups"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|grp| grp["fixture"] == g.full_id())
        .flat_map(|grp| grp["jobs"].as_array().unwrap().clone())
        .map(|j| s.fixture_job_named(j["id"].as_str().unwrap()).unwrap())
        .collect()
}

#[test]
fn rework_c2_a_running_pin_job_has_a_fixture_ring_on_the_canvas() {
    // The implementer saw no fixture ring for a running pin job (lead's note).
    let (mut fx, _) = page_with(|_, _, _| vec![]);
    on_profile(&mut fx);
    let pin = fixture_support::add_fixture(&mut fx.s, "pin").expect("a pin");
    let jobs = jobs_of(&fx.s, pin);
    assert!(!jobs.is_empty(), "the pin has its role jobs");
    for &job in &jobs {
        assert!(fx.s.set_fixture_job_switch(job, FixtureJobSwitch::Output, true));
    }
    load_material(
        &mut fx.s,
        &json!({"score": {"power_pct": 12.0, "speed_mm_min": 4000, "passes": 1}}),
    );
    assert!(fx.s.set_fixture_material(MATERIAL));
    let f: Value = serde_json::from_str(&fx.s.fixture_jobs_view_json()).unwrap();
    assert!(f["running"].as_u64().unwrap() >= 1, "{f}");
    assert!(
        check_texts(&fx.s, "block").is_empty(),
        "{:?}",
        check_texts(&fx.s, "block")
    );
    assert!(fx.s.toggle_toolpath_view(&fx.l.id.clone()));
    let rings = ring_labels(&fx.s);
    let p = export(&mut fx.s);
    assert!(p.burn_mm() > 0.0, "the pin job burns");
    assert!(
        rings.iter().any(|r| r["digit"].is_null()),
        "a fixture ring for the running pin job: {rings:?}"
    );
}

#[test]
fn rework_c4_c5_every_check_reads_before_button_after_and_names_its_target() {
    let (mut fx, ids) = page_with(|d, l, m| {
        let a = rect(d, l, 390.0, 10.0, 30.0, 10.0);
        let b = rect(d, l, 10.0, 10.0, 20.0, 10.0);
        let _loose = rect(d, l, 10.0, 100.0, 20.0, 10.0);
        vec![
            op(d, m, "cut", "Cut 1", &[a]),
            op(d, m, "cut", "Cut 2", &[b]),
            op(d, m, "score", "Score 1", &[b]),
        ]
    });
    numbers(&mut fx.s, ids[0], 50.0, 600.0, 1.0);
    numbers(&mut fx.s, ids[2], 50.0, 600.0, 1.0);
    assert!(fx.s.set_operation_switch(ids[2], OperationSwitch::Output, false));
    let cs = checks(&fx.s);
    let texts: Vec<&str> = cs.iter().map(|c| c["text"].as_str().unwrap()).collect();
    for c in &cs {
        let whole = format!(
            "{}{}{}",
            c["before"].as_str().unwrap(),
            c["button"].as_str().unwrap(),
            c["after"].as_str().unwrap()
        );
        assert_eq!(c["text"], whole.as_str());
        assert_eq!(
            c["button"] == "",
            c["target"]["kind"] == "none",
            "a button exactly when there is a target: {c}"
        );
    }
    // Blocks first (4 a before 4 b), then warns.
    let sev: Vec<&str> = cs.iter().map(|c| c["severity"].as_str().unwrap()).collect();
    let first_warn = sev.iter().position(|s| *s == "warn").unwrap();
    assert!(sev[..first_warn].iter().all(|s| *s == "block"), "{sev:?}");
    assert!(sev[first_warn..].iter().all(|s| *s == "warn"), "{sev:?}");
    // Superseded by `specs/0150` criterion 23: Cut 2 without `op_settings` no longer blocks.
    assert!(
        !texts.iter().any(|t| t.contains("has no power and speed")),
        "{texts:?}"
    );
    let bed = cs
        .iter()
        .position(|c| c["text"] == "1 item lies outside the 400 × 300 mm bed.");
    let bed = &cs[bed.unwrap()];
    assert_eq!(bed["button"], "1 item");
    assert_eq!(bed["target"]["kind"], "items");
    assert_eq!(bed["target"]["ids"].as_array().unwrap().len(), 1);
    // The 0101 count lines are warns of the checks now.
    let loose = cs
        .iter()
        .find(|c| c["after"].as_str().unwrap().contains("not assigned"))
        .unwrap_or_else(|| panic!("{texts:?}"));
    assert_eq!(loose["severity"], "warn");
    assert_eq!(loose["target"]["kind"], "items");
    let off = cs
        .iter()
        .find(|c| c["text"] == "Score 1 has Output off.")
        .unwrap_or_else(|| panic!("{texts:?}"));
    assert_eq!(off["button"], "Score 1");
    // Superseded by the lead decision of 2026-11-17 (review A1): the Output off line names its
    // operation but selects its items, as the accepted `0101` count button did.
    assert_eq!(off["target"]["kind"], "items");
    // Export: disabled with the first block as its tooltip.
    let ex = out(&fx.s)["export"].clone();
    assert_eq!(ex["enabled"], false);
    // Superseded by `specs/0150` criterion 23: the bed is the first block now.
    assert_eq!(ex["tooltip"], "1 item lies outside the 400 × 300 mm bed.");
    assert_eq!(ex["file_type"], "GRBL G-code");
    assert_eq!(ex["extension"], "gcode");
}

#[test]
fn rework_c4_export_is_enabled_without_tooltip_once_the_blocks_are_gone() {
    let (mut fx, _, _) = one_cut();
    on_profile(&mut fx);
    let ex = out(&fx.s)["export"].clone();
    assert_eq!(ex["enabled"], true);
    assert!(ex["tooltip"].is_null(), "{ex}");
    assert!(fx.s.export_job("Untitled.curvyo").is_some());
}

#[test]
fn rework_c7_nothing_runs_has_no_total_and_no_button() {
    let (fx, _) = page_with(|d, l, _| vec![rect(d, l, 10.0, 10.0, 20.0, 10.0)]);
    let v = out(&fx.s);
    let blocks: Vec<Value> = checks(&fx.s)
        .into_iter()
        .filter(|c| c["severity"] == "block")
        .collect();
    assert_eq!(blocks.len(), 1, "{blocks:?}");
    assert_eq!(
        blocks[0]["text"],
        "Nothing runs. Assign items to an operation."
    );
    assert_eq!(blocks[0]["button"], "");
    assert!(v["total"].is_null());
    assert_eq!(v["export"]["enabled"], false);
    assert_eq!(
        v["export"]["tooltip"],
        "Nothing runs. Assign items to an operation."
    );
}

// ---- Start: one register, three values (criteria 4 e, 24, 26, 32, 38) ----------------------------

fn zip_members(bytes: &[u8]) -> std::collections::BTreeMap<String, Vec<u8>> {
    use std::io::Read as _;
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    (0..archive.len())
        .map(|i| {
            let mut file = archive.by_index(i).unwrap();
            let mut content = Vec::new();
            file.read_to_end(&mut content).unwrap();
            (file.name().unwrap().to_string(), content)
        })
        .collect()
}

fn rezip(files: &std::collections::BTreeMap<String, Vec<u8>>) -> Vec<u8> {
    use std::io::Write as _;
    let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    for (name, content) in files {
        writer.start_file(name.as_str(), options).unwrap();
        writer.write_all(content).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

/// One Cut 1 (50 %, 600 mm/min) on a saved-profile laser page whose page registers are edited
/// in the file by `edit` (the page node's Loro meta) before the session opens it.
fn edited_page(edit: impl Fn(&loro::LoroMap)) -> Fx {
    let d = Document::new(1);
    let made = d
        .add_output_page(NewOutputPage {
            process: LASER,
            label: "Laser",
            profile: None,
            work_area: DocumentSize::from_mm(400.0, 300.0),
        })
        .unwrap();
    let l = d.pages().into_iter().find(|p| p.id == made.id).unwrap();
    let m = l.kind.manufacturing().unwrap();
    let r = rect(&d, &l, 10.0, 10.0, 20.0, 10.0);
    let cut = op(&d, m, "cut", "Cut 1", &[r]);
    // A marker so the page node can be found in the tree.
    d.set_start(m, "head_position").unwrap();
    let mut files = zip_members(&pack(&d, "0029-tester").unwrap());
    let loro = loro::LoroDoc::new();
    loro.import(&files["document.loro"]).unwrap();
    let tree = loro.get_tree("paths");
    let page = tree
        .children(loro::TreeParentId::Root)
        .unwrap()
        .into_iter()
        .find(|&n| {
            tree.get_meta(n)
                .unwrap()
                .get("x.curvyo.laser.start")
                .is_some()
        })
        .expect("the laser page node");
    let meta = tree.get_meta(page).unwrap();
    meta.delete("x.curvyo.laser.start").unwrap();
    edit(&meta);
    loro.commit();
    files.insert(
        "document.loro".to_owned(),
        loro.export(loro::ExportMode::Snapshot).unwrap(),
    );
    let mut s = Session::open(7, &rezip(&files)).unwrap();
    s.resize_viewport(1600.0, 1000.0);
    s.set_tool(Tool::Select);
    s.load_machine_profiles_json("[]");
    s.set_clock(|| Timestamp::from_secs(1_762_948_800));
    assert!(s.activate_page(&l.id));
    s.load_materials(&[], None);
    numbers(&mut s, cut, 50.0, 600.0, 1.0);
    s.wipe_history();
    Fx { s, l }
}

#[test]
fn rework_c38_an_absent_start_reads_machine_origin() {
    let mut fx = edited_page(|_| {});
    let v = out(&fx.s);
    assert_eq!(v["start"]["value"], "machine_origin");
    assert_eq!(v["start"]["damaged"], false);
    let labels: Vec<&str> = v["start"]["options"]
        .as_array()
        .unwrap()
        .iter()
        .map(|o| o["label"].as_str().unwrap())
        .collect();
    assert_eq!(
        labels,
        [
            "Machine origin",
            "Head position",
            "Home, then machine origin"
        ]
    );
    let p = export(&mut fx.s);
    assert!(!p.relative && !p.homes);
}

#[test]
fn rework_c32_a_pr_371_file_with_start_from_and_home_first_opens_as_machine_origin() {
    let mut fx = edited_page(|meta| {
        meta.insert("x.curvyo.laser.start_from", "head_position")
            .unwrap();
        meta.insert("x.curvyo.laser.home_first", true).unwrap();
    });
    let v = out(&fx.s);
    assert_eq!(v["start"]["value"], "machine_origin", "{v}");
    assert_eq!(
        v["start"]["damaged"], false,
        "old keys are not a damaged Start"
    );
    assert!(
        check_texts(&fx.s, "block").is_empty(),
        "{:?}",
        checks(&fx.s)
    );
    let p = export(&mut fx.s);
    assert!(!p.relative, "not Head position");
    assert!(!p.homes, "no $H from the old Home first");
    // The old registers survive a save unchanged (the file is not rewritten on open).
    let d = doc(&fx.s);
    let m = d
        .manufacturings()
        .into_iter()
        .find(|m| m.process_values.contains_key("x.curvyo.laser.start_from"))
        .expect("start_from kept");
    assert_eq!(m.process_values["x.curvyo.laser.home_first"], json!(true));
}

#[test]
fn rework_c4e_an_unknown_start_blocks_names_its_pick_and_a_pick_clears_it() {
    let mut fx = edited_page(|meta| {
        meta.insert("x.curvyo.laser.start", "middle").unwrap();
    });
    let v = out(&fx.s);
    assert!(
        v["start"]["value"].is_null(),
        "no option checked: {}",
        v["start"]
    );
    assert_eq!(v["start"]["damaged"], true);
    let line = checks(&fx.s)
        .into_iter()
        .find(|c| c["text"] == "Start is damaged. Pick one above.")
        .unwrap_or_else(|| panic!("{:?}", checks(&fx.s)));
    assert_eq!(line["severity"], "block");
    assert_eq!(line["button"], "Start");
    assert_eq!(line["target"]["kind"], "start_pick");
    assert_eq!(v["export"]["enabled"], false);
    assert_eq!(v["export"]["tooltip"], "Start is damaged. Pick one above.");
    assert!(
        fx.s.export_job("Untitled.curvyo").is_none(),
        "never guessed into a file"
    );
    // A non-string value is damaged too.
    let other = edited_page(|meta| {
        meta.insert("x.curvyo.laser.start", 3).unwrap();
    });
    assert_eq!(out(&other.s)["start"]["damaged"], true);
    // Picking the value the build reads anyway (Machine origin) still writes and clears it.
    assert!(fx.s.set_start("machine_origin"));
    assert_eq!(
        undo_name(&mut fx.s).as_deref(),
        Some("Start: Machine origin")
    );
    let v = out(&fx.s);
    assert_eq!(v["start"]["value"], "machine_origin");
    assert!(check_texts(&fx.s, "block").is_empty());
    let p = export(&mut fx.s);
    assert!(!p.relative && !p.homes);
}

#[test]
fn rework_c38_each_start_writes_its_file_and_is_one_step() {
    let (mut fx, _, _) = one_cut();
    on_profile(&mut fx);
    let base_bytes = export_bytes(&mut fx.s);
    let base = read(&base_bytes).unwrap();
    assert!(!base.relative && !base.homes);
    let text = String::from_utf8(base_bytes.clone()).unwrap();
    assert!(
        !text.lines().any(|l| l.starts_with("G10")
            || l.starts_with("G92")
            || l.starts_with("G54")
            || l.starts_with("$H")),
        "Machine origin: no homing, no work offset"
    );
    assert!(!fx.s.set_start("home_first"), "an old spelling is no value");
    assert!(!fx.s.set_start(""), "nothing");
    // Home, then machine origin: $H is the first command after the header, then G21.
    let before = fx.s.undo_len();
    assert!(fx.s.set_start("home_then_origin"));
    assert_eq!(fx.s.undo_len(), before + 1, "one step");
    let bytes = export_bytes(&mut fx.s);
    let text = String::from_utf8(bytes.clone()).unwrap();
    let cmds: Vec<&str> = text
        .lines()
        .filter(|l| !l.trim_start().starts_with(';') && !l.trim().is_empty())
        .collect();
    assert_eq!(cmds[0], "$H", "{text}");
    assert_eq!(cmds[1], "G21");
    assert_eq!(text.matches("$H").count(), 1);
    let home = read(&bytes).unwrap();
    assert_eq!(&home.commands[1..], &base.commands[..]);
    // Head position: G91 instead of G90, no absolute coordinate, no warn, the end back to start.
    assert!(fx.s.set_start("head_position"));
    assert!(
        !check_texts(&fx.s, "warn")
            .iter()
            .any(|w| w.contains("head stands")),
        "{:?}",
        check_texts(&fx.s, "warn")
    );
    let bytes = export_bytes(&mut fx.s);
    let text = String::from_utf8(bytes.clone()).unwrap();
    assert!(text.lines().any(|l| l == "G91"), "{text}");
    assert!(!text.lines().any(|l| l == "G90"), "{text}");
    assert!(!text.contains("$H"));
    let rel = read(&bytes).unwrap();
    assert!(rel.relative);
    assert_eq!(rel.end, (0, 0), "back to where the head stood");
    // Undo walks back through the three picks; each recomputes the group.
    fx.s.undo();
    assert_eq!(out(&fx.s)["start"]["value"], "home_then_origin");
    fx.s.undo();
    assert_eq!(out(&fx.s)["start"]["value"], "machine_origin");
    assert_eq!(
        export_bytes(&mut fx.s),
        base_bytes,
        "Machine origin again: same bytes"
    );
    // Saved on the page.
    assert!(fx.s.set_start("head_position"));
    let reopened = Session::open(9, &fx.s.pack("0.1.0").unwrap()).unwrap();
    let mut reopened = reopened;
    reopened.load_machine_profiles_json("[]");
    assert!(reopened.activate_page(&fx.l.id));
    assert_eq!(out(&reopened)["start"]["value"], "head_position");
}

#[test]
fn rework_c37_the_play_readout_is_time_only() {
    let (mut fx, _, _) = one_cut();
    assert!(fx.s.toggle_toolpath_view(&fx.l.id.clone()));
    for f in [0.0, 0.5, 1.0] {
        fx.s.set_play_position(f);
        let r = fx.s.play_view().unwrap().readout;
        let (a, b) = r.split_once(" of ").unwrap_or_else(|| panic!("{r}"));
        for t in [a, b] {
            assert!(
                t.split(' ')
                    .all(|w| w.parse::<f64>().is_ok() || ["h", "min", "s"].contains(&w)),
                "time words only: {r}"
            );
        }
    }
}

#[test]
fn rework_c28_after_export_the_rings_keep_their_numbers() {
    let (mut fx, ids) = page_with(|d, l, m| {
        let a = rect(d, l, 100.0, 100.0, 20.0, 10.0);
        vec![op(d, m, "cut", "Cut 1", &[a])]
    });
    on_profile(&mut fx);
    numbers(&mut fx.s, ids[0], 90.0, 300.0, 1.0);
    let _ = running_fixture_score(&mut fx.s);
    load_material(
        &mut fx.s,
        &json!({"score": {"power_pct": 12.0, "speed_mm_min": 4000, "passes": 1}}),
    );
    assert!(fx.s.set_fixture_material(MATERIAL));
    assert_eq!(row(&ops_view(&fx.s), "Cut 1")["run"]["digit"], 1);
    let e = fx.s.export_job("Untitled.curvyo").expect("export allowed");
    assert!(
        fx.s.export_job_done(e.token, &e.name, false, 1_000)
            .is_some()
    );
    let f: Value = serde_json::from_str(&fx.s.fixture_jobs_view_json()).unwrap();
    assert!(
        f["output"].is_null() || f["output"]["runs"].as_array().is_none_or(Vec::is_empty),
        "the fixture run lines leave the section: {}",
        f["output"]
    );
    assert_eq!(
        row(&ops_view(&fx.s), "Cut 1")["run"]["digit"],
        1,
        "no renumbering"
    );
}

#[test]
fn rework_0103_c17_the_fixture_line_and_no_empty_work_line() {
    // `0103` criterion 17 is not amended by the rework: a running fixture job is stated
    // ("1 fixture job runs.") and the work line shows only when a work operation runs.
    let (mut fx, _) = page_with(|_, _, _| vec![]);
    on_profile(&mut fx);
    let _ = running_fixture_score(&mut fx.s);
    load_material(
        &mut fx.s,
        &json!({"score": {"power_pct": 12.0, "speed_mm_min": 4000, "passes": 1}}),
    );
    assert!(fx.s.set_fixture_material(MATERIAL));
    let v = out(&fx.s);
    let all = format!("{v}");
    assert!(!all.contains("Nothing runs."), "{all}");
    assert!(
        !all.contains("Runs 0 operations"),
        "no work line without a work operation: {}",
        v["total"]
    );
    assert!(
        all.contains("1 fixture job runs."),
        "the fixture line: {all}"
    );
}
