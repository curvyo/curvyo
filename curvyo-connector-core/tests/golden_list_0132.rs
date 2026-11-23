//! Golden files of the job list messages (`specs/0132` criterion 2, 9), protocol 0, additive.
#![allow(clippy::unwrap_used)]

mod common;

use common::{fixture, job_id};
use curvyo_connector_core::{ListEntry, Message, RejectReason, decode, encode};
use uuid::Uuid;

fn entry(n: u128, name: &str, position: u32) -> ListEntry {
    ListEntry {
        job_id: Uuid::from_u128(n),
        name: name.into(),
        lines: 412,
        bytes: 4301,
        sent_by: "Phone".into(),
        position,
        run_next: false,
        interrupted_at: None,
    }
}

fn all() -> Vec<(&'static str, Message)> {
    let mut flagged = entry(3, "tags.gcode", 2);
    flagged.run_next = true;
    let mut interrupted = entry(9, "box.gcode", 0);
    interrupted.interrupted_at = Some(1234);
    vec![
        (
            "job_accepted_waiting",
            Message::JobAccepted {
                job_id: job_id(),
                position: Some(3),
            },
        ),
        (
            "job_rejected_list_full",
            Message::JobRejected {
                job_id: job_id(),
                reason: RejectReason::ListFull,
            },
        ),
        (
            "job_list",
            Message::JobList {
                current: Some(entry(1, "sign.gcode", 0)),
                waiting: vec![entry(2, "box.gcode", 1), flagged],
                note: None,
            },
        ),
        (
            "job_list_chain_stopped",
            Message::JobList {
                current: None,
                waiting: vec![entry(2, "box.gcode", 1)],
                note: Some("Chain stopped: Failed".into()),
            },
        ),
        (
            "job_list_interrupted",
            Message::JobList {
                current: None,
                waiting: vec![interrupted, entry(2, "sign.gcode", 1)],
                note: None,
            },
        ),
        (
            "move_job",
            Message::MoveJob {
                job_id: job_id(),
                from: 3,
                to: 1,
            },
        ),
        ("remove_job", Message::RemoveJob { job_id: job_id() }),
        (
            "set_run_next",
            Message::SetRunNext {
                job_id: job_id(),
                on: true,
            },
        ),
    ]
}

#[test]
fn every_list_message_matches_its_golden_file_and_round_trips() {
    for (name, message) in all() {
        let golden = fixture(&format!("messages/{name}.json"));
        assert_eq!(encode(&message).unwrap(), golden, "encoding of {name}");
        assert_eq!(decode(&golden).unwrap(), message, "decoding of {name}");
    }
}

#[test]
fn a_job_accepted_of_a_current_job_is_byte_identical_to_protocol_zero_before() {
    let golden = fixture("messages/job_accepted.json");
    let message = Message::JobAccepted {
        job_id: job_id(),
        position: None,
    };
    assert_eq!(encode(&message).unwrap(), golden);
    // An old connector's answer, without `position`, still decodes.
    assert_eq!(decode(&golden).unwrap(), message);
}
