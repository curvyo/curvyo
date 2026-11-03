//! The machine profile budgets (`specs/0026-machine-profile` criterion 14, `adrs.md` note 1): with
//! 200 profiles in the library, reading all records at start costs at most 200 ms of thread CPU
//! and building the Laser tab's profile list at most 100 ms (`tests/cpu_time/`). Like the other
//! budgets, the time is asserted in a release build (CI's `performance-budgets` job); a debug
//! build only prints.

#![allow(clippy::unwrap_used)]

mod cpu_time;

use curvyo_document_core::{Document, DocumentSize, pack};
use curvyo_editor_wasm::Session;
use curvyo_laser_core::{MachineProfile, new_profile_id, record_file_name, write_record};
use serde_json::Value;

const LOAD_BUDGET_S: f64 = 0.2;
const LIST_BUDGET_S: f64 = 0.1;

fn library_json() -> String {
    let entries: Vec<Value> = (0..200_u16)
        .map(|index| {
            let mut profile = MachineProfile::generic_laser();
            let [high, low] = index.to_be_bytes();
            let mut random = [0x5a; 16];
            random[14] = high;
            random[15] = low;
            profile.id = new_profile_id(random);
            profile.values.name = format!("Laser number {index} in the workshop");
            profile.work_area = DocumentSize::from_mm(300.0 + f64::from(index), 200.0);
            profile.connection.connector_address = format!("192.168.1.{}:8080", index % 250);
            serde_json::json!({"name": record_file_name(&profile.id), "text": write_record(&profile)})
        })
        .collect();
    Value::from(entries).to_string()
}

#[test]
fn reading_200_records_and_listing_them_stay_within_their_budgets() {
    let json = library_json();
    let mut session = Session::open(3, &pack(&Document::new(2), "budget").unwrap()).unwrap();
    let ((), load) = cpu_time::cpu_time_of(|| session.load_machine_profiles_json(&json));
    session.add_manufacturing("curvyo.laser").unwrap();
    let (view, list) = cpu_time::cpu_time_of(|| session.manufacturing_view_json());
    let view: Value = serde_json::from_str(&view).unwrap();
    assert_eq!(view["machine"]["profiles"].as_array().unwrap().len(), 201);
    let (load, list) = (load.as_secs_f64(), list.as_secs_f64());
    eprintln!("200 records: load {load:.4} s, list {list:.4} s thread CPU");
    if !cfg!(debug_assertions) {
        assert!(load <= LOAD_BUDGET_S, "{load:.4} s > {LOAD_BUDGET_S} s");
        assert!(list <= LIST_BUDGET_S, "{list:.4} s > {LIST_BUDGET_S} s");
    }
}
