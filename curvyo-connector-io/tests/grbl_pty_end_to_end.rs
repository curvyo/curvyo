//! `specs/0127` G23: `RunningServer` with the serial controller on the simulator's pty, driven by
//! a paired client through test, send, start, pause, resume, stop, a second send to `Completed`,
//! an alarm with Unlock, an unplugged port and the reconnect. Waiting is by status, never by
//! sleep; the connector's limits are shortened.
#![cfg(target_os = "linux")]
#![allow(clippy::unwrap_used)]

mod common;

use std::time::{Duration, Instant};

use common::grbl_sim::{GrblSim, SimConfig, Spindle};
use common::pty_link::PtySim;
use common::{WAIT, line_of, state_of, wait_for, wait_state};
use curvyo_connector_core::{JobResult, Message, State};
use curvyo_connector_io::client::{CONNECT_TIMEOUT, connect, open_paired};
use curvyo_connector_io::grbl::Timing;
use curvyo_connector_io::hub::{ControllerChoice, SerialSettings};
use curvyo_connector_io::pin_verifier::PinMode;
use curvyo_connector_io::server::{RunningServer, ServerConfig};

const CUT_LINE: &str =
    include_str!("../../curvyo-laser-core/tests/fixtures/output/grbl/cut_line.gcode");

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

fn detail_of(status: &Message) -> Option<String> {
    match status {
        Message::Status { detail, .. } => detail.clone(),
        _ => None,
    }
}

fn laser_off(pty: &PtySim) {
    let sim = pty.sim.lock().unwrap();
    assert!(!sim.laser_lit(), "laser off");
    assert_eq!(sim.spindle, Spindle::Off);
    assert!(!sim.air);
}

#[test]
#[allow(clippy::too_many_lines)] // one walk through the demo, step by step
fn g23_the_whole_demo_over_a_real_pty() {
    let dir = tempfile::tempdir().unwrap();
    let link = dir.path().join("ttyGRBL");
    let mut pty = PtySim::plug(
        GrblSim::new(SimConfig {
            steps_per_motion: 400,
            decel_steps: 10,
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
            timing: timing(),
        }),
        require_local_start: false,
        connector_name: "Test connector".to_owned(),
        issue_code: false,
    })
    .unwrap();
    if let Some(notices) = server.take_notices() {
        std::thread::spawn(move || notices.iter().for_each(|n| eprintln!("connector: {n}")));
    }
    let address = server.addr().to_string();
    let record = connect(&address, PinMode::Capture, CONNECT_TIMEOUT)
        .unwrap()
        .pair(
            &server.current_code().unwrap(),
            "Laptop",
            Instant::now() + WAIT,
        )
        .unwrap();
    let open = || open_paired(&address, record.spki, &record.token, CONNECT_TIMEOUT).unwrap();

    // Test: identified, the Controller line's values in the Welcome.
    let (mut c, first) = open();
    if state_of(&first) != State::Idle {
        wait_state(&mut c, State::Idle);
    }
    let (c2, _) = open();
    let info = c2.greeting.controller.clone().unwrap();
    assert_eq!(
        (info.name.as_str(), info.firmware.as_str()),
        ("GRBL", "1.1h")
    );
    assert_eq!(info.port.as_deref(), link.to_str());
    assert_eq!(info.baud, Some(115_200));
    drop(c2);

    // Send, start, pause, resume, stop.
    let job = c
        .send_job("cut_line.gcode", CUT_LINE, Instant::now() + WAIT)
        .unwrap();
    wait_state(&mut c, State::Ready);
    c.send(&Message::Start { job_id: job.job_id }).unwrap();
    wait_for(&mut c, |s| {
        state_of(s) == State::Running && line_of(s) >= 10
    });
    c.send(&Message::Pause).unwrap();
    wait_state(&mut c, State::Paused);
    assert!(!pty.sim.lock().unwrap().laser_lit(), "no beam while held");
    c.send(&Message::Resume).unwrap();
    wait_state(&mut c, State::Running);
    c.send(&Message::Stop).unwrap();
    let stopped = State::Done {
        result: JobResult::Stopped,
    };
    let seen = wait_for(&mut c, |s| state_of(s) != State::Running);
    if state_of(&seen) != stopped {
        wait_state(&mut c, stopped);
    }
    laser_off(&pty);

    // A second send runs to Completed.
    let job = c
        .send_job("cut_line.gcode", CUT_LINE, Instant::now() + WAIT)
        .unwrap();
    c.send(&Message::Start { job_id: job.job_id }).unwrap();
    let completed = State::Done {
        result: JobResult::Completed,
    };
    wait_state(&mut c, completed);
    assert!(
        !pty.sim.lock().unwrap().planner_busy(),
        "Completed only once the head is still"
    );

    // An injected alarm during a job, then Unlock from the app.
    let job = c
        .send_job("cut_line.gcode", CUT_LINE, Instant::now() + WAIT)
        .unwrap();
    c.send(&Message::Start { job_id: job.job_id }).unwrap();
    wait_for(&mut c, |s| {
        state_of(s) == State::Running && line_of(s) >= 10
    });
    pty.sim.lock().unwrap().inject_alarm(1);
    let alarm = wait_state(&mut c, State::Alarm);
    let Message::Status { alarm, .. } = alarm else {
        panic!()
    };
    assert_eq!(
        alarm.as_deref(),
        Some("Hard limit hit, position may be lost (alarm 1)")
    );
    laser_off(&pty);
    c.send(&Message::Unlock).unwrap();
    wait_state(&mut c, State::Idle);
    // Outside Alarm, Unlock writes nothing and answers the status unchanged (G33).
    let writes = pty.sim.lock().unwrap().lines.len();
    c.send(&Message::Unlock).unwrap();
    let answer = wait_for(&mut c, |_| true);
    assert_eq!(state_of(&answer), State::Idle);

    // An unplugged port mid-job: lost at its line, never resumed after the reconnect.
    let job = c
        .send_job("cut_line.gcode", CUT_LINE, Instant::now() + WAIT)
        .unwrap();
    c.send(&Message::Start { job_id: job.job_id }).unwrap();
    let running = wait_for(&mut c, |s| {
        state_of(s) == State::Running && line_of(s) >= 10
    });
    pty.unplug();
    let lost = wait_state(&mut c, State::NoController);
    let lost_text = detail_of(&lost).unwrap();
    assert!(
        lost_text.starts_with("Lost the controller at line "),
        "{lost_text}"
    );
    assert!(line_of(&running) <= 12);
    {
        // GRBL's planner runs out while the cable is off (no software can undo that, G16).
        let mut sim = pty.sim.lock().unwrap();
        while sim.planner_busy() {
            sim.step();
        }
    }
    pty.replug();
    let back = wait_state(&mut c, State::Idle);
    assert_eq!(
        detail_of(&back).as_deref(),
        Some(lost_text.as_str()),
        "the text stays"
    );
    let Message::Status { job, .. } = back else {
        panic!()
    };
    assert!(job.is_none(), "the job is gone");
    let log = std::fs::read_to_string(dir.path().join("state/jobs.log")).unwrap();
    assert!(
        log.contains("\tStopped\t") && log.contains("\tCompleted\t"),
        "{log}"
    );
    assert!(
        log.contains("\tAlarm: Hard limit hit, position may be lost (alarm 1)\t"),
        "{log}"
    );
    assert!(log.contains("\tFailed: controller lost at line "), "{log}");
    assert!(log.contains("\t\tUnlock: Laptop\t"), "{log}");
    let sim = pty.sim.lock().unwrap();
    assert_eq!(sim.lines.iter().filter(|l| *l == "$X").count(), 1, "one $X");
    assert!(sim.lines.len() > writes);
    drop(sim);

    // The byte log: no overflow, no real-time byte inside a line.
    let sim = pty.sim.lock().unwrap();
    assert!(sim.faults.is_empty(), "{:?}", sim.faults);
    assert!(!sim.lines.iter().any(|l| l.contains(['!', '~', '?'])));
    drop(sim);
    server.shutdown();
}
