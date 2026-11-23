//! `specs/0118` slice 2b at the hub with the fake controller (adrs F6): a frame runs in the time
//! passed in, moves the head, returns to the state it came from, is never listed and writes the
//! one `Frame: <client>` line and no result line. The hub is driven by hand with made-up
//! instants: no sleep, no wall time.
#![allow(clippy::unwrap_used, clippy::float_cmp)]

use std::time::{Duration, Instant};

use curvyo_connector_core::{JobKind, Message, Peer, Role, State};
use curvyo_connector_io::hub::{ControllerChoice, FakeSettings, Hub, HubSettings};
use curvyo_connector_io::spki::SpkiHash;
use sha2::{Digest, Sha256};
use uuid::Uuid;

const FRAME: &str = "G21\nG90\nM5\nM9\nG1 X30.000 Y0.000 F3000 S0\nG1 X30.000 Y20.000 F3000 S0\nG1 X0.000 Y20.000 F3000 S0\nG1 X0.000 Y0.000 F3000 S0\n";
const JOB: &str = "G21\nG90\nG0 X0 Y0\nM4 S0\nG1 X10 Y10 F1000 S500\nM5\n";

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

fn hub(dir: &tempfile::TempDir, started: Instant) -> Hub {
    let (hub, _notices) = Hub::new(
        HubSettings {
            state_dir: dir.path().to_owned(),
            controller: ControllerChoice::Fake(FakeSettings {
                lines_per_second: 20,
                alarm_at_line: Some(2),
            }),
            require_local_start: false,
            connector_name: "Test".into(),
            spki: SpkiHash::of_spki_der(b"test"),
        },
        started,
    );
    hub
}

fn sha(text: &str) -> String {
    use std::fmt::Write as _;
    Sha256::digest(text.as_bytes())
        .iter()
        .fold(String::new(), |mut s, b| {
            let _ = write!(s, "{b:02x}");
            s
        })
}

fn send(hub: &Hub, id: Uuid, kind: JobKind, text: &str, peer: &Peer, now: Instant) {
    let submit = Message::SubmitJob {
        job_id: id,
        name: "x".into(),
        kind,
        bytes: text.len() as u64,
        sha256: sha(text),
    };
    hub.handle_message(submit, peer, now);
    let chunk = Message::JobChunk {
        job_id: id,
        index: 0,
        text: text.to_owned(),
    };
    hub.handle_message(chunk, peer, now);
}

fn position(hub: &Hub) -> Option<[f64; 2]> {
    match hub.status() {
        Message::Status {
            machine_position: Some(p),
            ..
        } => Some([p[0].0, p[1].0]),
        _ => None,
    }
}

#[test]
fn a_frame_moves_the_head_in_passed_time_and_leaves_no_result_line() {
    let dir = tempfile::tempdir().unwrap();
    let t0 = Instant::now();
    let hub = hub(&dir, t0);
    let laptop = Peer::new("Laptop", Role::Owner);
    send(&hub, Uuid::from_u128(1), JobKind::Job, JOB, &laptop, t0);
    assert_eq!(hub.state(), State::Ready);
    send(&hub, Uuid::from_u128(2), JobKind::Frame, FRAME, &laptop, t0);
    assert_eq!(hub.state(), State::Framing);
    // 3,000 mm/min is 50 mm/s: the 30 mm of the first side take 600 ms.
    hub.tick(t0 + ms(300));
    let halfway = position(&hub).unwrap();
    assert!(halfway[0] > 10.0 && halfway[0] < 20.0, "{halfway:?}");
    assert_eq!(hub.state(), State::Framing);
    // 20 lines a second, 8 lines: done at 400 ms by the line pace; the head is the fake's.
    hub.tick(t0 + ms(5_000));
    assert_eq!(hub.state(), State::Ready, "the Ready job is kept");
    assert_eq!(position(&hub).unwrap(), [0.0, 0.0], "ends where it began");
    // The fake's alarm at line 2 did not touch the frame.
    let log = std::fs::read_to_string(dir.path().join("jobs.log")).unwrap();
    assert!(log.contains("\t\tFrame: Laptop\t"), "{log}");
    assert!(
        !log.contains("Completed") && !log.contains("Stopped"),
        "{log}"
    );
    let Message::JobList {
        current, waiting, ..
    } = hub.job_list()
    else {
        unreachable!()
    };
    assert_eq!(current.unwrap().job_id, Uuid::from_u128(1));
    assert!(waiting.is_empty(), "a frame is never listed");
}

#[test]
fn stop_ends_the_head_where_it_stands() {
    let dir = tempfile::tempdir().unwrap();
    let t0 = Instant::now();
    let hub = hub(&dir, t0);
    let me = Peer::host();
    send(&hub, Uuid::from_u128(2), JobKind::Frame, FRAME, &me, t0);
    hub.tick(t0 + ms(300));
    hub.handle_message(Message::Stop, &me, t0 + ms(300));
    assert_eq!(hub.state(), State::Idle);
    let there = position(&hub).unwrap();
    hub.tick(t0 + ms(5_000));
    assert_eq!(position(&hub).unwrap(), there, "the head stands");
    assert!(there[0] < 30.0);
}
