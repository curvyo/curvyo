//! `specs/0127` safety rules added by the architect's review of the laser-driving code, against the
//! GRBL simulator over an in-memory channel with stepped time: laser off in a pause without
//! relying on GRBL's build (G10, G34), the loss rule during the stop's reset (G12, G15), a banner
//! during the stop's hold, the `Bf:` cross-check for a foreign `ok` (G28), the safety bytes alone
//! at a loss, and no job in the door, check or sleep state.
#![allow(
    clippy::unwrap_used,
    clippy::format_push_string,
    clippy::naive_bytecount,
    clippy::assert_is_empty
)]

mod common;

use std::time::Duration;

use common::grbl_bench::{Bench, DT, job_id};
use common::grbl_sim::{SimConfig, Spindle};
use curvyo_connector_core::{
    ControllerCommand, Event, Job, JobKind, JobResult, Message, RejectReason, State,
};
use curvyo_connector_io::controller::Controller;
use curvyo_connector_io::grbl::controller::SAFETY;

const STOPPED: State = State::Done {
    result: JobResult::Stopped,
};
const FAILED: State = State::Done {
    result: JobResult::Failed,
};
const COMPLETED: State = State::Done {
    result: JobResult::Completed,
};
const LOSS: Duration = Duration::from_secs(3);
const M3_JOB: &str =
    include_str!("../../curvyo-laser-core/tests/fixtures/output/grbl/m3_constant_power.gcode");

fn long_job() -> String {
    let mut job = String::from("G21\nG90\nM4 S0\n");
    for i in 0..400 {
        job += &format!("G1 X{} Y{} F600 S500\n", i % 50 + 1, i % 7 + 1);
    }
    job + "M5\n"
}

fn slow() -> SimConfig {
    SimConfig {
        steps_per_motion: 20,
        decel_steps: 30,
        ..SimConfig::default()
    }
}

fn running(config: SimConfig) -> Bench {
    let mut bench = Bench::connected(config);
    bench.start(&long_job());
    assert!(bench.run_until(|b| b.line() > 10 && b.sim.laser_lit(), 5_000));
    bench
}

fn spindle_stops(bench: &Bench, from: usize) -> usize {
    bench.sim.log[from..].iter().filter(|b| **b == 0x9E).count()
}

fn pushed_count(bench: &Bench, state: State) -> usize {
    bench
        .pushed
        .iter()
        .filter(|p| matches!(p.status, Message::Status { state: s, .. } if s == state))
        .count()
}

/// A job handed to the controller directly, past the job machine's own checks.
fn direct_start(bench: &mut Bench, text: &str) -> Vec<Event> {
    let job = Job {
        job_id: job_id(77),
        name: "direct.gcode".into(),
        kind: JobKind::Job,
        text: text.into(),
    };
    let command = ControllerCommand::Start { job_id: job.job_id };
    bench.ctrl.command(&command, Some(&job), bench.at)
}

#[test]
fn g34_laser_mode_unknown_refuses_every_job_and_the_controller_refuses_start_too() {
    let mut bench = Bench::new(SimConfig::default());
    bench.sim.settings.remove(&32);
    assert!(bench.run_until(|b| b.state() != State::NoController, 1_000));
    let text = "Laser mode unknown ($32 not read)";
    assert!(
        bench.notices.iter().any(|n| n == text),
        "{:?}",
        bench.notices
    );
    let lines = bench.sim.lines.len();
    let job = "G21\nM4 S0\nG1 X10 F600 S500\n";
    let out = bench.machine.handle(Event::Message(Message::SubmitJob {
        job_id: job_id(9),
        name: "unknown.gcode".into(),
        kind: JobKind::Job,
        bytes: job.len() as u64,
        sha256: "0".repeat(64),
    }));
    assert_eq!(
        out.messages,
        [Message::JobRejected {
            job_id: job_id(9),
            reason: RejectReason::LaserModeUnknown,
        }]
    );
    assert_eq!(bench.state(), State::Idle);
    let events = direct_start(&mut bench, job);
    assert_eq!(
        events,
        [Event::JobFailedWith {
            elapsed_ms: 0,
            detail: text.into()
        }]
    );
    bench.run_for(Duration::from_secs(1));
    assert_eq!(bench.sim.lines.len(), lines, "no byte of a job");
}

#[test]
fn g10_a_build_that_leaves_the_laser_on_in_a_hold_gets_the_spindle_stop_once() {
    let mut bench = running(SimConfig {
        laser_on_in_hold: true,
        ..slow()
    });
    let mark = bench.sim.log.len();
    bench.app(Message::Pause);
    assert!(bench.run_until(|b| b.state() == State::Paused, 500));
    assert!(bench.run_until(|b| !b.sim.laser_lit(), 100), "laser off");
    bench.run_for(Duration::from_secs(3));
    assert!(!bench.sim.laser_lit(), "stays off while held");
    assert_eq!(spindle_stops(&bench, mark), 1, "Toggle Spindle Stop once");
    bench.app(Message::Resume);
    assert!(bench.run_until(|b| b.state() == State::Running, 500));
    assert!(
        bench.run_until(|b| b.sim.laser_lit(), 500),
        "back as the job left it"
    );
    assert!(bench.run_until(|b| b.state() == COMPLETED, 50_000));
    assert!(bench.sim.faults.is_empty(), "{:?}", bench.sim.faults);
}

#[test]
fn g10_a_build_that_turns_the_laser_off_gets_no_toggle() {
    let mut bench = running(slow());
    let mark = bench.sim.log.len();
    bench.app(Message::Pause);
    assert!(bench.run_until(|b| b.state() == State::Paused, 500));
    bench.run_for(Duration::from_secs(3));
    assert!(!bench.sim.laser_lit());
    assert_eq!(
        spindle_stops(&bench, mark),
        0,
        "the toggle would turn it on"
    );
}

#[test]
fn g10_m3_constant_power_is_off_in_a_pause_with_either_build() {
    for on_in_hold in [false, true] {
        let mut bench = Bench::connected(SimConfig {
            laser_on_in_hold: on_in_hold,
            steps_per_motion: 200,
            decel_steps: 10,
            ..SimConfig::default()
        });
        bench.start(M3_JOB);
        assert!(bench.run_until(|b| b.sim.laser_lit(), 5_000));
        assert_eq!(bench.sim.spindle, Spindle::Cw, "M3");
        bench.app(Message::Pause);
        assert!(bench.run_until(|b| b.state() == State::Paused, 500));
        bench.run_for(Duration::from_secs(2));
        assert!(!bench.sim.laser_lit(), "laser off (build on: {on_in_hold})");
        bench.app(Message::Resume);
        assert!(bench.run_until(|b| b.state() == COMPLETED, 50_000));
        assert!(bench.sim.faults.is_empty(), "{:?}", bench.sim.faults);
        assert!(!bench.sim.laser_lit());
    }
}

#[test]
fn g12_a_controller_that_dies_at_the_stop_is_unconfirmed_within_the_loss_limit() {
    let mut bench = running(slow());
    bench.sim.silent_on_hold = true;
    bench.app(Message::Stop);
    assert!(bench.run_until(|b| b.state() != State::Running, 1_000));
    assert_eq!(bench.state(), State::NoController);
    assert_eq!(
        bench.detail().as_deref(),
        Some("Could not confirm the machine stopped")
    );
    let end = bench.pushed.last().unwrap();
    assert!(
        end.report_age.unwrap() < LOSS + 2 * DT,
        "ended {:?} after the last report",
        end.report_age
    );
    assert!(
        bench.stale_running_push(LOSS).is_none(),
        "{:?}",
        bench.stale_running_push(LOSS)
    );
    assert!(bench.last_write.ends_with(SAFETY));
    assert_eq!(pushed_count(&bench, STOPPED), 0);
}

#[test]
fn g12_a_banner_during_the_hold_finishes_the_stop_and_the_next_job_streams() {
    let mut bench = running(slow());
    let mark = bench.sim.log.len();
    bench.app(Message::Stop);
    // The machine's reset button once the head stands, before the connector read `Hold:0`:
    // no alarm, just the banner.
    assert!(bench.run_until(|b| b.sim.held(), 100));
    assert!(
        !bench.sim.log[mark..].contains(&0x18),
        "the stop still holds"
    );
    bench.sim.reset();
    assert!(!bench.sim.alarm);
    assert!(bench.run_until(|b| b.state() == STOPPED, 100));
    assert_eq!(pushed_count(&bench, STOPPED), 1);
    bench.run_for(Duration::from_secs(3));
    assert_eq!(bench.state(), STOPPED, "nothing else ends it again");
    let lines = bench.sim.lines.len();
    bench.start("G21\nG0 X1\n");
    assert!(
        bench.run_until(|b| b.sim.lines.len() > lines, 100),
        "it streams"
    );
    assert!(bench.run_until(|b| b.state() == COMPLETED, 1_000));
}

#[test]
fn g12_start_while_a_stop_is_pending_is_ignored() {
    let mut bench = running(slow());
    bench.app(Message::Stop);
    let lines = bench.sim.lines.len();
    assert!(direct_start(&mut bench, "G0 X5\n").is_empty());
    assert!(bench.run_until(|b| b.state() == STOPPED, 1_000));
    assert!(
        !bench.sim.lines[lines..].iter().any(|l| l == "G0X5"),
        "{:?}",
        &bench.sim.lines[lines..]
    );
}

#[test]
fn g28_a_foreign_ok_while_lines_are_in_flight_is_caught_by_the_buffer_field() {
    let mut bench = running(SimConfig {
        buffer_reports: true,
        ..slow()
    });
    assert!(bench.run_until(|b| b.sim.rx_used() > 40, 1_000));
    bench.sim.inject_ok();
    assert!(bench.run_until(|b| b.state() == FAILED, 500));
    assert_eq!(
        bench.detail().as_deref(),
        Some("Unexpected answer from the controller")
    );
    assert!(bench.sim.faults.is_empty(), "{:?}", bench.sim.faults);
    assert!(!bench.sim.laser_lit());
    assert_eq!(bench.sim.spindle, Spindle::Off);
}

#[test]
fn g28_the_buffer_field_never_fails_a_clean_job() {
    let mut varied = String::from("G21\nG90\nM4 S0\n");
    for i in 0..3_000 {
        let pad = "0".repeat(i % 40);
        varied += &format!("G1 X{}.{pad}1 Y{} S{}\n", i % 300, (i * 7) % 200, i % 1000);
    }
    for job in [long_job(), varied] {
        let mut bench = Bench::connected(SimConfig {
            buffer_reports: true,
            mpos_reports: true,
            ..SimConfig::default()
        });
        bench.start(&job);
        let done = bench.run_until(|b| b.state() == COMPLETED, 200_000);
        assert!(done, "{:?}", bench.detail());
        assert!(bench.sim.faults.is_empty(), "{:?}", bench.sim.faults);
    }
}

#[test]
fn g15_at_a_loss_only_the_safety_bytes_go_out() {
    let mut bench = Bench::connected(SimConfig {
        homing_lock: true,
        ..SimConfig::default()
    });
    assert_eq!(bench.state(), State::Alarm);
    bench.sim.silent = true;
    while bench.closed.is_none() {
        // A queued `$X` must not go out ahead of the reset.
        bench.app(Message::Unlock);
        bench.step();
    }
    assert_eq!(bench.last_write, SAFETY);
}

#[test]
fn a_controller_in_the_door_state_takes_no_job() {
    let mut bench = Bench::connected(SimConfig::default());
    let id = bench.send("G21\nG0 X1\n");
    bench.sim.door = true;
    assert!(bench.run_until(
        |b| b.detail().as_deref() == Some("Controller state: Door"),
        300
    ));
    let lines = bench.sim.lines.len();
    bench.app(Message::Start { job_id: id });
    assert_eq!(bench.state(), FAILED);
    assert_eq!(bench.detail().as_deref(), Some("Controller state: Door"));
    bench.run_for(Duration::from_secs(1));
    assert_eq!(bench.sim.lines.len(), lines, "no byte of the job");
    let text = "G0 X2\n";
    let out = bench.machine.handle(Event::Message(Message::SubmitJob {
        job_id: job_id(5),
        name: "door.gcode".into(),
        kind: JobKind::Job,
        bytes: text.len() as u64,
        sha256: "0".repeat(64),
    }));
    assert_eq!(
        out.messages,
        [Message::JobRejected {
            job_id: job_id(5),
            reason: RejectReason::Busy,
        }]
    );
}
