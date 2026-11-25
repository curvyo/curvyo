//! The job list in the job machine (`specs/0132` criteria 1 to 5, 7, 9, 11): table and flow tests.
#![allow(clippy::unwrap_used, clippy::option_option, clippy::too_many_lines)]

mod common;

use common::sha256_hex;
use curvyo_connector_core::{
    ControllerCommand, ErrorCode, Event, JobKind, JobResult, ListEntry, Machine, Message,
    Millimetres, Options, Output, Peer, RejectReason, Role, StartRule, State,
};
use uuid::Uuid;

const TEXT: &str = "G21\nG90\nG0 X0 Y0\n";

fn id(n: u128) -> Uuid {
    Uuid::from_u128(n)
}

fn owner() -> Peer {
    Peer::new("Laptop", Role::Owner)
}

fn member() -> Peer {
    Peer::new("Phone", Role::Member)
}

struct Rig {
    m: Machine,
    /// Every output in order, follow-ups included.
    seen: Vec<Output>,
}

impl Rig {
    fn new(rule: StartRule) -> Self {
        let mut m = Machine::new(Options { start_rule: rule });
        m.handle(Event::ControllerConnected);
        Self {
            m,
            seen: Vec::new(),
        }
    }

    fn event(&mut self, event: Event) -> Vec<Message> {
        let mut messages = Vec::new();
        let mut queue = vec![event];
        while let Some(event) = queue.pop() {
            let out = self.m.handle(event);
            messages.extend(out.messages.clone());
            queue.extend(out.follow_up.clone());
            self.seen.push(out);
        }
        messages
    }

    fn msg(&mut self, message: Message, peer: &Peer) -> Vec<Message> {
        self.event(Event::Message(message, peer.clone()))
    }

    /// Sends a whole job in one chunk; the answers of the chunk.
    fn send(&mut self, n: u128, text: &str, peer: &Peer) -> Vec<Message> {
        self.msg(
            Message::SubmitJob {
                job_id: id(n),
                name: format!("j{n}.gcode"),
                kind: JobKind::Job,
                bytes: text.len() as u64,
                sha256: sha256_hex(text),
            },
            peer,
        );
        self.msg(
            Message::JobChunk {
                job_id: id(n),
                index: 0,
                text: text.to_owned(),
            },
            peer,
        )
    }

    fn waiting(&self) -> Vec<(u128, u32)> {
        match self.m.job_list() {
            Message::JobList { waiting, .. } => waiting
                .iter()
                .map(|e| (e.job_id.as_u128(), e.position))
                .collect(),
            other => panic!("{other:?}"),
        }
    }

    fn note(&self) -> Option<String> {
        match self.m.job_list() {
            Message::JobList { note, .. } => note,
            other => panic!("{other:?}"),
        }
    }

    fn flags(&self) -> Vec<bool> {
        match self.m.job_list() {
            Message::JobList { waiting, .. } => waiting.iter().map(|e| e.run_next).collect(),
            other => panic!("{other:?}"),
        }
    }
}

fn accepted(messages: &[Message], n: u128) -> Option<Option<u32>> {
    messages.iter().find_map(|m| match m {
        Message::JobAccepted { job_id, position } if *job_id == id(n) => Some(*position),
        _ => None,
    })
}

fn error_of(messages: &[Message]) -> Option<ErrorCode> {
    messages.iter().find_map(|m| match m {
        Message::Error { code, .. } => Some(*code),
        _ => None,
    })
}

fn lists(messages: &[Message]) -> Vec<&Message> {
    messages
        .iter()
        .filter(|m| matches!(m, Message::JobList { .. }))
        .collect()
}

/// A rig whose first job runs and whose later jobs wait.
fn running_with(rule: StartRule, waiting: &[u128]) -> Rig {
    let mut rig = Rig::new(rule);
    rig.send(1, TEXT, &owner());
    rig.msg(Message::Start { job_id: id(1) }, &owner());
    if rule == StartRule::LocalPress {
        rig.event(Event::LocalStartPressed);
    }
    assert_eq!(rig.m.state(), State::Running);
    for n in waiting {
        let answer = rig.send(*n, TEXT, &member());
        assert!(accepted(&answer, *n).is_some(), "{answer:?}");
    }
    rig
}

#[test]
fn a_send_while_a_job_is_held_waits_in_order_and_leaves_the_current_job_alone() {
    for held in [
        State::Ready,
        State::Running,
        State::Paused,
        State::AwaitingLocalStart,
    ] {
        let mut rig = Rig::new(StartRule::LocalPress);
        rig.send(1, TEXT, &owner());
        rig.msg(Message::Start { job_id: id(1) }, &owner());
        assert_eq!(rig.m.state(), State::AwaitingLocalStart);
        if held == State::Ready {
            rig = Rig::new(StartRule::Immediate);
            rig.send(1, TEXT, &owner());
        } else if held != State::AwaitingLocalStart {
            rig.event(Event::LocalStartPressed);
            if held == State::Paused {
                rig.msg(Message::Pause, &owner());
            }
        }
        assert_eq!(rig.m.state(), held);
        let a = rig.send(2, TEXT, &member());
        let b = rig.send(3, TEXT, &owner());
        assert_eq!(accepted(&a, 2), Some(Some(1)), "{held:?}");
        assert_eq!(accepted(&b, 3), Some(Some(2)), "{held:?}");
        assert_eq!(rig.waiting(), [(2, 1), (3, 2)]);
        assert_eq!(rig.m.state(), held, "the current job is untouched");
        assert_eq!(rig.m.held_job().unwrap().job_id, id(1));
    }
}

#[test]
fn a_send_with_no_current_job_still_becomes_current() {
    let mut rig = Rig::new(StartRule::Immediate);
    let answer = rig.send(1, TEXT, &owner());
    assert_eq!(accepted(&answer, 1), Some(None));
    assert_eq!(rig.m.state(), State::Ready);
    assert_eq!(rig.waiting(), []);
}

#[test]
fn the_list_is_pushed_on_change_and_not_for_a_current_job_alone() {
    let mut rig = Rig::new(StartRule::Immediate);
    let answer = rig.send(1, TEXT, &owner());
    assert!(
        lists(&answer).is_empty(),
        "nothing waits: the status says it"
    );
    let answer = rig.send(2, TEXT, &owner());
    let [
        Message::JobList {
            current, waiting, ..
        },
    ] = lists(&answer)[..]
    else {
        panic!("{answer:?}");
    };
    assert_eq!(current.as_ref().map(|e| e.job_id), Some(id(1)));
    assert_eq!(waiting.len(), 1);
    // Unchanged list: nothing pushed (a progress event, a status).
    let out = rig.event(Event::Position([
        Millimetres(1.0),
        Millimetres(0.0),
        Millimetres(0.0),
    ]));
    assert_eq!(lists(&out).len(), 0);
}

#[test]
fn limits_are_twenty_jobs_and_256_mib() {
    let mut rig = running_with(StartRule::Immediate, &[]);
    for n in 2..=21 {
        let answer = rig.send(n, TEXT, &member());
        assert!(accepted(&answer, n).is_some(), "{n}");
    }
    // 20 waiting: the 21st is refused at the announcement.
    let refusal = rig.msg(
        Message::SubmitJob {
            job_id: id(99),
            name: "x".into(),
            kind: JobKind::Job,
            bytes: 1,
            sha256: String::new(),
        },
        &member(),
    );
    assert_eq!(
        refusal,
        [Message::JobRejected {
            job_id: id(99),
            reason: RejectReason::ListFull
        }]
    );

    // Bytes: four announced 64 MiB jobs fill 256 MiB, a fifth byte does not fit.
    let mut rig = running_with(StartRule::Immediate, &[]);
    let big = |n: u128, bytes: u64| Message::SubmitJob {
        job_id: id(n),
        name: "big".into(),
        kind: JobKind::Job,
        bytes,
        sha256: String::new(),
    };
    for n in 2..=5 {
        let answer = rig.msg(big(n, 64 * 1024 * 1024), &member());
        assert!(answer.is_empty(), "{n}: {answer:?}");
    }
    let answer = rig.msg(big(6, 1), &member());
    assert_eq!(
        answer,
        [Message::JobRejected {
            job_id: id(6),
            reason: RejectReason::ListFull
        }]
    );
}

#[test]
fn a_frame_is_never_listed_and_a_second_equal_id_is_refused() {
    let mut rig = running_with(StartRule::Immediate, &[2]);
    let frame = rig.msg(
        Message::SubmitJob {
            job_id: id(8),
            name: "frame".into(),
            kind: JobKind::Frame,
            bytes: 3,
            sha256: sha256_hex("G0\n"),
        },
        &owner(),
    );
    assert_eq!(
        frame,
        [Message::JobRejected {
            job_id: id(8),
            reason: RejectReason::Busy
        }]
    );
    let twice = rig.msg(
        Message::SubmitJob {
            job_id: id(2),
            name: "again".into(),
            kind: JobKind::Job,
            bytes: 3,
            sha256: sha256_hex("G0\n"),
        },
        &owner(),
    );
    assert!(matches!(twice[..], [Message::JobRejected { .. }]));
}

#[test]
fn move_changes_the_order_for_everyone_and_a_stale_position_refreshes_the_list() {
    let mut rig = running_with(StartRule::OwnerConfirms, &[2, 3, 4]);
    let out = rig.msg(
        Message::MoveJob {
            job_id: id(4),
            from: 3,
            to: 1,
        },
        &owner(),
    );
    assert_eq!(error_of(&out), None);
    assert_eq!(lists(&out).len(), 1);
    assert_eq!(rig.waiting(), [(4, 1), (2, 2), (3, 3)]);
    // Stale: the job is at 1 now, not 3.
    let out = rig.msg(
        Message::MoveJob {
            job_id: id(4),
            from: 3,
            to: 2,
        },
        &owner(),
    );
    assert_eq!(error_of(&out), Some(ErrorCode::WrongState));
    assert_eq!(lists(&out).len(), 1, "the list is sent again");
    // The current job cannot be moved; an unknown id is unknown.
    let out = rig.msg(
        Message::MoveJob {
            job_id: id(1),
            from: 1,
            to: 2,
        },
        &owner(),
    );
    assert_eq!(error_of(&out), Some(ErrorCode::WrongState));
    let out = rig.msg(
        Message::MoveJob {
            job_id: id(77),
            from: 1,
            to: 2,
        },
        &owner(),
    );
    assert_eq!(error_of(&out), Some(ErrorCode::UnknownJob));
    // A member does not reorder (Frage 257 A).
    let out = rig.msg(
        Message::MoveJob {
            job_id: id(2),
            from: 2,
            to: 1,
        },
        &member(),
    );
    assert_eq!(error_of(&out), Some(ErrorCode::OwnerRequired));
    assert_eq!(rig.waiting(), [(4, 1), (2, 2), (3, 3)]);
}

#[test]
fn a_member_removes_only_its_own_waiting_job_and_an_owner_any() {
    let mut rig = running_with(StartRule::OwnerConfirms, &[2, 3]);
    rig.send(4, TEXT, &owner());
    let out = rig.msg(Message::RemoveJob { job_id: id(4) }, &member());
    assert_eq!(error_of(&out), Some(ErrorCode::OwnerRequired));
    let out = rig.msg(Message::RemoveJob { job_id: id(2) }, &member());
    assert_eq!(error_of(&out), None);
    assert!(out.iter().all(|m| !matches!(m, Message::Error { .. })));
    assert_eq!(rig.waiting(), [(3, 1), (4, 2)]);
    let out = rig.seen.last().unwrap();
    assert!(out.list_log.is_empty(), "removing its own is not logged");
    let out = rig.msg(Message::RemoveJob { job_id: id(3) }, &owner());
    assert_eq!(error_of(&out), None);
    assert_eq!(rig.waiting(), [(4, 1)]);
    assert_eq!(
        rig.seen.last().unwrap().list_log.len(),
        1,
        "an owner removing another's job"
    );
    let out = rig.msg(Message::RemoveJob { job_id: id(3) }, &owner());
    assert_eq!(error_of(&out), Some(ErrorCode::UnknownJob));
}

#[test]
fn start_takes_the_first_waiting_job_only_when_the_current_one_is_done() {
    let mut rig = running_with(StartRule::Immediate, &[2, 3]);
    // Running: nothing starts, the list is sent again with the answer.
    let out = rig.msg(Message::Start { job_id: id(2) }, &owner());
    assert_eq!(error_of(&out), Some(ErrorCode::WrongState));
    assert_eq!(lists(&out).len(), 1);
    rig.event(Event::JobFinished { elapsed_ms: 10 });
    assert_eq!(
        rig.m.state(),
        State::Done {
            result: JobResult::Completed
        }
    );
    // Only the first waiting job.
    let out = rig.msg(Message::Start { job_id: id(3) }, &owner());
    assert_eq!(error_of(&out), Some(ErrorCode::WrongState));
    assert_eq!(rig.waiting(), [(2, 1), (3, 2)]);
    let out = rig.msg(Message::Start { job_id: id(2) }, &owner());
    assert_eq!(error_of(&out), None);
    assert_eq!(rig.m.state(), State::Running);
    assert_eq!(rig.m.held_job().unwrap().job_id, id(2));
    assert_eq!(rig.waiting(), [(3, 1)]);
    assert!(
        rig.seen
            .iter()
            .any(|o| o.commands == [ControllerCommand::Start { job_id: id(2) }])
    );
}

#[test]
fn start_under_the_owner_rule_waits_for_an_owner_when_a_member_pressed_it() {
    let mut rig = running_with(StartRule::OwnerConfirms, &[2]);
    rig.event(Event::JobFinished { elapsed_ms: 10 });
    rig.msg(Message::Start { job_id: id(2) }, &member());
    assert_eq!(rig.m.state(), State::AwaitingLocalStart);
    let Message::Status { requested_by, .. } = rig.m.status() else {
        panic!()
    };
    assert_eq!(requested_by.as_deref(), Some("Phone"));
    rig.msg(Message::Start { job_id: id(2) }, &owner());
    assert_eq!(rig.m.state(), State::Running);

    // An owner's press runs at once; under `LocalPress` it waits for Enter.
    let mut rig = running_with(StartRule::LocalPress, &[]);
    rig.send(2, TEXT, &owner());
    rig.event(Event::JobFinished { elapsed_ms: 10 });
    rig.msg(Message::Start { job_id: id(2) }, &owner());
    assert_eq!(rig.m.state(), State::AwaitingLocalStart);
}

#[test]
fn stop_ends_only_the_current_job_and_the_list_keeps_the_rest() {
    let mut rig = running_with(StartRule::Immediate, &[2, 3]);
    rig.msg(Message::Stop, &owner());
    assert_eq!(
        rig.m.state(),
        State::Done {
            result: JobResult::Stopped
        }
    );
    // Frage 390 C (customer 2026-11-25): the stopped job stays listed as interrupted, first.
    assert_eq!(rig.waiting(), [(1, 0), (2, 1), (3, 2)]);
    // A Ready job that is stopped is discarded; the list stays.
    let mut rig = Rig::new(StartRule::Immediate);
    rig.send(1, TEXT, &owner());
    rig.send(2, TEXT, &owner());
    rig.msg(Message::Stop, &owner());
    assert_eq!(rig.m.state(), State::Idle);
    assert_eq!(rig.waiting(), [(2, 1)]);
    // With no current job, a send joins the list behind it, and Start takes the first.
    rig.send(3, TEXT, &owner());
    assert_eq!(rig.waiting(), [(2, 1), (3, 2)]);
    rig.msg(Message::Start { job_id: id(2) }, &owner());
    assert_eq!(rig.m.state(), State::Running);
}

#[test]
fn a_dropped_connection_drops_only_its_own_transfer() {
    let mut rig = running_with(StartRule::Immediate, &[]);
    for n in [2, 3] {
        rig.msg(
            Message::SubmitJob {
                job_id: id(n),
                name: format!("j{n}"),
                kind: JobKind::Job,
                bytes: TEXT.len() as u64,
                sha256: sha256_hex(TEXT),
            },
            &member(),
        );
    }
    rig.event(Event::TransferAbandoned { job_id: id(2) });
    let answer = rig.msg(
        Message::JobChunk {
            job_id: id(3),
            index: 0,
            text: TEXT.into(),
        },
        &member(),
    );
    assert_eq!(accepted(&answer, 3), Some(Some(1)));
    let answer = rig.msg(
        Message::JobChunk {
            job_id: id(2),
            index: 0,
            text: TEXT.into(),
        },
        &member(),
    );
    assert_eq!(error_of(&answer), Some(ErrorCode::UnknownJob));
    // A cut transfer of the current job still ends in Idle (`0125` S19), with a list behind it.
    let mut rig = Rig::new(StartRule::Immediate);
    rig.msg(
        Message::SubmitJob {
            job_id: id(1),
            name: "j1".into(),
            kind: JobKind::Job,
            bytes: 100,
            sha256: String::new(),
        },
        &owner(),
    );
    rig.send(2, TEXT, &owner());
    rig.event(Event::TransferAbandoned { job_id: id(1) });
    assert_eq!(rig.m.state(), State::Idle);
    assert_eq!(rig.waiting(), [(2, 1)]);
}

fn entry_flags(rig: &Rig) -> Vec<ListEntry> {
    match rig.m.job_list() {
        Message::JobList { waiting, .. } => waiting,
        other => panic!("{other:?}"),
    }
}

#[test]
fn run_next_is_set_by_an_owner_on_any_job_and_by_a_member_on_its_own() {
    let mut rig = running_with(StartRule::OwnerConfirms, &[2]);
    rig.send(3, TEXT, &owner());
    let on = |n| Message::SetRunNext {
        job_id: id(n),
        on: true,
    };
    assert_eq!(
        error_of(&rig.msg(on(3), &member())),
        Some(ErrorCode::OwnerRequired)
    );
    assert_eq!(error_of(&rig.msg(on(2), &member())), None);
    assert_eq!(error_of(&rig.msg(on(3), &owner())), None);
    assert_eq!(rig.flags(), [true, true]);
    rig.msg(
        Message::SetRunNext {
            job_id: id(2),
            on: false,
        },
        &member(),
    );
    assert_eq!(rig.flags(), [false, true]);
    assert!(!entry_flags(&rig)[0].run_next);
}

/// Ends the current job as `end` and returns the rig.
fn end_job(rig: &mut Rig, end: &str) {
    let event = match end {
        "completed" => Event::JobFinished { elapsed_ms: 1 },
        "failed" => Event::JobFailed { elapsed_ms: 1 },
        "stopped" => {
            rig.msg(Message::Stop, &owner());
            return;
        }
        "alarm" => Event::Alarm { text: "x".into() },
        "lost" => Event::ControllerLost,
        other => panic!("{other}"),
    };
    rig.event(event);
}

#[test]
fn chain_table_end_x_flag_x_rule() {
    // (rule, flagging peer, end) -> (state after, flags after, note)
    let owner_flag = owner();
    let member_flag = member();
    let cases: [(StartRule, &Peer, &str, State, Option<&str>); 9] = [
        (
            StartRule::Immediate,
            &owner_flag,
            "completed",
            State::Running,
            None,
        ),
        (
            StartRule::LocalPress,
            &owner_flag,
            "completed",
            State::AwaitingLocalStart,
            None,
        ),
        (
            StartRule::OwnerConfirms,
            &owner_flag,
            "completed",
            State::Running,
            None,
        ),
        (
            StartRule::OwnerConfirms,
            &member_flag,
            "completed",
            State::AwaitingLocalStart,
            None,
        ),
        (
            StartRule::Immediate,
            &owner_flag,
            "failed",
            State::Done {
                result: JobResult::Failed,
            },
            Some("Chain stopped: Failed"),
        ),
        (
            StartRule::Immediate,
            &owner_flag,
            "stopped",
            State::Done {
                result: JobResult::Stopped,
            },
            Some("Chain stopped: Stopped"),
        ),
        (
            StartRule::Immediate,
            &owner_flag,
            "alarm",
            State::Alarm,
            Some("Chain stopped: Alarm"),
        ),
        (
            StartRule::Immediate,
            &owner_flag,
            "lost",
            State::NoController,
            Some("Chain stopped: Lost the controller"),
        ),
        (
            StartRule::Immediate,
            &owner_flag,
            "none",
            State::Done {
                result: JobResult::Completed,
            },
            None,
        ),
    ];
    for (rule, flagger, end, state, note) in cases {
        let mut rig = running_with(rule, &[2, 3]);
        if rule == StartRule::LocalPress {
            // the first job needed Enter
        }
        if end != "none" {
            let on = Message::SetRunNext {
                job_id: id(2),
                on: true,
            };
            let flagger = if rule == StartRule::OwnerConfirms {
                flagger
            } else {
                &owner_flag
            };
            assert_eq!(error_of(&rig.msg(on, flagger)), None);
        }
        let end_word = if end == "none" { "completed" } else { end };
        end_job(&mut rig, end_word);
        let label = format!("{rule:?} {end}");
        assert_eq!(rig.m.state(), state, "{label}");
        assert_eq!(rig.note().as_deref(), note, "{label}");
        let started = matches!(state, State::Running | State::AwaitingLocalStart);
        if started {
            assert_eq!(rig.waiting(), [(3, 1)], "{label}");
            assert_eq!(rig.m.held_job().unwrap().job_id, id(2), "{label}");
        } else {
            // An interrupted entry (Frage 390 C: Stop, alarm, loss) stands outside the order.
            let waiting = rig.waiting().iter().filter(|(_, p)| *p > 0).count();
            assert_eq!(waiting, 2, "{label}");
            assert!(
                rig.flags().iter().all(|f| !f),
                "{label}: flags cleared or never set"
            );
        }
    }
}

#[test]
fn a_chain_of_three_runs_and_a_failure_in_the_second_cancels_it() {
    let mut rig = running_with(StartRule::Immediate, &[2, 3, 4]);
    for n in [2, 3, 4] {
        rig.msg(
            Message::SetRunNext {
                job_id: id(n),
                on: true,
            },
            &owner(),
        );
    }
    rig.event(Event::JobFinished { elapsed_ms: 1 });
    assert_eq!(rig.m.held_job().unwrap().job_id, id(2));
    assert_eq!(rig.m.state(), State::Running);
    rig.event(Event::JobFailed { elapsed_ms: 1 });
    assert_eq!(
        rig.m.state(),
        State::Done {
            result: JobResult::Failed
        }
    );
    assert_eq!(rig.waiting(), [(3, 1), (4, 2)]);
    assert_eq!(rig.flags(), [false, false]);
    assert_eq!(rig.note().as_deref(), Some("Chain stopped: Failed"));
    // Only an owner's Start continues, and it clears the note.
    rig.msg(Message::Start { job_id: id(3) }, &owner());
    assert_eq!(rig.m.state(), State::Running);
    assert_eq!(rig.note(), None);
}

#[test]
fn a_flagged_job_does_not_follow_when_the_controller_is_not_ready() {
    for word in ["Door", "Check", "Sleep"] {
        let mut rig = running_with(StartRule::Immediate, &[2]);
        rig.msg(
            Message::SetRunNext {
                job_id: id(2),
                on: true,
            },
            &owner(),
        );
        rig.event(Event::ControllerWord(Some(word.into())));
        rig.event(Event::JobFinished { elapsed_ms: 1 });
        assert_eq!(
            rig.m.state(),
            State::Done {
                result: JobResult::Completed
            }
        );
        assert_eq!(rig.note(), Some(format!("Chain stopped: {word}")));
    }
    let mut rig = running_with(StartRule::Immediate, &[2]);
    rig.msg(
        Message::SetRunNext {
            job_id: id(2),
            on: true,
        },
        &owner(),
    );
    rig.event(Event::LaserModeOff(true));
    rig.event(Event::JobFinished { elapsed_ms: 1 });
    assert_eq!(rig.note().as_deref(), Some("Chain stopped: Not ready"));
}

#[test]
fn restored_jobs_wait_in_order_nothing_starts_and_an_interrupted_job_cannot_be_started() {
    use curvyo_connector_core::{Entry, Item};
    use std::sync::Arc;
    let item = |n: u128, interrupted: Option<u32>| Item {
        entry: Entry {
            job_id: id(n),
            name: format!("j{n}.gcode"),
            lines: 3,
            bytes: TEXT.len() as u64,
            sha256: sha256_hex(TEXT),
            sent_by: "Phone".into(),
            run_next_by: None,
            interrupted_at: interrupted,
            stop: None,
        },
        text: Arc::from(TEXT),
    };
    let mut rig = Rig::new(StartRule::Immediate);
    rig.m
        .restore(vec![item(7, Some(2)), item(1, None), item(2, None)]);
    assert_eq!(rig.waiting(), [(7, 0), (1, 1), (2, 2)]);
    assert_eq!(rig.m.state(), State::Idle);
    let out = rig.msg(Message::Start { job_id: id(7) }, &owner());
    assert_eq!(error_of(&out), Some(ErrorCode::WrongState));
    assert_eq!(rig.m.state(), State::Idle);
    let out = rig.msg(
        Message::MoveJob {
            job_id: id(7),
            from: 0,
            to: 1,
        },
        &owner(),
    );
    assert_eq!(error_of(&out), Some(ErrorCode::WrongState));
    // An owner removes it; a member may not.
    let mut rig2 = Rig::new(StartRule::OwnerConfirms);
    rig2.m.restore(vec![item(7, Some(2)), item(1, None)]);
    assert_eq!(
        error_of(&rig2.msg(Message::RemoveJob { job_id: id(7) }, &member())),
        Some(ErrorCode::OwnerRequired)
    );
    assert_eq!(
        error_of(&rig2.msg(Message::RemoveJob { job_id: id(7) }, &owner())),
        None
    );
    assert_eq!(rig2.waiting(), [(1, 1)]);
    // Start takes the first not interrupted job, as soon as the interrupted one is out of the way.
    rig.msg(Message::Start { job_id: id(1) }, &owner());
    assert_eq!(rig.m.state(), State::Running);
}
