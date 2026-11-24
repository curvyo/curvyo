//! Independent acceptance tests for `specs/0118-machine-control` slice 2b (criteria 22, 26, 28 and
//! the member part of 12; ADR F2 to F5), written from the specification and `adrs.md` before
//! reading the implementation. The pure machine only: wire shapes are JSON, the clock is injected.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::format_collect,
    missing_docs
)]

use curvyo_connector_core::{
    Event, JobKind, Machine, Message, Options, Output, Peer, Role, StartRule,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use uuid::Uuid;

const JOB: &str = "G21\nG90\nG0 X0 Y0\nM4 S0\nG1 X10 Y10 F1000 S500\nM5\n";
const FRAME: &str = "; frame\nG21\nG90\nG17\nM5\nM9\nG1 X0 Y0 F3000 S0\nG1 X10 Y0 F3000 S0\n\
G1 X10 Y10 F3000 S0\nG1 X0 Y10 F3000 S0\nG1 X0 Y0 F3000 S0\n";

fn id(n: u128) -> Uuid {
    Uuid::from_u128(n)
}

fn sha_hex(text: &str) -> String {
    Sha256::digest(text.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn owner() -> Peer {
    Peer::new("Laptop", Role::Owner)
}

fn member() -> Peer {
    Peer::new("Phone", Role::Member)
}

fn dbg<T: std::fmt::Debug>(value: &T) -> String {
    format!("{value:?}")
}

fn command_names(out: &Output) -> Vec<String> {
    out.commands
        .iter()
        .map(|c| {
            dbg(c)
                .split(|ch: char| !ch.is_alphanumeric())
                .next()
                .unwrap_or("")
                .to_owned()
        })
        .collect()
}

fn says(out: &Output, needle: &str) -> bool {
    out.messages.iter().any(|m| dbg(m).contains(needle))
}

fn state(machine: &Machine) -> String {
    dbg(&machine.state())
}

fn status_json(machine: &Machine) -> Value {
    serde_json::to_value(machine.status()).unwrap()
}

fn wire(value: Value) -> Message {
    serde_json::from_value(value).expect("the wire shape")
}

fn jog(axis: &str, direction: &str, step: &str) -> Message {
    wire(json!({"type": "Jog", "axis": axis, "direction": direction, "step": step}))
}

fn merge(mut a: Output, b: Output) -> Output {
    a.messages.extend(b.messages);
    a.commands.extend(b.commands);
    a
}

/// A machine that has identified its controller, reports laser mode off and is `Idle`.
struct Rig {
    m: Machine,
}

impl Rig {
    fn idle() -> Self {
        let mut m = Machine::confirming(Options {
            start_rule: StartRule::OwnerConfirms,
        });
        m.handle(Event::ControllerConnected);
        m.handle(Event::LaserModeUnknown(false));
        m.handle(Event::LaserModeOff(false));
        assert_eq!(state(&m), "Idle");
        Self { m }
    }

    fn ev(&mut self, event: Event) -> Output {
        let out = self.m.handle(event);
        self.settle(out)
    }

    fn settle(&mut self, mut out: Output) -> Output {
        let mut next = out.follow_up.take();
        while let Some(event) = next {
            let mut more = self.m.handle(event);
            out.messages.append(&mut more.messages);
            out.commands.append(&mut more.commands);
            next = more.follow_up.take();
        }
        out
    }

    fn msg(&mut self, peer: &Peer, message: Message) -> Output {
        self.ev(Event::Message(message, peer.clone()))
    }

    fn submit_job(&mut self, n: u128, text: &str) -> Output {
        self.submit(n, JobKind::Job, text, &owner())
    }

    fn submit_frame(&mut self, n: u128, text: &str, peer: &Peer) -> Output {
        self.submit(n, JobKind::Frame, text, peer)
    }

    fn submit(&mut self, n: u128, kind: JobKind, text: &str, peer: &Peer) -> Output {
        let a = self.msg(
            peer,
            Message::SubmitJob {
                job_id: id(n),
                name: "bench.gcode".into(),
                kind,
                bytes: text.len() as u64,
                sha256: sha_hex(text),
            },
        );
        let b = self.msg(
            peer,
            Message::JobChunk {
                job_id: id(n),
                index: 0,
                text: text.to_owned(),
            },
        );
        merge(a, b)
    }

    /// A Ready job, held.
    fn ready(&mut self, n: u128) {
        self.submit_job(n, JOB);
        assert_eq!(state(&self.m), "Ready");
    }
}

// ---------------------------------------------------------------------------------------------
// Criterion 22: accepted in Idle, Ready and Done; it runs once and returns to where it started
// ---------------------------------------------------------------------------------------------

#[test]
fn a_frame_in_idle_runs_and_returns_to_idle() {
    let mut r = Rig::idle();
    let out = r.submit_frame(10, FRAME, &owner());
    assert!(
        !says(&out, "Busy"),
        "a frame in Idle is accepted: {:?}",
        out.messages
    );
    assert_eq!(
        command_names(&out).iter().filter(|c| *c == "Frame").count(),
        1,
        "{:?}",
        command_names(&out)
    );
    assert_eq!(state(&r.m), "Framing");
    let done = r.ev(Event::JobFinished { elapsed_ms: 800 });
    assert!(!says(&done, "Error"));
    assert_eq!(state(&r.m), "Idle", "the state it came from");
}

#[test]
fn a_ready_job_stays_held_through_a_frame_and_still_starts_afterwards() {
    let mut r = Rig::idle();
    r.ready(1);
    let out = r.submit_frame(20, FRAME, &owner());
    assert!(
        !says(&out, "Busy"),
        "a Ready job does not block a frame: {:?}",
        out.messages
    );
    assert_eq!(state(&r.m), "Framing");
    r.ev(Event::JobFinished { elapsed_ms: 800 });
    assert_eq!(
        state(&r.m),
        "Ready",
        "the frame returns to Ready and the job is held"
    );
    let start = r.msg(&owner(), Message::Start { job_id: id(1) });
    assert!(
        !says(&start, "Error"),
        "the held job starts: {:?}",
        start.messages
    );
    assert_eq!(
        command_names(&start)
            .iter()
            .filter(|c| *c == "Start")
            .count(),
        1,
        "{:?}",
        command_names(&start)
    );
    assert_eq!(state(&r.m), "Running");
}

#[test]
fn a_frame_in_done_returns_to_done() {
    let mut r = Rig::idle();
    r.ready(1);
    r.msg(&owner(), Message::Start { job_id: id(1) });
    r.ev(Event::JobFinished { elapsed_ms: 500 });
    assert!(state(&r.m).starts_with("Done"), "{}", state(&r.m));
    r.submit_frame(21, FRAME, &owner());
    assert_eq!(state(&r.m), "Framing");
    r.ev(Event::JobFinished { elapsed_ms: 800 });
    assert!(
        state(&r.m).starts_with("Done"),
        "a frame returns to Done: {}",
        state(&r.m)
    );
}

// ---------------------------------------------------------------------------------------------
// Criterion 22: a frame is never a job: not listed, no result, no RunNext
// ---------------------------------------------------------------------------------------------

#[test]
fn a_frame_never_appears_in_the_status_as_a_job() {
    let mut r = Rig::idle();
    r.submit_frame(30, FRAME, &owner());
    let s = status_json(&r.m);
    assert_eq!(
        s["frame_id"],
        json!(id(30).to_string()),
        "the frame is named in Status while it runs"
    );
    // The frame id is only in `frame_id`: no other field of Status carries it.
    let text = s.to_string();
    assert_eq!(text.matches(&id(30).to_string()).count(), 1, "{text}");
    r.ev(Event::JobFinished { elapsed_ms: 800 });
    let after = status_json(&r.m);
    assert!(
        after.get("frame_id").is_none() || after["frame_id"].is_null(),
        "frame_id is gone at rest: {after}"
    );
}

#[test]
fn frame_id_is_omitted_when_no_frame_is_running() {
    let r = Rig::idle();
    assert!(
        status_json(&r.m).get("frame_id").is_none(),
        "{}",
        status_json(&r.m)
    );
}

// ---------------------------------------------------------------------------------------------
// Criterion 22: Busy outside Idle, Ready and Done
// ---------------------------------------------------------------------------------------------

#[test]
fn a_frame_while_a_job_runs_or_is_paused_is_busy_and_no_command_is_given() {
    let mut r = Rig::idle();
    r.ready(1);
    r.msg(&owner(), Message::Start { job_id: id(1) });
    assert_eq!(state(&r.m), "Running");
    let out = r.submit_frame(40, FRAME, &owner());
    assert!(says(&out, "Busy"), "{:?}", out.messages);
    assert!(!command_names(&out).iter().any(|c| c == "Frame"));
    assert_eq!(state(&r.m), "Running");

    r.msg(&owner(), Message::Pause);
    r.ev(Event::Held);
    assert_eq!(state(&r.m), "Paused");
    let out = r.submit_frame(41, FRAME, &owner());
    assert!(says(&out, "Busy"), "paused is Busy: {:?}", out.messages);
    assert!(!command_names(&out).iter().any(|c| c == "Frame"));
}

#[test]
fn a_frame_while_a_job_is_being_received_is_busy() {
    let mut r = Rig::idle();
    r.msg(
        &owner(),
        Message::SubmitJob {
            job_id: id(2),
            name: "half.gcode".into(),
            kind: JobKind::Job,
            bytes: JOB.len() as u64,
            sha256: sha_hex(JOB),
        },
    );
    assert_eq!(state(&r.m), "Receiving");
    let out = r.submit_frame(42, FRAME, &owner());
    assert!(says(&out, "Busy"), "{:?}", out.messages);
    assert!(!command_names(&out).iter().any(|c| c == "Frame"));
}

#[test]
fn a_frame_in_alarm_is_busy_and_nothing_is_written() {
    let mut r = Rig::idle();
    r.ev(Event::Alarm {
        text: "ALARM:1".into(),
    });
    assert_eq!(state(&r.m), "Alarm");
    let out = r.submit_frame(43, FRAME, &owner());
    assert!(
        says(&out, "Busy") || says(&out, "Error"),
        "{:?}",
        out.messages
    );
    assert!(!command_names(&out).iter().any(|c| c == "Frame"));
    assert_eq!(state(&r.m), "Alarm");
}

// ---------------------------------------------------------------------------------------------
// Criterion 28 and F3: the frame's text holds only the laser-off words; any other line is NotAFrame
// ---------------------------------------------------------------------------------------------

#[test]
fn a_frame_text_with_a_laser_on_word_is_refused_as_not_a_frame() {
    let bad = [
        "G21\nG90\nM5\nG1 X10 Y0 F3000 S0\nM3 S500\nG1 X0 Y0 F3000 S0\n",
        "G21\nG90\nM5\nM4 S0\nG1 X10 Y0 F3000 S0\n",
        "G21\nG90\nM5\nG1 X10 Y0 F3000 S500\n",
        "G21\nG90\nM5\nG0 X10 Y0\n",
        "G21\nG90\nM5\nG1 X10 Y0 F3000 S0\n$H\n",
        "G21\nG90\nM5\nG10 L20 P1 X0 Y0\n",
        "G21\nG90\nM5\nG1 X10 Y0 F3000 S0 M8\n",
    ];
    for (n, text) in bad.iter().enumerate() {
        let mut r = Rig::idle();
        let out = r.submit_frame(50 + n as u128, text, &owner());
        assert!(
            says(&out, "NotAFrame"),
            "case {n} must be NotAFrame: {:?}\n{text}",
            out.messages
        );
        assert!(
            !command_names(&out).iter().any(|c| c == "Frame"),
            "case {n} gives no command"
        );
        assert_eq!(state(&r.m), "Idle", "case {n} leaves the state as it was");
    }
}

// ---------------------------------------------------------------------------------------------
// Criterion 10 and 22: during a frame, the other motion commands and Start are refused
// ---------------------------------------------------------------------------------------------

#[test]
fn during_a_frame_jog_home_start_pause_and_resume_are_refused_with_wrong_state() {
    let mut r = Rig::idle();
    r.ready(1);
    r.submit_frame(60, FRAME, &owner());
    assert_eq!(state(&r.m), "Framing");

    let jogged = r.msg(&owner(), jog("X", "Plus", "1"));
    assert!(says(&jogged, "WrongState"), "{:?}", jogged.messages);
    assert!(!command_names(&jogged).iter().any(|c| c.starts_with("Jog")));

    let home = r.msg(&owner(), Message::Home);
    assert!(says(&home, "WrongState"), "{:?}", home.messages);
    assert!(!command_names(&home).iter().any(|c| c == "Home"));

    let start = r.msg(&owner(), Message::Start { job_id: id(1) });
    assert!(says(&start, "WrongState"), "{:?}", start.messages);
    assert!(!command_names(&start).iter().any(|c| c == "Start"));

    let again = r.submit_job(2, JOB);
    assert!(
        says(&again, "Busy"),
        "a job cannot be sent during a frame: {:?}",
        again.messages
    );
    assert_eq!(state(&r.m), "Framing");
}

// ---------------------------------------------------------------------------------------------
// Criterion 10: Stop mid-frame returns to the state it came from with the job kept
// ---------------------------------------------------------------------------------------------

#[test]
fn stop_during_a_frame_gives_the_stop_command_and_ends_back_at_ready_with_the_job_kept() {
    let mut r = Rig::idle();
    r.ready(1);
    r.submit_frame(70, FRAME, &owner());
    let stop = r.msg(&owner(), Message::Stop);
    assert!(
        command_names(&stop).iter().any(|c| c == "Stop"),
        "{:?}",
        command_names(&stop)
    );
    r.ev(Event::JobFailed { elapsed_ms: 120 });
    assert_eq!(state(&r.m), "Ready", "the Ready job is kept");
    let start = r.msg(&owner(), Message::Start { job_id: id(1) });
    assert!(
        !says(&start, "Error"),
        "the kept job still starts: {:?}",
        start.messages
    );
}

#[test]
fn stop_during_a_frame_from_idle_returns_to_idle() {
    let mut r = Rig::idle();
    r.submit_frame(71, FRAME, &owner());
    r.msg(&owner(), Message::Stop);
    r.ev(Event::JobFailed { elapsed_ms: 120 });
    assert_eq!(state(&r.m), "Idle");
}

#[test]
fn a_frame_that_fails_on_the_controller_returns_to_its_state_with_the_detail() {
    let mut r = Rig::idle();
    r.ready(1);
    r.submit_frame(72, FRAME, &owner());
    r.ev(Event::JobFailedWith {
        elapsed_ms: 300,
        detail: "Outside the travel by 4 mm".into(),
    });
    assert_eq!(state(&r.m), "Ready", "a failed frame returns to Ready");
    assert!(
        !status_json(&r.m).to_string().contains("\"Done\""),
        "no result for a frame"
    );
}

// ---------------------------------------------------------------------------------------------
// Criterion 12 (members) and F5: a member's frame waits for an owner
// ---------------------------------------------------------------------------------------------

#[test]
fn a_members_frame_waits_for_an_owner_with_requested_by_and_no_job() {
    let mut r = Rig::idle();
    let out = r.submit_frame(80, FRAME, &member());
    assert!(
        !command_names(&out).iter().any(|c| c == "Frame"),
        "nothing is run yet"
    );
    assert_eq!(state(&r.m), "AwaitingLocalStart");
    let s = status_json(&r.m);
    assert_eq!(s["frame_id"], json!(id(80).to_string()), "{s}");
    assert!(
        s.to_string().contains("Phone"),
        "requested_by names the member: {s}"
    );
}

#[test]
fn a_member_cannot_confirm_its_own_frame_and_the_owner_does() {
    let mut r = Rig::idle();
    r.submit_frame(81, FRAME, &member());
    let own_try = r.msg(&member(), Message::Start { job_id: id(81) });
    assert!(says(&own_try, "WrongState"), "{:?}", own_try.messages);
    assert!(!command_names(&own_try).iter().any(|c| c == "Frame"));
    assert_eq!(state(&r.m), "AwaitingLocalStart");

    let confirm = r.msg(&owner(), Message::Start { job_id: id(81) });
    assert!(
        command_names(&confirm).iter().any(|c| c == "Frame"),
        "{:?}",
        command_names(&confirm)
    );
    assert_eq!(state(&r.m), "Framing");
}

#[test]
fn an_owners_own_frame_press_is_not_a_confirmation_of_a_members_frame() {
    let mut r = Rig::idle();
    r.submit_frame(82, FRAME, &member());
    let own_frame = r.submit_frame(83, FRAME, &owner());
    assert!(
        !command_names(&own_frame).iter().any(|c| c == "Frame") || state(&r.m) != "Framing",
        "the member's frame still waits: {:?}",
        own_frame.messages
    );
    assert_eq!(state(&r.m), "AwaitingLocalStart");
}

#[test]
fn stop_from_anyone_discards_a_waiting_members_frame() {
    let mut r = Rig::idle();
    r.submit_frame(84, FRAME, &member());
    r.msg(&owner(), Message::Stop);
    assert_eq!(state(&r.m), "Idle", "the waiting frame is discarded");
    assert!(status_json(&r.m).get("frame_id").is_none());
}

#[test]
fn a_frame_is_refused_as_busy_while_a_members_frame_waits() {
    let mut r = Rig::idle();
    r.submit_frame(85, FRAME, &member());
    let out = r.submit_frame(86, FRAME, &member());
    assert!(says(&out, "Busy"), "{:?}", out.messages);
    assert_eq!(state(&r.m), "AwaitingLocalStart");
}
