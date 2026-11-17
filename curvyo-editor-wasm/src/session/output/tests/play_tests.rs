//! The play bar: the slider, Play and its pause on a change (criterion 37).

#![allow(
    clippy::assert_is_empty,
    clippy::case_sensitive_file_extension_comparisons
)]

use super::{cut, laser_with_square};

#[test]
fn criterion_37_play_runs_from_the_slider_stops_at_the_end_and_restarts_at_zero() {
    let (mut s, square) = laser_with_square();
    cut(&mut s, &[square]);
    let page = s.active_page().id;
    assert!(s.play_view().is_none(), "no bar without the Toolpath view");
    s.toggle_toolpath_view(&page);
    let bar = s.play_view().unwrap();
    assert!(
        (bar.fraction - 1.0).abs() < f64::EPSILON,
        "starts at the end (P10)"
    );
    assert!(bar.enabled);
    assert!(s.toggle_play(0.0), "Play at the end starts at 0");
    assert!(s.play_tick(5_000.0));
    let half = s.play_view().unwrap();
    assert!((half.fraction - 0.5).abs() < 1e-9);
    assert!(half.playing);
    assert!(!s.play_tick(20_000.0), "stops at the end");
    assert!((s.play_view().unwrap().fraction - 1.0).abs() < f64::EPSILON);
    s.set_play_position(0.5);
    s.step_play_position(1.0);
    assert!((s.play_view().unwrap().fraction - 0.51).abs() < 1e-9);
    s.step_play_position(-100.0);
    assert!(s.play_view().unwrap().fraction.abs() < f64::EPSILON);
}

#[test]
fn criterion_37_slider_at_half_reads_half_the_total_time() {
    let (mut s, square) = laser_with_square();
    cut(&mut s, &[square]);
    let job = s.laser_job().unwrap();
    let head = job.timeline.head_at(&job.built.job, 0.5);
    assert!((head.elapsed_s - job.timeline.total_s() / 2.0).abs() < 0.5);
}

#[test]
fn play_pauses_when_the_job_changes() {
    let (mut s, square) = laser_with_square();
    let id = cut(&mut s, &[square]);
    let page = s.active_page().id;
    s.toggle_toolpath_view(&page);
    assert!(s.toggle_play(0.0));
    s.edit_operation_setting(id, "main", "power", 20.0).unwrap();
    assert!(!s.play_tick(100.0), "an edit while playing pauses (P12)");
}
