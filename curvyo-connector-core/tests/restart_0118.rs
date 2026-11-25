//! Restart from a line on both machines (`specs/0118` criteria 38, 39; `specs/0132` criteria 12,
//! 13; adrs R2 to R9): the offered line, the causes in order, the accept, the move to the line
//! and every end before it, which returns the entry unchanged.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::unnecessary_wraps)]

mod common;

use std::fmt::Write as _;
use std::sync::Arc;

use common::sha256_hex;
use curvyo_connector_core::{
    ControllerCommand, Entry, ErrorCode, Event, Item, JobResult, ListEntry, ListLog, Machine,
    Message, Millimetres, Options, Output, Peer, Role, StartRule, State, StopPoint,
};
use uuid::Uuid;

/// Lines 1 to 3 set the state, 4 to 43 are forty burn moves, 44 ends.
fn text() -> String {
    let mut text = "G21\nG90\nM4 S0\n".to_owned();
    for i in 0..40 {
        let _ = writeln!(text, "G1 X{i} Y5 F600 S500");
    }
    text.push_str("M5\n");
    text
}

fn id() -> Uuid {
    Uuid::from_u128(7)
}

fn mm(v: [f64; 3]) -> Option<[Millimetres; 3]> {
    Some(v.map(Millimetres))
}

fn entry(text: &str, acked: u32, offset: [f64; 3]) -> Entry {
    Entry {
        job_id: id(),
        name: "sign.gcode".into(),
        lines: u32::try_from(text.lines().count()).unwrap(),
        bytes: text.len() as u64,
        sha256: sha256_hex(text),
        sent_by: "Phone".into(),
        run_next_by: None,
        interrupted_at: Some(acked),
        stop: Some(StopPoint {
            machine_position: mm([-100.0, -100.0, 0.0]),
            work_offset: mm(offset),
            run_from: 1,
            exact: true,
        }),
    }
}

fn owner() -> Peer {
    Peer::new("Laptop", Role::Owner)
}

fn member() -> Peer {
    Peer::new("Phone", Role::Member)
}

fn msg(machine: &mut Machine, message: Message, peer: &Peer) -> Output {
    machine.handle(Event::Message(message, peer.clone()))
}

/// A connected machine with the interrupted job restored, its position reported, homed.
fn ready(mut machine: Machine, text: &str) -> Machine {
    machine.handle(Event::ControllerConnected);
    machine.restore(vec![Item {
        entry: entry(text, 43, [0.0; 3]),
        text: Arc::from(text),
    }]);
    machine.handle(Event::MachinePosition {
        machine: mm([-50.0, -50.0, 0.0]),
        offset: mm([0.0; 3]),
    });
    machine.handle(Event::Homed);
    machine
}

fn interrupted(machine: &Machine) -> Option<ListEntry> {
    match machine.job_list() {
        Message::JobList { waiting, .. } => {
            waiting.into_iter().find(|e| e.interrupted_at.is_some())
        }
        _ => None,
    }
}

fn cause(machine: &Machine) -> Option<String> {
    interrupted(machine).and_then(|e| e.restart_cause)
}

fn errors(out: &Output) -> Vec<(ErrorCode, String)> {
    out.messages
        .iter()
        .filter_map(|m| match m {
            Message::Error { code, message } => Some((*code, message.clone())),
            _ => None,
        })
        .collect()
}

fn progress(machine: &Machine) -> (u32, Option<u32>, bool) {
    match machine.status() {
        Message::Status { job: Some(p), .. } => (p.line, p.from_line, p.entering),
        other => panic!("{other:?}"),
    }
}

fn detail(machine: &Machine) -> Option<String> {
    match machine.status() {
        Message::Status { detail, .. } => detail,
        _ => None,
    }
}

#[test]
fn the_line_backs_off_by_the_planner_and_the_causes_follow_the_position() {
    let text = text();
    let mut machine = Machine::new(Options::default());
    machine.handle(Event::ControllerConnected);
    machine.restore(vec![Item {
        entry: entry(&text, 43, [0.0; 3]),
        text: Arc::from(text.as_str()),
    }]);
    let shown = interrupted(&machine).unwrap();
    assert_eq!(shown.restart_line, Some(28), "16 blocks until `$I` is read");
    assert!(!shown.line_approximate);
    assert_eq!(
        cause(&machine).as_deref(),
        Some("No position from the machine")
    );
    machine.handle(Event::Planner { blocks: 15 });
    assert_eq!(interrupted(&machine).unwrap().restart_line, Some(29));
    machine.handle(Event::MachinePosition {
        machine: mm([-50.0, -50.0, 0.0]),
        offset: mm([0.0; 3]),
    });
    assert_eq!(cause(&machine).as_deref(), Some("Not homed"));
    machine.handle(Event::Homed);
    assert_eq!(cause(&machine), None);
    machine.handle(Event::MachinePosition {
        machine: mm([-50.0, -50.0, 0.0]),
        offset: mm([5.0, 0.0, 0.0]),
    });
    assert_eq!(
        cause(&machine).as_deref(),
        Some("Work offset changed"),
        "also homed"
    );
}

#[test]
fn a_crash_line_reads_near_and_a_slice_one_file_has_no_position() {
    let text = text();
    let mut near = entry(&text, 43, [0.0; 3]);
    near.stop.as_mut().unwrap().exact = false;
    let mut old = entry(&text, 43, [0.0; 3]);
    old.job_id = Uuid::from_u128(8);
    old.stop = None;
    let mut machine = Machine::new(Options::default());
    machine.handle(Event::ControllerConnected);
    machine.restore(vec![
        Item {
            entry: near,
            text: Arc::from(text.as_str()),
        },
        Item {
            entry: old,
            text: Arc::from(text.as_str()),
        },
    ]);
    machine.handle(Event::MachinePosition {
        machine: mm([0.0; 3]),
        offset: mm([0.0; 3]),
    });
    machine.handle(Event::Homed);
    let Message::JobList { waiting, .. } = machine.job_list() else {
        panic!()
    };
    assert!(waiting[0].line_approximate);
    assert_eq!(waiting[0].restart_cause, None);
    assert!(waiting[1].line_approximate);
    assert_eq!(
        waiting[1].restart_cause.as_deref(),
        Some("No position from the machine")
    );
}

#[test]
fn the_job_text_comes_first_then_the_state_then_the_position() {
    let head = "G21\nG91\nM4 S0\n".to_owned() + &"G1 X1 Y0 F600 S500\n".repeat(41);
    let mut machine = Machine::new(Options::default());
    machine.handle(Event::ControllerConnected);
    machine.restore(vec![Item {
        entry: entry(&head, 43, [0.0; 3]),
        text: Arc::from(head.as_str()),
    }]);
    assert_eq!(
        cause(&machine).as_deref(),
        Some("Starts at the head position")
    );

    let text = text();
    let mut machine = ready(Machine::new(Options::default()), &text);
    machine.handle(Event::MachinePosition {
        machine: mm([0.0; 3]),
        offset: mm([0.0; 3]),
    });
    let table: [(Event, &str); 4] = [
        (
            Event::ControllerWord(Some("Door".into())),
            "Controller state: Door",
        ),
        (Event::ControllerWord(None), ""),
        (Event::LaserModeOff(true), "Laser mode is off ($32=0)"),
        (Event::Alarm { text: "x".into() }, "Alarm"),
    ];
    for (event, expected) in table {
        machine.handle(event);
        assert_eq!(cause(&machine).unwrap_or_default(), expected);
    }
}

#[test]
fn another_ready_job_comes_before_not_homed() {
    let text = text();
    let mut machine = Machine::new(Options::default());
    machine.handle(Event::ControllerConnected);
    let job = "G0 X1\n";
    let other = Uuid::from_u128(99);
    msg(
        &mut machine,
        Message::SubmitJob {
            job_id: other,
            name: "box.gcode".into(),
            kind: curvyo_connector_core::JobKind::Job,
            bytes: job.len() as u64,
            sha256: sha256_hex(job),
        },
        &owner(),
    );
    msg(
        &mut machine,
        Message::JobChunk {
            job_id: other,
            index: 0,
            text: job.into(),
        },
        &owner(),
    );
    machine.restore(vec![Item {
        entry: entry(&text, 43, [0.0; 3]),
        text: Arc::from(text.as_str()),
    }]);
    machine.handle(Event::MachinePosition {
        machine: mm([0.0; 3]),
        offset: mm([0.0; 3]),
    });
    assert_eq!(machine.state(), State::Ready);
    assert_eq!(cause(&machine).as_deref(), Some("Another job is ready"));
    let out = msg(
        &mut machine,
        Message::RestartFromLine {
            job_id: id(),
            line: 28,
        },
        &owner(),
    );
    assert_eq!(
        errors(&out),
        [(ErrorCode::WrongState, "Another job is ready".into())]
    );
    assert_eq!(out.commands, []);
}

fn accepted(machine: &mut Machine) -> Output {
    msg(
        machine,
        Message::RestartFromLine {
            job_id: id(),
            line: 28,
        },
        &owner(),
    )
}

#[test]
fn an_owner_restarts_the_entry_moves_to_its_line_then_runs_as_a_job() {
    let text = text();
    for confirming in [false, true] {
        let options = Options::default();
        let base = if confirming {
            Machine::confirming(options)
        } else {
            Machine::new(options)
        };
        let mut machine = ready(base, &text);
        let out = accepted(&mut machine);
        assert_eq!(errors(&out), []);
        let [
            ControllerCommand::StartAt {
                job_id,
                from_line,
                entry,
                target,
                homed,
            },
        ] = out.commands.as_slice()
        else {
            panic!("{:?}", out.commands)
        };
        assert_eq!((*job_id, *from_line, *homed), (id(), 28, true));
        assert_eq!(target, &[Millimetres(23.0), Millimetres(5.0)]);
        assert_eq!(
            entry,
            &[
                "M5",
                "M9",
                "G21 G90 G17 G94",
                "G1 X23.000 Y5.000 F3000 S0",
                "G21 G90 G17 G94",
                "G1 F600",
                "S500 M4"
            ]
        );
        assert_eq!(
            out.list_log,
            [ListLog::Restarted {
                job_id: id(),
                name: "sign.gcode".into(),
                line: 28,
                by: "Laptop".into(),
            }]
        );
        assert_eq!(machine.state(), State::Running);
        assert_eq!(progress(&machine), (27, Some(28), true));
        assert_eq!(interrupted(&machine), None, "the row leaves the list");
        let Message::JobList { current, .. } = machine.job_list() else {
            panic!()
        };
        assert_eq!(current.unwrap().job_id, id());
        machine.handle(Event::Progress {
            line: 27,
            elapsed_ms: 10,
        });
        assert!(progress(&machine).2, "entry lines count no job line");
        machine.handle(Event::EntryReached);
        assert_eq!(progress(&machine), (27, Some(28), false));
        machine.handle(Event::Progress {
            line: 44,
            elapsed_ms: 900,
        });
        machine.handle(Event::JobFinished { elapsed_ms: 1000 });
        assert_eq!(
            machine.state(),
            State::Done {
                result: JobResult::Completed
            }
        );
        assert_eq!(progress(&machine), (44, Some(28), false));
    }
}

#[test]
fn an_answered_job_line_does_not_end_entering_only_the_arrival_does() {
    // GRBL's `ok` means "in the planner": the head may still travel.
    let text = text();
    let mut machine = ready(Machine::new(Options::default()), &text);
    accepted(&mut machine);
    machine.handle(Event::Progress {
        line: 30,
        elapsed_ms: 10,
    });
    assert_eq!(progress(&machine), (30, Some(28), true));
    machine.handle(Event::EntryReached);
    assert_eq!(progress(&machine), (30, Some(28), false));
}

#[test]
fn a_stale_line_an_unknown_job_and_a_member_are_refused() {
    let text = text();
    let configured = Options {
        start_rule: StartRule::OwnerConfirms,
    };
    let mut machine = ready(Machine::new(configured), &text);
    let out = msg(
        &mut machine,
        Message::RestartFromLine {
            job_id: id(),
            line: 29,
        },
        &owner(),
    );
    assert_eq!(errors(&out)[0].0, ErrorCode::WrongState);
    assert!(
        out.messages
            .iter()
            .any(|m| matches!(m, Message::JobList { .. })),
        "a fresh list"
    );
    let out = msg(
        &mut machine,
        Message::RestartFromLine {
            job_id: Uuid::from_u128(1),
            line: 28,
        },
        &owner(),
    );
    assert_eq!(errors(&out)[0].0, ErrorCode::UnknownJob);
    let out = msg(
        &mut machine,
        Message::RestartFromLine {
            job_id: id(),
            line: 28,
        },
        &member(),
    );
    assert_eq!(errors(&out)[0].0, ErrorCode::OwnerRequired);
    assert_eq!(out.commands, []);
    assert_eq!(machine.state(), State::Idle);
}

/// The entry is back as it was, the machine `Idle` with `detail`.
fn back_unchanged(machine: &Machine, out: &Output, expected: Option<&str>) {
    let text = text();
    let shown = interrupted(machine).expect("the row is back");
    assert_eq!(shown.interrupted_at, Some(43));
    assert_eq!(shown.restart_line, Some(28));
    assert_eq!(
        machine.stored().waiting[0],
        entry(&text, 43, [0.0; 3]),
        "unchanged"
    );
    assert!(
        out.list_log
            .iter()
            .any(|l| matches!(l, ListLog::RestartReturned { line: 28, .. })),
        "{:?}",
        out.list_log
    );
    if let Some(expected) = expected {
        assert_eq!(machine.state(), State::Idle);
        assert_eq!(detail(machine).as_deref(), Some(expected));
    }
}

#[test]
fn stop_before_the_line_returns_the_entry_on_both_machines() {
    let text = text();
    let not_started = "Not started: Restart from line 28";
    let mut machine = ready(Machine::new(Options::default()), &text);
    accepted(&mut machine);
    let out = msg(&mut machine, Message::Stop, &member());
    assert_eq!(out.commands, [ControllerCommand::Stop]);
    back_unchanged(&machine, &out, Some(not_started));

    let mut machine = ready(Machine::confirming(Options::default()), &text);
    accepted(&mut machine);
    let out = msg(&mut machine, Message::Stop, &member());
    assert_eq!(out.commands, [ControllerCommand::Stop]);
    assert_eq!(
        machine.state(),
        State::Running,
        "until the controller confirms"
    );
    let out = machine.handle(Event::Stopped);
    back_unchanged(&machine, &out, Some(not_started));
    // Homed since connect stays (the asked reset): Restart is enabled again at once.
    assert_eq!(cause(&machine), None);
}

#[test]
fn a_failure_an_alarm_and_a_loss_before_the_line_return_the_entry() {
    let text = text();
    let mut machine = ready(Machine::new(Options::default()), &text);
    accepted(&mut machine);
    let out = machine.handle(Event::JobFailedWith {
        elapsed_ms: 0,
        detail: "Outside the travel by 12 mm".into(),
    });
    back_unchanged(&machine, &out, Some("Outside the travel by 12 mm"));

    let mut machine = ready(Machine::new(Options::default()), &text);
    accepted(&mut machine);
    let out = machine.handle(Event::Alarm {
        text: "Hard limit (alarm 1)".into(),
    });
    back_unchanged(&machine, &out, None);
    assert_eq!(machine.state(), State::Alarm);

    let mut machine = ready(Machine::new(Options::default()), &text);
    accepted(&mut machine);
    let out = machine.handle(Event::ControllerLost);
    back_unchanged(&machine, &out, None);
    assert_eq!(machine.state(), State::NoController);
    assert_eq!(detail(&machine).as_deref(), Some("Lost the controller"));
}

#[test]
fn under_local_press_the_restart_waits_for_enter_and_stop_discards_it() {
    let text = text();
    let local = Options {
        start_rule: StartRule::LocalPress,
    };
    let mut machine = ready(Machine::new(local), &text);
    let out = accepted(&mut machine);
    assert_eq!(out.commands, []);
    assert_eq!(machine.state(), State::AwaitingLocalStart);
    assert_eq!(progress(&machine), (27, Some(28), false));
    let out = machine.handle(Event::LocalStartPressed);
    assert!(matches!(
        out.commands.as_slice(),
        [ControllerCommand::StartAt { from_line: 28, .. }]
    ));
    assert_eq!(progress(&machine), (27, Some(28), true));

    let mut machine = ready(Machine::new(local), &text);
    accepted(&mut machine);
    let out = msg(&mut machine, Message::Stop, &owner());
    assert_eq!(out.commands, []);
    back_unchanged(&machine, &out, Some("Not started: Restart from line 28"));
}

#[test]
fn the_store_keeps_the_entry_until_the_line_then_the_run_with_its_stop_point() {
    let text = text();
    let mut machine = ready(Machine::new(Options::default()), &text);
    accepted(&mut machine);
    let stored = machine.stored_at(true);
    assert_eq!(stored.current, None);
    assert_eq!(stored.waiting, [entry(&text, 43, [0.0; 3])]);
    assert_eq!(machine.stored_texts().len(), 1);
    machine.handle(Event::EntryReached);
    machine.handle(Event::Progress {
        line: 30,
        elapsed_ms: 10,
    });
    let current = machine.stored_at(true).current.unwrap();
    assert!(current.started);
    assert_eq!(current.line, 30);
    let stop = current.entry.stop.unwrap();
    assert_eq!((stop.run_from, stop.exact), (28, true));
    assert_eq!(stop.machine_position, mm([-50.0, -50.0, 0.0]));
    assert!(!machine.stored().current.unwrap().entry.stop.unwrap().exact);
}

/// A connected, homed machine running the job `id()` with 43 lines acknowledged.
fn running(machine: Machine) -> Machine {
    let text = text();
    let mut machine = machine;
    machine.handle(Event::ControllerConnected);
    machine.handle(Event::MachinePosition {
        machine: mm([-50.0, -50.0, 0.0]),
        offset: mm([0.0; 3]),
    });
    machine.handle(Event::Homed);
    let submit = Message::SubmitJob {
        job_id: id(),
        name: "sign.gcode".into(),
        kind: curvyo_connector_core::JobKind::Job,
        bytes: text.len() as u64,
        sha256: sha256_hex(&text),
    };
    msg(&mut machine, submit, &owner());
    let chunk = Message::JobChunk {
        job_id: id(),
        index: 0,
        text: text.clone(),
    };
    msg(&mut machine, chunk, &owner());
    msg(&mut machine, Message::Start { job_id: id() }, &owner());
    machine.handle(Event::Progress {
        line: 43,
        elapsed_ms: 100,
    });
    machine.handle(Event::MachinePosition {
        machine: mm([-30.0, -40.0, 0.0]),
        offset: mm([0.0; 3]),
    });
    machine
}

#[test]
fn frage_390_c_a_stop_an_alarm_and_a_loss_leave_an_interrupted_entry() {
    // Stop: exact at the acknowledged line, the line backs off by the planner.
    let mut machine = running(Machine::new(Options::default()));
    msg(&mut machine, Message::Stop, &owner());
    let shown = interrupted(&machine).expect("listed");
    assert_eq!(shown.interrupted_at, Some(43));
    assert_eq!(shown.restart_line, Some(28));
    assert!(!shown.line_approximate);
    let stop = machine.stored().waiting[0].stop.clone().unwrap();
    assert_eq!(stop.machine_position, mm([-30.0, -40.0, 0.0]));
    assert!(stop.exact);
    assert_eq!(cause(&machine), None);
    // Nothing broke since the Stop and the head stands there: no Home needed, also after a
    // homing flag is gone; a reset breaks it.
    machine.handle(Event::ControllerReset);
    assert_eq!(cause(&machine).as_deref(), Some("Not homed"));

    let mut machine = running(Machine::confirming(Options::default()));
    msg(&mut machine, Message::Stop, &owner());
    assert_eq!(interrupted(&machine), None, "until the controller confirms");
    machine.handle(Event::Stopped);
    assert_eq!(interrupted(&machine).unwrap().interrupted_at, Some(43));

    // An alarm: listed, the position needs a Home.
    let mut machine = running(Machine::new(Options::default()));
    machine.handle(Event::Alarm {
        text: "Hard limit (alarm 1)".into(),
    });
    assert_eq!(interrupted(&machine).unwrap().interrupted_at, Some(43));
    machine.handle(Event::AlarmCleared);
    assert_eq!(cause(&machine).as_deref(), Some("Not homed"));
    machine.handle(Event::Homed);
    assert_eq!(cause(&machine), None);

    // A loss: listed with the position it had.
    let mut machine = running(Machine::new(Options::default()));
    machine.handle(Event::ControllerLost);
    let entry = machine.stored().waiting[0].clone();
    assert_eq!(entry.interrupted_at, Some(43));
    assert_eq!(
        entry.stop.unwrap().machine_position,
        mm([-30.0, -40.0, 0.0])
    );
}

#[test]
fn frage_390_c_completed_and_failed_jobs_are_never_interrupted() {
    let mut machine = running(Machine::new(Options::default()));
    machine.handle(Event::JobFinished { elapsed_ms: 200 });
    assert_eq!(interrupted(&machine), None);
    let mut machine = running(Machine::new(Options::default()));
    machine.handle(Event::JobFailed { elapsed_ms: 200 });
    assert_eq!(interrupted(&machine), None);
}

#[test]
fn frage_390_c_a_restarted_job_stopped_after_its_line_is_interrupted_again_from_it() {
    let text = text();
    let mut machine = ready(Machine::new(Options::default()), &text);
    accepted(&mut machine);
    machine.handle(Event::EntryReached);
    machine.handle(Event::Progress {
        line: 30,
        elapsed_ms: 10,
    });
    msg(&mut machine, Message::Stop, &owner());
    let entry = machine.stored().waiting[0].clone();
    assert_eq!(entry.interrupted_at, Some(30));
    assert_eq!(entry.stop.unwrap().run_from, 28);
    assert_eq!(
        interrupted(&machine).unwrap().restart_line,
        Some(28),
        "never before 28"
    );
}
