//! `specs/0118` first slice against the GRBL simulator (architect addendum M4, M7, M12): the Home
//! command writes `$H` once and nothing else, homes from Idle, from Ready and from the power-up
//! lock, survives a quiet homing, names a failed homing in words, and Stop ends it by the reset.
//! Time is stepped, never the wall clock.
#![allow(clippy::unwrap_used)]

mod common;

use curvyo_connector_core::Peer;
use std::time::Duration;

use common::grbl_bench::Bench;
use common::grbl_sim::{SimConfig, Spindle};
use curvyo_connector_core::{ErrorCode, Event, Message, State};
use curvyo_connector_io::controller::Controller;

const JOB: &str = "G21\nG90\nG0 X10 Y10\nM4 S500\nG1 X20 Y20 F1000\nM5\n";

/// GRBL 1.1f as the customer's laser: `$22=1`, locked at power-up, a homing loop that answers no
/// `?`; 9 s of homing, three times the loss limit.
fn grbl_1_1f(homing_fails: Option<u16>) -> SimConfig {
    SimConfig {
        version: "1.1f",
        homing_lock: true,
        homing_steps: 900,
        quiet_homing: true,
        homing_fails,
        ..SimConfig::default()
    }
}

/// Connected and unlocked with the Unlock of `0127` G33: `Idle`.
fn unlocked(config: SimConfig) -> Bench {
    let mut bench = Bench::connected(config);
    bench.app(Message::Unlock);
    assert!(bench.run_until(|b| b.state() == State::Idle, 500));
    bench
}

fn homes(bench: &Bench) -> usize {
    bench
        .sim
        .lines
        .iter()
        .filter(|l| l.as_str() == "$H")
        .count()
}

/// Applies `Home` and returns the error message the machine answered with, if any.
fn press_home(bench: &mut Bench) -> Option<String> {
    let out = bench
        .machine
        .handle(Event::Message(Message::Home, Peer::host()));
    let refusal = out.messages.iter().find_map(|m| match m {
        Message::Error {
            code: ErrorCode::WrongState,
            message,
        } => Some(message.clone()),
        _ => None,
    });
    for command in out.commands {
        let events = bench
            .ctrl
            .command(&command, bench.machine.held_job(), bench.at);
        for e in events {
            bench.apply(e);
        }
    }
    refusal
}

#[test]
fn m4_home_from_idle_writes_dollar_h_once_and_ends_idle() {
    let mut bench = unlocked(grbl_1_1f(None));
    let mark = bench.sim.lines.len();
    assert_eq!(press_home(&mut bench), None);
    assert_eq!(bench.state(), State::Homing);
    assert!(bench.run_until(|b| b.state() != State::Homing, 2_000));
    assert_eq!(bench.state(), State::Idle);
    assert_eq!(bench.sim.lines[mark..], ["$H"], "one line, nothing else");
    assert!(bench.closed.is_none());
    assert_eq!(bench.detail(), None);
    assert!(bench.sim.faults.is_empty(), "{:?}", bench.sim.faults);
}

#[test]
fn m4_a_quiet_nine_second_homing_is_no_loss() {
    let mut bench = unlocked(grbl_1_1f(None));
    press_home(&mut bench);
    bench.run_for(Duration::from_secs(5));
    assert_eq!(bench.state(), State::Homing, "still homing, the link kept");
    assert!(bench.closed.is_none());
    assert!(bench.run_until(|b| b.state() == State::Idle, 600));
    assert!(bench.closed.is_none());
    // The loss rule holds again once homing ended.
    bench.sim.silent = true;
    let silent_at = bench.at;
    assert!(bench.run_until(|b| b.state() == State::NoController, 400));
    assert!(bench.at.saturating_sub(silent_at) <= Duration::from_millis(3_100));
}

#[test]
fn m4_home_from_the_power_up_lock_needs_no_unlock() {
    let mut bench = Bench::connected(grbl_1_1f(None));
    assert_eq!(bench.state(), State::Alarm);
    let mark = bench.sim.lines.len();
    assert_eq!(press_home(&mut bench), None);
    assert_eq!(bench.alarm(), None, "no alarm text while homing");
    assert!(bench.run_until(|b| b.state() != State::Homing, 2_000));
    assert_eq!(bench.state(), State::Idle);
    assert_eq!(bench.alarm(), None);
    assert_eq!(bench.sim.lines[mark..], ["$H"], "no $X before or after");
    assert!(!bench.sim.alarm);
    // GRBL reports Idle from now on: no alarm comes back.
    bench.run_for(Duration::from_secs(1));
    assert_eq!(bench.state(), State::Idle);
}

#[test]
fn m3_a_ready_job_is_kept_through_a_home_and_starts_after_it() {
    let mut bench = unlocked(grbl_1_1f(None));
    let id = bench.send(JOB);
    press_home(&mut bench);
    let Message::Status { job, .. } = bench.status() else {
        panic!()
    };
    assert_eq!(job.map(|j| j.job_id), Some(id));
    assert!(bench.run_until(|b| b.state() != State::Homing, 2_000));
    assert_eq!(bench.state(), State::Ready);
    assert_eq!(bench.machine.held_job().map(|j| j.job_id), Some(id));
    bench.app(Message::Start { job_id: id });
    assert!(bench.run_until(|b| b.state() != State::Running, 3_000));
    assert!(
        matches!(bench.state(), State::Done { .. }),
        "{:?}",
        bench.detail()
    );
}

#[test]
fn m4_homing_not_enabled_names_error_5_and_keeps_the_state() {
    let mut config = grbl_1_1f(None);
    config.homing_lock = false;
    let mut bench = Bench::connected(config);
    assert_eq!(bench.state(), State::Idle);
    let id = bench.send(JOB);
    press_home(&mut bench);
    assert!(bench.run_until(|b| b.state() != State::Homing, 500));
    assert_eq!(bench.state(), State::Ready, "error 5 keeps the Ready job");
    assert_eq!(bench.machine.held_job().map(|j| j.job_id), Some(id));
    assert_eq!(
        bench.detail().as_deref(),
        Some("Homing is not enabled (error 5)")
    );
    assert!(bench.closed.is_none());
}

#[test]
fn m4_a_failed_homing_is_the_alarm_in_words_with_the_laser_off() {
    for (code, text) in [
        (9, "Homing did not find the switch (alarm 9)"),
        (8, "Homing pull-off did not clear the switch (alarm 8)"),
    ] {
        for from_lock in [false, true] {
            let mut bench = if from_lock {
                Bench::connected(grbl_1_1f(Some(code)))
            } else {
                unlocked(grbl_1_1f(Some(code)))
            };
            if !from_lock {
                bench.send(JOB);
            }
            press_home(&mut bench);
            assert!(bench.run_until(|b| b.state() != State::Homing, 2_000));
            assert_eq!(bench.state(), State::Alarm, "{text}");
            assert_eq!(
                bench.alarm().as_deref(),
                Some(text),
                "from lock {from_lock}"
            );
            assert!(bench.machine.held_job().is_none(), "an alarm drops the job");
            // The trailing `ok` and the restart do not end the alarm.
            bench.run_for(Duration::from_secs(1));
            assert_eq!(bench.state(), State::Alarm);
            assert_eq!(bench.alarm().as_deref(), Some(text));
            assert!(bench.closed.is_none());
            assert!(!bench.sim.laser_lit());
            assert_eq!(bench.sim.spindle, Spindle::Off);
            assert!(!bench.sim.air);
        }
    }
}

#[test]
fn m4_stop_during_home_resets_without_hold_and_alarm_6_follows() {
    let mut bench = unlocked(grbl_1_1f(None));
    bench.send(JOB);
    press_home(&mut bench);
    bench.run_for(Duration::from_secs(2));
    let mark = bench.sim.log.len();
    bench.app(Message::Stop);
    assert_eq!(bench.detail().as_deref(), Some("Stopping"));
    assert!(bench.run_until(|b| b.state() == State::Alarm, 500));
    assert_eq!(
        bench.alarm().as_deref(),
        Some("Reset during homing (alarm 6)")
    );
    assert!(
        bench.machine.held_job().is_none(),
        "on GRBL the alarm drops the job"
    );
    let sent = &bench.sim.log[mark..];
    assert!(sent.contains(&0x18), "the reset");
    assert!(!sent.contains(&b'!'), "no feed hold while homing (G9)");
    let after_reset = bench.lines_after(mark);
    assert!(after_reset.contains("M5\nM9\n"), "{after_reset:?}");
    assert!(bench.closed.is_none(), "the stop was confirmed");
    assert!(
        !bench.pushed.iter().any(|p| matches!(
            p.status,
            Message::Status {
                state: State::Done { .. },
                ..
            }
        )),
        "never Done: there was no job"
    );
}

#[test]
fn m3_home_while_a_job_runs_or_its_stop_is_pending_writes_nothing() {
    let mut bench = unlocked(grbl_1_1f(None));
    bench.start(&JOB.repeat(40));
    bench.run_for(Duration::from_millis(200));
    assert_eq!(bench.state(), State::Running);
    let before = homes(&bench);
    assert_eq!(press_home(&mut bench).as_deref(), Some("A job is running."));
    bench.app(Message::Stop);
    assert_eq!(press_home(&mut bench).as_deref(), Some("A job is running."));
    assert!(bench.run_until(|b| b.state() != State::Running, 500));
    bench.run_for(Duration::from_millis(500));
    assert_eq!(homes(&bench), before, "no $H reached GRBL");
}

#[test]
fn m7_the_simulator_refuses_dollar_h_while_running() {
    let mut bench = unlocked(grbl_1_1f(None));
    bench.start(&JOB.repeat(40));
    bench.run_for(Duration::from_millis(200));
    // Past the job machine's guard, straight to the port: GRBL itself says not idle.
    bench.sim.receive(b"$H\n");
    let mut answer = Vec::new();
    for _ in 0..50 {
        answer.extend(bench.sim.step());
    }
    assert!(String::from_utf8_lossy(&answer).contains("error:8"));
}

#[test]
fn g14_connect_an_alarm_and_a_reconnect_never_write_dollar_h() {
    let mut bench = Bench::connected(grbl_1_1f(None));
    bench.run_for(Duration::from_secs(2));
    bench.app(Message::Unlock);
    bench.run_for(Duration::from_secs(1));
    bench.sim.inject_alarm(1);
    bench.run_for(Duration::from_secs(2));
    assert_eq!(bench.state(), State::Alarm);
    bench.plugged = false;
    assert!(bench.run_until(|b| b.state() == State::NoController, 500));
    assert_eq!(homes(&bench), 0);
}
