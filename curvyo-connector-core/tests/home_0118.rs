//! `specs/0118` first slice, core: the `Home` command and the `Homing` state (architect addendum
//! M1 to M3, M5), with the three golden files.
#![allow(
    clippy::unwrap_used,
    clippy::unnecessary_wraps,
    clippy::assert_is_empty
)]

mod common;

use common::{JOB_TEXT, chunk, fixture, job_id, submit};
use curvyo_connector_core::{
    ControllerCommand, ErrorCode, Event, JobKind, JobResult, Machine, Message, Options, Output,
    Progress, RejectReason, State, decode, encode,
};
use curvyo_connector_core::{Peer, StartRule};

const REQUIRE: Options = Options {
    start_rule: StartRule::LocalPress,
};

fn app(m: &mut Machine, message: Message) -> Output {
    m.handle(Event::Message(message, Peer::host()))
}

fn idle(m: Machine) -> Machine {
    let mut m = m;
    m.handle(Event::ControllerConnected);
    m
}

fn ready(m: Machine) -> Machine {
    let mut m = idle(m);
    app(&mut m, submit(JOB_TEXT, JobKind::Job));
    app(&mut m, chunk(0, JOB_TEXT));
    assert_eq!(m.state(), State::Ready);
    m
}

fn alarmed(m: Machine) -> Machine {
    let mut m = idle(m);
    m.handle(Event::Alarm {
        text: "Locked, not homed".into(),
    });
    m
}

fn both() -> [Machine; 2] {
    [
        Machine::new(Options::default()),
        Machine::confirming(Options::default()),
    ]
}

fn fields(m: &Machine) -> (Option<Progress>, Option<String>, Option<String>) {
    match m.status() {
        Message::Status {
            job, alarm, detail, ..
        } => (job, alarm, detail),
        _ => unreachable!(),
    }
}

fn wrong_state(out: &Output) -> Option<String> {
    out.messages.iter().find_map(|m| match m {
        Message::Error {
            code: ErrorCode::WrongState,
            message,
        } => Some(message.clone()),
        _ => None,
    })
}

fn ready_progress() -> Option<Progress> {
    Some(Progress {
        job_id: job_id(),
        line: 0,
        lines: 6,
        elapsed_ms: 0,
    })
}

#[test]
fn m1_home_and_homing_match_their_golden_files() {
    let cases = [
        ("command_home", Message::Home),
        (
            "status_homing",
            Message::Status {
                state: State::Homing,
                job: None,
                position: None,
                alarm: None,
                detail: None,
                pause_pending: false,
                requested_by: None,
                job_name: None,
                sent_by: None,
                homed: false,
                machine_position: None,
                work_offset: None,
                frame_id: None,
            },
        ),
        (
            "status_homing_ready",
            Message::Status {
                state: State::Homing,
                job: ready_progress(),
                position: None,
                alarm: None,
                detail: None,
                pause_pending: false,
                requested_by: None,
                job_name: None,
                sent_by: None,
                homed: false,
                machine_position: None,
                work_offset: None,
                frame_id: None,
            },
        ),
    ];
    for (name, message) in cases {
        let golden = fixture(&format!("messages/{name}.json"));
        assert_eq!(encode(&message).unwrap(), golden, "encoding of {name}");
        assert_eq!(decode(&golden).unwrap(), message, "decoding of {name}");
    }
}

#[test]
fn m3_home_from_idle_ready_done_and_alarm_sends_one_home_and_reports_homing_at_once() {
    for m in both() {
        let mut m = idle(m);
        let out = app(&mut m, Message::Home);
        assert_eq!(out.commands, [ControllerCommand::Home]);
        assert_eq!(
            out.messages,
            [m.status()],
            "the Homing status comes before GRBL's ok"
        );
        assert_eq!(m.state(), State::Homing);
        assert_eq!(fields(&m), (None, None, None));
    }
    for m in both() {
        let mut m = ready(m);
        assert_eq!(
            app(&mut m, Message::Home).commands,
            [ControllerCommand::Home]
        );
        assert_eq!(
            fields(&m).0,
            ready_progress(),
            "the kept Ready job is carried"
        );
        assert_eq!(
            m.status(),
            decode(&fixture("messages/status_homing_ready.json")).unwrap()
        );
    }
    for m in both() {
        let mut m = alarmed(m);
        assert_eq!(
            app(&mut m, Message::Home).commands,
            [ControllerCommand::Home]
        );
        assert_eq!(fields(&m).1, None, "no alarm text while homing");
    }
    let mut m = ready(Machine::new(Options::default()));
    app(&mut m, Message::Start { job_id: job_id() });
    m.handle(Event::JobFinished { elapsed_ms: 10 });
    assert!(matches!(m.state(), State::Done { .. }));
    assert_eq!(
        app(&mut m, Message::Home).commands,
        [ControllerCommand::Home]
    );
    assert_eq!(fields(&m).0, None, "a done job is not carried into homing");
    m.handle(Event::Homed);
    assert_eq!(m.state(), State::Idle);
}

#[test]
fn m3_homed_returns_to_ready_with_the_same_job_or_to_idle_and_clears_the_alarm() {
    for m in both() {
        let mut m = ready(m);
        app(&mut m, Message::Home);
        let out = m.handle(Event::Homed);
        assert_eq!(m.state(), State::Ready);
        assert_eq!(out.messages, [m.status()]);
        assert_eq!(fields(&m).0, ready_progress());
        let out = app(&mut m, Message::Start { job_id: job_id() });
        assert_eq!(
            out.commands,
            [ControllerCommand::Start { job_id: job_id() }]
        );
    }
    for m in both() {
        let mut m = alarmed(m);
        app(&mut m, Message::Home);
        m.handle(Event::Homed);
        assert_eq!(m.state(), State::Idle);
        assert_eq!(fields(&m), (None, None, None));
    }
}

#[test]
fn m3_a_refused_homing_returns_where_it_started_with_its_sentence() {
    let text = "Homing is not enabled (error 5)";
    let failed = || Event::HomingFailed {
        detail: text.into(),
    };
    let mut m = idle(Machine::confirming(Options::default()));
    app(&mut m, Message::Home);
    m.handle(failed());
    assert_eq!(m.state(), State::Idle);
    assert_eq!(fields(&m).2.as_deref(), Some(text));
    let mut m = ready(Machine::confirming(Options::default()));
    app(&mut m, Message::Home);
    m.handle(failed());
    assert_eq!(m.state(), State::Ready, "error 5 keeps the Ready job");
    assert_eq!(fields(&m), (ready_progress(), None, Some(text.into())));
    let mut m = alarmed(Machine::confirming(Options::default()));
    app(&mut m, Message::Home);
    m.handle(failed());
    assert_eq!(m.state(), State::Alarm);
    assert_eq!(
        fields(&m).1.as_deref(),
        Some("Locked, not homed"),
        "the alarm text stays"
    );
    // The sentence stays until the next command changes the state.
    let mut m = idle(Machine::confirming(Options::default()));
    app(&mut m, Message::Home);
    m.handle(failed());
    app(&mut m, Message::Home);
    assert_eq!(fields(&m).2, None);
}

#[test]
fn m3_an_alarm_while_homing_is_an_alarm_and_drops_the_ready_job() {
    for m in both() {
        let mut m = ready(m);
        app(&mut m, Message::Home);
        m.handle(Event::Alarm {
            text: "Homing did not find the switch (alarm 9)".into(),
        });
        assert_eq!(m.state(), State::Alarm);
        assert!(m.held_job().is_none());
        assert_eq!(
            fields(&m),
            (
                None,
                Some("Homing did not find the switch (alarm 9)".into()),
                None
            )
        );
    }
    // From the power-up lock the homing failure's text supersedes the lock's.
    let mut m = alarmed(Machine::confirming(Options::default()));
    app(&mut m, Message::Home);
    m.handle(Event::Alarm {
        text: "Homing pull-off did not clear the switch (alarm 8)".into(),
    });
    assert_eq!(
        fields(&m).1.as_deref(),
        Some("Homing pull-off did not clear the switch (alarm 8)")
    );
}

#[test]
fn m3_home_is_refused_while_a_job_runs_or_waits_and_names_the_cause() {
    let mut m = ready(Machine::confirming(Options::default()));
    app(&mut m, Message::Start { job_id: job_id() });
    let out = app(&mut m, Message::Home);
    assert_eq!(wrong_state(&out).as_deref(), Some("A job is running."));
    assert!(out.commands.is_empty());
    m.handle(Event::Held);
    assert_eq!(m.state(), State::Paused);
    assert_eq!(
        wrong_state(&app(&mut m, Message::Home)).as_deref(),
        Some("A job is running.")
    );
    // A stop of the job is still pending: still refused.
    app(&mut m, Message::Stop);
    let out = app(&mut m, Message::Home);
    assert_eq!(wrong_state(&out).as_deref(), Some("A job is running."));
    m.handle(Event::Stopped);
    assert_eq!(
        m.state(),
        State::Done {
            result: JobResult::Stopped
        }
    );
    assert_eq!(
        app(&mut m, Message::Home).commands,
        [ControllerCommand::Home]
    );

    let mut m = Machine::confirming(Options::default());
    assert_eq!(
        wrong_state(&app(&mut m, Message::Home)).as_deref(),
        Some("No controller.")
    );
    let mut m = idle(Machine::confirming(Options::default()));
    app(&mut m, submit(JOB_TEXT, JobKind::Job));
    assert_eq!(m.state(), State::Receiving);
    assert!(wrong_state(&app(&mut m, Message::Home)).is_some());
    let mut m = ready(Machine::confirming(REQUIRE));
    app(&mut m, Message::Start { job_id: job_id() });
    assert_eq!(m.state(), State::AwaitingLocalStart);
    assert!(wrong_state(&app(&mut m, Message::Home)).is_some());
}

#[test]
fn m3_home_is_refused_while_the_controller_reads_door_check_or_sleep() {
    for word in ["Door", "Check", "Sleep"] {
        let mut m = idle(Machine::confirming(Options::default()));
        m.handle(Event::ControllerWord(Some(word.into())));
        let out = app(&mut m, Message::Home);
        assert_eq!(
            wrong_state(&out),
            Some(format!("Controller state: {word}.")),
            "{word}"
        );
        assert!(out.commands.is_empty());
        assert_eq!(m.state(), State::Idle);
    }
}

#[test]
fn m3_while_homing_jobs_and_job_controls_are_refused_and_home_unlock_write_nothing() {
    let mut m = idle(Machine::confirming(Options::default()));
    app(&mut m, Message::Home);
    let out = app(&mut m, submit(JOB_TEXT, JobKind::Job));
    assert_eq!(
        out.messages,
        [Message::JobRejected {
            job_id: job_id(),
            reason: RejectReason::Busy
        }]
    );
    for message in [
        Message::Start { job_id: job_id() },
        Message::Pause,
        Message::Resume,
    ] {
        let out = app(&mut m, message.clone());
        assert!(wrong_state(&out).is_some(), "{message:?}");
        assert!(out.commands.is_empty());
    }
    assert_eq!(app(&mut m, Message::Home), Output::default());
    assert_eq!(app(&mut m, Message::Unlock), Output::default());
    assert_eq!(m.handle(Event::ClientDisconnected), Output::default());
    // The controller's own `Home` word is not shown while the state already says it.
    m.handle(Event::ControllerWord(Some("Home".into())));
    assert_eq!(fields(&m).2, None);
    assert_eq!(m.state(), State::Homing);
}

#[test]
fn m3_losing_the_controller_while_homing_reads_lost_the_controller() {
    let mut m = ready(Machine::confirming(Options::default()));
    app(&mut m, Message::Home);
    m.handle(Event::ControllerLost);
    assert_eq!(m.state(), State::NoController);
    assert_eq!(fields(&m).2.as_deref(), Some("Lost the controller"));
    assert!(m.held_job().is_none());
}

#[test]
fn m3_stop_while_homing_without_confirmation_returns_at_once_and_keeps_a_ready_job() {
    let mut m = ready(Machine::new(Options::default()));
    app(&mut m, Message::Home);
    let out = app(&mut m, Message::Stop);
    assert_eq!(out.commands, [ControllerCommand::Stop]);
    assert_eq!(m.state(), State::Ready);
    assert_eq!(fields(&m).0, ready_progress());
    let mut m = idle(Machine::new(Options::default()));
    app(&mut m, Message::Home);
    app(&mut m, Message::Stop);
    assert_eq!(m.state(), State::Idle);
}

#[test]
fn m3_stop_while_homing_with_confirmation_waits_then_alarm_6_drops_the_job_and_never_done() {
    let mut m = ready(Machine::confirming(Options::default()));
    app(&mut m, Message::Home);
    let out = app(&mut m, Message::Stop);
    assert_eq!(out.commands, [ControllerCommand::Stop]);
    assert_eq!(m.state(), State::Homing);
    assert_eq!(fields(&m).2.as_deref(), Some("Stopping"));
    assert_eq!(
        app(&mut m, Message::Stop),
        Output::default(),
        "a pending stop is not repeated"
    );
    m.handle(Event::Stopped);
    assert_eq!(m.state(), State::Ready);
    assert_eq!(fields(&m).2, None);
    m.handle(Event::Alarm {
        text: "Reset during homing (alarm 6)".into(),
    });
    assert_eq!(m.state(), State::Alarm);
    assert!(m.held_job().is_none());
}

#[test]
fn m5_with_local_start_home_waits_for_the_press_without_a_job_and_stop_cancels_it() {
    let mut m = ready(Machine::confirming(REQUIRE));
    let out = app(&mut m, Message::Home);
    assert!(out.commands.is_empty());
    assert_eq!(m.state(), State::AwaitingLocalStart);
    assert_eq!(fields(&m).0, None, "a waiting Home names no job");
    let out = m.handle(Event::LocalStartPressed);
    assert_eq!(out.commands, [ControllerCommand::Home]);
    assert_eq!(m.state(), State::Homing);
    m.handle(Event::Homed);
    assert_eq!(m.state(), State::Ready);

    let mut m = ready(Machine::confirming(REQUIRE));
    app(&mut m, Message::Home);
    let out = app(&mut m, Message::Stop);
    assert!(out.commands.is_empty());
    assert_eq!(
        m.state(),
        State::Ready,
        "Stop cancels the waiting Home, the job is kept"
    );
    assert_eq!(fields(&m).0, ready_progress());
    // The job's own start still waits and starts as before.
    app(&mut m, Message::Start { job_id: job_id() });
    let out = m.handle(Event::LocalStartPressed);
    assert_eq!(
        out.commands,
        [ControllerCommand::Start { job_id: job_id() }]
    );

    let mut m = idle(Machine::confirming(REQUIRE));
    app(&mut m, Message::Home);
    app(&mut m, Message::Stop);
    assert_eq!(m.state(), State::Idle);
}
