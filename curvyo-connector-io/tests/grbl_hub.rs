//! `specs/0127` G15 at the hub: `GetStatus` judges a stale controller before it answers, so a
//! serial thread that died never leaves `Running` on show. The hub is driven by hand with the
//! simulator and made-up instants, as the serial thread would drive it.
#![allow(clippy::unwrap_used, clippy::format_collect)]

mod common;

use std::time::{Duration, Instant};

use common::grbl_sim::{GrblSim, SimConfig};
use curvyo_connector_core::{JobKind, Message, State};
use curvyo_connector_io::grbl::Timing;
use curvyo_connector_io::hub::{ControllerChoice, Hub, HubSettings, SerialSettings};
use curvyo_connector_io::spki::SpkiHash;
use sha2::{Digest, Sha256};
use uuid::Uuid;

const DT: Duration = Duration::from_millis(10);

fn state_of(message: &Message) -> State {
    match message {
        Message::Status { state, .. } => *state,
        other => panic!("{other:?}"),
    }
}

#[test]
fn g15_get_status_after_the_serial_thread_died_is_not_running() {
    let dir = tempfile::tempdir().unwrap();
    let serial = SerialSettings {
        port: "/dev/ttyHub".into(),
        baud: 115_200,
        timing: Timing::default(),
    };
    let started = Instant::now();
    let (hub, _notices) = Hub::new(
        HubSettings {
            state_dir: dir.path().to_owned(),
            controller: ControllerChoice::Serial(serial.clone()),
            require_local_start: false,
            connector_name: "Test".into(),
            spki: SpkiHash::of_spki_der(b"test"),
        },
        started,
    );
    hub.attach_serial(&serial, started);
    let mut sim = GrblSim::new(SimConfig {
        steps_per_motion: 50,
        ..SimConfig::default()
    });
    let mut at = Duration::ZERO;
    let turn = |hub: &Hub, sim: &mut GrblSim, at: &mut Duration| {
        *at += DT;
        let exchange = hub.exchange(&sim.step(), started + *at);
        sim.receive(&exchange.write);
    };
    while hub.state() != State::Idle {
        turn(&hub, &mut sim, &mut at);
        assert!(at < Duration::from_secs(5), "identified");
    }
    let text = "G21\nM4 S0\nG1 X100 F600 S500\nG1 X0\nG1 X100\nG1 X0\nM5\n";
    let id = Uuid::from_u128(1);
    let sha256 = Sha256::digest(text.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let now = started + at;
    hub.handle_message(
        Message::SubmitJob {
            job_id: id,
            name: "hub.gcode".into(),
            kind: JobKind::Job,
            bytes: text.len() as u64,
            sha256,
        },
        now,
    );
    hub.handle_message(
        Message::JobChunk {
            job_id: id,
            index: 0,
            text: text.into(),
        },
        now,
    );
    hub.handle_message(Message::Start { job_id: id }, now);
    for _ in 0..20 {
        turn(&hub, &mut sim, &mut at);
    }
    assert_eq!(hub.state(), State::Running);
    // The serial thread stops turning; only `GetStatus` comes, 3 s later.
    let answer = hub.handle_message(Message::GetStatus, started + at + Duration::from_secs(3));
    assert_eq!(answer.len(), 1);
    assert_eq!(state_of(&answer[0]), State::NoController, "{answer:?}");
}
