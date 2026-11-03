//! The speed gate of the workspace tree (`specs/0094-workspace` criterion 5): with 500 documents
//! in nested folders, taking a scan and expanding or collapsing a folder, with the rows the page
//! reads, each stay within one 16 ms frame of thread CPU time. The budget holds in every profile.
//! Each step is measured as the best of three runs on fresh state, so a busy machine (the full
//! gate runs thousands of tests in parallel) does not count cache misses of other threads.

#![allow(clippy::unwrap_used)]

mod cpu_time;

use std::time::Duration;

use cpu_time::cpu_time_of;
use curvyo_editor_wasm::WorkspaceGlue;

const FRAME: Duration = Duration::from_millis(16);

/// 20 folders of 5 subfolders of 5 documents: 500 documents, 120 folders.
fn scan_json() -> String {
    let mut entries = Vec::new();
    for folder in 0..20 {
        let top = format!("Job {folder}");
        entries.push(format!(
            r#"{{"path":["{top}"],"kind":"folder","modified":0}}"#
        ));
        for sub in 0..5 {
            let inner = format!("Part {sub}");
            entries.push(format!(
                r#"{{"path":["{top}","{inner}"],"kind":"folder","modified":0}}"#
            ));
            for doc in 0..5 {
                entries.push(format!(
                    r#"{{"path":["{top}","{inner}","Piece {doc}.curvyo"],"kind":"document","modified":1793707380}}"#
                ));
            }
        }
    }
    format!("[{}]", entries.join(","))
}

const RUNS: usize = 3;

/// The scan time and the slowest expand or collapse of one run on a fresh glue.
fn one_run(json: &str) -> (Duration, Duration) {
    let mut glue = WorkspaceGlue::new("Big");
    let (rows, scan) = cpu_time_of(|| {
        assert!(glue.set_scan(json));
        glue.rows_json(1_793_707_380, 60)
    });
    assert!(rows.contains("500 documents"));

    // Every folder open: all 620 rows shown.
    let mut slowest = Duration::ZERO;
    for folder in 0..20 {
        for sub in 0..5 {
            glue.set_expanded(&format!(r#"["Job {folder}","Part {sub}"]"#), true);
        }
        let (_, took) = cpu_time_of(|| {
            glue.set_expanded(&format!(r#"["Job {folder}"]"#), true);
            glue.rows_json(1_793_707_380, 60)
        });
        slowest = slowest.max(took);
    }
    let (rows, took) = cpu_time_of(|| {
        glue.set_expanded(r#"["Job 0"]"#, false);
        glue.rows_json(1_793_707_380, 60)
    });
    assert_eq!(rows.matches(r#""kind":"document""#).count(), 475);
    (scan, slowest.max(took))
}

#[test]
fn five_hundred_documents_scan_and_expand_within_a_frame_of_cpu() {
    let json = scan_json();
    let runs: Vec<_> = (0..RUNS).map(|_| one_run(&json)).collect();
    let scan = runs.iter().map(|r| r.0).min().unwrap();
    let slowest = runs.iter().map(|r| r.1).min().unwrap();
    println!("scan: {scan:?}, slowest expand with rows: {slowest:?}");
    assert!(scan < FRAME, "taking the scan: {scan:?}");
    assert!(slowest < FRAME, "slowest expand: {slowest:?}");
}
