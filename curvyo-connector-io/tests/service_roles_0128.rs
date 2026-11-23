//! A8 to A16, A18 (`specs/0128`): owners and members on a configured connector, the start rule
//! with the owner's confirmation, the owner calls and the logs.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use std::time::{Duration, Instant};

use common::service::{CTL, Service, code_of, deadline, fake_config, service};
use common::{JOB, wait_for, wait_state};
use curvyo_connector_core::{
    ClientEntry, ControllerConfig, ErrorCode, LogName, Message, Role, State,
};
use curvyo_connector_io::client::Connection;

fn configured() -> (Service, Connection) {
    let s = service();
    let (owner, _) = s.setup(20);
    (s, owner)
}

fn requested_by(status: &Message) -> Option<&str> {
    match status {
        Message::Status { requested_by, .. } => requested_by.as_deref(),
        _ => None,
    }
}

#[test]
fn a8_login_and_pairing_say_who_the_client_is_and_clients_json_is_format_2() {
    let (s, mut owner) = configured();
    let (token, _phone) = s.pair(&mut owner, "Phone", Role::Member);
    let phone = s.login(&token);
    assert_eq!(phone.session, Some(("Phone".to_owned(), Role::Member)));
    let clients = s.file("clients.json");
    assert!(clients.contains("\"format\": 2"), "{clients}");
    assert!(clients.contains("\"name\": \"Phone\",\n      \"role\": \"member\""));
}

#[test]
fn a9_a_member_gets_owner_required_for_every_owner_call_and_nothing_changes() {
    let (s, mut owner) = configured();
    let (_, mut phone) = s.pair(&mut owner, "Phone", Role::Member);
    let calls = [
        Message::GetConfig,
        Message::IssueCode {
            client_name: "Mallory".into(),
            role: Role::Owner,
        },
        Message::ListClients,
        Message::RevokeClient { name: CTL.into() },
        Message::GetLog {
            log: LogName::Admin,
            last: 5,
        },
        Message::Reset {
            connector_name: "workshop-laser".into(),
        },
        Message::ListPorts,
        Message::ApplyConfig {
            config: fake_config(5),
            client_name: "Phone".into(),
        },
        // Amended in `specs/0118` slice 2a: a member's `Home` waits for an owner (criterion 37),
        // a member's `Jog` is an owner's call.
        Message::Jog {
            axis: curvyo_connector_core::JogAxis::X,
            direction: curvyo_connector_core::JogDirection::Plus,
            step: curvyo_connector_core::JogStep::One,
            hold: false,
        },
        Message::JogKeep,
    ];
    let before = (s.file("clients.json"), s.file("connector.toml"));
    for call in calls {
        let answer = phone.request(&call, deadline()).unwrap();
        assert_eq!(code_of(&answer), Some(ErrorCode::OwnerRequired), "{call:?}");
    }
    assert_eq!(before, (s.file("clients.json"), s.file("connector.toml")));
}

#[test]
fn a10_a_code_for_a_name_and_role_is_recorded_without_the_code() {
    let (s, mut owner) = configured();
    let code = s.issue(&mut owner, "Laptop", Role::Owner);
    let mut app = s.insecure();
    app.pair(code.expose(), "whatever", deadline()).unwrap();
    assert_eq!(app.session, Some(("Laptop".to_owned(), Role::Owner)));
    let admin = s.file("admin.log");
    assert!(
        admin.contains("\tcode-issued\tcurvyoctl@test\tLaptop\towner\n"),
        "{admin}"
    );
    assert!(admin.contains("\tpaired\tLaptop\t\towner\n"), "{admin}");
    assert!(!admin.contains(code.expose()));
    let seen = s.notice_until(|l| l.starts_with("Controller: "));
    let shown = code.expose().replace('-', "");
    for line in s.notices.try_iter().chain(seen) {
        assert!(
            !line.contains(code.expose()) && !line.contains(&shown),
            "{line}"
        );
    }
}

#[test]
fn a11_list_revoke_last_owner_and_unknown_name() {
    let (s, mut owner) = configured();
    let (_, mut phone) = s.pair(&mut owner, "Phone", Role::Member);
    let answer = owner.request(&Message::ListClients, deadline()).unwrap();
    let Message::Clients { clients } = answer else {
        panic!("{answer:?}")
    };
    let names: Vec<(&str, Role)> = clients
        .iter()
        .map(|c: &ClientEntry| (c.name.as_str(), c.role))
        .collect();
    assert_eq!(names, [(CTL, Role::Owner), ("Phone", Role::Member)]);
    let last = owner
        .request(&Message::RevokeClient { name: CTL.into() }, deadline())
        .unwrap();
    assert_eq!(code_of(&last), Some(ErrorCode::WrongState));
    // A member code for the only owner's name would demote the last owner.
    let demote = owner
        .request(
            &Message::IssueCode {
                client_name: CTL.into(),
                role: Role::Member,
            },
            deadline(),
        )
        .unwrap();
    assert_eq!(code_of(&demote), Some(ErrorCode::WrongState));
    let nobody = owner
        .request(
            &Message::RevokeClient {
                name: "Phon".into(),
            },
            deadline(),
        )
        .unwrap();
    assert_eq!(code_of(&nobody), Some(ErrorCode::UnknownName));
    let done = owner
        .request(
            &Message::RevokeClient {
                name: "Phone".into(),
            },
            deadline(),
        )
        .unwrap();
    assert_eq!(
        done,
        Message::ClientRevoked {
            name: "Phone".into()
        }
    );
    // The revoked client's connection is closed at once.
    let until = Instant::now() + common::WAIT;
    let closed = loop {
        match phone.try_recv(Instant::now() + Duration::from_millis(100)) {
            Err(_) => break true,
            Ok(_) if Instant::now() > until => break false,
            Ok(_) => {}
        }
    };
    assert!(closed);
    assert!(
        s.file("admin.log")
            .contains("\trevoked\tcurvyoctl@test\tPhone\tmember\n")
    );
}

#[test]
fn a12_to_a15_a_members_start_waits_for_the_owners_confirmation() {
    let (s, mut owner) = configured();
    let (_, mut phone) = s.pair(&mut owner, "Phone", Role::Member);
    let job = phone.send_job("sign.gcode", JOB, deadline()).unwrap();
    phone.send(&Message::Start { job_id: job.job_id }).unwrap();
    let waiting = wait_state(&mut phone, State::AwaitingLocalStart);
    assert_eq!(requested_by(&waiting), Some("Phone"));
    s.notice_until(|l| l == "Start of sign.gcode requested by Phone");
    // A second request meanwhile does not fit; Pause is no start.
    let again = phone.request(&Message::Start { job_id: job.job_id }, deadline());
    assert_eq!(code_of(&again.unwrap()), Some(ErrorCode::WrongState));
    // Another id is refused; the owner's Start of the job runs it.
    let other = owner
        .request(
            &Message::Start {
                job_id: uuid::Uuid::nil(),
            },
            deadline(),
        )
        .unwrap();
    assert_eq!(code_of(&other), Some(ErrorCode::UnknownJob));
    owner.send(&Message::Start { job_id: job.job_id }).unwrap();
    wait_for(&mut phone, |m| {
        matches!(
            m,
            Message::Status {
                state: State::Done { .. },
                ..
            }
        )
    });
    s.notice_until(|l| l == "Started sign.gcode, confirmed by curvyoctl@test for Phone");
    let admin = s.file("admin.log");
    let expected = format!(
        "\tstart-confirmed\t{CTL}\tPhone\tsign.gcode {}\n",
        job.job_id
    );
    assert!(admin.contains(&expected), "{admin}");
    let jobs = s.file("jobs.log");
    assert!(
        jobs.contains("\tsign.gcode\tCompleted\tPhone\tPhone\tcurvyoctl@test\n"),
        "{jobs}"
    );
}

#[test]
fn a14_a_members_stop_withdraws_its_waiting_start() {
    let (s, mut owner) = configured();
    let (_, mut phone) = s.pair(&mut owner, "Phone", Role::Member);
    let job = phone.send_job("cut.gcode", JOB, deadline()).unwrap();
    phone.send(&Message::Start { job_id: job.job_id }).unwrap();
    wait_state(&mut phone, State::AwaitingLocalStart);
    phone.send(&Message::Stop).unwrap();
    wait_state(&mut phone, State::Idle);
    let jobs = s.file("jobs.log");
    assert!(
        jobs.contains("\tcut.gcode\tDiscarded\tPhone\t\t\n"),
        "{jobs}"
    );
}

#[test]
fn a12_an_owners_start_runs_at_once() {
    let (s, mut owner) = configured();
    let (token, _) = s.pair(&mut owner, "Laptop", Role::Owner);
    let mut laptop = s.login(&token);
    let job = laptop.send_job("own.gcode", JOB, deadline()).unwrap();
    laptop.send(&Message::Start { job_id: job.job_id }).unwrap();
    wait_state(&mut laptop, State::Running);
}

#[test]
fn a16_a_controller_change_waits_for_no_job_and_applies_without_a_restart() {
    let (s, mut owner) = configured();
    let job = owner.send_job("held.gcode", JOB, deadline()).unwrap();
    let mut changed = fake_config(500);
    changed.name = "garage-laser".into();
    let busy = owner
        .request(
            &Message::ApplyConfig {
                config: changed.clone(),
                client_name: CTL.into(),
            },
            deadline(),
        )
        .unwrap();
    assert_eq!(code_of(&busy), Some(ErrorCode::WrongState));
    owner.send(&Message::Stop).unwrap();
    let _ = job;
    wait_state(&mut owner, State::Idle);
    let done = owner
        .request(
            &Message::ApplyConfig {
                config: changed.clone(),
                client_name: CTL.into(),
            },
            deadline(),
        )
        .unwrap();
    assert_eq!(done, Message::ConfigApplied { token: None });
    let config = owner.request(&Message::GetConfig, deadline()).unwrap();
    assert_eq!(config, Message::Config { config: changed });
    assert_eq!(s.insecure().greeting.name, "garage-laser");
    let admin = s.file("admin.log");
    assert!(
        admin.contains("\tconfig-changed\tcurvyoctl@test\t\tcontroller, name\n"),
        "{admin}"
    );
    // The new controller streams at its own pace.
    let job = owner.send_job("fast.gcode", JOB, deadline()).unwrap();
    owner.send(&Message::Start { job_id: job.job_id }).unwrap();
    wait_for(&mut owner, |m| {
        matches!(
            m,
            Message::Status {
                state: State::Done { .. },
                ..
            }
        )
    });
    let grbl = ControllerConfig::GrblSerial {
        port: String::new(),
        baud: 115_200,
    };
    let mut bad = fake_config(5);
    bad.controller = grbl;
    let invalid = owner
        .request(
            &Message::ApplyConfig {
                config: bad,
                client_name: CTL.into(),
            },
            deadline(),
        )
        .unwrap();
    assert_eq!(code_of(&invalid), Some(ErrorCode::InvalidConfig));
}

#[test]
fn a18_get_log_returns_the_last_lines_of_either_log() {
    let (s, mut owner) = configured();
    let job = owner.send_job("one.gcode", JOB, deadline()).unwrap();
    owner.send(&Message::Start { job_id: job.job_id }).unwrap();
    wait_for(&mut owner, |m| {
        matches!(
            m,
            Message::Status {
                state: State::Done { .. },
                ..
            }
        )
    });
    let jobs = owner
        .request(
            &Message::GetLog {
                log: LogName::Jobs,
                last: 20,
            },
            deadline(),
        )
        .unwrap();
    let Message::LogLines { lines } = jobs else {
        panic!("{jobs:?}")
    };
    assert_eq!(lines.len(), 1);
    assert!(lines[0].contains("\tone.gcode\tCompleted\t"), "{lines:?}");
    let admin = owner
        .request(
            &Message::GetLog {
                log: LogName::Admin,
                last: 0,
            },
            deadline(),
        )
        .unwrap();
    let Message::LogLines { lines } = admin else {
        panic!("{admin:?}")
    };
    assert_eq!(lines.len(), 1, "last is clamped to at least 1");
    assert!(lines[0].contains("\tconfigured\t"), "{lines:?}");
    let _ = s;
}
