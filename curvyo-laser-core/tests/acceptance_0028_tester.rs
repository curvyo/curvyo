//! Tester acceptance cases for `specs/0028-material-test-library` in the laser extension,
//! written from the specification: the material record file (criteria 16, 17, 29: every
//! damage reads as damaged with a reason, a newer record is left alone, a read never writes,
//! the other records load), the library list per machine (19, 22, 23) and the `0029`-facing
//! rule of which numbers run (Where settings live 5, criterion 26: live numbers with the
//! overrides over them, the stored numbers when the record is not usable here).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::type_complexity,
    clippy::many_single_char_names,
    missing_docs
)]

use std::path::PathBuf;

use curvyo_laser_core::material_library::{
    MAX_MATERIAL_BYTES, MaterialFileError, Materials, read_material, write_material,
};
use curvyo_laser_core::operations::OperationKind;
use curvyo_laser_core::settings::{
    MaterialRead, SetKind, SettingsRead, SettingsState, StoredReason, read_settings, resolve,
};
use serde_json::{Value, json};

const ID: &str = "0b6c3f1e-2a4d-4b8e-9c7f-1d2e3f4a5b6c";
const MACHINE: &str = "6f1c2a4e-9b3d-4c1a-8e2f-0a1b2c3d4e5f";

fn full() -> Value {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/materials/full.json");
    serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap()
}

fn text(v: &Value) -> String {
    serde_json::to_string_pretty(v).unwrap()
}

fn file() -> String {
    format!("{ID}.json")
}

fn damaged(v: &Value) -> String {
    match read_material(&file(), &text(v)) {
        Err(MaterialFileError::Damaged(reason)) => reason,
        other => panic!("expected damaged, got {other:?}"),
    }
}

// ---- criterion 29: every damage -----------------------------------------------------------------

#[test]
fn c29_the_full_record_reads_and_writes_back_stably() {
    let m = read_material(&file(), &text(&full())).unwrap();
    assert_eq!(m.name, "Birch plywood");
    assert_eq!(m.set(SetKind::Cut).unwrap().power.percent(), 60.0);
    let w = write_material(&m);
    let again = read_material(&file(), &w).unwrap();
    assert_eq!(write_material(&again), w, "write(read(write)) is stable");
    let a: Value = serde_json::from_str(&w).unwrap();
    assert_eq!(
        a["history"],
        full()["history"],
        "the history survives a round trip"
    );
    assert_eq!(a["format_version"], 1);
}

#[test]
fn c29_each_invalid_value_makes_the_record_damaged_with_a_reason() {
    let cases: Vec<(&str, Box<dyn Fn(&mut Value)>)> = vec![
        ("unknown key", Box::new(|v| v["colour"] = json!("red"))),
        (
            "unknown set key",
            Box::new(|v| v["sets"]["cut"]["focus"] = json!(1)),
        ),
        (
            "power 0.9",
            Box::new(|v| v["sets"]["cut"]["power_pct"] = json!(0.9)),
        ),
        (
            "power 100.1",
            Box::new(|v| v["sets"]["cut"]["power_pct"] = json!(100.1)),
        ),
        (
            "power two decimals",
            Box::new(|v| v["sets"]["cut"]["power_pct"] = json!(50.25)),
        ),
        (
            "speed 0",
            Box::new(|v| v["sets"]["score"]["speed_mm_min"] = json!(0)),
        ),
        (
            "speed negative",
            Box::new(|v| v["sets"]["score"]["speed_mm_min"] = json!(-5)),
        ),
        (
            "passes 0",
            Box::new(|v| v["sets"]["cut"]["passes"] = json!(0)),
        ),
        (
            "passes 21",
            Box::new(|v| v["sets"]["cut"]["passes"] = json!(21)),
        ),
        ("thickness 0", Box::new(|v| v["thickness_mm"] = json!(0.0))),
        (
            "thickness 0.05",
            Box::new(|v| v["thickness_mm"] = json!(0.05)),
        ),
        (
            "thickness 100.1",
            Box::new(|v| v["thickness_mm"] = json!(100.1)),
        ),
        ("name 61", Box::new(|v| v["name"] = json!("n".repeat(61)))),
        ("name empty", Box::new(|v| v["name"] = json!(""))),
        (
            "source without entry",
            Box::new(|v| {
                v["sets"]["cut"]["source"] =
                    json!({"test": "99999999-9999-4999-8999-999999999999"});
            }),
        ),
        (
            "id not the file name",
            Box::new(|v| v["id"] = json!("11111111-1111-4111-8111-111111111111")),
        ),
        (
            "unknown set kind",
            Box::new(|v| v["sets"]["weld"] = v["sets"]["cut"].clone()),
        ),
        (
            "no name",
            Box::new(|v| {
                v.as_object_mut().unwrap().remove("name");
            }),
        ),
        (
            "no modified",
            Box::new(|v| {
                v.as_object_mut().unwrap().remove("modified");
            }),
        ),
    ];
    for (what, edit) in cases {
        let mut v = full();
        edit(&mut v);
        let reason = damaged(&v);
        assert!(!reason.is_empty(), "{what}: a reason is named");
    }
}

/// Criterion 29: "text over 60 characters" is damage; the machine name at the time is text of
/// the record (a profile name is 1 to 60 characters, `0026`).
#[test]
fn c29_a_machine_name_over_60_characters_is_damaged() {
    let mut v = full();
    v["machine"]["name"] = json!("m".repeat(61));
    let _ = damaged(&v);
}

#[test]
fn c29_history_up_to_500_entries_reads_and_501_is_damaged() {
    let mut v = full();
    let base = v["history"][1].clone();
    let mut h = v["history"].as_array().unwrap().clone();
    let mut n = 0u32;
    while h.len() < 500 {
        let mut e = base.clone();
        e["id"] = json!(format!("00000000-0000-4000-8000-{n:012}"));
        h.push(e);
        n += 1;
    }
    v["history"] = Value::Array(h.clone());
    let t = text(&v);
    assert!(
        t.len() <= MAX_MATERIAL_BYTES,
        "500 entries fit in 256 KiB: {}",
        t.len()
    );
    read_material(&file(), &t).unwrap();
    let mut e = base.clone();
    e["id"] = json!("00000000-0000-4000-8000-ffffffffffff");
    h.push(e);
    v["history"] = Value::Array(h);
    let _ = damaged(&v);
}

#[test]
fn c29_a_record_over_256_kib_is_damaged() {
    assert_eq!(MAX_MATERIAL_BYTES, 256 * 1024);
    let t = text(&full());
    let padded = format!("{}{}", t, " ".repeat(MAX_MATERIAL_BYTES + 1 - t.len()));
    assert!(matches!(
        read_material(&file(), &padded),
        Err(MaterialFileError::Damaged(_))
    ));
    // Exactly at the limit is fine.
    let at = format!("{}{}", t, " ".repeat(MAX_MATERIAL_BYTES - t.len()));
    read_material(&file(), &at).unwrap();
}

#[test]
fn c17_a_newer_format_is_newer_not_damaged_even_with_unknown_keys() {
    let mut v = full();
    v["format_version"] = json!(2);
    v["a_future_key"] = json!({"x": 1});
    assert!(matches!(
        read_material(&file(), &text(&v)),
        Err(MaterialFileError::Newer)
    ));
}

#[test]
fn c17_c29_the_library_skips_and_names_damaged_records_and_loads_the_others() {
    let good = text(&full());
    let mut bad = full();
    bad["id"] = json!("33333333-3333-4333-8333-333333333333");
    bad["sets"]["cut"]["speed_mm_min"] = json!(0);
    let mut newer = full();
    newer["id"] = json!("44444444-4444-4444-8444-444444444444");
    newer["format_version"] = json!(7);
    let files = vec![
        (file(), good.clone()),
        (
            "33333333-3333-4333-8333-333333333333.json".to_owned(),
            text(&bad),
        ),
        (
            "44444444-4444-4444-8444-444444444444.json".to_owned(),
            text(&newer),
        ),
        (
            "55555555-5555-4555-8555-555555555555.json".to_owned(),
            "\u{0}garbage".to_owned(),
        ),
    ];
    let lib = Materials::load(&files, None);
    assert!(lib.find(ID).is_some(), "the good record loads");
    assert_eq!(lib.reads().len(), 4);
    let damaged = lib
        .reads()
        .iter()
        .filter(|r| matches!(r, MaterialRead::Damaged { .. }))
        .count();
    assert_eq!(damaged, 2);
    assert!(
        lib.reads()
            .iter()
            .any(|r| matches!(r, MaterialRead::Newer { .. }))
    );
    assert!(lib.load_notices().len() >= 3, "{:?}", lib.load_notices());
    // The texts are kept as read (a read never writes): the damaged one is unchanged.
    assert_eq!(lib.text(ID), Some(good.as_str()));
}

#[test]
fn c23_the_list_of_a_machine_holds_only_its_records() {
    let mut other = full();
    other["id"] = json!("66666666-6666-4666-8666-666666666666");
    other["machine"]["id"] = json!("generic-laser");
    let files = vec![
        (file(), text(&full())),
        (
            "66666666-6666-4666-8666-666666666666.json".to_owned(),
            text(&other),
        ),
    ];
    let lib = Materials::load(&files, None);
    let ids: Vec<&str> = lib.listed(MACHINE).iter().map(|r| r.id()).collect();
    assert_eq!(ids, vec![ID]);
    let ids: Vec<&str> = lib.listed("generic-laser").iter().map(|r| r.id()).collect();
    assert_eq!(ids, vec!["66666666-6666-4666-8666-666666666666"]);
}

// ---- the 0029-facing rule: which numbers run ----------------------------------------------------

fn materials_with(cut_power: f64, machine: &str) -> Vec<MaterialRead> {
    let mut v = full();
    v["sets"]["cut"]["power_pct"] = json!(cut_power);
    v["history"][0]["set"]["power_pct"] = json!(cut_power);
    v["machine"]["id"] = json!(machine);
    vec![MaterialRead::Loaded(
        read_material(&file(), &text(&v)).unwrap(),
    )]
}

fn linked_cut(overrides: Option<Value>) -> Value {
    let mut s = json!({
        "main": {"power_pct": 55.0, "speed_mm_min": 280, "passes": 2},
        "material": {"id": ID, "name": "Old name", "modified": 1_000}
    });
    if let Some(o) = overrides {
        s["overrides"] = o;
    }
    s
}

fn run_of(state: &SettingsState) -> (f64, u32, u64) {
    let run = state.run().unwrap();
    (
        run.main.power.percent(),
        run.main.speed.get(),
        u64::from(run.main.passes.get()),
    )
}

#[test]
fn rule_live_numbers_with_the_overrides_put_over_them() {
    let v = linked_cut(Some(json!({"main": {"passes": 3}})));
    let read = read_settings(Some(&v), OperationKind::Cut);
    assert!(matches!(read, SettingsRead::Stored(_)));
    let st = resolve(
        &read,
        OperationKind::Cut,
        &materials_with(62.5, MACHINE),
        MACHINE,
    );
    assert!(matches!(st, SettingsState::Live { .. }), "{st:?}");
    assert_eq!(run_of(&st), (62.5, 300, 3));
    let m = st.material().unwrap();
    assert_eq!(
        m.main.passes.get(),
        2,
        "the material's own passes beside the typed 3"
    );
}

#[test]
fn rule_stored_numbers_with_overrides_when_the_record_is_missing_or_elsewhere() {
    let v = linked_cut(Some(json!({"main": {"power_pct": 70.0}})));
    let read = read_settings(Some(&v), OperationKind::Cut);
    let missing = resolve(&read, OperationKind::Cut, &[], MACHINE);
    match &missing {
        SettingsState::Stored { reason, name, .. } => {
            assert_eq!(*reason, StoredReason::Missing);
            assert_eq!(name, "Old name");
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(run_of(&missing), (70.0, 280, 2));
    let other_machine = resolve(
        &read,
        OperationKind::Cut,
        &materials_with(62.5, "generic-laser"),
        MACHINE,
    );
    assert!(matches!(
        other_machine,
        SettingsState::Stored {
            reason: StoredReason::OtherMachine,
            ..
        }
    ));
    assert_eq!(run_of(&other_machine), (70.0, 280, 2));
}

#[test]
fn rule_no_settings_and_damaged_run_nothing() {
    let none = read_settings(None, OperationKind::Score);
    assert!(
        resolve(&none, OperationKind::Score, &[], MACHINE)
            .run()
            .is_none()
    );
    for bad in [
        json!(5),
        json!({"main": {"power_pct": 0.0, "speed_mm_min": 100, "passes": 1}}),
        json!({"main": {"power_pct": 50.0, "speed_mm_min": 100, "passes": 1}, "overrides": {"main": {"passes": 2}}}),
        json!({"main": {"power_pct": 50.0, "speed_mm_min": 100, "passes": 1}, "outline": {"power_pct": 50.0, "speed_mm_min": 100, "passes": 1}}),
        json!({"main": {"power_pct": 50.0, "speed_mm_min": 100, "passes": 1}, "material": {"id": ID, "name": "x", "modified": 1}, "overrides": {}}),
        json!({"main": {"power_pct": 50.0, "speed_mm_min": 100, "passes": 1}, "material": {"name": "x", "modified": 1}}),
        json!({"main": {"power_pct": 50.0, "speed_mm_min": 100, "passes": 1}, "zap": 1}),
    ] {
        let read = read_settings(Some(&bad), OperationKind::Cut);
        assert!(matches!(read, SettingsRead::Damaged(_)), "{bad}");
        assert!(
            resolve(
                &read,
                OperationKind::Cut,
                &materials_with(60.0, MACHINE),
                MACHINE
            )
            .run()
            .is_none(),
            "{bad}"
        );
    }
}

#[test]
fn rule_a_speed_above_the_max_feed_is_not_damage() {
    let v = json!({"main": {"power_pct": 50.0, "speed_mm_min": 60_000, "passes": 1}});
    let read = read_settings(Some(&v), OperationKind::Engrave);
    assert!(matches!(read, SettingsRead::Stored(_)));
}
