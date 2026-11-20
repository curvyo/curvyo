//! A1 to A7, A20, A29, A31 (`specs/0128`): maintenance mode, the setup over the wire, its race,
//! and what a reset leaves for the next start.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use std::time::Duration;

use common::service::{
    CTL, Service, code_of, deadline, fake_config, service, service_config, service_in,
};
use common::wait_state;
use curvyo_connector_core::{ConnectorMode, ErrorCode, JobKind, LogName, Message, Role, State};
use curvyo_connector_io::client::{CONNECT_TIMEOUT, ClientError, open_paired};
use curvyo_connector_io::frames::{Frame, IoFrame};
use curvyo_connector_io::hub::HubMode;
use curvyo_connector_io::server::RunningServer;
use curvyo_connector_io::state_dir;

fn mode_of(service: &Service) -> Option<ConnectorMode> {
    service.insecure().greeting.mode
}

#[test]
fn a1_a2_a_fresh_connector_is_in_maintenance_mode_and_answers_three_messages() {
    let s = service();
    assert!(matches!(s.server.mode(), HubMode::Maintenance(_)));
    assert_eq!(s.server.current_code(), None, "no pairing code");
    let mut c = s.insecure();
    assert_eq!(c.greeting.name, "workshop-pi");
    match &c.greeting.mode {
        Some(ConnectorMode::Maintenance { closes_in_s }) => {
            assert!((1790..=1800).contains(closes_in_s), "{closes_in_s}");
        }
        other => panic!("{other:?}"),
    }
    let ports = c.request(&Message::ListPorts, deadline()).unwrap();
    let Message::Ports { paths } = ports else {
        panic!("{ports:?}");
    };
    assert!(paths.iter().all(|p| p.starts_with("/dev/")), "{paths:?}");
    // Nothing was written but the key and the lock.
    let mut names: Vec<_> = std::fs::read_dir(s.dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    assert_eq!(names, ["identity.pem", "lock"]);
}

#[test]
fn a3_every_other_message_answers_maintenance() {
    let s = service();
    let mut c = s.insecure();
    let job_id = uuid::Uuid::nil();
    let refused = [
        Message::GetStatus,
        Message::SubmitJob {
            job_id,
            name: "x.gcode".into(),
            kind: JobKind::Job,
            bytes: 1,
            sha256: "00".repeat(32),
        },
        Message::JobChunk {
            job_id,
            index: 0,
            text: "G0\n".into(),
        },
        Message::Start { job_id },
        Message::Pause,
        Message::Resume,
        Message::Stop,
        Message::Unlock,
        Message::Home,
        Message::Pair {
            code: "K7QM4XTR".into(),
            client_name: "x".into(),
        },
        Message::Auth {
            token: "ab".repeat(32),
        },
        Message::GetConfig,
        Message::IssueCode {
            client_name: "x".into(),
            role: Role::Owner,
        },
        Message::ListClients,
        Message::RevokeClient { name: "x".into() },
        Message::GetLog {
            log: LogName::Jobs,
            last: 5,
        },
        Message::Reset {
            connector_name: "workshop-pi".into(),
        },
    ];
    for message in refused {
        let answer = c.request(&message, deadline()).unwrap();
        assert_eq!(
            code_of(&answer),
            Some(ErrorCode::Maintenance),
            "{message:?}"
        );
    }
    c.send_frame(&Frame::Io(IoFrame::PairSpake {
        round: 1,
        client_name: Some("x".into()),
        message: "00".into(),
    }))
    .unwrap();
    let answer = c.recv(deadline()).unwrap();
    assert!(
        matches!(&answer, Frame::Core(m) if code_of(m) == Some(ErrorCode::Maintenance)),
        "{answer:?}"
    );
}

#[test]
fn a5_after_the_window_apply_config_is_closed_and_hello_and_list_ports_still_work() {
    // A window of zero: closed from the start; no wait is needed.
    let s = service_in(tempfile::tempdir().unwrap(), Duration::ZERO);
    let mut c = s.insecure();
    assert_eq!(
        c.greeting.mode,
        Some(ConnectorMode::Maintenance { closes_in_s: 0 })
    );
    let answer = s.apply(&mut c, fake_config(20));
    assert_eq!(code_of(&answer), Some(ErrorCode::MaintenanceClosed));
    let ports = c.request(&Message::ListPorts, deadline()).unwrap();
    assert!(matches!(ports, Message::Ports { .. }), "{ports:?}");
    assert!(!s.dir.path().join("connector.toml").exists());
    s.notice_until(|l| l.starts_with("Maintenance window closed at "));
}

#[test]
fn a6_a_valid_configuration_makes_the_sender_owner_and_switches_without_a_restart() {
    let s = service();
    let mut other = s.insecure();
    let (mut owner, token) = s.setup(2000);
    // `clients.json` names the owner by hash, `connector.toml` is the configuration.
    let clients = s.file("clients.json");
    assert!(clients.contains("\"role\": \"owner\""), "{clients}");
    assert!(clients.contains(&token.sha256_hex()));
    assert!(!clients.contains(token.to_hex().as_str()));
    assert!(
        s.file("connector.toml")
            .contains("name = \"workshop-laser\"")
    );
    // The applying connection goes on as an owner, without a new login.
    let config = owner.request(&Message::GetConfig, deadline()).unwrap();
    assert_eq!(
        config,
        Message::Config {
            config: fake_config(2000)
        }
    );
    // Every other maintenance connection is closed.
    let until = deadline();
    let closed = loop {
        match other.try_recv(std::time::Instant::now() + Duration::from_millis(100)) {
            Err(_) => break true,
            Ok(_) if std::time::Instant::now() > until => break false,
            Ok(_) => {}
        }
    };
    assert!(closed, "the other maintenance connection stays open");
    // The controller of the configuration is attached and the mode shows.
    let mut app = s.login(&token);
    assert_eq!(app.session, Some((CTL.to_owned(), Role::Owner)));
    assert_eq!(
        app.greeting.mode,
        Some(ConnectorMode::Configured {
            owner: Some("Marc".into())
        })
    );
    app.send(&Message::GetStatus).unwrap();
    wait_state(&mut app, State::Idle);
    let lines = s.notice_until(|l| l.starts_with("Controller: "));
    assert!(
        lines
            .iter()
            .any(|l| l == &format!("Configured by {CTL} from 127.0.0.1"))
    );
    assert!(
        s.file("admin.log")
            .contains("\tconfigured\tcurvyoctl@test\t\t127.0.0.1\n")
    );
}

#[test]
fn a6_an_invalid_configuration_names_the_field_and_leaves_the_window() {
    let s = service();
    let mut c = s.insecure();
    let mut bad = fake_config(20);
    bad.controller = curvyo_connector_core::ControllerConfig::Fake {
        lines_per_second: 0,
        alarm_at_line: None,
    };
    let answer = s.apply(&mut c, bad);
    assert_eq!(
        answer,
        Message::Error {
            code: ErrorCode::InvalidConfig,
            message: "controller.lines_per_second: 0 is not from 1 to 10,000".into()
        }
    );
    assert!(!s.dir.path().join("connector.toml").exists());
    let answer = s.apply(&mut c, fake_config(20));
    assert!(matches!(answer, Message::ConfigApplied { token: Some(_) }));
}

#[test]
fn a7_a_second_setup_is_refused_and_the_terminal_names_who_configured() {
    let s = service();
    let mut late = s.insecure();
    let (_owner, _) = s.setup(20);
    let mut fresh = s.insecure();
    let answer = s.apply(&mut fresh, fake_config(20));
    assert_eq!(
        code_of(&answer),
        Some(ErrorCode::OwnerRequired),
        "{answer:?}"
    );
    s.notice_until(|l| {
        l == format!("Setup from 127.0.0.1 refused: configured by {CTL} from 127.0.0.1")
    });
    // A connection opened in maintenance mode is closed by the switch.
    assert!(late.request(&Message::ListPorts, deadline()).is_err());
}

#[test]
fn a7_of_two_setups_at_once_exactly_one_wins() {
    let s = service();
    let mut a = s.insecure();
    let mut b = s.insecure();
    let config = fake_config(20);
    let first = Message::ApplyConfig {
        config: config.clone(),
        client_name: "a".into(),
    };
    let second = Message::ApplyConfig {
        config,
        client_name: "b".into(),
    };
    a.send(&first).unwrap();
    b.send(&second).unwrap();
    let answer = |c: &mut curvyo_connector_io::client::Connection| loop {
        match c.recv_message(deadline()) {
            Ok(m @ (Message::ConfigApplied { .. } | Message::Error { .. })) => break Some(m),
            Ok(_) => {}
            Err(_) => break None,
        }
    };
    let answers = [answer(&mut a), answer(&mut b)];
    let won = answers
        .iter()
        .filter(|m| matches!(m, Some(Message::ConfigApplied { .. })))
        .count();
    assert_eq!(won, 1, "{answers:?}");
    let owners = s
        .file("clients.json")
        .matches("\"role\": \"owner\"")
        .count();
    assert_eq!(owners, 1);
}

#[test]
fn a20_a31_a_reset_connector_starts_fresh_and_a_configured_one_keeps_its_listen() {
    let dir = tempfile::tempdir().unwrap();
    let s = service_in(dir, Duration::from_secs(1800));
    let old_key = s.server.spki();
    let (mut owner, token) = s.setup(20);
    let answer = owner
        .request(
            &Message::Reset {
                connector_name: "workshop-lasr".into(),
            },
            deadline(),
        )
        .unwrap();
    assert_eq!(code_of(&answer), Some(ErrorCode::UnknownName));
    let answer = owner
        .request(
            &Message::Reset {
                connector_name: "workshop-laser".into(),
            },
            deadline(),
        )
        .unwrap();
    assert_eq!(answer, Message::ResetDone);
    let exits = s.server.take_exits().unwrap();
    assert!(matches!(
        exits.recv_timeout(common::WAIT).unwrap(),
        curvyo_connector_io::hub::ExitReason::Reset { by } if by == CTL
    ));
    let Service { server, dir, .. } = s;
    server.wipe().unwrap();
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
    // The next start: maintenance, a new key, the old token unknown.
    let again = service_in(dir, Duration::from_secs(1800));
    assert_ne!(again.server.spki(), old_key);
    assert!(matches!(
        mode_of(&again),
        Some(ConnectorMode::Maintenance { .. })
    ));
    let refused = open_paired(
        &again.address(),
        again.server.spki(),
        &token,
        CONNECT_TIMEOUT,
    );
    assert_eq!(refused.err(), Some(ClientError::InMaintenance));
    // A configuration's listen wins over the start's.
    let mut config = fake_config(20);
    let free = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = free.local_addr().unwrap().port();
    drop(free);
    config.listen = Some(format!("127.0.0.1:{port}"));
    let mut c = again.insecure();
    assert!(matches!(
        again.apply(&mut c, config),
        Message::ConfigApplied { .. }
    ));
    let Service { server, dir, .. } = again;
    drop(c);
    server.shutdown();
    let restarted =
        RunningServer::start_service(&service_config(dir.path(), Duration::ZERO)).unwrap();
    assert_eq!(restarted.addr().port(), port);
    assert!(matches!(restarted.mode(), HubMode::Configured(_)));
}

#[test]
fn a21_a_second_connector_on_one_state_directory_is_refused() {
    let s = service();
    let again = RunningServer::start_service(&service_config(s.dir.path(), Duration::ZERO));
    let error = again.err().unwrap().to_string();
    assert!(error.contains("a running connector holds"), "{error}");
    assert!(matches!(
        state_dir::reset(s.dir.path()),
        Err(state_dir::StateDirError::Locked(_))
    ));
}

#[test]
fn a8_a_damaged_connector_toml_refuses_the_start_and_names_the_field() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("connector.toml"),
        "format = 1\nname = \"a\"\n[controller]\nkind = \"fake\"\nlines_per_second = 0\n",
    )
    .unwrap();
    let error = RunningServer::start_service(&service_config(dir.path(), Duration::ZERO))
        .err()
        .unwrap()
        .to_string();
    assert!(error.contains("connector.toml"), "{error}");
    assert!(error.contains("controller.lines_per_second"), "{error}");
}
