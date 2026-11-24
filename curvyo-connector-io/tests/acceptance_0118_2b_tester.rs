//! Independent acceptance tests for `specs/0118-machine-control` slice 2b (criteria 22, 25, 26
//! and 28, the simulator part; ADR F4 and F6), written from the specification and `adrs.md`
//! before reading the implementation. The GRBL simulator of `tests/common/grbl_sim` runs in the
//! bench's stepped time (no sleeps); every test waits by condition.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::missing_panics_doc,
    missing_docs
)]

mod common;

use std::fmt::Write as _;
use std::time::Duration;

use common::grbl_bench::Bench;
use common::grbl_sim::SimConfig;
use curvyo_connector_core::{Message, State};

const JOB: &str = "G21\nG90\nG0 X0 Y0\nM4 S0\nG1 X10 Y10 F1000 S500\nM5\n";

/// A frame of laser-off moves in a 10 mm box, as the app writes it (F1 and F3).
const FRAME: &str = "; frame\nG21\nG90\nG17\nM5\nM9\nG1 X0 Y0 F3000 S0\nG1 X10 Y0 F3000 S0\n\
G1 X10 Y10 F3000 S0\nG1 X0 Y10 F3000 S0\nG1 X0 Y0 F3000 S0\n";

/// The same shape with a feed no other line uses, to find its lines in the simulator's log.
const FRAME_MARKED: &str =
    "G21\nG90\nM5\nG1 X0 Y0 F2999 S0\nG1 X10 Y0 F2999 S0\nG1 X10 Y10 F2999 S0\n";

/// A frame long enough to stay `Framing` for several steps.
fn long_frame() -> String {
    let mut text = String::from("G21\nG90\nG17\nM5\nM9\n");
    for i in 0..60 {
        let x = if i % 2 == 0 { 50 } else { 0 };
        let y = i % 3;
        let _ = writeln!(text, "G1 X{x} Y{y} F3000 S0");
    }
    text
}

/// A job of many burning moves, so it is still `Running` for a while.
fn long_job() -> String {
    let mut text = String::from("G21\nG90\nG0 X0 Y0\nM4 S0\n");
    for i in 0..200 {
        let _ = writeln!(text, "G1 X{} Y{} F1000 S500", i % 40, (i / 40) % 5);
    }
    text.push_str("M5\n");
    text
}

fn frame_moves(b: &Bench) -> Vec<String> {
    b.sim
        .lines
        .iter()
        .filter(|l| l.starts_with("G1X") && l.ends_with("S0"))
        .cloned()
        .collect()
}

fn lines_with(b: &Bench, needle: &str) -> Vec<String> {
    b.sim
        .lines
        .iter()
        .filter(|l| l.contains(needle))
        .cloned()
        .collect()
}

fn state(b: &Bench) -> State {
    b.state()
}

fn homed_flag(b: &Bench) -> bool {
    serde_json::to_value(b.status()).unwrap()["homed"] == serde_json::json!(true)
}

fn travel_config() -> SimConfig {
    SimConfig {
        travel: [380.0, 300.0],
        soft_limits: true,
        homing_lock: true,
        mpos_reports: true,
        ..SimConfig::default()
    }
}

/// Unlocked and homed, `Idle`.
fn homed_bench(config: SimConfig) -> Bench {
    let mut b = Bench::connected(config);
    b.app(Message::Unlock);
    assert!(
        b.run_until(|b| state(b) == State::Idle, 500),
        "unlock: {:?}",
        state(&b)
    );
    b.app(Message::Home);
    assert!(
        b.run_until(|b| homed_flag(b) && state(b) == State::Idle, 50_000),
        "homing did not end: {:?}",
        state(&b)
    );
    b
}

/// Steps until the frame's lines have all been written and the state is `Idle` again.
fn finish_frame(b: &mut Bench) {
    assert!(
        b.run_until(
            |b| state(b) != State::Framing && !frame_moves(b).is_empty(),
            50_000
        ),
        "the frame did not run: {:?} detail {:?}",
        state(b),
        b.detail()
    );
}

// ---------------------------------------------------------------------------------------------
// Criterion 25, F4: the laser is off; M5 comes before the first move; no M3, M4, S above 0
// ---------------------------------------------------------------------------------------------

#[test]
fn a_frame_writes_m5_first_then_only_g1_s0_moves_and_returns_to_idle() {
    let mut b = Bench::connected(SimConfig::default());
    b.frame(FRAME);
    finish_frame(&mut b);
    assert!(
        b.run_until(|b| state(b) == State::Idle, 50_000),
        "{:?}",
        state(&b)
    );
    let lines = b.sim.lines.clone();
    let m5 = lines.iter().position(|l| l == "M5").expect("an M5 line");
    let first_move = lines
        .iter()
        .position(|l| l.starts_with("G1X"))
        .expect("a move");
    assert!(m5 < first_move, "M5 before the first move: {lines:?}");
    for line in &lines {
        assert!(
            !line.contains("M3") && !line.contains("M4"),
            "no M3 or M4: {line}"
        );
        if line.starts_with("G1X") {
            assert!(line.ends_with("S0"), "every move is S0: {line}");
        }
    }
    assert_eq!(
        frame_moves(&b).len(),
        5,
        "the five moves of the frame: {:?}",
        frame_moves(&b)
    );
    assert!(b.alarm().is_none(), "no alarm: {:?}", b.alarm());
}

#[test]
fn a_frame_never_homes() {
    let mut b = Bench::connected(SimConfig {
        homing_lock: false,
        ..SimConfig::default()
    });
    b.frame(FRAME);
    finish_frame(&mut b);
    assert!(b.run_until(|b| state(b) == State::Idle, 50_000));
    assert!(
        lines_with(&b, "$H").is_empty(),
        "no homing cycle: {:?}",
        b.sim.lines
    );
}

#[test]
fn a_frame_with_a_laser_word_never_reaches_the_port() {
    let mut b = Bench::connected(SimConfig::default());
    let id = b.frame("G21\nG90\nM5\nM3 S500\nG1 X10 Y0 F3000 S0\n");
    let _ = id;
    b.run_for(Duration::from_millis(200));
    assert!(
        lines_with(&b, "M3").is_empty(),
        "the refused frame wrote no M3: {:?}",
        b.sim.lines
    );
    assert!(frame_moves(&b).is_empty(), "no move of it reached the port");
    assert_eq!(state(&b), State::Idle, "refused, nothing runs");
}

// ---------------------------------------------------------------------------------------------
// Criterion 22: $32=0 (laser mode off) does not refuse a frame; a Ready job stays held
// ---------------------------------------------------------------------------------------------

#[test]
fn a_frame_runs_with_laser_mode_off_because_the_check_proves_the_laser_is_off() {
    let mut b = Bench::connected(SimConfig {
        laser_mode: false,
        ..SimConfig::default()
    });
    b.frame(FRAME);
    finish_frame(&mut b);
    assert!(
        b.run_until(|b| state(b) == State::Idle, 50_000),
        "{:?}",
        state(&b)
    );
    assert_eq!(frame_moves(&b).len(), 5, "the frame ran: {:?}", b.sim.lines);
}

#[test]
fn a_ready_job_is_held_through_a_frame_then_starts_and_runs_to_done() {
    let mut b = Bench::connected(SimConfig::default());
    let job = b.send(JOB);
    assert_eq!(state(&b), State::Ready);
    b.frame(FRAME);
    finish_frame(&mut b);
    assert!(
        b.run_until(|b| state(b) == State::Ready, 50_000),
        "back to Ready: {:?}",
        state(&b)
    );
    b.app(Message::Start { job_id: job });
    assert!(
        b.run_until(|b| matches!(state(b), State::Done { .. }), 50_000),
        "the held job runs after the frame: {:?} {:?}",
        state(&b),
        b.detail()
    );
    assert!(
        b.sim.lines.iter().any(|l| l == "G1X10Y10F1000S500"),
        "the job's own line reached the port: {:?}",
        b.sim.lines
    );
}

// ---------------------------------------------------------------------------------------------
// Criterion 22: Busy while a job runs; Jog, Home and Start refused while Framing
// ---------------------------------------------------------------------------------------------

#[test]
fn a_frame_while_a_job_runs_writes_nothing() {
    let mut b = Bench::connected(SimConfig::default());
    b.send(&long_job());
    b.app(Message::Start {
        job_id: common::grbl_bench::job_id(1),
    });
    assert!(
        b.run_until(|b| state(b) == State::Running, 5_000),
        "{:?}",
        state(&b)
    );
    b.frame(FRAME_MARKED);
    b.run_for(Duration::from_millis(100));
    assert!(
        lines_with(&b, "F2999").is_empty(),
        "a frame while a job runs is refused and writes no byte: {:?}",
        lines_with(&b, "F2999")
    );
    assert!(
        b.run_until(|b| matches!(state(b), State::Done { .. }), 100_000),
        "the job runs on"
    );
}

#[test]
fn jog_home_and_start_are_refused_while_a_frame_is_framing() {
    let mut b = Bench::connected(SimConfig::default());
    b.send(JOB);
    b.frame(&long_frame());
    assert!(
        b.run_until(|b| state(b) == State::Framing, 5_000),
        "the frame is framing: {:?}",
        state(&b)
    );
    b.app(
        serde_json::from_value(serde_json::json!({
            "type": "Jog", "axis": "X", "direction": "Plus", "step": "10"
        }))
        .unwrap(),
    );
    b.app(Message::Home);
    b.app(Message::Start {
        job_id: common::grbl_bench::job_id(1),
    });
    b.run_for(Duration::from_millis(10));
    assert_eq!(state(&b), State::Framing, "still framing");
    assert!(
        lines_with(&b, "$J=").is_empty(),
        "no jog line during a frame"
    );
    assert!(lines_with(&b, "$H").is_empty(), "no homing during a frame");
    assert_eq!(lines_with(&b, "M3"), Vec::<String>::new());
    assert!(
        b.run_until(|b| state(b) == State::Ready, 100_000),
        "the frame ends at Ready"
    );
    assert!(lines_with(&b, "$J=").is_empty() && lines_with(&b, "$H").is_empty());
}

// ---------------------------------------------------------------------------------------------
// Criterion 10, 26: Stop mid-frame ends it with G12 and returns to Ready with the job kept
// ---------------------------------------------------------------------------------------------

#[test]
fn stop_mid_frame_resets_the_controller_returns_to_ready_with_the_job_kept() {
    let mut b = Bench::connected(SimConfig::default());
    let job = b.send(JOB);
    b.frame(&long_frame());
    assert!(
        b.run_until(|b| state(b) == State::Framing, 5_000),
        "framing: {:?}",
        state(&b)
    );
    b.run_for(Duration::from_millis(30));
    b.app(Message::Stop);
    assert!(
        b.run_until(|b| state(b) == State::Ready, 50_000),
        "Stop returns to Ready: {:?} detail {:?}",
        state(&b),
        b.detail()
    );
    assert!(
        b.sim.log.contains(&0x18),
        "the reset (G12) reached the controller"
    );
    assert!(
        b.alarm().is_none(),
        "Stop mid-frame is no alarm: {:?}",
        b.alarm()
    );
    b.app(Message::Start { job_id: job });
    assert!(
        b.run_until(
            |b| matches!(state(b), State::Done { .. } | State::Running),
            50_000
        ),
        "the kept job starts: {:?}",
        state(&b)
    );
}

#[test]
fn stop_mid_frame_from_idle_returns_to_idle() {
    let mut b = Bench::connected(SimConfig::default());
    b.frame(&long_frame());
    assert!(b.run_until(|b| state(b) == State::Framing, 5_000));
    b.app(Message::Stop);
    assert!(
        b.run_until(|b| state(b) == State::Idle, 50_000),
        "{:?}",
        state(&b)
    );
    assert!(b.alarm().is_none());
}

// ---------------------------------------------------------------------------------------------
// F6: a homed frame outside the travel is refused before the head moves (334 A)
// ---------------------------------------------------------------------------------------------

#[test]
fn a_homed_frame_outside_the_travel_is_refused_and_nothing_is_written() {
    let mut b = homed_bench(travel_config());
    // Machine coordinates after homing: the travel is [-380, 0] in X; +10 is outside.
    b.frame("G21\nG90\nM5\nG1 X10 Y0 F3000 S0\nG1 X10 Y10 F3000 S0\nG1 X0 Y0 F3000 S0\n");
    assert!(
        b.run_until(|b| state(b) == State::Idle && b.detail().is_some(), 5_000),
        "the frame is refused: {:?} {:?}",
        state(&b),
        b.detail()
    );
    let detail = b.detail().unwrap_or_default().to_lowercase();
    assert!(
        detail.contains("travel"),
        "the refusal names the travel: {detail:?}"
    );
    assert!(
        frame_moves(&b).is_empty(),
        "no move of the frame was written: {:?}",
        b.sim.lines
    );
    assert!(
        lines_with(&b, "$H").len() == 1,
        "only the homing of the test: {:?}",
        lines_with(&b, "$H")
    );
}

#[test]
fn a_homed_frame_inside_the_travel_runs() {
    let mut b = homed_bench(travel_config());
    b.frame("G21\nG90\nM5\nG1 X-10 Y-10 F3000 S0\nG1 X-20 Y-10 F3000 S0\nG1 X-20 Y-20 F3000 S0\n");
    finish_frame(&mut b);
    assert!(b.run_until(|b| state(b) == State::Idle, 50_000));
    assert_eq!(
        frame_moves(&b).len(),
        3,
        "the three moves ran: {:?}",
        b.sim.lines
    );
}

#[test]
fn an_unhomed_frame_is_not_clamped_and_runs_as_written() {
    let mut b = Bench::connected(SimConfig {
        homing_lock: false,
        travel: [380.0, 300.0],
        soft_limits: false,
        ..SimConfig::default()
    });
    assert!(!homed_flag(&b));
    b.frame("G21\nG90\nM5\nG1 X10 Y0 F3000 S0\n");
    finish_frame(&mut b);
    assert!(b.run_until(|b| state(b) == State::Idle, 50_000));
    assert!(
        frame_moves(&b).iter().any(|l| l.starts_with("G1X10Y")),
        "{:?}",
        b.sim.lines
    );
}

// ---------------------------------------------------------------------------------------------
// Criterion 28: the simulator's byte log of a frame holds no M3, M4 and no S above 0
// ---------------------------------------------------------------------------------------------

#[test]
fn the_byte_log_of_a_frame_holds_no_laser_on_word() {
    let mut b = Bench::connected(SimConfig::default());
    b.frame(&long_frame());
    finish_frame(&mut b);
    assert!(b.run_until(|b| state(b) == State::Idle, 50_000));
    let text = b.lines_after(0);
    assert!(!text.contains("M3") && !text.contains("M4"), "{text}");
    for word in text.split(|c: char| c.is_whitespace()) {
        if let Some(value) = word.strip_prefix('S') {
            assert_eq!(value, "0", "no S above 0 in the frame: {word}");
        }
    }
}
