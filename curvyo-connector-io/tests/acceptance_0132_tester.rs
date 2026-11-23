//! Independent acceptance tests for `specs/0132-connector-job-queue` at the socket and on disk,
//! written from the specification and `adrs.md` before reading the implementation. They run the
//! connector server in-process with the fake controller in a state directory of its own; restart
//! is a shutdown and a new server on the same directory. Waits are by condition with a deadline;
//! "nothing arrives" checks use a short quiet window.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::format_collect,
    clippy::option_option
)]

mod common;

use std::time::{Duration, Instant};

use common::service::{Service, deadline, service, service_config};
use common::{wait_for, wait_state};
use curvyo_connector_core::{ErrorCode, JobKind, JobResult, Message, RejectReason, Role, State};
use curvyo_connector_io::client::{CONNECT_TIMEOUT, Connection, open_paired};
use curvyo_connector_io::server::RunningServer;
use curvyo_connector_io::token::Token;
use sha2::{Digest, Sha256};
use uuid::Uuid;

const JOB: &str = "G21\nG90\nG0 X0 Y0\nM4 S0\nG1 X10 Y10 F1000 S500\nM5\n";

/// A job that runs for minutes at one line per second: it stays current while a test works.
fn long_job() -> String {
    "G1 X1 Y1 F1000 S500\n".repeat(300)
}

fn uid(n: u128) -> Uuid {
    Uuid::from_u128(n)
}

fn sha_hex(text: &str) -> String {
    Sha256::digest(text.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Submits a job and its one chunk; the connector's answer for it (`JobAccepted` or
/// `JobRejected`).
fn submit(conn: &mut Connection, job: Uuid, name: &str, text: &str, kind: JobKind) -> Message {
    conn.send(&Message::SubmitJob {
        job_id: job,
        name: name.to_owned(),
        kind,
        bytes: text.len() as u64,
        sha256: sha_hex(text),
    })
    .unwrap();
    conn.send(&Message::JobChunk {
        job_id: job,
        index: 0,
        text: text.to_owned(),
    })
    .unwrap();
    let until = deadline();
    loop {
        let m = conn.recv_message(until).unwrap();
        match &m {
            Message::JobAccepted { job_id, .. } | Message::JobRejected { job_id, .. }
                if *job_id == job =>
            {
                return m;
            }
            _ => {}
        }
    }
}

fn accepted(answer: &Message) -> Option<Option<u32>> {
    match answer {
        Message::JobAccepted { position, .. } => Some(*position),
        _ => None,
    }
}

/// Every message that arrives in the next `window`; a quiet window ends the read.
fn drain(conn: &mut Connection, window: Duration) -> Vec<Message> {
    let mut out = Vec::new();
    while let Ok(m) = conn.recv_message(Instant::now() + window) {
        out.push(m);
    }
    out
}

/// The last `JobList` among `messages`.
fn last_list(messages: &[Message]) -> Option<Message> {
    messages
        .iter()
        .rev()
        .find(|m| matches!(m, Message::JobList { .. }))
        .cloned()
}

fn waiting_ids(list: &Message) -> Vec<Uuid> {
    match list {
        Message::JobList { waiting, .. } => waiting.iter().map(|e| e.job_id).collect(),
        other => panic!("not a JobList: {other:?}"),
    }
}

fn current_id(list: &Message) -> Option<Uuid> {
    match list {
        Message::JobList { current, .. } => current.as_ref().map(|c| c.job_id),
        other => panic!("not a JobList: {other:?}"),
    }
}

fn is_error(messages: &[Message], code: ErrorCode) -> bool {
    messages
        .iter()
        .any(|m| matches!(m, Message::Error { code: c, .. } if *c == code))
}

/// A connector with an owner (the setup connection) and a member `Phone`.
fn two_clients() -> (Service, Connection, Token, Connection) {
    let s = service();
    let (mut owner, owner_token) = s.setup(1);
    let (_, phone) = s.pair(&mut owner, "Phone", Role::Member);
    (s, owner, owner_token, phone)
}

/// Shuts the connector down and starts a new one on the same state directory.
fn restart(s: Service) -> Service {
    let Service { server, dir, .. } = s;
    server.shutdown();
    start_on(dir)
}

fn start_on(dir: tempfile::TempDir) -> Service {
    let server =
        RunningServer::start_service(&service_config(dir.path(), Duration::from_secs(1800)))
            .unwrap();
    let notices = server.take_notices().unwrap();
    Service {
        server,
        dir,
        notices,
    }
}

/// Owner's job 1 (long, running), member's job 2 and owner's job 3 waiting.
fn queue_of_three() -> (Service, Connection, Connection) {
    let (s, mut owner, _, mut phone) = two_clients();
    submit(&mut owner, uid(1), "a.gcode", &long_job(), JobKind::Job);
    owner.send(&Message::Start { job_id: uid(1) }).unwrap();
    wait_state(&mut owner, State::Running);
    submit(&mut phone, uid(2), "b.gcode", JOB, JobKind::Job);
    submit(&mut owner, uid(3), "c.gcode", JOB, JobKind::Job);
    (s, owner, phone)
}

// ---------------------------------------------------------------------------------------------
// Criterion 1: a send while busy is added to the list; Busy still refuses frames and Home.
// ---------------------------------------------------------------------------------------------

#[test]
fn c1_a_send_while_the_laser_works_is_listed_with_position_and_a_frame_is_still_busy() {
    let (_s, mut owner, _, mut phone) = two_clients();
    let a = accepted(&submit(
        &mut owner,
        uid(1),
        "a.gcode",
        &long_job(),
        JobKind::Job,
    ));
    assert_eq!(a, Some(None), "the first job becomes current");
    owner.send(&Message::Start { job_id: uid(1) }).unwrap();
    wait_state(&mut owner, State::Running);

    let b = submit(&mut phone, uid(2), "b.gcode", JOB, JobKind::Job);
    assert_eq!(accepted(&b), Some(Some(1)), "{b:?}");
    let c = submit(&mut owner, uid(3), "c.gcode", JOB, JobKind::Job);
    assert_eq!(accepted(&c), Some(Some(2)), "{c:?}");

    let frame = submit(&mut owner, uid(4), "frame.gcode", JOB, JobKind::Frame);
    assert!(
        matches!(
            frame,
            Message::JobRejected {
                reason: RejectReason::Busy,
                ..
            }
        ),
        "{frame:?}"
    );
    owner.send(&Message::Home).unwrap();
    let home = drain(&mut owner, Duration::from_millis(300));
    assert!(
        !home.iter().any(|m| matches!(
            m,
            Message::Status {
                state: State::Homing,
                ..
            }
        )),
        "Home does not start while a job runs: {home:?}"
    );
}

// ---------------------------------------------------------------------------------------------
// Criterion 2: the list comes on connect, before the first Status.
// ---------------------------------------------------------------------------------------------

#[test]
fn c2_a_list_is_sent_on_connect_before_the_first_status() {
    let (s, mut owner, _, mut phone) = two_clients();
    submit(&mut owner, uid(1), "a.gcode", &long_job(), JobKind::Job);
    owner.send(&Message::Start { job_id: uid(1) }).unwrap();
    wait_state(&mut owner, State::Running);
    submit(&mut phone, uid(2), "b.gcode", JOB, JobKind::Job);

    // A fresh member connection: the client's login keeps a JobList that came before the first
    // Status (`Connection::job_list`).
    let token = s.pair(&mut owner, "Second", Role::Member).0;
    let (app, first) = open_paired(&s.address(), s.server.spki(), &token, CONNECT_TIMEOUT).unwrap();
    assert!(matches!(first, Message::Status { .. }), "{first:?}");
    let list = app
        .job_list
        .clone()
        .unwrap_or_else(|| panic!("no JobList before the first Status"));
    assert_eq!(waiting_ids(&list), vec![uid(2)]);
    assert_eq!(current_id(&list), Some(uid(1)));
    drop(phone);
}

// ---------------------------------------------------------------------------------------------
// Criteria 3 and 5 over the socket: owner moves, stale positions, member rights.
// ---------------------------------------------------------------------------------------------

#[test]
fn c3_the_owner_moves_a_job_for_everyone_and_a_stale_position_is_refused_with_a_list() {
    let (_s, mut owner, mut phone) = queue_of_three();
    owner
        .send(&Message::MoveJob {
            job_id: uid(3),
            from: 2,
            to: 1,
        })
        .unwrap();
    let got = drain(&mut owner, Duration::from_millis(400));
    assert_eq!(
        waiting_ids(&last_list(&got).expect("a JobList after the move")),
        vec![uid(3), uid(2)],
        "{got:?}"
    );
    let seen = drain(&mut phone, Duration::from_millis(400));
    assert_eq!(
        waiting_ids(&last_list(&seen).expect("the member sees the change")),
        vec![uid(3), uid(2)],
        "the order changes for all clients at once"
    );

    // Job 2 is now at position 2; a client that still thinks it is at 1 is stale.
    owner
        .send(&Message::MoveJob {
            job_id: uid(2),
            from: 1,
            to: 2,
        })
        .unwrap();
    let stale = drain(&mut owner, Duration::from_millis(400));
    assert!(is_error(&stale, ErrorCode::WrongState), "{stale:?}");
    assert_eq!(
        waiting_ids(&last_list(&stale).expect("a fresh JobList")),
        vec![uid(3), uid(2)],
        "the refused move changes nothing"
    );
}

#[test]
fn c5_a_member_gets_owner_required_for_move_and_others_removal_and_keeps_its_own() {
    let (_s, _owner, mut phone) = queue_of_three();
    phone
        .send(&Message::MoveJob {
            job_id: uid(2),
            from: 1,
            to: 2,
        })
        .unwrap();
    let mv = drain(&mut phone, Duration::from_millis(300));
    assert!(is_error(&mv, ErrorCode::OwnerRequired), "{mv:?}");

    phone.send(&Message::RemoveJob { job_id: uid(3) }).unwrap();
    let others = drain(&mut phone, Duration::from_millis(300));
    assert!(is_error(&others, ErrorCode::OwnerRequired), "{others:?}");

    phone.send(&Message::RemoveJob { job_id: uid(2) }).unwrap();
    let own = drain(&mut phone, Duration::from_millis(300));
    assert!(
        last_list(&own).is_some_and(|l| waiting_ids(&l) == vec![uid(3)]),
        "a member removes its own waiting job: {own:?}"
    );
}

// ---------------------------------------------------------------------------------------------
// Criterion 4 over the socket: the first waiting job starts on the owner's Start after Done;
// a member's Start waits for an owner.
// ---------------------------------------------------------------------------------------------

#[test]
fn c4_queued_jobs_run_in_order_with_the_owner_start_and_a_members_start_waits() {
    let (_s, mut owner, _, mut phone) = two_clients();
    // Sent while job 1 is only Ready (held, not started): 2 and 3 wait behind it.
    assert_eq!(
        accepted(&submit(&mut owner, uid(1), "a.gcode", JOB, JobKind::Job)),
        Some(None)
    );
    assert_eq!(
        accepted(&submit(&mut phone, uid(2), "b.gcode", JOB, JobKind::Job)),
        Some(Some(1))
    );
    assert_eq!(
        accepted(&submit(&mut owner, uid(3), "c.gcode", JOB, JobKind::Job)),
        Some(Some(2))
    );
    owner.send(&Message::Start { job_id: uid(1) }).unwrap();
    wait_for(&mut owner, |m| {
        matches!(
            m,
            Message::Status {
                state: State::Done { .. },
                ..
            }
        )
    });

    phone.send(&Message::Start { job_id: uid(2) }).unwrap();
    wait_state(&mut phone, State::AwaitingLocalStart);
    // Job 2 is now the held job; job 3 is not the first waiting one any more.
    owner.send(&Message::Start { job_id: uid(3) }).unwrap();
    let refused = drain(&mut owner, Duration::from_millis(300));
    assert!(
        is_error(&refused, ErrorCode::WrongState) || is_error(&refused, ErrorCode::UnknownJob),
        "a waiting job other than the first does not start: {refused:?}"
    );
    owner.send(&Message::Start { job_id: uid(2) }).unwrap();
    wait_for(&mut owner, |m| {
        matches!(
            m,
            Message::Status { job: Some(p), state: State::Running | State::Done { .. }, .. }
                if p.job_id == uid(2)
        )
    });
}

// ---------------------------------------------------------------------------------------------
// Criterion 7: Stop stops only the current job; Reset deletes the list.
// ---------------------------------------------------------------------------------------------

#[test]
fn c7_stop_ends_the_current_job_and_the_list_keeps_the_rest() {
    let (_s, mut owner, _, _phone) = two_clients();
    submit(&mut owner, uid(1), "a.gcode", &long_job(), JobKind::Job);
    owner.send(&Message::Start { job_id: uid(1) }).unwrap();
    wait_state(&mut owner, State::Running);
    submit(&mut owner, uid(2), "b.gcode", JOB, JobKind::Job);
    submit(&mut owner, uid(3), "c.gcode", JOB, JobKind::Job);
    owner.send(&Message::Stop).unwrap();
    wait_state(
        &mut owner,
        State::Done {
            result: JobResult::Stopped,
        },
    );
    // The rest is kept: the owner's Start runs job 2, the first waiting one.
    owner.send(&Message::Start { job_id: uid(2) }).unwrap();
    wait_for(
        &mut owner,
        |m| matches!(m, Message::Status { job: Some(p), .. } if p.job_id == uid(2)),
    );
}

/// The same Reset, but after the send has had time to reach the disk: isolates a race between
/// Reset and the writer of the job text and `list.json`.
#[test]
fn c7_reset_after_the_send_has_settled_deletes_the_list() {
    let (s, mut owner, _, _phone) = two_clients();
    submit(&mut owner, uid(2), "b.gcode", JOB, JobKind::Job);
    drain(&mut owner, Duration::from_millis(500));
    let answer = owner
        .request(
            &Message::Reset {
                connector_name: "workshop-laser".into(),
            },
            deadline(),
        )
        .unwrap();
    assert!(matches!(answer, Message::ResetDone), "{answer:?}");
    let exits = s.server.take_exits().unwrap();
    assert!(exits.recv_timeout(common::WAIT).is_ok());
    let Service { server, dir, .. } = s;
    server.wipe().unwrap();
    // Nothing is rewritten once the wipe returned: the writer thread is gone with the server.
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
    let until = Instant::now() + common::WAIT;
    let names = || -> Vec<String> {
        std::fs::read_dir(dir.path().join("jobs"))
            .map(|d| {
                d.map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
                    .collect()
            })
            .unwrap_or_default()
    };
    while !names().is_empty() && Instant::now() < until {
        std::thread::yield_now();
    }
    assert!(names().is_empty(), "{:?}", names());
}

#[test]
fn c7_reset_deletes_the_list_with_the_state_directory() {
    let (s, mut owner, _, _phone) = two_clients();
    submit(&mut owner, uid(2), "b.gcode", JOB, JobKind::Job);
    let answer = owner
        .request(
            &Message::Reset {
                connector_name: "workshop-laser".into(),
            },
            deadline(),
        )
        .unwrap();
    assert!(matches!(answer, Message::ResetDone), "{answer:?}");
    let exits = s.server.take_exits().unwrap();
    assert!(exits.recv_timeout(common::WAIT).is_ok());
    let Service { server, dir, .. } = s;
    server.wipe().unwrap();
    // Nothing is rewritten once the wipe returned: the writer thread is gone with the server.
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
    // The deletion may finish a moment after the answer; wait by condition, up to the deadline.
    let until = Instant::now() + common::WAIT;
    let names = || -> Vec<String> {
        std::fs::read_dir(dir.path().join("jobs"))
            .map(|d| {
                d.map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
                    .collect()
            })
            .unwrap_or_default()
    };
    while !names().is_empty() && Instant::now() < until {
        std::thread::yield_now();
    }
    let left = names();
    assert!(
        left.is_empty(),
        "the job texts and the list are deleted with the state directory: {left:?} list.json={:?}",
        std::fs::read_to_string(dir.path().join("jobs").join("list.json")).ok()
    );
}

// ---------------------------------------------------------------------------------------------
// Criterion 6: restart. Order, senders, interrupted job, corrupt job, limits.
// ---------------------------------------------------------------------------------------------

#[test]
fn c6_after_a_restart_the_waiting_jobs_keep_order_and_sender_and_the_running_one_is_interrupted() {
    let (s, mut owner, owner_token, mut phone) = two_clients();
    submit(&mut owner, uid(1), "a.gcode", &long_job(), JobKind::Job);
    owner.send(&Message::Start { job_id: uid(1) }).unwrap();
    wait_state(&mut owner, State::Running);
    submit(&mut phone, uid(2), "b.gcode", JOB, JobKind::Job);
    submit(&mut owner, uid(3), "c.gcode", JOB, JobKind::Job);
    drop(owner);
    drop(phone);

    let s = restart(s);
    let (mut owner, first) =
        open_paired(&s.address(), s.server.spki(), &owner_token, CONNECT_TIMEOUT).unwrap();
    assert!(matches!(first, Message::Status { .. }), "{first:?}");
    let list = owner
        .job_list
        .clone()
        .expect("a JobList before the first Status");
    let Message::JobList {
        current, waiting, ..
    } = &list
    else {
        unreachable!()
    };
    assert_eq!(current, &None, "nothing is current after a restart");
    let ids: Vec<Uuid> = waiting.iter().map(|e| e.job_id).collect();
    assert_eq!(
        ids,
        vec![uid(1), uid(2), uid(3)],
        "the same order, the interrupted job listed first"
    );
    let interrupted = waiting.iter().find(|e| e.job_id == uid(1)).unwrap();
    assert!(interrupted.interrupted_at.is_some(), "{interrupted:?}");
    let sender = waiting.iter().find(|e| e.job_id == uid(2)).unwrap();
    assert_eq!(sender.sent_by, "Phone");

    // Nothing starts by itself, and the interrupted job cannot be started by Start.
    let quiet = drain(&mut owner, Duration::from_millis(500));
    assert!(
        quiet.iter().all(|m| !matches!(
            m,
            Message::Status {
                state: State::Running | State::Ready,
                ..
            }
        )),
        "nothing starts after a restart: {quiet:?}"
    );
    owner.send(&Message::Start { job_id: uid(1) }).unwrap();
    let refused = drain(&mut owner, Duration::from_millis(300));
    assert!(is_error(&refused, ErrorCode::WrongState), "{refused:?}");
}

#[test]
fn c6_a_corrupt_stored_job_is_dropped_with_an_admin_line_and_the_rest_stays() {
    let (s, mut owner, owner_token, mut phone) = two_clients();
    submit(&mut owner, uid(1), "a.gcode", &long_job(), JobKind::Job);
    owner.send(&Message::Start { job_id: uid(1) }).unwrap();
    wait_state(&mut owner, State::Running);
    submit(&mut phone, uid(2), "b.gcode", JOB, JobKind::Job);
    submit(&mut owner, uid(3), "c.gcode", JOB, JobKind::Job);
    drop(owner);
    drop(phone);

    // Corrupt the stored text of job 2 (same length, other bytes) while the connector is down.
    let Service { server, dir, .. } = s;
    server.shutdown();
    let path = dir.path().join("jobs").join(format!("{}.gcode", uid(2)));
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("no stored text at {path:?}: {e}"));
    std::fs::write(&path, "X".repeat(text.len())).unwrap();

    let s = start_on(dir);
    let (owner, first) =
        open_paired(&s.address(), s.server.spki(), &owner_token, CONNECT_TIMEOUT).unwrap();
    assert!(matches!(first, Message::Status { .. }), "{first:?}");
    let list = owner
        .job_list
        .clone()
        .expect("a JobList before the first Status");
    assert_eq!(
        waiting_ids(&list),
        vec![uid(1), uid(3)],
        "job 2 dropped, the rest kept"
    );
    let admin = s.file("admin.log");
    assert!(admin.contains("job-dropped"), "{admin}");
    assert!(admin.contains("b.gcode"), "{admin}");
}

#[test]
fn c6_the_list_takes_20_waiting_jobs_and_the_21st_is_list_full() {
    let (_s, mut owner, _, _phone) = two_clients();
    submit(&mut owner, uid(1), "a.gcode", &long_job(), JobKind::Job);
    owner.send(&Message::Start { job_id: uid(1) }).unwrap();
    wait_state(&mut owner, State::Running);
    for n in 2..=21u128 {
        let answer = submit(&mut owner, uid(n), "w.gcode", JOB, JobKind::Job);
        assert!(
            accepted(&answer).is_some(),
            "waiting job {} of 20 refused: {answer:?}",
            n - 1
        );
    }
    let over = submit(&mut owner, uid(99), "over.gcode", JOB, JobKind::Job);
    assert!(
        matches!(
            over,
            Message::JobRejected {
                reason: RejectReason::ListFull,
                ..
            }
        ),
        "{over:?}"
    );
}
