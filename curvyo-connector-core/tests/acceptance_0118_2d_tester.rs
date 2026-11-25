//! Independent acceptance tests for `specs/0118-machine-control` slice 2d (criteria 10, 12, 17,
//! 34, 35 c, 36; `adrs.md` P2 to P4, P6, P12), written from the specification before reading the
//! implementation. The job machine runs on injected time (`Event::Clock`); no wall clock.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::missing_panics_doc,
    missing_docs
)]

use std::time::Duration;

use curvyo_connector_core::{
    ARRIVED, ControllerCommand, ErrorCode, Event, JogStep, Machine, Message, Millimetres, Options,
    Output, PULSE_GRACE, Peer, PulseLength, PulsePower, State, TravelBox, arrived, clamp_target,
};

/// An identified controller, homed since connect, `Idle` (criterion 34: the toggle needs homing).
fn idle_homed() -> Machine {
    let mut m = Machine::confirming(Options::default());
    m.handle(Event::ControllerConnected);
    m.handle(Event::LaserModeOff(false));
    m.handle(Event::LaserModeUnknown(false));
    m.handle(Event::Homed);
    assert_eq!(m.state(), State::Idle, "the setup reaches Idle");
    m
}

fn send(m: &mut Machine, message: Message) -> Output {
    m.handle(Event::Message(message, Peer::host()))
}

fn go_to(x: f64, y: f64, pulse: Option<(u16, u16)>) -> Message {
    let mut value = serde_json::json!({ "type": "GoToPoint", "x": x, "y": y });
    if let Some((power_permille, length_ms)) = pulse {
        value["pulse"] = serde_json::json!({
            "power_permille": power_permille,
            "length_ms": length_ms,
        });
    }
    serde_json::from_value(value).unwrap()
}

fn mm(v: [f64; 3]) -> [Millimetres; 3] {
    v.map(Millimetres)
}

/// Starts a go to point at (x, y) and returns the machine in `Focusing`.
fn focusing(x: f64, y: f64, pulse: Option<(u16, u16)>) -> Machine {
    let mut m = idle_homed();
    let out = send(&mut m, go_to(x, y, pulse));
    assert!(
        out.commands
            .iter()
            .any(|c| matches!(c, ControllerCommand::GoTo { .. })),
        "the move is taken: {:?}",
        out.commands
    );
    assert_eq!(m.state(), State::Focusing);
    m
}

fn has_pulse(out: &Output) -> bool {
    out.commands
        .iter()
        .any(|c| matches!(c, ControllerCommand::Pulse { .. }))
}

fn has_stop(out: &Output) -> bool {
    out.commands
        .iter()
        .any(|c| matches!(c, ControllerCommand::Stop))
}

fn has_jog_cancel(out: &Output) -> bool {
    out.commands
        .iter()
        .any(|c| matches!(c, ControllerCommand::JogCancel))
}

// ---------------------------------------------------------------------------------------------
// Criterion 34, P3: accepted when homed and idle; the move at the 10 mm feed; the target
// ---------------------------------------------------------------------------------------------

#[test]
fn a_go_to_point_is_taken_when_homed_and_idle_at_the_ten_mm_feed() {
    let mut m = idle_homed();
    let out = send(&mut m, go_to(12.5, -40.0, Some((10, 300))));
    let goto = out
        .commands
        .iter()
        .find_map(|c| match c {
            ControllerCommand::GoTo { target, feed } => Some((*target, *feed)),
            _ => None,
        })
        .expect("a GoTo command");
    assert_eq!(goto.0, [Millimetres(12.5), Millimetres(-40.0)]);
    assert_eq!(goto.1, JogStep::Ten.feed(), "the 10 mm jog feed");
    assert_eq!(m.state(), State::Focusing);
    assert!(
        out.messages.iter().any(|msg| matches!(
            msg,
            Message::Status {
                state: State::Focusing,
                target: Some(_),
                ..
            }
        )),
        "Status.target is the target while Focusing: {:?}",
        out.messages
    );
}

#[test]
fn a_go_to_point_without_homing_is_refused_and_nothing_is_commanded() {
    let mut m = Machine::confirming(Options::default());
    m.handle(Event::ControllerConnected);
    m.handle(Event::LaserModeOff(false));
    m.handle(Event::LaserModeUnknown(false));
    assert_eq!(m.state(), State::Idle);
    let out = send(&mut m, go_to(10.0, 10.0, Some((10, 300))));
    assert!(
        out.commands.is_empty(),
        "not homed: no command {:?}",
        out.commands
    );
    assert_eq!(m.state(), State::Idle);
}

#[test]
fn a_go_to_point_in_alarm_is_refused() {
    let mut m = idle_homed();
    m.handle(Event::Alarm {
        text: "Alarm: Hard limit".into(),
    });
    assert_eq!(m.state(), State::Alarm);
    let out = send(&mut m, go_to(10.0, 10.0, None));
    assert!(
        out.commands.is_empty(),
        "no move in Alarm: {:?}",
        out.commands
    );
    assert_eq!(m.state(), State::Alarm);
}

#[test]
fn a_second_go_to_point_while_focusing_or_pulsing_is_refused_with_its_own_word() {
    let mut m = focusing(10.0, 10.0, Some((10, 300)));
    let out = send(&mut m, go_to(20.0, 20.0, None));
    assert!(
        out.commands.is_empty(),
        "no second move while focusing: {:?}",
        out.commands
    );
    assert!(
        out.messages.iter().any(|msg| matches!(
            msg,
            Message::Error {
                code: ErrorCode::WrongState,
                ..
            }
        )),
        "WrongState: {:?}",
        out.messages
    );
    assert_eq!(m.state(), State::Focusing);
}

// ---------------------------------------------------------------------------------------------
// Criterion 35 c and P3: the pulse only after arrival within 0.05 mm, never with laser flags
// ---------------------------------------------------------------------------------------------

#[test]
fn the_pulse_follows_an_arrival_within_the_tolerance_with_the_asked_power_and_length() {
    let mut m = focusing(12.5, -40.0, Some((10, 300)));
    m.handle(Event::Position(mm([12.5 + 0.049, -40.0, 0.0])));
    let out = m.handle(Event::JogEnded);
    let pulse = out
        .commands
        .iter()
        .find_map(|c| match c {
            ControllerCommand::Pulse { power, length } => Some((*power, *length)),
            _ => None,
        })
        .expect("a Pulse command after the arrival");
    assert_eq!(pulse.0, PulsePower::new(10).unwrap());
    assert_eq!(pulse.1, PulseLength::new(300).unwrap());
    assert_eq!(m.state(), State::Pulsing);
}

#[test]
fn an_arrival_just_outside_the_tolerance_writes_no_pulse_and_returns_to_idle() {
    let mut m = focusing(12.5, -40.0, Some((10, 300)));
    m.handle(Event::Position(mm([12.5 + 0.051, -40.0, 0.0])));
    let out = m.handle(Event::JogEnded);
    assert!(!has_pulse(&out), "no pulse at 0.051 mm: {:?}", out.commands);
    assert_eq!(m.state(), State::Idle);
}

#[test]
fn a_move_without_a_pulse_asked_ends_idle_with_no_pulse() {
    let mut m = focusing(5.0, 5.0, None);
    m.handle(Event::Position(mm([5.0, 5.0, 0.0])));
    let out = m.handle(Event::JogEnded);
    assert!(!has_pulse(&out));
    assert_eq!(m.state(), State::Idle);
}

#[test]
fn laser_mode_off_or_unknown_on_the_controller_gives_no_pulse_and_the_head_still_arrives() {
    for flag in [Event::LaserModeOff(true), Event::LaserModeUnknown(true)] {
        let mut m = focusing(8.0, 8.0, Some((10, 300)));
        m.handle(flag);
        m.handle(Event::Position(mm([8.0, 8.0, 0.0])));
        let out = m.handle(Event::JogEnded);
        assert!(
            !has_pulse(&out),
            "no pulse with the laser flag set: {:?}",
            out.commands
        );
        assert_eq!(m.state(), State::Idle);
    }
}

#[test]
fn a_pulse_that_ended_returns_to_idle_and_sends_nothing_more() {
    let mut m = focusing(12.5, -40.0, Some((10, 300)));
    m.handle(Event::Position(mm([12.5, -40.0, 0.0])));
    m.handle(Event::JogEnded);
    assert_eq!(m.state(), State::Pulsing);
    let out = m.handle(Event::PulseEnded);
    assert_eq!(m.state(), State::Idle);
    assert!(
        out.commands.is_empty(),
        "nothing after the pulse: {:?}",
        out.commands
    );
}

#[test]
fn a_stop_while_focusing_cancels_the_jog_and_no_pulse_follows() {
    let mut m = focusing(12.5, -40.0, Some((10, 300)));
    let out = send(&mut m, Message::Stop);
    assert!(has_jog_cancel(&out), "the jog cancel: {:?}", out.commands);
    assert!(!has_stop(&out), "no G12 stop for a jog: {:?}", out.commands);
    m.handle(Event::Position(mm([12.5, -40.0, 0.0])));
    let late = m.handle(Event::JogEnded);
    assert!(!has_pulse(&late), "no pulse after a stopped move");
    assert_eq!(m.state(), State::Idle);
}

#[test]
fn a_stop_while_pulsing_runs_the_g12_stop() {
    let mut m = focusing(12.5, -40.0, Some((10, 300)));
    m.handle(Event::Position(mm([12.5, -40.0, 0.0])));
    m.handle(Event::JogEnded);
    assert_eq!(m.state(), State::Pulsing);
    let out = send(&mut m, Message::Stop);
    assert!(
        has_stop(&out),
        "the stop sequence of G12: {:?}",
        out.commands
    );
}

#[test]
fn a_busy_pulse_refuses_any_other_motion_or_job_word_with_its_own_word() {
    let mut m = focusing(12.5, -40.0, Some((10, 300)));
    m.handle(Event::Position(mm([12.5, -40.0, 0.0])));
    m.handle(Event::JogEnded);
    assert_eq!(m.state(), State::Pulsing);
    for message in [
        go_to(1.0, 1.0, None),
        serde_json::from_str::<Message>(r#"{"type":"Home"}"#).unwrap(),
        serde_json::from_str::<Message>(r#"{"type":"Pause"}"#).unwrap(),
    ] {
        let out = send(&mut m, message);
        assert!(
            out.commands.iter().all(|c| !matches!(
                c,
                ControllerCommand::GoTo { .. } | ControllerCommand::Home | ControllerCommand::Pause
            )),
            "no motion or pause while pulsing: {:?}",
            out.commands
        );
    }
    assert_eq!(m.state(), State::Pulsing);
}

// ---------------------------------------------------------------------------------------------
// Criterion 35 c and P2: the caps are the connector's; refusals before any state change
// ---------------------------------------------------------------------------------------------

#[test]
fn the_wire_caps_refuse_power_and_length_outside_their_range() {
    for (p, l) in [(0u16, 300u16), (51, 300), (10, 49), (10, 2001)] {
        let text = format!(
            r#"{{"type":"GoToPoint","x":1,"y":1,"pulse":{{"power_permille":{p},"length_ms":{l}}}}}"#
        );
        assert!(
            serde_json::from_str::<Message>(&text).is_err(),
            "refused: {text}"
        );
    }
    assert!(
        serde_json::from_str::<Message>(
            r#"{"type":"GoToPoint","x":1,"y":1,"pulse":{"power_permille":50,"length_ms":2000}}"#
        )
        .is_ok()
    );
    assert!(
        serde_json::from_str::<Message>(
            r#"{"type":"GoToPoint","x":1,"y":1,"pulse":{"power_permille":1,"length_ms":50}}"#
        )
        .is_ok()
    );
}

#[test]
fn the_caps_are_the_documented_ones() {
    assert_eq!((PulsePower::MIN, PulsePower::MAX), (1, 50), "0.1 to 5 %");
    assert_eq!(
        (PulseLength::MIN, PulseLength::MAX),
        (50, 2000),
        "50 to 2,000 ms"
    );
    assert!(PulsePower::new(0).is_none() && PulsePower::new(51).is_none());
    assert!(PulseLength::new(49).is_none() && PulseLength::new(2001).is_none());
    assert_eq!(ARRIVED, Millimetres(0.05));
}

#[test]
fn the_arrival_test_is_per_axis_within_the_tolerance() {
    let target = [Millimetres(10.0), Millimetres(10.0)];
    assert!(arrived(mm([10.049, 9.951, 0.0]), target));
    assert!(!arrived(mm([10.051, 10.0, 0.0]), target));
    assert!(!arrived(mm([10.0, 9.949, 0.0]), target));
}

// ---------------------------------------------------------------------------------------------
// Criterion 17 and 34: the clamp is one function of the target, the offset and the travel
// ---------------------------------------------------------------------------------------------

#[test]
fn a_target_inside_the_travel_is_kept_and_one_beyond_it_is_clamped_to_the_box() {
    let within = TravelBox::new([380.0, 300.0], [false, false]);
    let inside = clamp_target(
        [Millimetres(-10.0), Millimetres(-20.0)],
        [Millimetres(0.0), Millimetres(0.0)],
        Some(&within),
    );
    assert_eq!(inside, [Millimetres(-10.0), Millimetres(-20.0)]);
    let far = clamp_target(
        [Millimetres(-1000.0), Millimetres(-1000.0)],
        [Millimetres(0.0), Millimetres(0.0)],
        Some(&within),
    );
    assert!(far[0].0.abs() <= 380.0 + 1e-9, "x inside: {far:?}");
    assert!(far[1].0.abs() <= 300.0 + 1e-9, "y inside: {far:?}");
    assert!(
        far[0].0.abs() > 1.0 && far[1].0.abs() > 1.0,
        "moved to the edge: {far:?}"
    );
}

#[test]
fn without_a_known_travel_the_target_is_not_clamped() {
    let far = clamp_target(
        [Millimetres(1000.0), Millimetres(-1000.0)],
        [Millimetres(0.0), Millimetres(0.0)],
        None,
    );
    assert_eq!(far, [Millimetres(1000.0), Millimetres(-1000.0)]);
}

#[test]
fn the_clamp_respects_the_work_offset_so_a_target_is_in_job_coordinates() {
    let within = TravelBox::new([380.0, 300.0], [false, false]);
    // Job -10 with the offset -5 is machine -15, inside the negative travel: unchanged.
    let kept = clamp_target(
        [Millimetres(-10.0), Millimetres(-10.0)],
        [Millimetres(-5.0), Millimetres(-5.0)],
        Some(&within),
    );
    assert_eq!(
        kept,
        [Millimetres(-10.0), Millimetres(-10.0)],
        "job coordinates come back unchanged"
    );
    // Job 0 with the offset -5 is machine -5 (inside); job 0 with offset -400 is machine -400,
    // clamped to -380 and handed back in job coordinates: -380 - (-400) = 20.
    let shifted = clamp_target(
        [Millimetres(0.0), Millimetres(0.0)],
        [Millimetres(-400.0), Millimetres(0.0)],
        Some(&within),
    );
    assert_eq!(
        shifted[0],
        Millimetres(20.0),
        "clamped in machine coordinates: {shifted:?}"
    );
}

// ---------------------------------------------------------------------------------------------
// Criterion 35 c: the decoder gives the core the shape the goldens name
// ---------------------------------------------------------------------------------------------

#[test]
fn the_goldens_decode_to_a_point_with_and_without_a_pulse() {
    let plain: Message =
        serde_json::from_str(r#"{"type":"GoToPoint","x":12.5,"y":-40.0}"#).unwrap();
    let pulse: Message = serde_json::from_str(
        r#"{"type":"GoToPoint","x":12.5,"y":-40.0,"pulse":{"power_permille":10,"length_ms":300}}"#,
    )
    .unwrap();
    assert!(matches!(plain, Message::GoToPoint { pulse: None, .. }));
    assert!(matches!(pulse, Message::GoToPoint { pulse: Some(_), .. }));
    let mut m = idle_homed();
    let out = send(&mut m, plain);
    assert!(!out.commands.is_empty(), "the plain point is accepted");
}
#[test]
fn a_pulse_not_ended_within_its_length_plus_grace_is_stopped_by_the_clock() {
    assert_eq!(PULSE_GRACE, Duration::from_secs(1));
    let mut m = focusing(12.5, -40.0, Some((10, 300)));
    m.handle(Event::Position(mm([12.5, -40.0, 0.0])));
    m.handle(Event::Clock(Duration::from_secs(10)));
    m.handle(Event::JogEnded);
    assert_eq!(m.state(), State::Pulsing);
    let early = m.handle(Event::Clock(
        Duration::from_secs(10) + Duration::from_millis(1000),
    ));
    assert!(!has_stop(&early), "not yet at 1.0 s: {:?}", early.commands);
    let late = m.handle(Event::Clock(
        Duration::from_secs(10) + Duration::from_millis(1301),
    ));
    assert!(
        has_stop(&late),
        "stopped after length + 1 s: {:?}",
        late.commands
    );
    assert!(
        late.messages.iter().any(|msg| matches!(
            msg,
            Message::Status { detail: Some(d), .. } if d == "The pulse did not end"
        )),
        "the sentence names the pulse: {:?}",
        late.messages
    );
}
