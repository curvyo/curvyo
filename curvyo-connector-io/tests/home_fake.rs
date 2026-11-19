//! `specs/0118` first slice through the running connector with the fake controller (architect
//! addendum M6, M8): Home from Idle, Ready and Alarm, Stop from a second app while homing, the
//! local start, the refusal while a job runs, the log and the terminal. Waiting is by status,
//! never by sleep.
#![allow(clippy::unwrap_used)]

mod common;

use std::time::Instant;

use common::{JOB, WAIT, rig, rig_with, state_of, wait_for, wait_state};
use curvyo_connector_core::{ErrorCode, JobResult, Message, State};

fn deadline() -> Instant {
    Instant::now() + WAIT
}

fn job_of(status: &Message) -> Option<uuid::Uuid> {
    match status {
        Message::Status { job, .. } => job.as_ref().map(|j| j.job_id),
        _ => None,
    }
}

#[test]
fn m8_home_reaches_every_app_at_once_and_a_second_app_may_stop_it() {
    let rig = rig(20);
    let notices = rig.server.take_notices().unwrap();
    let laptop = rig.pair("Laptop");
    rig.server.new_code().unwrap();
    let shop = rig.pair("Shop PC");
    let (mut first, _) = rig.open(&laptop);
    let (mut second, _) = rig.open(&shop);
    first.send(&Message::Home).unwrap();
    wait_state(&mut first, State::Homing);
    wait_state(&mut second, State::Homing);
    second.send(&Message::Stop).unwrap();
    wait_state(&mut second, State::Idle);
    wait_state(&mut first, State::Idle);
    let log = rig.log();
    assert!(log.contains("\t\tHome: Laptop\n"), "{log}");
    assert!(
        !log.contains("Stopped"),
        "a stopped homing is no job result: {log}"
    );
    let lines: Vec<String> = notices.try_iter().collect();
    assert!(
        lines.contains(&"Home sent by Laptop".to_owned()),
        "{lines:?}"
    );
}

#[test]
fn m6_a_fake_homing_ends_idle_and_keeps_a_ready_job() {
    let rig = rig(5000);
    let record = rig.pair("Laptop");
    let (mut connection, _) = rig.open(&record);
    connection.send(&Message::Home).unwrap();
    wait_state(&mut connection, State::Homing);
    wait_state(&mut connection, State::Idle);

    let job = connection.send_job("sign.gcode", JOB, deadline()).unwrap();
    wait_state(&mut connection, State::Ready);
    connection.send(&Message::Home).unwrap();
    let homing = wait_state(&mut connection, State::Homing);
    assert_eq!(job_of(&homing), Some(job.job_id), "the kept job is carried");
    let ready = wait_state(&mut connection, State::Ready);
    assert_eq!(job_of(&ready), Some(job.job_id));
    connection
        .send(&Message::Start { job_id: job.job_id })
        .unwrap();
    wait_state(
        &mut connection,
        State::Done {
            result: JobResult::Completed,
        },
    );
}

#[test]
fn m6_stop_during_a_fake_homing_keeps_the_ready_job() {
    let rig = rig(5000);
    let record = rig.pair("Laptop");
    let (mut connection, _) = rig.open(&record);
    let job = connection.send_job("sign.gcode", JOB, deadline()).unwrap();
    wait_state(&mut connection, State::Ready);
    connection.send(&Message::Home).unwrap();
    wait_state(&mut connection, State::Homing);
    connection.send(&Message::Stop).unwrap();
    let ready = wait_state(&mut connection, State::Ready);
    assert_eq!(job_of(&ready), Some(job.job_id));
}

#[test]
fn m6_home_in_the_fake_alarm_ends_it() {
    let rig = rig_with(5000, Some(2), false);
    let record = rig.pair("Laptop");
    let (mut connection, _) = rig.open(&record);
    let job = connection.send_job("sign.gcode", JOB, deadline()).unwrap();
    wait_state(&mut connection, State::Ready);
    connection
        .send(&Message::Start { job_id: job.job_id })
        .unwrap();
    wait_state(&mut connection, State::Alarm);
    connection.send(&Message::Home).unwrap();
    let homing = wait_state(&mut connection, State::Homing);
    assert!(matches!(homing, Message::Status { alarm: None, .. }));
    let idle = wait_state(&mut connection, State::Idle);
    assert!(matches!(idle, Message::Status { alarm: None, .. }));
}

#[test]
fn m5_with_a_local_start_home_waits_for_the_press() {
    let rig = rig_with(5000, None, true);
    let notices = rig.server.take_notices().unwrap();
    let record = rig.pair("Laptop");
    let (mut connection, _) = rig.open(&record);
    connection.send(&Message::Home).unwrap();
    let waiting = wait_state(&mut connection, State::AwaitingLocalStart);
    assert_eq!(job_of(&waiting), None, "a waiting Home names no job");
    let lines: Vec<String> = notices.try_iter().collect();
    assert!(
        lines.contains(&"Press Enter to start Home".to_owned()),
        "{lines:?}"
    );
    assert_eq!(rig.server.local_start().as_deref(), Some("Home"));
    wait_state(&mut connection, State::Homing);
    wait_state(&mut connection, State::Idle);
}

#[test]
fn m3_home_while_a_job_runs_is_refused_and_stop_still_works() {
    let rig = rig(20);
    let record = rig.pair("Laptop");
    let (mut connection, _) = rig.open(&record);
    let job = connection
        .send_job("long.gcode", &"G1 X1 Y1\n".repeat(10_000), deadline())
        .unwrap();
    wait_state(&mut connection, State::Ready);
    connection
        .send(&Message::Start { job_id: job.job_id })
        .unwrap();
    wait_state(&mut connection, State::Running);
    connection.send(&Message::Home).unwrap();
    let until = deadline();
    let refusal = loop {
        match connection.recv_message(until).unwrap() {
            Message::Error { code, message } => break (code, message),
            Message::Status { state, .. } => assert_ne!(state, State::Homing),
            _ => {}
        }
    };
    assert_eq!(
        refusal,
        (ErrorCode::WrongState, "A job is running.".to_owned())
    );
    connection.send(&Message::Stop).unwrap();
    let done = wait_for(&mut connection, |s| state_of(s) != State::Running);
    assert_eq!(
        state_of(&done),
        State::Done {
            result: JobResult::Stopped
        }
    );
    assert!(!rig.log().contains("Home:"));
}
