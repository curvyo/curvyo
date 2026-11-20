//! `specs/0142` over a real pty: `RunningServer` with the serial controller, a 1 s homing cap and
//! the simulator's silent homing. A job's `$H` that never answers ends as a lost controller with
//! the cap's sentence in the status and in `jobs.log`; the port coming back identifies afresh;
//! a port pulled during homing is the loss of `0127` G15. Waiting is by status, never by sleep.
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
const CAP_TEXT: &str = "Lost the controller: homing took longer than 1 s";
/// Reset, then `M5`, then `M9`.
const SAFETY: &[u8] = b"\x18M5\nM9\n";

fn timing() -> Timing {
    Timing {
        retry: Duration::from_millis(100),
        identify: Duration::from_secs(2),
        loss: Duration::from_secs(1),
        hold: Duration::from_secs(1),
        banner: Duration::from_secs(1),
        poll_fast: Duration::from_millis(50),
        poll_slow: Duration::from_millis(200),
        homing: Duration::from_secs(1),
    }
}

fn server(link: &Path, state: &Path) -> RunningServer {
    RunningServer::start(&ServerConfig {
        listen: "127.0.0.1:0".parse().unwrap(),
        state_dir: state.to_owned(),
        controller: ControllerChoice::Serial(SerialSettings {
            port: link.to_str().unwrap().to_owned(),
            baud: 115_200,
            timing: timing(),
        }),
        require_local_start: false,
        connector_name: "Test connector".to_owned(),
        issue_code: false,
    })
    .unwrap()
}

fn detail_of(status: &Message) -> Option<&str> {
    match status {
        Message::Status { detail, .. } => detail.as_deref(),
        _ => None,
    }
}

/// From `Alarm` (the power-up lock) or `Idle` to `Idle`.
fn to_idle(c: &mut Connection, status: &Message) {
    if state_of(status) == State::Alarm {
        c.send(&Message::Unlock).unwrap();
    }
    if state_of(status) != State::Idle {
        wait_state(c, State::Idle);
    }
}

/// Sends and starts the job, waits until GRBL homes.
fn start_homing(c: &mut Connection) {
    let job = c
        .send_job("home_first.gcode", HOME_FIRST, Instant::now() + WAIT)
        .unwrap();
    wait_state(c, State::Ready);
    c.send(&Message::Start { job_id: job.job_id }).unwrap();
    wait_for(c, |s| {
        state_of(s) == State::Running && detail_of(s) == Some("Controller state: Home")
    });
}

#[test]
fn hc3_hc7_a_silent_homing_over_a_real_pty_ends_at_the_cap_and_reconnects() {
    let dir = tempfile::tempdir().unwrap();
    let link = dir.path().join("ttyGRBL");
    let state = dir.path().join("state");
    let mut pty = PtySim::plug(
        GrblSim::new(SimConfig {
            version: "1.1f",
            homing_lock: true,
            homing_steps: 10_000_000,
            quiet_homing: true,
            ..SimConfig::default()
        }),
        &link,
    );
    let running = server(&link, &state);
    let address = running.addr().to_string();
    let record = connect(&address, PinMode::Capture, CONNECT_TIMEOUT)
        .unwrap()
        .pair(
            &running.current_code().unwrap(),
            "Laptop",
            Instant::now() + WAIT,
        )
        .unwrap();
    let (mut c, first) =
        open_paired(&address, record.spki, &record.token, CONNECT_TIMEOUT).unwrap();
    let first = if state_of(&first) == State::NoController {
        wait_for(&mut c, |s| state_of(s) != State::NoController)
    } else {
        first
    };
    to_idle(&mut c, &first);

    // HC3: the cap ends the job as a lost controller, its sentence in the status and the log.
    start_homing(&mut c);
    let mark = pty.sim.lock().unwrap().log.len();
    let lost = wait_for(&mut c, |s| state_of(s) != State::Running);
    assert_eq!(state_of(&lost), State::NoController);
    assert_eq!(detail_of(&lost), Some(CAP_TEXT));
    // The status goes out before the serial thread writes: wait for the bytes to arrive.
    let deadline = Instant::now() + WAIT;
    loop {
        {
            let sim = pty.sim.lock().unwrap();
            let after = &sim.log[mark..];
            if let Some(reset) = after.iter().position(|b| *b == 0x18)
                && after.len() >= reset + SAFETY.len()
            {
                assert_eq!(&after[reset..reset + SAFETY.len()], SAFETY, "{after:?}");
                assert!(!sim.laser_lit());
                assert!(!sim.air);
                break;
            }
        }
        assert!(Instant::now() < deadline, "the safety bytes never arrived");
        std::thread::yield_now();
    }
    let log = std::fs::read_to_string(state.join("jobs.log")).unwrap();
    assert!(
        log.lines().any(|l| l.contains(
            "\thome_first.gcode\tFailed: controller lost, homing took longer than 1 s\t"
        )),
        "{log}"
    );

    // HC7: the port stays: the connector retries, identifies afresh, the job is gone.
    let back = wait_for(&mut c, |s| state_of(s) != State::NoController);
    let Message::Status { job, .. } = &back else {
        panic!()
    };
    assert!(job.is_none(), "the job is not resumed");
    to_idle(&mut c, &back);

    // A pulled port during homing is the loss of G15, at once, never the cap's sentence.
    start_homing(&mut c);
    pty.unplug();
    let pulled = wait_state(&mut c, State::NoController);
    assert_ne!(detail_of(&pulled), Some(CAP_TEXT));
    assert!(pty.sim.lock().unwrap().faults.is_empty());
    drop(c);
    running.shutdown();
}
