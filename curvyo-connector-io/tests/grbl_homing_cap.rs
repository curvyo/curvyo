//! `specs/0142` against the GRBL simulator's silent homing with a 5 s test cap: a `$H` that gets
//! no answer for the cap ends as a lost controller, the job's and the Home command's alike; an
//! `ok` before or in the step of the cap wins; a pending pause goes; Stop keeps its own rules.
//! Time is stepped, never the wall clock.
#![allow(clippy::unwrap_used)]

mod common;

use std::time::Duration;

use common::grbl_bench::{Bench, DT, PORT};
use common::grbl_sim::{SimConfig, Spindle};
use curvyo_connector_core::{ErrorCode, Event, JobResult, Message, State};
use curvyo_connector_io::controller::Controller;
use curvyo_connector_io::grbl::{GrblController, Timing};
use curvyo_connector_io::hub_log::logged_outcome;

const HOME_FIRST: &str =
    include_str!("../../curvyo-laser-core/tests/fixtures/output/grbl/home_first.gcode");
const JOB: &str = "G21\nG90\nG0 X10 Y10\nM4 S500\nG1 X20 Y20 F1000\nM5\n";
/// The test's cap.
const CAP: Duration = Duration::from_secs(5);
/// The sentence of the test's cap.
const CAP_TEXT: &str = "Lost the controller: homing took longer than 5 s";
/// What the connector writes when it gives the controller up: reset, then `M5`, then `M9`.
const SAFETY: &[u8] = b"\x18M5\nM9\n";

fn timing() -> Timing {
    Timing {
        homing: CAP,
        ..Timing::default()
    }
}

/// GRBL 1.1f with homing that answers no `?` and takes far longer than the cap (1000 s).
fn endless_homing() -> SimConfig {
    SimConfig {
        version: "1.1f",
        homing_lock: true,
        homing_steps: 100_000,
        quiet_homing: true,
        ..SimConfig::default()
    }
}

/// Connected with the test cap and unlocked: `Idle`.
fn unlocked() -> Bench {
    let mut bench = Bench::with_timing(endless_homing(), timing());
    assert!(bench.run_until(|b| b.state() == State::Alarm, 1_000));
    bench.app(Message::Unlock);
    assert!(bench.run_until(|b| b.state() == State::Idle, 500));
    bench
}

/// Applies `Home` as the app's button sends it; no refusal.
fn press_home(bench: &mut Bench) {
    let out = bench.machine.handle(Event::Message(Message::Home));
    assert!(
        !out.messages
            .iter()
            .any(|m| matches!(m, Message::Error { .. })),
        "{:?}",
        out.messages
    );
    for command in out.commands {
        let events = bench
            .ctrl
            .command(&command, bench.machine.held_job(), bench.at);
        for e in events {
            bench.apply(e);
        }
    }
}

/// Steps until `$H` went out; the time of that write, the clock start of the cap (HC1).
fn written_home(bench: &mut Bench) -> Duration {
    let from = bench.sim.log.len();
    assert!(bench.run_until(|b| b.sim.log[from..].windows(3).any(|w| w == b"$H\n"), 50));
    bench.at
}

/// Steps until the time is `at`.
fn run_to(bench: &mut Bench, at: Duration) {
    while bench.at < at {
        bench.step();
    }
}

fn pending(status: &Message) -> bool {
    matches!(
        status,
        Message::Status {
            pause_pending: true,
            ..
        }
    )
}

fn job_of(bench: &Bench) -> bool {
    matches!(bench.status(), Message::Status { job: Some(_), .. })
}

/// The loss as HC3 asks for it: at the cap exactly, the safety bytes in order and nothing else,
/// the sentence, the laser off.
fn assert_capped(bench: &Bench, since: Duration) {
    assert_eq!(bench.state(), State::NoController);
    assert_eq!(
        bench.at.saturating_sub(since),
        CAP,
        "the cap counts from the $H write"
    );
    assert_eq!(bench.detail().as_deref(), Some(CAP_TEXT));
    assert_eq!(bench.closed.as_deref(), Some(CAP_TEXT));
    assert_eq!(
        bench.last_write, SAFETY,
        "0x18, then M5, then M9, nothing else"
    );
    assert!(!job_of(bench), "the job is dropped");
    assert!(!pending(&bench.status()));
    assert!(!bench.sim.laser_lit());
    assert_eq!(bench.sim.spindle, Spindle::Off);
    assert!(!bench.sim.air);
    assert!(bench.sim.faults.is_empty(), "{:?}", bench.sim.faults);
}

#[test]
fn hc1_hc3_a_jobs_silent_homing_ends_at_the_cap() {
    let mut bench = unlocked();
    let mark = bench.sim.lines.len();
    bench.start(HOME_FIRST);
    let since = written_home(&mut bench);
    bench.run_for(CAP.saturating_sub(2 * DT));
    assert_eq!(bench.state(), State::Running, "no loss before the cap");
    assert!(bench.run_until(|b| b.state() != State::Running, 100));
    assert_capped(&bench, since);
    assert_eq!(bench.sim.lines[mark..], ["$H"], "nothing after $H");
    let (log, terminal) =
        logged_outcome(State::Running, State::NoController, &bench.status()).unwrap();
    assert_eq!(
        log.text(),
        "Failed: controller lost, homing took longer than 5 s"
    );
    assert_eq!(terminal.as_deref(), Some(CAP_TEXT));
    assert!(
        !bench
            .pushed
            .iter()
            .any(|p| p.at >= since + CAP && p.status != bench.status()),
        "no status after the cap but the loss"
    );
}

#[test]
fn hc1_hc3_the_home_button_ends_the_same_way_and_drops_a_ready_job() {
    let mut bench = unlocked();
    bench.send(JOB);
    let mark = bench.sim.lines.len();
    press_home(&mut bench);
    assert_eq!(bench.state(), State::Homing);
    let since = written_home(&mut bench);
    bench.run_for(CAP.saturating_sub(2 * DT));
    assert_eq!(bench.state(), State::Homing, "no loss before the cap");
    assert!(bench.run_until(|b| b.state() != State::Homing, 100));
    assert_capped(&bench, since);
    assert_eq!(bench.sim.lines[mark..], ["$H"], "nothing after $H");
    assert_eq!(
        logged_outcome(State::Homing, State::NoController, &bench.status()),
        None,
        "the Home button writes nothing to jobs.log"
    );
}

#[test]
fn hc5_an_ok_just_before_or_in_the_step_of_the_cap_wins() {
    // (steps before the cap at which the `ok` is read, the Home button instead of a job)
    for (early, button) in [(1, false), (0, false), (1, true), (0, true)] {
        let mut bench = unlocked();
        if button {
            press_home(&mut bench);
        } else {
            bench.start(HOME_FIRST);
        }
        let since = written_home(&mut bench);
        let read_at = (since + CAP).saturating_sub(DT * early);
        run_to(&mut bench, read_at.saturating_sub(DT));
        bench.sim.end_homing_next_step();
        bench.step();
        assert_eq!(bench.at, read_at);
        assert!(!bench.sim.homing(), "the ok was printed in this step");
        assert!(bench.closed.is_none(), "early {early}, button {button}");
        let end = if button {
            State::Idle
        } else {
            State::Done {
                result: JobResult::Completed,
            }
        };
        assert!(bench.run_until(|b| b.state() == end, 3_000), "{early}");
        bench.run_for(2 * CAP);
        assert!(bench.closed.is_none(), "early {early}, button {button}");
        assert_eq!(bench.state(), end);
        assert!(!bench.pushed.iter().any(|p| matches!(
            &p.status,
            Message::Status { detail: Some(d), .. } if d.starts_with("Lost the controller")
        )));
    }
}

#[test]
fn hc4_a_pending_pause_is_dropped_at_the_cap() {
    let mut bench = unlocked();
    bench.start(HOME_FIRST);
    let since = written_home(&mut bench);
    bench.run_for(Duration::from_secs(1));
    bench.app(Message::Pause);
    assert!(pending(&bench.status()));
    assert!(bench.run_until(|b| b.state() != State::Running, 600));
    assert_capped(&bench, since);
    assert!(
        !bench.pushed.iter().any(|p| matches!(
            p.status,
            Message::Status {
                state: State::Paused,
                ..
            }
        )),
        "never Paused"
    );
}

#[test]
fn hc6_stop_during_homing_keeps_its_rules_and_clears_the_cap() {
    let mut bench = unlocked();
    bench.start(HOME_FIRST);
    let since = written_home(&mut bench);
    run_to(
        &mut bench,
        since + CAP.saturating_sub(Duration::from_secs(1)),
    );
    bench.app(Message::Stop);
    assert!(bench.run_until(|b| b.state() == State::Alarm, 500));
    assert!(
        bench.pushed.iter().any(|p| matches!(
            p.status,
            Message::Status {
                state: State::Done {
                    result: JobResult::Stopped
                },
                ..
            }
        )),
        "Done {{ Stopped }} after the banner"
    );
    assert_eq!(
        bench.alarm().as_deref(),
        Some("Reset during homing (alarm 6)")
    );
    bench.run_for(2 * CAP);
    assert!(
        bench.closed.is_none(),
        "the cap cannot turn the stop into a loss"
    );
    assert_eq!(bench.state(), State::Alarm);
}

#[test]
fn hc6_a_controller_silent_while_the_stop_waits_is_unconfirmed_not_capped() {
    let mut bench = unlocked();
    bench.start(HOME_FIRST);
    let since = written_home(&mut bench);
    run_to(
        &mut bench,
        since + CAP.saturating_sub(Duration::from_secs(1)),
    );
    bench.sim.silent = true;
    bench.app(Message::Stop);
    assert!(bench.run_until(|b| b.state() == State::NoController, 1_000));
    assert_eq!(
        bench.detail().as_deref(),
        Some("Could not confirm the machine stopped")
    );
}

#[test]
fn hc7_after_the_cap_a_reconnect_identifies_afresh_and_resumes_nothing() {
    let mut bench = unlocked();
    let job = bench.send(JOB);
    press_home(&mut bench);
    written_home(&mut bench);
    assert!(bench.run_until(|b| b.state() == State::NoController, 1_000));
    // The port comes back: a fresh controller identifies (G3, G2).
    bench.ctrl = GrblController::new(PORT, 115_200, timing(), bench.at);
    bench.closed = None;
    let mark = bench.sim.lines.len();
    assert!(bench.run_until(|b| b.state() != State::NoController, 1_000));
    assert!(!job_of(&bench), "the dropped job is not back");
    let out = bench
        .machine
        .handle(Event::Message(Message::Start { job_id: job }));
    assert!(
        out.messages.iter().any(|m| matches!(
            m,
            Message::Error {
                code: ErrorCode::WrongState | ErrorCode::UnknownJob,
                ..
            }
        )),
        "{:?}",
        out.messages
    );
    assert!(out.commands.is_empty(), "a new Start needs a new Send");
    bench.run_for(Duration::from_secs(1));
    assert!(
        bench.sim.lines[mark..]
            .iter()
            .all(|l| l.starts_with('$') || l == "M5" || l == "M9"),
        "the safety lines and the identify only, no job line: {:?}",
        &bench.sim.lines[mark..]
    );
}
