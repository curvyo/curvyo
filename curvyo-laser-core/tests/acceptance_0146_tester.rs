//! Acceptance tests for `specs/0146-material-test-picks` (criteria 2, 3, 10, 11, 23), written
//! black-box from the specification and ADR 0090 before the implementation was read. They use
//! only the public record API: records are JSON text in, `Material` and `write_record` out.
#![allow(clippy::expect_used, clippy::too_many_lines, clippy::unwrap_used)]

use curvyo_laser_core::grids::TestId;
use curvyo_laser_core::library::{
    MAX_RECORD_TEXT, MaterialRead, RECORD_FORMAT_VERSION, RecordRefusal, Stamp, load_record,
    read_record, write_record,
};
use curvyo_laser_core::settings::{CellIndex, PickKind, SetKind};
use serde_json::Value;

const FILE: &str = "0b6c3f1e-2a4d-4b8e-9c7f-1d2e3f4a5b6c.json";
const PROFILE: &str = "6f1c2a4e-9b3d-4c1a-8e2f-0a1b2c3d4e5f";
const MACHINE: &str = r#"{"id":"6f1c2a4e-9b3d-4c1a-8e2f-0a1b2c3d4e5f","name":"Diode 20 W"}"#;
const STAMP: Stamp<'static> = Stamp {
    entry_id: "00000000-0000-4000-8000-0000000000aa",
    now: 1_762_949_000_000,
};

fn set(power: f64, speed: u32, source: &str) -> String {
    format!(r#"{{"power_pct":{power},"speed_mm_min":{speed},"passes":1,"source":{source}}}"#)
}

fn engrave_set(power: f64, speed: u32, interval: f64, source: &str) -> String {
    format!(
        r#"{{"power_pct":{power},"speed_mm_min":{speed},"passes":1,"interval_mm":{interval},"source":{source}}}"#
    )
}

/// A 5 x 5 Cut grid (power 20 to 100 by 20, speed 600 to 6000 by 1350) with `id`.
fn cut_test(id: &str, marks: &str) -> String {
    format!(
        r#"{{"id":"{id}","created":1762948800000,"machine":{MACHINE},"recipe":{{"kind":"cut","columns":{{"param":"power","min":20.0,"max":100.0,"count":5}},"rows":{{"param":"speed","min":600,"max":6000,"count":5}},"fixed":{{"passes":1}},"cell_mm":10.0,"crosshatch":false}},"lasered":1762948801000{marks},"note":""}}"#
    )
}

/// A 5 x 5 Engrave grid (power 10 to 50, interval 0.05 to 0.25 mm) with `id`.
fn engrave_test(id: &str, marks: &str) -> String {
    format!(
        r#"{{"id":"{id}","created":1762948803000,"machine":{MACHINE},"recipe":{{"kind":"engrave","columns":{{"param":"power","min":10.0,"max":50.0,"count":5}},"rows":{{"param":"interval","min":0.05,"max":0.25,"count":5}},"fixed":{{"speed_mm_min":3000,"passes":1}},"cell_mm":8.0,"crosshatch":true}},"lasered":null,"note":""{marks}}}"#
    )
}

/// A record of `format` with the given `lasers` entries and `tests` objects (JSON text).
fn record(format: u64, lasers: &str, tests: &str) -> String {
    format!(
        r#"{{"format_version":{format},"id":"0b6c3f1e-2a4d-4b8e-9c7f-1d2e3f4a5b6c","kind":"laser","modified":1762948805000,"name":"Birch plywood","thickness_mm":3.0,"lasers":[{lasers}],"tests":[{tests}],"history":[]}}"#
    )
}

/// One laser with a Cut set from test T1 and the given `picks` object text (or none).
fn laser_with(picks: Option<&str>) -> String {
    let picks = picks.map_or(String::new(), |p| format!(r#","picks":{p}"#));
    format!(
        r#"{{"machine":{MACHINE},"sets":{{"cut":{}}}{picks}}}"#,
        set(60.0, 1950, r#"{"test":"T0001"}"#)
    )
}

/// The valid base: test T0001 (Best at column 2, row 1) and an Engrave test E0001 for the
/// kind-mismatch cases.
fn base_tests() -> String {
    format!(
        "{},{}",
        cut_test("T0001", r#","best":{"column":2,"row":1}"#),
        engrave_test("E0001", "")
    )
}

fn loaded(text: &str) -> curvyo_laser_core::library::Material {
    match load_record(FILE, text) {
        MaterialRead::Loaded(material) => material,
        other => panic!("expected Loaded, got {other:?}"),
    }
}

fn damaged_reason(text: &str) -> String {
    match load_record(FILE, text) {
        MaterialRead::Damaged { reason, .. } => reason,
        other => panic!("expected Damaged, got {other:?}"),
    }
}

// ---- Criterion 1, 6 (order), ADR 0090 decision 1: the four kinds ----

#[test]
fn four_pick_kinds_in_order_with_stored_names() {
    assert_eq!(
        PickKind::ALL,
        [
            PickKind::Best,
            PickKind::SecondBest,
            PickKind::Fastest,
            PickKind::Deepest
        ]
    );
    let names: Vec<&str> = PickKind::ALL.iter().map(|k| k.as_str()).collect();
    assert_eq!(names, ["best", "second_best", "fastest", "deepest"]);
    assert_eq!(PickKind::parse("cleanest"), None);
    assert_eq!(PickKind::parse("second_best"), Some(PickKind::SecondBest));
}

// ---- Criterion 10: a pick survives the deletion of its test ----

#[test]
fn pick_with_deleted_test_source_is_valid_in_format_5() {
    let lasers = format!(
        r#"{{"machine":{MACHINE},"sets":{{"cut":{}}},"picks":{{"cut":{{"fastest":{}}}}}}}"#,
        set(60.0, 1950, r#"{"test":"GONE1"}"#),
        set(80.0, 4650, r#"{"test":"GONE1"}"#),
    );
    let text = record(5, &lasers, &base_tests());
    let material = loaded(&text);
    material
        .pick_set(PROFILE, SetKind::Cut, PickKind::Fastest)
        .expect("the Fastest set survives");
    assert_eq!(
        written(&material)["lasers"][0]["picks"]["cut"]["fastest"]["source"]["test"],
        "GONE1"
    );
    assert_eq!(
        written(&material)["lasers"][0]["picks"]["cut"]["fastest"]["speed_mm_min"],
        4650
    );
}

#[test]
fn set_with_deleted_test_source_is_valid_in_format_4_too() {
    let lasers = format!(
        r#"{{"machine":{MACHINE},"sets":{{"cut":{}}}}}"#,
        set(60.0, 1950, r#"{"test":"GONE1"}"#)
    );
    loaded(&record(4, &lasers, &base_tests()));
}

#[test]
fn set_source_naming_a_present_test_of_another_kind_is_damage() {
    // The set is Cut but its source is the Engrave test E0001 in the same record.
    let lasers = format!(
        r#"{{"machine":{MACHINE},"sets":{{"cut":{}}}}}"#,
        set(60.0, 1950, r#"{"test":"E0001"}"#)
    );
    damaged_reason(&record(5, &lasers, &base_tests()));
}

// ---- Criterion 11: old records open unchanged; newer ones are left alone ----

#[test]
fn format_2_record_without_picks_opens_and_shows_its_best() {
    let lasers = format!(
        r#"{{"machine":{MACHINE},"sets":{{"cut":{}}}}}"#,
        set(90.0, 300, r#""typed""#)
    );
    let material = loaded(&record(2, &lasers, ""));
    assert!(material.set_for(PROFILE, SetKind::Cut).is_some());
    assert!(
        material
            .pick_set(PROFILE, SetKind::Cut, PickKind::Fastest)
            .is_none()
    );
}

#[test]
fn format_6_record_is_newer_and_skipped() {
    let text = record(6, "", "");
    assert!(matches!(
        load_record(FILE, &text),
        MaterialRead::Newer { .. }
    ));
    assert!(read_record(FILE, &text).is_err());
}

#[test]
fn picks_below_format_5_are_damage() {
    let lasers = laser_with(Some(
        r#"{"cut":{"fastest":{"power_pct":80.0,"speed_mm_min":4650,"passes":1,"source":{"test":"T0001"}}}}"#,
    ));
    damaged_reason(&record(4, &lasers, &base_tests()));
    let tests = cut_test("T0001", r#","fastest":{"column":3,"row":3}"#);
    damaged_reason(&record(4, &laser_with(None), &tests));
}

#[test]
fn format_5_record_with_all_four_kinds_round_trips_to_format_5() {
    let picks = format!(
        r#"{{"cut":{{"second_best":{},"fastest":{},"deepest":{}}},"engrave":{{"fastest":{}}}}}"#,
        set(40.0, 1950, r#"{"test":"T0001"}"#),
        set(80.0, 4650, r#"{"test":"T0001"}"#),
        set(80.0, 4650, r#"{"test":"T0001"}"#),
        engrave_set(30.0, 3000, 0.1, r#"{"test":"E0001"}"#),
    );
    let tests = format!(
        "{},{}",
        cut_test(
            "T0001",
            r#","best":{"column":2,"row":1},"second_best":{"column":1,"row":1},"fastest":{"column":3,"row":3},"deepest":{"column":3,"row":3}"#
        ),
        engrave_test("E0001", r#","fastest":{"column":0,"row":4}"#),
    );
    let lasers = format!(
        r#"{{"machine":{MACHINE},"sets":{{"cut":{}}},"picks":{picks}}}"#,
        set(60.0, 1950, r#"{"test":"T0001"}"#)
    );
    let text = record(5, &lasers, &tests);
    let written = write_record(&loaded(&text));
    let value: Value = serde_json::from_str(&written).expect("record is JSON");
    assert_eq!(value["format_version"], 5);
    assert_eq!(
        value["lasers"][0]["picks"]["cut"]["fastest"]["speed_mm_min"],
        4650
    );
    assert_eq!(
        value["tests"][0]["deepest"],
        serde_json::json!({"column":3,"row":3})
    );
    // Reading what was written gives the same record again (nothing lost, nothing changed).
    assert_eq!(write_record(&loaded(&written)), written);
}

// ---- Criterion 11 (size): the limit is 640 KiB ----

#[test]
fn record_limit_is_640_kib() {
    assert_eq!(MAX_RECORD_TEXT, 640 * 1024);
    assert_eq!(RECORD_FORMAT_VERSION, 5);
}

#[test]
fn record_of_600_kib_is_accepted_and_700_kib_is_refused() {
    let text = record(5, &laser_with(None), &base_tests());
    let padded = format!("{text}{}", " ".repeat(600 * 1024 - text.len()));
    loaded(&padded);
    let too_big = format!("{text}{}", " ".repeat(700 * 1024 - text.len()));
    assert!(damaged_reason(&too_big).contains("640 KiB"));
}

// ---- Criterion 23 (ADR 0090 decision 5): damage cases, each read as Damaged ----

#[test]
fn valid_format_5_pick_control_loads() {
    let picks = format!(
        r#"{{"cut":{{"fastest":{}}},"engrave":{{"second_best":{}}}}}"#,
        set(80.0, 4650, r#"{"test":"T0001"}"#),
        engrave_set(10.0, 3000, 0.05, r#"{"test":"E0001"}"#),
    );
    loaded(&record(5, &laser_with(Some(&picks)), &base_tests()));
}

#[test]
fn damaged_format_5_pick_records_are_each_refused() {
    let cut_pick = set(80.0, 4650, r#"{"test":"T0001"}"#);
    let cases: Vec<(&str, String, String)> = vec![
        (
            "mark outside the counts",
            record(
                5,
                &laser_with(None),
                &cut_test("T0001", r#","fastest":{"column":5,"row":0}"#),
            ),
            String::new(),
        ),
        (
            "best inside picks",
            record(
                5,
                &laser_with(Some(&format!(r#"{{"cut":{{"best":{cut_pick}}}}}"#))),
                &base_tests(),
            ),
            String::new(),
        ),
        (
            "kind key not cut, score, engrave or image",
            record(
                5,
                &laser_with(Some(&format!(r#"{{"pen":{{"fastest":{cut_pick}}}}}"#))),
                &base_tests(),
            ),
            String::new(),
        ),
        (
            "engrave pick without interval",
            record(
                5,
                &laser_with(Some(&format!(
                    r#"{{"engrave":{{"fastest":{}}}}}"#,
                    set(30.0, 3000, r#"{"test":"E0001"}"#)
                ))),
                &base_tests(),
            ),
            String::new(),
        ),
        (
            "interval on a cut pick",
            record(
                5,
                &laser_with(Some(&format!(
                    r#"{{"cut":{{"fastest":{}}}}}"#,
                    engrave_set(30.0, 3000, 0.1, r#"{"test":"T0001"}"#)
                ))),
                &base_tests(),
            ),
            String::new(),
        ),
        (
            "pick source typed",
            record(
                5,
                &laser_with(Some(&format!(
                    r#"{{"cut":{{"fastest":{}}}}}"#,
                    set(80.0, 4650, r#""typed""#)
                ))),
                &base_tests(),
            ),
            String::new(),
        ),
        (
            "pick source a present test of another kind",
            record(
                5,
                &laser_with(Some(&format!(
                    r#"{{"cut":{{"fastest":{}}}}}"#,
                    set(80.0, 4650, r#"{"test":"E0001"}"#)
                ))),
                &base_tests(),
            ),
            String::new(),
        ),
        (
            "empty picks map",
            record(5, &laser_with(Some("{}")), &base_tests()),
            String::new(),
        ),
        (
            "empty kind map",
            record(5, &laser_with(Some(r#"{"cut":{}}"#)), &base_tests()),
            String::new(),
        ),
        (
            "source that is not a test",
            record(
                5,
                &laser_with(Some(&format!(
                    r#"{{"cut":{{"fastest":{}}}}}"#,
                    set(80.0, 4650, r#"{"history":"x"}"#)
                ))),
                &base_tests(),
            ),
            String::new(),
        ),
        (
            "mark with a non-number column",
            record(
                5,
                &laser_with(None),
                &cut_test("T0001", r#","deepest":{"column":"a","row":0}"#),
            ),
            String::new(),
        ),
    ];
    for (name, text, _) in &cases {
        let reason = match load_record(FILE, text) {
            MaterialRead::Damaged { reason, .. } => reason,
            other => panic!("case '{name}': expected Damaged, got {other:?}"),
        };
        assert!(!reason.is_empty(), "case '{name}' names a reason");
        // A wrong-typed mark is refused by the generic envelope text (reported, not a FAIL).
        assert!(
            *name == "mark with a non-number column" || !reason.contains("not a material record"),
            "case '{name}' must fail for its own reason, not a malformed file: {reason}"
        );
    }
}

// ---- Criteria 2, 3, 10: marking, unmarking and deleting picks through Material ----

fn stamp_test(id: &str) -> TestId {
    TestId::parse(id).expect("valid test id")
}

/// The material as the record file writes it (the public contract, read back as JSON).
fn written(material: &curvyo_laser_core::library::Material) -> Value {
    serde_json::from_str(&write_record(material)).expect("record text is JSON")
}

#[test]
fn judge_fastest_writes_the_pick_set_from_the_cell_and_moves_the_mark() {
    let mut material = loaded(&record(5, &laser_with(None), &base_tests()));
    let t1 = stamp_test("T0001");
    material
        .judge(
            t1,
            CellIndex { column: 3, row: 3 },
            PickKind::Fastest,
            STAMP,
        )
        .expect("judged");
    material
        .pick_set(PROFILE, SetKind::Cut, PickKind::Fastest)
        .expect("set");
    assert_eq!(
        written(&material)["lasers"][0]["picks"]["cut"]["fastest"]["source"]["test"],
        "T0001"
    );
    let v = written(&material);
    assert_eq!(
        v["lasers"][0]["picks"]["cut"]["fastest"]["speed_mm_min"],
        4650
    );
    assert_eq!(v["lasers"][0]["picks"]["cut"]["fastest"]["power_pct"], 80.0);
    // Judging the same cell again is no change.
    assert_eq!(
        material.judge(
            t1,
            CellIndex { column: 3, row: 3 },
            PickKind::Fastest,
            STAMP
        ),
        Err(RecordRefusal::Unchanged)
    );
    // Another cell moves the mark and the pick set; Best is untouched.
    material
        .judge(
            t1,
            CellIndex { column: 1, row: 1 },
            PickKind::Fastest,
            STAMP,
        )
        .expect("moved");
    let v = written(&material);
    assert_eq!(v["lasers"][0]["picks"]["cut"]["fastest"]["power_pct"], 40.0);
    assert_eq!(
        v["lasers"][0]["picks"]["cut"]["fastest"]["speed_mm_min"],
        1950
    );
    assert_eq!(v["lasers"][0]["sets"]["cut"]["power_pct"], 60.0);
    assert_eq!(
        v["tests"][0]["fastest"],
        serde_json::json!({"column":1,"row":1})
    );
}

#[test]
fn judge_refuses_a_cell_outside_the_counts() {
    let mut material = loaded(&record(5, &laser_with(None), &base_tests()));
    let refused = material.judge(
        stamp_test("T0001"),
        CellIndex { column: 5, row: 0 },
        PickKind::Deepest,
        STAMP,
    );
    assert_eq!(refused, Err(RecordRefusal::NoCell));
}

#[test]
fn unmark_fastest_removes_its_pick_set_and_mark() {
    let mut material = loaded(&record(5, &laser_with(None), &base_tests()));
    let t1 = stamp_test("T0001");
    material
        .judge(
            t1,
            CellIndex { column: 3, row: 3 },
            PickKind::Fastest,
            STAMP,
        )
        .expect("judged");
    material
        .unmark(t1, PickKind::Fastest, STAMP)
        .expect("unmarked");
    assert!(
        material
            .pick_set(PROFILE, SetKind::Cut, PickKind::Fastest)
            .is_none()
    );
    let written: Value = serde_json::from_str(&write_record(&material)).expect("json");
    assert!(written["tests"][0].get("fastest").is_none());
    assert!(written["lasers"][0]["picks"].get("cut").is_none());
}

#[test]
fn unmark_best_keeps_its_numbers_and_turns_typed() {
    let mut material = loaded(&record(5, &laser_with(None), &base_tests()));
    material
        .unmark(stamp_test("T0001"), PickKind::Best, STAMP)
        .expect("unmarked");
    let best = material
        .set_for(PROFILE, SetKind::Cut)
        .expect("the laser's set stays");
    assert!(matches!(
        best.source,
        curvyo_laser_core::library::SetSource::Typed
    ));
    let v = written(&material);
    assert_eq!(v["lasers"][0]["sets"]["cut"]["power_pct"], 60.0);
    assert_eq!(v["lasers"][0]["sets"]["cut"]["speed_mm_min"], 1950);
    assert_eq!(v["lasers"][0]["sets"]["cut"]["source"], "typed");
}

#[test]
fn unmark_leaves_a_pick_set_taken_from_another_test() {
    // T0001 carries the Fastest mark; its Fastest set comes from the other Cut test T0002.
    let tests = format!(
        "{},{}",
        cut_test("T0001", r#","fastest":{"column":3,"row":3}"#),
        cut_test("T0002", ""),
    );
    let picks = format!(
        r#"{{"cut":{{"fastest":{}}}}}"#,
        set(80.0, 4650, r#"{"test":"T0002"}"#)
    );
    let mut material = loaded(&record(5, &laser_with(Some(&picks)), &tests));
    material
        .unmark(stamp_test("T0001"), PickKind::Fastest, STAMP)
        .expect("unmarked");
    assert!(
        material
            .pick_set(PROFILE, SetKind::Cut, PickKind::Fastest)
            .is_some(),
        "the set names T0002, so it stays"
    );
}

#[test]
fn deleting_the_test_keeps_its_pick_set_reading_test_id() {
    let mut material = loaded(&record(5, &laser_with(None), &base_tests()));
    let t1 = stamp_test("T0001");
    material
        .judge(
            t1,
            CellIndex { column: 3, row: 3 },
            PickKind::Fastest,
            STAMP,
        )
        .expect("judged");
    material.delete_test(t1, STAMP.now).expect("deleted");
    assert!(
        material
            .pick_set(PROFILE, SetKind::Cut, PickKind::Fastest)
            .is_some(),
        "the Fastest set survives the deletion"
    );
    assert_eq!(
        written(&material)["lasers"][0]["picks"]["cut"]["fastest"]["source"]["test"],
        "T0001"
    );
    let again = write_record(&material);
    loaded(&again);
}
