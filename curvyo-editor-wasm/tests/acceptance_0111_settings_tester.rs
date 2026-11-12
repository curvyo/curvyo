//! Tester acceptance cases for `specs/0111-laser-operation-tools` on the unfolded row, written
//! from the specification before the implementation was read: one row at a time (criterion 17),
//! the upper part and No settings (18), Interval in `op_settings.main` (19, 23), the
//! operation-only settings with their ranges, defaults and kinds (20, 24), Z offset only with a
//! motorised Z axis (21, 22), the material paths that never touch them (23) and the two goldens
//! read in a session (25). Black box through the public session.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::too_many_lines,
    clippy::float_cmp,
    clippy::needless_pass_by_value,
    clippy::many_single_char_names,
    clippy::assert_is_empty,
    missing_docs
)]

use std::path::PathBuf;

use curvyo_document_core::{
    Direction, Document, DocumentSize, NewOperation, NewOutputPage, NodeId, Point, RectBounds,
    pack, unpack,
};
use curvyo_editor_wasm::{Session, Tool};
use serde_json::{Value, json};

const LASER: &str = "curvyo.laser";
const BIRCH: &str = "22222222-2222-4222-8222-222222222222";

struct Fx {
    s: Session,
    laser: String,
    m: NodeId,
    ops: Vec<NodeId>,
}

fn record_text(id: &str, name: &str, sets: Value) -> String {
    let mut history = Vec::new();
    let mut with_sources = serde_json::Map::new();
    for (i, (kind, set)) in sets.as_object().unwrap().iter().enumerate() {
        let entry = format!("aaaaaaaa-0000-4000-8000-00000000000{i}");
        history.push(json!({
            "id": entry, "date": 1_762_948_800_000_u64, "event": "edited", "kind": kind,
            "set": set
        }));
        let mut s = set.as_object().unwrap().clone();
        s.insert("source".into(), json!("edited"));
        with_sources.insert(kind.clone(), Value::Object(s));
    }
    serde_json::to_string_pretty(&json!({
        "format_version": 1, "id": id, "kind": "laser", "modified": 1_762_948_800_000_u64,
        "name": name, "thickness_mm": 3.0, "machine": {"id": "generic-laser", "name": "Generic laser"},
        "sets": with_sources, "history": history
    }))
    .unwrap()
}

fn birch_files() -> Vec<(String, String)> {
    vec![(
        format!("{BIRCH}.json"),
        record_text(
            BIRCH,
            "Birch",
            json!({
                "cut": {"power_pct": 60.0, "speed_mm_min": 300, "passes": 2},
                "engrave": {"power_pct": 35.0, "speed_mm_min": 6000, "passes": 1}
            }),
        ),
    )]
}

/// A laser page with one rectangle and an operation of each of `kinds`.
fn fx(kinds: &[&str]) -> Fx {
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
    let r = d.create_rect(RectBounds::from_corners(
        Point::new(l.origin.x + 10.0, l.origin.y + 10.0),
        Point::new(l.origin.x + 30.0, l.origin.y + 20.0),
    ));
    let m = made.kind.manufacturing().unwrap();
    let ops = kinds
        .iter()
        .enumerate()
        .map(|(i, kind)| {
            let name = format!("{kind} {}", i + 1);
            d.add_operation(m, &NewOperation { kind, name: &name }, i, &[vec![r]])
                .unwrap()
        })
        .collect();
    let mut s = Session::open(3, &pack(&d, "0111-settings").unwrap()).unwrap();
    s.resize_viewport(1600.0, 1000.0);
    s.set_tool(Tool::Select);
    s.load_materials(&birch_files(), None);
    assert!(s.activate_page(&made.id));
    s.wipe_history();
    Fx {
        s,
        laser: made.id,
        m,
        ops,
    }
}

impl Fx {
    fn row(&self, id: NodeId) -> Value {
        let want = serde_json::to_value(id).unwrap();
        let v: Value = serde_json::from_str(&self.s.operations_view_json()).unwrap();
        v["rows"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["id"] == want)
            .cloned()
            .unwrap()
    }
    fn settings(&self, id: NodeId) -> Value {
        self.row(id)["settings"].clone()
    }
    fn stored(&self, id: NodeId) -> (Option<Value>, Option<Value>) {
        let d = unpack(9, &self.s.pack("0.1.0").unwrap()).unwrap();
        let op = d
            .operations(self.m)
            .unwrap()
            .into_iter()
            .find(|o| o.id == id)
            .unwrap();
        (op.settings, op.options)
    }
    fn undo_name(&mut self) -> String {
        self.s
            .history_target_name(Direction::Undo)
            .unwrap_or_default()
    }
    /// Own numbers on operation `id`: power 40, speed 3000.
    fn own_numbers(&mut self, id: NodeId) {
        self.s
            .edit_operation_setting(id, "main", "power", 40.0)
            .unwrap();
        self.s
            .edit_operation_setting(id, "main", "speed", 3000.0)
            .unwrap();
    }
}

// ---- criterion 17: one row at a time ---------------------------------------------------------

#[test]
fn c17_one_row_unfolds_at_a_time_a_second_press_folds_and_no_step() {
    let mut f = fx(&["cut", "engrave", "score"]);
    let [a, b, c] = [f.ops[0], f.ops[1], f.ops[2]];
    for id in [a, b, c] {
        assert_eq!(f.row(id)["unfolded"], false);
    }
    assert!(f.s.unfold_operation_row(a));
    assert_eq!(f.row(a)["unfolded"], true);
    assert!(f.s.unfold_operation_row(b));
    assert_eq!(f.row(a)["unfolded"], false, "the row before folds");
    assert_eq!(f.row(b)["unfolded"], true);
    assert_eq!(f.row(c)["unfolded"], false);
    assert!(f.s.unfold_operation_row(b));
    assert_eq!(
        f.row(b)["unfolded"],
        false,
        "a press on the unfolded row folds it"
    );
    assert_eq!(f.s.undo_len(), 0, "unfolding is no step");
    let r = f.row(a);
    for k in ["show", "output", "name"] {
        assert!(!r[k].is_null(), "the row keeps {k}");
    }
}

#[test]
fn c17_the_unfolded_row_is_not_saved() {
    let mut f = fx(&["cut"]);
    let a = f.ops[0];
    assert!(f.s.unfold_operation_row(a));
    let bytes = f.s.pack("0.1.0").unwrap();
    let mut s = Session::open(4, &bytes).unwrap();
    assert!(s.activate_page(&f.laser));
    let v: Value = serde_json::from_str(&s.operations_view_json()).unwrap();
    assert_eq!(v["rows"][0]["unfolded"], false);
}

// ---- criterion 18: the upper part ------------------------------------------------------------

#[test]
fn c18_an_operation_without_numbers_reads_no_settings() {
    let f = fx(&["cut", "engrave"]);
    assert_eq!(f.settings(f.ops[0])["state"], "no_settings");
    assert_eq!(f.settings(f.ops[1])["state"], "no_settings");
}

#[test]
fn c18_own_numbers_and_a_material_pick_read_as_before() {
    let mut f = fx(&["cut"]);
    let a = f.ops[0];
    f.own_numbers(a);
    assert_eq!(f.settings(a)["state"], "own");
    assert!(f.s.use_material_on(&[a], BIRCH));
    let st = f.settings(a)["state"].as_str().unwrap().to_owned();
    assert!(st == "live" || st == "stored", "linked state {st}");
    assert_eq!(f.settings(a)["sets"][0]["run"]["power_pct"], 60.0);
}

// ---- criterion 19: Interval ------------------------------------------------------------------

#[test]
fn c19_interval_defaults_to_0_10_and_is_stored_in_main() {
    let mut f = fx(&["engrave"]);
    let a = f.ops[0];
    f.own_numbers(a);
    assert_eq!(f.settings(a)["interval_mm"], 0.1, "default 0.10");
    let len = f.s.undo_len();
    assert_eq!(f.s.edit_operation_option(a, "interval", 0.25), Ok(true));
    assert_eq!(f.s.undo_len(), len + 1);
    assert_eq!(f.undo_name(), "Edit settings");
    assert_eq!(f.settings(a)["interval_mm"], 0.25);
    let (settings, _) = f.stored(a);
    assert_eq!(settings.unwrap()["main"]["interval_mm"], 0.25);
    assert_eq!(
        f.s.edit_operation_option(a, "interval", 0.25),
        Ok(false),
        "same value"
    );
    assert_eq!(f.s.undo_len(), len + 1);
}

#[test]
fn c19_c24_interval_out_of_range_or_three_decimals_is_refused_with_the_range() {
    let mut f = fx(&["engrave"]);
    let a = f.ops[0];
    f.own_numbers(a);
    let len = f.s.undo_len();
    for bad in [0.005, 0.0, -0.1, 2.01, f64::NAN] {
        let err = f.s.edit_operation_option(a, "interval", bad);
        assert!(err.is_err(), "{bad} accepted, stored {:?}", f.stored(a).0);
        if !bad.is_nan() {
            let text = err.unwrap_err();
            assert!(
                text.contains("0.01") && text.contains('2'),
                "{bad}: message {text:?}"
            );
        }
    }
    assert_eq!(f.s.undo_len(), len, "nothing written");
    assert_eq!(f.s.edit_operation_option(a, "interval", 0.01), Ok(true));
    assert_eq!(f.s.edit_operation_option(a, "interval", 2.0), Ok(true));
}

#[test]
fn c19_no_interval_on_a_cut_score_or_cut_out() {
    let mut f = fx(&["cut", "score", "cut_out"]);
    for &id in &f.ops.clone() {
        f.own_numbers(id);
        let len = f.s.undo_len();
        let r = f.s.edit_operation_option(id, "interval", 0.2);
        assert!(
            r == Ok(false) || r.is_err(),
            "interval on a non-hatching kind: {r:?}"
        );
        assert_eq!(f.s.undo_len(), len);
        assert!(f.settings(id)["interval_mm"].is_null());
    }
}

#[test]
fn c19_c23_a_linked_interval_is_written_to_main_and_kept_by_every_material_path() {
    let mut f = fx(&["engrave"]);
    let a = f.ops[0];
    assert!(f.s.use_material_on(&[a], BIRCH));
    assert_eq!(f.s.edit_operation_option(a, "interval", 0.3), Ok(true));
    let (settings, _) = f.stored(a);
    let settings = settings.unwrap();
    assert_eq!(settings["main"]["interval_mm"], 0.3, "in main: {settings}");
    let over = settings.get("overrides").cloned().unwrap_or(Value::Null);
    assert!(
        over.pointer("/main/interval_mm").is_none(),
        "a typed interval is never an override: {settings}"
    );
    assert!(!settings["material"].is_null(), "still linked");
    // Reset, Use material again, a material edit, Use own numbers.
    let _ = f.s.reset_operation_to_material(a);
    assert_eq!(f.settings(a)["interval_mm"], 0.3, "after Reset to material");
    let _ = f.s.use_material_on(&[a], BIRCH);
    assert_eq!(f.settings(a)["interval_mm"], 0.3, "after Use material");
    assert_eq!(
        f.s.edit_material_set(BIRCH, "engrave", "power", 50.0, &[7; 16]),
        Ok(true),
        "the material edit"
    );
    assert_eq!(f.settings(a)["interval_mm"], 0.3, "after a material edit");
    let _ = f.s.pack_for_save();
    assert_eq!(
        f.settings(a)["interval_mm"],
        0.3,
        "after the save-time refresh"
    );
}

// ---- criterion 20, 24: the operation-only settings -------------------------------------------

#[test]
fn c20_engrave_shows_the_four_hatch_options_with_their_defaults() {
    let f = fx(&["engrave", "engrave_outline", "cut", "score", "cut_out"]);
    for &id in &f.ops[..2] {
        let o = f.settings(id)["options"].clone();
        assert_eq!(o["hatch"]["crosshatch"], false);
        assert_eq!(o["hatch"]["bidirectional"], true);
        assert_eq!(o["hatch"]["scan_angle_deg"], 0.0);
        assert_eq!(o["hatch"]["overscan_pct"], 2);
        assert_eq!(o["z_offset_mm"], 0.0);
    }
    for &id in &f.ops[2..] {
        let o = f.settings(id)["options"].clone();
        assert!(
            o["hatch"].is_null(),
            "no hatch options on {:?}",
            f.row(id)["kind"]
        );
        assert_eq!(o["z_offset_mm"], 0.0);
    }
}

#[test]
fn c20_c24_each_option_edit_is_one_step_and_writes_op_options_only() {
    let mut f = fx(&["engrave"]);
    let a = f.ops[0];
    let (settings_before, _) = f.stored(a);
    let mut len = f.s.undo_len();
    for (opt, v) in [
        ("crosshatch", 1.0),
        ("bidirectional", 0.0),
        ("scan_angle", 15.0),
        ("overscan", 5.0),
    ] {
        assert_eq!(f.s.edit_operation_option(a, opt, v), Ok(true), "{opt}");
        len += 1;
        assert_eq!(f.s.undo_len(), len, "{opt}: one step");
        assert_eq!(f.undo_name(), "Edit settings", "{opt}");
    }
    let (settings, options) = f.stored(a);
    assert_eq!(settings, settings_before, "op_settings untouched");
    let o = options.unwrap();
    assert_eq!(o["crosshatch"], true);
    assert_eq!(o["bidirectional"], false);
    assert_eq!(o["scan_angle_deg"], 15.0);
    assert_eq!(o["overscan_pct"], 5);
    // Options also on "No settings".
    assert_eq!(f.settings(a)["state"], "no_settings");
    assert_eq!(f.settings(a)["options"]["hatch"]["crosshatch"], true);
    let _ = f.s.undo();
    assert_eq!(
        f.settings(a)["options"]["hatch"]["overscan_pct"],
        2,
        "undo one field"
    );
    assert_eq!(f.settings(a)["options"]["hatch"]["scan_angle_deg"], 15.0);
}

#[test]
fn c20_c24_ranges_of_scan_angle_and_overscan() {
    let mut f = fx(&["engrave"]);
    let a = f.ops[0];
    let len = f.s.undo_len();
    for bad in [90.1, -90.1, 1000.0] {
        let e = f.s.edit_operation_option(a, "scan_angle", bad);
        assert!(e.is_err(), "scan angle {bad} accepted");
        if bad.abs() > 90.0 {
            let t = e.unwrap_err();
            assert!(t.contains("90"), "scan angle message {t:?}");
        }
    }
    for bad in [51.0, -1.0, 2.5] {
        let e = f.s.edit_operation_option(a, "overscan", bad);
        assert!(e.is_err(), "overscan {bad} accepted");
        if bad != 2.5 {
            let t = e.unwrap_err();
            assert!(t.contains("50"), "overscan message {t:?}");
        }
    }
    assert_eq!(f.s.undo_len(), len, "nothing written");
    for ok in [90.0, -90.0, 12.5] {
        assert_eq!(
            f.s.edit_operation_option(a, "scan_angle", ok),
            Ok(true),
            "{ok}"
        );
    }
    for ok in [0.0, 50.0] {
        assert_eq!(
            f.s.edit_operation_option(a, "overscan", ok),
            Ok(true),
            "{ok}"
        );
    }
}

#[test]
fn c20_hatch_options_are_not_written_on_a_cut() {
    let mut f = fx(&["cut", "score", "cut_out"]);
    for &id in &f.ops.clone() {
        for (opt, v) in [
            ("crosshatch", 1.0),
            ("bidirectional", 0.0),
            ("scan_angle", 15.0),
            ("overscan", 5.0),
        ] {
            let r = f.s.edit_operation_option(id, opt, v);
            assert!(
                r == Ok(false) || r.is_err(),
                "{opt} on a non-hatching kind: {r:?}"
            );
        }
        assert_eq!(f.stored(id).1, None, "nothing stored");
    }
    assert_eq!(f.s.undo_len(), 0);
}

#[test]
fn c20_c24_extra_decimals_never_store_a_damaged_value() {
    let mut f = fx(&["engrave"]);
    let a = f.ops[0];
    f.own_numbers(a);
    assert!(f.s.set_machine_motorised_z(true));
    for (opt, v) in [
        ("interval", 0.125),
        ("scan_angle", 15.05),
        ("z_offset", 1.25),
    ] {
        let r = f.s.edit_operation_option(a, opt, v);
        assert!(r.is_ok() || r.is_err(), "{opt}");
        assert_ne!(
            f.settings(a)["state"],
            "damaged",
            "{opt} {v} stored a value with too many decimals: {:?}",
            f.stored(a)
        );
    }
}

// ---- criteria 21, 22: Z offset and the motorised Z axis --------------------------------------

#[test]
fn c21_z_offset_is_absent_and_not_writable_without_a_motorised_z_axis() {
    let mut f = fx(&["cut", "engrave"]);
    for &id in &f.ops.clone() {
        assert_eq!(f.settings(id)["z_offset_shown"], false);
        assert_eq!(f.s.edit_operation_option(id, "z_offset", 1.5), Ok(false));
    }
    assert_eq!(f.s.undo_len(), 0);
}

#[test]
fn c22_the_capability_is_one_step_and_shows_z_offset() {
    let mut f = fx(&["cut"]);
    let a = f.ops[0];
    assert!(f.s.set_machine_motorised_z(true));
    assert_eq!(f.s.undo_len(), 1);
    assert_eq!(f.undo_name(), "Motorised Z axis on");
    assert_eq!(f.settings(a)["z_offset_shown"], true);
    assert!(!f.s.set_machine_motorised_z(true), "already on: nothing");
    assert_eq!(f.s.undo_len(), 1);
    assert!(f.s.set_machine_motorised_z(false));
    assert_eq!(f.undo_name(), "Motorised Z axis off");
    assert_eq!(f.settings(a)["z_offset_shown"], false);
    let _ = f.s.undo();
    assert_eq!(f.settings(a)["z_offset_shown"], true, "undo brings it back");
}

#[test]
fn c21_c24_z_offset_range_and_a_stored_value_survives_the_capability_off() {
    let mut f = fx(&["cut"]);
    let a = f.ops[0];
    assert!(f.s.set_machine_motorised_z(true));
    let len = f.s.undo_len();
    for bad in [20.1, -20.1] {
        assert!(
            f.s.edit_operation_option(a, "z_offset", bad).is_err(),
            "{bad}"
        );
    }
    let t = f.s.edit_operation_option(a, "z_offset", 25.0).unwrap_err();
    assert!(t.contains("20"), "message {t:?}");
    assert_eq!(f.s.undo_len(), len);
    assert_eq!(f.s.edit_operation_option(a, "z_offset", -1.5), Ok(true));
    assert_eq!(f.undo_name(), "Edit settings");
    assert!(f.s.set_machine_motorised_z(false));
    assert_eq!(f.settings(a)["z_offset_shown"], false);
    assert_eq!(f.stored(a).1.unwrap()["z_offset_mm"], -1.5, "kept");
    // Saved and reopened with the capability off: kept, then shown again when it returns.
    let bytes = f.s.pack_for_save().unwrap();
    let mut s = Session::open(5, &bytes).unwrap();
    assert!(s.activate_page(&f.laser));
    let d = unpack(9, &s.pack("0.1.0").unwrap()).unwrap();
    let op = d
        .operations(f.m)
        .unwrap()
        .into_iter()
        .find(|o| o.id == a)
        .unwrap();
    assert_eq!(op.options.unwrap()["z_offset_mm"], -1.5);
    assert!(s.set_machine_motorised_z(true));
    let v: Value = serde_json::from_str(&s.operations_view_json()).unwrap();
    assert_eq!(v["rows"][0]["settings"]["z_offset_shown"], true);
    assert_eq!(v["rows"][0]["settings"]["options"]["z_offset_mm"], -1.5);
}

#[test]
fn c22_the_capability_is_in_the_project_copy_after_a_reopen() {
    let mut f = fx(&["engrave"]);
    assert!(f.s.set_machine_motorised_z(true));
    let bytes = f.s.pack_for_save().unwrap();
    let mut s = Session::open(5, &bytes).unwrap();
    assert!(s.activate_page(&f.laser));
    let v: Value = serde_json::from_str(&s.operations_view_json()).unwrap();
    assert_eq!(
        v["rows"][0]["settings"]["z_offset_shown"], true,
        "read from the page copy, no profile file loaded"
    );
}

// ---- criterion 23: material paths never touch the options ------------------------------------

#[test]
fn c23_pick_and_edit_a_material_keep_crosshatch_and_scan_angle() {
    let mut f = fx(&["engrave"]);
    let a = f.ops[0];
    assert_eq!(f.s.edit_operation_option(a, "crosshatch", 1.0), Ok(true));
    assert_eq!(f.s.edit_operation_option(a, "scan_angle", 15.0), Ok(true));
    let (_, options) = f.stored(a);
    assert!(f.s.use_material_on(&[a], BIRCH));
    assert_eq!(f.stored(a).1, options, "Use material");
    let _ = f.s.edit_operation_setting(a, "main", "power", 41.0);
    let _ = f.s.reset_operation_to_material(a);
    assert_eq!(f.stored(a).1, options, "Reset to material");
    assert_eq!(
        f.s.edit_material_set(BIRCH, "engrave", "power", 50.0, &[7; 16]),
        Ok(true),
        "the material edit"
    );
    assert_eq!(f.stored(a).1, options, "a material edit");
    let _ = f.s.use_own_numbers_on(&[a]);
    assert_eq!(f.stored(a).1, options, "Use own numbers");
    let h = f.settings(a)["options"]["hatch"].clone();
    assert_eq!(h["crosshatch"], true);
    assert_eq!(h["scan_angle_deg"], 15.0);
}

// ---- criterion 25: the goldens in a session --------------------------------------------------

fn golden(name: &str) -> Vec<u8> {
    std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../curvyo-document-core/tests/fixtures")
            .join(name),
    )
    .unwrap()
}

/// Every laser page's rows of a session on `bytes`.
fn all_rows(bytes: &[u8]) -> Vec<Value> {
    let mut s = Session::open(5, bytes).unwrap();
    let pages: Value = serde_json::from_str(&s.pages_json()).unwrap();
    let list = pages
        .as_array()
        .cloned()
        .unwrap_or_else(|| pages["pages"].as_array().cloned().unwrap());
    let mut out = Vec::new();
    for p in list.iter().filter(|p| !p["manufacturing"].is_null()) {
        assert!(s.activate_page(p["id"].as_str().unwrap()));
        let v: Value = serde_json::from_str(&s.operations_view_json()).unwrap();
        out.extend(v["rows"].as_array().cloned().unwrap_or_default());
    }
    out
}

#[test]
fn c25_every_operation_of_the_damaged_golden_reads_damaged_settings() {
    let rows = all_rows(&golden("operation_options_damaged_v44.curvyo"));
    assert!(rows.len() >= 7, "seven damage cases, {} rows", rows.len());
    for r in &rows {
        assert_eq!(
            r["settings"]["state"], "damaged",
            "{}: {}",
            r["name"], r["settings"]
        );
    }
}

#[test]
fn c25_the_tools_golden_reads_its_interval_options_and_z() {
    let rows = all_rows(&golden("operation_tools_v44.curvyo"));
    let engr: Vec<&Value> = rows.iter().filter(|r| r["kind"] == "engrave").collect();
    assert!(!engr.is_empty());
    let e = engr[0]["settings"].clone();
    assert_ne!(e["state"], "damaged", "{e}");
    assert!(e["interval_mm"].is_number(), "{e}");
    assert!(!e["options"]["hatch"].is_null());
    for r in &rows {
        assert_ne!(
            r["settings"]["state"], "damaged",
            "{}: {}",
            r["name"], r["settings"]
        );
    }
    assert!(
        rows.iter().any(|r| r["settings"]["z_offset_shown"] == true),
        "the second manufacturing has the capability"
    );
    assert!(
        rows.iter()
            .any(|r| r["kind"] == "cut" && r["settings"]["z_offset_shown"] == false),
        "the Cut with a Z offset under a copy without it"
    );
}

#[test]
fn c25_a_format_43_file_shows_the_defaults() {
    let rows = all_rows(&golden("laser_settings_v43.curvyo"));
    for r in rows.iter().filter(|r| r["grid"].is_null()) {
        let st = &r["settings"];
        if st["state"] == "damaged" {
            continue;
        }
        assert_eq!(st["options"]["z_offset_mm"], 0.0, "{}", r["name"]);
        if r["kind"] == "engrave" || r["kind"] == "engrave_outline" {
            assert_eq!(st["options"]["hatch"]["bidirectional"], true);
            assert_eq!(st["options"]["hatch"]["overscan_pct"], 2);
            if st["state"] != "no_settings" {
                assert_eq!(st["interval_mm"], 0.1, "{}", r["name"]);
            }
        }
        assert_eq!(st["z_offset_shown"], false);
    }
}
