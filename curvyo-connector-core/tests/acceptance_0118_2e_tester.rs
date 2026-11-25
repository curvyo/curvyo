//! Acceptance tests of `specs/0118` slice 2e and `specs/0132` slice 2 criterion 12 at the pure
//! machine (no controller, no clock): a restart for an id the connector does not list is refused
//! with no command, and an owner's restart with no interrupted job holds nothing. The rules of
//! the restart line, the modal rebuild and the entry move are private modules of this crate
//! (`restart`, `modal`); their tables are tested where they are, and the wire and the GRBL bytes
//! are tested in `curvyo-connector-io/tests/acceptance_0118_2e_tester.rs`.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use curvyo_connector_core::{ControllerCommand, ErrorCode, Event, Machine, Message, Options, Peer};
use uuid::Uuid;

fn restart(job_id: Uuid, line: u32) -> Event {
    Event::Message(Message::RestartFromLine { job_id, line }, Peer::host())
}

#[test]
fn criterion_12_an_unknown_job_is_refused_and_no_command_is_given() {
    let mut machine = Machine::confirming(Options::default());
    let out = machine.handle(restart(Uuid::from_u128(0x2e), 5));
    assert!(
        out.messages.iter().any(|m| matches!(
            m,
            Message::Error {
                code: ErrorCode::UnknownJob,
                ..
            }
        )),
        "{:?}",
        out.messages
    );
    assert!(
        out.commands
            .iter()
            .all(|c| !matches!(c, ControllerCommand::StartAt { .. })),
        "no StartAt for an unknown job: {:?}",
        out.commands
    );
}

#[test]
fn criterion_12_a_restart_for_a_line_of_an_unknown_job_is_never_a_motion() {
    // The first byte of a restart is `M5`; a refusal before it writes nothing at all.
    let mut machine = Machine::confirming(Options::default());
    for line in [0, 1, 5, u32::MAX] {
        let out = machine.handle(restart(Uuid::from_u128(u128::from(line)), line));
        assert!(
            out.commands.is_empty(),
            "line {line} gave commands: {:?}",
            out.commands
        );
    }
}
