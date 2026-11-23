//! `specs/0118` slice 2a through the configured connector: a member's Jog is an owner's call, a
//! member's Home waits for an owner (criterion 37), a closed connection ends its hold (criterion
//! 18) and the logs name Home. Waiting is by status, never by sleep.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::service::{CTL, Service, code_of, deadline, service};
use common::{wait_for, wait_state};
use curvyo_connector_core::{ErrorCode, JogAxis, JogDirection, JogStep, Message, Role, State};
use curvyo_connector_io::client::Connection;

fn configured() -> (Service, Connection) {
    let s = service();
    let (owner, _) = s.setup(20);
    (s, owner)
}

fn jog(hold: bool) -> Message {
    Message::Jog {
        axis: JogAxis::X,
        direction: JogDirection::Plus,
        step: JogStep::Ten,
        hold,
    }
}

fn requested_by(status: &Message) -> Option<&str> {
    match status {
        Message::Status { requested_by, .. } => requested_by.as_deref(),
        _ => None,
    }
}

#[test]
fn c12_a_members_jog_is_refused_and_its_release_is_not() {
    let (s, mut owner) = configured();
    let (_, mut phone) = s.pair(&mut owner, "Phone", Role::Member);
    for message in [jog(false), jog(true), Message::JogKeep] {
        let answer = phone.request(&message, deadline()).unwrap();
        assert_eq!(
            code_of(&answer),
            Some(ErrorCode::OwnerRequired),
            "{message:?}"
        );
    }
    // A release passes the gate: the first error that comes back is the Pause's, not its.
    phone.send(&Message::JogStop).unwrap();
    let answer = phone.request(&Message::Pause, deadline()).unwrap();
    assert_eq!(code_of(&answer), Some(ErrorCode::WrongState));
}

#[test]
fn c37_a_members_home_waits_an_owners_home_confirms_and_the_logs_name_home() {
    let (s, mut owner) = configured();
    let (_, mut phone) = s.pair(&mut owner, "Phone", Role::Member);
    phone.send(&Message::Home).unwrap();
    let waiting = wait_state(&mut owner, State::AwaitingLocalStart);
    assert_eq!(requested_by(&waiting), Some("Phone"));
    assert!(matches!(
        waiting,
        Message::Status {
            job: None,
            job_name: None,
            ..
        }
    ));
    // A second member Home meanwhile is refused where it stands.
    let answer = phone.request(&Message::Home, deadline()).unwrap();
    assert_eq!(code_of(&answer), Some(ErrorCode::WrongState));
    owner.send(&Message::Home).unwrap();
    wait_state(&mut owner, State::Homing);
    wait_state(&mut owner, State::Idle);
    let admin = s.file("admin.log");
    assert!(
        admin.contains(&format!("\tstart-confirmed\t{CTL}\tPhone\tHome\n")),
        "{admin}"
    );
    let jobs = s.file("jobs.log");
    assert!(jobs.contains("\tHome: Phone\t"), "{jobs}");
    assert_eq!(
        jobs.matches("Home:").count(),
        1,
        "the confirmation is no second line"
    );
}

#[test]
fn c37_stop_from_anyone_discards_a_waiting_home() {
    let (s, mut owner) = configured();
    let (_, mut phone) = s.pair(&mut owner, "Phone", Role::Member);
    phone.send(&Message::Home).unwrap();
    wait_state(&mut owner, State::AwaitingLocalStart);
    owner.send(&Message::Stop).unwrap();
    wait_state(&mut owner, State::Idle);
    assert!(!s.file("admin.log").contains("Home"));
}

#[test]
fn c18_a_closed_connection_ends_the_hold_it_started() {
    let (s, mut owner) = configured();
    let (token, _) = s.pair(&mut owner, "Laptop", Role::Owner);
    let mut laptop = s.login(&token);
    laptop.send(&jog(true)).unwrap();
    wait_state(&mut owner, State::Jogging);
    drop(laptop);
    // No keep-alive ever came, so the dead-man would end it too; the close does it at once and
    // either way it ends without a Stop.
    wait_for(&mut owner, |m| {
        matches!(
            m,
            Message::Status {
                state: State::Idle,
                ..
            }
        )
    });
}

#[test]
fn c10_a_second_apps_stop_ends_a_jog_and_c19_the_owner_sees_jogging() {
    let (s, mut owner) = configured();
    let (token, _) = s.pair(&mut owner, "Laptop", Role::Owner);
    let mut laptop = s.login(&token);
    laptop.send(&jog(true)).unwrap();
    wait_state(&mut owner, State::Jogging);
    owner.send(&Message::Stop).unwrap();
    wait_state(&mut owner, State::Idle);
    assert!(s.file("jobs.log").contains("\tJog: Laptop\t"));
}
