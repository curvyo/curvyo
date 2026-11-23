//! Independent acceptance tests for `specs/0132-connector-job-queue` (criteria 1 to 5, 7 and 11),
//! written from the specification and `adrs.md` before reading the implementation. The pure
//! machine only: no clock, no I/O. Restart and wire-level criteria live in the io crate.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::format_collect,
    clippy::option_option
)]

use curvyo_connector_core::{
    ControllerCommand, ErrorCode, Event, JobKind, JobResult, Machine, Message, Options, Output,
    Peer, RejectReason, Role, StartRule, State,
};
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

fn laptop() -> Peer {
    Peer::new("Laptop", Role::Owner)
}

fn phone() -> Peer {
    Peer::new("Phone", Role::Member)
}

/// One row of the list as a client shows it.
#[derive(Debug, Clone, PartialEq)]
struct Row {
    id: Uuid,
    name: String,
    bytes: u64,
    sent_by: String,
    position: u32,
    run_next: bool,
}

/// The last list pushed in `out`.
#[derive(Debug, Clone, PartialEq)]
struct Snap {
    current: Option<Uuid>,
    waiting: Vec<Row>,
    note: Option<String>,
}

fn list_in(out: &Output) -> Option<Snap> {
    out.messages.iter().rev().find_map(|m| match m {
        Message::JobList {
            current,
            waiting,
            note,
        } => Some(Snap {
            current: current.as_ref().map(|c| c.job_id),
            waiting: waiting
                .iter()
                .map(|e| Row {
                    id: e.job_id,
                    name: e.name.clone(),
                    bytes: e.bytes,
                    sent_by: e.sent_by.clone(),
                    position: e.position,
                    run_next: e.run_next,
                })
                .collect(),
            note: note.clone(),
        }),
        _ => None,
    })
}

fn accepted_in(out: &Output, job: Uuid) -> Option<Option<u32>> {
    out.messages.iter().find_map(|m| match m {
        Message::JobAccepted { job_id, position } if *job_id == job => Some(*position),
        _ => None,
    })
}

fn rejected_in(out: &Output, job: Uuid) -> Option<RejectReason> {
    out.messages.iter().find_map(|m| match m {
        Message::JobRejected { job_id, reason } if *job_id == job => Some(*reason),
        _ => None,
    })
}

fn error_in(out: &Output) -> Option<ErrorCode> {
    out.messages.iter().find_map(|m| match m {
        Message::Error { code, .. } => Some(*code),
        _ => None,
    })
}

fn starts(out: &Output) -> Vec<Uuid> {
    out.commands
        .iter()
        .filter_map(|c| match c {
            ControllerCommand::Start { job_id } => Some(*job_id),
            _ => None,
        })
        .collect()
}

struct Rig {
    m: Machine,
}

impl Rig {
    fn new(rule: StartRule) -> Self {
        let mut m = Machine::new(Options { start_rule: rule });
        m.handle(Event::ControllerConnected);
        // The controller reports its laser mode on (G34 satisfied) and readable.
        m.handle(Event::LaserModeUnknown(false));
        m.handle(Event::LaserModeOff(false));
        Self { m }
    }

    /// Applies `event` and, as the caller must, the follow-up event the machine asks for (the
    /// chain's `RunNext`); both answers are merged.
    fn ev(&mut self, event: Event) -> Output {
        let out = self.m.handle(event);
        self.settle(out)
    }

    fn msg(&mut self, peer: &Peer, message: Message) -> Output {
        self.ev(Event::Message(message, peer.clone()))
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

    /// Submits `text` as job `n` and sends its single chunk; both answers together.
    fn send_as(&mut self, n: u128, peer: &Peer, name: &str, text: &str, kind: JobKind) -> Output {
        let submit = self.msg(
            peer,
            Message::SubmitJob {
                job_id: id(n),
                name: name.to_owned(),
                kind,
                bytes: text.len() as u64,
                sha256: sha_hex(text),
            },
        );
        let chunk = self.msg(
            peer,
            Message::JobChunk {
                job_id: id(n),
                index: 0,
                text: text.to_owned(),
            },
        );
        let mut messages = submit.messages;
        messages.extend(chunk.messages);
        let mut commands = submit.commands;
        commands.extend(chunk.commands);
        Output {
            messages,
            commands,
            ..Output::default()
        }
    }

    fn send(&mut self, n: u128, peer: &Peer, name: &str) -> Output {
        self.send_as(n, peer, name, JOB, JobKind::Job)
    }

    fn start(&mut self, peer: &Peer, n: u128) -> Output {
        self.msg(peer, Message::Start { job_id: id(n) })
    }

    /// The job id the status shows as running or paused, if any.
    fn status_job(&self) -> Option<Uuid> {
        match self.m.status() {
            Message::Status { job: Some(p), .. } => Some(p.job_id),
            _ => None,
        }
    }

    fn run_first(&mut self, n: u128, sender: &Peer) {
        self.send(n, sender, "a.gcode");
        let mut out = self.start(&laptop(), n);
        if self.m.state() == State::AwaitingLocalStart {
            let press = self.ev(Event::LocalStartPressed);
            out.commands.extend(press.commands);
        }
        assert_eq!(starts(&out), vec![id(n)], "Start of the first job runs it");
        assert_eq!(self.m.state(), State::Running);
    }
}

// ---------------------------------------------------------------------------------------------
// Criterion 1: a send while busy is listed with its position.
// ---------------------------------------------------------------------------------------------

#[test]
fn c1_a_send_while_running_is_listed_with_position_and_current_untouched() {
    let mut r = Rig::new(StartRule::Immediate);
    let a = r.send(1, &laptop(), "a.gcode");
    assert_eq!(
        accepted_in(&a, id(1)),
        Some(None),
        "current job: no position"
    );
    r.start(&laptop(), 1);
    assert_eq!(r.m.state(), State::Running);

    let b = r.send(2, &phone(), "b.gcode");
    assert_eq!(accepted_in(&b, id(2)), Some(Some(1)));
    assert_eq!(rejected_in(&b, id(2)), None);
    let c = r.send(3, &laptop(), "c.gcode");
    assert_eq!(accepted_in(&c, id(3)), Some(Some(2)));
    assert_eq!(error_in(&c), None);

    assert_eq!(r.m.state(), State::Running, "current job untouched");
    assert_eq!(r.status_job(), Some(id(1)));
}

#[test]
fn c1_busy_still_refuses_a_frame_and_home_while_a_job_runs() {
    let mut r = Rig::new(StartRule::Immediate);
    r.run_first(1, &laptop());

    let frame = r.send_as(5, &laptop(), "frame.gcode", JOB, JobKind::Frame);
    assert_eq!(rejected_in(&frame, id(5)), Some(RejectReason::Busy));
    assert_eq!(accepted_in(&frame, id(5)), None);

    let home = r.msg(&laptop(), Message::Home);
    assert!(
        !home.commands.contains(&ControllerCommand::Home),
        "Home must not reach the controller while a job runs"
    );
    assert_eq!(r.m.state(), State::Running);
}

// ---------------------------------------------------------------------------------------------
// Criterion 2: the list carries job id, name, lines, size, sender, position.
// ---------------------------------------------------------------------------------------------

#[test]
fn c2_the_list_pushed_on_each_change_carries_current_and_waiting_entries() {
    let mut r = Rig::new(StartRule::Immediate);
    r.run_first(1, &laptop());
    let b = r.send(2, &phone(), "b.gcode");
    let snap = list_in(&b).expect("a JobList follows a change");
    assert_eq!(snap.current, Some(id(1)));
    assert_eq!(
        snap.waiting,
        vec![Row {
            id: id(2),
            name: "b.gcode".into(),
            bytes: JOB.len() as u64,
            sent_by: "Phone".into(),
            position: 1,
            run_next: false,
        }]
    );
    assert_eq!(snap.note, None);
}

// ---------------------------------------------------------------------------------------------
// Criteria 3 and 5 (owner): move, stale position, current job cannot move.
// ---------------------------------------------------------------------------------------------

fn three_waiting() -> Rig {
    three_waiting_with(StartRule::Immediate)
}

fn three_waiting_with(rule: StartRule) -> Rig {
    let mut r = Rig::new(rule);
    r.run_first(1, &laptop());
    r.send(2, &phone(), "b.gcode");
    r.send(3, &laptop(), "c.gcode");
    r
}

#[test]
fn c3_owner_moves_a_waiting_job_and_the_order_changes_for_everyone() {
    let mut r = three_waiting();
    let out = r.msg(
        &laptop(),
        Message::MoveJob {
            job_id: id(3),
            from: 2,
            to: 1,
        },
    );
    assert_eq!(error_in(&out), None);
    let order: Vec<Uuid> = list_in(&out)
        .unwrap()
        .waiting
        .iter()
        .map(|w| w.id)
        .collect();
    assert_eq!(order, vec![id(3), id(2)]);
    let positions: Vec<u32> = list_in(&out)
        .unwrap()
        .waiting
        .iter()
        .map(|w| w.position)
        .collect();
    assert_eq!(positions, vec![1, 2]);
}

#[test]
fn c3_the_current_job_cannot_be_moved_and_a_stale_position_is_refused_with_a_fresh_list() {
    let mut r = three_waiting();
    let current = r.msg(
        &laptop(),
        Message::MoveJob {
            job_id: id(1),
            from: 1,
            to: 2,
        },
    );
    assert_eq!(error_in(&current), Some(ErrorCode::WrongState));
    assert!(
        list_in(&current).is_some(),
        "WrongState comes with a JobList"
    );

    // Job 2 is at position 1; a client that still thinks it is at 2 is stale.
    let stale = r.msg(
        &laptop(),
        Message::MoveJob {
            job_id: id(2),
            from: 2,
            to: 1,
        },
    );
    assert_eq!(error_in(&stale), Some(ErrorCode::WrongState));
    let fresh = list_in(&stale).expect("a stale position answers with a fresh JobList");
    let order: Vec<Uuid> = fresh.waiting.iter().map(|w| w.id).collect();
    assert_eq!(
        order,
        vec![id(2), id(3)],
        "the refused move changes nothing"
    );
}

// ---------------------------------------------------------------------------------------------
// Criterion 5: member rights (Frage 257 A).
// ---------------------------------------------------------------------------------------------

#[test]
fn c5_a_member_moves_are_owner_required_and_it_removes_and_flags_only_its_own_jobs() {
    // A configured connector has owners; a foreground one makes every paired client an owner.
    let mut r = three_waiting_with(StartRule::OwnerConfirms); // 2 by Phone, 3 by Laptop
    let mv = r.msg(
        &phone(),
        Message::MoveJob {
            job_id: id(2),
            from: 1,
            to: 2,
        },
    );
    assert_eq!(
        error_in(&mv),
        Some(ErrorCode::OwnerRequired),
        "{:?}",
        mv.messages
    );

    let others = r.msg(&phone(), Message::RemoveJob { job_id: id(3) });
    assert_eq!(error_in(&others), Some(ErrorCode::OwnerRequired));
    let flag_other = r.msg(
        &phone(),
        Message::SetRunNext {
            job_id: id(3),
            on: true,
        },
    );
    assert_eq!(error_in(&flag_other), Some(ErrorCode::OwnerRequired));

    let flag_own = r.msg(
        &phone(),
        Message::SetRunNext {
            job_id: id(2),
            on: true,
        },
    );
    assert_eq!(error_in(&flag_own), None);
    assert!(list_in(&flag_own).unwrap().waiting[0].run_next);

    let own = r.msg(&phone(), Message::RemoveJob { job_id: id(2) });
    assert_eq!(error_in(&own), None);
    let left: Vec<Uuid> = list_in(&own)
        .unwrap()
        .waiting
        .iter()
        .map(|w| w.id)
        .collect();
    assert_eq!(left, vec![id(3)], "the member removed its own waiting job");
}

#[test]
fn c5_an_owner_removes_others_jobs_and_flags_any_job() {
    let mut r = three_waiting();
    let removed = r.msg(&laptop(), Message::RemoveJob { job_id: id(2) });
    assert_eq!(error_in(&removed), None);
    let flagged = r.msg(
        &laptop(),
        Message::SetRunNext {
            job_id: id(3),
            on: true,
        },
    );
    assert_eq!(error_in(&flagged), None);
    assert!(list_in(&flagged).unwrap().waiting[0].run_next);
}

// ---------------------------------------------------------------------------------------------
// Criterion 4: Start on the first waiting job once the current one is done.
// ---------------------------------------------------------------------------------------------

#[test]
fn c4_an_owners_start_of_the_first_waiting_job_runs_after_done_and_not_of_others() {
    let mut r = three_waiting();
    let done = r.ev(Event::JobFinished { elapsed_ms: 5 });
    assert!(
        starts(&done).is_empty(),
        "nothing starts by itself (no mark)"
    );
    assert_eq!(
        r.m.state(),
        State::Done {
            result: JobResult::Completed
        }
    );

    let wrong = r.start(&laptop(), 3);
    assert_eq!(
        error_in(&wrong),
        Some(ErrorCode::WrongState),
        "not the first"
    );
    assert_eq!(starts(&wrong), Vec::<Uuid>::new());

    let ok = r.start(&laptop(), 2);
    assert_eq!(starts(&ok), vec![id(2)]);
    assert_eq!(r.m.state(), State::Running);
    assert_eq!(r.status_job(), Some(id(2)));
}

#[test]
fn c4_a_start_while_the_current_job_still_runs_is_refused() {
    let mut r = three_waiting();
    let out = r.start(&laptop(), 2);
    assert_eq!(error_in(&out), Some(ErrorCode::WrongState));
    assert_eq!(starts(&out), Vec::<Uuid>::new());
}

#[test]
fn c4_a_members_start_of_the_first_waiting_job_waits_for_an_owner() {
    let mut r = Rig::new(StartRule::OwnerConfirms);
    r.run_first(1, &laptop());
    r.send(2, &phone(), "b.gcode");
    r.ev(Event::JobFinished { elapsed_ms: 5 });
    let asked = r.start(&phone(), 2);
    assert!(
        starts(&asked).is_empty(),
        "a member's start does not run at once"
    );
    assert_eq!(r.m.state(), State::AwaitingLocalStart);
    let confirmed = r.start(&laptop(), 2);
    assert_eq!(starts(&confirmed), vec![id(2)]);
}

// ---------------------------------------------------------------------------------------------
// Criterion 7: Stop stops only the current job and the list keeps the rest.
// ---------------------------------------------------------------------------------------------

#[test]
fn c7_stop_stops_only_the_current_job_and_keeps_the_list() {
    let mut r = three_waiting();
    // A connector without a confirming controller takes Stop at once (`Machine::new`).
    let end = r.msg(&laptop(), Message::Stop);
    assert_eq!(
        r.m.state(),
        State::Done {
            result: JobResult::Stopped
        }
    );
    // The list was not changed by the stop (the ended job stays the held current job), so
    // no JobList is due; the rest of the list is shown by the owner's Start of the first one.
    let _ = end;
    let next = r.start(&laptop(), 2);
    assert_eq!(
        starts(&next),
        vec![id(2)],
        "job 2 is still first in the list"
    );
    r.ev(Event::JobFinished { elapsed_ms: 1 });
    let last = r.start(&laptop(), 3);
    assert_eq!(starts(&last), vec![id(3)], "job 3 is still behind it");
}

// ---------------------------------------------------------------------------------------------
// Criterion 11: run next automatically (Frage 259 B, 303 A). Table: end x flag x start rule.
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq)]
enum End {
    Completed,
    Failed,
    Stopped,
    Alarm,
    Lost,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Flag {
    None,
    Owner,
    Member,
}

fn end_current(r: &mut Rig, end: End) -> Output {
    match end {
        End::Completed => r.ev(Event::JobFinished { elapsed_ms: 3 }),
        End::Failed => r.ev(Event::JobFailed { elapsed_ms: 3 }),
        End::Stopped => r.msg(&laptop(), Message::Stop),
        End::Alarm => r.ev(Event::Alarm {
            text: "Hard limit".into(),
        }),
        End::Lost => r.ev(Event::ControllerLost),
    }
}

/// A's run with B waiting and flagged by `flag` (`sender` is B's sender).
fn chain_rig(rule: StartRule, flag: Flag) -> Rig {
    let mut r = Rig::new(rule);
    let (sender, flagger) = match flag {
        Flag::Member => (phone(), phone()),
        _ => (laptop(), laptop()),
    };
    r.run_first(1, &laptop());
    r.send(2, &sender, "b.gcode");
    if flag != Flag::None {
        let out = r.msg(
            &flagger,
            Message::SetRunNext {
                job_id: id(2),
                on: true,
            },
        );
        assert_eq!(error_in(&out), None, "{flag:?} flags its own job");
    }
    r
}

#[test]
fn c11_completed_chain_table() {
    for rule in [
        StartRule::Immediate,
        StartRule::LocalPress,
        StartRule::OwnerConfirms,
    ] {
        for flag in [Flag::None, Flag::Owner, Flag::Member] {
            let mut r = chain_rig(rule, flag);
            let out = end_current(&mut r, End::Completed);
            let auto = match flag {
                Flag::None => false,
                Flag::Owner => rule != StartRule::LocalPress,
                Flag::Member => rule == StartRule::Immediate,
            };
            let case = format!("rule {rule:?}, flag {flag:?}");
            if auto {
                assert_eq!(starts(&out), vec![id(2)], "{case}: runs at once");
                assert_eq!(r.m.state(), State::Running, "{case}");
            } else {
                assert_eq!(
                    starts(&out),
                    Vec::<Uuid>::new(),
                    "{case}: no start by itself"
                );
                if flag == Flag::None {
                    assert_eq!(
                        r.m.state(),
                        State::Done {
                            result: JobResult::Completed
                        },
                        "{case}"
                    );
                } else {
                    assert_eq!(
                        r.m.state(),
                        State::AwaitingLocalStart,
                        "{case}: waits for an owner"
                    );
                }
            }
            if flag != Flag::None {
                assert_eq!(
                    list_in(&out).and_then(|s| s.note),
                    None,
                    "{case}: no chain stop"
                );
            }
        }
    }
}

#[test]
fn c11_a_flagged_owner_job_starts_after_the_machine_press_under_local_start() {
    let mut r = chain_rig(StartRule::LocalPress, Flag::Owner);
    end_current(&mut r, End::Completed);
    let out = r.ev(Event::LocalStartPressed);
    assert_eq!(starts(&out), vec![id(2)]);
}

#[test]
fn c11_a_member_flagged_job_waits_for_an_owner_confirmation() {
    let mut r = chain_rig(StartRule::OwnerConfirms, Flag::Member);
    let out = end_current(&mut r, End::Completed);
    assert_eq!(starts(&out), Vec::<Uuid>::new());
    assert_eq!(r.m.state(), State::AwaitingLocalStart);
    let confirmed = r.start(&laptop(), 2);
    assert_eq!(starts(&confirmed), vec![id(2)]);
}

#[test]
fn c11_every_other_end_cancels_the_chain_clears_flags_and_says_so() {
    for end in [End::Failed, End::Stopped, End::Alarm, End::Lost] {
        for rule in [
            StartRule::Immediate,
            StartRule::LocalPress,
            StartRule::OwnerConfirms,
        ] {
            for flag in [Flag::Owner, Flag::Member] {
                let mut r = chain_rig(rule, flag);
                let out = end_current(&mut r, end);
                let case = format!("{end:?}, rule {rule:?}, flag {flag:?}");
                assert_eq!(
                    starts(&out),
                    Vec::<Uuid>::new(),
                    "{case}: no job starts after it"
                );
                let snap = list_in(&out).unwrap_or_else(|| panic!("{case}: no JobList"));
                assert_eq!(snap.waiting.len(), 1, "{case}: the job waits, not started");
                assert!(!snap.waiting[0].run_next, "{case}: flag cleared");
                let note = snap.note.unwrap_or_else(|| panic!("{case}: no note"));
                assert!(note.starts_with("Chain stopped: "), "{case}: {note}");
            }
        }
    }
}

#[test]
fn c11_chain_of_three_with_a_failure_in_the_second_starts_only_the_first_follower() {
    let mut r = Rig::new(StartRule::Immediate);
    r.run_first(1, &laptop());
    r.send(2, &laptop(), "b.gcode");
    r.send(3, &laptop(), "c.gcode");
    r.msg(
        &laptop(),
        Message::SetRunNext {
            job_id: id(2),
            on: true,
        },
    );
    r.msg(
        &laptop(),
        Message::SetRunNext {
            job_id: id(3),
            on: true,
        },
    );

    let first = r.ev(Event::JobFinished { elapsed_ms: 3 });
    assert_eq!(starts(&first), vec![id(2)]);
    let second = r.ev(Event::JobFailed { elapsed_ms: 3 });
    assert!(
        starts(&second).is_empty(),
        "C must not start after B failed"
    );
    let snap = list_in(&second).unwrap();
    assert_eq!(snap.waiting.len(), 1);
    assert_eq!(snap.waiting[0].id, id(3));
    assert!(!snap.waiting[0].run_next);
    assert!(snap.note.unwrap().starts_with("Chain stopped: "));

    let by_owner = r.start(&laptop(), 3);
    assert_eq!(
        starts(&by_owner),
        vec![id(3)],
        "only an owner's Start continues"
    );
}

/// D1 (Frage 303 A): the member's flag is carried out as a request that names the member.
#[test]
fn c11_a_member_flagged_job_shows_who_asked_while_it_waits_for_an_owner() {
    let mut r = chain_rig(StartRule::OwnerConfirms, Flag::Member);
    end_current(&mut r, End::Completed);
    assert_eq!(r.m.state(), State::AwaitingLocalStart);
    match r.m.status() {
        Message::Status { requested_by, .. } => assert_eq!(requested_by.as_deref(), Some("Phone")),
        other => panic!("{other:?}"),
    }
}

/// D2 (ADR L2): a listed job that is not the first waiting one, or that follows a held job, is
/// `WrongState`; only an id the connector does not know is `UnknownJob`.
#[test]
fn c4_a_start_of_a_listed_job_that_may_not_start_now_is_wrong_state_in_every_rule() {
    for rule in [
        StartRule::Immediate,
        StartRule::LocalPress,
        StartRule::OwnerConfirms,
    ] {
        let mut r = Rig::new(rule);
        r.run_first(1, &laptop());
        r.send(2, &laptop(), "b.gcode");
        r.send(3, &laptop(), "c.gcode");
        r.ev(Event::JobFinished { elapsed_ms: 3 });
        // Job 2 is first, 3 is not: Start 3 while 1 is Done.
        let out = r.start(&laptop(), 3);
        assert_eq!(error_in(&out), Some(ErrorCode::WrongState), "{rule:?} Done");
        // Start 2 makes it the held job; Start 3 while it waits (Ready / AwaitingLocalStart).
        let starter = if rule == StartRule::OwnerConfirms {
            phone()
        } else {
            laptop()
        };
        r.start(&starter, 2);
        let out = r.start(&laptop(), 3);
        if r.m.state() != State::Running {
            assert_eq!(
                error_in(&out),
                Some(ErrorCode::WrongState),
                "{rule:?} {:?}",
                r.m.state()
            );
        }
        if rule == StartRule::OwnerConfirms {
            assert_eq!(r.m.state(), State::AwaitingLocalStart);
            let out = r.start(&laptop(), 99);
            assert_eq!(error_in(&out), Some(ErrorCode::UnknownJob), "{rule:?}");
        }
    }
}
