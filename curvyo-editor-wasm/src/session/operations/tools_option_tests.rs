#![allow(clippy::assert_is_empty, clippy::many_single_char_names)]

//! Session-level tests of `specs/0111-laser-operation-tools` criteria 19 to 24: the edits of an
//! unfolded row's body.

use super::settings_view::SettingsStateView;
use super::tools_tests::{empty_op, fixture, op_named, row, undo_label};

// ---- criteria 19 to 24: the body's edits --------------------------------------------------

#[test]
fn criterion_20_and_24_an_option_is_one_step_and_out_of_range_is_refused() {
    let mut f = fixture(false);
    let engrave = empty_op(&mut f.s, "engrave", "Engrave 1");
    let view = row(&f.s, "Engrave 1").settings.unwrap();
    let options = view.options.unwrap();
    let hatch = options.hatch.unwrap();
    assert!(!hatch.crosshatch && hatch.bidirectional);
    assert_eq!((hatch.scan_angle_deg, hatch.overscan_pct), (0.0, 2));
    assert!(!view.z_offset_shown);
    let steps = f.s.undo_len();
    assert_eq!(
        f.s.edit_operation_option(engrave, "scan_angle", 90.1),
        Err("Scan angle is -90.0 to 90.0°.".to_owned())
    );
    assert_eq!(
        f.s.edit_operation_option(engrave, "overscan", 51.0),
        Err("Overscanning is 0 to 50 %.".to_owned())
    );
    assert_eq!(f.s.undo_len(), steps, "nothing written");
    assert_eq!(
        f.s.edit_operation_option(engrave, "scan_angle", 15.0),
        Ok(true)
    );
    assert_eq!(undo_label(&mut f.s), "Edit settings");
    assert_eq!(
        f.s.edit_operation_option(engrave, "crosshatch", 1.0),
        Ok(true)
    );
    assert_eq!(f.s.undo_len(), steps + 2);
    let hatch = row(&f.s, "Engrave 1")
        .settings
        .unwrap()
        .options
        .unwrap()
        .hatch
        .unwrap();
    assert!(hatch.crosshatch);
    assert_eq!(hatch.scan_angle_deg, 15.0);
    assert_eq!(
        f.s.edit_operation_option(engrave, "scan_angle", 15.0),
        Ok(false)
    );
}

#[test]
fn criterion_20_a_cut_takes_no_hatch_option() {
    let mut f = fixture(false);
    let cut = empty_op(&mut f.s, "cut", "Cut 1");
    let options = row(&f.s, "Cut 1").settings.unwrap().options.unwrap();
    assert!(options.hatch.is_none());
    assert_eq!(f.s.edit_operation_option(cut, "crosshatch", 1.0), Ok(false));
}

#[test]
fn criterion_21_and_22_z_offset_needs_the_motorised_z_axis_and_is_kept_without_it() {
    let mut f = fixture(false);
    let cut = empty_op(&mut f.s, "cut", "Cut 1");
    assert_eq!(f.s.edit_operation_option(cut, "z_offset", 1.5), Ok(false));
    assert!(f.s.set_machine_motorised_z(true));
    assert_eq!(undo_label(&mut f.s), "Motorised Z axis on");
    assert!(!f.s.set_machine_motorised_z(true), "no change, no step");
    assert!(row(&f.s, "Cut 1").settings.unwrap().z_offset_shown);
    assert_eq!(
        f.s.edit_operation_option(cut, "z_offset", 20.1),
        Err("Z offset is -20.0 to 20.0 mm.".to_owned())
    );
    assert_eq!(f.s.edit_operation_option(cut, "z_offset", 1.5), Ok(true));
    assert!(f.s.set_machine_motorised_z(false));
    let settings = row(&f.s, "Cut 1").settings.unwrap();
    assert!(!settings.z_offset_shown);
    assert_eq!(settings.options.unwrap().z_offset_mm, 1.5, "kept");
    assert!(f.s.set_machine_motorised_z(true));
    assert_eq!(
        row(&f.s, "Cut 1")
            .settings
            .unwrap()
            .options
            .unwrap()
            .z_offset_mm,
        1.5
    );
}

#[test]
fn criterion_19_an_interval_typed_with_no_settings_is_a_draft_written_with_the_numbers() {
    let mut f = fixture(false);
    let engrave = empty_op(&mut f.s, "engrave", "Engrave 1");
    let steps = f.s.undo_len();
    assert_eq!(
        f.s.edit_operation_option(engrave, "interval", 0.004),
        Err("Interval is 0.01 to 2.00 mm.".to_owned())
    );
    assert_eq!(
        f.s.edit_operation_option(engrave, "interval", 0.25),
        Ok(false),
        "kept in the draft, nothing written"
    );
    assert_eq!(f.s.undo_len(), steps);
    assert!(op_named(&f.s, "Engrave 1").settings.is_none());
    let view = row(&f.s, "Engrave 1").settings.unwrap();
    assert_eq!(view.state, SettingsStateView::NoSettings);
    assert_eq!(view.interval_mm, Some(0.25), "the draft shows");
    f.s.edit_operation_setting(engrave, "main", "power", 40.0)
        .unwrap();
    assert_eq!(f.s.undo_len(), steps);
    assert!(
        f.s.edit_operation_setting(engrave, "main", "speed", 3000.0)
            .unwrap()
    );
    assert_eq!(f.s.undo_len(), steps + 1, "one step with the numbers");
    assert_eq!(undo_label(&mut f.s), "Edit settings");
    let stored = op_named(&f.s, "Engrave 1").settings.unwrap();
    assert_eq!(stored["main"]["interval_mm"], 0.25, "{stored:?}");
    assert_eq!(
        row(&f.s, "Engrave 1").settings.unwrap().interval_mm,
        Some(0.25)
    );
}

#[test]
fn criterion_19_a_material_pick_with_no_settings_writes_the_draft_interval() {
    const BIRCH: &str = "22222222-2222-4222-8222-222222222222";
    let set = serde_json::json!({"power_pct": 35.0, "speed_mm_min": 6000, "passes": 1});
    let record = serde_json::json!({
        "format_version": 1, "id": BIRCH, "kind": "laser", "modified": 1_762_948_800_000_u64,
        "name": "Birch", "thickness_mm": 3.0,
        "machine": {"id": "generic-laser", "name": "Generic laser"},
        "sets": {"engrave": {"power_pct": 35.0, "speed_mm_min": 6000, "passes": 1, "source": "edited"}},
        "history": [{"id": "aaaaaaaa-0000-4000-8000-000000000000", "date": 1_762_948_800_000_u64,
            "event": "edited", "kind": "engrave", "set": set}]
    });
    let mut f = fixture(false);
    f.s.load_materials(&[(format!("{BIRCH}.json"), record.to_string())], None);
    let engrave = empty_op(&mut f.s, "engrave", "Engrave 1");
    assert_eq!(
        f.s.edit_operation_option(engrave, "interval", 0.4),
        Ok(false)
    );
    assert!(f.s.use_material_on(&[engrave], BIRCH));
    let stored = op_named(&f.s, "Engrave 1").settings.unwrap();
    assert_eq!(stored["main"]["interval_mm"], 0.4, "{stored:?}");
    f.s.undo();
    assert_eq!(
        row(&f.s, "Engrave 1").settings.unwrap().interval_mm,
        Some(0.1),
        "the draft went with the pick"
    );
}

#[test]
fn criterion_19_a_cut_with_no_settings_shows_no_interval_and_takes_none() {
    let mut f = fixture(false);
    let cut = empty_op(&mut f.s, "cut", "Cut 1");
    assert_eq!(row(&f.s, "Cut 1").settings.unwrap().interval_mm, None);
    assert_eq!(f.s.edit_operation_option(cut, "interval", 0.25), Ok(false));
    assert_eq!(row(&f.s, "Cut 1").settings.unwrap().interval_mm, None);
}

#[test]
fn criterion_19_the_interval_is_typed_into_main_and_checked() {
    let mut f = fixture(false);
    let engrave = empty_op(&mut f.s, "engrave", "Engrave 1");
    assert_eq!(
        row(&f.s, "Engrave 1").settings.unwrap().interval_mm,
        Some(0.1),
        "No settings shows the default interval"
    );
    f.s.edit_operation_setting(engrave, "main", "power", 40.0)
        .unwrap();
    f.s.edit_operation_setting(engrave, "main", "speed", 3000.0)
        .unwrap();
    assert_eq!(
        row(&f.s, "Engrave 1").settings.unwrap().interval_mm,
        Some(0.1)
    );
    assert_eq!(
        f.s.edit_operation_option(engrave, "interval", 0.004),
        Err("Interval is 0.01 to 2.00 mm.".to_owned())
    );
    assert_eq!(
        f.s.edit_operation_option(engrave, "interval", 0.25),
        Ok(true)
    );
    assert_eq!(undo_label(&mut f.s), "Edit settings");
    assert_eq!(
        row(&f.s, "Engrave 1").settings.unwrap().interval_mm,
        Some(0.25)
    );
}
