//! Over-limit profile records of `specs/0103-fixture-jobs` (criteria 23, 24, ADR 0052 §7): a set
//! of 101 jobs or 201 part tokens is damaged, as the maximum set of 100 and 200 reads. Written
//! from the spec; the maximum is the golden `format3_max_set.json`, edited by one job or token.

#![allow(clippy::unwrap_used, missing_docs)]

use std::path::PathBuf;

use curvyo_laser_core::{RecordError, read_record, record_file_name};
use serde_json::Value;

const ID: &str = "6f1c2a4e-9b3d-4c1a-8e2f-0a1b2c3d4e5f";

fn max_set() -> Value {
    let text = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/machine_profiles/format3_max_set.json"),
    )
    .unwrap();
    serde_json::from_str(&text).unwrap()
}

fn read(value: &Value) -> Result<(), RecordError> {
    let text = serde_json::to_string(value).unwrap();
    read_record(&record_file_name(ID), &text).map(|_| ())
}

#[test]
fn the_maximum_set_reads_and_one_job_more_is_damaged() {
    let mut value = max_set();
    assert_eq!(read(&value), Ok(()), "the maximum reads");
    let fixtures = value["fixtures"].as_array_mut().unwrap();
    let first = fixtures[0]["jobs"][0].clone();
    fixtures[0]["jobs"].as_array_mut().unwrap().push(first);
    assert_eq!(read(&value), Err(RecordError::Damaged), "101 jobs");
}

#[test]
fn the_maximum_set_reads_and_one_token_more_is_damaged() {
    let mut value = max_set();
    let fixtures = value["fixtures"].as_array_mut().unwrap();
    let mut added = false;
    'search: for fixture in fixtures.iter_mut() {
        let slots = fixture["slots"].as_array().map_or(0, Vec::len);
        let Some(jobs) = fixture["jobs"].as_array_mut() else {
            continue;
        };
        for job in jobs.iter_mut() {
            let targets = job["targets"].as_array_mut().unwrap();
            let token = (0..slots)
                .map(|k| format!("slot:{k}"))
                .find(|t| !targets.iter().any(|x| x.as_str() == Some(t.as_str())));
            if let Some(token) = token {
                targets.push(Value::String(token));
                added = true;
                break 'search;
            }
        }
    }
    assert!(added, "the golden has a job with a free slot token");
    assert_eq!(read(&value), Err(RecordError::Damaged), "201 part tokens");
}
