//! Export job file: the handed-out file, its one step, the fixture jobs it switches off and the test grids it marks (criteria 2, 4, 20, 27, 28).

#![allow(
    clippy::assert_is_empty,
    clippy::case_sensitive_file_extension_comparisons
)]

use super::{cut, laser_with_square, rows, undo_label, view};
use crate::session::Session;

#[test]
fn criteria_27_and_28_export_hands_out_the_file_and_the_write_is_one_step() {
    let (mut s, square) = laser_with_square();
    cut(&mut s, &[square]);
    let steps = s.undo_len();
    let job = s.export_job("/home/m/Coasters.curvyo").unwrap();
    assert_eq!(s.undo_len(), steps, "handing out writes nothing");
    assert!(job.name.starts_with("Coasters - Laser"), "{}", job.name);
    assert!(job.name.ends_with(".gcode"));
    assert_eq!(job.file_type, "GRBL G-code");
    let text = String::from_utf8(job.bytes.clone()).unwrap();
    assert!(text.contains("G21"), "{text}");
    assert_eq!(
        s.export_job("x").unwrap().bytes,
        job.bytes,
        "byte-identical (criterion 31)"
    );
    let (notice, ms) = s
        .export_job_done(job.token, &job.name, false, 1_000)
        .unwrap();
    assert_eq!(notice, format!("Exported {}.", job.name));
    assert_eq!(ms, crate::session::output::export::OUTPUT_NOTICE_MS);
    assert!(
        s.export_job_done(job.token, &job.name, false, 1_000)
            .is_none(),
        "once"
    );
}

#[test]
fn criterion_4_export_is_refused_while_a_block_stands() {
    let (mut s, _) = laser_with_square();
    assert!(s.export_job("").is_none());
}

#[test]
fn criteria_2_20_27_28_fixture_jobs_run_first_with_the_pick_and_turn_off_on_export() {
    use curvyo_document_core::FixtureJobSwitch;
    let (mut s, square) = laser_with_square();
    cut(&mut s, &[square]);
    let holder = crate::session::fixtures::add_fixture(&mut s, "holder").unwrap();
    assert!(s.select_fixture(holder));
    let job = s.add_fixture_job("score").unwrap();
    assert!(s.set_fixture_job_part(job, "board", true));
    assert!(s.set_fixture_job_switch(job, FixtureJobSwitch::Output, true));
    let v = view(&s);
    let first = &v["checks"][0];
    assert_eq!(
        first["text"],
        "Fixture jobs have no material. Pick one above."
    );
    assert_eq!(first["button"], "Fixture jobs");
    assert_eq!(first["target"]["kind"], "fixture_pick");
    // Superseded by 0029 rework (2026-11-17): the pick and the fixture run lines sit in the
    // Fixture jobs section (P32); the operation rows carry their own digit (P26).
    let fixture = |s: &Session| -> serde_json::Value {
        serde_json::from_str::<serde_json::Value>(&s.fixture_jobs_view_json()).unwrap()["output"]
            .clone()
    };
    let f = fixture(&s);
    assert!(f["pick"]["material"].is_null());
    assert_eq!(f["runs"][0]["kind"], "score");
    assert_eq!(f["runs"][0]["time"], "–");
    assert_eq!(
        f["time"], "–",
        "no run has a time: the header shows none (UX U4)"
    );
    assert!(f["runs"][0]["numbers"].as_array().unwrap().is_empty());
    let birch = s.add_material("Birch 3 mm", 3.0, &[1; 16]).unwrap();
    let options = &fixture(&s)["pick"]["options"];
    assert_eq!(options[0]["pickable"], false);
    assert_eq!(options[0]["note"], "No Score set");
    let laser = s.active_profile_id().unwrap();
    s.add_material_set(&birch, &laser, "score", &[2; 16])
        .unwrap();
    assert!(s.set_fixture_material(&birch));
    assert_eq!(undo_label(&mut s), "Pick fixture material");
    let v = view(&s);
    let f = fixture(&s);
    assert_eq!(f["pick"]["name"], "Birch 3 mm");
    assert!(f["runs"][0]["time"].as_str().unwrap().ends_with(" s"));
    assert_eq!(f["time"], f["runs"][0]["time"], "the header's time");
    assert_eq!(
        rows(&s)["rows"][0]["run"]["digit"],
        1,
        "the operation keeps its row's digit"
    );
    assert_eq!(v["export"]["enabled"], true, "{:#}", v["checks"]);
    let file = s.export_job("").unwrap();
    let text = String::from_utf8(file.bytes.clone()).unwrap();
    let score = text.find("Fixture jobs").unwrap();
    let cut_at = text.find("Cut 1").unwrap();
    assert!(score < cut_at, "the fixture jobs run first (criterion 20)");
    let (notice, ms) = s.export_job_done(file.token, &file.name, true, 5).unwrap();
    assert_eq!(
        notice,
        format!(
            "Downloaded {}.\nOutput turned off on 1 fixture job.",
            file.name
        )
    );
    assert_eq!(ms, crate::session::output::export::OUTPUT_NOTICE_LONG_MS);
    assert_eq!(undo_label(&mut s), "Export job file");
    assert!(
        !s.document.fixture_job(job).unwrap().output,
        "criterion 28 a"
    );
    assert!(
        fixture(&s).is_null(),
        "the pick and run lines leave the section"
    );
    assert_eq!(
        rows(&s)["rows"][0]["run"]["digit"],
        1,
        "rings keep their numbers"
    );
    let _ = s.undo();
    assert!(
        s.document.fixture_job(job).unwrap().output,
        "undo restores it"
    );
}

#[test]
fn criterion_28_c_an_export_marks_its_test_grid_lasered_and_undo_keeps_it() {
    let (mut s, birch, group) = crate::session::materials::tests::birch_grid();
    let v = view(&s);
    assert!(
        v["export"]["enabled"].as_bool().unwrap(),
        "the grid exports: {:#}",
        v["checks"]
    );
    let file = s.export_job("").unwrap();
    s.export_job_done(file.token, &file.name, false, 1_762_948_800_000)
        .unwrap();
    let grid = s.grid_object(group).unwrap();
    assert_eq!(grid.lasered, Some(1_762_948_800_000));
    let test = grid.recipe.id;
    let lasered = |s: &Session| {
        s.materials()
            .find(&birch)
            .and_then(|m| m.test(test))
            .and_then(|t| t.lasered)
    };
    assert_eq!(lasered(&s), Some(1_762_948_800_000), "the record too");
    if undo_label(&mut s) == "Export job file" {
        let _ = s.undo();
    }
    assert!(
        s.grid_object(group).unwrap().lasered.is_some(),
        "Lasered stays"
    );
    assert_eq!(lasered(&s), Some(1_762_948_800_000));
}
