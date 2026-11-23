//! `specs/0118` slice 2b against the GRBL simulator (adrs F3, F6, Frage 334): a frame writes
//! `M5` before any motion, only `G1 ... S0`, never lights the laser, stops on Stop with the Ready
//! job kept, refuses a homed frame that would leave the travel before a byte is written, and is
//! not refused by `$32=0`. Time is stepped, never the wall clock.
#![allow(clippy::unwrap_used, clippy::float_cmp)]

mod common;

use common::grbl_bench::Bench;
use common::grbl_sim::{SimConfig, Spindle};
use curvyo_connector_core::{Message, State};

const JOB: &str = "G21\nG90\nG0 X0 Y0\nM4 S0\nG1 X-5 Y-5 F1000 S500\nM5\n";

/// A frame of the writer's shape: a rectangle in the negative quadrant GRBL homes into.
const FRAME: &str = "; Curvyo frame: a\nG21\nG90\nG17\nM5\nM9\nG1 X-20.000 Y-20.000 F3000 S0\nG1 X-60.000 Y-20.000 F3000 S0\nG1 X-60.000 Y-40.000 F3000 S0\nG1 X-20.000 Y-40.000 F3000 S0\nG1 X-20.000 Y-20.000 F3000 S0\n";

fn config() -> SimConfig {
    SimConfig {
        mpos_reports: true,
        ..SimConfig::default()
    }
}

fn homed_config() -> SimConfig {
    SimConfig {
        homing_lock: true,
        homing_steps: 20,
        travel: [400.0, 300.0],
        ..config()
    }
}

fn homed(config: SimConfig) -> Bench {
    let mut bench = Bench::connected(config);
    assert_eq!(bench.state(), State::Alarm);
    bench.app(Message::Home);
    assert!(bench.run_until(|b| b.state() == State::Idle, 500));
    bench.run_for(std::time::Duration::from_millis(300));
    bench
}

/// Steps until `done`, asserting at every step that the laser is dark.
fn run_dark(bench: &mut Bench, done: impl Fn(&Bench) -> bool, max: usize) -> bool {
    for _ in 0..max {
        if done(bench) {
            return true;
        }
        bench.step();
        assert!(!bench.sim.laser_lit(), "the laser lit during a frame");
    }
    done(bench)
}

fn assert_only_dark_lines(lines: &[String]) {
    for line in lines {
        let upper = line.to_ascii_uppercase();
        assert!(!upper.contains("M3") && !upper.contains("M4"), "{line}");
        assert!(!upper.starts_with("G0"), "{line}");
        // Reading commands of the identify pass; never `$H` or a setting.
        assert!(!upper.starts_with("$H") && !upper.contains('='), "{line}");
        if upper.starts_with("G1X") {
            assert!(upper.ends_with("S0"), "{line}");
        }
    }
}

#[test]
fn f6_a_frame_writes_m5_first_then_g1_s0_and_ends_where_it_began() {
    let mut bench = Bench::connected(config());
    let mark = bench.sim.lines.len();
    bench.frame(FRAME);
    assert_eq!(bench.state(), State::Framing);
    assert!(run_dark(&mut bench, |b| b.state() == State::Idle, 3_000));
    let lines = &bench.sim.lines[mark..];
    let m5 = lines.iter().position(|l| l == "M5").unwrap();
    let first_move = lines.iter().position(|l| l.starts_with("G1X")).unwrap();
    assert!(m5 < first_move, "{lines:?}");
    assert_only_dark_lines(lines);
    assert_eq!(lines.iter().filter(|l| l.starts_with("G1X")).count(), 5);
    assert_eq!(bench.sim.spindle, Spindle::Off);
    assert_eq!(bench.sim.faults, Vec::<String>::new());
    assert_eq!(bench.detail(), None, "no line, no notice after a frame");
    assert_eq!(bench.sim.rx_pos_for_tests(), [-20.0, -20.0]);
}

#[test]
fn f4_a_frame_keeps_the_ready_job_and_the_next_start_runs_it() {
    let mut bench = Bench::connected(config());
    let id = bench.send(JOB);
    bench.frame(FRAME);
    assert_eq!(bench.state(), State::Framing);
    assert_eq!(bench.line(), 0, "the frame's lines are nobody's progress");
    assert!(run_dark(&mut bench, |b| b.state() == State::Ready, 3_000));
    assert_eq!(bench.machine.held_job().unwrap().job_id, id);
    let mark = bench.sim.lines.len();
    bench.app(Message::Start { job_id: id });
    assert!(bench.run_until(|b| matches!(b.state(), State::Done { .. }), 3_000));
    assert!(
        bench.sim.lines[mark..].iter().any(|l| l.starts_with("M4")),
        "the job ran after the frame, the frame text did not"
    );
}

#[test]
fn f4_stop_mid_frame_returns_to_ready_with_the_job_and_no_alarm() {
    let mut config = config();
    config.steps_per_motion = 30;
    let mut bench = Bench::connected(config);
    let id = bench.send(JOB);
    bench.frame(FRAME);
    assert!(run_dark(&mut bench, |b| b.sim.lines.len() > 8, 2_000));
    assert_eq!(bench.state(), State::Framing);
    bench.app(Message::Stop);
    assert!(run_dark(&mut bench, |b| b.state() == State::Ready, 2_000));
    assert_eq!(bench.machine.held_job().unwrap().job_id, id);
    assert!(bench.alarm().is_none());
    assert!(bench.closed.is_none());
    assert_eq!(bench.sim.spindle, Spindle::Off);
    assert_eq!(bench.detail(), None);
}

#[test]
fn frage_334_a_homed_frame_that_leaves_the_travel_writes_nothing() {
    let mut bench = homed(homed_config());
    let mark = bench.sim.lines.len();
    let log_mark = bench.sim.log.len();
    // Work offset 0: -450 is 50 mm beyond the 400 mm of X.
    bench.frame("G21\nG90\nM5\nG1 X-450.000 Y-20.000 F3000 S0\n");
    assert_eq!(bench.state(), State::Idle);
    assert_eq!(
        bench.detail().as_deref(),
        Some("Outside the travel by 50 mm")
    );
    assert_eq!(bench.sim.lines[mark..], Vec::<String>::new(), "no line");
    let bytes = bench.lines_after(log_mark);
    assert!(bytes.is_empty(), "no byte: {bytes:?}");
}

#[test]
fn frage_334_a_frame_inside_the_travel_runs_when_homed() {
    let mut bench = homed(homed_config());
    bench.frame(FRAME);
    assert_eq!(bench.state(), State::Framing);
    assert!(run_dark(&mut bench, |b| b.state() == State::Idle, 3_000));
    assert_eq!(bench.detail(), None);
}

#[test]
fn an_unhomed_frame_is_not_checked_against_the_travel() {
    let mut config = config();
    config.travel = [400.0, 300.0];
    let mut bench = Bench::connected(config);
    bench.frame("G21\nG90\nM5\nG1 X-450.000 Y-20.000 F3000 S0\n");
    assert_eq!(bench.state(), State::Framing, "GRBL's own limits decide");
}

#[test]
fn f3_laser_mode_off_does_not_refuse_a_frame_but_still_refuses_a_job() {
    let mut config = config();
    config.laser_mode = false;
    let mut bench = Bench::connected(config);
    bench.frame(FRAME);
    assert_eq!(bench.state(), State::Framing, "{:?}", bench.status());
    assert!(run_dark(&mut bench, |b| b.state() == State::Idle, 3_000));
    assert_only_dark_lines(&bench.sim.lines);
    let refused = bench.machine.handle(curvyo_connector_core::Event::Message(
        Message::SubmitJob {
            job_id: common::grbl_bench::job_id(77),
            name: "job".into(),
            kind: curvyo_connector_core::JobKind::Job,
            bytes: 3,
            sha256: String::new(),
        },
        curvyo_connector_core::Peer::host(),
    ));
    assert!(refused.messages.iter().any(|m| matches!(
        m,
        Message::JobRejected {
            reason: curvyo_connector_core::RejectReason::LaserModeOff,
            ..
        }
    )));
}

#[test]
fn a_grbl_error_ends_the_frame_with_its_words_and_no_line_number() {
    let mut config = config();
    config.error_on_line = Some((7, 22));
    let mut bench = Bench::connected(config);
    bench.frame(FRAME);
    assert!(run_dark(&mut bench, |b| b.state() == State::Idle, 3_000));
    let detail = bench.detail().unwrap();
    assert!(!detail.contains("line"), "{detail}");
    assert!(detail.contains("error 22"), "{detail}");
}
