//! `specs/0133` against the GRBL simulator set to GRBL 1.1f's silent homing: a Pause pressed while
//! a job's own `$H` runs is kept pending and applied when homing ends, before any further line.
#![allow(clippy::unwrap_used)]

mod common;

use std::fmt::Write as _;
use std::time::Duration;

use common::grbl_bench::Bench;
use common::grbl_sim::{SimConfig, Spindle};
use curvyo_connector_core::{Event, JobResult, Message, State};
use curvyo_connector_io::grbl::prepare::prepare;

const HOME_FIRST: &str =
    include_str!("../../curvyo-laser-core/tests/fixtures/output/grbl/home_first.gcode");

/// The line number of `$H` in `HOME_FIRST` (three comment lines first).
const HOME_LINE: u32 = 4;

/// GRBL 1.1f's silent homing for 9 s: far longer than the 2 s hold limit of `0127` G29.
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

/// Connected, unlocked, `Idle`.
fn unlocked(config: SimConfig) -> Bench {
    let mut bench = Bench::connected(config);
    bench.app(Message::Unlock);
    assert!(bench.run_until(|b| b.state() == State::Idle, 500));
    bench
}

/// The job started and its `$H` in flight for a second; the sim's byte log mark before the `$H`.
fn homing() -> (Bench, usize) {
    let mut bench = unlocked(grbl_1_1f(None));
    bench.start(HOME_FIRST);
    let mark = bench.sim.log.len();
    bench.run_for(Duration::from_secs(1));
    assert_eq!(bench.sim.lines.last().map(String::as_str), Some("$H"));
    assert_eq!(bench.state(), State::Running);
    (bench, mark)
}

fn pending(bench: &Bench) -> bool {
    matches!(
        bench.status(),
        Message::Status {
            pause_pending: true,
            ..
        }
    )
}

/// Whether the simulator received the feed hold `!` after byte `from`.
fn hold_sent_after(bench: &Bench, from: usize) -> bool {
    bench.sim.log[from..].contains(&b'!')
}

#[test]
fn ph1_pause_during_the_jobs_homing_is_pending_and_writes_nothing() {
    let (mut bench, mark) = homing();
    let before = bench.line();
    let pushed = bench.pushed.len();
    bench.app(Message::Pause);
    assert!(pending(&bench), "{:?}", bench.status());
    assert_eq!(bench.pushed.len(), pushed + 1, "the answer is one Status");
    assert_eq!(bench.state(), State::Running);
    assert_eq!(bench.detail().as_deref(), Some("Controller state: Home"));
    assert_eq!(bench.line(), before, "line count unchanged");
    bench.run_for(Duration::from_secs(5));
    assert!(pending(&bench), "kept past the 2 s hold limit (PH7)");
    assert!(!hold_sent_after(&bench, mark), "no `!` while `$H` runs");
}

#[test]
fn ph2_pause_during_homing_holds_when_homing_ends_before_any_further_line() {
    let (mut bench, _) = homing();
    let mark = bench.sim.lines.len();
    bench.app(Message::Pause);
    assert!(
        bench.run_until(|b| b.state() != State::Running, 2_000),
        "never left Running"
    );
    assert_eq!(bench.state(), State::Paused, "detail {:?}", bench.detail());
    assert!(!pending(&bench));
    assert_eq!(bench.line(), HOME_LINE, "lines acknowledged: up to `$H`");
    assert_eq!(bench.sim.lines.last().map(String::as_str), Some("$H"));
    assert_eq!(bench.sim.lines.len(), mark, "no line after `$H`");
    assert!(bench.sim.held(), "GRBL holds");
    bench.run_for(Duration::from_secs(5));
    assert_eq!(bench.state(), State::Paused, "stays paused");
    assert_eq!(bench.sim.lines.len(), mark, "still no line after `$H`");
    assert_eq!(bench.sim.spindle, Spindle::Off);
    assert!(bench.sim.power <= 0.0);
    assert!(!bench.sim.laser_lit());
    assert!(bench.sim.faults.is_empty(), "{:?}", bench.sim.faults);

    // Resume continues with the next line and runs the job to the end.
    let from = bench.sim.log.len();
    bench.app(Message::Resume);
    bench.step();
    assert!(bench.sim.log[from..].contains(&b'~'));
    assert!(bench.run_until(|b| matches!(b.state(), State::Done { .. }), 3_000));
    assert_eq!(
        bench.state(),
        State::Done {
            result: JobResult::Completed
        }
    );
    let sent: Vec<String> = prepare(HOME_FIRST)
        .unwrap()
        .into_iter()
        .filter(|l| !l.is_empty())
        .collect();
    assert_eq!(bench.sim.lines[mark - 1..], sent, "every line, once");
}

#[test]
fn ph4_resume_while_pending_cancels_it_and_the_job_runs_on() {
    let (mut bench, mark) = homing();
    bench.app(Message::Pause);
    bench.app(Message::Pause);
    assert!(pending(&bench), "a second Pause is idempotent");
    let pushed = bench.pushed.len();
    bench.app(Message::Pause);
    assert_eq!(bench.pushed.len(), pushed, "nothing for a second Pause");
    bench.app(Message::Resume);
    assert!(!pending(&bench));
    assert_eq!(bench.state(), State::Running);
    assert!(bench.run_until(|b| matches!(b.state(), State::Done { .. }), 3_000));
    assert_eq!(
        bench.state(),
        State::Done {
            result: JobResult::Completed
        }
    );
    assert!(!hold_sent_after(&bench, mark), "no `!`");
    assert!(!bench.sim.log[mark..].contains(&b'~'), "no `~`");
    assert!(!bench.pushed.iter().any(|p| matches!(
        p.status,
        Message::Status {
            state: State::Paused,
            ..
        }
    )));
}

#[test]
fn ph5_stop_while_pending_resets_without_a_feed_hold() {
    let (mut bench, mark) = homing();
    bench.app(Message::Pause);
    bench.app(Message::Stop);
    assert!(!pending(&bench), "Stop clears the pending pause");
    assert_eq!(bench.detail().as_deref(), Some("Stopping"));
    assert!(bench.run_until(|b| b.state() != State::Running, 500));
    // GRBL raises alarm 6 for a reset while homing (`0129`), after the stop's `Stopped`.
    assert_eq!(bench.state(), State::Alarm);
    assert_eq!(
        bench.alarm().as_deref(),
        Some("Reset during homing (alarm 6)")
    );
    assert!(bench.pushed.iter().any(|p| matches!(
        p.status,
        Message::Status {
            state: State::Done {
                result: JobResult::Stopped
            },
            ..
        }
    )));
    assert!(!hold_sent_after(&bench, mark), "no `!`");
    assert!(bench.sim.log[mark..].contains(&0x18), "the reset");
    assert!(!bench.sim.laser_lit());
}

/// Never `Paused` after the pending pause, and the pending flag is gone.
fn never_paused(bench: &Bench, from: usize) -> bool {
    !bench.pushed[from..].iter().any(|p| {
        matches!(
            p.status,
            Message::Status {
                state: State::Paused,
                ..
            }
        )
    }) && !pending(bench)
}

#[test]
fn ph6_homing_alarms_drop_the_pending_pause() {
    for (code, text) in [
        (9, "Homing did not find the switch (alarm 9)"),
        (8, "Homing pull-off did not clear the switch (alarm 8)"),
    ] {
        let mut bench = unlocked(grbl_1_1f(Some(code)));
        bench.start(HOME_FIRST);
        bench.run_for(Duration::from_secs(1));
        let pushed = bench.pushed.len();
        bench.app(Message::Pause);
        assert!(pending(&bench));
        assert!(
            bench.run_until(|b| b.state() == State::Alarm, 3_000),
            "{text}"
        );
        assert_eq!(bench.alarm().as_deref(), Some(text));
        bench.run_for(Duration::from_secs(1));
        assert_eq!(bench.state(), State::Alarm, "{text}: stays");
        assert!(never_paused(&bench, pushed), "{text}");
        assert!(!bench.sim.laser_lit());
    }
}

#[test]
fn ph6_homing_refused_with_error_5_fails_the_job() {
    let mut config = grbl_1_1f(None);
    config.homing_lock = false;
    let mut bench = Bench::connected(config);
    bench.start(HOME_FIRST);
    // `$22=0`: GRBL answers `error:5` at once, so the pause comes before the answer is read.
    bench.app(Message::Pause);
    let pushed = bench.pushed.len();
    let failed = State::Done {
        result: JobResult::Failed,
    };
    assert!(bench.run_until(|b| b.state() == failed, 1_000));
    assert_eq!(
        bench.detail().as_deref(),
        Some("Failed at line 4: Homing is not enabled (error 5)")
    );
    assert!(never_paused(&bench, pushed));
    assert_eq!(bench.sim.lines.last().map(String::as_str), Some("$H"));
}

#[test]
fn ph6_a_reset_during_homing_drops_the_pending_pause() {
    let (mut bench, _) = homing();
    bench.app(Message::Pause);
    let pushed = bench.pushed.len();
    bench.sim.reset();
    assert!(bench.run_until(|b| b.state() != State::Running, 500));
    assert!(never_paused(&bench, pushed));
    assert!(!bench.sim.laser_lit());
}

#[test]
fn ph6_ph9_a_lost_controller_drops_the_pending_pause() {
    let (mut bench, _) = homing();
    bench.app(Message::Pause);
    let pushed = bench.pushed.len();
    // The serial link reports the port error as a lost controller (`0127` G15).
    bench.plugged = false;
    bench.apply(Event::ControllerLost);
    assert_eq!(bench.state(), State::NoController);
    assert!(never_paused(&bench, pushed));
}

#[test]
fn ph7_a_pause_in_a_normal_run_is_unchanged() {
    let mut bench = Bench::connected(SimConfig::default());
    let mut job = String::from("G21\nG90\nM4 S0\n");
    for i in 0..200 {
        writeln!(job, "G1 X{} Y{} F600 S500", i % 50, i % 30).unwrap();
    }
    job += "M5\n";
    bench.start(&job);
    assert!(bench.run_until(|b| b.line() > 20, 2_000));
    let from = bench.sim.log.len();
    bench.app(Message::Pause);
    assert!(!pending(&bench), "no pending pause outside homing");
    bench.step();
    assert!(bench.sim.log[from..].contains(&b'!'), "`!` at once");
    assert!(bench.run_until(|b| b.state() == State::Paused, 500));
    assert!(!pending(&bench));
    assert!(!bench.sim.laser_lit());
}
