//! Session tests of the Machines tab's undo steps (ADR 0098 decision 4, UX unify 4.2 and 4.4):
//! every field edit, add, duplicate, rename and delete is one step with the library journal; the
//! connection is a plain write.

use curvyo_document_core::Direction;

use super::super::Session;

fn page(kind: &str) -> Session {
    let mut s = Session::new(98_201);
    s.add_page_of_kind(kind, None).unwrap();
    s
}

/// The record writes the host was handed since the last call, as `(kind, id)`.
fn writes(s: &mut Session) -> Vec<(String, String)> {
    let json: serde_json::Value = serde_json::from_str(&s.take_machine_writes_json()).unwrap();
    json.as_array()
        .unwrap()
        .iter()
        .map(|w| {
            (
                w["kind"].as_str().unwrap().to_owned(),
                w["id"].as_str().unwrap_or_default().to_owned(),
            )
        })
        .collect()
}

/// One machine of each family, the field and value of an edit, and the History name.
const EDITS: [(&str, &str, &str, &str, &str); 4] = [
    (
        "curvyo.laser",
        "generic-laser",
        "max_feed",
        "5000",
        "Edit max feed",
    ),
    (
        "curvyo.cutter",
        "generic-hpgl-cutter",
        "default_pressure",
        "55",
        "Edit default pressure",
    ),
    ("curvyo.embroidery", "w6-eu7", "speed", "900", "Edit speed"),
    (
        "curvyo.cnc",
        "generic-grbl-router",
        "z_travel_mm",
        "80",
        "Edit Z travel",
    ),
];

/// The shown value of the field `key` of machine `id`.
fn field(s: &Session, id: &str, key: &str) -> String {
    let view = s.machines_view().unwrap();
    let row = view
        .sections
        .iter()
        .flat_map(|section| &section.rows)
        .find(|row| row.id == id)
        .unwrap();
    row.fields
        .iter()
        .find(|f| f.key == key)
        .unwrap()
        .value
        .clone()
}

#[test]
fn a_field_edit_is_one_step_that_undo_and_redo_take_back_and_forth() {
    for (kind, builtin, key, value, name) in EDITS {
        let mut s = page(kind);
        let id = s.duplicate_machine(builtin, [7; 16], 1).unwrap();
        let before = field(&s, &id, key);
        let _ = writes(&mut s);
        let steps = s.undo_len();
        assert!(s.set_machine_value(&id, key, value, 2).is_ok(), "{kind}");
        assert_eq!(s.undo_len(), steps + 1, "{kind}: one step");
        assert_eq!(
            s.history_target_name(Direction::Undo).as_deref(),
            Some(name),
            "{kind}"
        );
        assert_eq!(field(&s, &id, key), value, "{kind}");
        assert_eq!(writes(&mut s), [("write".to_owned(), id.clone())], "{kind}");
        s.undo();
        assert_eq!(
            field(&s, &id, key),
            before,
            "{kind}: the undo takes it back"
        );
        assert_eq!(writes(&mut s), [("write".to_owned(), id.clone())], "{kind}");
        s.redo();
        assert_eq!(
            field(&s, &id, key),
            value,
            "{kind}: the redo writes it again"
        );
    }
}

#[test]
fn add_and_duplicate_are_steps_whose_undo_removes_the_record() {
    for (kind, builtin, ..) in EDITS {
        let mut s = page(kind);
        let family = s.family_of_record(builtin).unwrap();
        let steps = s.undo_len();
        let added = s
            .add_machine(super::family_key(family), [3; 16], 1)
            .unwrap();
        assert_eq!(s.undo_len(), steps + 1, "{kind}");
        assert_eq!(
            s.history_target_name(Direction::Undo).as_deref(),
            Some("Add machine")
        );
        let copy = s.duplicate_machine(&added, [4; 16], 2).unwrap();
        assert_eq!(
            s.history_target_name(Direction::Undo).as_deref(),
            Some("Duplicate machine")
        );
        let _ = writes(&mut s);
        s.undo();
        assert!(s.family_of_record(&copy).is_none(), "{kind}");
        assert_eq!(writes(&mut s), [("remove".to_owned(), copy.clone())]);
        s.undo();
        assert!(s.family_of_record(&added).is_none(), "{kind}");
        s.redo();
        assert_eq!(s.family_of_record(&added), Some(family), "{kind}");
    }
}

#[test]
fn a_delete_is_a_step_whose_undo_brings_the_record_and_the_pages_machine_back() {
    for (kind, builtin, ..) in EDITS {
        let mut s = page(kind);
        let id = s.duplicate_machine(builtin, [5; 16], 1).unwrap();
        assert!(s.pick_output_profile(&id), "{kind}");
        let state = |s: &Session| s.machine_picker_view().unwrap()["state"].clone();
        let name = s.record_name(&id);
        assert!(s.delete_machine(&id));
        assert_eq!(
            s.history_target_name(Direction::Undo).as_deref(),
            Some("Delete machine")
        );
        assert_eq!(state(&s), "missing", "{kind}");
        s.undo();
        assert_eq!(state(&s), serde_json::Value::Null, "{kind}");
        assert_eq!(s.record_name(&id), name, "{kind}");
    }
}

#[test]
fn a_rename_is_a_step() {
    for (kind, builtin, ..) in EDITS {
        let mut s = page(kind);
        let id = s.duplicate_machine(builtin, [6; 16], 1).unwrap();
        let name = s.record_name(&id);
        let steps = s.undo_len();
        assert!(s.rename_machine(&id, "Shop machine", 2));
        assert_eq!(s.undo_len(), steps + 1, "{kind}");
        assert_eq!(
            s.history_target_name(Direction::Undo).as_deref(),
            Some("Rename machine")
        );
        s.undo();
        assert_eq!(s.record_name(&id), name, "{kind}");
    }
}

#[test]
fn the_connection_is_a_plain_write_without_a_step() {
    let mut s = page("curvyo.cnc");
    let id = s
        .duplicate_machine("generic-grbl-router", [8; 16], 1)
        .unwrap();
    let steps = s.undo_len();
    assert_eq!(
        s.set_machine_value(&id, "connector", "lab:4242", 2),
        Ok(None)
    );
    assert_eq!(s.undo_len(), steps, "no step");
    let mut s = page("curvyo.laser");
    let id = s.duplicate_machine("generic-laser", [8; 16], 1).unwrap();
    let steps = s.undo_len();
    assert_eq!(
        s.set_machine_connection_of(&id, "lab:4242", "/dev/ttyUSB0", 115_200.0, 2),
        None
    );
    assert_eq!(s.undo_len(), steps, "no step");
    let view = s.machines_view().unwrap();
    let row = view.sections[0].rows.iter().find(|r| r.id == id).unwrap();
    assert_eq!(
        row.connection.as_ref().unwrap().connector_address,
        "lab:4242"
    );
}

/// Frage 434 C with ADR R4: Update pages is its own step after the edit's; the first undo returns
/// the pages, the second the record; an undo of the edit ends the offer.
#[test]
fn update_pages_then_two_undos_return_the_pages_then_the_record() {
    let mut s = page("curvyo.cnc");
    let id = s
        .duplicate_machine("generic-grbl-router", [9; 16], 1)
        .unwrap();
    assert!(s.pick_output_profile(&id));
    let page = s.active_cnc().unwrap().id;
    let before = field(&s, &id, "z_travel_mm");
    assert!(s.set_machine_value(&id, "z_travel_mm", "80", 2).is_ok());
    assert!(s.follow_machine());
    assert!(!s.page_differs(super::Family::Cnc, page, &id));
    s.undo();
    assert!(s.page_differs(super::Family::Cnc, page, &id), "pages back");
    s.undo();
    assert_eq!(field(&s, &id, "z_travel_mm"), before, "record back");
    assert!(!s.page_differs(super::Family::Cnc, page, &id));
    assert!(s.set_machine_value(&id, "z_travel_mm", "70", 3).is_ok());
    assert!(s.machines_view().unwrap().follow.is_some());
    s.undo();
    s.redo();
    assert!(
        s.machines_view().unwrap().follow.is_none(),
        "an undo of the edit ends the offer"
    );
}

/// The materials rule: a record changed since the step (a sync) is left as it is, with a notice.
#[test]
fn an_undo_after_the_record_changed_elsewhere_leaves_it_with_a_notice() {
    let mut s = page("curvyo.cnc");
    let id = s
        .duplicate_machine("generic-grbl-router", [10; 16], 1)
        .unwrap();
    assert!(s.set_machine_value(&id, "z_travel_mm", "80", 2).is_ok());
    let mut synced = s.manufacturing.cnc.find(&id).unwrap().clone();
    synced.values.spindle_max = curvyo_cnc_core::Rpm::new(15_000).unwrap();
    s.manufacturing.cnc.put(synced);
    let kept = s.machine_record_text(&id);
    let result = s.undo();
    assert!(result.notice.contains("changed since"), "{}", result.notice);
    assert_eq!(s.machine_record_text(&id), kept);
}
