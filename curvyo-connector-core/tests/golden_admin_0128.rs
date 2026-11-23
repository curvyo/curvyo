//! A2, A8 to A11, A16, A17, A19, A32 (`specs/0128`): the goldens of every new message, error code
//! and configuration, and the configurations that break a rule.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::{fixture, job_id};
use curvyo_connector_core::{
    ClientEntry, ConnectorConfig, ConnectorMode, ControllerConfig, ErrorCode, LogName, Message,
    Progress, Role, Secret, State, decode, encode, refusal,
};

fn grbl() -> ConnectorConfig {
    ConnectorConfig {
        format: 1,
        name: "workshop-laser".into(),
        owner: Some("Marc's workshop".into()),
        listen: Some("0.0.0.0:7447".into()),
        controller: ControllerConfig::GrblSerial {
            port: "/dev/serial/by-id/usb-1a86_USB_Serial-if00-port0".into(),
            baud: 115_200,
        },
    }
}

fn fake() -> ConnectorConfig {
    ConnectorConfig {
        format: 1,
        name: "workshop-laser".into(),
        owner: None,
        listen: None,
        controller: ControllerConfig::Fake {
            lines_per_second: 20,
            alarm_at_line: Some(3),
        },
    }
}

/// One value per new golden file under `tests/fixtures/messages/`.
#[allow(clippy::too_many_lines)] // one flat table, a split would only hide a row
fn all() -> Vec<(&'static str, Message)> {
    vec![
        ("list_ports", Message::ListPorts),
        (
            "apply_config",
            Message::ApplyConfig {
                config: fake(),
                client_name: "curvyoctl@marc-laptop".into(),
            },
        ),
        ("get_config", Message::GetConfig),
        (
            "issue_code",
            Message::IssueCode {
                client_name: "Phone".into(),
                role: Role::Member,
            },
        ),
        ("list_clients", Message::ListClients),
        (
            "revoke_client",
            Message::RevokeClient {
                name: "Phone".into(),
            },
        ),
        (
            "get_log",
            Message::GetLog {
                log: LogName::Admin,
                last: 20,
            },
        ),
        (
            "reset",
            Message::Reset {
                connector_name: "workshop-laser".into(),
            },
        ),
        (
            "welcome_maintenance",
            Message::Welcome {
                connector_name: "workshop-pi".into(),
                controller: None,
                mode: Some(ConnectorMode::Maintenance { closes_in_s: 1680 }),
            },
        ),
        (
            "welcome_configured",
            Message::Welcome {
                connector_name: "workshop-laser".into(),
                controller: None,
                mode: Some(ConnectorMode::Configured {
                    owner: Some("Marc's workshop".into()),
                }),
            },
        ),
        (
            "welcome_foreground",
            Message::Welcome {
                connector_name: "workshop-pi".into(),
                controller: None,
                mode: Some(ConnectorMode::Foreground),
            },
        ),
        (
            "authenticated",
            Message::Authenticated {
                client_name: "Phone".into(),
                role: Role::Member,
            },
        ),
        (
            "ports",
            Message::Ports {
                paths: vec![
                    "/dev/serial/by-id/usb-1a86_USB2.0-Serial-if00-port0".into(),
                    "/dev/ttyUSB0".into(),
                ],
            },
        ),
        (
            "config_applied",
            Message::ConfigApplied {
                token: Some(Secret::new("ab".repeat(32))),
            },
        ),
        (
            "config_applied_no_token",
            Message::ConfigApplied { token: None },
        ),
        ("config", Message::Config { config: grbl() }),
        (
            "code_issued",
            Message::CodeIssued {
                code: Secret::new("K7QM-4XTR"),
                expires_in_s: 600,
            },
        ),
        (
            "clients",
            Message::Clients {
                clients: vec![
                    ClientEntry {
                        name: "Laptop".into(),
                        role: Role::Owner,
                        paired_at: "2026-11-18T14:09:02Z".into(),
                    },
                    ClientEntry {
                        name: "Phone".into(),
                        role: Role::Member,
                        paired_at: "2026-11-18T14:13:05Z".into(),
                    },
                ],
            },
        ),
        (
            "client_revoked",
            Message::ClientRevoked {
                name: "Phone".into(),
            },
        ),
        (
            "log_lines",
            Message::LogLines {
                lines: vec!["2026-11-18T14:20:01Z\tunlocked\tLaptop\t\t".into()],
            },
        ),
        ("reset_done", Message::ResetDone),
        (
            "status_awaiting_confirmation",
            Message::Status {
                state: State::AwaitingLocalStart,
                job: Some(Progress {
                    job_id: job_id(),
                    line: 0,
                    lines: 6,
                    elapsed_ms: 0,
                }),
                position: None,
                alarm: None,
                detail: None,
                pause_pending: false,
                requested_by: Some("Phone".into()),
                job_name: Some("sign.gcode".into()),
                sent_by: Some("Phone".into()),
                homed: false,
                machine_position: None,
                work_offset: None,
            },
        ),
    ]
}

#[test]
fn every_new_message_matches_its_golden_and_decodes_back() {
    for (name, message) in all() {
        let golden = fixture(&format!("messages/{name}.json"));
        assert_eq!(encode(&message).unwrap(), golden, "{name}");
        assert_eq!(decode(&golden).unwrap(), message, "{name}");
    }
}

#[test]
fn the_new_codes_have_goldens() {
    let cases = [
        ("maintenance", refusal(ErrorCode::Maintenance)),
        ("maintenance_closed", refusal(ErrorCode::MaintenanceClosed)),
        ("owner_required", refusal(ErrorCode::OwnerRequired)),
        (
            "invalid_config",
            Message::Error {
                code: ErrorCode::InvalidConfig,
                message: "controller.baud: 0 is not from 1 to 4,000,000".into(),
            },
        ),
        (
            "unknown_name",
            Message::Error {
                code: ErrorCode::UnknownName,
                message: "No client is called Phon.".into(),
            },
        ),
    ];
    for (name, message) in cases {
        assert_eq!(
            encode(&message).unwrap(),
            fixture(&format!("errors/{name}.json")),
            "{name}"
        );
    }
}

#[test]
fn a_welcome_and_a_status_of_an_older_connector_decode_without_the_new_fields() {
    match decode(&fixture("messages/welcome.json")).unwrap() {
        Message::Welcome { mode, .. } => assert_eq!(mode, None),
        other => panic!("{other:?}"),
    }
    match decode(&fixture("messages/status_ready.json")).unwrap() {
        Message::Status { requested_by, .. } => assert_eq!(requested_by, None),
        other => panic!("{other:?}"),
    }
}

#[test]
fn the_secrets_never_show_in_debug() {
    for (_, message) in all() {
        let text = format!("{message:?}");
        assert!(!text.contains("K7QM-4XTR"), "{text}");
        assert!(!text.contains(&"ab".repeat(32)), "{text}");
    }
}

#[test]
fn the_configurations_match_their_goldens() {
    for (name, config) in [("connector_grbl", grbl()), ("connector_fake", fake())] {
        let golden = fixture(&format!("config/{name}.json"));
        assert_eq!(serde_json::to_string(&config).unwrap(), golden, "{name}");
        let back: ConnectorConfig = serde_json::from_str(&golden).unwrap();
        assert_eq!(back, config);
        assert_eq!(back.validate(), Ok(()));
    }
}

/// `config_failures.txt`: one configuration per line, then ` -> ` and the field and rule it
/// breaks, or `refused` when serde refuses it (an unknown or missing field).
#[test]
fn the_configurations_that_break_a_rule() {
    let golden = fixture("config_failures.txt");
    let mut lines = 0;
    for line in golden.lines() {
        let (json, expected) = line.split_once(" -> ").unwrap();
        let got = match serde_json::from_str::<ConnectorConfig>(json) {
            Err(_) => "refused".to_owned(),
            Ok(config) => config
                .validate()
                .map_or_else(|e| e.to_string(), |()| "valid".to_owned()),
        };
        assert_eq!(got, expected, "{json}");
        lines += 1;
    }
    assert!(lines >= 15, "{lines}");
}
