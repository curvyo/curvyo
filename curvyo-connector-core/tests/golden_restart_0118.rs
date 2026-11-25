//! Golden files of the restart-from-a-line messages (`specs/0118` 2e, adrs R7), protocol 0,
//! additive: old goldens keep their bytes.
#![allow(clippy::unwrap_used)]

mod common;

use common::{fixture, job_id};
use curvyo_connector_core::{
    JobResult, ListEntry, Message, Millimetres, Progress, State, decode, encode,
};
use uuid::Uuid;

fn interrupted(cause: Option<&str>, approximate: bool) -> ListEntry {
    ListEntry {
        job_id: Uuid::from_u128(9),
        name: "sign.gcode".into(),
        lines: 4000,
        bytes: 4301,
        sent_by: "Phone".into(),
        position: 0,
        run_next: false,
        interrupted_at: Some(1249),
        restart_line: Some(1234),
        restart_cause: cause.map(str::to_owned),
        line_approximate: approximate,
    }
}

fn status(state: State, line: u32, entering: bool) -> Message {
    Message::Status {
        state,
        job: Some(Progress {
            job_id: job_id(),
            line,
            lines: 4000,
            elapsed_ms: 1500,
            from_line: Some(1234),
            entering,
        }),
        position: Some([12.5, 0.0, 0.0].map(Millimetres)),
        alarm: None,
        detail: None,
        pause_pending: false,
        requested_by: None,
        job_name: None,
        sent_by: None,
        homed: true,
        machine_position: Some([-387.5, -300.0, 0.0].map(Millimetres)),
        work_offset: Some([-400.0, -300.0, 0.0].map(Millimetres)),
        frame_id: None,
        target: None,
    }
}

fn all() -> Vec<(&'static str, Message)> {
    vec![
        (
            "command_restart_from_line",
            Message::RestartFromLine {
                job_id: job_id(),
                line: 1234,
            },
        ),
        (
            "job_list_restart_not_homed",
            Message::JobList {
                current: None,
                waiting: vec![interrupted(Some("Not homed"), false)],
                note: None,
            },
        ),
        (
            "job_list_restart_near",
            Message::JobList {
                current: None,
                waiting: vec![interrupted(None, true)],
                note: None,
            },
        ),
        ("status_restart_moving", status(State::Running, 1233, true)),
        (
            "status_restart_running",
            status(State::Running, 2001, false),
        ),
        (
            "status_done_from_line",
            status(
                State::Done {
                    result: JobResult::Completed,
                },
                4000,
                false,
            ),
        ),
    ]
}

#[test]
fn every_restart_message_matches_its_golden_file_and_round_trips() {
    for (name, message) in all() {
        let golden = fixture(&format!("messages/{name}.json"));
        assert_eq!(encode(&message).unwrap(), golden, "encoding of {name}");
        assert_eq!(decode(&golden).unwrap(), message, "decoding of {name}");
    }
}

#[test]
fn an_interrupted_entry_of_slice_one_decodes_without_the_new_fields() {
    let golden = fixture("messages/job_list_interrupted.json");
    let Message::JobList { waiting, .. } = decode(&golden).unwrap() else {
        panic!()
    };
    assert_eq!(waiting[0].restart_line, None);
    assert_eq!(waiting[0].restart_cause, None);
    assert!(!waiting[0].line_approximate);
}
