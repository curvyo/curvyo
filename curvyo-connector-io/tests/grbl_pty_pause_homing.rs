//! `specs/0133` over a real pty: `RunningServer` with the serial controller on the simulator's
//! silent homing; a paired client pauses during the job's `$H` (pending, then `Paused` before
//! any further line), and a pending pause is gone after an unplugged port and after a restart
//! of the connector (PH9). Waiting is by status, never by sleep.
#![cfg(target_os = "linux")]
#![allow(clippy::unwrap_used, clippy::assert_is_empty)]

mod common;

use std::path::Path;
use std::time::{Duration, Instant};

use common::grbl_sim::{GrblSim, SimConfig};
use common::pty_link::PtySim;
use common::{WAIT, state_of, wait_for, wait_state};
use curvyo_connector_core::{Message, State};
use curvyo_connector_io::client::{CONNECT_TIMEOUT, Connection, connect, open_paired};
use curvyo_connector_io::grbl::Timing;
use curvyo_connector_io::hub::{ControllerChoice, SerialSettings};
use curvyo_connector_io::pin_verifier::PinMode;
use curvyo_connector_io::server::{RunningServer, ServerConfig};

const HOME_FIRST: &str =
    include_str!("../../curvyo-laser-core/tests/fixtures/output/grbl/home_first.gcode");

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

fn server(link: &Path, state: &Path) -> RunningServer {
    RunningServer::start(&ServerConfig {
        listen: "127.0.0.1:0".parse().unwrap(),
        state_dir: state.to_owned(),
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
    .unwrap()
}

fn pending(status: &Message) -> bool {
    matches!(
        status,
        Message::Status {
            pause_pending: true,
            ..
        }
    )
}

/// Sends the job, starts it, waits until GRBL homes, then pauses: the answer is pending.
fn pause_while_homing(c: &mut Connection, pty: &PtySim) {
    let job = c
        .send_job("home_first.gcode", HOME_FIRST, Instant::now() + WAIT)
        .unwrap();
    wait_state(c, State::Ready);
    c.send(&Message::Start { job_id: job.job_id }).unwrap();
    wait_for(c, |s| {
        state_of(s) == State::Running
            && matches!(s, Message::Status { detail: Some(d), .. } if d == "Controller state: Home")
    });
    assert_eq!(
        pty.sim.lock().unwrap().lines.last().map(String::as_str),
        Some("$H")
    );
    c.send(&Message::Pause).unwrap();
    let answer = wait_for(c, pending);
    assert_eq!(state_of(&answer), State::Running);
}

#[test]
fn ph2_ph9_pause_during_homing_over_a_real_pty() {
    let dir = tempfile::tempdir().unwrap();
    let link = dir.path().join("ttyGRBL");
    let state = dir.path().join("state");
    let mut pty = PtySim::plug(
        GrblSim::new(SimConfig {
            version: "1.1f",
            homing_lock: true,
            homing_steps: 600,
            quiet_homing: true,
            ..SimConfig::default()
        }),
        &link,
    );
    let server1 = server(&link, &state);
    let address = server1.addr().to_string();
    let record = connect(&address, PinMode::Capture, CONNECT_TIMEOUT)
        .unwrap()
        .pair(
            &server1.current_code().unwrap(),
            "Laptop",
            Instant::now() + WAIT,
        )
        .unwrap();
    let (mut c, first) =
        open_paired(&address, record.spki, &record.token, CONNECT_TIMEOUT).unwrap();
    if state_of(&first) != State::Alarm {
        wait_state(&mut c, State::Alarm);
    }
    c.send(&Message::Unlock).unwrap();
    wait_state(&mut c, State::Idle);

    // PH2: pending, then `Paused` when homing ends, no line after `$H`; Resume runs it on.
    pause_while_homing(&mut c, &pty);
    let paused = wait_for(&mut c, |s| state_of(s) != State::Running);
    assert_eq!(state_of(&paused), State::Paused);
    assert!(!pending(&paused));
    {
        let sim = pty.sim.lock().unwrap();
        assert_eq!(sim.lines.last().map(String::as_str), Some("$H"));
        assert!(!sim.laser_lit());
        assert!(sim.held());
    }
    c.send(&Message::Stop).unwrap();
    wait_for(&mut c, |s| matches!(state_of(s), State::Done { .. }));

    // PH9: an unplugged port while pending: the pending pause goes with the job.
    pause_while_homing(&mut c, &pty);
    pty.unplug();
    let lost = wait_state(&mut c, State::NoController);
    assert!(!pending(&lost));
    pty.replug();
    let back = wait_for(&mut c, |s| state_of(s) != State::NoController);
    assert!(!pending(&back));
    assert_ne!(state_of(&back), State::Paused);
    if state_of(&back) == State::Alarm {
        c.send(&Message::Unlock).unwrap();
        wait_state(&mut c, State::Idle);
    } else {
        assert_eq!(state_of(&back), State::Idle);
    }

    // PH9: the connector restarted while pending: no pending pause, no job.
    pause_while_homing(&mut c, &pty);
    drop(c);
    server1.shutdown();
    let server2 = server(&link, &state);
    let address = server2.addr().to_string();
    let (mut c, first) =
        open_paired(&address, record.spki, &record.token, CONNECT_TIMEOUT).unwrap();
    assert!(!pending(&first));
    let settled = if state_of(&first) == State::NoController {
        wait_for(&mut c, |s| state_of(s) != State::NoController)
    } else {
        first
    };
    assert!(!pending(&settled));
    let Message::Status { job, .. } = &settled else {
        panic!()
    };
    assert!(job.is_none(), "the job is gone");
    assert!(pty.sim.lock().unwrap().faults.is_empty());
    server2.shutdown();
}
