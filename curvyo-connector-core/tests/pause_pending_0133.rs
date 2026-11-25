//! `specs/0133` core: a pause the controller defers while a job's `$H` runs (`pause_pending`),
//! what clears it, and the refusal of Pause in the Home button's `Homing` (addendum P2, P3).
#![allow(clippy::unwrap_used, clippy::assert_is_empty)]

mod common;

use common::{JOB_TEXT, chunk, fixture, job_id, submit};
use curvyo_connector_core::Peer;
use curvyo_connector_core::{
    ControllerCommand, ErrorCode, Event, JobKind, JobResult, Machine, Message, Options, Output,
    Progress, State, decode, encode,
};

fn app(m: &mut Machine, message: Message) -> Output {
    m.handle(Event::Message(message, Peer::host()))
}

fn ready(mut m: Machine) -> Machine {
    m.handle(Event::ControllerConnected);
    app(&mut m, submit(JOB_TEXT, JobKind::Job));
    app(&mut m, chunk(0, JOB_TEXT));
    assert_eq!(m.state(), State::Ready);
    m
}

/// A confirming machine running the job, its controller homing (`Controller state: Home`).
fn running() -> Machine {
    let mut m = ready(Machine::confirming(Options::default()));
    app(&mut m, Message::Start { job_id: job_id() });
    m.handle(Event::ControllerWord(Some("Home".into())));
    assert_eq!(m.state(), State::Running);
    m
}

/// Running with the pause the controller deferred.
fn pending() -> Machine {
    let mut m = running();
    let out = app(&mut m, Message::Pause);
    assert_eq!(out.commands, [ControllerCommand::Pause]);
    assert!(out.messages.is_empty(), "the confirming machine waits");
    let out = m.handle(Event::PauseDeferred);
    assert_eq!(out.messages.len(), 1, "one Status answers the Pause");
    assert!(is_pending(&out.messages[0]));
    m
}

fn is_pending(message: &Message) -> bool {
    matches!(
        message,
        Message::Status {
            pause_pending: true,
            ..
        }
    )
}

fn flag(m: &Machine) -> bool {
    is_pending(&m.status())
}

#[test]
fn p2_status_pause_pending_matches_its_golden_file_and_old_files_are_unchanged() {
    let message = Message::Status {
        state: State::Running,
        job: Some(Progress {
            job_id: job_id(),
            line: 3,
            lines: 6,
            elapsed_ms: 1500,
            from_line: None,
            entering: false,
        }),
        position: None,
        alarm: None,
        detail: Some("Controller state: Home".into()),
        pause_pending: true,
        requested_by: None,
        job_name: None,
        sent_by: None,
        homed: false,
        machine_position: None,
        work_offset: None,
        frame_id: None,
        target: None,
    };
    let golden = fixture("messages/status_pause_pending.json");
    assert_eq!(encode(&message).unwrap(), golden);
    assert_eq!(decode(&golden).unwrap(), message);
    // Without the field it decodes as false and re-encodes byte for byte.
    let running = fixture("messages/status_running.json");
    let decoded = decode(&running).unwrap();
    assert!(!is_pending(&decoded));
    assert_eq!(encode(&decoded).unwrap(), running);
}

#[test]
fn ph1_a_deferred_pause_is_pending_in_running_with_the_line_unchanged() {
    let m = pending();
    assert_eq!(m.state(), State::Running);
    let Message::Status { job, detail, .. } = m.status() else {
        unreachable!()
    };
    assert_eq!(job.unwrap().line, 0);
    assert_eq!(detail.as_deref(), Some("Controller state: Home"));
}

#[test]
fn ph1_a_deferred_pause_outside_running_is_ignored() {
    let mut m = ready(Machine::confirming(Options::default()));
    assert!(m.handle(Event::PauseDeferred).messages.is_empty());
    assert!(!flag(&m));
    let mut m = running();
    app(&mut m, Message::Stop);
    assert!(
        m.handle(Event::PauseDeferred).messages.is_empty(),
        "stop pending"
    );
    assert!(!flag(&m));
}

#[test]
fn ph2_held_pauses_and_clears_the_flag() {
    let mut m = pending();
    m.handle(Event::Progress {
        line: 4,
        elapsed_ms: 9_000,
    });
    assert!(flag(&m), "progress keeps it");
    let out = m.handle(Event::Held);
    assert_eq!(m.state(), State::Paused);
    assert!(!is_pending(&out.messages[0]));
    assert!(!flag(&m));
}

#[test]
fn ph4_a_second_pause_is_idempotent_and_resume_cancels() {
    let mut m = pending();
    let out = app(&mut m, Message::Pause);
    assert_eq!(out, Output::default(), "nothing for a second Pause");
    let out = app(&mut m, Message::Resume);
    assert_eq!(out.commands, [ControllerCommand::Resume]);
    assert_eq!(out.messages.len(), 1);
    assert!(!is_pending(&out.messages[0]));
    assert_eq!(m.state(), State::Running);
    // Without a pending pause, Resume in Running stays a no-op.
    assert_eq!(app(&mut m, Message::Resume), Output::default());
}

#[test]
fn ph5_stop_clears_the_pending_pause_and_is_never_held_back() {
    let mut m = pending();
    let out = app(&mut m, Message::Stop);
    assert_eq!(out.commands, [ControllerCommand::Stop]);
    assert!(!is_pending(&out.messages[0]));
    let Message::Status { detail, .. } = m.status() else {
        unreachable!()
    };
    assert_eq!(detail.as_deref(), Some("Stopping"));
    m.handle(Event::Stopped);
    assert_eq!(
        m.state(),
        State::Done {
            result: JobResult::Stopped
        }
    );
}

#[test]
fn ph6_ph9_failures_drop_the_pending_pause_and_never_pause() {
    let ends: [Event; 4] = [
        Event::Alarm {
            text: "Homing did not find the switch (alarm 9)".into(),
        },
        Event::JobFailedWith {
            elapsed_ms: 10,
            detail: "Failed at line 4: Homing is not enabled (error 5)".into(),
        },
        Event::ControllerLost,
        Event::StopUnconfirmed,
    ];
    for end in ends {
        let mut m = pending();
        let out = m.handle(end.clone());
        assert!(out.messages.iter().all(|s| !is_pending(s)), "{end:?}");
        assert!(!flag(&m), "{end:?}");
        assert_ne!(m.state(), State::Paused, "{end:?}");
        m.handle(Event::Held);
        assert_ne!(
            m.state(),
            State::Paused,
            "{end:?}: a late hold changes nothing"
        );
    }
}

#[test]
fn ph3_pause_while_the_home_button_homes_is_no_job_to_pause() {
    let homing_idle = {
        let mut m = Machine::confirming(Options::default());
        m.handle(Event::ControllerConnected);
        m
    };
    for m in [
        homing_idle,
        ready(Machine::confirming(Options::default())),
        ready(Machine::new(Options::default())),
    ] {
        let mut m = m;
        app(&mut m, Message::Home);
        assert_eq!(m.state(), State::Homing);
        let out = app(&mut m, Message::Pause);
        assert_eq!(
            out.messages,
            [Message::Error {
                code: ErrorCode::WrongState,
                message: "No job to pause".into(),
            }]
        );
        assert!(out.commands.is_empty());
        assert_eq!(m.state(), State::Homing);
        assert!(!flag(&m));
        m.handle(Event::Homed);
        assert!(!flag(&m), "nothing remembered");
        assert_ne!(m.state(), State::Paused);
    }
}

#[test]
fn the_fake_controllers_machine_pauses_at_once() {
    let mut m = ready(Machine::new(Options::default()));
    app(&mut m, Message::Start { job_id: job_id() });
    app(&mut m, Message::Pause);
    assert_eq!(m.state(), State::Paused);
    assert!(!flag(&m));
}
