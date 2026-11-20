//! A12 to A15 (`specs/0128`, architect addendum D5): under the owner rule a member's start or
//! resume waits until an owner sends `Start`; Pause and Stop work for anyone at once.
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::{JOB_TEXT, chunk, job_id, last_status, submit};
use curvyo_connector_core::{
    Confirmation, ControllerCommand, ErrorCode, Event, JobKind, JobResult, Machine, Message,
    Options, Output, Peer, People, Role, StartRule, State,
};

const OWNER_RULE: Options = Options {
    start_rule: StartRule::OwnerConfirms,
};

fn phone() -> Peer {
    Peer::new("Phone", Role::Member)
}

fn laptop() -> Peer {
    Peer::new("Laptop", Role::Owner)
}

struct Rig {
    machine: Machine,
}

impl Rig {
    /// A job sent by `sender`, `Ready`; `confirming` gives the real machine's confirmations.
    fn ready(sender: &Peer, confirming: bool) -> Self {
        let mut machine = if confirming {
            Machine::confirming(OWNER_RULE)
        } else {
            Machine::new(OWNER_RULE)
        };
        machine.handle(Event::ControllerConnected);
        let mut rig = Self { machine };
        rig.from(sender, submit(JOB_TEXT, JobKind::Job));
        rig.from(sender, chunk(0, JOB_TEXT));
        assert_eq!(rig.machine.state(), State::Ready);
        rig
    }

    fn from(&mut self, peer: &Peer, message: Message) -> Output {
        self.machine.handle(Event::Message(message, peer.clone()))
    }

    fn start(&mut self, peer: &Peer) -> Output {
        self.from(peer, Message::Start { job_id: job_id() })
    }

    fn requested_by(&self) -> Option<String> {
        match self.machine.status() {
            Message::Status { requested_by, .. } => requested_by,
            _ => None,
        }
    }
}

fn error_code(out: &Output) -> Option<ErrorCode> {
    out.messages.iter().find_map(|m| match m {
        Message::Error { code, .. } => Some(*code),
        _ => None,
    })
}

#[test]
fn a12_an_owners_start_runs_at_once() {
    let mut rig = Rig::ready(&phone(), false);
    let out = rig.start(&laptop());
    assert_eq!(
        out.commands,
        vec![ControllerCommand::Start { job_id: job_id() }]
    );
    assert_eq!(rig.machine.state(), State::Running);
    assert_eq!(out.confirmed, None);
    assert_eq!(
        rig.machine.people(),
        Some(&People {
            sent_by: "Phone".into(),
            started_by: Some("Laptop".into()),
            confirmed_by: None,
        })
    );
}

#[test]
fn a13_a_members_start_waits_and_names_the_member() {
    let mut rig = Rig::ready(&phone(), false);
    let out = rig.start(&phone());
    assert_eq!(out.commands, Vec::new());
    assert_eq!(rig.machine.state(), State::AwaitingLocalStart);
    assert_eq!(rig.requested_by().as_deref(), Some("Phone"));
    assert!(matches!(
        last_status(&out),
        Message::Status { requested_by: Some(name), job: Some(_), .. } if name == "Phone"
    ));
    // A second request meanwhile does not fit; another member cannot confirm.
    assert_eq!(
        error_code(&rig.start(&phone())),
        Some(ErrorCode::WrongState)
    );
    assert_eq!(rig.machine.state(), State::AwaitingLocalStart);
}

#[test]
fn a14_an_owner_confirms_with_the_job_id_and_the_confirmation_is_reported() {
    let mut rig = Rig::ready(&phone(), false);
    rig.start(&phone());
    let other = uuid::Uuid::nil();
    let out = rig.from(&laptop(), Message::Start { job_id: other });
    assert_eq!(error_code(&out), Some(ErrorCode::UnknownJob));
    assert_eq!(rig.machine.state(), State::AwaitingLocalStart);
    let out = rig.start(&laptop());
    assert_eq!(
        out.commands,
        vec![ControllerCommand::Start { job_id: job_id() }]
    );
    assert_eq!(rig.machine.state(), State::Running);
    assert_eq!(
        out.confirmed,
        Some(Confirmation {
            job_id: job_id(),
            requested_by: "Phone".into(),
            confirmed_by: Some("Laptop".into()),
            resume: false,
        })
    );
    assert_eq!(rig.requested_by(), None);
    let people = rig.machine.people().unwrap();
    assert_eq!(people.started_by.as_deref(), Some("Phone"));
    assert_eq!(people.confirmed_by.as_deref(), Some("Laptop"));
}

#[test]
fn a14_a_members_stop_discards_a_waiting_start() {
    let mut rig = Rig::ready(&phone(), false);
    rig.start(&phone());
    let out = rig.from(&phone(), Message::Stop);
    assert_eq!(out.commands, Vec::new());
    assert_eq!(rig.machine.state(), State::Idle);
    assert_eq!(rig.machine.held_job(), None);
}

#[test]
fn a13_a_members_resume_waits_and_an_owner_resumes_it() {
    let mut rig = Rig::ready(&phone(), false);
    rig.start(&laptop());
    rig.machine.handle(Event::Progress {
        line: 3,
        elapsed_ms: 500,
    });
    // Pause works for a member at once (Frage 249 A).
    let out = rig.from(&phone(), Message::Pause);
    assert_eq!(out.commands, vec![ControllerCommand::Pause]);
    assert_eq!(rig.machine.state(), State::Paused);
    let out = rig.from(&phone(), Message::Resume);
    assert_eq!(out.commands, Vec::new());
    assert_eq!(rig.machine.state(), State::AwaitingLocalStart);
    assert!(matches!(
        last_status(&out),
        Message::Status { job: Some(p), requested_by: Some(_), .. } if p.line == 3
    ));
    // The owner's confirmation is its Start of that job (the app's and curvyoctl's way).
    let out = rig.start(&laptop());
    assert_eq!(out.commands, vec![ControllerCommand::Resume]);
    assert_eq!(rig.machine.state(), State::Running);
    assert!(out.confirmed.unwrap().resume);
}

#[test]
fn a14_a_stop_of_a_waiting_resume_stops_the_paused_job_through_the_controller() {
    let mut rig = Rig::ready(&phone(), true);
    rig.start(&laptop());
    rig.from(&phone(), Message::Pause);
    rig.machine.handle(Event::Held);
    rig.from(&phone(), Message::Resume);
    assert!(rig.machine.waits_for_resume());
    let out = rig.from(&phone(), Message::Stop);
    assert_eq!(out.commands, vec![ControllerCommand::Stop]);
    rig.machine.handle(Event::Stopped);
    assert_eq!(
        rig.machine.state(),
        State::Done {
            result: JobResult::Stopped
        }
    );
}

#[test]
fn a14_an_owners_resume_of_a_waiting_resume_waits_for_the_controller_on_a_real_machine() {
    let mut rig = Rig::ready(&phone(), true);
    rig.start(&laptop());
    rig.from(&laptop(), Message::Pause);
    rig.machine.handle(Event::Held);
    rig.from(&phone(), Message::Resume);
    let out = rig.from(&laptop(), Message::Resume);
    assert_eq!(out.commands, vec![ControllerCommand::Resume]);
    assert_eq!(rig.machine.state(), State::Paused);
    rig.machine.handle(Event::Resumed);
    assert_eq!(rig.machine.state(), State::Running);
}

#[test]
fn a_members_home_is_refused_under_the_owner_rule() {
    let mut rig = Rig::ready(&phone(), false);
    let out = rig.from(&phone(), Message::Home);
    assert_eq!(error_code(&out), Some(ErrorCode::OwnerRequired));
    assert_eq!(out.commands, Vec::new());
    let out = rig.from(&laptop(), Message::Home);
    assert_eq!(out.commands, vec![ControllerCommand::Home]);
}

#[test]
fn the_terminal_press_reports_a_confirmation_without_an_owner() {
    let options = Options {
        start_rule: StartRule::LocalPress,
    };
    let mut machine = Machine::new(options);
    machine.handle(Event::ControllerConnected);
    let phone = phone();
    machine.handle(Event::Message(
        submit(JOB_TEXT, JobKind::Job),
        phone.clone(),
    ));
    machine.handle(Event::Message(chunk(0, JOB_TEXT), phone.clone()));
    machine.handle(Event::Message(Message::Start { job_id: job_id() }, phone));
    // The Enter wait carries no requester on the wire (UX notes A5).
    assert!(matches!(
        machine.status(),
        Message::Status {
            requested_by: None,
            ..
        }
    ));
    let out = machine.handle(Event::LocalStartPressed);
    assert_eq!(machine.state(), State::Running);
    assert_eq!(out.confirmed.unwrap().confirmed_by, None);
}

#[test]
fn the_terminal_press_does_nothing_under_the_owner_rule() {
    let mut rig = Rig::ready(&phone(), false);
    rig.start(&phone());
    let out = rig.machine.handle(Event::LocalStartPressed);
    assert_eq!(out.commands, Vec::new());
    assert_eq!(rig.machine.state(), State::AwaitingLocalStart);
}
