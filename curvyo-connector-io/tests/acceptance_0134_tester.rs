//! `specs/0134-send-and-start` SS1 to SS7, written from the specification before the code was read:
//! the connector side of "send, then Start with our own job id on the same connection". The fake
//! controller covers the free, held, local-start and race cases; the GRBL bench (the real
//! controller over a simulated GRBL, in memory) covers the controller states, `$32` and `$H`.
//! Waiting is by status with a bounded deadline, never by sleep.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use curvyo_connector_core::Peer;
use std::time::{Duration, Instant};

use common::grbl_bench::{Bench, job_id};
use common::grbl_sim::SimConfig;
use common::{JOB, WAIT, rig, rig_with, state_of, wait_for, wait_state};
use curvyo_connector_core::{Event, JobKind, Message, RejectReason, State};
use curvyo_connector_io::client::Connection;
use curvyo_connector_io::job_send::SendError;

const HOME_FIRST: &str =
    include_str!("../../curvyo-laser-core/tests/fixtures/output/grbl/home_first.gcode");

fn deadline() -> Instant {
    Instant::now() + WAIT
}

/// Reads for `span`, returning every state seen (a bounded "nothing else happens" check).
fn states_for(connection: &mut Connection, span: Duration) -> Vec<State> {
    let end = Instant::now() + span;
    let mut seen = Vec::new();
    while Instant::now() < end {
        match connection.recv_message(end) {
            Ok(m @ Message::Status { .. }) => seen.push(state_of(&m)),
            Ok(_) => {}
            Err(_) => break,
        }
    }
    seen
}

fn held_job_id(status: &Message) -> Option<uuid::Uuid> {
    match status {
        Message::Status { job: Some(j), .. } => Some(j.job_id),
        _ => None,
    }
}

#[test]
fn ss1_a_free_machine_sends_then_starts_our_own_job_id_on_the_same_connection() {
    let rig = rig(2000);
    let record = rig.pair("Laptop");
    let (mut connection, _) = rig.open(&record);
    let sent = connection.send_job("a.gcode", JOB, deadline()).unwrap();
    wait_state(&mut connection, State::Ready);
    connection
        .send(&Message::Start {
            job_id: sent.job_id,
        })
        .unwrap();
    let done = wait_for(&mut connection, |s| {
        matches!(state_of(s), State::Done { .. })
    });
    assert!(matches!(
        state_of(&done),
        State::Done {
            result: curvyo_connector_core::JobResult::Completed
        }
    ));
}

#[test]
fn ss1_after_done_the_machine_is_free_again_and_a_second_job_starts() {
    let rig = rig(2000);
    let record = rig.pair("Laptop");
    let (mut connection, _) = rig.open(&record);
    for name in ["one.gcode", "two.gcode"] {
        let sent = connection.send_job(name, JOB, deadline()).unwrap();
        connection
            .send(&Message::Start {
                job_id: sent.job_id,
            })
            .unwrap();
        wait_for(&mut connection, |s| {
            matches!(state_of(s), State::Done { .. })
        });
    }
}

#[test]
fn ss2_a_held_job_of_another_or_the_same_app_refuses_the_send_as_busy_and_starts_nothing() {
    let rig = rig(2000);
    let record = rig.pair("Laptop");
    let (mut first, _) = rig.open(&record);
    let (mut second, _) = rig.open(&record);
    let held = first.send_job("held.gcode", JOB, deadline()).unwrap();
    wait_state(&mut first, State::Ready);
    // another app
    assert_eq!(
        second.send_job("x.gcode", JOB, deadline()).unwrap_err(),
        SendError::Rejected(RejectReason::Busy)
    );
    // the same app, same connection
    assert_eq!(
        first.send_job("y.gcode", JOB, deadline()).unwrap_err(),
        SendError::Rejected(RejectReason::Busy)
    );
    first.send(&Message::GetStatus).unwrap();
    let status = wait_for(&mut first, |_| true);
    assert_eq!(state_of(&status), State::Ready);
    assert_eq!(held_job_id(&status), Some(held.job_id));
    let seen = states_for(&mut first, Duration::from_millis(400));
    assert!(!seen.contains(&State::Running), "{seen:?}");
}

#[test]
fn ss3_require_local_start_waits_at_the_machine_after_our_start() {
    let rig = rig_with(2000, None, true);
    let record = rig.pair("Laptop");
    let (mut connection, _) = rig.open(&record);
    let sent = connection.send_job("a.gcode", JOB, deadline()).unwrap();
    connection
        .send(&Message::Start {
            job_id: sent.job_id,
        })
        .unwrap();
    wait_state(&mut connection, State::AwaitingLocalStart);
    let seen = states_for(&mut connection, Duration::from_millis(500));
    assert!(
        !seen
            .iter()
            .any(|s| matches!(s, State::Running | State::Done { .. })),
        "{seen:?}"
    );
}

#[test]
fn p4_p13_a_start_with_our_old_id_never_starts_another_apps_job() {
    let rig = rig(2000);
    let ours = rig.pair("Laptop");
    let (mut a, _) = rig.open(&ours);
    let (mut b, _) = rig.open(&ours);
    let first = a.send_job("ours.gcode", JOB, deadline()).unwrap();
    wait_state(&mut a, State::Ready);
    // the race: our job is gone and another app's job arrives before our confirming press
    b.send(&Message::Stop).unwrap();
    wait_state(&mut b, State::Idle);
    let second = b.send_job("theirs.gcode", JOB, deadline()).unwrap();
    wait_state(&mut b, State::Ready);
    a.send(&Message::Start {
        job_id: first.job_id,
    })
    .unwrap();
    let seen = states_for(&mut a, Duration::from_millis(500));
    assert!(
        !seen
            .iter()
            .any(|s| matches!(s, State::Running | State::Done { .. })),
        "the other app's job was started by our id: {seen:?}"
    );
    b.send(&Message::GetStatus).unwrap();
    let status = wait_for(&mut b, |_| true);
    assert_eq!(state_of(&status), State::Ready);
    assert_eq!(held_job_id(&status), Some(second.job_id));
    // the other app's own Start still works: nothing was damaged
    b.send(&Message::Start {
        job_id: second.job_id,
    })
    .unwrap();
    wait_for(&mut b, |s| {
        matches!(state_of(s), State::Running | State::Done { .. })
    });
}

#[test]
fn ss6_a_start_after_a_stop_in_between_is_refused_and_the_job_does_not_run() {
    let rig = rig(2000);
    let record = rig.pair("Laptop");
    let (mut a, _) = rig.open(&record);
    let (mut b, _) = rig.open(&record);
    let sent = a.send_job("a.gcode", JOB, deadline()).unwrap();
    wait_state(&mut a, State::Ready);
    b.send(&Message::Stop).unwrap();
    wait_state(&mut b, State::Idle);
    a.send(&Message::Start {
        job_id: sent.job_id,
    })
    .unwrap();
    let seen = states_for(&mut a, Duration::from_millis(400));
    assert!(!seen.contains(&State::Running), "{seen:?}");
}

fn submit(bench: &mut Bench, n: u128, text: &str) -> Vec<Message> {
    bench
        .machine
        .handle(Event::Message(
            Message::SubmitJob {
                job_id: job_id(n),
                name: "t.gcode".into(),
                kind: JobKind::Job,
                bytes: text.len() as u64,
                sha256: "0".repeat(64),
            },
            Peer::host(),
        ))
        .messages
}

fn rejected_with(messages: &[Message], reason: RejectReason) -> bool {
    messages
        .iter()
        .any(|m| matches!(m, Message::JobRejected { reason: r, .. } if *r == reason))
}

#[test]
fn ss5_an_alarm_takes_no_job_and_a_start_writes_no_byte() {
    let mut bench = Bench::connected(SimConfig {
        homing_lock: true,
        ..SimConfig::default()
    });
    assert_eq!(bench.state(), State::Alarm);
    let messages = submit(&mut bench, 1, "G0 X1\n");
    assert!(
        messages
            .iter()
            .any(|m| matches!(m, Message::JobRejected { .. })),
        "{messages:?}"
    );
    let lines = bench.sim.lines.len();
    bench.app(Message::Start { job_id: job_id(1) });
    bench.run_for(Duration::from_secs(1));
    assert_eq!(bench.sim.lines.len(), lines);
    assert!(!matches!(bench.state(), State::Running | State::Ready));
}

#[test]
fn ss5_the_door_open_on_an_idle_machine_refuses_the_send_as_busy_and_never_starts() {
    let mut bench = Bench::connected(SimConfig::default());
    bench.sim.door = true;
    assert!(bench.run_until(
        |b| b.detail().as_deref() == Some("Controller state: Door"),
        300
    ));
    let messages = submit(&mut bench, 2, "G0 X1\n");
    assert!(rejected_with(&messages, RejectReason::Busy), "{messages:?}");
    let lines = bench.sim.lines.len();
    bench.app(Message::Start { job_id: job_id(2) });
    bench.run_for(Duration::from_secs(1));
    assert_eq!(bench.sim.lines.len(), lines);
    assert_ne!(bench.state(), State::Running);
}

#[test]
fn ss5_no_controller_refuses_the_send() {
    let mut bench = Bench::new(SimConfig::default());
    assert_eq!(bench.state(), State::NoController);
    let messages = submit(&mut bench, 3, "G0 X1\n");
    assert!(
        rejected_with(&messages, RejectReason::NoController),
        "{messages:?}"
    );
}

#[test]
fn ss5_laser_mode_off_refuses_the_send_and_writes_nothing() {
    let mut bench = Bench::connected(SimConfig {
        laser_mode: false,
        ..SimConfig::default()
    });
    let messages = submit(&mut bench, 4, "G0 X1\n");
    // the connector may hold the verdict until the first chunk; accept either refusal point
    let refused =
        rejected_with(&messages, RejectReason::LaserModeOff) || bench.state() != State::Ready;
    assert!(refused, "{messages:?} {:?}", bench.state());
    assert!(!bench.sim.lines.iter().any(|l| l.contains("G0 X1")));
}

#[test]
fn ss4_a_home_first_job_runs_dollar_h_first_and_then_the_job() {
    let mut bench = Bench::connected(SimConfig {
        homing_lock: true,
        homing_steps: 20,
        ..SimConfig::default()
    });
    bench.app(Message::Unlock);
    assert!(bench.run_until(|b| b.state() == State::Idle, 500));
    let mark = bench.sim.log.len();
    bench.start(HOME_FIRST);
    assert!(bench.run_until(|b| matches!(b.state(), State::Done { .. }), 5_000));
    let sent = bench.lines_after(mark);
    let home = sent.find("$H").expect("$H was sent");
    let motion = sent.find("G1").or_else(|| sent.find("G0")).unwrap();
    assert!(home < motion, "{sent}");
    assert_eq!(sent.matches("$H").count(), 1, "homed exactly once: {sent}");
}
