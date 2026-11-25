//! The job list on a connector with the fake controller (`specs/0132` criteria 1 to 7, 11):
//! three jobs, reorder, start, restart, a chain with a failure. Waiting is by condition.
#![allow(clippy::unwrap_used, clippy::too_many_lines)]

mod common;

use std::time::Instant;

use common::service::service;
use common::{JOB, WAIT, rig, rig_with, state_of, wait_state};
use curvyo_connector_core::{ErrorCode, ListEntry, Message, Role, State};
use curvyo_connector_io::client::{CONNECT_TIMEOUT, Connection, open_paired};
use curvyo_connector_io::server::RunningServer;

fn lines(n: usize) -> String {
    "G1 X1 F1000\n".repeat(n)
}

/// Reads until a `JobList` satisfies `done`; panics after [`WAIT`].
fn wait_list(
    connection: &mut Connection,
    done: impl Fn(&Option<ListEntry>, &[ListEntry], &Option<String>) -> bool,
) -> (Option<ListEntry>, Vec<ListEntry>, Option<String>) {
    let deadline = Instant::now() + WAIT;
    loop {
        if let Message::JobList {
            current,
            waiting,
            note,
        } = connection.recv_message(deadline).unwrap()
            && done(&current, &waiting, &note)
        {
            return (current, waiting, note);
        }
    }
}

fn names(waiting: &[ListEntry]) -> Vec<&str> {
    waiting.iter().map(|e| e.name.as_str()).collect()
}

fn first_list(connection: &Connection) -> (Option<ListEntry>, Vec<ListEntry>) {
    match &connection.job_list {
        Some(Message::JobList {
            current, waiting, ..
        }) => (current.clone(), waiting.clone()),
        other => panic!("a connection starts with the list: {other:?}"),
    }
}

fn wait_error(connection: &mut Connection, code: ErrorCode) {
    let deadline = Instant::now() + WAIT;
    loop {
        if let Message::Error { code: got, .. } = connection.recv_message(deadline).unwrap() {
            assert_eq!(got, code);
            return;
        }
    }
}

#[test]
fn three_jobs_are_listed_reordered_for_everyone_and_started_by_a_press() {
    let rig = rig(20);
    let record = rig.pair("Laptop");
    let (mut a, _) = rig.open(&record);
    let (mut b, _) = rig.open(&record);
    let one = a
        .send_job("one.gcode", JOB, common::service::deadline())
        .unwrap();
    assert_eq!(one.position, None);
    let two = a
        .send_job("two.gcode", JOB, common::service::deadline())
        .unwrap();
    let three = b
        .send_job("three.gcode", &lines(200), common::service::deadline())
        .unwrap();
    assert_eq!((two.position, three.position), (Some(1), Some(2)));
    wait_list(&mut b, |_, w, _| names(w) == ["two.gcode", "three.gcode"]);
    // A move changes the order for both connections at once.
    a.send(&Message::MoveJob {
        job_id: three.job_id,
        from: 2,
        to: 1,
    })
    .unwrap();
    for connection in [&mut a, &mut b] {
        let (_, waiting, _) = wait_list(connection, |_, w, _| {
            names(w) == ["three.gcode", "two.gcode"]
        });
        assert_eq!(waiting[0].sent_by, "Laptop");
    }
    // A stale move answers WrongState and the list again.
    a.send(&Message::MoveJob {
        job_id: three.job_id,
        from: 2,
        to: 1,
    })
    .unwrap();
    wait_error(&mut a, ErrorCode::WrongState);
    // Nothing starts by itself; Start runs the first job, then the first waiting one.
    a.send(&Message::Start { job_id: one.job_id }).unwrap();
    wait_state(
        &mut a,
        State::Done {
            result: curvyo_connector_core::JobResult::Completed,
        },
    );
    a.send(&Message::Start { job_id: two.job_id }).unwrap();
    wait_error(&mut a, ErrorCode::WrongState);
    a.send(&Message::Start {
        job_id: three.job_id,
    })
    .unwrap();
    let (current, waiting, _) = wait_list(&mut a, |c, _, _| {
        c.as_ref().is_some_and(|c| c.job_id == three.job_id)
    });
    assert_eq!(current.unwrap().name, "three.gcode");
    assert_eq!(names(&waiting), ["two.gcode"]);
    // Stop ends only the current job; the list keeps the rest.
    a.send(&Message::Stop).unwrap();
    wait_state(
        &mut a,
        State::Done {
            result: curvyo_connector_core::JobResult::Stopped,
        },
    );
    a.send(&Message::GetStatus).unwrap();
    let (_, kept) = first_list(&rig.open(&record).0);
    // Frage 390 C (customer 2026-11-25): the stopped job stays first as an interrupted entry.
    assert_eq!(names(&kept), ["three.gcode", "two.gcode"]);
    assert!(kept[0].interrupted_at.is_some());
}

#[test]
fn a_connection_starts_with_the_list_before_its_first_status() {
    let rig = rig(20);
    let record = rig.pair("Laptop");
    let (mut a, _) = rig.open(&record);
    a.send_job("one.gcode", JOB, common::service::deadline())
        .unwrap();
    a.send_job("two.gcode", JOB, common::service::deadline())
        .unwrap();
    let (fresh, status) = rig.open(&record);
    let (current, waiting) = first_list(&fresh);
    assert_eq!(current.unwrap().name, "one.gcode");
    assert_eq!(names(&waiting), ["two.gcode"]);
    assert_eq!(state_of(&status), State::Ready);
}

#[test]
fn waiting_jobs_and_flags_survive_a_restart_and_a_running_job_comes_back_interrupted() {
    let rig = rig(20);
    let record = rig.pair("Laptop");
    let address = rig.address();
    let dir = rig.dir.path().to_owned();
    let (mut a, _) = rig.open(&record);
    let running = a
        .send_job("long.gcode", &lines(400), common::service::deadline())
        .unwrap();
    a.send(&Message::Start {
        job_id: running.job_id,
    })
    .unwrap();
    wait_state(&mut a, State::Running);
    let one = a
        .send_job("one.gcode", JOB, common::service::deadline())
        .unwrap();
    let two = a
        .send_job("two.gcode", JOB, common::service::deadline())
        .unwrap();
    let three = a
        .send_job("three.gcode", JOB, common::service::deadline())
        .unwrap();
    a.send(&Message::SetRunNext {
        job_id: two.job_id,
        on: true,
    })
    .unwrap();
    wait_list(&mut a, |_, w, _| w.iter().any(|e| e.run_next));
    let _ = (address, one, three);
    // Let it reach a line, then stop the connector as a signal would.
    let mut progress = wait_state(&mut a, State::Running);
    while common::line_of(&progress) == 0 {
        progress = common::wait_for(&mut a, |_| true);
    }
    drop(a);
    assert_eq!(rig.server.stop_for_exit().as_deref(), Some("long.gcode"));
    rig.server.shutdown();

    // A new connector on the same state directory.
    let server = RunningServer::start(&common::config(dir.clone(), 20, None)).unwrap();
    let (b, status) = open_paired(
        &server.addr().to_string(),
        record.spki,
        &record.token,
        CONNECT_TIMEOUT,
    )
    .unwrap();
    assert_eq!(state_of(&status), State::Idle, "nothing starts by itself");
    let (current, waiting) = first_list(&b);
    assert!(current.is_none());
    assert_eq!(
        names(&waiting),
        ["long.gcode", "one.gcode", "two.gcode", "three.gcode"]
    );
    assert!(waiting[0].interrupted_at.is_some_and(|line| line > 0));
    assert_eq!(waiting[0].position, 0);
    assert_eq!(waiting[1].sent_by, "Laptop");
    assert!(waiting[2].run_next, "the flag survives");
    let mut b = b;
    // The interrupted job is never started by Start.
    b.send(&Message::Start {
        job_id: waiting[0].job_id,
    })
    .unwrap();
    wait_error(&mut b, ErrorCode::WrongState);
    // An owner (foreground: every paired client) removes it; the rest runs on from one.
    b.send(&Message::RemoveJob {
        job_id: waiting[0].job_id,
    })
    .unwrap();
    wait_list(&mut b, |_, w, _| {
        names(w) == ["one.gcode", "two.gcode", "three.gcode"]
    });
    b.send(&Message::Start {
        job_id: waiting[1].job_id,
    })
    .unwrap();
    wait_state(&mut b, State::Running);
    server.shutdown();
}

#[test]
fn a_corrupt_stored_job_is_dropped_with_a_log_line_and_the_rest_stays() {
    let rig = rig(20);
    let record = rig.pair("Laptop");
    let dir = rig.dir.path().to_owned();
    let (mut a, _) = rig.open(&record);
    a.send_job("one.gcode", JOB, common::service::deadline())
        .unwrap();
    let two = a
        .send_job("two.gcode", JOB, common::service::deadline())
        .unwrap();
    a.send_job("three.gcode", JOB, common::service::deadline())
        .unwrap();
    drop(a);
    rig.server.shutdown();
    let path = dir.join("jobs").join(format!("{}.gcode", two.job_id));
    std::fs::write(&path, "G0 X99\n").unwrap();
    let server = RunningServer::start(&common::config(dir.clone(), 20, None)).unwrap();
    let (b, _) = open_paired(
        &server.addr().to_string(),
        record.spki,
        &record.token,
        CONNECT_TIMEOUT,
    )
    .unwrap();
    let (_, waiting) = first_list(&b);
    assert_eq!(names(&waiting), ["one.gcode", "three.gcode"]);
    let log = std::fs::read_to_string(dir.join("admin.log")).unwrap();
    assert!(log.contains("job-dropped\t\ttwo.gcode\t"), "{log}");
    assert!(log.contains("HashMismatch"), "{log}");
    server.shutdown();
}

#[test]
fn a_chain_follows_by_its_flags_and_an_alarm_in_the_second_job_cancels_it() {
    // The fake controller raises an alarm at line 3: a two-line job never reaches it.
    let rig = rig_with(2000, Some(3), false);
    let record = rig.pair("Laptop");
    let (mut a, _) = rig.open(&record);
    let short = "G21\nG90\n";
    let one = a
        .send_job("one.gcode", short, common::service::deadline())
        .unwrap();
    let two = a
        .send_job("two.gcode", JOB, common::service::deadline())
        .unwrap();
    let three = a
        .send_job("three.gcode", short, common::service::deadline())
        .unwrap();
    for id in [two.job_id, three.job_id] {
        a.send(&Message::SetRunNext {
            job_id: id,
            on: true,
        })
        .unwrap();
    }
    wait_list(&mut a, |_, w, _| {
        w.iter().all(|e| e.run_next) && w.len() == 2
    });
    a.send(&Message::Start { job_id: one.job_id }).unwrap();
    // No further press: the second job starts by itself and meets the alarm.
    let (current, waiting, note) = wait_list(&mut a, |_, _, n| n.is_some());
    a.send(&Message::GetStatus).unwrap();
    wait_state(&mut a, State::Alarm);
    assert!(current.is_none(), "the alarm dropped the held job");
    // Frage 390 C: the job the alarm ended stays first as an interrupted entry.
    assert_eq!(names(&waiting), ["two.gcode", "three.gcode"]);
    assert!(waiting[0].interrupted_at.is_some());
    assert!(waiting.iter().all(|e| !e.run_next), "the flags are cleared");
    assert_eq!(note.as_deref(), Some("Chain stopped: Alarm"));
    let log = rig.log();
    assert!(log.contains("\ttwo.gcode\tAlarm"), "{log}");
}

#[test]
fn members_and_owners_have_their_rights_on_a_configured_connector() {
    let service = service();
    let (mut owner_c, _token) = service.setup(20);
    let (_, mut member_c) = service.pair(&mut owner_c, "Phone", Role::Member);
    let one = owner_c
        .send_job("one.gcode", JOB, common::service::deadline())
        .unwrap();
    let mine = member_c
        .send_job("mine.gcode", JOB, common::service::deadline())
        .unwrap();
    let other = owner_c
        .send_job("other.gcode", JOB, common::service::deadline())
        .unwrap();
    assert_eq!(mine.position, Some(1));
    // A member does not reorder, does not remove the owner's job, may flag its own.
    member_c
        .send(&Message::MoveJob {
            job_id: other.job_id,
            from: 2,
            to: 1,
        })
        .unwrap();
    wait_error(&mut member_c, ErrorCode::OwnerRequired);
    member_c
        .send(&Message::RemoveJob {
            job_id: other.job_id,
        })
        .unwrap();
    wait_error(&mut member_c, ErrorCode::OwnerRequired);
    member_c
        .send(&Message::SetRunNext {
            job_id: other.job_id,
            on: true,
        })
        .unwrap();
    wait_error(&mut member_c, ErrorCode::OwnerRequired);
    member_c
        .send(&Message::SetRunNext {
            job_id: mine.job_id,
            on: true,
        })
        .unwrap();
    wait_list(&mut owner_c, |_, w, _| {
        w.iter().any(|e| e.run_next && e.name == "mine.gcode")
    });
    // The member removes its own waiting job; the owner removes another's.
    let extra = member_c
        .send_job("extra.gcode", JOB, common::service::deadline())
        .unwrap();
    member_c
        .send(&Message::RemoveJob {
            job_id: extra.job_id,
        })
        .unwrap();
    wait_list(&mut owner_c, |_, w, _| {
        names(w) == ["mine.gcode", "other.gcode"]
    });
    // The owner starts the first job; the member's flagged job follows but waits for an owner
    // (Frage 303 A, `0128` A13).
    owner_c
        .send(&Message::Start { job_id: one.job_id })
        .unwrap();
    let waiting = common::wait_for(&mut owner_c, |s| state_of(s) == State::AwaitingLocalStart);
    let Message::Status { requested_by, .. } = waiting else {
        panic!()
    };
    assert_eq!(requested_by.as_deref(), Some("Phone"));
    owner_c
        .send(&Message::Start {
            job_id: mine.job_id,
        })
        .unwrap();
    wait_state(&mut owner_c, State::Running);
    let again = member_c
        .send_job("again.gcode", JOB, common::service::deadline())
        .unwrap();
    owner_c
        .send(&Message::RemoveJob {
            job_id: again.job_id,
        })
        .unwrap();
    wait_list(&mut owner_c, |_, w, _| names(w) == ["other.gcode"]);
    let log = service.file("admin.log");
    assert!(
        log.contains("job-removed\tcurvyoctl@test\tagain.gcode"),
        "{log}"
    );
    assert!(log.contains("run-next-set\tPhone\tmine.gcode"), "{log}");
}
