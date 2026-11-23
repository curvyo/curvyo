//! `specs/0118` criterion 18 at the hub with the fake controller: a held jog lives on keep-alives
//! and ends within 100 ms of the release, and without one within 350 ms of the last. The hub is
//! driven by hand with made-up instants: no sleep, no wall time.
#![allow(clippy::unwrap_used)]

use std::time::{Duration, Instant};

use curvyo_connector_core::{JogAxis, JogDirection, JogStep, Message, Peer, State};
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

fn hold(axis: JogAxis, direction: JogDirection) -> Message {
    Message::Jog {
        axis,
        direction,
        step: JogStep::Ten,
        hold: true,
    }
}

fn x_of(hub: &Hub) -> f64 {
    match hub.status() {
        Message::Status {
            machine_position: Some(p),
            ..
        } => p[0].0,
        Message::Status { .. } => 0.0,
        other => panic!("{other:?}"),
    }
}

#[test]
fn c18_a_hold_moves_while_keep_alives_come_and_the_release_ends_it_at_once() {
    let dir = tempfile::tempdir().unwrap();
    let t0 = Instant::now();
    let hub = hub(&dir, t0);
    let me = Peer::host();
    hub.jog(hold(JogAxis::X, JogDirection::Plus), &me, t0);
    assert_eq!(hub.state(), State::Jogging);
    for tenth in 1..=10_u64 {
        let now = t0 + ms(tenth * 100);
        hub.tick(now);
        hub.jog(Message::JogKeep, &me, now);
        assert_eq!(hub.state(), State::Jogging, "{tenth}");
    }
    // 3,000 mm/min is 50 mm/s: after a second the head is 50 mm along.
    assert!((x_of(&hub) - 50.0).abs() < 0.5, "{}", x_of(&hub));
    // The release 40 ms after the last keep-alive: back at once, within the 100 ms.
    let release = t0 + ms(1_040);
    hub.tick(release);
    hub.jog(Message::JogStop, &me, release);
    assert_eq!(hub.state(), State::Idle);
    let there = x_of(&hub);
    hub.tick(t0 + ms(5_000));
    assert!((x_of(&hub) - there).abs() < 1e-9, "the head stands");
    assert!(there > 50.0 && there < 53.0, "{there}");
}

#[test]
fn c18_without_a_keep_alive_a_hold_ends_by_350_ms_and_not_before_300() {
    let dir = tempfile::tempdir().unwrap();
    let t0 = Instant::now();
    let hub = hub(&dir, t0);
    let me = Peer::host();
    hub.jog(hold(JogAxis::Y, JogDirection::Minus), &me, t0);
    hub.tick(t0 + ms(299));
    assert_eq!(hub.state(), State::Jogging, "299 ms: still alive");
    hub.tick(t0 + ms(300));
    assert_eq!(hub.state(), State::Idle, "300 ms: cancelled");
    // A late keep-alive and a late release change nothing.
    hub.jog(Message::JogKeep, &me, t0 + ms(320));
    hub.jog(Message::JogStop, &me, t0 + ms(330));
    assert_eq!(hub.state(), State::Idle);
}

#[test]
fn c19_a_step_moves_the_head_and_a_press_during_it_queues_one() {
    let dir = tempfile::tempdir().unwrap();
    let t0 = Instant::now();
    let hub = hub(&dir, t0);
    let me = Peer::host();
    let step = |direction| Message::Jog {
        axis: JogAxis::X,
        direction,
        step: JogStep::Ten,
        hold: false,
    };
    assert!(
        hub.jog(step(JogDirection::Plus), &me, t0)
            .iter()
            .all(|m| matches!(m, Message::Status { .. }))
    );
    hub.jog(step(JogDirection::Plus), &me, t0);
    // The third press is dropped: the answer is the status, not an error.
    let answer = hub.jog(step(JogDirection::Plus), &me, t0);
    assert!(matches!(answer.as_slice(), [Message::Status { .. }]));
    hub.tick(t0 + ms(150));
    assert_eq!(hub.state(), State::Jogging);
    hub.tick(t0 + ms(500));
    assert_eq!(hub.state(), State::Idle);
    assert!(
        (x_of(&hub) - 20.0).abs() < 1e-6,
        "two steps of 10 mm, not three"
    );
}

#[test]
fn c12_one_jobs_log_line_per_pad_session_and_any_other_line_ends_the_run() {
    let dir = tempfile::tempdir().unwrap();
    let t0 = Instant::now();
    let hub = hub(&dir, t0);
    let laptop = Peer::new("Laptop", curvyo_connector_core::Role::Owner);
    let shop = Peer::new("Shop PC", curvyo_connector_core::Role::Owner);
    let one = |step| Message::Jog {
        axis: JogAxis::X,
        direction: JogDirection::Plus,
        step,
        hold: false,
    };
    let mut at = t0;
    let press = |at: &mut Instant, peer: &Peer, message: Message| {
        *at += ms(1_000);
        hub.tick(*at);
        hub.jog(message, peer, *at);
        *at += ms(1_000);
        hub.tick(*at);
    };
    press(&mut at, &laptop, one(JogStep::One));
    press(&mut at, &laptop, one(JogStep::Ten));
    press(&mut at, &laptop, hold(JogAxis::X, JogDirection::Minus));
    press(&mut at, &shop, one(JogStep::One));
    hub.home(&shop, at + ms(10));
    at += ms(3_000);
    hub.tick(at);
    press(&mut at, &shop, one(JogStep::One));
    let log = std::fs::read_to_string(dir.path().join("jobs.log")).unwrap();
    let results: Vec<&str> = log.lines().map(|l| l.split('\t').nth(2).unwrap()).collect();
    assert_eq!(
        results,
        [
            "Jog: Laptop",
            "Jog: Shop PC",
            "Home: Shop PC",
            "Jog: Shop PC"
        ],
        "{log}"
    );
}
