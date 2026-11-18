//! `specs/0127` pause, resume, stop, alarm, loss and the other safety rules against the GRBL
//! simulator over an in-memory channel with stepped time (G10 to G18, G28, G29, Frage 239 B and
//! 240 B). Laser off is checked on the simulator after every stop, alarm and loss.
#![allow(
    clippy::unwrap_used,
    clippy::format_push_string,
    clippy::naive_bytecount
)]

mod common;

use std::time::Duration;

use common::grbl_bench::Bench;
use common::grbl_sim::{SimConfig, Spindle};
use curvyo_connector_core::{JobResult, Message, Options, State};
use curvyo_connector_io::grbl::controller::SAFETY;

const STOPPED: State = State::Done {
    result: JobResult::Stopped,
};
const FAILED: State = State::Done {
    result: JobResult::Failed,
};
const LOSS: Duration = Duration::from_secs(3);

/// A long job of real motions with the laser on.
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

fn assert_laser_off(bench: &Bench) {
    assert!(!bench.sim.laser_lit());
    assert_eq!(bench.sim.spindle, Spindle::Off, "spindle off");
    assert!(!bench.sim.air, "air off");
}

fn pushed_count(bench: &Bench, state: State) -> usize {
    bench
        .pushed
        .iter()
        .filter(|p| matches!(p.status, Message::Status { state: s, .. } if s == state))
        .count()
}

#[test]
fn g10_paused_only_at_hold_zero_with_the_laser_off_and_g11_resume_goes_on() {
    let mut bench = running(slow());
    let sent = bench.sim.lines.len();
    bench.app(Message::Pause);
    bench.run_for(Duration::from_millis(100));
    assert_eq!(bench.state(), State::Running, "Hold:1 still reads Running");
    assert!(bench.run_until(|b| b.state() == State::Paused, 500));
    let held_line = bench.line();
    let received = bench.sim.lines.len();
    bench.run_for(Duration::from_secs(3));
    assert!(!bench.sim.laser_lit(), "laser off while held");
    assert_eq!(
        bench.sim.lines.len(),
        received,
        "no line goes out while held"
    );
    assert!(received - sent < 20, "only what was in flight");
    assert!(bench.line() >= held_line);
    let count = bench.line();
    bench.run_for(Duration::from_secs(1));
    assert_eq!(
        bench.line(),
        count,
        "the count stops once the buffer is answered"
    );
    bench.app(Message::Pause);
    assert_eq!(
        bench.state(),
        State::Paused,
        "Pause while Paused is idempotent"
    );
    bench.app(Message::Resume);
    assert!(bench.run_until(|b| b.state() == State::Running, 500));
    assert!(
        bench.run_until(|b| b.line() > count, 500),
        "continues from the same line"
    );
    assert!(
        bench.run_until(|b| b.sim.laser_lit(), 500),
        "the job's own laser state"
    );
    let done = State::Done {
        result: JobResult::Completed,
    };
    assert!(bench.run_until(|b| b.state() == done, 50_000));
    assert!(bench.sim.faults.is_empty(), "{:?}", bench.sim.faults);
}

#[test]
fn g12_stop_from_running_holds_resets_and_ends_once_after_the_banner() {
    let mut bench = running(slow());
    let mark = bench.sim.log.len();
    bench.app(Message::Stop);
    assert_eq!(
        bench.detail().as_deref(),
        Some("Stopping"),
        "G30: the answer at once"
    );
    assert_eq!(bench.state(), State::Running);
    assert!(bench.run_until(|b| b.state() == STOPPED, 1_000));
    let after = &bench.sim.log[mark..];
    let hold = after.iter().position(|b| *b == b'!').unwrap();
    let reset = after.iter().position(|b| *b == 0x18).unwrap();
    assert!(hold < reset, "hold before reset");
    assert!(bench.lines_after(mark).ends_with("M5\nM9\n"));
    assert!(!bench.sim.alarm, "a reset after Hold:0 keeps the position");
    assert_laser_off(&bench);
    assert_eq!(pushed_count(&bench, STOPPED), 1);
    bench.run_for(Duration::from_secs(2));
    assert_eq!(bench.state(), STOPPED);
    assert!(bench.closed.is_none());
    // The next job needs no Home.
    bench.start("G0 X1\n");
    let done = State::Done {
        result: JobResult::Completed,
    };
    assert!(bench.run_until(|b| b.state() == done, 1_000));
}

#[test]
fn g12_stop_from_paused_starts_at_the_reset() {
    let mut bench = running(slow());
    bench.app(Message::Pause);
    assert!(bench.run_until(|b| b.state() == State::Paused, 500));
    let mark = bench.sim.log.len();
    bench.app(Message::Stop);
    assert!(bench.run_until(|b| b.state() == STOPPED, 500));
    assert!(!bench.sim.log[mark..].contains(&b'!'));
    assert!(bench.sim.log[mark..].contains(&0x18));
    assert!(!bench.sim.alarm);
    assert_laser_off(&bench);
}

#[test]
fn g12_stop_during_the_last_line() {
    let mut bench = Bench::connected(SimConfig {
        steps_per_motion: 300,
        ..slow()
    });
    bench.start("G21\nM4 S0\nG1 X100 F600 S500\n");
    assert!(bench.run_until(|b| b.line() == 3 && b.sim.laser_lit(), 500));
    bench.app(Message::Stop);
    assert!(bench.run_until(|b| b.state() == STOPPED, 1_000));
    assert_laser_off(&bench);
}

#[test]
fn g12_without_a_banner_the_stop_is_not_confirmed() {
    let mut bench = running(slow());
    bench.app(Message::Stop);
    assert!(bench.run_until(|b| b.sim.log.last() == Some(&0x18), 500));
    bench.sim.silent = true;
    assert!(bench.run_until(|b| b.state() != State::Running, 1_000));
    assert_eq!(bench.state(), State::NoController);
    assert_eq!(
        bench.detail().as_deref(),
        Some("Could not confirm the machine stopped")
    );
    assert_eq!(pushed_count(&bench, STOPPED), 0, "never a clean Stopped");
    assert!(
        bench
            .closed
            .as_deref()
            .unwrap()
            .starts_with("Could not confirm")
    );
    assert!(bench.last_write.ends_with(SAFETY));
}

#[test]
fn g13_an_alarm_during_a_job_resets_discards_and_shows_the_first_text() {
    for paused in [false, true] {
        let mut bench = running(slow());
        if paused {
            bench.app(Message::Pause);
            assert!(bench.run_until(|b| b.state() == State::Paused, 500));
        }
        let mark = bench.sim.log.len();
        bench.sim.inject_alarm(1);
        assert!(bench.run_until(|b| b.state() == State::Alarm, 100));
        assert_eq!(
            bench.alarm().as_deref(),
            Some("Hard limit hit, position may be lost (alarm 1)")
        );
        bench.run_for(Duration::from_secs(2));
        assert!(
            bench.sim.log[mark..].contains(&0x18),
            "the reset after the alarm"
        );
        assert_eq!(bench.lines_after(mark), "", "no line after the alarm");
        assert_eq!(
            bench.alarm().as_deref(),
            Some("Hard limit hit, position may be lost (alarm 1)"),
            "stands through GRBL's lock message"
        );
        assert!(bench.machine.held_job().is_none());
        assert_laser_off(&bench);
    }
}

#[test]
fn g13_an_alarm_while_idle() {
    let mut bench = Bench::connected(SimConfig::default());
    bench.sim.inject_alarm(2);
    assert!(bench.run_until(|b| b.state() == State::Alarm, 100));
    assert_eq!(
        bench.alarm().as_deref(),
        Some("Target beyond the soft limits (alarm 2)")
    );
}

#[test]
fn g14_g17_locked_at_power_up_and_the_app_unlocks() {
    let mut bench = Bench::connected(SimConfig {
        homing_lock: true,
        ..SimConfig::default()
    });
    assert_eq!(bench.state(), State::Alarm);
    assert_eq!(bench.alarm().as_deref(), Some("Locked, not homed"));
    assert!(
        !bench.sim.lines.iter().any(|l| l == "$X" || l == "$H"),
        "never on its own"
    );
    bench.app(Message::Unlock);
    assert!(bench.run_until(|b| b.state() == State::Idle, 300));
    assert_eq!(bench.sim.lines.last().map(String::as_str), Some("$X"));
    assert!(!bench.sim.alarm);
    assert!(bench.notices.iter().any(|n| n == "Alarm cleared"));
}

#[test]
fn g15_a_pulled_cable_stops_at_once_and_never_shows_running_after_three_seconds() {
    for (paused, silent) in [(false, false), (true, false), (false, true)] {
        let mut bench = running(slow());
        if paused {
            bench.app(Message::Pause);
            assert!(bench.run_until(|b| b.state() == State::Paused, 500));
        }
        let line = bench.line();
        if silent {
            bench.sim.silent = true; // stops answering while staying open
        } else {
            bench.plugged = false;
        }
        let mark = bench.sim.log.len();
        assert!(bench.run_until(|b| b.state() == State::NoController, 1_000));
        assert_eq!(
            bench.detail(),
            Some(format!("Lost the controller at line {line}"))
        );
        assert!(
            bench.stale_running_push(LOSS).is_none(),
            "{:?}",
            bench.stale_running_push(LOSS)
        );
        assert!(bench.last_write.ends_with(SAFETY), "reset, M5, M9 tried");
        if silent {
            assert!(bench.sim.log.ends_with(SAFETY));
            assert_eq!(
                bench.lines_after(mark),
                "M5\nM9\n",
                "nothing but the safety bytes"
            );
        }
        assert!(bench.machine.held_job().is_none(), "dropped, never resumed");
    }
}

#[test]
fn g18_no_job_byte_before_the_local_start() {
    let mut bench = Bench::with(
        SimConfig::default(),
        Options {
            require_local_start: true,
        },
    );
    assert!(bench.run_until(|b| b.state() == State::Idle, 1_000));
    let lines = bench.sim.lines.len();
    let id = bench.send("G21\nG0 X1\n");
    bench.app(Message::Start { job_id: id });
    assert_eq!(bench.state(), State::AwaitingLocalStart);
    bench.run_for(Duration::from_secs(5));
    assert_eq!(bench.sim.lines.len(), lines, "only ? went out");
    bench.apply(curvyo_connector_core::Event::LocalStartPressed);
    assert!(bench.run_until(|b| b.sim.lines.len() > lines, 100));
}

#[test]
fn g28_a_foreign_ok_stops_and_fails_the_job() {
    // Every line is answered and the head still moves: no line is in flight.
    let mut bench = Bench::connected(SimConfig {
        steps_per_motion: 300,
        ..slow()
    });
    bench.start("G21\nM4 S0\nG1 X100 F600 S500\n");
    assert!(bench.run_until(|b| b.line() == 3 && b.sim.laser_lit(), 500));
    bench.sim.inject_ok();
    assert!(bench.run_until(|b| b.state() == FAILED, 500));
    assert_eq!(
        bench.detail().as_deref(),
        Some("Unexpected answer from the controller")
    );
    assert!(bench.sim.faults.is_empty(), "{:?}", bench.sim.faults);
    assert_laser_off(&bench);
}

#[test]
fn g7_an_error_stops_like_g12_and_names_the_rejected_line() {
    // `$I` and `$$` are the simulator's lines 1 and 2; job line 9 is received line 11.
    let mut bench = Bench::connected(SimConfig {
        error_on_line: Some((11, 22)),
        ..slow()
    });
    bench.start(&long_job());
    assert!(bench.run_until(|b| b.state() == FAILED, 2_000));
    assert_eq!(
        bench.detail().as_deref(),
        Some("Failed at line 9: No feed rate (error 22)")
    );
    assert!(
        bench.sim.lines.len() < 40,
        "only lines that were in GRBL's buffer"
    );
    assert!(bench.lines_after(0).ends_with("M5\nM9\n"));
    assert_laser_off(&bench);
    let mark = bench.sim.log.len();
    bench.run_for(Duration::from_secs(1));
    assert_eq!(bench.lines_after(mark), "", "nothing more is sent");
}

#[test]
fn g29_the_machine_reset_button_fails_the_job_and_nothing_resumes() {
    // Paused: no motion, so no alarm.
    let mut bench = running(slow());
    bench.app(Message::Pause);
    assert!(bench.run_until(|b| b.state() == State::Paused, 500));
    let line = bench.line();
    bench.sim.reset();
    assert!(bench.run_until(|b| b.state() == FAILED, 100));
    assert_eq!(
        bench.detail(),
        Some(format!("The controller was reset at line {line}"))
    );
    // Running: GRBL raises alarm 3 and the job fails before the alarm shows.
    let mut bench = running(slow());
    bench.sim.reset();
    assert!(bench.run_until(|b| b.state() == State::Alarm, 100));
    assert_eq!(pushed_count(&bench, FAILED), 1);
    assert_eq!(
        bench.alarm().as_deref(),
        Some("Reset while moving, position may be lost (alarm 3)")
    );
    assert_laser_off(&bench);
}

#[test]
fn g29_a_pause_grbl_does_not_confirm_is_dropped() {
    let job = include_str!("../../curvyo-laser-core/tests/fixtures/output/grbl/home_first.gcode");
    let mut bench = Bench::connected(SimConfig {
        homing_steps: 600,
        ..SimConfig::default()
    });
    bench.sim.set_setting(22, "1");
    bench.start(job);
    bench.run_for(Duration::from_millis(500));
    bench.app(Message::Pause);
    bench.run_for(Duration::from_secs(3));
    assert_eq!(bench.state(), State::Running);
    let done = State::Done {
        result: JobResult::Completed,
    };
    assert!(
        bench.run_until(|b| b.state() == done, 5_000),
        "the job goes on"
    );
}

#[test]
fn g34_no_job_is_taken_with_laser_mode_off() {
    let mut bench = Bench::connected(SimConfig {
        laser_mode: false,
        ..SimConfig::default()
    });
    assert!(
        bench
            .notices
            .iter()
            .any(|n| n == "Laser mode is off ($32=0)")
    );
    let lines = bench.sim.lines.len();
    let text = "G21\nM3 S500\nG1 X10 F600\n";
    bench.app(Message::SubmitJob {
        job_id: common::grbl_bench::job_id(99),
        name: "m3.gcode".into(),
        kind: curvyo_connector_core::JobKind::Job,
        bytes: text.len() as u64,
        sha256: "0".repeat(64),
    });
    assert_eq!(bench.state(), State::Idle, "refused, nothing held");
    assert!(bench.machine.held_job().is_none());
    bench.run_for(Duration::from_secs(1));
    assert_eq!(bench.sim.lines.len(), lines, "no byte of a job");
}
