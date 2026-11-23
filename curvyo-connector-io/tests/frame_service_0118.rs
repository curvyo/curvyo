//! `specs/0118` slice 2b through the running service with the fake controller (adrs F5): a
//! member's frame waits for an owner, the owner's Start with the frame's id confirms it, a Stop
//! discards a waiting one, and the logs name Frame. Waiting is by status, never by sleep.
#![allow(clippy::unwrap_used)]

mod common;

use std::fmt::Write as _;
use std::time::{Duration, Instant};

use common::service::{self, code_of};
use common::{WAIT, wait_for};
use curvyo_connector_core::{JobKind, Message, Role, State, Uuid};
use sha2::{Digest, Sha256};

const FRAME: &str = "G21\nG90\nM5\nM9\nG1 X5.000 Y0.000 F3000 S0\nG1 X0.000 Y0.000 F3000 S0\n";

fn id() -> Uuid {
    Uuid::from_u128(0xf4a3e)
}

fn sha(text: &str) -> String {
    Sha256::digest(text.as_bytes())
        .iter()
        .fold(String::new(), |mut s, b| {
            let _ = write!(s, "{b:02x}");
            s
        })
}

fn submit() -> Message {
    Message::SubmitJob {
        job_id: id(),
        name: "frame".into(),
        kind: JobKind::Frame,
        bytes: FRAME.len() as u64,
        sha256: sha(FRAME),
    }
}

fn chunk() -> Message {
    Message::JobChunk {
        job_id: id(),
        index: 0,
        text: FRAME.to_owned(),
    }
}

fn waiting_for(frame: Uuid) -> impl Fn(&Message) -> bool {
    move |m| {
        matches!(
            m,
            Message::Status {
                state: State::AwaitingLocalStart,
                job: None,
                requested_by: Some(who),
                frame_id: Some(found),
                ..
            } if who == "Phone" && *found == frame
        )
    }
}

fn wait_file(svc: &service::Service, name: &str, needle: &str) -> String {
    let until = Instant::now() + WAIT;
    loop {
        let text = svc.file(name);
        if text.contains(needle) || Instant::now() > until {
            return text;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn a_members_frame_waits_and_the_owners_start_with_its_id_runs_it() {
    let svc = service::service();
    let (mut setup, token) = svc.setup(4);
    let mut owner = svc.login(&token);
    let (_t, mut member) = svc.pair(&mut setup, "Phone", Role::Member);

    member.send(&submit()).unwrap();
    member.send(&chunk()).unwrap();
    wait_for(&mut owner, waiting_for(id()));
    // A member cannot confirm its own frame.
    member.send(&Message::Start { job_id: id() }).unwrap();
    let until = Instant::now() + WAIT;
    let refused = loop {
        let message = member.recv_message(until).unwrap();
        if matches!(message, Message::Error { .. }) {
            break message;
        }
    };
    assert_eq!(
        code_of(&refused),
        Some(curvyo_connector_core::ErrorCode::WrongState)
    );

    owner.send(&Message::Start { job_id: id() }).unwrap();
    wait_for(&mut owner, |m| {
        matches!(
            m,
            Message::Status {
                state: State::Framing,
                frame_id: Some(_),
                ..
            }
        )
    });
    wait_for(&mut owner, |m| {
        matches!(
            m,
            Message::Status {
                state: State::Idle,
                frame_id: None,
                ..
            }
        )
    });
    let admin = wait_file(&svc, "admin.log", "start-confirmed");
    assert!(
        admin
            .lines()
            .any(|l| l.contains("start-confirmed") && l.contains("Frame")),
        "admin.log names Frame: {admin:?}"
    );
    let jobs = wait_file(&svc, "jobs.log", "Frame: Phone");
    assert!(jobs.contains("Frame: Phone"), "{jobs:?}");
    assert!(
        !jobs.contains("Completed"),
        "a frame has no result line: {jobs:?}"
    );
}

#[test]
fn an_owners_stop_discards_a_waiting_frame_which_never_runs() {
    let svc = service::service();
    let (mut setup, token) = svc.setup(4);
    let mut owner = svc.login(&token);
    let (_t, mut member) = svc.pair(&mut setup, "Phone", Role::Member);

    member.send(&submit()).unwrap();
    member.send(&chunk()).unwrap();
    wait_for(&mut owner, waiting_for(id()));
    owner.send(&Message::Stop).unwrap();
    wait_for(&mut owner, |m| {
        matches!(
            m,
            Message::Status {
                state: State::Idle,
                frame_id: None,
                ..
            }
        )
    });
    let jobs = svc.file("jobs.log");
    assert!(!jobs.contains("Frame: "), "it never ran: {jobs:?}");
}
