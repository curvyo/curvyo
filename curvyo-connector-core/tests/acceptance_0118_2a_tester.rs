//! Independent acceptance tests for `specs/0118-machine-control` second slice 2a (criteria 8 to 21
//! and 37), written from the specification and `adrs.md` J1 to J9 before reading the
//! implementation. The pure machine only: the clock is the injected `Event::Clock`, no I/O. Wire
//! shapes are written as JSON of J1, so the tests do not depend on Rust field types.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::format_collect,
    missing_docs
)]

use std::time::Duration;

use curvyo_connector_core::{
    Event, JobKind, Machine, Message, Mode, Options, Output, Peer, Role, StartRule, admit,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use uuid::Uuid;

const JOB: &str = "G21\nG90\nG0 X0 Y0\nM4 S0\nG1 X10 Y10 F1000 S500\nM5\n";

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

fn ms(n: u64) -> Event {
    Event::Clock(Duration::from_millis(n))
}

/// The wire message of J1 (a Jog step, a held Jog, a keep-alive, a release).
fn wire(value: Value) -> Message {
    serde_json::from_value(value).expect("the wire shape of J1")
}

fn jog(axis: &str, direction: &str, step: &str) -> Message {
    wire(json!({"type": "Jog", "axis": axis, "direction": direction, "step": step}))
}

fn hold(axis: &str, direction: &str, step: &str) -> Message {
    wire(json!({"type": "Jog", "axis": axis, "direction": direction, "step": step, "hold": true}))
}

fn keep() -> Message {
    wire(json!({"type": "JogKeep"}))
}

fn jog_stop() -> Message {
    wire(json!({"type": "JogStop"}))
}

fn dbg<T: std::fmt::Debug>(value: &T) -> String {
    format!("{value:?}")
}

/// The name of each command (its first word): `Jog`, `JogCancel`, `Home`, and so on.
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

/// A command that moves the head (a step or a held jog), not the cancel.
fn is_move(name: &str) -> bool {
    name.starts_with("Jog") && name != "JogCancel"
}

fn moves(out: &Output) -> usize {
    command_names(out).iter().filter(|n| is_move(n)).count()
}

fn cancels(out: &Output) -> usize {
    command_names(out)
        .iter()
        .filter(|n| *n == "JogCancel")
        .count()
}

/// Whether a message of `out` (its debug form) contains `needle`.
fn says(out: &Output, needle: &str) -> bool {
    out.messages.iter().any(|m| dbg(m).contains(needle))
}

fn state(machine: &Machine) -> String {
    dbg(&machine.state())
}

fn status(machine: &Machine) -> String {
    dbg(&machine.status())
}

fn merge(mut a: Output, b: Output) -> Output {
    a.messages.extend(b.messages);
    a.commands.extend(b.commands);
    a
}

/// A machine that has identified its controller, reports its laser mode and is `Idle`.
struct Rig {
    m: Machine,
}

impl Rig {
    fn idle() -> Self {
        // Configured mode is the owner rule (ADR 0071); the real machine confirms.
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

    /// Submits `text` as job `n` and sends its one chunk: the job is `Ready`.
    fn submit(&mut self, n: u128, text: &str) -> Output {
        let a = self.msg(
            &owner(),
            Message::SubmitJob {
                job_id: id(n),
                name: "bench.gcode".into(),
                kind: JobKind::Job,
                bytes: text.len() as u64,
                sha256: sha_hex(text),
            },
        );
        let b = self.msg(
            &owner(),
            Message::JobChunk {
                job_id: id(n),
                index: 0,
                text: text.to_owned(),
            },
        );
        merge(a, b)
    }
}

// ---------------------------------------------------------------------------------------------
// Criteria 8, 11, 19, 20: which states accept a motion command
// ---------------------------------------------------------------------------------------------

#[test]
fn idle_accepts_a_jog_step_and_the_pad_reads_jogging() {
    let mut r = Rig::idle();
    let out = r.msg(&owner(), jog("X", "Plus", "1"));
    assert_eq!(moves(&out), 1, "{:?}", command_names(&out));
    assert!(!says(&out, "Error"), "{:?}", out.messages);
    assert_eq!(state(&r.m), "Jogging");
}

#[test]
fn ready_accepts_a_jog_and_the_job_stays_ready_after_the_jog() {
    let mut r = Rig::idle();
    r.submit(1, JOB);
    assert_eq!(state(&r.m), "Ready");
    let out = r.msg(&owner(), jog("Y", "Minus", "10"));
    assert_eq!(moves(&out), 1, "{:?}", command_names(&out));
    assert!(!says(&out, "Error"));
    r.ev(Event::JogEnded);
    assert_eq!(state(&r.m), "Ready", "a jog must never discard a Ready job");
}

#[test]
fn receiving_refuses_a_jog_with_wrong_state_and_no_command() {
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
    let out = r.msg(&owner(), jog("X", "Plus", "1"));
    assert_eq!(moves(&out), 0, "{:?}", command_names(&out));
    assert!(says(&out, "WrongState"), "{:?}", out.messages);
    assert_eq!(state(&r.m), "Receiving");
}

#[test]
fn homing_refuses_a_jog_and_writes_nothing() {
    let mut r = Rig::idle();
    r.msg(&owner(), Message::Home);
    assert_eq!(state(&r.m), "Homing");
    let out = r.msg(&owner(), jog("X", "Minus", "1"));
    assert_eq!(moves(&out), 0, "{:?}", command_names(&out));
    assert!(says(&out, "WrongState"), "{:?}", out.messages);
}

#[test]
fn no_controller_refuses_a_jog() {
    let mut m = Machine::new(Options::default());
    assert_eq!(state(&m), "NoController");
    let out = m.handle(Event::Message(jog("X", "Plus", "1"), owner()));
    assert!(
        !command_names(&out).iter().any(|n| is_move(n)),
        "{:?}",
        out.commands
    );
    assert!(says(&out, "WrongState"), "{:?}", out.messages);
}

#[test]
fn a_jog_while_jogging_queues_at_most_one_more_step_and_home_says_jogging() {
    let mut r = Rig::idle();
    let first = r.msg(&owner(), jog("X", "Plus", "1"));
    assert_eq!(moves(&first), 1);
    let mut more = 0;
    for _ in 0..3 {
        more += moves(&r.msg(&owner(), jog("X", "Plus", "1")));
    }
    assert!(more <= 1, "at most one step is queued, got {more} more");
    let home = r.msg(&owner(), Message::Home);
    assert_eq!(
        command_names(&home).iter().filter(|c| *c == "Home").count(),
        0,
        "Home must not start while jogging"
    );
    assert!(
        says(&home, "Jogging"),
        "the cause is named: {:?}",
        home.messages
    );
}

#[test]
fn a_jog_refused_while_locked_by_an_unknown_step_word_is_no_move() {
    let mut r = Rig::idle();
    let parsed = serde_json::from_value::<Message>(
        json!({"type": "Jog", "axis": "X", "direction": "Plus", "step": "5"}),
    );
    match parsed {
        Err(_) => {}
        Ok(message) => {
            let out = r.msg(&owner(), message);
            assert_eq!(moves(&out), 0, "a step of 5 mm is not in the closed set");
        }
    }
    assert_eq!(state(&r.m), "Idle");
}

// ---------------------------------------------------------------------------------------------
// Criteria 15, 16: the closed step set and the axes; no numeric step
// ---------------------------------------------------------------------------------------------

#[test]
fn the_four_steps_of_frage_315_parse_and_a_numeric_step_does_not() {
    for step in ["0.1", "1", "10", "50"] {
        let parsed = serde_json::from_value::<Message>(
            json!({"type": "Jog", "axis": "Y", "direction": "Plus", "step": step}),
        );
        assert!(parsed.is_ok(), "step {step} is in the set");
    }
    let numeric = serde_json::from_value::<Message>(
        json!({"type": "Jog", "axis": "X", "direction": "Plus", "step": 1}),
    );
    assert!(
        numeric.is_err(),
        "the step is a closed string, never a number (J1)"
    );
}

#[test]
fn every_step_of_the_pad_is_one_move_per_press() {
    for step in ["0.1", "1", "10", "50"] {
        let mut r = Rig::idle();
        let out = r.msg(&owner(), jog("X", "Plus", step));
        assert_eq!(moves(&out), 1, "step {step}: {:?}", command_names(&out));
    }
}

// ---------------------------------------------------------------------------------------------
// Criteria 10, 18, 21: Stop, release and the dead-man (injected clock, J4)
// ---------------------------------------------------------------------------------------------

#[test]
fn stop_during_a_jog_cancels_the_jog_and_is_not_an_alarm() {
    let mut r = Rig::idle();
    r.msg(&owner(), jog("X", "Plus", "1"));
    assert_eq!(state(&r.m), "Jogging");
    let out = r.msg(&owner(), Message::Stop);
    assert_eq!(cancels(&out), 1, "{:?}", command_names(&out));
    assert!(!says(&out, "Alarm"), "{:?}", out.messages);
    assert!(!status(&r.m).contains("Alarm"));
}

#[test]
fn a_held_jog_moves_until_the_dead_man_at_300_ms_and_299_keeps() {
    let mut r = Rig::idle();
    r.ev(ms(0));
    let out = r.msg(&owner(), hold("X", "Plus", "1"));
    assert_eq!(
        moves(&out),
        1,
        "a hold starts a move: {:?}",
        command_names(&out)
    );
    assert_eq!(cancels(&r.ev(ms(299))), 0, "299 ms keeps the jog");
    assert_eq!(
        cancels(&r.ev(ms(300))),
        1,
        "300 ms without a keep cancels the jog"
    );
}

#[test]
fn a_keep_at_299_ms_pushes_the_dead_man_to_599_ms() {
    let mut r = Rig::idle();
    r.ev(ms(0));
    r.msg(&owner(), hold("Y", "Minus", "1"));
    r.ev(ms(299));
    r.msg(&owner(), keep());
    assert_eq!(cancels(&r.ev(ms(598))), 0, "598 ms: still held");
    assert_eq!(
        cancels(&r.ev(ms(599))),
        1,
        "599 ms: 300 ms after the last keep"
    );
}

#[test]
fn a_keep_at_200_ms_pushes_the_dead_man_to_500_ms() {
    let mut r = Rig::idle();
    r.ev(ms(0));
    r.msg(&owner(), hold("X", "Minus", "1"));
    r.ev(ms(200));
    r.msg(&owner(), keep());
    assert_eq!(cancels(&r.ev(ms(499))), 0);
    assert_eq!(cancels(&r.ev(ms(500))), 1);
}

#[test]
fn release_within_a_hold_cancels_at_once() {
    let mut r = Rig::idle();
    r.ev(ms(0));
    r.msg(&owner(), hold("X", "Plus", "1"));
    r.ev(ms(100));
    let out = r.msg(&owner(), jog_stop());
    assert_eq!(cancels(&out), 1, "{:?}", command_names(&out));
}

#[test]
fn a_late_release_outside_a_jog_does_nothing() {
    let mut r = Rig::idle();
    let out = r.msg(&owner(), jog_stop());
    assert!(command_names(&out).is_empty(), "{:?}", command_names(&out));
    assert_eq!(state(&r.m), "Idle");
}

#[test]
fn a_late_release_never_discards_a_ready_job() {
    let mut r = Rig::idle();
    r.submit(3, JOB);
    r.msg(&owner(), jog_stop());
    assert_eq!(state(&r.m), "Ready");
    assert!(!says(&r.msg(&owner(), jog_stop()), "Error"));
    assert_eq!(state(&r.m), "Ready");
}

#[test]
fn jog_is_never_sent_on_its_own() {
    let mut r = Rig::idle();
    let connected = r.ev(Event::LaserModeOff(false));
    assert_eq!(moves(&connected), 0);
    let later = r.ev(ms(5_000));
    assert_eq!(moves(&later), 0, "no motion without a press (criterion 11)");
    assert_eq!(cancels(&later), 0);
}

// ---------------------------------------------------------------------------------------------
// Criteria 12, 37: owner-only motion; a member's Home waits (J7)
// ---------------------------------------------------------------------------------------------

#[test]
fn a_member_is_refused_jog_and_keep_by_the_access_table_and_may_release() {
    // The role table is `access::admit` (the session calls it before the machine).
    let step = jog("X", "Plus", "1");
    let held = hold("Y", "Plus", "1");
    assert_eq!(
        admit(Mode::Configured, Some(Role::Member), &step),
        Err(curvyo_connector_core::ErrorCode::OwnerRequired)
    );
    assert_eq!(
        admit(Mode::Configured, Some(Role::Member), &held),
        Err(curvyo_connector_core::ErrorCode::OwnerRequired)
    );
    assert_eq!(
        admit(Mode::Configured, Some(Role::Member), &keep()),
        Err(curvyo_connector_core::ErrorCode::OwnerRequired)
    );
    assert!(
        admit(Mode::Configured, Some(Role::Member), &jog_stop()).is_ok(),
        "JogStop is allowed for all"
    );
    assert!(admit(Mode::Configured, Some(Role::Owner), &step).is_ok());
    assert!(admit(Mode::Configured, Some(Role::Owner), &keep()).is_ok());
    assert!(
        admit(Mode::Configured, Some(Role::Member), &Message::Home).is_ok(),
        "a member's Home passes to the machine"
    );

    let mut r = Rig::idle();
    let release = r.msg(&owner(), jog_stop());
    assert!(!says(&release, "Error"), "{:?}", release.messages);
    assert_eq!(moves(&r.msg(&owner(), step)), 1, "the owner jogs");
}

#[test]
fn a_member_home_waits_for_an_owner_and_a_second_member_home_is_wrong_state() {
    let mut r = Rig::idle();
    let waiting = r.msg(&member(), Message::Home);
    assert!(!says(&waiting, "Error"), "{:?}", waiting.messages);
    assert_eq!(
        command_names(&waiting)
            .iter()
            .filter(|c| *c == "Home")
            .count(),
        0,
        "a member's Home moves nothing before the owner confirms"
    );
    assert!(
        status(&r.m).contains("Phone"),
        "requested_by names the member: {}",
        status(&r.m)
    );

    let second = r.msg(&Peer::new("Watch", Role::Member), Message::Home);
    assert!(says(&second, "WrongState"), "{:?}", second.messages);
}

#[test]
fn the_owners_home_confirms_a_waiting_member_home() {
    let mut r = Rig::idle();
    r.msg(&member(), Message::Home);
    let confirm = r.msg(&owner(), Message::Home);
    assert_eq!(
        command_names(&confirm)
            .iter()
            .filter(|c| *c == "Home")
            .count(),
        1,
        "{:?}",
        command_names(&confirm)
    );
    assert!(!status(&r.m).contains("Phone"), "the wait is over");
}

#[test]
fn stop_discards_a_waiting_member_home_and_moves_nothing() {
    let mut r = Rig::idle();
    r.msg(&member(), Message::Home);
    let stop = r.msg(&owner(), Message::Stop);
    assert_eq!(
        command_names(&stop).iter().filter(|c| *c == "Home").count(),
        0,
        "{:?}",
        command_names(&stop)
    );
    assert!(
        !status(&r.m).contains("Phone"),
        "the waiting Home is discarded"
    );
}

// ---------------------------------------------------------------------------------------------
// Criterion 13 (homed since connect), J5
// ---------------------------------------------------------------------------------------------

#[test]
fn homed_since_connect_is_set_by_homed_and_cleared_by_a_reconnect() {
    let mut r = Rig::idle();
    assert!(status(&r.m).contains("homed: false"), "{}", status(&r.m));
    r.ev(Event::Homed);
    assert!(status(&r.m).contains("homed: true"), "{}", status(&r.m));
    r.ev(Event::ControllerLost);
    assert!(
        status(&r.m).contains("homed: false"),
        "a loss of the controller clears it: {}",
        status(&r.m)
    );
    r.ev(Event::Homed);
    r.ev(Event::ControllerLost);
    r.ev(Event::ControllerConnected);
    assert!(
        status(&r.m).contains("homed: false"),
        "a reconnect clears it: {}",
        status(&r.m)
    );
}
