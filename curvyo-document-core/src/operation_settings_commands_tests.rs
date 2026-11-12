use serde_json::{Map, Value, json};

use super::*;
use crate::{DocumentSize, NewOperation, NewOutputPage, OperationError, pack, unpack};

fn laser(d: &Document) -> NodeId {
    let page = d
        .add_output_page(NewOutputPage {
            process: "curvyo.laser",
            label: "Laser",
            profile: None,
            work_area: DocumentSize::from_mm(400.0, 300.0),
        })
        .unwrap();
    page.kind.manufacturing().unwrap()
}

fn op(d: &Document, m: NodeId, kind: &str) -> NodeId {
    d.add_operation(m, &NewOperation { kind, name: kind }, 9, &[])
        .unwrap()
}

fn map(value: Value) -> Map<String, Value> {
    let Value::Object(map) = value else {
        panic!("a map")
    };
    map
}

fn own(power: f64) -> Map<String, Value> {
    map(json!({"main": {"power_pct": power, "speed_mm_min": 3000, "passes": 1}}))
}

fn settings_of(d: &Document, m: NodeId, id: NodeId) -> Option<Value> {
    d.operations(m)
        .unwrap()
        .into_iter()
        .find(|o| o.id == id)
        .unwrap()
        .settings
}

fn last_message(d: &Document) -> String {
    let frontiers = d.loro().oplog_frontiers();
    let id = frontiers.iter().next().unwrap();
    d.loro()
        .get_change(id)
        .unwrap()
        .message
        .unwrap()
        .to_string()
}

#[test]
fn settings_are_written_as_plain_values_in_one_step_and_read_back() {
    let d = Document::new(1);
    let m = laser(&d);
    let (a, b) = (op(&d, m, "cut"), op(&d, m, "score"));
    assert_eq!(settings_of(&d, m, a), None, "No settings");
    let steps = d.own_step_count();
    let writes = [
        SettingsWrite {
            operation: a,
            value: Some(own(60.0)),
        },
        SettingsWrite {
            operation: b,
            value: Some(own(20.5)),
        },
    ];
    assert!(
        d.write_operation_settings(SettingsStep::UseMaterial, &writes)
            .unwrap()
    );
    assert_eq!(
        d.own_step_count(),
        steps + 1,
        "several operations, one step"
    );
    assert!(last_message(&d).starts_with("use_material;"));
    assert_eq!(settings_of(&d, m, a), Some(Value::Object(own(60.0))));
    assert_eq!(
        settings_of(&d, m, b).unwrap()["main"]["power_pct"],
        json!(20.5)
    );
    assert!(
        !d.write_operation_settings(SettingsStep::EditSettings, &writes)
            .unwrap(),
        "nothing differs: no step"
    );
    let back = unpack(2, &pack(&d, "settings").unwrap()).unwrap();
    assert_eq!(settings_of(&back, m, a), Some(Value::Object(own(60.0))));
    let clear = [SettingsWrite {
        operation: a,
        value: None,
    }];
    assert!(
        d.write_operation_settings(SettingsStep::UseOwnNumbers, &clear)
            .unwrap()
    );
    assert_eq!(settings_of(&d, m, a), None);
}

#[test]
fn a_settings_register_that_does_not_read_keeps_the_kind_readable() {
    let d = Document::new(1);
    let m = laser(&d);
    let a = op(&d, m, "cut");
    let meta = d.operation_meta(a).unwrap();
    meta.insert_container(KEY_OP_SETTINGS, loro::LoroList::new())
        .unwrap();
    d.commit_with_label(crate::operation::RENAME_OPERATION_COMMIT_LABEL);
    let read = d.operations(m).unwrap().remove(0);
    assert_eq!(
        read.kind.as_deref(),
        Some("cut"),
        "Damaged settings, not Unknown"
    );
    assert_eq!(read.settings, Some(Value::Null));
    meta.insert(KEY_OP_SETTINGS, 7).unwrap();
    d.commit_with_label(crate::operation::RENAME_OPERATION_COMMIT_LABEL);
    assert_eq!(d.operations(m).unwrap()[0].settings, Some(json!(7)));
}

#[test]
fn a_grid_row_takes_no_settings_and_no_items() {
    let d = Document::new(1);
    let m = laser(&d);
    let row = op(&d, m, "cut");
    let meta = d.operation_meta(row).unwrap();
    meta.insert(crate::operation_codec::KEY_OP_GRID, true)
        .unwrap();
    d.commit_with_label(crate::operation::RENAME_OPERATION_COMMIT_LABEL);
    assert!(d.operations(m).unwrap()[0].grid);
    let write = [SettingsWrite {
        operation: row,
        value: Some(own(60.0)),
    }];
    assert_eq!(
        d.write_operation_settings(SettingsStep::UseMaterial, &write),
        Err(OperationError::GridRow)
    );
    let item = d.create_rect(crate::RectBounds::from_corners(
        crate::Point::new(0.0, 0.0),
        crate::Point::new(5.0, 5.0),
    ));
    assert_eq!(
        d.set_operation_members(row, &[vec![item]], true),
        Err(OperationError::GridRow)
    );
    meta.insert(crate::operation_codec::KEY_OP_GRID, "yes")
        .unwrap();
    d.commit_with_label(crate::operation::RENAME_OPERATION_COMMIT_LABEL);
    assert_eq!(
        d.operations(m).unwrap()[0].kind,
        None,
        "an op_grid that is not a boolean: Unknown operation"
    );
}

#[test]
fn a_material_edit_writes_the_marker_and_the_refresh_in_its_own_step() {
    let d = Document::new(1);
    let m = laser(&d);
    let a = op(&d, m, "cut");
    let steps = d.own_step_count();
    d.commit_material_edit(MaterialStep::RenameMaterial, "m1", 42, &[])
        .unwrap();
    assert_eq!(
        d.own_step_count(),
        steps + 1,
        "a step also without a pointing operation"
    );
    assert_eq!(
        d.material_edit_marker(),
        Some(json!({"id": "m1", "modified": 42}))
    );
    assert!(last_message(&d).starts_with("rename_material;"));
    let refreshed = [SettingsWrite {
        operation: a,
        value: Some(own(65.0)),
    }];
    d.commit_material_edit(MaterialStep::EditMaterial, "m1", 43, &refreshed)
        .unwrap();
    assert_eq!(settings_of(&d, m, a), Some(Value::Object(own(65.0))));
    assert_eq!(d.own_step_count(), steps + 2);
    let back = unpack(2, &pack(&d, "marker").unwrap()).unwrap();
    assert_eq!(
        back.material_edit_marker(),
        Some(json!({"id": "m1", "modified": 43}))
    );
}

#[test]
fn the_save_refresh_is_a_reserved_commit_without_a_step_and_only_when_something_differs() {
    let d = Document::new(1);
    let m = laser(&d);
    let a = op(&d, m, "cut");
    let steps = d.own_step_count();
    let changes = d.loro().len_changes();
    assert!(!d.refresh_material_numbers(&[]).unwrap());
    assert_eq!(
        d.loro().len_changes(),
        changes,
        "nothing differs: nothing committed"
    );
    let refreshed = [SettingsWrite {
        operation: a,
        value: Some(own(65.0)),
    }];
    assert!(d.refresh_material_numbers(&refreshed).unwrap());
    assert_eq!(d.own_step_count(), steps, "no step on the stacks");
    assert!(last_message(&d).starts_with("refresh_material_numbers;"));
    assert_eq!(settings_of(&d, m, a), Some(Value::Object(own(65.0))));
    assert!(!d.refresh_material_numbers(&refreshed).unwrap());
}
