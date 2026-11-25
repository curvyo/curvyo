//! `specs/0118` slice 2d, criteria 10, 12, 34, 35 c (adrs P2 to P4, P6, P12): the go to point and
//! its pulse in the job machine, per state in both machines, by an injected clock; the wire
//! goldens and the decode refusals of the connector's caps. No wall time anywhere.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]

mod common;

use std::time::Duration;

use common::{JOB_TEXT, chunk, fixture, job_id, submit};
use curvyo_connector_core::{
    CodecError, ControllerCommand, ErrorCode, Event, JobKind, JobResult, JogStep, Machine, Message,
    Millimetres, Options, Output, Peer, Progress, Pulse, PulseLength, PulsePower, RejectReason,
    Role, StartRule, State, TravelBox, decode, encode,
};

const LOCAL: Options = Options {
    start_rule: StartRule::LocalPress,
};

fn owner() -> Peer {
    Peer::new("Laptop", Role::Owner)
}

fn pulse() -> Pulse {
    Pulse {
        power_permille: PulsePower::new(10).unwrap(),
        length_ms: PulseLength::new(300).unwrap(),
    }
}

fn go(x: f64, y: f64, pulse: Option<Pulse>) -> Message {
    Message::GoToPoint {
        x: Millimetres(x),
        y: Millimetres(y),
        pulse,
    }
}

fn machine(confirming: bool, options: Options) -> Machine {
    let mut m = if confirming {
        Machine::confirming(options)
    } else {
        Machine::new(options)
    };
    m.handle(Event::ControllerConnected);
    m
}

fn homed(confirming: bool) -> Machine {
    let mut m = machine(confirming, Options::default());
    m.handle(Event::Homed);
    m
}

fn say(m: &mut Machine, message: Message) -> Output {
    m.handle(Event::Message(message, owner()))
}

fn ready(m: &mut Machine) {
    say(m, submit(JOB_TEXT, JobKind::Job));
    say(m, chunk(0, JOB_TEXT));
    assert_eq!(m.state(), State::Ready);
}

fn error_of(out: &Output) -> Option<(ErrorCode, String)> {
    out.messages.iter().find_map(|m| match m {
        Message::Error { code, message } => Some((*code, message.clone())),
        _ => None,
    })
}

fn detail_of(m: &Machine) -> Option<String> {
    match m.status() {
        Message::Status { detail, .. } => detail,
        _ => None,
    }
}

fn target_of(m: &Machine) -> Option<[f64; 2]> {
    match m.status() {
        Message::Status { target, .. } => target.map(|t| t.map(|v| v.0)),
        _ => None,
    }
}

fn at(x: f64, y: f64) -> Event {
    Event::Position([Millimetres(x), Millimetres(y), Millimetres(0.0)])
}

/// A homed machine in `Focusing` towards (10, 20) with the pulse.
fn focusing(confirming: bool) -> Machine {
    let mut m = homed(confirming);
    let out = say(&mut m, go(10.0, 20.0, Some(pulse())));
    assert_eq!(m.state(), State::Focusing);
    assert_eq!(
        out.commands,
        vec![ControllerCommand::GoTo {
            target: [Millimetres(10.0), Millimetres(20.0)],
            feed: JogStep::Ten.feed(),
        }]
    );
    m
}

/// `focusing`, arrived, the pulse started.
fn pulsing(confirming: bool) -> Machine {
    let mut m = focusing(confirming);
    m.handle(at(10.0, 20.0));
    let out = m.handle(Event::JogEnded);
    assert_eq!(m.state(), State::Pulsing);
    assert_eq!(
        out.commands,
        vec![ControllerCommand::Pulse {
            power: PulsePower::new(10).unwrap(),
            length: PulseLength::new(300).unwrap(),
        }]
    );
    m
}

fn status(state: State, target: Option<[f64; 2]>, requested_by: Option<&str>) -> Message {
    Message::Status {
        state,
        job: None,
        position: None,
        alarm: None,
        detail: None,
        pause_pending: false,
        requested_by: requested_by.map(str::to_owned),
        job_name: None,
        sent_by: None,
        homed: true,
        machine_position: None,
        work_offset: None,
        frame_id: None,
        target: target.map(|t| t.map(Millimetres)),
    }
}

#[test]
fn the_goldens_fix_the_wire_and_round_trip() {
    let table = [
        ("command_go_to_point", go(12.5, -40.0, None)),
        ("command_go_to_point_pulse", go(12.5, -40.0, Some(pulse()))),
        (
            "status_focusing",
            status(State::Focusing, Some([12.5, -40.0]), None),
        ),
        (
            "status_pulsing",
            status(State::Pulsing, Some([12.5, -40.0]), None),
        ),
        (
            "status_awaiting_point",
            status(State::AwaitingLocalStart, Some([12.5, -40.0]), None),
        ),
    ];
    for (name, message) in table {
        let golden = fixture(&format!("messages/{name}.json"));
        assert_eq!(encode(&message).unwrap(), golden, "encoding of {name}");
        assert_eq!(decode(&golden).unwrap(), message, "decoding of {name}");
    }
}

#[test]
fn the_connector_refuses_a_pulse_or_a_target_outside_its_caps() {
    let base = r#"{"type":"GoToPoint","x":1.0,"y":2.0"#;
    let refused = [
        r#","pulse":{"power_permille":0,"length_ms":300}}"#,
        r#","pulse":{"power_permille":51,"length_ms":300}}"#,
        r#","pulse":{"power_permille":10,"length_ms":49}}"#,
        r#","pulse":{"power_permille":10,"length_ms":2001}}"#,
        r#","pulse":{"power_permille":-1,"length_ms":300}}"#,
        r#","pulse":{"power_permille":10}}"#,
    ];
    for tail in refused {
        let frame = format!("{base}{tail}");
        assert!(
            matches!(decode(&frame), Err(CodecError::Malformed(_))),
            "{frame}"
        );
    }
    for frame in [
        r#"{"type":"GoToPoint","x":10000.001,"y":0}"#,
        r#"{"type":"GoToPoint","x":0,"y":-10001}"#,
        r#"{"type":"GoToPoint","x":0}"#,
    ] {
        assert!(
            matches!(decode(frame), Err(CodecError::Malformed(_))),
            "{frame}"
        );
    }
    let edge = r#"{"type":"GoToPoint","x":-10000,"y":10000,"pulse":{"power_permille":50,"length_ms":2000}}"#;
    assert!(decode(edge).is_ok());
    assert!(decode(&format!("{base}}}")).is_ok());
}

#[test]
fn a_go_to_point_needs_a_homed_controller_in_idle_ready_or_done() {
    for confirming in [false, true] {
        let mut m = machine(confirming, Options::default());
        let out = say(&mut m, go(1.0, 1.0, None));
        assert_eq!(
            error_of(&out),
            Some((ErrorCode::WrongState, "Not homed.".to_owned()))
        );
        assert!(out.commands.is_empty(), "nothing is written");
        m.handle(Event::Homed);
        m.handle(Event::ControllerWord(Some("Door".into())));
        let out = say(&mut m, go(1.0, 1.0, None));
        assert_eq!(
            error_of(&out),
            Some((ErrorCode::WrongState, "Controller state: Door.".to_owned()))
        );
        m.handle(Event::ControllerWord(None));
        say(&mut m, go(1.0, 1.0, None));
        assert_eq!(m.state(), State::Focusing, "confirming {confirming}");
    }
}

#[test]
fn every_other_state_refuses_with_its_cause_and_writes_nothing() {
    let mut alarm = homed(false);
    alarm.handle(Event::Alarm { text: "x".into() });
    let mut homing = homed(false);
    say(&mut homing, Message::Home);
    let mut running = homed(false);
    ready(&mut running);
    say(&mut running, Message::Start { job_id: job_id() });
    let mut receiving = homed(false);
    say(&mut receiving, submit(JOB_TEXT, JobKind::Job));
    let mut no_controller = Machine::new(Options::default());
    let mut focus = focusing(false);
    let mut pulse_m = pulsing(false);
    let table: [(&mut Machine, &str); 7] = [
        (&mut alarm, "Alarm."),
        (&mut homing, "Homing."),
        (&mut running, "A job is running."),
        (&mut receiving, "A job is arriving."),
        (&mut no_controller, "No controller."),
        (&mut focus, "Focus point."),
        (&mut pulse_m, "Pulse."),
    ];
    for (m, cause) in table {
        let before = m.state();
        let out = say(m, go(1.0, 1.0, Some(pulse())));
        assert_eq!(
            error_of(&out),
            Some((ErrorCode::WrongState, cause.to_owned()))
        );
        assert!(out.commands.is_empty(), "{cause}");
        assert_eq!(m.state(), before);
    }
}

#[test]
fn a_ready_job_is_kept_through_the_move_and_the_pulse() {
    for confirming in [false, true] {
        let mut m = homed(confirming);
        ready(&mut m);
        say(&mut m, go(10.0, 20.0, Some(pulse())));
        assert_eq!(m.state(), State::Focusing);
        let Message::Status { job, .. } = m.status() else {
            panic!()
        };
        assert_eq!(job.map(|p: Progress| p.job_id), Some(job_id()));
        m.handle(at(10.0, 20.0));
        m.handle(Event::JogEnded);
        assert_eq!(m.state(), State::Pulsing);
        m.handle(Event::PulseEnded);
        assert_eq!(m.state(), State::Ready);
        assert!(m.held_job().is_some());
        assert_eq!(target_of(&m), None, "the target leaves with the state");
    }
}

#[test]
fn from_done_the_go_to_point_returns_to_done() {
    let mut m = homed(false);
    ready(&mut m);
    say(&mut m, Message::Start { job_id: job_id() });
    m.handle(Event::JobFinished { elapsed_ms: 10 });
    let done = State::Done {
        result: JobResult::Completed,
    };
    assert_eq!(m.state(), done);
    say(&mut m, go(1.0, 1.0, None));
    m.handle(Event::JogEnded);
    assert_eq!(m.state(), done);
}

#[test]
fn the_status_carries_the_clamped_target() {
    let mut m = focusing(false);
    assert_eq!(target_of(&m), Some([10.0, 20.0]));
    let out = m.handle(Event::GoToTarget {
        target: [Millimetres(10.0), Millimetres(0.0)],
    });
    assert!(!out.messages.is_empty(), "the status goes out");
    assert_eq!(target_of(&m), Some([10.0, 0.0]));
    // The arrival is judged against the clamped target.
    m.handle(at(10.0, 0.0));
    m.handle(Event::JogEnded);
    assert_eq!(m.state(), State::Pulsing);
    assert_eq!(target_of(&m), Some([10.0, 0.0]));
}

#[test]
fn the_pulse_fires_within_five_hundredths_and_not_beyond() {
    let mut near = focusing(false);
    near.handle(at(10.049, 19.951));
    near.handle(Event::JogEnded);
    assert_eq!(near.state(), State::Pulsing);

    let mut far = focusing(false);
    far.handle(at(10.051, 20.0));
    let out = far.handle(Event::JogEnded);
    assert!(out.commands.is_empty(), "nothing is written");
    assert_eq!(far.state(), State::Idle);
    assert_eq!(detail_of(&far).as_deref(), Some("Did not reach the point"));

    let mut unknown = focusing(false);
    unknown.handle(Event::JogEnded);
    assert_eq!(unknown.state(), State::Idle, "no position, no pulse");
}

#[test]
fn without_a_pulse_the_head_arrives_and_nothing_else_happens() {
    let mut m = homed(false);
    say(&mut m, go(10.0, 20.0, None));
    m.handle(at(3.0, 3.0));
    let out = m.handle(Event::JogEnded);
    assert!(out.commands.is_empty(), "nothing is written");
    assert_eq!(m.state(), State::Idle);
    assert_eq!(detail_of(&m), None);
}

#[test]
fn no_pulse_with_laser_mode_off_or_unknown_or_a_controller_word() {
    let cases: [(&str, Event); 3] = [
        ("off", Event::LaserModeOff(true)),
        ("unknown", Event::LaserModeUnknown(true)),
        ("door", Event::ControllerWord(Some("Door".into()))),
    ];
    for (name, event) in cases {
        let mut m = focusing(false);
        m.handle(at(10.0, 20.0));
        m.handle(event);
        let out = m.handle(Event::JogEnded);
        assert!(
            !out.commands
                .iter()
                .any(|c| matches!(c, ControllerCommand::Pulse { .. })),
            "{name}"
        );
        assert_eq!(m.state(), State::Idle, "{name}");
        assert_ne!(
            detail_of(&m).as_deref(),
            Some("Did not reach the point"),
            "{name}: no detail of its own"
        );
    }
}

#[test]
fn stop_during_the_move_is_the_jog_cancel_and_no_pulse_follows() {
    let mut m = focusing(false);
    let out = say(&mut m, Message::Stop);
    assert_eq!(out.commands, vec![ControllerCommand::JogCancel]);
    assert_eq!(m.state(), State::Idle);

    let mut c = focusing(true);
    let out = say(&mut c, Message::Stop);
    assert_eq!(out.commands, vec![ControllerCommand::JogCancel]);
    assert_eq!(c.state(), State::Focusing);
    assert_eq!(detail_of(&c).as_deref(), Some("Stopping"));
    assert!(say(&mut c, Message::Stop).commands.is_empty(), "once");
    c.handle(at(10.0, 20.0));
    let out = c.handle(Event::JogEnded);
    assert!(out.commands.is_empty(), "no pulse after a cancelled move");
    assert_eq!(c.state(), State::Idle);
}

#[test]
fn stop_during_the_pulse_is_the_stop_of_g12() {
    let mut m = pulsing(false);
    let out = say(&mut m, Message::Stop);
    assert_eq!(out.commands, vec![ControllerCommand::Stop]);
    assert_eq!(m.state(), State::Idle);

    let mut c = pulsing(true);
    let out = say(&mut c, Message::Stop);
    assert_eq!(out.commands, vec![ControllerCommand::Stop]);
    assert_eq!(c.state(), State::Pulsing);
    assert!(say(&mut c, Message::Stop).commands.is_empty(), "once");
    c.handle(Event::Stopped);
    assert_eq!(c.state(), State::Idle);
    let Message::Status { homed, .. } = c.status() else {
        panic!()
    };
    assert!(homed, "the stop's reset keeps homed since connect");
}

#[test]
fn a_pulse_not_ended_within_its_length_and_one_second_is_stopped() {
    for confirming in [false, true] {
        // The pulse started at the machine's last clock (zero): the deadline counts from there.
        let mut m = pulsing(confirming);
        let t0 = Duration::ZERO;
        assert!(
            m.handle(Event::Clock(t0 + Duration::from_millis(900)))
                .commands
                .is_empty(),
            "nothing is written"
        );
        let late = t0 + Duration::from_millis(1_299);
        assert!(
            m.handle(Event::Clock(late)).commands.is_empty(),
            "nothing is written"
        );
        let out = m.handle(Event::Clock(t0 + Duration::from_millis(1_300)));
        assert_eq!(out.commands, vec![ControllerCommand::Stop]);
        if confirming {
            assert_eq!(m.state(), State::Pulsing);
            m.handle(Event::Stopped);
        }
        assert_eq!(m.state(), State::Idle);
        assert_eq!(detail_of(&m).as_deref(), Some("The pulse did not end"));
    }
}

#[test]
fn the_pulse_ends_by_its_own_end_a_failure_or_a_skip() {
    let mut m = pulsing(false);
    m.handle(Event::PulseEnded);
    assert_eq!(m.state(), State::Idle);
    assert_eq!(detail_of(&m), None);

    let mut m = pulsing(false);
    m.handle(Event::PulseFailed {
        detail: "Unsupported command (error 20)".into(),
    });
    assert_eq!(m.state(), State::Idle);
    assert_eq!(
        detail_of(&m).as_deref(),
        Some("Unsupported command (error 20)")
    );

    let mut m = pulsing(false);
    m.handle(Event::PulseSkipped);
    assert_eq!(m.state(), State::Idle);
}

#[test]
fn a_busy_word_during_the_pulse_stops_it_and_an_alarm_ends_it() {
    let mut m = pulsing(true);
    let out = m.handle(Event::ControllerWord(Some("Door".into())));
    assert_eq!(out.commands, vec![ControllerCommand::Stop]);
    m.handle(Event::Stopped);
    assert_eq!(m.state(), State::Idle);

    let mut m = pulsing(true);
    refuses_a_job(&mut m);
    m.handle(Event::Alarm {
        text: "Hard limit".into(),
    });
    assert_eq!(m.state(), State::Alarm);
    assert_eq!(target_of(&m), None);

    let mut m = pulsing(false);
    m.handle(Event::ControllerLost);
    assert_eq!(m.state(), State::NoController);
    assert_eq!(detail_of(&m).as_deref(), Some("Lost the controller"));
}

/// A pulsing machine refuses a submitted job as busy.
fn refuses_a_job(m: &mut Machine) {
    let out = say(m, submit(JOB_TEXT, JobKind::Job));
    assert!(out.messages.contains(&Message::JobRejected {
        job_id: job_id(),
        reason: RejectReason::Busy,
    }));
}

#[test]
fn while_it_moves_or_pulses_other_commands_are_refused_with_its_word() {
    for (mut m, word) in [
        (focusing(false), "Focus point."),
        (pulsing(false), "Pulse."),
    ] {
        for message in [
            Message::Home,
            Message::Pause,
            Message::Resume,
            Message::Start { job_id: job_id() },
        ] {
            let out = say(&mut m, message);
            assert_eq!(
                error_of(&out),
                Some((ErrorCode::WrongState, word.to_owned()))
            );
            assert!(out.commands.is_empty(), "nothing is written");
        }
        assert!(
            say(&mut m, Message::JogStop).messages.is_empty(),
            "nothing is written"
        );
        let out = say(&mut m, submit(JOB_TEXT, JobKind::Job));
        assert!(out.messages.contains(&Message::JobRejected {
            job_id: job_id(),
            reason: RejectReason::Busy,
        }));
    }
}

#[test]
fn under_require_local_start_a_go_to_point_waits_for_enter_and_stop_discards_it() {
    let mut m = machine(false, LOCAL);
    m.handle(Event::Homed);
    let out = say(&mut m, go(10.0, 20.0, Some(pulse())));
    assert!(out.commands.is_empty(), "nothing is written");
    assert_eq!(m.state(), State::AwaitingLocalStart);
    assert_eq!(target_of(&m), Some([10.0, 20.0]));
    let Message::Status { job, .. } = m.status() else {
        panic!()
    };
    assert_eq!(job, None);
    let out = m.handle(Event::LocalStartPressed);
    assert!(matches!(
        out.commands.as_slice(),
        [ControllerCommand::GoTo { .. }]
    ));
    assert_eq!(m.state(), State::Focusing);
    m.handle(Event::JogEnded);

    say(&mut m, go(10.0, 20.0, None));
    assert_eq!(m.state(), State::AwaitingLocalStart);
    let out = say(&mut m, Message::Stop);
    assert!(out.commands.is_empty(), "nothing is written");
    assert_eq!(m.state(), State::Idle);
    assert_eq!(target_of(&m), None);
    assert!(
        m.handle(Event::LocalStartPressed).commands.is_empty(),
        "nothing is written"
    );
}

#[test]
fn a_reset_ends_the_move_and_the_target() {
    let mut m = focusing(false);
    m.handle(Event::ControllerReset);
    assert_eq!(m.state(), State::Idle);
    assert_eq!(target_of(&m), None);
    assert_eq!(m.focus_client(), None);
}

#[test]
fn the_machine_names_who_pressed() {
    let m = focusing(false);
    assert_eq!(m.focus_client(), Some("Laptop"));
}

#[test]
fn the_named_travel_clamps_the_target_before_the_first_status() {
    for start_rule in [StartRule::Immediate, StartRule::LocalPress] {
        let mut m = machine(true, Options { start_rule });
        m.handle(Event::Homed);
        m.handle(Event::MachinePosition {
            machine: None,
            offset: Some([Millimetres(-100.0), Millimetres(-50.0), Millimetres(0.0)]),
        });
        m.handle(Event::Travel(Some(TravelBox::new(
            [400.0, 300.0],
            [false; 2],
        ))));
        let out = say(&mut m, go(500.0, -900.0, None));
        let clamped = [100.0, -250.0];
        for message in &out.messages {
            if let Message::Status { target, .. } = message {
                assert_eq!(
                    target.map(|t| t.map(|v| v.0)),
                    Some(clamped),
                    "{start_rule:?}"
                );
            }
        }
        assert_eq!(target_of(&m), Some(clamped));
    }
}
