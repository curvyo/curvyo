//! The job path through the connector with the fake controller: send, start, pause, resume, stop,
//! the local start, the alarm, the log. Waiting is by status, never by sleep.
#![allow(clippy::unwrap_used)]

mod common;

use std::time::{Duration, Instant};

use common::{JOB, WAIT, line_of, rig, rig_with, state_of, wait_for, wait_state};
use curvyo_connector_core::{JobResult, Message, RejectReason, State};
use curvyo_connector_io::job_send::SendError;

fn deadline() -> Instant {
    Instant::now() + WAIT
}

fn many_lines(n: usize) -> String {
    "G1 X1 Y1\n".repeat(n)
}

#[test]
fn q13_a_job_is_held_as_ready_with_its_lines_and_size() {
    let rig = rig(20);
    let record = rig.pair("Laptop");
    let (mut connection, _) = rig.open(&record);
    let sent = connection
        .send_job("coaster.gcode", JOB, deadline())
        .unwrap();
    assert_eq!(sent.lines, 6);
    assert_eq!(sent.bytes, JOB.len() as u64);
    wait_state(&mut connection, State::Ready);
}

#[test]
fn q13_refusals_are_named_and_logged() {
    let rig = rig(20);
    let record = rig.pair("Laptop");
    let (mut connection, _) = rig.open(&record);
    assert_eq!(
        connection
            .send_job("binary.bin", "G1 \u{e9}\n", deadline())
            .unwrap_err(),
        SendError::Rejected(RejectReason::NotAscii)
    );
    assert_eq!(
        connection
            .announce_oversized("huge.gcode", 65 * 1024 * 1024, deadline())
            .unwrap_err(),
        SendError::Rejected(RejectReason::TooLarge)
    );
    connection.send_job("one.gcode", JOB, deadline()).unwrap();
    assert_eq!(
        connection
            .send_job("two.gcode", JOB, deadline())
            .unwrap_err(),
        SendError::Rejected(RejectReason::Busy)
    );
    let log = rig.log();
    assert!(log.contains("\tbinary.bin\tRejected: NotAscii\t"), "{log}");
    assert!(log.contains("\thuge.gcode\tRejected: TooLarge\t"), "{log}");
    assert!(log.contains("\ttwo.gcode\tRejected: Busy\t"), "{log}");
    // The refused jobs held nothing: the first one is still ready.
    connection.send(&Message::GetStatus).unwrap();
    assert_eq!(state_of(&wait_for(&mut connection, |_| true)), State::Ready);
}

#[test]
fn q14_q15_start_pause_resume_stop_and_the_log() {
    let rig = rig(2000);
    let record = rig.pair("Laptop");
    let (mut connection, _) = rig.open(&record);
    let job = connection
        .send_job("long.gcode", &many_lines(1_000_000), deadline())
        .unwrap();
    wait_state(&mut connection, State::Ready);
    connection
        .send(&Message::Start { job_id: job.job_id })
        .unwrap();
    wait_state(&mut connection, State::Running);
    wait_for(&mut connection, |s| line_of(s) > 0);
    connection.send(&Message::Pause).unwrap();
    let paused = wait_state(&mut connection, State::Paused);
    let held = line_of(&paused);
    connection.send(&Message::GetStatus).unwrap();
    // Newer pushes may still be in flight; GetStatus answers with the newest state.
    let after = wait_state(&mut connection, State::Paused);
    assert!(line_of(&after) >= held);
    connection.send(&Message::Resume).unwrap();
    wait_state(&mut connection, State::Running);
    let resumed = wait_for(&mut connection, |s| line_of(s) > line_of(&after));
    assert!(line_of(&resumed) > line_of(&after));
    connection.send(&Message::Stop).unwrap();
    wait_state(
        &mut connection,
        State::Done {
            result: JobResult::Stopped,
        },
    );
    assert!(rig.log().contains("\tlong.gcode\tStopped\t"));
}

#[test]
fn q14_a_short_job_completes_and_the_row_can_read_the_end() {
    let rig = rig(5000);
    let record = rig.pair("Laptop");
    let (mut connection, _) = rig.open(&record);
    let job = connection
        .send_job("coaster.gcode", JOB, deadline())
        .unwrap();
    connection
        .send(&Message::Start { job_id: job.job_id })
        .unwrap();
    let done = wait_state(
        &mut connection,
        State::Done {
            result: JobResult::Completed,
        },
    );
    assert_eq!(line_of(&done), 6);
    assert!(rig.log().contains("\tcoaster.gcode\tCompleted\t"));
}

#[test]
fn q15_pause_and_resume_do_nothing_in_the_wrong_state() {
    let rig = rig(20);
    let record = rig.pair("Laptop");
    let (mut connection, _) = rig.open(&record);
    let result = connection.command(&Message::Pause, State::Idle);
    assert!(result.is_err());
    assert!(
        connection
            .command(&Message::Stop, State::Idle)
            .unwrap()
            .is_none()
    );
}

#[test]
fn q16_a_running_job_survives_the_app_and_stop_works_from_a_fresh_connection() {
    let rig = rig(20);
    let record = rig.pair("Laptop");
    let job_id = {
        let (mut connection, _) = rig.open(&record);
        let job = connection
            .send_job("long.gcode", &many_lines(10_000), deadline())
            .unwrap();
        wait_state(&mut connection, State::Ready);
        connection
            .send(&Message::Start { job_id: job.job_id })
            .unwrap();
        wait_state(&mut connection, State::Running);
        job.job_id
    };
    let _ = job_id;
    let (mut second, status) = rig.open(&record);
    assert_eq!(state_of(&status), State::Running);
    second.send(&Message::Stop).unwrap();
    wait_state(
        &mut second,
        State::Done {
            result: JobResult::Stopped,
        },
    );
}

#[test]
fn q16_start_works_from_a_fresh_connection_with_the_id_from_the_ready_status() {
    let rig = rig(5000);
    let record = rig.pair("Laptop");
    let sent = {
        let (mut a, _) = rig.open(&record);
        let sent = a.send_job("coaster.gcode", JOB, deadline()).unwrap();
        wait_state(&mut a, State::Ready);
        sent
    };
    let (mut b, status) = rig.open(&record);
    let Message::Status {
        state: State::Ready,
        job: Some(progress),
        ..
    } = status
    else {
        panic!("Ready status with the job expected, got {status:?}");
    };
    assert_eq!(progress.job_id, sent.job_id);
    assert_eq!((progress.line, progress.elapsed_ms), (0, 0));
    b.send(&Message::Start {
        job_id: progress.job_id,
    })
    .unwrap();
    wait_state(
        &mut b,
        State::Done {
            result: JobResult::Completed,
        },
    );
}

#[test]
fn q17_with_a_local_start_the_job_waits_for_the_connector_terminal() {
    let rig = rig_with(5000, None, true);
    let notices = rig.server.take_notices().unwrap();
    let record = rig.pair("Laptop");
    let (mut connection, _) = rig.open(&record);
    let job = connection.send_job("sign.gcode", JOB, deadline()).unwrap();
    connection
        .send(&Message::Start { job_id: job.job_id })
        .unwrap();
    wait_state(&mut connection, State::AwaitingLocalStart);
    let lines: Vec<String> = notices.try_iter().collect();
    assert!(
        lines.contains(&"Press Enter to start sign.gcode".to_owned()),
        "{lines:?}"
    );
    assert_eq!(rig.server.local_start().as_deref(), Some("sign.gcode"));
    wait_state(
        &mut connection,
        State::Done {
            result: JobResult::Completed,
        },
    );
    assert_eq!(rig.server.local_start(), None);
}

#[test]
fn q16_a_connection_cut_mid_transfer_drops_the_job_and_the_next_submit_is_accepted() {
    let rig = rig(20);
    let record = rig.pair("Laptop");
    {
        let (mut cut, _) = rig.open(&record);
        cut.send(&Message::SubmitJob {
            job_id: uuid::Uuid::from_u128(7),
            name: "half.gcode".to_owned(),
            kind: curvyo_connector_core::JobKind::Job,
            bytes: 1000,
            sha256: "0".repeat(64),
        })
        .unwrap();
        wait_state(&mut cut, State::Receiving);
    }
    let (mut second, opening) = rig.open(&record);
    if state_of(&opening) != State::Idle {
        wait_state(&mut second, State::Idle);
    }
    second.send_job("again.gcode", JOB, deadline()).unwrap();
    wait_state(&mut second, State::Ready);
}

#[test]
fn q17_stop_while_waiting_discards_the_job() {
    let rig = rig_with(20, None, true);
    let record = rig.pair("Laptop");
    let (mut connection, _) = rig.open(&record);
    let job = connection.send_job("sign.gcode", JOB, deadline()).unwrap();
    connection
        .send(&Message::Start { job_id: job.job_id })
        .unwrap();
    wait_state(&mut connection, State::AwaitingLocalStart);
    connection.send(&Message::Stop).unwrap();
    wait_state(&mut connection, State::Idle);
    assert_eq!(rig.server.local_start(), None);
}

#[test]
fn q19_an_alarm_stops_the_stream_is_logged_and_keeps_the_machine_busy() {
    let rig = rig_with(5000, Some(3), false);
    let record = rig.pair("Laptop");
    let (mut connection, _) = rig.open(&record);
    let job = connection
        .send_job("etch.gcode", &many_lines(100), deadline())
        .unwrap();
    connection
        .send(&Message::Start { job_id: job.job_id })
        .unwrap();
    let alarm = wait_state(&mut connection, State::Alarm);
    let Message::Status { alarm: text, .. } = &alarm else {
        panic!()
    };
    assert_eq!(text.as_deref(), Some("Fake alarm at line 3"));
    assert!(
        rig.log()
            .contains("\tetch.gcode\tAlarm: Fake alarm at line 3\t")
    );
    assert_eq!(
        connection
            .send_job("next.gcode", JOB, deadline())
            .unwrap_err(),
        SendError::Rejected(RejectReason::Busy)
    );
    // The stream really stopped: the state stays Alarm.
    std::thread::sleep(Duration::from_millis(200));
    connection.send(&Message::GetStatus).unwrap();
    assert_eq!(state_of(&wait_for(&mut connection, |_| true)), State::Alarm);
}

#[test]
fn q12_a_second_connection_sees_a_job_another_computer_sent() {
    let rig = rig(20);
    let record = rig.pair("Laptop");
    {
        let (mut one, _) = rig.open(&record);
        one.send_job("x.gcode", JOB, deadline()).unwrap();
    }
    let (_, status) = rig.open(&record);
    assert_eq!(state_of(&status), State::Ready);
}

/// The sample job of the demo (`specs/0064`, Frage 1 A): the connector holds it as Ready.
#[test]
fn the_sample_job_of_the_demo_is_accepted_and_inside_the_generic_limits() {
    let job = include_str!("../../tests/fixtures/connector/sample-job.gcode");
    assert!(job.bytes().all(|b| b == b'\n' || (0x20..0x7f).contains(&b)));
    let words = |prefix: char| {
        job.split_whitespace()
            .filter_map(|w| w.strip_prefix(prefix)?.parse::<f64>().ok())
            .fold(0.0_f64, f64::max)
    };
    assert!(words('S') <= 1000.0 && words('F') <= 6000.0);
    let rig = rig(20);
    let record = rig.pair("Laptop");
    let (mut connection, _) = rig.open(&record);
    let sent = connection
        .send_job("sample-job.gcode", job, deadline())
        .unwrap();
    assert!((395..=405).contains(&sent.lines), "{}", sent.lines);
    wait_state(&mut connection, State::Ready);
}
