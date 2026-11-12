//! Tester acceptance cases for `specs/0111-laser-operation-tools` in the project file, written
//! from the specification and ADR 0056 before the implementation was read: `format_version` 44,
//! the two goldens open and save back unchanged (criterion 25), a format 43 file opens with no
//! new values and is not rewritten, old clone-member keys are kept (Frage 212 A), the assign
//! command refuses a path key (ADR 0056 §2), `op_options` round trips and merges between peers
//! as one atomic value beside `op_settings` (ADR 0056 §6), and `mfg_profile.motorised_z` is kept.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::too_many_lines,
    clippy::needless_pass_by_value,
    clippy::assert_is_empty,
    missing_docs
)]

use std::collections::BTreeMap;
use std::io::{Cursor, Read};
use std::path::PathBuf;

use curvyo_document_core::{
    CURRENT_FORMAT_VERSION, Document, DocumentSize, NewOperation, NewOutputPage, NodeId,
    OperationSnapshot, SettingsStep, SettingsWrite, pack, unpack,
};
use serde_json::{Map, Value, json};

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name),
    )
    .unwrap()
}

fn members(bytes: &[u8]) -> BTreeMap<String, Vec<u8>> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut out = BTreeMap::new();
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();
        let mut data = Vec::new();
        file.read_to_end(&mut data).unwrap();
        out.insert(file.name().unwrap().to_string(), data);
    }
    out
}

fn format_of(bytes: &[u8]) -> u64 {
    let json: Value = serde_json::from_slice(&members(bytes)["manifest.json"]).unwrap();
    json["format_version"].as_u64().unwrap()
}

/// Every operation of every manufacturing, with its registers, in a comparable text.
fn all_ops(d: &Document) -> Vec<OperationSnapshot> {
    d.manufacturings()
        .into_iter()
        .flat_map(|m| d.operations(m.id).unwrap())
        .collect()
}

fn ops_text(d: &Document) -> String {
    format!("{:?}", all_ops(d))
}

fn profiles_text(d: &Document) -> String {
    format!(
        "{:?}",
        d.manufacturings()
            .into_iter()
            .map(|m| m.profile)
            .collect::<Vec<_>>()
    )
}

fn laser() -> (Document, NodeId, NodeId) {
    let d = Document::new(1);
    let made = d
        .add_output_page(NewOutputPage {
            process: "curvyo.laser",
            label: "Laser",
            profile: None,
            work_area: DocumentSize::from_mm(400.0, 300.0),
        })
        .unwrap();
    let m = made.kind.manufacturing().unwrap();
    let op = d
        .add_operation(
            m,
            &NewOperation {
                kind: "engrave",
                name: "Engrave 1",
            },
            0,
            &[],
        )
        .unwrap();
    (d, m, op)
}

fn map(v: Value) -> Map<String, Value> {
    v.as_object().unwrap().clone()
}

fn op_of(d: &Document, m: NodeId, id: NodeId) -> OperationSnapshot {
    d.operations(m)
        .unwrap()
        .into_iter()
        .find(|o| o.id == id)
        .unwrap()
}

fn merge(a: &Document, b: &Document) {
    let aa = a.export_updates(&b.version()).unwrap();
    let bb = b.export_updates(&a.version()).unwrap();
    a.import_updates(&bb).unwrap();
    b.import_updates(&aa).unwrap();
}

// ---- criterion 25: format and goldens --------------------------------------------------------

#[test]
fn c25_the_format_is_44() {
    assert_eq!(CURRENT_FORMAT_VERSION, 45);
    let (d, _, _) = laser();
    assert_eq!(format_of(&pack(&d, "t").unwrap()), 45);
}

#[test]
fn c25_both_goldens_are_format_44_and_save_back_with_the_same_registers() {
    for name in [
        "operation_tools_v44.curvyo",
        "operation_options_damaged_v44.curvyo",
    ] {
        let bytes = fixture(name);
        assert_eq!(format_of(&bytes), 44, "{name}");
        let d = unpack(2, &bytes).unwrap();
        assert_eq!(d.own_step_count(), 0, "{name}: opening writes no step");
        let again = unpack(3, &pack(&d, "0.1.0").unwrap()).unwrap();
        assert_eq!(
            ops_text(&again),
            ops_text(&d),
            "{name}: operations identical"
        );
        assert_eq!(
            profiles_text(&again),
            profiles_text(&d),
            "{name}: profile copies identical"
        );
    }
}

#[test]
fn c25_the_tools_golden_holds_what_adr_0056_section_9_names() {
    let d = unpack(2, &fixture("operation_tools_v44.curvyo")).unwrap();
    let ops = all_ops(&d);
    let engrave = ops
        .iter()
        .find(|o| {
            o.kind.as_deref() == Some("engrave")
                && o.options
                    .as_ref()
                    .is_some_and(|v| v.as_object().is_some_and(|m| m.len() == 5))
        })
        .expect("an Engrave with every option set");
    assert!(
        engrave.settings.as_ref().unwrap()["main"]["interval_mm"].is_number(),
        "and an interval"
    );
    assert!(
        ops.iter()
            .any(|o| o.kind.as_deref() == Some("engrave_outline")
                && o.settings
                    .as_ref()
                    .is_some_and(|s| !s["material"].is_null())),
        "an Engrave + Outline linked to a material"
    );
    assert!(
        ops.iter().any(|o| o.kind.as_deref() == Some("cut")
            && o.options
                .as_ref()
                .is_some_and(|v| v.get("z_offset_mm").is_some())),
        "a Cut with a Z offset"
    );
    let copies: Vec<Option<bool>> = d
        .manufacturings()
        .into_iter()
        .map(|m| {
            m.profile
                .as_ref()
                .and_then(|p| p.get("motorised_z"))
                .and_then(Value::as_bool)
        })
        .collect();
    assert!(
        copies.contains(&Some(true)),
        "a copy with the capability: {copies:?}"
    );
    assert!(
        copies.iter().any(|c| *c != Some(true)),
        "a copy without it: {copies:?}"
    );
}

#[test]
fn c25_a_format_43_file_opens_with_no_options_and_is_not_rewritten() {
    for name in ["laser_settings_v43.curvyo", "operations_v39.curvyo"] {
        let bytes = fixture(name);
        let d = unpack(2, &bytes).unwrap();
        assert_eq!(d.own_step_count(), 0, "{name}");
        for op in all_ops(&d) {
            assert_eq!(op.options, None, "{name}: {}", op.name);
            if let Some(s) = &op.settings {
                assert!(s.pointer("/main/interval_mm").is_none(), "{name}");
            }
        }
        for m in d.manufacturings() {
            assert!(
                m.profile
                    .as_ref()
                    .is_none_or(|p| p.get("motorised_z").is_none()),
                "{name}: no motorised_z written on open"
            );
        }
        let again = unpack(3, &pack(&d, "0.1.0").unwrap()).unwrap();
        assert_eq!(ops_text(&again), ops_text(&d), "{name}");
    }
}

// ---- criterion 14, Frage 212 A: clone-member keys --------------------------------------------

#[test]
fn c14_clone_member_keys_of_old_files_are_kept_and_saved_back() {
    let bytes = fixture("operations_clone_member_v39.curvyo");
    let d = unpack(2, &bytes).unwrap();
    let before = ops_text(&d);
    let again = unpack(3, &pack(&d, "0.1.0").unwrap()).unwrap();
    assert_eq!(ops_text(&again), before, "kept, saved back unchanged");
}

#[test]
fn c14_the_assign_command_refuses_a_clone_member_path() {
    let (d, m, op) = laser();
    let page = d
        .pages()
        .into_iter()
        .find(|p| p.kind.manufacturing() == Some(m))
        .unwrap();
    let a = d.create_rect(curvyo_document_core::RectBounds::from_corners(
        curvyo_document_core::Point::new(page.origin.x + 5.0, page.origin.y + 5.0),
        curvyo_document_core::Point::new(page.origin.x + 25.0, page.origin.y + 15.0),
    ));
    let b = d.create_rect(curvyo_document_core::RectBounds::from_corners(
        curvyo_document_core::Point::new(page.origin.x + 45.0, page.origin.y + 5.0),
        curvyo_document_core::Point::new(page.origin.x + 65.0, page.origin.y + 15.0),
    ));
    let steps = d.own_step_count();
    assert!(d.set_operation_members(op, &[vec![a, b]], true).is_err());
    assert_eq!(d.own_step_count(), steps, "nothing written");
    assert!(op_of(&d, m, op).members.is_empty());
    assert!(d.set_operation_members(op, &[vec![a]], true).unwrap());
}

// ---- ADR 0056 §6: op_options register --------------------------------------------------------

fn full_options() -> Value {
    json!({"crosshatch": true, "bidirectional": false, "scan_angle_deg": 15.0,
           "overscan_pct": 5, "z_offset_mm": 1.5})
}

#[test]
fn c20_c25_op_options_round_trip_and_one_step_each() {
    let (d, m, op) = laser();
    let steps = d.own_step_count();
    assert!(d.write_operation_options(op, &map(full_options())).unwrap());
    assert_eq!(d.own_step_count(), steps + 1, "one step");
    assert!(
        !d.write_operation_options(op, &map(full_options())).unwrap(),
        "same: nothing"
    );
    assert_eq!(d.own_step_count(), steps + 1);
    let again = unpack(3, &pack(&d, "0.1.0").unwrap()).unwrap();
    assert_eq!(op_of(&again, m, op).options, Some(full_options()));
    assert_eq!(
        op_of(&again, m, op).settings,
        None,
        "options without settings"
    );
}

#[test]
fn c25_a_damaged_options_value_is_kept_as_stored() {
    let (d, m, op) = laser();
    let bad = json!({"crosshatch": "yes", "bogus": 1, "scan_angle_deg": 90.1});
    d.write_operation_options(op, &map(bad.clone())).unwrap();
    let again = unpack(3, &pack(&d, "0.1.0").unwrap()).unwrap();
    assert_eq!(op_of(&again, m, op).options, Some(bad), "never repaired");
}

#[test]
fn peer_merge_keeps_an_options_toggle_and_a_concurrent_settings_write() {
    let (a, m, op) = laser();
    let b = unpack(2, &pack(&a, "t").unwrap()).unwrap();
    a.write_operation_options(op, &map(json!({"crosshatch": true})))
        .unwrap();
    let birch = json!({
        "main": {"power_pct": 60.0, "speed_mm_min": 300, "passes": 2},
        "material": {"id": "22222222-2222-4222-8222-222222222222", "name": "Birch", "modified": 5}
    });
    b.write_operation_settings(
        SettingsStep::UseMaterial,
        &[SettingsWrite {
            operation: op,
            value: Some(map(birch.clone())),
        }],
    )
    .unwrap();
    merge(&a, &b);
    for d in [&a, &b] {
        let o = op_of(d, m, op);
        assert_eq!(
            o.options,
            Some(json!({"crosshatch": true})),
            "the toggle survives"
        );
        assert_eq!(o.settings, Some(birch.clone()), "the pick survives");
    }
}

#[test]
fn peer_merge_of_two_options_writes_keeps_one_whole_value() {
    let (a, m, op) = laser();
    let b = unpack(2, &pack(&a, "t").unwrap()).unwrap();
    let va = json!({"crosshatch": true, "scan_angle_deg": 15.0});
    let vb = json!({"bidirectional": false, "overscan_pct": 7});
    a.write_operation_options(op, &map(va.clone())).unwrap();
    b.write_operation_options(op, &map(vb.clone())).unwrap();
    merge(&a, &b);
    let oa = op_of(&a, m, op).options.unwrap();
    assert_eq!(
        Some(&oa),
        op_of(&b, m, op).options.as_ref(),
        "both peers agree"
    );
    assert!(oa == va || oa == vb, "never a mix: {oa}");
}
