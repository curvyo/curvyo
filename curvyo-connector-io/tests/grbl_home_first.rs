//! `specs/0129` against the GRBL simulator set to GRBL 1.1f's homing: a job that starts with
//! `$H` homes and runs to the end, and a homing that fails names its cause, never "Locked".
#![allow(clippy::unwrap_used)]

mod common;

use std::time::Duration;

use common::grbl_bench::Bench;
use common::grbl_sim::{SimConfig, Spindle};
use curvyo_connector_core::{JobResult, Message, State};
use curvyo_connector_io::grbl::prepare::prepare;

const HOME_FIRST: &str =
    include_str!("../../curvyo-laser-core/tests/fixtures/output/grbl/home_first.gcode");

/// The customer's laser: `$22=1`, locked at power-up, and GRBL 1.1f's homing loop, which
/// answers no `?` until the cycle ends; 9 s of homing, three times the loss limit.
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

/// Connected, unlocked with the Unlock of `0127` G33, `Idle`.
fn unlocked(config: SimConfig) -> Bench {
    let mut bench = Bench::connected(config);
    assert_eq!(bench.state(), State::Alarm);
    bench.app(Message::Unlock);
    assert!(bench.run_until(|b| b.state() == State::Idle, 500));
    bench
}

/// Whether a status pushed from `from` on names "Locked" (the power-up alarm before the
/// Unlock does, by `0127` G14).
fn says_locked(bench: &Bench, from: usize) -> bool {
    bench.pushed[from..].iter().any(|p| {
        let Message::Status { detail, alarm, .. } = &p.status else {
            return false;
        };
        [detail, alarm]
            .iter()
            .any(|t| t.as_deref().is_some_and(|t| t.contains("Locked")))
    })
}

#[test]
fn c1_home_first_homes_and_runs_to_the_end_on_grbl_1_1f() {
    let mut bench = unlocked(grbl_1_1f(None));
    let mark = bench.sim.lines.len();
    let pushed = bench.pushed.len();
    bench.start(HOME_FIRST);
    assert!(bench.run_until(|b| b.state() != State::Running, 3_000));
    assert_eq!(
        bench.state(),
        State::Done {
            result: JobResult::Completed
        },
        "detail {:?}, closed {:?}",
        bench.detail(),
        bench.closed
    );
    assert!(bench.closed.is_none(), "the link was never given up");
    let sent: Vec<String> = prepare(HOME_FIRST)
        .unwrap()
        .into_iter()
        .filter(|l| !l.is_empty())
        .collect();
    assert_eq!(
        bench.sim.lines[mark..],
        sent,
        "every line after $H reached GRBL"
    );
    assert!(!bench.sim.alarm);
    assert!(!says_locked(&bench, pushed), "no Locked text");
    assert!(bench.sim.faults.is_empty(), "{:?}", bench.sim.faults);
}

#[test]
fn c1_the_status_says_home_while_grbl_homes_without_answering() {
    let mut bench = unlocked(grbl_1_1f(None));
    bench.start(HOME_FIRST);
    bench.run_for(Duration::from_secs(5));
    assert_eq!(bench.state(), State::Running);
    assert_eq!(bench.detail().as_deref(), Some("Controller state: Home"));
    assert_eq!(bench.sim.lines.last().map(String::as_str), Some("$H"));
}

#[test]
fn c2_a_failed_homing_names_its_cause_and_leaves_the_laser_off() {
    let table = [
        (
            Some(9),
            true,
            State::Alarm,
            "Homing did not find the switch (alarm 9)",
        ),
        (
            Some(8),
            true,
            State::Alarm,
            "Homing pull-off did not clear the switch (alarm 8)",
        ),
        (
            None,
            false,
            State::Done {
                result: JobResult::Failed,
            },
            "Failed at line 4: Homing is not enabled (error 5)",
        ),
    ];
    for (fails, homing_on, end, text) in table {
        let mut bench = if homing_on {
            unlocked(grbl_1_1f(fails))
        } else {
            let mut config = grbl_1_1f(None);
            config.homing_lock = false;
            Bench::connected(config)
        };
        let mark = bench.sim.lines.len();
        let pushed = bench.pushed.len();
        bench.start(HOME_FIRST);
        assert!(bench.run_until(|b| b.state() == end, 3_000), "{text}");
        let shown = if end == State::Alarm {
            bench.alarm()
        } else {
            bench.detail()
        };
        assert_eq!(shown.as_deref(), Some(text));
        assert!(!says_locked(&bench, pushed), "{text}: no Locked text");
        assert_eq!(bench.sim.lines[mark..], ["$H"], "{text}: nothing after $H");
        bench.run_for(Duration::from_secs(1));
        assert_eq!(bench.state(), end, "{text}: stays");
        assert!(bench.closed.is_none());
        assert!(!bench.sim.laser_lit());
        assert_eq!(bench.sim.spindle, Spindle::Off);
    }
}

#[test]
fn the_loss_rule_holds_again_once_homing_ends() {
    let mut bench = unlocked(grbl_1_1f(None));
    bench.start(HOME_FIRST);
    // The `ok` of `$H` is read; then GRBL falls silent.
    assert!(bench.run_until(|b| b.line() > 4, 2_000));
    bench.sim.silent = true;
    let silent_at = bench.at;
    assert!(bench.run_until(|b| b.state() == State::NoController, 400));
    assert!(bench.at.saturating_sub(silent_at) <= Duration::from_millis(3_100));
    assert_eq!(
        bench.closed.as_deref(),
        Some("Lost the controller: no answer to ? for 3 s")
    );
}

#[test]
fn stop_during_a_quiet_homing_ends_after_the_banner() {
    let mut bench = unlocked(grbl_1_1f(None));
    bench.start(HOME_FIRST);
    bench.run_for(Duration::from_secs(5));
    bench.app(Message::Stop);
    assert!(bench.run_until(|b| b.state() == State::Alarm, 500));
    assert!(bench.closed.is_none(), "the stop was confirmed");
    assert_eq!(
        bench.alarm().as_deref(),
        Some("Reset during homing (alarm 6)")
    );
    assert!(!bench.sim.laser_lit());
}
