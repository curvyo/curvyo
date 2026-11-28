//! Session tests of the Machines tab (ADR 0098 decisions 4 and 5, Frage 434 C).

use super::super::Session;

fn cnc() -> Session {
    let mut s = Session::new(98_101);
    s.add_page_of_kind("curvyo.cnc", None).unwrap();
    s
}

#[test]
fn every_enabled_family_has_a_section_the_pages_family_first() {
    let s = cnc();
    let view = s.machines_view().unwrap();
    let families: Vec<&str> = view.sections.iter().map(|s| s.family).collect();
    assert_eq!(families[0], "cnc");
    assert!(families.contains(&"laser") && families.contains(&"plotter"));
    assert!(view.sections[0].active);
    let builtin = &view.sections[0].rows[0];
    assert!(builtin.builtin);
    assert!(
        builtin
            .fields
            .iter()
            .all(|f| f.kind == super::FieldKind::Text),
        "a built-in shows plain lines"
    );
}

/// Frage 434 C: an edit of a machine a page uses as it was offers Update pages; Update is one
/// step that an undo returns; Keep leaves the page differing.
#[test]
fn an_edit_offers_to_update_the_pages_that_used_the_machine() {
    let mut s = cnc();
    let id = s
        .duplicate_machine("generic-grbl-router", [7; 16], 1)
        .unwrap();
    assert!(s.pick_cnc_profile(&id));
    assert!(s.machines_view().unwrap().follow.is_none());
    assert_eq!(s.set_machine_value(&id, "z_travel_mm", "80", 2), Ok(None));
    let follow = s.machines_view().unwrap().follow.unwrap();
    assert_eq!(follow.pages, 1);
    assert!(
        follow.sentence.starts_with("1 page uses "),
        "{}",
        follow.sentence
    );
    let page = s.active_cnc().unwrap().id;
    assert!(s.page_differs(super::Family::Cnc, page, &id));
    let steps = s.undo_len();
    assert!(s.follow_machine());
    assert_eq!(s.undo_len(), steps + 1, "one step");
    assert!(!s.page_differs(super::Family::Cnc, page, &id));
    assert!(s.machines_view().unwrap().follow.is_none());
    s.undo();
    assert!(
        s.page_differs(super::Family::Cnc, page, &id),
        "undo returns the page"
    );
    assert_eq!(s.set_machine_value(&id, "safe_height_mm", "4", 3), Ok(None));
    s.keep_machine_pages();
    assert!(s.machines_view().unwrap().follow.is_none());
    assert_eq!(
        s.set_machine_value(&id, "z_travel_mm", "0", 4),
        Err("Z travel is 1 to 1,000 mm.".to_owned())
    );
    assert_eq!(
        s.set_machine_value("generic-grbl-router", "z_travel_mm", "80", 5),
        Err(String::new()),
        "a built-in is not edited"
    );
}

#[test]
fn rename_and_delete_work_on_every_family() {
    let mut s = cnc();
    for builtin in [
        "generic-laser",
        "cricut-maker-3",
        "w6-eu7",
        "generic-grbl-router",
    ] {
        let id = s.duplicate_machine(builtin, [9; 16], 1).unwrap();
        assert!(s.rename_machine(&id, "Mine", 2), "{builtin}");
        assert_eq!(
            s.family_of_record(&id),
            s.family_of_record(builtin),
            "{builtin}"
        );
        assert!(s.delete_machine(&id), "{builtin}");
        assert!(s.family_of_record(&id).is_none(), "{builtin}");
        assert!(!s.delete_machine(builtin), "a built-in stays");
    }
}

/// ADR 0098 decision 4: the five built-in ids are distinct.
#[test]
fn the_builtin_ids_are_distinct() {
    let mut ids = vec![
        curvyo_laser_core::GENERIC_LASER_ID,
        curvyo_cutter_core::CRICUT_MAKER_3.id,
        curvyo_cutter_core::GENERIC_HPGL_CUTTER.id,
        curvyo_embroidery_core::W6_EU7.id,
        curvyo_cnc_core::GENERIC_GRBL_ROUTER.id,
    ];
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), 5);
}

/// ADR 0098 R2, R3: the Machine group shows "differs" after a kept edit, "missing" once the
/// machine is deleted, and Save as machine puts the copy back into the library.
#[test]
fn the_machine_group_names_the_copys_state_against_the_library() {
    let mut s = cnc();
    let state = |s: &Session| s.machine_picker_view().unwrap()["state"].clone();
    assert_eq!(state(&s), serde_json::Value::Null);
    let id = s
        .duplicate_machine("generic-grbl-router", [3; 16], 1)
        .unwrap();
    assert!(s.pick_output_profile(&id));
    assert_eq!(
        s.set_machine_value(&id, "spindle_max_rpm", "18500", 2),
        Ok(None)
    );
    s.keep_machine_pages();
    assert_eq!(state(&s), "differs");
    assert!(s.reset_output_profile());
    assert_eq!(state(&s), serde_json::Value::Null);
    assert!(s.delete_machine(&id));
    assert_eq!(state(&s), "missing");
    assert!(s.save_output_profile([4; 16], 3).is_some());
    assert_eq!(state(&s), serde_json::Value::Null);
}
