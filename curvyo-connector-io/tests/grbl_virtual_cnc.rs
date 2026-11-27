//! `specs/0161-cnc-output` criteria 13, 15 and 19: the CNC goldens pass the connector's line
//! check, and `cnc_grbl_two_parts.nc` runs through the real grbl-serial controller on a
//! pseudo-terminal against the virtual GRBL to `Done { Completed }`; without `--spindle` the
//! laser guard still refuses it (`$32=0`).
#![cfg(target_os = "linux")]
#![allow(clippy::unwrap_used)]

mod common;

use std::io::{Read as _, Write as _};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use common::{WAIT, state_of, wait_for, wait_state};
use curvyo_connector_core::virtual_grbl::{GrblSettings, VirtualGrbl};
use curvyo_connector_core::{JobResult, State};
use curvyo_connector_io::client::{CONNECT_TIMEOUT, connect, open_paired};
use curvyo_connector_io::grbl::Timing;
use curvyo_connector_io::grbl::prepare::prepare;
use curvyo_connector_io::hub::{ControllerChoice, SerialSettings};
use curvyo_connector_io::pin_verifier::PinMode;
use curvyo_connector_io::server::{RunningServer, ServerConfig};
use serialport::{SerialPort as _, TTYPort};

const GOLDENS: [&str; 3] = [
    include_str!("../../curvyo-cnc-core/tests/fixtures/cnc_grbl_profile_outside.nc"),
    include_str!("../../curvyo-cnc-core/tests/fixtures/cnc_grbl_profile_inside.nc"),
    include_str!("../../curvyo-cnc-core/tests/fixtures/cnc_grbl_two_parts.nc"),
];

#[test]
fn c13_every_golden_line_passes_the_connector_s_line_check() {
    for golden in GOLDENS {
        assert!(prepare(golden).is_ok());
        assert!(golden.lines().all(|l| l.len() <= 79));
    }
}

/// The virtual GRBL behind a pty reached through `link`, its clock fast.
struct VirtualPty {
    grbl: Arc<Mutex<VirtualGrbl>>,
    stop: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl VirtualPty {
    fn plug(link: &std::path::Path) -> Self {
        let mut settings = GrblSettings::default();
        for (n, v) in [(30, "12000"), (130, "300"), (131, "300"), (132, "60")] {
            settings.set(n, v).unwrap();
        }
        let grbl = Arc::new(Mutex::new(VirtualGrbl::new(settings)));
        let (mut master, slave) = TTYPort::pair().unwrap();
        std::os::unix::fs::symlink(slave.name().unwrap(), link).unwrap();
        drop(slave);
        let stop = Arc::new(AtomicBool::new(false));
        let (g, s) = (Arc::clone(&grbl), Arc::clone(&stop));
        let thread = std::thread::spawn(move || {
            master.set_timeout(Duration::from_millis(5)).unwrap();
            let mut buf = [0_u8; 512];
            while !s.load(Ordering::Relaxed) {
                let n = master.read(&mut buf).unwrap_or(0);
                let out = {
                    let mut g = g.lock().unwrap();
                    g.receive(&buf[..n]);
                    g.advance(0.05)
                };
                if !out.is_empty() {
                    let _ = master.write_all(&out);
                }
            }
        });
        Self {
            grbl,
            stop,
            thread: Some(thread),
        }
    }
}

impl Drop for VirtualPty {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(t) = self.thread.take() {
            t.join().unwrap();
        }
    }
}

fn server(link: &std::path::Path, dir: &std::path::Path, spindle: bool) -> RunningServer {
    RunningServer::start(&ServerConfig {
        listen: "127.0.0.1:0".parse().unwrap(),
        state_dir: dir.join("state"),
        controller: ControllerChoice::Serial(SerialSettings {
            port: link.to_str().unwrap().to_owned(),
            baud: 115_200,
            spindle,
            timing: Timing {
                retry: Duration::from_millis(100),
                identify: Duration::from_secs(2),
                poll_fast: Duration::from_millis(50),
                poll_slow: Duration::from_millis(200),
                ..Timing::default()
            },
        }),
        require_local_start: false,
        connector_name: "Test connector".to_owned(),
        issue_code: false,
    })
    .unwrap()
}

fn run_job(spindle: bool) -> (State, Arc<Mutex<VirtualGrbl>>) {
    let dir = tempfile::tempdir().unwrap();
    let link = dir.path().join("ttyVGRBL");
    let pty = VirtualPty::plug(&link);
    let server = server(&link, dir.path(), spindle);
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
    if state_of(&first) != State::Idle {
        wait_state(&mut c, State::Idle);
    }
    let sent = c.send_job("cnc_grbl_two_parts.nc", GOLDENS[2], Instant::now() + WAIT);
    let end = match sent {
        Err(_) => State::Idle,
        Ok(job) => {
            wait_state(&mut c, State::Ready);
            c.send(&curvyo_connector_core::Message::Start { job_id: job.job_id })
                .unwrap();
            let seen = wait_for(&mut c, |s| matches!(state_of(s), State::Done { .. }));
            state_of(&seen)
        }
    };
    server.shutdown();
    let grbl = Arc::clone(&pty.grbl);
    drop(pty);
    (end, grbl)
}

#[test]
fn c15_c19_the_two_parts_job_completes_on_the_virtual_grbl_through_the_connector() {
    let (end, grbl) = run_job(true);
    assert_eq!(
        end,
        State::Done {
            result: JobResult::Completed
        }
    );
    let g = grbl.lock().unwrap();
    assert!(g.faults.is_empty(), "{:?}", g.faults);
    assert!(
        g.lines.iter().any(|l| l == "G21G90G17G94"),
        "the job's lines arrived"
    );
    assert!((g.position()[2] - 5.0).abs() < 1e-3, "{:?}", g.position());
}

#[test]
fn without_spindle_mode_the_laser_guard_still_refuses_laser_mode_off() {
    let (end, _) = run_job(false);
    assert_ne!(
        end,
        State::Done {
            result: JobResult::Completed
        }
    );
}
