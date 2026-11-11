//! `specs/0103-fixture-jobs` criterion 26 in thread CPU time (feature `adrs.md` §8): one tick and
//! the rebuilt Operations view on a page with 50 fixture jobs and 200 ticked parts within 16 ms;
//! the summary after a change on a page with 50 fixture jobs and 1,000 work items within 16 ms;
//! the Fixture jobs section with 50 jobs showing its first rows within 100 ms. Budgets are
//! asserted in release builds (the `performance-budgets` job); a debug build prints the times.

#![allow(clippy::unwrap_used, clippy::cast_precision_loss)]

mod cpu_time;

use std::time::Duration;

use curvyo_document_core::{
    Affine, Document, DocumentSize, FixtureAdd, FixtureJobSwitch, FixturePartKey, NewFixture,
    NewOutputPage, NodeId, Point, RectBounds, Vec2, pack,
};
use curvyo_editor_wasm::{Session, Tool};
use serde_json::json;

const BUDGET: Duration = Duration::from_millis(16);
const SECTION_BUDGET: Duration = Duration::from_millis(100);

/// Whether the times are asserted: release builds only.
const TIMED: bool = !cfg!(debug_assertions);

/// A laser page of 400 x 300 mm with `items` squares, and 10 holders of 20 slots each carrying 5
/// jobs with Output on, each job ticking 4 slots: 50 jobs, 200 ticked parts.
fn page_file(items: usize) -> (Vec<u8>, String) {
    let d = Document::new(1);
    let laser = d
        .add_output_page(NewOutputPage {
            process: "curvyo.laser",
            label: "Laser",
            profile: None,
            work_area: DocumentSize::from_mm(400.0, 300.0),
        })
        .unwrap();
    let origin = laser.origin;
    for i in 0..items {
        let (col, row) = ((i % 40) as f64, (i / 40) as f64);
        let (x, y) = (origin.x + 2.0 + col * 9.0, origin.y + 2.0 + row * 9.0);
        let _ = d.create_rect(RectBounds::from_corners(
            Point::new(x, y),
            Point::new(x + 6.0, y + 6.0),
        ));
    }
    let m = laser.kind.manufacturing().unwrap();
    let size = json!({"width_mm": 100.0, "height_mm": 60.0});
    let serde_json::Value::Object(size) = size else {
        unreachable!()
    };
    for h in 0..10 {
        let holder = d
            .add_fixture(
                m,
                &NewFixture {
                    add: FixtureAdd::Holder,
                    kind: "curvyo.laser/holder",
                    label: "Holder",
                    params: &size,
                    matrix: Some(Affine::translation(Vec2::new(10.0 + 20.0 * h as f64, 10.0))),
                },
            )
            .unwrap();
        let slots: Vec<NodeId> = (0..20)
            .map(|k| {
                let p = json!({"shape": "rect", "cx_mm": 5.0 + 4.5 * f64::from(k), "cy_mm": 30.0,
                    "w_mm": 4.0, "h_mm": 4.0});
                let serde_json::Value::Object(p) = p else {
                    unreachable!()
                };
                d.add_slot(holder, "Slot", &p).unwrap()
            })
            .collect();
        for j in 0..5 {
            let job = d
                .add_fixture_job(holder, "cut", &format!("Cut {}", h * 5 + j + 1))
                .unwrap();
            let keys: Vec<FixturePartKey> = slots[j * 4..j * 4 + 4]
                .iter()
                .map(|&node| FixturePartKey { node, suffix: None })
                .collect();
            d.set_fixture_job_targets(job, &keys, true).unwrap();
            d.set_fixture_job_switch(job, FixtureJobSwitch::Output, true)
                .unwrap();
        }
    }
    (pack(&d, "budget").unwrap(), laser.id)
}

fn open_on(bytes: &[u8], laser: &str) -> Session {
    let mut s = Session::open(2, bytes).unwrap();
    s.resize_viewport(1600.0, 1000.0);
    s.set_tool(Tool::Select);
    assert!(s.activate_page(laser));
    s
}

/// The first job of the section.
fn first_job(s: &Session) -> NodeId {
    let view = s.fixture_jobs_view().unwrap();
    s.fixture_job_named(&view.groups[0].jobs[0].id).unwrap()
}

/// The cheapest of five runs of `work` on fresh sessions made by `make`.
fn cheapest<S>(mut make: impl FnMut() -> S, mut work: impl FnMut(&mut S)) -> Duration {
    (0..5)
        .map(|_| {
            let mut s = make();
            cpu_time::cpu_time_of(|| work(&mut s)).1
        })
        .min()
        .unwrap()
}

#[test]
fn criterion_26_a_one_tick_on_a_page_with_50_jobs_and_200_ticked_parts() {
    let (bytes, laser) = page_file(0);
    let cost = cheapest(
        || {
            let s = open_on(&bytes, &laser);
            assert_eq!(s.running_fixture_jobs(), 50);
            let _ = s.fixture_jobs_view_json();
            let job = first_job(&s);
            (s, job)
        },
        |(s, job)| {
            assert!(s.set_fixture_job_part(*job, "board", true));
            assert!(s.operations_view_json().contains("summary"));
            assert!(s.fixture_jobs_view_json().contains("groups"));
        },
    );
    eprintln!("criterion 26 a: one tick and the view {cost:?} (CPU time)");
    assert!(!TIMED || cost < BUDGET, "{cost:?}");
}

#[test]
fn criterion_26_b_the_summary_after_a_change_on_50_jobs_and_1000_items() {
    let (bytes, laser) = page_file(1000);
    let cost = cheapest(
        || {
            let s = open_on(&bytes, &laser);
            let _ = s.operations_view_json();
            let _ = s.fixture_jobs_view_json();
            let job = first_job(&s);
            (s, job)
        },
        |(s, job)| {
            assert!(s.set_fixture_job_switch(*job, FixtureJobSwitch::Output, false));
            assert!(s.operations_view_json().contains("summary"));
            assert_eq!(s.fixture_jobs_view().unwrap().running, 49);
        },
    );
    eprintln!("criterion 26 b: change and summary {cost:?} (CPU time)");
    assert!(!TIMED || cost < BUDGET, "{cost:?}");
}

#[test]
fn criterion_26_c_the_section_with_50_jobs_shows_its_first_rows() {
    let (bytes, laser) = page_file(0);
    let cost = cheapest(
        || open_on(&bytes, &laser),
        |s| assert!(s.fixture_jobs_view_json().contains("\"groups\"")),
    );
    eprintln!("criterion 26 c: section first rows {cost:?} (CPU time)");
    assert!(!TIMED || cost < SECTION_BUDGET, "{cost:?}");
}
