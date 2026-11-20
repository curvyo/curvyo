//! `specs/0127` core additions: the wire fields with their golden files and the job machine that
//! waits for the controller's confirmation (G7, G10 to G12, G14, G15, G19, G29, G30).
#![allow(
    clippy::unwrap_used,
    clippy::unnecessary_wraps,
    clippy::assert_is_empty
)]

mod common;

use common::{JOB_TEXT, chunk, fixture, job_id, submit};
use curvyo_connector_core::Peer;
use curvyo_connector_core::{
    ControllerCommand, ControllerInfo, ControllerSettings, Event, JobKind, JobResult, Machine,
    Message, Millimetres, Options, Output, Progress, RejectReason, State, decode, encode,
};

fn detail_of(message: &Message) -> Option<String> {
    match message {
        Message::Status { detail, .. } => detail.clone(),
        _ => None,
    }
}

fn detail(m: &Machine) -> Option<String> {
    detail_of(&m.status())
}

fn progress(line: u32, lines: u32, elapsed_ms: u64) -> Option<Progress> {
    Some(Progress {
        job_id: job_id(),
        line,
        lines,
        elapsed_ms,
    })
}

#[test]
fn g19_g21_the_new_fields_match_their_golden_files_and_old_files_still_decode() {
    let cases = [
        (
            "status_failed",
            Message::Status {
                state: State::Done {
                    result: JobResult::Failed,
                },
                job: progress(11, 412, 4200),
                position: Some([Millimetres(12.5), Millimetres(0.0), Millimetres(0.0)]),
                alarm: None,
                detail: Some("Failed at line 12: No feed rate (error 22)".into()),
                pause_pending: false,
                requested_by: None,
                job_name: None,
                sent_by: None,
            },
        ),
        (
            "status_lost",
            Message::Status {
                state: State::NoController,
                job: None,
                position: None,
                alarm: None,
                detail: Some("Lost the controller at line 120".into()),
                pause_pending: false,
                requested_by: None,
                job_name: None,
                sent_by: None,
            },
        ),
        (
            "status_controller_state",
            Message::Status {
                state: State::Idle,
                job: None,
                position: Some([Millimetres(120.0), Millimetres(45.25), Millimetres(0.0)]),
                alarm: None,
                detail: Some("Controller state: Door".into()),
                pause_pending: false,
                requested_by: None,
                job_name: None,
                sent_by: None,
            },
        ),
        (
            "welcome_grbl",
            Message::Welcome {
                connector_name: "Workshop Pi".into(),
                controller: Some(ControllerInfo {
                    name: "GRBL".into(),
                    firmware: "1.1h".into(),
                    port: Some("/dev/ttyUSB0".into()),
                    baud: Some(115_200),
                    settings: Some(ControllerSettings {
                        laser_mode: Some(true),
                        s_max: Some(1000),
                        travel: Some([Millimetres(400.0), Millimetres(300.0)]),
                    }),
                }),
                mode: None,
            },
        ),
    ];
    for (name, message) in cases {
        let golden = fixture(&format!("messages/{name}.json"));
        assert_eq!(encode(&message).unwrap(), golden, "encoding of {name}");
        assert_eq!(decode(&golden).unwrap(), message, "decoding of {name}");
    }
    // The old files carry none of the new fields and decode with them absent.
    match decode(&fixture("messages/welcome.json")).unwrap() {
        Message::Welcome {
            controller: Some(info),
            ..
        } => assert_eq!((info.port, info.baud, info.settings), (None, None, None)),
        other => panic!("{other:?}"),
    }
    assert_eq!(
        detail_of(&decode(&fixture("messages/status_running.json")).unwrap()),
        None
    );
}

#[test]
fn g19_settings_write_null_for_a_value_the_controller_did_not_report() {
    let settings = ControllerSettings {
        laser_mode: None,
        s_max: Some(255),
        travel: None,
    };
    assert_eq!(
        serde_json::to_string(&settings).unwrap(),
        r#"{"laser_mode":null,"s_max":255,"travel":null}"#
    );
}

/// A confirming machine with a controller, the job of `JOB_TEXT` held.
fn ready() -> Machine {
    let mut m = Machine::confirming(Options::default());
    m.handle(Event::ControllerConnected);
    m.handle(Event::Message(submit(JOB_TEXT, JobKind::Job), Peer::host()));
    m.handle(Event::Message(chunk(0, JOB_TEXT), Peer::host()));
    assert_eq!(m.state(), State::Ready);
    m
}

fn running() -> Machine {
    let mut m = ready();
    let out = m.handle(Event::Message(
        Message::Start { job_id: job_id() },
        Peer::host(),
    ));
    assert_eq!(
        out.commands,
        [ControllerCommand::Start { job_id: job_id() }]
    );
    assert_eq!(m.state(), State::Running);
    m
}

fn app(m: &mut Machine, message: Message) -> Output {
    m.handle(Event::Message(message, Peer::host()))
}

#[test]
fn g10_pause_waits_for_the_hold_and_g11_resume_waits_for_the_run() {
    let mut m = running();
    let out = app(&mut m, Message::Pause);
    assert_eq!(out.commands, [ControllerCommand::Pause]);
    assert_eq!(m.state(), State::Running, "Running until Hold:0");
    assert!(out.messages.is_empty());
    m.handle(Event::Held);
    assert_eq!(m.state(), State::Paused);
    // Pause while Paused stays idempotent.
    assert_eq!(app(&mut m, Message::Pause), Output::default());
    let out = app(&mut m, Message::Resume);
    assert_eq!(out.commands, [ControllerCommand::Resume]);
    assert_eq!(m.state(), State::Paused, "Paused until GRBL runs");
    m.handle(Event::Resumed);
    assert_eq!(m.state(), State::Running);
}

#[test]
fn g10_a_hold_nobody_asked_for_reads_paused() {
    let mut m = running();
    m.handle(Event::Held);
    assert_eq!(m.state(), State::Paused);
}

#[test]
fn g12_g30_stop_answers_stopping_at_once_and_ends_once_when_confirmed() {
    for paused in [false, true] {
        let mut m = running();
        if paused {
            app(&mut m, Message::Pause);
            m.handle(Event::Held);
        }
        let before = m.state();
        let out = app(&mut m, Message::Stop);
        assert_eq!(out.commands, [ControllerCommand::Stop]);
        assert_eq!(out.messages.len(), 1);
        assert_eq!(detail_of(&out.messages[0]).as_deref(), Some("Stopping"));
        assert_eq!(m.state(), before, "state unchanged until the stop ends");
        // While the stop is pending nothing else is taken.
        for message in [Message::Pause, Message::Resume, Message::Stop] {
            assert!(app(&mut m, message).commands.is_empty());
        }
        m.handle(Event::Held);
        m.handle(Event::Resumed);
        assert_eq!(m.state(), before);
        let out = m.handle(Event::Stopped);
        let done = State::Done {
            result: JobResult::Stopped,
        };
        assert_eq!(m.state(), done);
        assert_eq!(out.messages.len(), 1, "Done {{ Stopped }} exactly once");
        assert_eq!(detail(&m), None);
        assert!(m.handle(Event::Stopped).messages.is_empty());
    }
}

#[test]
fn g12_an_unconfirmed_stop_is_no_clean_stop_and_names_itself() {
    let mut m = running();
    app(&mut m, Message::Stop);
    m.handle(Event::StopUnconfirmed);
    assert_eq!(m.state(), State::NoController);
    assert_eq!(
        detail(&m).as_deref(),
        Some("Could not confirm the machine stopped")
    );
    assert!(m.held_job().is_none());
}

#[test]
fn g7_g29_a_failure_with_its_sentence_from_running_or_paused() {
    for paused in [false, true] {
        let mut m = running();
        m.handle(Event::Progress {
            line: 3,
            elapsed_ms: 900,
        });
        if paused {
            m.handle(Event::Held);
        }
        m.handle(Event::JobFailedWith {
            elapsed_ms: 1000,
            detail: "Failed at line 4: No feed rate (error 22)".into(),
        });
        assert_eq!(
            m.state(),
            State::Done {
                result: JobResult::Failed
            }
        );
        let status = m.status();
        assert_eq!(
            detail_of(&status).as_deref(),
            Some("Failed at line 4: No feed rate (error 22)")
        );
        let Message::Status { job, .. } = status else {
            panic!()
        };
        assert_eq!(job.unwrap().line, 3, "the bar holds the acknowledged count");
        // The next send clears the sentence.
        app(&mut m, submit(JOB_TEXT, JobKind::Job));
        assert_eq!(detail(&m), None);
    }
}

#[test]
fn g15_a_loss_names_the_line_and_stays_through_the_reconnect_until_the_next_send() {
    let mut m = running();
    m.handle(Event::Progress {
        line: 12_345,
        elapsed_ms: 1,
    });
    m.handle(Event::ControllerLost);
    assert_eq!(m.state(), State::NoController);
    let lost = Some("Lost the controller at line 6".to_owned());
    // JOB_TEXT has 6 lines: the count is capped at the job's lines.
    assert_eq!(detail(&m), lost);
    // The retry's open failure does not replace it.
    m.handle(Event::NoControllerDetail {
        detail: "Port /dev/ttyUSB0 not found".into(),
    });
    assert_eq!(detail(&m), lost);
    m.handle(Event::ControllerConnected);
    assert_eq!(m.state(), State::Idle);
    assert_eq!(
        detail(&m),
        lost,
        "a client after the reconnect reads it too"
    );
    assert!(m.held_job().is_none(), "never resumed");
    app(&mut m, submit(JOB_TEXT, JobKind::Job));
    assert_eq!(detail(&m), None);
}

#[test]
fn hc3_a_loss_with_its_own_sentence_drops_the_job_and_stays_through_the_reconnect() {
    let text = "Lost the controller: homing took longer than 120 s";
    for mut m in [running(), ready()] {
        m.handle(Event::PauseDeferred);
        m.handle(Event::ControllerLostWith {
            detail: text.into(),
        });
        assert_eq!(m.state(), State::NoController);
        assert_eq!(detail(&m).as_deref(), Some(text));
        assert!(m.held_job().is_none(), "dropped");
        assert!(!matches!(
            m.status(),
            Message::Status {
                pause_pending: true,
                ..
            }
        ));
        m.handle(Event::NoControllerDetail {
            detail: "Port /dev/ttyUSB0 not found".into(),
        });
        m.handle(Event::ControllerLostWith {
            detail: "other".into(),
        });
        m.handle(Event::ControllerConnected);
        assert_eq!(m.state(), State::Idle);
        assert_eq!(detail(&m).as_deref(), Some(text));
    }
}

#[test]
fn g15_a_job_not_yet_started_reads_lost_the_controller() {
    let mut m = ready();
    m.handle(Event::ControllerLost);
    assert_eq!(detail(&m).as_deref(), Some("Lost the controller"));
}

#[test]
fn g15_the_line_count_has_a_thousands_comma() {
    let text = "G1 X1\n".repeat(13_000);
    let mut m = Machine::confirming(Options::default());
    m.handle(Event::ControllerConnected);
    app(&mut m, submit(&text, JobKind::Job));
    for (i, part) in text.as_bytes().chunks(60_000).enumerate() {
        app(
            &mut m,
            chunk(
                u32::try_from(i).unwrap(),
                std::str::from_utf8(part).unwrap(),
            ),
        );
    }
    app(&mut m, Message::Start { job_id: job_id() });
    m.handle(Event::Progress {
        line: 12_345,
        elapsed_ms: 1,
    });
    m.handle(Event::ControllerLost);
    assert_eq!(
        detail(&m).as_deref(),
        Some("Lost the controller at line 12,345")
    );
}

#[test]
fn g2_g3_why_there_is_no_controller_is_pushed_once_per_change() {
    let mut m = Machine::confirming(Options::default());
    let text = "No GRBL answer on /dev/ttyUSB0".to_owned();
    let out = m.handle(Event::NoControllerDetail {
        detail: text.clone(),
    });
    assert_eq!(out.messages.len(), 1);
    assert!(
        m.handle(Event::NoControllerDetail {
            detail: text.clone()
        })
        .messages
        .is_empty(),
        "a retry with the same cause pushes nothing"
    );
    assert_eq!(detail(&m), Some(text));
    m.handle(Event::ControllerConnected);
    assert_eq!(
        detail(&m),
        None,
        "an open failure does not outlive the connect"
    );
    // An idle loss names nothing.
    m.handle(Event::ControllerLost);
    assert_eq!(detail(&m), None);
}

#[test]
fn g13_the_first_alarm_text_stands_until_it_clears() {
    let mut m = running();
    m.handle(Event::Alarm {
        text: "Hard limit hit, position may be lost (alarm 1)".into(),
    });
    let out = m.handle(Event::Alarm {
        text: "Locked, not homed".into(),
    });
    assert!(out.messages.is_empty());
    let Message::Status { alarm, .. } = m.status() else {
        panic!()
    };
    assert_eq!(
        alarm.as_deref(),
        Some("Hard limit hit, position may be lost (alarm 1)")
    );
    m.handle(Event::AlarmCleared);
    m.handle(Event::Alarm {
        text: "Locked".into(),
    });
    let Message::Status { alarm, .. } = m.status() else {
        panic!()
    };
    assert_eq!(alarm.as_deref(), Some("Locked"));
}

#[test]
fn g33_unlock_reaches_the_controller_only_in_alarm() {
    let mut m = Machine::confirming(Options::default());
    m.handle(Event::ControllerConnected);
    let out = app(&mut m, Message::Unlock);
    assert_eq!(out, Output::default(), "outside Alarm nothing is written");
    m.handle(Event::Alarm {
        text: "Locked, not homed".into(),
    });
    let out = app(&mut m, Message::Unlock);
    assert_eq!(out.commands, [ControllerCommand::Unlock]);
    assert_eq!(m.state(), State::Alarm, "Idle only when GRBL reports Idle");
    assert_eq!(
        encode(&Message::Unlock).unwrap(),
        fixture("messages/command_unlock.json")
    );
    assert_eq!(
        decode(&fixture("messages/command_unlock.json")).unwrap(),
        Message::Unlock
    );
}

#[test]
fn g34_laser_mode_off_refuses_every_job_and_changes_nothing() {
    let mut m = Machine::confirming(Options::default());
    m.handle(Event::LaserModeOff(true));
    m.handle(Event::ControllerConnected);
    let out = app(&mut m, submit(JOB_TEXT, JobKind::Job));
    let rejected = Message::JobRejected {
        job_id: job_id(),
        reason: RejectReason::LaserModeOff,
    };
    assert_eq!(out.messages, std::slice::from_ref(&rejected));
    assert_eq!(m.state(), State::Idle);
    assert!(m.held_job().is_none());
    assert_eq!(
        encode(&rejected).unwrap(),
        fixture("messages/reject_laser_mode_off.json")
    );
    // A new identify with laser mode on takes jobs again; a loss forgets it.
    m.handle(Event::LaserModeOff(false));
    app(&mut m, submit(JOB_TEXT, JobKind::Job));
    assert_eq!(m.state(), State::Receiving);
    let mut n = Machine::confirming(Options::default());
    n.handle(Event::ControllerConnected);
    n.handle(Event::LaserModeOff(true));
    n.handle(Event::ControllerLost);
    n.handle(Event::ControllerConnected);
    app(&mut n, submit(JOB_TEXT, JobKind::Job));
    assert_eq!(n.state(), State::Receiving);
}

#[test]
fn g34_laser_mode_unknown_refuses_every_job_until_an_identify_reads_it() {
    let mut m = Machine::confirming(Options::default());
    m.handle(Event::LaserModeUnknown(true));
    m.handle(Event::ControllerConnected);
    let out = app(&mut m, submit(JOB_TEXT, JobKind::Job));
    let rejected = Message::JobRejected {
        job_id: job_id(),
        reason: RejectReason::LaserModeUnknown,
    };
    assert_eq!(out.messages, std::slice::from_ref(&rejected));
    assert_eq!(m.state(), State::Idle);
    assert!(m.held_job().is_none());
    assert_eq!(
        encode(&rejected).unwrap(),
        fixture("messages/reject_laser_mode_unknown.json")
    );
    assert_eq!(
        decode(&fixture("messages/reject_laser_mode_unknown.json")).unwrap(),
        rejected
    );
    m.handle(Event::LaserModeUnknown(false));
    app(&mut m, submit(JOB_TEXT, JobKind::Job));
    assert_eq!(m.state(), State::Receiving);
    // A loss forgets it; a machine that never hears of `$32` (the fake controller) accepts.
    let mut n = Machine::confirming(Options::default());
    n.handle(Event::ControllerConnected);
    n.handle(Event::LaserModeUnknown(true));
    n.handle(Event::ControllerLost);
    n.handle(Event::ControllerConnected);
    app(&mut n, submit(JOB_TEXT, JobKind::Job));
    assert_eq!(n.state(), State::Receiving);
}

#[test]
fn a_controller_in_door_check_or_sleep_is_busy_for_a_new_job() {
    for word in ["Door", "Check", "Sleep"] {
        let mut m = Machine::confirming(Options::default());
        m.handle(Event::ControllerConnected);
        m.handle(Event::ControllerWord(Some(word.into())));
        let out = app(&mut m, submit(JOB_TEXT, JobKind::Job));
        assert_eq!(
            out.messages,
            [Message::JobRejected {
                job_id: job_id(),
                reason: RejectReason::Busy,
            }],
            "{word}"
        );
        assert_eq!(m.state(), State::Idle);
        m.handle(Event::ControllerWord(None));
        app(&mut m, submit(JOB_TEXT, JobKind::Job));
        assert_eq!(m.state(), State::Receiving, "{word}");
    }
}

#[test]
fn g4_a_foreign_controller_state_is_a_detail_and_never_changes_the_state() {
    let mut m = Machine::confirming(Options::default());
    m.handle(Event::ControllerConnected);
    let out = m.handle(Event::ControllerWord(Some("Door".into())));
    assert_eq!(out.messages.len(), 1);
    assert_eq!(m.state(), State::Idle);
    assert_eq!(detail(&m).as_deref(), Some("Controller state: Door"));
    assert!(
        m.handle(Event::ControllerWord(Some("Door".into())))
            .messages
            .is_empty()
    );
    m.handle(Event::ControllerWord(None));
    assert_eq!(detail(&m), None);
}

#[test]
fn g20_the_immediate_machine_keeps_the_slice_two_behaviour() {
    let mut m = Machine::new(Options::default());
    m.handle(Event::ControllerConnected);
    app(&mut m, submit(JOB_TEXT, JobKind::Job));
    app(&mut m, chunk(0, JOB_TEXT));
    app(&mut m, Message::Start { job_id: job_id() });
    app(&mut m, Message::Pause);
    assert_eq!(m.state(), State::Paused);
    app(&mut m, Message::Stop);
    assert_eq!(
        m.state(),
        State::Done {
            result: JobResult::Stopped
        }
    );
    assert_eq!(detail(&m), None);
}
