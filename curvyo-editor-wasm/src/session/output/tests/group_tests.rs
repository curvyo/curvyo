//! The output group of the Operations tab: the machine line, the checks, Start, the total and the rows' lines 2 and 3 (criteria 1 to 7, 10, 25, 38).

#![allow(
    clippy::assert_is_empty,
    clippy::case_sensitive_file_extension_comparisons
)]

use super::{cut, laser_with_square, rows, undo_label, view};
use crate::session::Session;
use crate::session::align_test_support::draw_rect;

#[test]
fn criterion_1_only_a_laser_page_has_an_output_group() {
    // Superseded by 0029 rework (2026-11-17): the output group of the Operations tab replaces the
    // Output tab's view.
    let mut s = Session::new(290_002);
    assert_eq!(s.output_group_json(), "null");
    s.add_page_of_kind("curvyo.laser", None).unwrap();
    assert!(s.output_group_view().is_some());
}

#[test]
fn criterion_2_the_machine_line_names_the_profile_and_the_format() {
    // Superseded by 0029 rework (2026-11-17): one machine line "<profile> · <format>" (P30); the
    // bed, origin and mode line stay in the Machine block.
    let (s, _) = laser_with_square();
    let v = view(&s);
    assert_eq!(v["machine"], "Generic laser · GRBL");
    assert!(v.get("mode").is_none());
}

#[test]
fn criterion_7_nothing_runs_blocks_export_and_has_no_total() {
    let (s, _) = laser_with_square();
    let v = view(&s);
    let texts: Vec<&str> = v["checks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["text"].as_str().unwrap())
        .collect();
    assert!(
        texts.contains(&"Nothing runs. Assign items to an operation."),
        "{texts:?}"
    );
    assert_eq!(v["export"]["enabled"], false);
    assert_eq!(
        v["export"]["tooltip"],
        "Nothing runs. Assign items to an operation."
    );
    assert!(v["total"].is_null());
    assert!(v.get("runs").is_none(), "runs are the rows' line 2");
}

#[test]
fn criterion_4_a_a_new_operation_has_numbers_and_never_blocks() {
    // Superseded by `specs/0150` criterion 23: "has no power and speed" is gone.
    let (mut s, square) = laser_with_square();
    s.selection.set(&[square]);
    s.press_operation_kind("cut", true).unwrap();
    let v = view(&s);
    assert!(
        v["checks"]
            .as_array()
            .unwrap()
            .iter()
            .all(|c| c["severity"] != "block"),
        "{v}"
    );
    assert_eq!(v["export"]["enabled"], true);
    let row = &rows(&s)["rows"][0];
    assert_ne!(row["run"]["time"], "–", "a running row always has a time");
}

#[test]
fn criteria_2_3_and_10_a_row_follows_a_typed_number_in_the_same_frame() {
    // Superseded by 0029 rework (2026-11-17): the run is the row's line 2 (ring digit, time) and
    // line 3 (numbers) in the Operations tab; the total gains the "Runs ..." line.
    let (mut s, square) = laser_with_square();
    let id = cut(&mut s, &[square]);
    let v = view(&s);
    let row = &rows(&s)["rows"][0];
    assert_eq!(row["name"], "Cut 1");
    assert_eq!(row["run"]["digit"], 1);
    let time = row["run"]["time"].as_str().unwrap().to_owned();
    assert!(time.ends_with(" s"), "{time}");
    assert_eq!(row["accessible_name"], format!("Cut 1, run 1, {time}"));
    assert_eq!(row["numbers"][0]["text"], "60 %, 600 mm/min, 1 ×");
    assert_eq!(v["total"]["runs"]["text"], "Runs 1 operation, 1 item.");
    assert!(v["total"]["time"].as_str().unwrap().starts_with("About "));
    assert!(
        v["total"]["lengths"]
            .as_str()
            .unwrap()
            .starts_with("Burn 80 mm · travel ")
    );
    s.edit_operation_setting(id, "main", "power", 35.0).unwrap();
    assert_eq!(
        rows(&s)["rows"][0]["numbers"][0]["text"],
        "35 %, 600 mm/min, 1 ×"
    );
}

#[test]
fn criterion_3_a_row_that_does_not_run_shows_its_numbers_and_no_ring() {
    let (mut s, square) = laser_with_square();
    let id = cut(&mut s, &[square]);
    s.recording_unguarded(|this| {
        this.document
            .set_operation_switch(id, curvyo_document_core::OperationSwitch::Output, false)
            .unwrap()
    });
    let row = &rows(&s)["rows"][0];
    assert!(row["run"].is_null(), "no ring, no time");
    assert_eq!(row["numbers"][0]["text"], "60 %, 600 mm/min, 1 ×");
    assert_eq!(row["accessible_name"], "Cut 1");
    assert!(
        rows(&s).get("summary").is_none(),
        "the counts are checks (P28)"
    );
}

#[test]
fn criterion_38_start_is_one_pick_with_one_step_per_value() {
    // Superseded by 0029 rework (2026-11-17): one Start pick of three values, no greying.
    let (mut s, _) = laser_with_square();
    let v = view(&s);
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
    // Every value of the core's closed list, so a fourth value cannot miss its step here; User
    // origin needs a stored point first (`specs/0118` O2).
    assert!(s.set_user_origin(5.0, 6.0));
    assert!(s.set_start("machine_origin"));
    for start in curvyo_laser_core::job::Start::ALL.into_iter().rev() {
        let value = start.as_str();
        assert!(s.set_start(value), "{value}");
        assert_eq!(
            undo_label(&mut s),
            format!("Start: {}", curvyo_laser_core::job::start_text(start))
        );
        assert_eq!(view(&s)["start"]["value"], value);
        assert!(!s.set_start(value), "no change, no step");
    }
    assert!(!s.set_start("middle"));
}

#[test]
fn criterion_4_e_a_damaged_start_blocks_and_any_pick_clears_it() {
    // The format 48 golden whose second laser page stores `start` true (ADR 0065 §6).
    let bytes = std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../curvyo-document-core/tests/fixtures/laser_output_damaged_v48.curvyo"),
    )
    .unwrap();
    let mut s = Session::open(4, &bytes).unwrap();
    let page = s
        .document
        .pages()
        .into_iter()
        .filter(|p| p.kind.manufacturing().is_some())
        .nth(1)
        .unwrap();
    assert!(s.activate_page(&page.id));
    let v = view(&s);
    assert!(v["start"]["value"].is_null(), "no option checked");
    assert_eq!(v["start"]["damaged"], true);
    let check = v["checks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["button"] == "Start")
        .unwrap()
        .clone();
    assert_eq!(check["severity"], "block");
    assert_eq!(check["text"], "Start is damaged. Pick one above.");
    assert_eq!(check["target"]["kind"], "start_pick");
    assert_eq!(v["export"]["enabled"], false);
    assert!(
        s.set_start("machine_origin"),
        "the default pick rewrites it"
    );
    assert_eq!(undo_label(&mut s), "Start: Machine origin");
    let v = view(&s);
    assert_eq!(v["start"]["damaged"], false);
    assert!(
        v["checks"]
            .as_array()
            .unwrap()
            .iter()
            .all(|c| c["button"] != "Start")
    );
}

#[test]
fn criterion_25_air_assist_is_one_step_and_ends_the_machine_line() {
    // Superseded by 0029 rework (2026-11-17): " · air assist" ends the one machine line (P30).
    let (mut s, _) = laser_with_square();
    assert!(s.set_machine_air_assist(true));
    assert_eq!(undo_label(&mut s), "Air assist on");
    assert!(!s.set_machine_air_assist(true), "no change, no step");
    assert_eq!(view(&s)["machine"], "Generic laser · GRBL · air assist");
}

#[test]
fn criterion_1_there_is_no_output_tab() {
    // Superseded by 0029 rework (2026-11-17): the output group lives in the Operations tab.
    let mut s = Session::new(290_050);
    s.add_page_of_kind("curvyo.laser", None).unwrap();
    let names: Vec<&'static str> = s.panel_view().tabs.iter().map(|tab| tab.name).collect();
    assert!(!names.contains(&"output"), "{names:?}");
    assert!(!s.press_panel_tab("output"));
}

#[test]
fn part_d_a_check_button_selects_its_items_or_reveals_its_operation() {
    let (mut s, square) = laser_with_square();
    let op = cut(&mut s, &[square]);
    s.selection.set(&[]);
    assert!(s.select_output_items(&[square]));
    assert_eq!(s.selected_object_count(), 1);
    assert!(!s.select_output_items(&[]));
    // Revealing keeps the row unfolded however often it is pressed.
    assert!(s.reveal_operation_row(op));
    assert!(s.reveal_operation_row(op));
    let ops: serde_json::Value = serde_json::from_str(&s.operations_view_json()).unwrap();
    let unfolded: Vec<bool> = ops["rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["unfolded"].as_bool().unwrap())
        .collect();
    assert_eq!(unfolded, [true]);
}

#[test]
fn criterion_10_the_runs_line_selects_the_running_items_and_output_off_selects_its_items() {
    // `0101` criterion 17 as amended: the counts and the select buttons are unchanged.
    let (mut s, square) = laser_with_square();
    let origin = s.active_manufacturing().unwrap().page.unwrap().origin;
    let off_item = draw_rect(
        &mut s,
        (origin.x + 50.0, origin.y + 10.0),
        (origin.x + 70.0, origin.y + 30.0),
    );
    cut(&mut s, &[square]);
    s.selection.set(&[off_item]);
    let score = s.press_operation_kind("score", true).unwrap();
    assert!(s.set_operation_switch(score, curvyo_document_core::OperationSwitch::Output, false));
    let v = view(&s);
    let runs = &v["total"]["runs"];
    assert_eq!(runs["text"], "Runs 1 operation, 1 item.");
    assert_eq!(runs["button"], "1 item");
    let off = v["checks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["after"] == " has Output off.")
        .unwrap()
        .clone();
    assert_eq!(off["target"]["kind"], "items");
    assert_eq!(off["target"]["ids"], serde_json::json!([off_item]));
    s.selection.set(&[]);
    assert!(s.select_operation_items("running"));
    assert_eq!(s.selection.ids(), &[square]);
    assert!(s.select_operation_items("output_off"));
    assert_eq!(s.selection.ids(), &[off_item]);
}
