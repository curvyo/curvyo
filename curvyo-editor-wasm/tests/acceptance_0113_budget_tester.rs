//! Independent budget tests for `specs/0113-fixture-sets-and-pins` criterion 42, in thread CPU
//! time (`cpu_time` helper, never wall time), asserted in release builds only and printed in
//! debug: (a) a press resolved within 2 ms and the draw list within 16 ms on 3 sets of 20
//! fixtures, 64 job fixtures and a 400-centre pin grid with Centre mark and Diameters on; (b) a
//! drag move of a job fixture with centre snapping on 1,000 shapes within 16 ms; (c) Apply of 64
//! job fixtures within 100 ms; (d) the Fixtures tab view of 16 sets and 64 job fixtures within
//! 100 ms.

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
    Direction, Document, DocumentSize, DocumentSnapping, EllipseFrame, FixtureJobSwitch, Length,
    NewOutputPage, NodeId, Point, pack,
};
use curvyo_editor_wasm::{Session, Tool};
use serde_json::Value;

const TIMED: bool = !cfg!(debug_assertions);

/// Laser pages of 1000 x 1000 mm; the first holds `circles` small circles as own items.
fn session(pages: usize, circles: usize) -> (Session, Vec<String>) {
    let d = Document::new(1);
    let mut ids = Vec::new();
    for i in 0..pages {
        let made = d
            .add_output_page(NewOutputPage {
                process: "curvyo.laser",
                label: "Laser",
                profile: None,
                work_area: DocumentSize::from_mm(1000.0, 1000.0),
            })
            .unwrap();
        if i == 0 {
            let o = d
                .pages()
                .into_iter()
                .find(|p| p.id == made.id)
                .unwrap()
                .origin;
            for k in 0..circles {
                let _ = d.create_ellipse(EllipseFrame {
                    center: Point::new(
                        o.x + 20.0 + (k % 40) as f64 * 24.0,
                        o.y + 20.0 + (k / 40) as f64 * 24.0,
                    ),
                    rx: Length::from_mm(4.0),
                    ry: Length::from_mm(4.0),
                });
            }
        }
        ids.push(made.id);
    }
    let mut s = Session::open(9150, &pack(&d, "0113-budget").unwrap()).unwrap();
    s.resize_viewport(1600.0, 1000.0);
    s.set_tool(Tool::Select);
    s.load_machine_profiles_json("[]");
    s.activate_page(&ids[0]);
    // Superseded by rework (2026-11-16): sets live in the machine profile, so the page is saved
    // as a profile before it gets sets (criterion 3: Generic laser has no "+ Set").
    let _ = s.save_machine_profile([15; 16], 1_000);
    s.wipe_history();
    (s, ids)
}

fn job_fixtures(s: &mut Session, n: usize) -> Vec<NodeId> {
    let kinds = ["pin", "screw", "holder", "clamp", "stop"];
    let mut out = Vec::new();
    for i in 0..n {
        assert!(s.arm_job_fixture(kinds[i % kinds.len()]));
        let x = (i % 8) as f64 * 100.0 - 350.0;
        let y = (i / 8) as f64 * 100.0 - 350.0;
        assert!(s.nudge_fixture_placement(x, y));
        out.push(s.drop_fixture_placement().expect("dropped"));
        s.end_fixture_placement();
    }
    out
}

fn role_jobs(s: &Session, fixture: NodeId) -> Vec<NodeId> {
    let v: Value = serde_json::from_str(&s.fixture_jobs_view_json()).unwrap();
    v["groups"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|g| g["fixture"] == fixture.full_id())
        .flat_map(|g| g["jobs"].as_array().unwrap().clone())
        .map(|j| s.fixture_job_named(j["id"].as_str().unwrap()).unwrap())
        .collect()
}

/// The page of criterion 42a; returns the 400-centre pin grid.
fn heavy(s: &mut Session) -> NodeId {
    for _ in 0..3 {
        let set = s.add_fixture_set().unwrap();
        let kinds = ["holder", "clamp", "stop", "screw", "pin"];
        for i in 0..20 {
            s.add_set_fixture(set, kinds[i % kinds.len()])
                .expect("a set fixture");
        }
    }
    let fx = job_fixtures(s, 64);
    let pin = fx[0];
    assert!(s.set_fixture_grid_value(pin, "columns", 20.0));
    assert!(s.set_fixture_grid_value(pin, "rows", 20.0));
    assert!(s.set_fixture_grid_value(pin, "column_spacing_mm", 5.0));
    assert!(s.set_fixture_grid_value(pin, "row_spacing_mm", 5.0));
    for job in role_jobs(s, pin) {
        assert!(s.set_fixture_job_switch(job, FixtureJobSwitch::Output, true));
    }
    pin
}

fn min_of(runs: usize, mut work: impl FnMut() -> Duration) -> Duration {
    (0..runs).map(|_| work()).min().unwrap()
}

fn report(what: &str, t: Duration, budget_ms: u64) {
    eprintln!("0113 budget {what}: {t:?} (budget {budget_ms} ms, asserted: {TIMED})");
    if TIMED {
        assert!(t <= Duration::from_millis(budget_ms), "{what} took {t:?}");
    }
}

#[test]
fn ac42a_a_press_within_2_ms_and_the_draw_list_within_16_ms() {
    let (mut s, _) = session(1, 0);
    heavy(&mut s);
    s.escape();
    let o = s.active_page().origin;
    let p = Point::new(o.x + 333.0, o.y + 777.0);
    let press = min_of(5, || {
        cpu_time::cpu_time_of(|| {
            s.pointer_down(p, false);
            s.pointer_up(p, false, false);
        })
        .1
    });
    report("press", press, 2);
    let list = min_of(5, || cpu_time::cpu_time_of(|| s.frame_draw_list()).1);
    report("draw list", list, 16);
}

#[test]
fn ac42b_a_snapping_drag_move_on_1000_shapes_within_16_ms() {
    let (mut s, _) = session(1, 1000);
    assert!(s.set_snapping(DocumentSnapping {
        enabled: true,
        ..DocumentSnapping::DEFAULT
    }));
    let pin = job_fixtures(&mut s, 1)[0];
    assert!(s.set_fixture_grid_value(pin, "columns", 20.0));
    assert!(s.set_fixture_grid_value(pin, "rows", 20.0));
    assert!(s.select_fixture(pin));
    let o = s.active_page().origin;
    let start = Point::new(o.x + 150.0, o.y + 150.0);
    s.pointer_down(start, false);
    let mut k = 0.0;
    let t = min_of(10, || {
        k += 1.3;
        let p = Point::new(start.x + k, start.y + k * 0.7);
        cpu_time::cpu_time_of(|| s.pointer_hover(p, false, false)).1
    });
    s.pointer_up(start, false, false);
    report("snapping drag move", t, 16);
}

#[test]
fn ac42c_apply_of_64_job_fixtures_within_100_ms() {
    let (mut s, ids) = session(2, 0);
    job_fixtures(&mut s, 64);
    assert!(s.activate_page(&ids[1]));
    let src = s
        .apply_candidate_named(&s.apply_fixture_candidates()[0].id)
        .unwrap();
    let t = min_of(3, || {
        let t = cpu_time::cpu_time_of(|| s.apply_fixtures_from_page(src)).1;
        assert_eq!(
            s.history_target_name(Direction::Undo).as_deref(),
            Some("Apply fixtures from page")
        );
        s.undo();
        t
    });
    report("apply 64", t, 100);
}

#[test]
fn ac42d_the_fixtures_tab_of_16_sets_and_64_job_fixtures_within_100_ms() {
    let (mut s, _) = session(1, 0);
    for _ in 0..16 {
        let set = s.add_fixture_set().unwrap();
        s.add_set_fixture(set, "holder").unwrap();
    }
    job_fixtures(&mut s, 64);
    let t = min_of(5, || cpu_time::cpu_time_of(|| s.fixtures_view_json()).1);
    report("fixtures tab view", t, 100);
}
