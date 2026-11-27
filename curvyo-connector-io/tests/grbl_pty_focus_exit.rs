//! `specs/0118` slice 2d over a real pty (adrs P6): the connector's exit during a pulse names the
//! focus point and waits until GRBL confirms the stop, so the reset of `0127` G12 reaches the
//! controller before the port closes, not only the feed hold. Waiting is by status, never by
//! sleep; the connector's limits are shortened.
#![cfg(target_os = "linux")]
#![allow(clippy::unwrap_used)]

mod common;

use std::time::{Duration, Instant};

use common::grbl_sim::{GrblSim, SimConfig};
use common::pty_link::PtySim;
use common::{WAIT, state_of, wait_for, wait_state};
use curvyo_connector_core::{Message, Millimetres, Pulse, PulseLength, PulsePower, State};
use curvyo_connector_io::client::{CONNECT_TIMEOUT, connect, open_paired};
use curvyo_connector_io::grbl::Timing;
use curvyo_connector_io::hub::{ControllerChoice, SerialSettings};
use curvyo_connector_io::pin_verifier::PinMode;
use curvyo_connector_io::server::{RunningServer, ServerConfig};

const RESET: u8 = 0x18;

fn timing() -> Timing {
    Timing {
        retry: Duration::from_millis(100),
        identify: Duration::from_secs(2),
        loss: Duration::from_secs(1),
        hold: Duration::from_secs(1),
        banner: Duration::from_secs(1),
        poll_fast: Duration::from_millis(50),
        poll_slow: Duration::from_millis(200),
        ..Timing::default()
    }
}

#[test]
fn the_exit_during_a_pulse_waits_for_the_confirmed_stop_and_its_reset() {
    let dir = tempfile::tempdir().unwrap();
    let link = dir.path().join("ttyGRBL");
    let pty = PtySim::plug(
        GrblSim::new(SimConfig {
            mpos_reports: true,
            homing_lock: true,
            homing_steps: 20,
            travel: [400.0, 300.0],
            ..SimConfig::default()
        }),
        &link,
    );
    let server = RunningServer::start(&ServerConfig {
        listen: "127.0.0.1:0".parse().unwrap(),
        state_dir: dir.path().join("state"),
        controller: ControllerChoice::Serial(SerialSettings {
            port: link.to_str().unwrap().to_owned(),
            baud: 115_200,
            spindle: false,
            timing: timing(),
        }),
        require_local_start: false,
        connector_name: "Test connector".to_owned(),
        issue_code: false,
    })
    .unwrap();
    let address = server.addr().to_string();
    let record = connect(&address, PinMode::Capture, CONNECT_TIMEOUT)
        .unwrap()
        .pair(
            &server.current_code().unwrap(),
            "Laptop",
            Instant::now() + WAIT,
        )
        .unwrap();
    let (mut c, first) =
        open_paired(&address, record.spki, &record.token, CONNECT_TIMEOUT).unwrap();
    if state_of(&first) != State::Alarm {
        wait_state(&mut c, State::Alarm);
    }
    c.send(&Message::Home).unwrap();
    wait_for(&mut c, |s| {
        state_of(s) == State::Idle && matches!(s, Message::Status { homed: true, .. })
    });
    c.send(&Message::GoToPoint {
        x: Millimetres(-10.0),
        y: Millimetres(-10.0),
        pulse: Some(Pulse {
            power_permille: PulsePower::new(10).unwrap(),
            length_ms: PulseLength::new(2_000).unwrap(),
        }),
    })
    .unwrap();
    wait_state(&mut c, State::Pulsing);
    let mark = pty.sim.lock().unwrap().log.len();

    assert_eq!(server.stop_for_exit().as_deref(), Some("Focus point"));
    server.await_exit_stop();

    assert_eq!(server.state(), State::Idle, "GRBL confirmed the stop");
    let sim = pty.sim.lock().unwrap();
    assert!(
        sim.log[mark..].contains(&RESET),
        "the reset went out before the exit: {:?}",
        &sim.log[mark..]
    );
    assert!(!sim.laser_lit());
    drop(sim);
    drop(c);
    server.shutdown();
}
