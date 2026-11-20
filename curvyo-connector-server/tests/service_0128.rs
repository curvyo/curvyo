//! A1, A19, A21, A28, A31 (`specs/0128`): the program as a service in its state directory, its
//! journal lines, the host-side `--reset` and `--fingerprint`, and the exit after a reset.
#![allow(clippy::unwrap_used)]

use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{Receiver, channel};
use std::time::{Duration, Instant};

use curvyo_connector_core::{ConnectorConfig, ControllerConfig, Message};
use curvyo_connector_io::client::{CONNECT_TIMEOUT, connect};
use curvyo_connector_io::pin_verifier::PinMode;

const BIN: &str = env!("CARGO_BIN_EXE_curvyo-connector-server");
const WAIT: Duration = Duration::from_secs(20);

struct Running {
    child: Child,
    lines: Receiver<String>,
    address: String,
    fingerprint: String,
    seen: Vec<String>,
}

impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Running {
    fn line_until(&mut self, want: impl Fn(&str) -> bool) -> String {
        let until = Instant::now() + WAIT;
        while Instant::now() < until {
            if let Ok(line) = self.lines.recv_timeout(Duration::from_millis(50)) {
                self.seen.push(line.clone());
                if want(&line) {
                    return line;
                }
            }
        }
        panic!("no such line in {:?}", self.seen);
    }
}

fn start(dir: &std::path::Path) -> Running {
    let mut child = Command::new(BIN)
        .args(["--listen", "127.0.0.1:0", "--state-dir"])
        .arg(dir)
        .stdin(Stdio::null())
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
        fingerprint: String::new(),
        seen: Vec::new(),
    };
    let listening = running.line_until(|l| l.starts_with("Listening on "));
    listening["Listening on ".len()..].clone_into(&mut running.address);
    let key = running.line_until(|l| l.starts_with("Key fingerprint: "));
    key["Key fingerprint: ".len()..].clone_into(&mut running.fingerprint);
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

fn fake() -> ConnectorConfig {
    ConnectorConfig {
        format: 1,
        name: "workshop-laser".into(),
        owner: Some("Marc".into()),
        listen: None,
        controller: ControllerConfig::Fake {
            lines_per_second: 20,
            alarm_at_line: None,
        },
    }
}

#[test]
fn a1_a21_maintenance_lines_fingerprint_and_reset_refused_while_running() {
    let dir = tempfile::tempdir().unwrap();
    // --fingerprint on an empty directory creates nothing.
    let (code, _, err) = run(dir.path(), &["--fingerprint"]);
    assert_eq!(code, 1, "{err}");
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
    let running = start(dir.path());
    let maintenance = running
        .seen
        .iter()
        .find(|l| l.starts_with("Maintenance mode until "))
        .unwrap();
    assert!(
        maintenance.ends_with(" UTC: waiting for curvyoctl apply-config --insecure"),
        "{maintenance}"
    );
    assert_eq!(running.fingerprint.split(' ').count(), 8);
    let (code, out, _) = run(dir.path(), &["--fingerprint"]);
    assert_eq!(code, 0);
    assert_eq!(out, format!("Key fingerprint: {}\n", running.fingerprint));
    let (code, _, err) = run(dir.path(), &["--reset"]);
    assert_eq!(code, 1);
    assert!(err.contains("a running connector holds"), "{err}");
    assert!(err.contains("lock"), "{err}");
    drop(running);
    let (code, out, _) = run(dir.path(), &["--reset"]);
    assert_eq!(code, 0);
    assert_eq!(
        out,
        format!("Deleted the contents of {}\n", dir.path().display())
    );
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
}

#[test]
fn a19_a28_a_reset_over_the_network_ends_the_process_with_0_and_empties_the_directory() {
    let dir = tempfile::tempdir().unwrap();
    let mut running = start(dir.path());
    let mut c = connect(&running.address, PinMode::Capture, CONNECT_TIMEOUT).unwrap();
    let deadline = Instant::now() + WAIT;
    let answer = c
        .request(
            &Message::ApplyConfig {
                config: fake(),
                client_name: "curvyoctl@test".into(),
            },
            deadline,
        )
        .unwrap();
    assert!(matches!(answer, Message::ConfigApplied { token: Some(_) }));
    running.line_until(|l| l == "Configured by curvyoctl@test from 127.0.0.1");
    // A run flag is refused while the configuration exists.
    let (code, _, err) = run(dir.path(), &["--fake-controller"]);
    assert_eq!(code, 2);
    assert!(
        err.contains("--fake-controller cannot be used: ")
            && err.ends_with("connector.toml exists\n"),
        "{err}"
    );
    let answer = c
        .request(
            &Message::Reset {
                connector_name: "workshop-laser".into(),
            },
            deadline,
        )
        .unwrap();
    assert_eq!(answer, Message::ResetDone);
    running.line_until(|l| l == "Reset by curvyoctl@test: state directory deleted");
    let status = running.child.wait().unwrap();
    assert_eq!(status.code(), Some(0));
    assert!(dir.path().exists());
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
}

#[test]
fn a31_a_configured_start_names_the_connector_and_the_ignored_listen_flag() {
    let dir = tempfile::tempdir().unwrap();
    let free = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = free.local_addr().unwrap().port();
    drop(free);
    let mut config = fake();
    config.listen = Some(format!("127.0.0.1:{port}"));
    curvyo_connector_io::config_file::store(dir.path(), &config).unwrap();
    let running = start(dir.path());
    assert_eq!(running.address, format!("127.0.0.1:{port}"));
    for line in [
        "Configured: workshop-laser, belongs to Marc".to_owned(),
        "Controller: Fake controller: nothing moves".to_owned(),
        format!("--listen 127.0.0.1:0 ignored: connector.toml sets 127.0.0.1:{port}"),
    ] {
        assert!(running.seen.contains(&line), "{line} in {:?}", running.seen);
    }
    drop(running);
}

#[test]
fn a30_the_unit_file_has_the_hardening_of_the_spec() {
    let unit = include_str!("../dist/curvyo-connector.service");
    for line in [
        "ExecStart=/usr/bin/curvyo-connector-server --state-dir %S/curvyo-connector",
        "DynamicUser=yes",
        "StateDirectory=curvyo-connector",
        "StateDirectoryMode=0700",
        "UMask=0077",
        "SupplementaryGroups=dialout",
        "Restart=on-failure",
        "TimeoutStopSec=15",
        "KillSignal=SIGTERM",
        "NoNewPrivileges=yes",
        "ProtectSystem=strict",
        "ProtectHome=yes",
        "PrivateTmp=yes",
        "DevicePolicy=closed",
        "DeviceAllow=char-ttyUSB rw",
        "DeviceAllow=char-ttyACM rw",
        "WantedBy=multi-user.target",
    ] {
        assert!(unit.lines().any(|l| l == line), "{line}");
    }
    // No run flag of foreground mode in the service.
    for flag in [
        "--serial",
        "--fake-controller",
        "--require-local-start",
        "--pair",
    ] {
        assert!(!unit.contains(flag), "{flag}");
    }
}
