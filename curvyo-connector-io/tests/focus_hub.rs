//! `specs/0118` slice 2d at the hub with the fake controller (criterion 35 c, adrs P9, P10): a go
//! to point moves the head in the time passed in, its pulse is a dwell of its length, the move
//! and the pulse write their own `jobs.log` lines, another client sees `Focusing` and `Pulsing`
//! and may Stop, and the shutdown stops a pulse. Made-up instants: no sleep, no wall time.
#![allow(clippy::unwrap_used, clippy::float_cmp)]

use std::time::{Duration, Instant};

use curvyo_connector_core::{
    Message, Millimetres, Peer, Pulse, PulseLength, PulsePower, Role, State,
};
use curvyo_connector_io::hub::{ControllerChoice, FakeSettings, Hub, HubSettings};
use curvyo_connector_io::spki::SpkiHash;

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

fn hub(dir: &tempfile::TempDir, started: Instant) -> Hub {
    let (hub, _notices) = Hub::new(
        HubSettings {
            state_dir: dir.path().to_owned(),
            controller: ControllerChoice::Fake(FakeSettings {
                lines_per_second: 20,
                alarm_at_line: None,
            }),
            require_local_start: false,
            connector_name: "Test".into(),
            spki: SpkiHash::of_spki_der(b"test"),
        },
        started,
    );
    hub
}

/// A hub whose fake controller has homed by `t0 + 2 s`.
fn homed(dir: &tempfile::TempDir, t0: Instant) -> Hub {
    let hub = hub(dir, t0);
    hub.home(&Peer::host(), t0);
    hub.tick(t0 + ms(2_000));
    assert!(matches!(hub.status(), Message::Status { homed: true, .. }));
    hub
}

fn point(x: f64, y: f64, length: Option<u16>) -> Message {
    Message::GoToPoint {
        x: Millimetres(x),
        y: Millimetres(y),
        pulse: length.map(|ms| Pulse {
            power_permille: PulsePower::new(10).unwrap(),
            length_ms: PulseLength::new(ms).unwrap(),
        }),
    }
}

fn log(dir: &tempfile::TempDir) -> String {
    std::fs::read_to_string(dir.path().join("jobs.log")).unwrap_or_default()
}

#[test]
fn the_move_and_the_pulse_each_write_their_line() {
    let dir = tempfile::tempdir().unwrap();
    let t0 = Instant::now();
    let hub = homed(&dir, t0);
    let laptop = Peer::new("Laptop", Role::Owner);
    let start = t0 + ms(3_000);
    let answers = hub.go_to_point(point(30.0, 0.0, Some(300)), &laptop, start);
    assert!(matches!(
        answers.as_slice(),
        [Message::Status {
            state: State::Focusing,
            ..
        }]
    ));
    assert!(log(&dir).contains("\t\tFocus point: Laptop\t"));
    assert!(!log(&dir).contains("Focus pulse"));
    // 30 mm at 3,000 mm/min: 600 ms.
    hub.tick(start + ms(300));
    assert_eq!(hub.state(), State::Focusing);
    hub.tick(start + ms(700));
    assert_eq!(hub.state(), State::Pulsing);
    assert!(
        log(&dir).contains("\t\tFocus pulse: Laptop\t"),
        "{}",
        log(&dir)
    );
    hub.tick(start + ms(900));
    assert_eq!(hub.state(), State::Pulsing);
    hub.tick(start + ms(1_100));
    assert_eq!(hub.state(), State::Idle);
    let lines = log(&dir);
    assert!(
        !lines.contains("Completed") && !lines.contains("Stopped"),
        "{lines}"
    );
}

#[test]
fn without_a_pulse_only_the_move_is_logged() {
    let dir = tempfile::tempdir().unwrap();
    let t0 = Instant::now();
    let hub = homed(&dir, t0);
    let laptop = Peer::new("Laptop", Role::Owner);
    let start = t0 + ms(3_000);
    hub.go_to_point(point(10.0, 0.0, None), &laptop, start);
    hub.tick(start + ms(1_000));
    assert_eq!(hub.state(), State::Idle);
    let lines = log(&dir);
    assert!(lines.contains("Focus point: Laptop"));
    assert!(!lines.contains("Focus pulse"));
}

#[test]
fn another_client_sees_both_states_and_may_stop_the_pulse() {
    let dir = tempfile::tempdir().unwrap();
    let t0 = Instant::now();
    let hub = homed(&dir, t0);
    let statuses = hub.subscribe();
    let laptop = Peer::new("Laptop", Role::Owner);
    let phone = Peer::new("Phone", Role::Member);
    let start = t0 + ms(3_000);
    hub.go_to_point(point(5.0, 0.0, Some(2_000)), &laptop, start);
    hub.tick(start + ms(200));
    assert_eq!(hub.state(), State::Pulsing);
    hub.handle_message(Message::Stop, &phone, start + ms(300));
    assert_eq!(hub.state(), State::Idle);
    let seen: Vec<State> = statuses
        .try_iter()
        .filter_map(|m| match m {
            Message::Status { state, .. } => Some(state),
            _ => None,
        })
        .collect();
    assert!(seen.contains(&State::Focusing), "{seen:?}");
    assert!(seen.contains(&State::Pulsing), "{seen:?}");
    // The fake's dwell was cancelled: nothing ends later.
    hub.tick(start + ms(5_000));
    assert_eq!(hub.state(), State::Idle);
}

#[test]
fn the_shutdown_stops_a_running_pulse() {
    let dir = tempfile::tempdir().unwrap();
    let t0 = Instant::now();
    let hub = homed(&dir, t0);
    let start = t0 + ms(3_000);
    hub.go_to_point(point(5.0, 0.0, Some(2_000)), &Peer::host(), start);
    hub.tick(start + ms(200));
    assert_eq!(hub.state(), State::Pulsing);
    assert_eq!(
        hub.stop_for_exit(start + ms(300)).as_deref(),
        Some("Focus point")
    );
    assert_eq!(hub.state(), State::Idle);
}

#[test]
fn a_refused_go_to_point_writes_no_line() {
    let dir = tempfile::tempdir().unwrap();
    let t0 = Instant::now();
    let hub = hub(&dir, t0);
    let answers = hub.go_to_point(point(5.0, 0.0, None), &Peer::host(), t0);
    assert!(matches!(
        answers.as_slice(),
        [Message::Error { message, .. }] if message == "Not homed."
    ));
    assert!(!log(&dir).contains("Focus point"));
}
