//! P13 to P20 and P23: the job state machine, driven with the fake controller.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::match_same_arms,
    clippy::single_match_else,
    clippy::assert_is_empty
)]

mod common;

use common::{JOB_TEXT, Rig, chunk, job_id, last_status, sha256_hex, submit};
use curvyo_connector_core::{
    ControllerCommand, ErrorCode, Event, JobKind, JobResult, Message, Millimetres, Options,
    RejectReason, State,
};
use curvyo_connector_core::{Peer, StartRule};

const REQUIRE: Options = Options {
    start_rule: StartRule::LocalPress,
};

fn is_error(m: &Message) -> bool {
    matches!(m, Message::Error { .. } | Message::JobRejected { .. })
}

fn done(result: JobResult) -> State {
    State::Done { result }
}

fn alarmed() -> Rig {
    let mut rig = Rig::plugged();
    let e = rig.controller.alarm("ALARM:1 Hard limit");
    rig.send(e);
    rig
}

fn running() -> Rig {
    let mut rig = Rig::ready(Options::default());
    rig.app(Message::Start { job_id: job_id() });
    rig
}

/// One rig per state, named by the state's `Debug` text.
fn reach(name: &str) -> Rig {
    match name {
        "NoController" => Rig::new(Options::default()),
        "Idle" => Rig::plugged(),
        "Receiving" => {
            let mut rig = Rig::plugged();
            rig.app(submit(JOB_TEXT, JobKind::Job));
            rig
        }
        "Ready" => Rig::ready(Options::default()),
        "AwaitingLocalStart" => {
            let mut rig = Rig::ready(REQUIRE);
            rig.app(Message::Start { job_id: job_id() });
            rig
        }
        "Running" => running(),
        "Paused" => {
            let mut rig = running();
            rig.app(Message::Pause);
            rig
        }
        "Done" => {
            let mut rig = running();
            let e = rig.controller.finish();
            rig.send(e);
            rig
        }
        "Alarm" => alarmed(),
        other => panic!("{other}"),
    }
}

const STATES: [&str; 9] = [
    "NoController",
    "Idle",
    "Receiving",
    "Ready",
    "AwaitingLocalStart",
    "Running",
    "Paused",
    "Done",
    "Alarm",
];
const EVENTS: [&str; 19] = [
    "Submit",
    "Chunk",
    "GetStatus",
    "Start",
    "StartOtherJob",
    "Pause",
    "Resume",
    "Stop",
    "Connected",
    "Lost",
    "Progress",
    "Position",
    "Finished",
    "Failed",
    "Alarm",
    "AlarmCleared",
    "LocalStart",
    "ClientDisconnected",
    "Hello",
];

fn event(name: &str) -> Event {
    let other: uuid::Uuid = "00000000-0000-4000-8000-000000000001".parse().unwrap();
    match name {
        "Submit" => Event::Message(submit(JOB_TEXT, JobKind::Job), Peer::host()),
        "Chunk" => Event::Message(chunk(0, JOB_TEXT), Peer::host()),
        "GetStatus" => Event::Message(Message::GetStatus, Peer::host()),
        "Start" => Event::Message(Message::Start { job_id: job_id() }, Peer::host()),
        "StartOtherJob" => Event::Message(Message::Start { job_id: other }, Peer::host()),
        "Pause" => Event::Message(Message::Pause, Peer::host()),
        "Resume" => Event::Message(Message::Resume, Peer::host()),
        "Stop" => Event::Message(Message::Stop, Peer::host()),
        "Connected" => Event::ControllerConnected,
        "Lost" => Event::ControllerLost,
        "Progress" => Event::Progress {
            line: 2,
            elapsed_ms: 500,
        },
        "Position" => Event::Position([Millimetres(1.0), Millimetres(2.0), Millimetres(0.0)]),
        "Finished" => Event::JobFinished { elapsed_ms: 1500 },
        "Failed" => Event::JobFailed { elapsed_ms: 900 },
        "Alarm" => Event::Alarm {
            text: "ALARM:1".to_owned(),
        },
        "AlarmCleared" => Event::AlarmCleared,
        "LocalStart" => Event::LocalStartPressed,
        "ClientDisconnected" => Event::ClientDisconnected,
        "Hello" => Event::Message(
            Message::Hello {
                protocol: 0,
                software: "x".into(),
                version: "0".into(),
            },
            Peer::host(),
        ),
        other => panic!("{other}"),
    }
}

/// The only pairs that change the state; the new state is its `Debug` text.
fn expected(state: &str, event: &str) -> Option<&'static str> {
    let with_controller = state != "NoController";
    match (state, event) {
        ("NoController", "Connected") => Some("Idle"),
        ("Idle" | "Done", "Submit") => Some("Receiving"),
        ("Receiving", "Chunk") => Some("Ready"),
        ("Receiving" | "Ready" | "AwaitingLocalStart", "Stop") => Some("Idle"),
        ("Receiving", "ClientDisconnected") => Some("Idle"),
        ("Ready", "Start") => Some("Running"),
        ("AwaitingLocalStart", "LocalStart") => Some("Running"),
        ("Running", "Pause") => Some("Paused"),
        ("Paused", "Resume") => Some("Running"),
        ("Running" | "Paused", "Stop") => Some("Done { result: Stopped }"),
        ("Running", "Finished") => Some("Done { result: Completed }"),
        ("Running", "Failed") => Some("Done { result: Failed }"),
        ("Alarm", "AlarmCleared") => Some("Idle"),
        (_, "Lost") if with_controller => Some("NoController"),
        (_, "Alarm") if with_controller && state != "Alarm" => Some("Alarm"),
        _ => None,
    }
}

fn name_of(state: State) -> String {
    format!("{state:?}")
}

#[test]
fn every_state_event_pair_follows_the_table() {
    let status_events = ["GetStatus", "Position", "Progress", "Alarm"];
    for state in STATES {
        for ev in EVENTS {
            let mut rig = reach(state);
            let before = name_of(rig.machine.state());
            assert!(before.starts_with(state), "{state}: {before}");
            let out = rig.send(event(ev));
            let after = name_of(rig.machine.state());
            match expected(state, ev) {
                Some(next) => {
                    assert_eq!(after, next, "{state} + {ev}");
                    assert!(
                        matches!(last_status(&out), Message::Status { .. }),
                        "{state} + {ev}"
                    );
                }
                None => {
                    assert_eq!(after, before, "{state} + {ev} must not change the state");
                    let only_refusals = out.messages.iter().all(is_error);
                    let status_only = status_events.contains(&ev);
                    assert!(
                        only_refusals || status_only,
                        "{state} + {ev}: {:?}",
                        out.messages
                    );
                }
            }
        }
    }
}

#[test]
fn a_job_walks_from_plug_in_to_done_with_the_fake_controller() {
    let mut rig = Rig::new(Options::default());
    assert_eq!(rig.machine.state(), State::NoController);
    let e = rig.controller.plug_in();
    rig.send(e);
    assert_eq!(rig.machine.state(), State::Idle);
    rig.app(submit(JOB_TEXT, JobKind::Job));
    let out = rig.app(chunk(0, JOB_TEXT));
    assert_eq!(rig.machine.state(), State::Ready);
    assert_eq!(out.messages[0], Message::JobAccepted { job_id: job_id() });

    let out = rig.app(Message::Start { job_id: job_id() });
    assert_eq!(
        out.commands,
        [ControllerCommand::Start { job_id: job_id() }]
    );
    assert_eq!(rig.controller.streaming, Some(job_id()));
    rig.app(Message::Pause);
    assert!(rig.controller.paused);
    assert_eq!(rig.machine.state(), State::Paused);
    rig.app(Message::Resume);
    assert!(!rig.controller.paused);
    let e = rig.controller.advance(3);
    rig.send(e);
    let e = rig.controller.finish();
    let out = rig.send(e);
    assert_eq!(rig.machine.state(), done(JobResult::Completed));
    let Message::Status { job: Some(job), .. } = last_status(&out) else {
        panic!()
    };
    assert_eq!((job.line, job.lines, job.elapsed_ms), (6, 6, 1500));

    let e = rig.controller.unplug();
    rig.send(e);
    assert_eq!(rig.machine.state(), State::NoController);
}

#[test]
fn a_failed_job_ends_in_done_failed_and_the_next_job_is_accepted() {
    let mut rig = running();
    let e = rig.controller.fail();
    rig.send(e);
    assert_eq!(rig.machine.state(), done(JobResult::Failed));
    rig.app(submit(JOB_TEXT, JobKind::Job));
    assert_eq!(rig.machine.state(), State::Receiving);
}

#[test]
fn submit_is_busy_unless_idle_or_done_and_the_held_job_is_untouched() {
    for state in [
        "Receiving",
        "Ready",
        "AwaitingLocalStart",
        "Running",
        "Paused",
        "Alarm",
    ] {
        let mut rig = reach(state);
        let held = rig.machine.held_job().cloned().map(|j| j.text);
        let before = name_of(rig.machine.state());
        let out = rig.app(submit(JOB_TEXT, JobKind::Job));
        assert_eq!(
            out.messages,
            [Message::JobRejected {
                job_id: job_id(),
                reason: RejectReason::Busy
            }],
            "{state}"
        );
        assert_eq!(name_of(rig.machine.state()), before);
        assert_eq!(rig.machine.held_job().cloned().map(|j| j.text), held);
    }
}

#[test]
fn submitting_the_same_job_twice_gives_busy() {
    let mut rig = Rig::plugged();
    rig.app(submit(JOB_TEXT, JobKind::Job));
    let out = rig.app(submit(JOB_TEXT, JobKind::Job));
    assert!(matches!(
        out.messages[0],
        Message::JobRejected {
            reason: RejectReason::Busy,
            ..
        }
    ));
}

#[test]
fn submit_without_a_controller_is_rejected() {
    let mut rig = Rig::new(Options::default());
    let out = rig.app(submit(JOB_TEXT, JobKind::Job));
    assert_eq!(
        out.messages,
        [Message::JobRejected {
            job_id: job_id(),
            reason: RejectReason::NoController
        }]
    );
    assert_eq!(rig.machine.state(), State::NoController);
}

#[test]
fn stop_pause_and_resume_are_idempotent() {
    for state in ["NoController", "Idle", "Done", "Alarm"] {
        let mut rig = reach(state);
        let before = name_of(rig.machine.state());
        let out = rig.app(Message::Stop);
        assert!(
            out.messages.is_empty() && out.commands.is_empty(),
            "{state}"
        );
        assert_eq!(name_of(rig.machine.state()), before);
    }
    let mut rig = reach("Paused");
    assert!(rig.app(Message::Pause).messages.is_empty());
    assert_eq!(rig.machine.state(), State::Paused);
    let mut rig = reach("Running");
    assert!(rig.app(Message::Resume).messages.is_empty());
    assert_eq!(rig.machine.state(), State::Running);
}

#[test]
fn stop_in_ready_discards_the_job_and_stop_while_running_ends_once() {
    let mut rig = reach("Ready");
    rig.app(Message::Stop);
    assert_eq!(rig.machine.state(), State::Idle);
    assert!(rig.machine.held_job().is_none());

    for state in ["Running", "Paused"] {
        let mut rig = reach(state);
        let first = rig.app(Message::Stop);
        assert_eq!(first.commands, [ControllerCommand::Stop]);
        assert_eq!(rig.machine.state(), done(JobResult::Stopped));
        let second = rig.app(Message::Stop);
        assert!(second.messages.is_empty() && second.commands.is_empty());
        assert_eq!(rig.machine.state(), done(JobResult::Stopped));
    }
}

#[test]
fn local_start_option_holds_the_start_until_the_button() {
    let mut rig = Rig::ready(REQUIRE);
    let out = rig.app(Message::Start { job_id: job_id() });
    assert_eq!(rig.machine.state(), State::AwaitingLocalStart);
    assert!(out.commands.is_empty());
    assert_eq!(rig.controller.streaming, None);
    rig.send(Event::LocalStartPressed);
    assert_eq!(rig.machine.state(), State::Running);
    assert_eq!(rig.controller.streaming, Some(job_id()));

    let mut rig = Rig::ready(REQUIRE);
    rig.app(Message::Start { job_id: job_id() });
    rig.app(Message::Stop);
    assert_eq!(rig.machine.state(), State::Idle);

    let mut rig = Rig::ready(Options::default());
    rig.app(Message::Start { job_id: job_id() });
    assert_eq!(rig.machine.state(), State::Running);
}

#[test]
fn start_with_another_job_id_gets_an_error() {
    let mut rig = Rig::ready(Options::default());
    let out = rig.send(event("StartOtherJob"));
    assert!(matches!(
        &out.messages[0],
        Message::Error {
            code: ErrorCode::UnknownJob,
            ..
        }
    ));
    assert_eq!(rig.machine.state(), State::Ready);
}

#[test]
fn a_lost_app_connection_does_not_stop_the_job_and_a_new_one_can_stop_it() {
    let mut rig = running();
    let e = rig.controller.advance(2);
    rig.send(e);
    let out = rig.send(Event::ClientDisconnected);
    assert!(out.messages.is_empty() && out.commands.is_empty());
    assert_eq!(rig.machine.state(), State::Running);
    assert!(rig.controller.streaming.is_some());

    let out = rig.app(Message::GetStatus);
    let Message::Status {
        state,
        job: Some(job),
        ..
    } = &out.messages[0]
    else {
        panic!()
    };
    assert_eq!(*state, State::Running);
    assert_eq!((job.job_id, job.line, job.lines), (job_id(), 2, 6));

    rig.app(Message::Stop);
    assert_eq!(rig.machine.state(), done(JobResult::Stopped));
    assert_eq!(rig.controller.streaming, None);
}

#[test]
fn status_carries_progress_only_in_the_five_states() {
    for state in STATES {
        let rig = reach(state);
        let Message::Status { job, state: s, .. } = rig.machine.status() else {
            panic!()
        };
        let carries = matches!(
            s,
            State::Ready
                | State::Running
                | State::Paused
                | State::AwaitingLocalStart
                | State::Done { .. }
        );
        assert_eq!(job.is_some(), carries, "{state}");
    }
}

#[test]
fn position_and_alarm_text_are_part_of_the_status() {
    let mut rig = alarmed();
    rig.send(event("Position"));
    let Message::Status {
        position, alarm, ..
    } = rig.machine.status()
    else {
        panic!()
    };
    assert_eq!(
        position,
        Some([Millimetres(1.0), Millimetres(2.0), Millimetres(0.0)])
    );
    assert_eq!(alarm.as_deref(), Some("ALARM:1 Hard limit"));
    rig.send(Event::AlarmCleared);
    let Message::Status { alarm, .. } = rig.machine.status() else {
        panic!()
    };
    assert_eq!(alarm, None);
}

#[test]
fn messages_the_machine_does_not_take_get_an_error() {
    let mut rig = Rig::plugged();
    let out = rig.send(event("Hello"));
    assert!(matches!(
        &out.messages[0],
        Message::Error {
            code: ErrorCode::UnexpectedMessage,
            ..
        }
    ));
}

#[test]
fn a_zero_byte_job_is_accepted_without_chunks() {
    let mut rig = Rig::new(Options::default());
    let e = rig.controller.plug_in();
    rig.send(e);
    let out = rig.app(submit("", JobKind::Job));
    assert_eq!(out.messages.len(), 3);
    assert!(matches!(
        &out.messages[0],
        Message::Status {
            state: State::Receiving,
            ..
        }
    ));
    assert_eq!(out.messages[1], Message::JobAccepted { job_id: job_id() });
    assert!(matches!(
        &out.messages[2],
        Message::Status {
            state: State::Ready,
            ..
        }
    ));
    assert_eq!(rig.machine.state(), State::Ready);
}

#[test]
fn a_zero_byte_job_with_a_wrong_hash_is_rejected_and_the_machine_is_idle() {
    let mut rig = Rig::new(Options::default());
    let e = rig.controller.plug_in();
    rig.send(e);
    let Message::SubmitJob {
        job_id, name, kind, ..
    } = submit("", JobKind::Job)
    else {
        panic!()
    };
    let out = rig.app(Message::SubmitJob {
        job_id,
        name,
        kind,
        bytes: 0,
        sha256: sha256_hex("x"),
    });
    assert!(out.messages.iter().any(|m| matches!(
        m,
        Message::JobRejected {
            reason: RejectReason::HashMismatch,
            ..
        }
    )));
    assert_eq!(rig.machine.state(), State::Idle);
}

#[test]
fn a_client_lost_mid_transfer_drops_the_half_received_job() {
    let mut rig = reach("Receiving");
    let out = rig.send(Event::ClientDisconnected);
    assert_eq!(rig.machine.state(), State::Idle);
    assert!(matches!(last_status(&out), Message::Status { .. }));
    let out = rig.send(event("Chunk"));
    assert!(out.messages.iter().all(is_error), "{:?}", out.messages);
    let out = rig.send(event("Submit"));
    assert_eq!(rig.machine.state(), State::Receiving, "{:?}", out.messages);
}
