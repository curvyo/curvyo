//! A connector on a free local port with a state directory of its own, and helpers to pair,
//! connect and wait for a status, and the GRBL simulator with its in-memory bench. Waiting is by
//! condition with a long deadline, never by sleep.
#![allow(dead_code, clippy::unwrap_used, clippy::expect_used)]

pub mod grbl_bench;
pub mod grbl_sim;
pub mod pty_link;

use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use curvyo_connector_core::{Message, State};
use curvyo_connector_io::client::{CONNECT_TIMEOUT, Connection, connect, open_paired};
use curvyo_connector_io::hub::{ControllerChoice, FakeSettings};
use curvyo_connector_io::pin_verifier::PinMode;
use curvyo_connector_io::record::PairingRecord;
use curvyo_connector_io::server::{RunningServer, ServerConfig};

pub const WAIT: Duration = Duration::from_secs(20);

pub struct Rig {
    pub server: RunningServer,
    pub dir: tempfile::TempDir,
}

pub fn config(dir: PathBuf, lines_per_second: u32, alarm_at_line: Option<u32>) -> ServerConfig {
    ServerConfig {
        listen: "127.0.0.1:0".parse().unwrap(),
        state_dir: dir,
        controller: ControllerChoice::Fake(FakeSettings {
            lines_per_second,
            alarm_at_line,
        }),
        require_local_start: false,
        connector_name: "Test connector".to_owned(),
        issue_code: false,
    }
}

pub fn rig(lines_per_second: u32) -> Rig {
    rig_with(lines_per_second, None, false)
}

pub fn rig_with(lines_per_second: u32, alarm_at_line: Option<u32>, local_start: bool) -> Rig {
    let dir = tempfile::tempdir().unwrap();
    let mut config = config(dir.path().to_owned(), lines_per_second, alarm_at_line);
    config.require_local_start = local_start;
    Rig {
        server: RunningServer::start(&config).unwrap(),
        dir,
    }
}

impl Rig {
    pub fn address(&self) -> String {
        self.server.addr().to_string()
    }

    pub fn addr(&self) -> SocketAddr {
        self.server.addr()
    }

    pub fn code(&self) -> String {
        self.server.current_code().unwrap()
    }

    /// Pairs a new app with the current code.
    pub fn pair(&self, name: &str) -> PairingRecord {
        let mut connection = connect(&self.address(), PinMode::Capture, CONNECT_TIMEOUT).unwrap();
        connection
            .pair(&self.code(), name, Instant::now() + WAIT)
            .unwrap()
    }

    pub fn open(&self, record: &PairingRecord) -> (Connection, Message) {
        open_paired(&self.address(), record.spki, &record.token, CONNECT_TIMEOUT).unwrap()
    }

    pub fn log(&self) -> String {
        std::fs::read_to_string(self.dir.path().join("jobs.log")).unwrap_or_default()
    }
}

pub fn state_of(status: &Message) -> State {
    match status {
        Message::Status { state, .. } => *state,
        other => panic!("not a status: {other:?}"),
    }
}

pub fn line_of(status: &Message) -> u32 {
    match status {
        Message::Status { job: Some(job), .. } => job.line,
        _ => 0,
    }
}

/// Reads until a status satisfies `done`; panics after [`WAIT`].
pub fn wait_for(connection: &mut Connection, done: impl Fn(&Message) -> bool) -> Message {
    let deadline = Instant::now() + WAIT;
    loop {
        let message = connection.recv_message(deadline).unwrap();
        if matches!(message, Message::Status { .. }) && done(&message) {
            return message;
        }
    }
}

pub fn wait_state(connection: &mut Connection, state: State) -> Message {
    wait_for(connection, |s| state_of(s) == state)
}

pub const JOB: &str = "G21\nG90\nG0 X0 Y0\nM4 S0\nG1 X10 Y10 F1000 S500\nM5\n";
