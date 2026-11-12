//! Session tests of the material library (`specs/0028` criteria 15 to 26, 32): Save as material,
//! the record edits as steps with their refresh, the journal's undo and redo, the save refresh.

use curvyo_document_core::{Direction, NodeId};
use curvyo_laser_core::settings::{SettingsRead, read_settings};

use super::super::Session;
use super::super::test_grid::tests::{add_grid, laser_with_fonts};
use super::SaveAsMaterial;

const RANDOM: [u8; 32] = [7; 32];

fn random(n: u8) -> [u8; 32] {
    let mut bytes = RANDOM;
    bytes[0] = n;
    bytes[16] = n.wrapping_add(100);
    bytes
}

fn undo_label(s: &mut Session) -> String {
    s.history_target_name(Direction::Undo).unwrap_or_default()
}

/// A laser page with a marked default grid saved as "Birch 3 mm".
fn saved_birch() -> (Session, String) {
    let mut s = laser_with_fonts();
    add_grid(&mut s);
    assert!(s.press_test_cell(2, 1));
    let id = s
        .save_as_material(
            &SaveAsMaterial {
                existing: None,
                name: "Birch 3 mm".to_owned(),
                thickness_mm: 3.0,
                passes: 1.0,
            },
            &random(1),
        )
        .unwrap();
    (s, id)
}

fn cut_with_birch() -> (Session, String, NodeId) {
    let (mut s, id) = saved_birch();
    let cut = s.add_test_operation("cut").unwrap();
    assert!(s.use_material_on(&[cut], &id));
    (s, id, cut)
}

fn row_settings(s: &Session, id: NodeId) -> super::super::operations::SettingsView {
    s.operations_view()
        .unwrap()
        .rows
        .into_iter()
        .find(|row| row.id == id)
        .and_then(|row| row.settings)
        .unwrap()
}

#[test]
fn criterion_15_save_as_material_makes_a_record_in_one_step_with_its_write() {
    let (mut s, id) = saved_birch();
    assert_eq!(undo_label(&mut s), "Save as material");
    let material = s.materials().find(&id).unwrap();
    assert_eq!(material.name, "Birch 3 mm");
    assert_eq!(material.history.len(), 1);
    assert_eq!(material.machine.id, "generic-laser");
    let writes = s.take_material_writes_json();
    assert!(writes.contains("\"op\":\"write\""), "{writes}");
    assert!(writes.contains(&id));
    let view = s.materials_view().unwrap();
    assert_eq!(view.rows.len(), 1);
    assert_eq!(view.next_name, "Material 1");
}

#[test]
fn criterion_15_the_form_names_its_first_invalid_field() {
    let mut s = laser_with_fonts();
    add_grid(&mut s);
    let mut form = SaveAsMaterial {
        existing: None,
        name: " ".to_owned(),
        thickness_mm: 3.0,
        passes: 1.0,
    };
    assert_eq!(
        s.save_as_material(&form, &RANDOM),
        Err("No cell marked".to_owned())
    );
    assert!(s.press_test_cell(0, 0));
    assert_eq!(
        s.save_as_material(&form, &RANDOM),
        Err("Enter a name.".to_owned())
    );
    form.name = "Oak".to_owned();
    form.thickness_mm = f64::NAN;
    assert_eq!(
        s.save_as_material(&form, &RANDOM),
        Err("Enter a thickness.".to_owned())
    );
    form.thickness_mm = 100.1;
    assert_eq!(
        s.save_as_material(&form, &RANDOM),
        Err("Thickness is 0.1 to 100 mm.".to_owned())
    );
    form.thickness_mm = 4.0;
    form.passes = 21.0;
    assert_eq!(
        s.save_as_material(&form, &RANDOM),
        Err("Passes are 1 to 20.".to_owned())
    );
}

#[test]
fn criterion_32_undo_of_save_as_material_removes_the_record_and_redo_brings_it_back() {
    let (mut s, id) = saved_birch();
    let _ = s.take_material_writes_json();
    let result = s.undo();
    assert!(result.changed);
    assert!(s.materials().find(&id).is_none());
    assert!(s.take_material_writes_json().contains("\"op\":\"remove\""));
    s.redo();
    let material = s.materials().find(&id).unwrap();
    assert_eq!(
        material.history.len(),
        1,
        "redo adds the entry again (Frage 190 A)"
    );
}

#[test]
fn criteria_22_26_a_set_edit_changes_every_linked_operation_in_its_step() {
    let (mut s, id, cut) = cut_with_birch();
    assert_eq!(row_settings(&s, cut).sets[0].run.power_pct, Some(60.0));
    assert_eq!(
        s.edit_material_set(&id, "cut", "power", 65.0, &random(2)),
        Ok(true)
    );
    assert_eq!(undo_label(&mut s), "Edit material");
    assert_eq!(row_settings(&s, cut).sets[0].run.power_pct, Some(65.0));
    // The stored numbers were refreshed in the same step.
    let stored = stored_power(&s, cut);
    assert_eq!(stored, 65.0);
    s.undo();
    assert_eq!(row_settings(&s, cut).sets[0].run.power_pct, Some(60.0));
    assert_eq!(stored_power(&s, cut), 60.0);
    assert_eq!(s.materials().find(&id).unwrap().history.len(), 1);
    s.redo();
    assert_eq!(row_settings(&s, cut).sets[0].run.power_pct, Some(65.0));
}

fn stored_power(s: &Session, op: NodeId) -> f64 {
    let manufacturing = s.active_manufacturing().unwrap();
    let snapshot = s
        .document
        .operations(manufacturing.id)
        .unwrap()
        .into_iter()
        .find(|o| o.id == op)
        .unwrap();
    let kind = curvyo_laser_core::operations::OperationKind::Cut;
    match read_settings(snapshot.settings.as_ref(), kind) {
        SettingsRead::Stored(stored) => stored.main.power.percent(),
        other => panic!("{other:?}"),
    }
}

#[test]
fn criterion_32_undo_leaves_a_record_that_changed_since_and_says_so() {
    let (mut s, id, _) = cut_with_birch();
    assert_eq!(
        s.edit_material_set(&id, "cut", "power", 65.0, &random(2)),
        Ok(true)
    );
    // Another computer's sync: the record on disk is loaded with other numbers.
    let text = s.materials().text(&id).unwrap().replace("65.0", "70.0");
    s.load_materials(&[(format!("{id}.json"), text)], None);
    let result = s.undo();
    assert_eq!(
        result.notice,
        "Birch 3 mm changed since; the material was not undone."
    );
    assert_eq!(
        s.materials()
            .find(&id)
            .unwrap()
            .set(curvyo_laser_core::settings::SetKind::Cut)
            .unwrap()
            .power
            .percent(),
        70.0
    );
}

#[test]
fn criterion_32_a_dropped_journal_entry_undoes_only_the_project() {
    let (mut s, id, _) = cut_with_birch();
    s.material_journal().set_limit(10);
    assert_eq!(s.rename_material(&id, "Birch plywood"), Ok(true));
    assert_eq!(s.material_journal().bytes(), 0);
    let result = s.undo();
    assert_eq!(
        result.notice,
        "Only the project was undone; Birch plywood is too old."
    );
    assert_eq!(s.materials().find(&id).unwrap().name, "Birch plywood");
}

#[test]
fn criteria_21_26_delete_leaves_stored_numbers_and_undo_brings_the_record_back() {
    let (mut s, id, cut) = cut_with_birch();
    assert!(s.delete_material(&id).is_ok());
    assert_eq!(undo_label(&mut s), "Delete material");
    let settings = row_settings(&s, cut);
    assert_eq!(
        settings.reason.as_deref(),
        Some("Material not found, using stored numbers")
    );
    assert_eq!(settings.sets[0].run.power_pct, Some(60.0));
    s.undo();
    assert!(s.materials().find(&id).is_some());
    assert_eq!(row_settings(&s, cut).reason, None);
}

#[test]
fn criteria_20_22_rename_thickness_and_order() {
    let (mut s, id) = saved_birch();
    assert_eq!(s.rename_material(&id, ""), Ok(false), "empty reverts");
    assert_eq!(s.rename_material(&id, "Oak"), Ok(true));
    assert_eq!(undo_label(&mut s), "Rename material");
    assert_eq!(s.set_material_thickness(&id, 4.0), Ok(true));
    assert_eq!(undo_label(&mut s), "Edit material");
    assert_eq!(
        s.set_material_thickness(&id, 0.0),
        Err("Thickness is 0.1 to 100 mm.".to_owned())
    );
    let steps = s.undo_len();
    let _ = s.take_material_writes_json();
    assert!(s.move_material(&id, 0), "the order list gets its first id");
    assert_eq!(s.undo_len(), steps, "the order is no step");
    assert!(s.take_material_writes_json().contains("\"op\":\"order\""));
    s.select_material(Some(&id));
    let fields = s.materials_view().unwrap().selected.unwrap();
    assert_eq!(fields.thickness_mm, 4.0);
    assert!(
        fields.sets[0].source.starts_with("Tested "),
        "{}",
        fields.sets[0].source
    );
}

#[test]
fn criterion_26_save_refreshes_stored_numbers_without_a_step() {
    let (mut s, id, cut) = cut_with_birch();
    // A newer record from another computer: live numbers 70, stored 60.
    let mut material = s.materials().find(&id).unwrap().clone();
    material.modified += 10;
    let set = material
        .sets
        .get_mut(&curvyo_laser_core::settings::SetKind::Cut)
        .unwrap();
    set.settings.power = curvyo_laser_core::settings::PowerPercent::from_tenths(700).unwrap();
    let text = curvyo_laser_core::material_library::write_material(&material);
    s.load_materials(&[(format!("{id}.json"), text)], None);
    assert_eq!(row_settings(&s, cut).sets[0].run.power_pct, Some(70.0));
    assert_eq!(stored_power(&s, cut), 60.0);
    let steps = s.undo_len();
    s.pack_for_save().unwrap();
    assert_eq!(stored_power(&s, cut), 70.0);
    assert_eq!(s.undo_len(), steps, "the save refresh is no step");
}

#[test]
fn criterion_24_a_typed_number_keeps_the_link_and_reset_removes_it() {
    let (mut s, _, cut) = cut_with_birch();
    assert_eq!(
        s.edit_operation_setting(cut, "main", "power", 70.0),
        Ok(true)
    );
    assert_eq!(undo_label(&mut s), "Edit settings");
    let settings = row_settings(&s, cut);
    assert_eq!(settings.differs.as_deref(), Some("Differs from Birch 3 mm"));
    assert_eq!(settings.sets[0].run.power_pct, Some(70.0));
    assert_eq!(settings.sets[0].material.unwrap().power_pct, 60.0);
    assert!(settings.reset);
    assert!(s.reset_operation_to_material(cut));
    assert_eq!(undo_label(&mut s), "Reset to material");
    assert!(!row_settings(&s, cut).reset);
    assert_eq!(
        s.edit_operation_setting(cut, "main", "power", 70.0),
        Ok(true)
    );
    assert!(s.use_own_numbers_on(&[cut]));
    let own = row_settings(&s, cut);
    assert_eq!(own.material, None);
    assert_eq!(own.sets[0].run.power_pct, Some(70.0));
}

#[test]
fn criterion_27_a_pick_for_several_skips_a_kind_without_a_set_and_says_so() {
    let (mut s, id) = saved_birch();
    let cut = s.add_test_operation("cut").unwrap();
    let score = s.add_test_operation("score").unwrap();
    assert!(s.use_material_on(&[cut, score], &id));
    assert_eq!(undo_label(&mut s), "Use material");
    assert_eq!(
        s.operations_notice().as_deref(),
        Some("Score 1 skipped: Birch 3 mm has no Score set.")
    );
    let disabled = row_settings(&s, score).disabled;
    assert_eq!(disabled[0].reason, "No Score set");
}

#[test]
fn criterion_24_no_settings_waits_for_power_and_speed() {
    let (mut s, _) = saved_birch();
    let cut = s.add_test_operation("cut").unwrap();
    let steps = s.undo_len();
    assert_eq!(
        s.edit_operation_setting(cut, "main", "power", 50.0),
        Ok(false)
    );
    assert_eq!(s.undo_len(), steps);
    assert_eq!(row_settings(&s, cut).sets[0].run.power_pct, Some(50.0));
    assert_eq!(
        s.edit_operation_setting(cut, "main", "speed", 1200.0),
        Ok(true)
    );
    let own = row_settings(&s, cut);
    assert_eq!(own.sets[0].run.passes, Some(1));
    assert_eq!(
        s.edit_operation_setting(cut, "main", "speed", 999_999.0),
        Err("Speed is 1 to 6000 mm/min.".to_owned())
    );
}
