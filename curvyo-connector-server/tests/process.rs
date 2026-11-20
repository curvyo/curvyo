//! The connector program as a process: its output, its flags and its exit codes.
#![allow(clippy::unwrap_used)]

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{Receiver, channel};
use std::time::{Duration, Instant};

use curvyo_connector_core::{JobResult, Message, State};
use curvyo_connector_io::client::{CONNECT_TIMEOUT, connect};
use curvyo_connector_io::pin_verifier::PinMode;

const BIN: &str = env!("CARGO_BIN_EXE_curvyo-connector-server");
const WAIT: Duration = Duration::from_secs(20);
const JOB: &str = "G21\nG90\nM4 S0\nG1 X10 F1000 S500\nM5\n";

struct Running {
    child: Child,
    lines: Receiver<String>,
    address: String,
    code: String,
    fingerprint: String,
}

impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn start(dir: &std::path::Path, extra: &[&str]) -> Running {
    let mut child = Command::new(BIN)
        .args([
            "--fake-controller",
            "--listen",
            "127.0.0.1:0",
            "--state-dir",
        ])
        .arg(dir)
        .args(extra)
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
    let mut running = Running {
        child,
        lines,
        address: String::new(),
        code: String::new(),
        fingerprint: String::new(),
    };
    let mut seen = Vec::new();
    while running.code.is_empty() {
        let line = running.lines.recv_timeout(WAIT).unwrap();
        if let Some(a) = line.strip_prefix("Listening on ") {
            a.clone_into(&mut running.address);
        }
        if let Some(f) = line.strip_prefix("Key fingerprint: ") {
            f.clone_into(&mut running.fingerprint);
        }
        if let Some(c) = line.strip_prefix("Pairing code: ") {
            c.split(',').next().unwrap().clone_into(&mut running.code);
        }
        seen.push(line);
    }
    assert!(seen[0].starts_with("Curvyo connector "), "{seen:?}");
    assert_eq!(
        seen[1],
        "Fake controller: no machine is attached, nothing moves."
    );
    running
}

fn run(dir: &std::path::Path, args: &[&str]) -> (i32, String, String) {
    let output = Command::new(BIN)
        .args(args)
        .arg("--state-dir")
        .arg(dir)
        .output()
        .unwrap();
    (
        output.status.code().unwrap(),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

#[test]
fn q1_it_prints_address_fingerprint_and_code_and_listens() {
    let dir = tempfile::tempdir().unwrap();
    let running = start(dir.path(), &[]);
    assert!(running.address.starts_with("127.0.0.1:"));
    let groups: Vec<_> = running.fingerprint.split(' ').collect();
    assert_eq!(groups.len(), 8);
    assert!(
        groups
            .iter()
            .all(|g| g.len() == 4 && g.bytes().all(|b| b.is_ascii_hexdigit()))
    );
    let code = &running.code;
    assert_eq!(code.len(), 9);
    assert_eq!(&code[4..5], "-");
    let connection = connect(&running.address, PinMode::Capture, CONNECT_TIMEOUT).unwrap();
    assert_eq!(connection.spki.fingerprint(), running.fingerprint);
}

#[test]
fn q1_help_lists_every_flag_and_a_bad_line_exits_with_2() {
    let dir = tempfile::tempdir().unwrap();
    let (code, out, _) = run(dir.path(), &["--help"]);
    assert_eq!(code, 0);
    for flag in [
        "--listen",
        "--state-dir",
        "--fake-controller",
        "--fake-lines-per-second",
        "--fake-alarm-at-line",
        "--require-local-start",
        "--pair",
        "--list-clients",
        "--revoke",
    ] {
        assert!(out.contains(flag), "{flag}");
    }
    let (code, _, err) = run(dir.path(), &["--fake-alarm-at-line", "3"]);
    assert_eq!(code, 2);
    assert!(err.contains("--fake-controller"), "{err}");
    assert_eq!(run(dir.path(), &["--bogus"]).0, 2);
}

#[test]
fn q3_a_damaged_key_file_exits_with_1_and_names_the_file() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("identity.pem"), "damaged").unwrap();
    let (code, _, err) = run(dir.path(), &["--fake-controller"]);
    assert_eq!(code, 1);
    assert!(err.contains("identity.pem"), "{err}");
    assert!(!err.contains("panicked"), "{err}");
}

#[test]
fn q9_list_clients_and_revoke_work_on_the_state_directory() {
    let dir = tempfile::tempdir().unwrap();
    let running = start(dir.path(), &[]);
    let mut connection = connect(&running.address, PinMode::Capture, CONNECT_TIMEOUT).unwrap();
    let record = connection
        .pair(&running.code, "Studio laptop", Instant::now() + WAIT)
        .unwrap();
    let (code, out, _) = run(dir.path(), &["--list-clients"]);
    assert_eq!(code, 0);
    assert!(out.starts_with("Studio laptop\t20"), "{out}");
    assert!(!out.contains(record.token.to_hex().as_str()));
    let (code, _, err) = run(dir.path(), &["--revoke", "Nobody"]);
    assert_eq!(code, 1);
    assert!(err.contains("Nobody"), "{err}");
    assert_eq!(run(dir.path(), &["--revoke", "Studio laptop"]).0, 0);
    assert_eq!(run(dir.path(), &["--list-clients"]).1, "");
}

#[test]
fn q17_q18_the_enter_key_starts_a_waiting_job_and_the_log_gets_a_line() {
    let dir = tempfile::tempdir().unwrap();
    let mut running = start(
        dir.path(),
        &["--require-local-start", "--fake-lines-per-second", "5000"],
    );
    let mut connection = connect(&running.address, PinMode::Capture, CONNECT_TIMEOUT).unwrap();
    let record = connection
        .pair(&running.code, "Laptop", Instant::now() + WAIT)
        .unwrap();
    drop(connection);
    let (mut connection, _) = curvyo_connector_io::client::open_paired(
        &running.address,
        record.spki,
        &record.token,
        CONNECT_TIMEOUT,
    )
    .unwrap();
    let job = connection
        .send_job("sign.gcode", JOB, Instant::now() + WAIT)
        .unwrap();
    connection
        .send(&Message::Start { job_id: job.job_id })
        .unwrap();
    loop {
        let line = running.lines.recv_timeout(WAIT).unwrap();
        if line == "Press Enter to start sign.gcode" {
            break;
        }
    }
    running
        .child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(b"\n")
        .unwrap();
    let deadline = Instant::now() + WAIT;
    loop {
        if let Message::Status {
            state: State::Done {
                result: JobResult::Completed,
            },
            ..
        } = connection.recv_message(deadline).unwrap()
        {
            break;
        }
    }
    let log = std::fs::read_to_string(dir.path().join("jobs.log")).unwrap();
    let fields: Vec<&str> = log.trim_end_matches('\n').split('\t').collect();
    assert_eq!(
        &fields[1..],
        ["sign.gcode", "Completed", "Laptop", "Laptop", ""]
    );
    assert!(
        fields[0].ends_with('Z') && fields[0].len() == 20,
        "{}",
        fields[0]
    );
}

#[test]
fn q19_the_alarm_flag_raises_an_alarm_at_the_line() {
    let dir = tempfile::tempdir().unwrap();
    let running = start(
        dir.path(),
        &[
            "--fake-alarm-at-line",
            "2",
            "--fake-lines-per-second",
            "5000",
        ],
    );
    let mut connection = connect(&running.address, PinMode::Capture, CONNECT_TIMEOUT).unwrap();
    let record = connection
        .pair(&running.code, "Laptop", Instant::now() + WAIT)
        .unwrap();
    drop(connection);
    let (mut connection, _) = curvyo_connector_io::client::open_paired(
        &running.address,
        record.spki,
        &record.token,
        CONNECT_TIMEOUT,
    )
    .unwrap();
    let job = connection
        .send_job("etch.gcode", JOB, Instant::now() + WAIT)
        .unwrap();
    connection
        .send(&Message::Start { job_id: job.job_id })
        .unwrap();
    let deadline = Instant::now() + WAIT;
    loop {
        if let Message::Status {
            state: State::Alarm,
            alarm,
            ..
        } = connection.recv_message(deadline).unwrap()
        {
            assert_eq!(alarm.as_deref(), Some("Fake alarm at line 2"));
            break;
        }
    }
    let log = std::fs::read_to_string(dir.path().join("jobs.log")).unwrap();
    assert!(
        log.lines()
            .last()
            .is_some_and(|l| l.contains("\tetch.gcode\tAlarm: Fake alarm at line 2\t")),
        "{log}"
    );
}

#[test]
fn q2_the_pair_line_prints_a_new_code() {
    let dir = tempfile::tempdir().unwrap();
    let mut running = start(dir.path(), &[]);
    running
        .child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(b"pair\n")
        .unwrap();
    let line = loop {
        let line = running.lines.recv_timeout(WAIT).unwrap();
        if line.starts_with("Pairing code: ") {
            break line;
        }
    };
    assert_ne!(
        line.split(',').next().unwrap(),
        format!("Pairing code: {}", running.code)
    );
}
