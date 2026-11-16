//! The contract of the Fixtures tab's JSON and of the role jobs in the Fixture jobs JSON
//! (`specs/0113-fixture-sets-and-pins` criteria 1, 2, 11, 12, 16, 25, 27, 28, 31, 33, 39, 40;
//! `adrs.md` finding 3): representative states are built and their view JSON compared with the
//! shared fixtures in `tests/fixtures/fixtures_view/` and `tests/fixtures/fixture_jobs_view/`,
//! which the frontend decoders read too. A drift fails. The fixtures are written only when
//! `CURVYO_UPDATE_GOLDEN` is set.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::many_single_char_names,
    clippy::assert_is_empty
)]

use std::path::PathBuf;

use curvyo_document_core::{FixtureJobSwitch, NodeId, Point};
use curvyo_editor_wasm::{Session, Tool};
use serde_json::Value;

fn dir(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

/// A session with one laser page on Generic laser, the Select tool on.
fn generic_laser() -> Session {
    let mut s = Session::new(1);
    s.resize_viewport(1200.0, 800.0);
    s.show_default_view();
    s.add_page_of_kind("curvyo.laser", None).unwrap();
    s.set_tool(Tool::Select);
    s
}

/// A session with one laser page on a saved profile, so its sets are editable (`0113` rework,
/// criterion 3).
fn laser() -> Session {
    let mut s = generic_laser();
    s.save_machine_profile([7; 16], 1_000);
    s
}

/// A locked set holding a holder, and a set holding a locked clamp.
fn locked_set() -> Session {
    let mut s = laser();
    let a = s.add_fixture_set().unwrap();
    s.add_set_fixture(a, "holder").unwrap();
    s.set_fixture_set_locked(a, true);
    let b = s.add_fixture_set().unwrap();
    let clamp = s.add_set_fixture(b, "clamp").unwrap();
    s.set_fixture_locked(clamp, true);
    s
}

/// The project of [`locked_set`] opened where its profile is not: the sets read only.
fn read_only_sets() -> Session {
    let bytes = locked_set().pack("0.1.0").unwrap();
    let page = curvyo_document_core::unpack(4, &bytes)
        .unwrap()
        .pages()
        .into_iter()
        .find(|p| p.kind.manufacturing().is_some())
        .unwrap()
        .id;
    let mut s = Session::open(3, &bytes).unwrap();
    assert!(s.activate_page(&page));
    s
}

/// A job fixture of `kind` dropped at the page-local `(x, y)`; placement ended.
fn job_fixture(s: &mut Session, kind: &str, x: f64, y: f64) -> NodeId {
    let o = s.active_page().origin;
    assert!(s.arm_job_fixture(kind));
    let p = Point::new(o.x + x, o.y + y);
    s.pointer_down(p, false);
    s.pointer_up(p, false, false);
    s.end_fixture_placement();
    s.selected_fixture().unwrap()
}

/// The role job `role` (`centre_mark`, `diameters`) of the fixture `id`.
fn role_job(s: &Session, id: NodeId, role: &str) -> NodeId {
    let jobs = s.fixture_jobs_view().unwrap();
    let group = jobs
        .groups
        .iter()
        .find(|g| g.fixture == id.full_id())
        .unwrap();
    let job = group.jobs.iter().find(|j| j.role == Some(role)).unwrap();
    s.fixture_job_named(&job.id).unwrap()
}

/// A laser page of a project file of `curvyo-document-core/tests/fixtures/`, activated.
fn opened(file: &str) -> Session {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../curvyo-document-core/tests/fixtures")
        .join(file);
    let bytes = std::fs::read(path).unwrap();
    let laser = curvyo_document_core::unpack(4, &bytes)
        .unwrap()
        .pages()
        .into_iter()
        .find(|p| p.kind.manufacturing().is_some())
        .unwrap()
        .id;
    let mut s = Session::open(3, &bytes).unwrap();
    assert!(s.activate_page(&laser));
    s
}

/// Two sets, the second off this page; a grid, a holder with two slots and its own check off,
/// a screw in the set that is off.
fn two_sets_one_off() -> Session {
    let mut s = laser();
    let a = s.add_fixture_set().unwrap();
    let b = s.add_fixture_set().unwrap();
    s.add_set_fixture(a, "grid").unwrap();
    let holder = s.add_set_fixture(a, "holder").unwrap();
    s.add_slot(holder).unwrap();
    s.add_slot(holder).unwrap();
    s.set_fixture_active(holder, false);
    s.add_set_fixture(b, "screw").unwrap();
    s.rename_fixture_set(b, "Coaster jig");
    s.set_fixture_set_on_page(b, false);
    s.set_fixture_set_unfolded(b, false);
    s
}

/// A locked 2 x 2 pin grid with Diameters on, selected.
fn locked_pin_grid() -> Session {
    let mut s = laser();
    let pin = job_fixture(&mut s, "pin", 40.0, 40.0);
    s.set_fixture_grid_value(pin, "columns", 2.0);
    s.set_fixture_grid_value(pin, "rows", 2.0);
    s.set_fixture_grid_value(pin, "column_spacing_mm", 30.0);
    let diameters = role_job(&s, pin, "diameters");
    s.set_fixture_job_switch(diameters, FixtureJobSwitch::Output, true);
    s.set_fixture_locked(pin, true);
    s
}

/// A 4 x 1 screw grid with Centre mark on and a 3 mm Size.
fn screw_grid_centre_mark() -> Session {
    let mut s = laser();
    let screw = job_fixture(&mut s, "screw", 50.0, 50.0);
    s.set_fixture_grid_value(screw, "columns", 4.0);
    s.set_fixture_grid_value(screw, "mark_mm", 3.0);
    let mark = role_job(&s, screw, "centre_mark");
    s.set_fixture_job_switch(mark, FixtureJobSwitch::Output, true);
    s
}

/// Two laser pages: two job fixtures on the first; the second active with the first picked.
fn apply_candidates() -> Session {
    let mut s = laser();
    job_fixture(&mut s, "pin", 20.0, 20.0);
    job_fixture(&mut s, "holder", 100.0, 80.0);
    s.add_page_of_kind("curvyo.laser", None).unwrap();
    let source = s.apply_fixture_candidates()[0].id.clone();
    let source = s.apply_candidate_named(&source).unwrap();
    assert!(s.set_apply_preview(Some(source)));
    s
}

/// A job pin with its two role jobs, Centre mark Output on.
fn role_jobs() -> Session {
    let mut s = laser();
    let pin = job_fixture(&mut s, "pin", 40.0, 40.0);
    let mark = role_job(&s, pin, "centre_mark");
    s.set_fixture_job_switch(mark, FixtureJobSwitch::Output, true);
    s
}

/// A screw in a set that is off this page: its jobs read "not on this page".
fn set_off() -> Session {
    let mut s = laser();
    let set = s.add_fixture_set().unwrap();
    s.add_set_fixture(set, "screw").unwrap();
    s.set_fixture_set_on_page(set, false);
    s
}

fn fixtures_states() -> Vec<(&'static str, Session)> {
    vec![
        ("empty_page", laser()),
        ("generic_laser_hint", generic_laser()),
        ("two_sets_one_off", two_sets_one_off()),
        ("locked_set", locked_set()),
        ("read_only_sets", read_only_sets()),
        ("locked_pin_grid", locked_pin_grid()),
        ("screw_grid_centre_mark", screw_grid_centre_mark()),
        ("apply_candidates", apply_candidates()),
        ("damaged_set", opened("fixture_sets_damaged_v47.curvyo")),
        (
            "old_file_job_fixtures",
            opened("fixtures_pre_sets_v46.curvyo"),
        ),
    ]
}

fn jobs_states() -> Vec<(&'static str, Session)> {
    vec![("role_jobs", role_jobs()), ("set_off", set_off())]
}

/// Compares `json` with the fixture `name` of `folder`, written first under
/// `CURVYO_UPDATE_GOLDEN`.
fn pinned(folder: &str, name: &str, json: &Value) {
    std::fs::create_dir_all(dir(folder)).unwrap();
    let path = dir(folder).join(format!("{name}.json"));
    if std::env::var_os("CURVYO_UPDATE_GOLDEN").is_some() {
        let text = format!("{}\n", serde_json::to_string_pretty(json).unwrap());
        std::fs::write(&path, text).unwrap();
    }
    let want: Value = serde_json::from_str(
        &std::fs::read_to_string(&path)
            .unwrap_or_else(|_| panic!("{} exists (CURVYO_UPDATE_GOLDEN=1)", path.display())),
    )
    .unwrap();
    assert_eq!(json, &want, "{name} drifted from {}", path.display());
}

#[test]
fn every_fixtures_tab_state_matches_its_fixture() {
    for (name, s) in fixtures_states() {
        let json: Value = serde_json::from_str(&s.fixtures_view_json()).unwrap();
        pinned("fixtures_view", name, &json);
    }
}

#[test]
fn every_role_job_state_matches_its_fixture() {
    for (name, s) in jobs_states() {
        let json: Value = serde_json::from_str(&s.fixture_jobs_view_json()).unwrap();
        pinned("fixture_jobs_view", name, &json);
    }
}

#[test]
fn the_states_hold_what_the_decoders_must_read() {
    let empty: Value = serde_json::from_str(&laser().fixtures_view_json()).unwrap();
    assert_eq!(empty["sets"], serde_json::json!([]));
    assert_eq!(empty["job_fixtures"], serde_json::json!([]));
    assert_eq!(
        empty["apply_candidates"],
        serde_json::json!([]),
        "Apply hidden"
    );
    assert_eq!(empty["can_add_set"], true);
    let two: Value = serde_json::from_str(&two_sets_one_off().fixtures_view_json()).unwrap();
    assert_eq!(two["sets_on_page"], 1);
    assert_eq!(two["sets"][1]["line2"], "Screw · not on this page");
    assert_eq!(two["sets"][0]["can_add_grid"], false);
    assert_eq!(
        two["sets"][0]["fixtures"][1]["line2"],
        "Holder · 2 slots · not on the bed"
    );
    // Superseded by `0113` rework (criterion 32 amended 2026-11-16): a set fixture has a Lock.
    assert_eq!(two["sets"][0]["fixtures"][1]["locked"], false);
    assert_eq!(two["sets"][0]["fixtures"][1]["locked_by_set"], false);
    assert_eq!(two["read_only"], Value::Null);
    let locked: Value = serde_json::from_str(&locked_set().fixtures_view_json()).unwrap();
    assert_eq!(locked["sets"][0]["locked"], true);
    assert_eq!(locked["sets"][0]["can_add"], false);
    assert_eq!(locked["sets"][0]["line2"], "Holder · 1 fixture · locked");
    assert_eq!(locked["sets"][0]["fixtures"][0]["locked_by_set"], true);
    assert_eq!(locked["sets"][1]["fixtures"][0]["locked"], true);
    let read_only: Value = serde_json::from_str(&read_only_sets().fixtures_view_json()).unwrap();
    assert!(
        read_only["read_only"]
            .as_str()
            .unwrap()
            .ends_with("is not on this computer.")
    );
    assert_eq!(read_only["can_add_set"], false);
    assert_eq!(read_only["sets"][1]["can_add"], false);
    let generic: Value = serde_json::from_str(&generic_laser().fixtures_view_json()).unwrap();
    assert_eq!(generic["can_add_set"], false);
    assert!(generic["profile_hint"].is_string());
    let pins: Value = serde_json::from_str(&locked_pin_grid().fixtures_view_json()).unwrap();
    assert_eq!(
        pins["job_fixtures"][0]["line2"],
        "Pin · 2 × 2 · ⌀ 6 mm · locked · Diameters"
    );
    let screws: Value =
        serde_json::from_str(&screw_grid_centre_mark().fixtures_view_json()).unwrap();
    assert_eq!(
        screws["job_fixtures"][0]["line2"],
        "Screw · 4 × 1 · M6 · Centre mark"
    );
    assert_eq!(screws["job_fixtures"][0]["round"]["mark_mm"], 3.0);
    let old: Value =
        serde_json::from_str(&opened("fixtures_pre_sets_v46.curvyo").fixtures_view_json()).unwrap();
    assert_eq!(
        old["sets"],
        serde_json::json!([]),
        "no set made (criterion 39)"
    );
    assert!(!old["job_fixtures"].as_array().unwrap().is_empty());
    let jobs: Value = serde_json::from_str(&set_off().fixture_jobs_view_json()).unwrap();
    assert_eq!(jobs["groups"][0]["jobs"][0]["on_this_page"], false);
    assert_eq!(jobs["groups"][0]["jobs"][0]["role"], "centre_mark");
}
