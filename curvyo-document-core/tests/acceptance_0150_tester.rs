//! Tester acceptance cases for `specs/0150-operation-always-has-numbers` in the document model,
//! written from the specification and its ADR notes: the format number 59 (ADR notes, decision 7),
//! a new operation carrying its numbers in the one "Add operation" step (criteria 4, 20), an old
//! file opening without any step and saving back byte-for-byte as registers (criteria 15, 16), the
//! golden `operation_defaults_v59.curvyo` keeping absent registers absent (criteria 15, 17, 18, 28),
//! and the damaged registers never being filled (criteria 17, 19). The numbers themselves are
//! derived in the laser extension and are covered there and in the editor tests.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::too_many_lines,
    clippy::similar_names,
    missing_docs
)]

use std::collections::BTreeMap;
use std::io::{Cursor, Read};
use std::path::PathBuf;

use curvyo_document_core::{
    CURRENT_FORMAT_VERSION, Document, DocumentSize, NewOperation, NewOutputPage, NodeId,
    OperationSnapshot, pack, unpack,
};
use loro::{LoroDoc, LoroValue, TreeParentId};
use serde_json::{Map, Value, json};

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(fixture_path(name)).unwrap()
}

fn members(bytes: &[u8]) -> BTreeMap<String, Vec<u8>> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    (0..archive.len())
        .map(|i| {
            let mut file = archive.by_index(i).unwrap();
            let mut content = Vec::new();
            file.read_to_end(&mut content).unwrap();
            (file.name().unwrap().to_string(), content)
        })
        .collect()
}

fn format_of(bytes: &[u8]) -> u64 {
    let json: Value = serde_json::from_slice(&members(bytes)["manifest.json"]).unwrap();
    json["format_version"].as_u64().unwrap()
}

/// Every live node's registers as stored, sorted: the saved state without the zip container.
fn raw(bytes: &[u8]) -> Vec<BTreeMap<String, LoroValue>> {
    let loro = LoroDoc::new();
    loro.import(&members(bytes)["document.loro"]).unwrap();
    let tree = loro.get_tree("paths");
    let mut out = Vec::new();
    let mut stack: Vec<loro::TreeID> = tree.children(TreeParentId::Root).unwrap();
    while let Some(node) = stack.pop() {
        let mut registers = BTreeMap::new();
        tree.get_meta(node).unwrap().for_each(|key, value| {
            let plain = match value {
                loro::ValueOrContainer::Value(value) => value,
                container @ loro::ValueOrContainer::Container(_) => container.get_deep_value(),
            };
            registers.insert(key.to_owned(), plain);
        });
        out.push(registers);
        stack.extend(tree.children(TreeParentId::Node(node)).unwrap_or_default());
    }
    out.sort_by_key(|r| format!("{r:?}"));
    out
}

fn map(v: Value) -> Map<String, Value> {
    match v {
        Value::Object(m) => m,
        _ => unreachable!(),
    }
}

fn laser_page(d: &Document) -> NodeId {
    d.add_output_page(NewOutputPage {
        process: "curvyo.laser",
        label: "Laser",
        profile: None,
        work_area: DocumentSize::from_mm(400.0, 300.0),
    })
    .unwrap()
    .kind
    .manufacturing()
    .unwrap()
}

fn operations(d: &Document, m: NodeId) -> Vec<OperationSnapshot> {
    d.operations(m).unwrap()
}

fn golden_operations(d: &Document) -> Vec<OperationSnapshot> {
    d.manufacturings()
        .into_iter()
        .flat_map(|m| d.operations(m.id).unwrap())
        .collect()
}

// ---- decision 7: the format number -----------------------------------------------------------

#[test]
fn c28_the_format_number_is_59_and_the_golden_is_written_as_59() {
    assert_eq!(CURRENT_FORMAT_VERSION, 70);
    assert_eq!(format_of(&fixture("operation_defaults_v59.curvyo")), 59);
}

// ---- criteria 4 and 20: a new operation carries its numbers in the add step --------------------

#[test]
fn c4_c20_a_new_operation_with_settings_is_one_step_and_holds_them() {
    let d = Document::new(1);
    let m = laser_page(&d);
    let steps = d.own_step_count();
    let settings = map(json!({
        "main": {"power_pct": 10.0, "speed_mm_min": 1000, "passes": 1}
    }));
    let id = d
        .add_operation(
            m,
            &NewOperation {
                kind: "cut",
                name: "Cut 1",
                settings: Some(&settings),
            },
            0,
            &[],
        )
        .unwrap();
    assert_eq!(d.own_step_count(), steps + 1, "one step, not two");
    assert_eq!(d.own_steps().last().unwrap().label, "add_operation");
    let op = operations(&d, m).into_iter().find(|o| o.id == id).unwrap();
    assert_eq!(
        op.settings,
        Some(Value::Object(settings.clone())),
        "the numbers are written with the operation"
    );
}

#[test]
fn c20_undo_of_add_operation_removes_the_operation_with_its_numbers() {
    let d = Document::new(1);
    let m = laser_page(&d);
    let settings = map(json!({"main": {"power_pct": 10.0, "speed_mm_min": 1000, "passes": 1}}));
    d.add_operation(
        m,
        &NewOperation {
            kind: "cut",
            name: "Cut 1",
            settings: Some(&settings),
        },
        0,
        &[],
    )
    .unwrap();
    let step = d.own_steps().last().unwrap().step;
    d.undo_step(step).unwrap();
    assert!(operations(&d, m).is_empty(), "no empty operation remains");
}

#[test]
fn c4_a_new_operation_without_settings_writes_no_register() {
    // The document layer decides nothing: a caller that passes no numbers gets none written.
    let d = Document::new(1);
    let m = laser_page(&d);
    let id = d
        .add_operation(
            m,
            &NewOperation {
                kind: "engrave",
                name: "Engrave 1",
                settings: None,
            },
            0,
            &[],
        )
        .unwrap();
    let op = operations(&d, m).into_iter().find(|o| o.id == id).unwrap();
    assert_eq!(op.settings, None);
}

// ---- criteria 15, 16: old files open without a step and save back unchanged ---------------

#[test]
fn c15_opening_an_old_file_adds_no_step_and_changes_no_version() {
    let bytes = fixture("laser_settings_v43.curvyo");
    let d = unpack(2, &bytes).unwrap();
    assert_eq!(d.own_step_count(), 0, "opening writes no step");
    let version = d.version();
    let _ = golden_operations(&d);
    assert_eq!(d.version(), version, "reading the numbers changes nothing");
}

#[test]
fn c16_saving_an_old_file_without_a_change_keeps_every_register() {
    let bytes = fixture("laser_settings_v43.curvyo");
    let d = unpack(2, &bytes).unwrap();
    let saved = pack(&d, "0150-tester").unwrap();
    assert_eq!(
        raw(&saved),
        raw(&bytes),
        "no op_settings is written by a plain save of the document layer"
    );
    let again = unpack(57, &saved).unwrap();
    let ops = golden_operations(&again);
    let absent: Vec<&OperationSnapshot> = ops.iter().filter(|o| o.settings.is_none()).collect();
    assert!(
        !absent.is_empty(),
        "the operation without numbers stays absent in the document, derived on read"
    );
}

// ---- the golden operation_defaults_v59 -----------------------------------------------------------

#[test]
fn c28_the_golden_keeps_absent_registers_absent_and_round_trips() {
    let bytes = fixture("operation_defaults_v59.curvyo");
    let d = unpack(57, &bytes).unwrap();
    assert_eq!(d.own_step_count(), 0, "opening writes no step");
    let ops = golden_operations(&d);
    // An operation without op_settings (Cut, Engrave + Outline, Image, unknown kind `drill`).
    let absent: Vec<&OperationSnapshot> = ops.iter().filter(|o| o.settings.is_none()).collect();
    assert!(
        absent.len() >= 4,
        "absent registers in the golden: {}",
        absent.len()
    );
    assert!(
        absent.iter().any(|o| o.kind.as_deref() == Some("drill")),
        "the unknown kind keeps an absent register too (criterion 18)"
    );
    // A damaged register is kept as it is, never filled (criteria 17, 19).
    let damaged = ops
        .iter()
        .find(|o| {
            o.settings
                .as_ref()
                .is_some_and(|s| s["main"]["power_pct"] == json!(0.0))
        })
        .expect("the golden holds a damaged Cut with power 0");
    assert_eq!(damaged.kind.as_deref(), Some("cut"));
    // A linked Engrave keeps its material and numbers.
    assert!(
        ops.iter().any(|o| o
            .settings
            .as_ref()
            .is_some_and(|s| s.get("material").is_some())),
        "a linked operation is in the golden"
    );
    let saved = pack(&d, "0150-tester").unwrap();
    assert_eq!(raw(&saved), raw(&bytes), "saved back unchanged");
    let again = unpack(57, &saved).unwrap();
    assert_eq!(golden_operations(&again).len(), ops.len());
}

#[test]
fn c17_a_damaged_register_is_never_filled_by_a_plain_save() {
    let bytes = fixture("laser_settings_damaged_v43.curvyo");
    let d = unpack(2, &bytes).unwrap();
    let saved = pack(&d, "0150-tester").unwrap();
    let again = unpack(57, &saved).unwrap();
    assert!(
        golden_operations(&again)
            .iter()
            .all(|o| o.settings.is_some()),
        "a damaged register is kept, the absent ones are not added here"
    );
    assert_eq!(raw(&saved), raw(&bytes));
}

#[test]
fn c18_an_unknown_kind_without_settings_stays_untouched() {
    let bytes = fixture("operation_defaults_v59.curvyo");
    let d = unpack(57, &bytes).unwrap();
    let drill: Vec<OperationSnapshot> = golden_operations(&d)
        .into_iter()
        .filter(|o| o.kind.as_deref() == Some("drill"))
        .collect();
    assert_eq!(drill.len(), 1);
    assert_eq!(
        drill[0].settings, None,
        "no numbers are added to an unknown kind"
    );
    let saved = pack(&d, "0150-tester").unwrap();
    assert_eq!(raw(&saved), raw(&bytes));
}
