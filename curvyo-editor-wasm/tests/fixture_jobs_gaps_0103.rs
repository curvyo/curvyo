//! Acceptance tests of `specs/0103-fixture-jobs` the tester left open: the session-level damage
//! total of a job (criterion 22, ADR 0052 §3: the document counts 3, the session 4) and the
//! independence of a fixture job and a work operation (criterion 16). Written from the spec.

#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

mod operation_shims;
use operation_shims::add_operation;
use std::path::PathBuf;

use curvyo_document_core::{
    Document, DocumentSize, FixtureJobSwitch, NewOutputPage, NodeId, OperationSwitch, pack, unpack,
};
use curvyo_editor_wasm::{Session, Tool};
use serde_json::Value;

const LASER: &str = "curvyo.laser";

/// The laser page with one rectangle on it (a work operation is made by assigning one,
/// `specs/0111` criterion 8).
fn one_page() -> Session {
    let d = Document::new(1);
    let made = d
        .add_output_page(NewOutputPage {
            process: LASER,
            label: "Laser",
            profile: None,
            work_area: DocumentSize::from_mm(600.0, 400.0),
        })
        .unwrap();
    let o = made.origin;
    let _ = d.create_rect(curvyo_document_core::RectBounds::from_corners(
        curvyo_document_core::Point::new(o.x + 500.0, o.y + 300.0),
        curvyo_document_core::Point::new(o.x + 520.0, o.y + 320.0),
    ));
    let mut s = Session::open(9401, &pack(&d, "0103-gaps").unwrap()).unwrap();
    s.resize_viewport(1600.0, 1000.0);
    s.set_tool(Tool::Select);
    s.load_machine_profiles_json("[]");
    s.activate_page(&made.id);
    s.wipe_history();
    s
}

fn holder(s: &mut Session, slots: usize) -> NodeId {
    let h = s.add_fixture("holder").expect("a holder");
    for _ in 0..slots {
        s.add_slot(h).expect("a slot");
    }
    h
}

fn fj(s: &Session) -> Value {
    serde_json::from_str(&s.fixture_jobs_view_json()).unwrap()
}

fn ops(s: &Session) -> Value {
    serde_json::from_str(&s.operations_view_json()).unwrap()
}

fn golden_session(name: &str, peer: u64) -> Session {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../curvyo-document-core/tests/fixtures")
        .join(name);
    let bytes = std::fs::read(&path).unwrap();
    let d = unpack(peer, &bytes).unwrap();
    let mut s = Session::open(peer, &bytes).unwrap();
    s.load_machine_profiles_json("[]");
    s.resize_viewport(1600.0, 1000.0);
    for page in d.pages() {
        if s.activate_page(&page.id) && s.operations_view_json() != "null" {
            return s;
        }
    }
    panic!("{name} has no laser page with an operations view");
}

/// ADR 0052 §3 and criterion 22: the golden holds four damaged values of its job (a text show, a
/// text output, a false target and a `.tapes` key). The session total is 4 and one notice names
/// the fixture's job; the document-level count (3) is checked in document-core.
#[test]
fn ac22_the_session_counts_four_damaged_job_values_in_one_notice() {
    let s = golden_session("fixture_jobs_damaged_v41.curvyo", 9402);
    let v = fj(&s);
    assert_eq!(v["damaged"], 4, "the session total of the job values");
    assert_eq!(
        v["damage_notices"],
        serde_json::json!(["4 values damaged on holder 1 job."]),
        "one notice per fixture, the spec's wording"
    );
}

/// Criterion 16: a fixture job with Output on and a work operation are independent; no switch of
/// one changes the other.
#[test]
fn ac16_a_fixture_job_and_a_work_operation_do_not_change_each_other() {
    let mut s = one_page();
    let h = holder(&mut s, 0);
    s.select_fixture(h);
    let job = s.add_fixture_job("engrave").unwrap();
    assert!(s.set_fixture_job_part(job, "board", true));
    assert!(s.set_fixture_job_switch(job, FixtureJobSwitch::Output, true));
    let op = add_operation(&mut s, "cut").expect("a work operation");
    let runs_before = (
        fj(&s)["running"].clone(),
        ops(&s)["summary"]["operations"].clone(),
    );
    assert_eq!(runs_before.0, 1, "the fixture job runs");

    assert!(s.set_operation_switch(op, OperationSwitch::Output, false));
    assert_eq!(
        fj(&s)["running"],
        1,
        "the work switch does not stop the job"
    );
    assert!(s.set_fixture_job_switch(job, FixtureJobSwitch::Output, false));
    assert_eq!(fj(&s)["running"], 0);
    assert_eq!(
        ops(&s)["summary"]["operations"],
        runs_before.1,
        "the job switch does not change the work operations"
    );
}
