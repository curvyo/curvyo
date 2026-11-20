//! `specs/0127` at the program's edge: `--serial` and `--baud` (G1), a port that is not there
//! (G2, G3), and SIGINT/SIGTERM stopping a running job before the exit (G16). Waiting is by
//! output line with a long deadline, never by sleep.
#![cfg(unix)]
#![allow(clippy::unwrap_used)]

use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{Receiver, channel};
use std::time::{Duration, Instant};

use curvyo_connector_core::{Message, State};
use curvyo_connector_io::client::{CONNECT_TIMEOUT, connect, open_paired};
use curvyo_connector_io::pin_verifier::PinMode;
use nix::sys::signal::{Signal, kill};
use nix::unistd::Pid;

const BIN: &str = env!("CARGO_BIN_EXE_curvyo-connector-server");
const WAIT: Duration = Duration::from_secs(20);

struct Running {
    child: Child,
    lines: Receiver<String>,
    seen: Vec<String>,
}

impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Running {
    fn start(dir: &std::path::Path, args: &[&str]) -> Self {
        let mut child = Command::new(BIN)
            .args(["--listen", "127.0.0.1:0", "--state-dir"])
            .arg(dir)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
        let out = child.stdout.take().unwrap();
        let (sender, lines) = channel();
        std::thread::spawn(move || {
            for line in BufReader::new(out).lines().map_while(Result::ok) {
                let _ = sender.send(line);
            }
        });
        Self {
            child,
            lines,
            seen: Vec::new(),
        }
    }

    /// Reads lines until one satisfies `done`; returns it.
    fn until(&mut self, done: impl Fn(&str) -> bool) -> String {
        loop {
            let line = self.lines.recv_timeout(WAIT).unwrap_or_else(|_| {
                panic!("no such line; seen: {:?}", self.seen);
            });
            self.seen.push(line.clone());
            if done(&line) {
                return line;
            }
        }
    }

    fn value(&mut self, prefix: &str) -> String {
        let line = self.until(|l| l.starts_with(prefix));
        line[prefix.len()..].to_owned()
    }

    fn signal(&self, signal: Signal) {
        let pid = i32::try_from(self.child.id()).unwrap();
        kill(Pid::from_raw(pid), signal).unwrap();
    }

    fn exit_code(&mut self) -> i32 {
        let deadline = Instant::now() + WAIT;
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                return status.code().unwrap();
            }
            assert!(Instant::now() < deadline, "did not exit");
            std::thread::yield_now();
        }
    }
}

#[test]
fn g1_usage_errors_exit_with_2_and_name_the_flags() {
    let dir = tempfile::tempdir().unwrap();
    for (args, text) in [
        (
            &["--serial", "/dev/ttyUSB0", "--fake-controller"][..],
            "--serial and --fake-controller cannot be used together",
        ),
        (&["--baud", "9600"], "--baud needs --serial"),
        (
            &["--serial", "/dev/ttyUSB0", "--baud", "0"],
            "--baud must be 1 to 4000000",
        ),
        // `specs/0142` HC2: refused before the port is opened (the port does not exist).
        (
            &["--serial", "/dev/ttyUSB0", "--homing-timeout", "0"],
            "--homing-timeout must be 10 to 3600 seconds, got 0",
        ),
        (
            &["--serial", "/dev/ttyUSB0", "--homing-timeout", "3601"],
            "--homing-timeout must be 10 to 3600 seconds, got 3601",
        ),
        (
            &["--serial", "/dev/ttyUSB0", "--homing-timeout", "abc"],
            "--homing-timeout must be 10 to 3600 seconds, got abc",
        ),
        (
            &["--homing-timeout", "60"],
            "--homing-timeout needs --serial",
        ),
    ] {
        let out = Command::new(BIN)
            .args(args)
            .arg("--state-dir")
            .arg(dir.path())
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(2), "{args:?}");
        assert!(
            String::from_utf8_lossy(&out.stderr).contains(text),
            "{args:?}"
        );
    }
}

#[test]
fn g2_g3_a_missing_port_is_named_retried_and_the_program_keeps_running() {
    let dir = tempfile::tempdir().unwrap();
    let port = dir.path().join("ttyNOPE");
    let port = port.to_str().unwrap().to_owned();
    let mut running = Running::start(dir.path(), &["--serial", &port]);
    let address = running.value("Listening on ");
    let code = running.value("Pairing code: ");
    let code = code.split(',').next().unwrap().to_owned();
    assert_eq!(
        running.until(|l| l.starts_with("Serial port ")),
        format!("Serial port {port}, 115200 baud")
    );
    running.until(|l| l == format!("Cannot open {port}: not found"));
    running.until(|l| l == format!("Trying {port} again every 2 s"));
    let record = connect(&address, PinMode::Capture, CONNECT_TIMEOUT)
        .unwrap()
        .pair(&code, "Laptop", Instant::now() + WAIT)
        .unwrap();
    let (_, status) = open_paired(&address, record.spki, &record.token, CONNECT_TIMEOUT).unwrap();
    let Message::Status { state, detail, .. } = status else {
        panic!("{status:?}")
    };
    assert_eq!(state, State::NoController);
    assert_eq!(detail, Some(format!("Port {port} not found")));
    assert!(
        running.child.try_wait().unwrap().is_none(),
        "never exits for a missing port"
    );
    running.signal(Signal::SIGTERM);
    running.until(|l| l == "Exiting");
    assert_eq!(running.exit_code(), 0);
}

#[test]
fn g16_sigterm_stops_a_running_job_and_logs_it_before_exiting() {
    let dir = tempfile::tempdir().unwrap();
    let mut running = Running::start(
        dir.path(),
        &["--fake-controller", "--fake-lines-per-second", "1"],
    );
    let address = running.value("Listening on ");
    let code = running.value("Pairing code: ");
    let code = code.split(',').next().unwrap().to_owned();
    let record = connect(&address, PinMode::Capture, CONNECT_TIMEOUT)
        .unwrap()
        .pair(&code, "Laptop", Instant::now() + WAIT)
        .unwrap();
    let (mut connection, _) =
        open_paired(&address, record.spki, &record.token, CONNECT_TIMEOUT).unwrap();
    let job = connection
        .send_job("long.gcode", &"G1 X1\n".repeat(1000), Instant::now() + WAIT)
        .unwrap();
    connection
        .send(&Message::Start { job_id: job.job_id })
        .unwrap();
    loop {
        let message = connection.recv_message(Instant::now() + WAIT).unwrap();
        if matches!(
            message,
            Message::Status {
                state: State::Running,
                ..
            }
        ) {
            break;
        }
    }
    running.signal(Signal::SIGTERM);
    running.until(|l| l == "Stopping long.gcode");
    running.until(|l| l == "Job long.gcode: Stopped");
    running.until(|l| l == "Exiting");
    assert_eq!(running.exit_code(), 0);
    let log = std::fs::read_to_string(dir.path().join("jobs.log")).unwrap();
    assert!(log.contains("\tlong.gcode\tStopped\n"), "{log}");
}

#[test]
fn g16_sigint_without_a_job_exits_at_once() {
    let dir = tempfile::tempdir().unwrap();
    let mut running = Running::start(dir.path(), &["--fake-controller"]);
    running.value("Pairing code: ");
    running.signal(Signal::SIGINT);
    running.until(|l| l == "Exiting");
    assert_eq!(running.exit_code(), 0);
    assert!(!running.seen.iter().any(|l| l.starts_with("Stopping")));
}
