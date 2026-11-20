//! A3, A4 (`specs/0128`) inside the crate: in maintenance mode nothing reaches a controller, and
//! a peer that is not local is closed after accept (injected peer addresses on the loopback).

use std::net::{IpAddr, TcpListener};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use curvyo_connector_core::{
    ControllerCommand, ControllerInfo, ErrorCode, Event, Job, JobKind, Message, Role,
};

use crate::client::{CONNECT_TIMEOUT, connect};
use crate::controller::{Advance, Controller};
use crate::hub::{ControllerChoice, Hub, HubMode, HubSettings};
use crate::maintenance::MaintenanceWindow;
use crate::pin_verifier::PinMode;
use crate::server_tls::server_config;

/// A controller that records every command and every byte read, and writes nothing.
struct Recording {
    commands: Arc<Mutex<Vec<ControllerCommand>>>,
}

impl Controller for Recording {
    fn info(&self) -> Option<ControllerInfo> {
        None
    }

    fn command(&mut self, command: &ControllerCommand, _: Option<&Job>, _: Duration) -> Vec<Event> {
        self.commands.lock().unwrap().push(command.clone());
        Vec::new()
    }

    fn advance(&mut self, _: &[u8], _: Duration) -> Advance {
        Advance::default()
    }
}

/// A hub in maintenance mode with the recording controller, served on the loopback; every
/// connection is served as if it came from `peer`.
fn serve_as(
    peer: IpAddr,
) -> (
    String,
    Arc<Mutex<Vec<ControllerCommand>>>,
    tempfile::TempDir,
) {
    let dir = tempfile::tempdir().unwrap();
    let identity = crate::identity::load_or_create(dir.path()).unwrap();
    let tls = server_config(&identity).unwrap();
    let (hub, _notices) = Hub::new(
        HubSettings {
            state_dir: dir.path().to_owned(),
            controller: ControllerChoice::None,
            require_local_start: false,
            connector_name: "workshop-pi".into(),
            spki: identity.spki,
        },
        Instant::now(),
    );
    let commands = Arc::new(Mutex::new(Vec::new()));
    hub.set_test_controller(Box::new(Recording {
        commands: Arc::clone(&commands),
    }));
    hub.set_mode(HubMode::Maintenance(MaintenanceWindow {
        opened: Instant::now(),
        length: Duration::from_secs(1800),
    }));
    let hub = Arc::new(hub);
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap().to_string();
    std::thread::spawn(move || {
        for stream in listener.incoming().map_while(Result::ok) {
            let (tls, hub) = (Arc::clone(&tls), Arc::clone(&hub));
            std::thread::spawn(move || {
                crate::session::serve(stream, peer, &tls, hub, &AtomicBool::new(false));
            });
        }
    });
    (address, commands, dir)
}

#[test]
fn a3_in_maintenance_mode_no_message_reaches_the_controller() {
    let (address, commands, _dir) = serve_as("127.0.0.1".parse().unwrap());
    let mut c = connect(&address, PinMode::Capture, CONNECT_TIMEOUT).unwrap();
    let job_id = uuid::Uuid::nil();
    let text = "G0 X1\n";
    let messages = [
        Message::SubmitJob {
            job_id,
            name: "x.gcode".into(),
            kind: JobKind::Job,
            bytes: text.len() as u64,
            sha256: crate::hex::encode(
                ring::digest::digest(&ring::digest::SHA256, text.as_bytes()).as_ref(),
            ),
        },
        Message::JobChunk {
            job_id,
            index: 0,
            text: text.into(),
        },
        Message::Start { job_id },
        Message::Pause,
        Message::Resume,
        Message::Stop,
        Message::Unlock,
        Message::Home,
        Message::IssueCode {
            client_name: "x".into(),
            role: Role::Owner,
        },
    ];
    let deadline = Instant::now() + Duration::from_secs(10);
    for message in messages {
        let answer = c.request(&message, deadline).unwrap();
        assert!(
            matches!(
                answer,
                Message::Error {
                    code: ErrorCode::Maintenance,
                    ..
                }
            ),
            "{message:?}: {answer:?}"
        );
    }
    assert_eq!(*commands.lock().unwrap(), Vec::new());
}

#[test]
fn a4_a_public_peer_is_closed_after_accept_and_a_local_one_is_served() {
    for public in ["8.8.8.8", "2001:db8::1", "::ffff:8.8.8.8"] {
        let (address, _, _dir) = serve_as(public.parse().unwrap());
        assert!(
            connect(&address, PinMode::Capture, CONNECT_TIMEOUT).is_err(),
            "{public}"
        );
    }
    for local in ["192.168.1.20", "fd00::1", "100.64.1.2"] {
        let (address, _, _dir) = serve_as(local.parse().unwrap());
        assert!(
            connect(&address, PinMode::Capture, CONNECT_TIMEOUT).is_ok(),
            "{local}"
        );
    }
}

#[test]
fn a5_the_window_closes_by_the_injected_clock_and_nothing_reopens_it() {
    let dir = tempfile::tempdir().unwrap();
    let identity = crate::identity::load_or_create(dir.path()).unwrap();
    let (hub, _notices) = Hub::new(
        HubSettings {
            state_dir: dir.path().to_owned(),
            controller: ControllerChoice::None,
            require_local_start: false,
            connector_name: "workshop-pi".into(),
            spki: identity.spki,
        },
        Instant::now(),
    );
    let opened = Instant::now();
    hub.set_mode(HubMode::Maintenance(MaintenanceWindow {
        opened,
        length: Duration::from_secs(1800),
    }));
    let config = curvyo_connector_core::ConnectorConfig {
        format: 1,
        name: "workshop-laser".into(),
        owner: None,
        listen: None,
        controller: curvyo_connector_core::ControllerConfig::Fake {
            lines_per_second: 20,
            alarm_at_line: None,
        },
    };
    let peer: IpAddr = "192.168.1.30".parse().unwrap();
    let late = opened + Duration::from_secs(1800);
    let refused = hub.apply_config(config.clone(), "ctl", None, peer, late);
    assert_eq!(refused.unwrap_err().code, ErrorCode::MaintenanceClosed);
    assert_eq!(
        hub.access_mode(late),
        curvyo_connector_core::Mode::Maintenance { window_open: false }
    );
    // Still closed later, and a Welcome says 0.
    assert!(
        hub.apply_config(
            config.clone(),
            "ctl",
            None,
            peer,
            late + Duration::from_secs(60)
        )
        .is_err()
    );
    let in_time = opened + Duration::from_secs(1799);
    assert!(
        hub.apply_config(config, "ctl", None, peer, in_time)
            .unwrap()
            .is_some()
    );
}
