//! Independent budget tests for `specs/0103-fixture-jobs` criterion 26 (ADR 0052 §8): one tick
//! with 50 jobs and 200 ticked parts within 16 ms, the summary after a change with 50 jobs and 1,000
//! work items within 16 ms, and the section with 50 jobs showing its first rows within 100 ms.
//! Thread CPU time, asserted in release builds only (`cpu_time` helper, as `operations_budget.rs`).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_precision_loss,
    clippy::too_many_lines,
    missing_docs
)]

#[path = "cpu_time/mod.rs"]
mod cpu_time;

use std::time::Duration;

use curvyo_document_core::{
    Document, DocumentSize, FixtureJobSwitch, NewOutputPage, NodeId, Point, RectBounds, pack,
};
use curvyo_editor_wasm::{Session, Tool};

/// Whether the times are asserted: release builds only.
const TIMED: bool = !cfg!(debug_assertions);

/// A laser page with 25 holders of 4 slots, 50 jobs (two per holder) with 200 ticked parts, and
/// `items` rectangles on the page as work items.
fn page(items: usize) -> (Session, Vec<NodeId>) {
    let d = Document::new(1);
    let made = d
        .add_output_page(NewOutputPage {
            process: "curvyo.laser",
            label: "Laser",
            profile: None,
            work_area: DocumentSize::from_mm(1000.0, 1000.0),
        })
        .unwrap();
    let origin = d
        .pages()
        .into_iter()
        .find(|p| p.id == made.id)
        .unwrap()
        .origin;
    for i in 0..items {
        let x = origin.x + (i % 40) as f64 * 8.0;
        let y = origin.y + (i / 40) as f64 * 4.0;
        let _ = d.create_rect(RectBounds::from_corners(
            Point::new(x, y),
            Point::new(x + 6.0, y + 3.0),
        ));
    }
    let mut s = Session::open(9401, &pack(&d, "0103-budget").unwrap()).unwrap();
    s.resize_viewport(1600.0, 1000.0);
    s.set_tool(Tool::Select);
    s.load_machine_profiles_json("[]");
    s.activate_page(&made.id);
    s.wipe_history();
    let mut jobs = Vec::new();
    for _ in 0..25 {
        let h = s.add_fixture("holder").expect("a holder");
        for _ in 0..4 {
            s.add_slot(h).expect("a slot");
        }
        s.select_fixture(h);
        for kind in ["engrave", "engrave_outline"] {
            let job = s.add_fixture_job(kind).expect("a job");
            assert!(s.set_fixture_job_part(job, "slot:0", true));
            assert!(s.set_fixture_job_part(job, "slot:1", true));
            jobs.push(job);
        }
    }
    (s, jobs)
}

fn min_of(runs: usize, mut work: impl FnMut() -> Duration) -> Duration {
    (0..runs).map(|_| work()).min().unwrap()
}

#[test]
fn c26a_one_tick_with_50_jobs_and_200_ticked_parts_is_within_16_ms() {
    let (mut s, jobs) = page(0);
    let job = jobs[0];
    let mut on = false;
    let t = min_of(5, || {
        on = !on;
        let on_now = on;
        cpu_time::cpu_time_of(|| {
            assert!(s.set_fixture_job_part(job, "slot:2", on_now));
            let _ = s.operations_view_json();
            let _ = s.fixture_jobs_view_json();
        })
        .1
    });
    if TIMED {
        assert!(t <= Duration::from_millis(16), "one tick took {t:?}");
    }
}

#[test]
fn c26b_the_summary_after_a_change_with_1000_work_items_is_within_16_ms() {
    let (mut s, jobs) = page(1000);
    let job = jobs[1];
    let mut on = false;
    let t = min_of(5, || {
        on = !on;
        let on_now = on;
        cpu_time::cpu_time_of(|| {
            assert!(s.set_fixture_job_switch(job, FixtureJobSwitch::Output, on_now));
            let _ = s.fixture_jobs_view_json();
            let _ = s.operations_view_json();
        })
        .1
    });
    if TIMED {
        assert!(
            t <= Duration::from_millis(16),
            "a job command with 1,000 items took {t:?}"
        );
    }
}

#[test]
fn c26c_the_section_with_50_jobs_shows_its_first_rows_within_100_ms() {
    let (s, _) = page(0);
    let t = min_of(5, || cpu_time::cpu_time_of(|| s.fixture_jobs_view_json()).1);
    if TIMED {
        assert!(t <= Duration::from_millis(100), "the section took {t:?}");
    }
}
