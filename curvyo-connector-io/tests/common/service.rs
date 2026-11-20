//! A connector run as a service in a temporary state directory (`specs/0128`): set up over the
//! wire with `ApplyConfig`, with owners and members, and its terminal lines.
#![allow(
    dead_code,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::unused_self
)]

use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use curvyo_connector_core::{ConnectorConfig, ControllerConfig, Message, Role, Secret};
use curvyo_connector_io::client::{CONNECT_TIMEOUT, Connection, connect, open_paired};
use curvyo_connector_io::pin_verifier::PinMode;
use curvyo_connector_io::server::{RunningServer, ServerTiming, ServiceConfig};
use curvyo_connector_io::token::Token;

use super::WAIT;

/// The name the tests set up as.
pub const CTL: &str = "curvyoctl@test";

pub struct Service {
    pub server: RunningServer,
    pub dir: tempfile::TempDir,
    pub notices: Receiver<String>,
}

pub fn fake_config(lines_per_second: u32) -> ConnectorConfig {
    ConnectorConfig {
        format: 1,
        name: "workshop-laser".into(),
        owner: Some("Marc".into()),
        listen: None,
        controller: ControllerConfig::Fake {
            lines_per_second,
            alarm_at_line: None,
        },
    }
}

pub fn service_config(dir: &std::path::Path, window: Duration) -> ServiceConfig {
    ServiceConfig {
        listen: "127.0.0.1:0".parse().unwrap(),
        state_dir: dir.to_owned(),
        host_name: "workshop-pi".into(),
        timing: ServerTiming {
            maintenance_window: window,
            ..ServerTiming::default()
        },
    }
}

/// A fresh connector in maintenance mode with a 30-minute window.
pub fn service() -> Service {
    service_in(tempfile::tempdir().unwrap(), Duration::from_secs(1800))
}

pub fn service_in(dir: tempfile::TempDir, window: Duration) -> Service {
    let server = RunningServer::start_service(&service_config(dir.path(), window)).unwrap();
    let notices = server.take_notices().unwrap();
    Service {
        server,
        dir,
        notices,
    }
}

pub fn deadline() -> Instant {
    Instant::now() + WAIT
}

pub fn code_of(answer: &Message) -> Option<curvyo_connector_core::ErrorCode> {
    match answer {
        Message::Error { code, .. } => Some(*code),
        _ => None,
    }
}

impl Service {
    pub fn address(&self) -> String {
        self.server.addr().to_string()
    }

    /// A connection without a login, trusting the key it is shown.
    pub fn insecure(&self) -> Connection {
        connect(&self.address(), PinMode::Capture, CONNECT_TIMEOUT).unwrap()
    }

    pub fn apply(&self, connection: &mut Connection, config: ConnectorConfig) -> Message {
        connection
            .request(
                &Message::ApplyConfig {
                    config,
                    client_name: CTL.into(),
                },
                deadline(),
            )
            .unwrap()
    }

    /// Sets the connector up with the fake controller; the owner's connection and token.
    pub fn setup(&self, lines_per_second: u32) -> (Connection, Token) {
        let mut connection = self.insecure();
        let answer = self.apply(&mut connection, fake_config(lines_per_second));
        let Message::ConfigApplied { token: Some(token) } = answer else {
            panic!("{answer:?}");
        };
        (connection, Token::from_hex(token.expose()).unwrap())
    }

    /// A logged-in connection with `token`; its first status read.
    pub fn login(&self, token: &Token) -> Connection {
        let spki = self.server.spki();
        open_paired(&self.address(), spki, token, CONNECT_TIMEOUT)
            .unwrap()
            .0
    }

    /// A code for `name` with `role`, issued by the owner's connection.
    pub fn issue(&self, owner: &mut Connection, name: &str, role: Role) -> Secret {
        let answer = owner
            .request(
                &Message::IssueCode {
                    client_name: name.into(),
                    role,
                },
                deadline(),
            )
            .unwrap();
        let Message::CodeIssued { code, .. } = answer else {
            panic!("{answer:?}");
        };
        code
    }

    /// Pairs a new app with a code issued for `name` and `role`; its token and connection.
    pub fn pair(&self, owner: &mut Connection, name: &str, role: Role) -> (Token, Connection) {
        let code = self.issue(owner, name, role);
        let mut app = self.insecure();
        let record = app
            .pair(code.expose(), "app's own name", deadline())
            .unwrap();
        assert_eq!(app.session, Some((name.to_owned(), role)));
        (record.token, app)
    }

    /// Reads terminal lines until one satisfies `want`; every line read on the way.
    pub fn notice_until(&self, want: impl Fn(&str) -> bool) -> Vec<String> {
        let until = deadline();
        let mut seen = Vec::new();
        while Instant::now() < until {
            if let Ok(line) = self.notices.recv_timeout(Duration::from_millis(50)) {
                let done = want(&line);
                seen.push(line);
                if done {
                    return seen;
                }
            }
        }
        panic!("no such terminal line in {seen:?}");
    }

    pub fn file(&self, name: &str) -> String {
        std::fs::read_to_string(self.dir.path().join(name)).unwrap_or_default()
    }
}
