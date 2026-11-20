//! P1 to P5: encoding, golden files, typed decode errors and the frame limit.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]

mod common;

use common::{fixture, job_id};
use curvyo_connector_core::{
    CodecError, ControllerInfo, ErrorCode, JobKind, JobResult, MAX_FRAME_BYTES, Message,
    Millimetres, Progress, RejectReason, State, decode, encode,
};

const HASH: &str = "afdfb1bfea4a611e31fc3372a8960825bb26d849ab5c4de252388c182ac1b7cd";

fn submit(kind: JobKind, name: &str) -> Message {
    Message::SubmitJob {
        job_id: job_id(),
        name: name.to_owned(),
        kind,
        bytes: 52,
        sha256: HASH.to_owned(),
    }
}

fn status(
    state: State,
    job: Option<Progress>,
    position: Option<[f64; 3]>,
    alarm: Option<&str>,
) -> Message {
    Message::Status {
        state,
        job,
        position: position.map(|p| p.map(Millimetres)),
        alarm: alarm.map(str::to_owned),
        detail: None,
        pause_pending: false,
        requested_by: None,
        job_name: None,
        sent_by: None,
    }
}

fn progress(line: u32, elapsed_ms: u64) -> Progress {
    Progress {
        job_id: job_id(),
        line,
        lines: 6,
        elapsed_ms,
    }
}

fn controller() -> ControllerInfo {
    ControllerInfo {
        name: "/dev/ttyUSB0".to_owned(),
        firmware: "Grbl 1.1h".to_owned(),
        port: None,
        baud: None,
        settings: None,
    }
}

/// One value per golden file under `tests/fixtures/messages/`; the 18 types all appear.
#[allow(clippy::too_many_lines)] // one flat table, a split would only hide a row
fn all() -> Vec<(&'static str, Message)> {
    vec![
        (
            "hello",
            Message::Hello {
                protocol: 0,
                software: "curvyo-app".into(),
                version: "0.1.0".into(),
            },
        ),
        (
            "pair",
            Message::Pair {
                code: "K7M2QX9P".into(),
                client_name: "Workshop laptop".into(),
            },
        ),
        (
            "auth",
            Message::Auth {
                token: "c2VjcmV0LXRva2Vu".into(),
            },
        ),
        ("get_status", Message::GetStatus),
        ("submit_job", submit(JobKind::Job, "coaster.gcode")),
        ("submit_job_frame", submit(JobKind::Frame, "coaster frame")),
        (
            "job_chunk",
            Message::JobChunk {
                job_id: job_id(),
                index: 0,
                text: "G21\nG90\nG0 X0 Y0\n".into(),
            },
        ),
        ("start", Message::Start { job_id: job_id() }),
        ("pause", Message::Pause),
        ("resume", Message::Resume),
        ("stop", Message::Stop),
        ("command_unlock", Message::Unlock),
        ("command_home", Message::Home),
        (
            "welcome",
            Message::Welcome {
                connector_name: "Workshop Pi".into(),
                controller: Some(controller()),
                mode: None,
            },
        ),
        (
            "welcome_no_controller",
            Message::Welcome {
                connector_name: "Workshop Pi".into(),
                controller: None,
                mode: None,
            },
        ),
        (
            "paired",
            Message::Paired {
                token: "c2VjcmV0LXRva2Vu".into(),
            },
        ),
        ("job_accepted", Message::JobAccepted { job_id: job_id() }),
        (
            "job_rejected",
            Message::JobRejected {
                job_id: job_id(),
                reason: RejectReason::HashMismatch,
            },
        ),
        ("status_idle", status(State::Idle, None, None, None)),
        (
            "status_ready",
            status(State::Ready, Some(progress(0, 0)), None, None),
        ),
        (
            "status_running",
            status(
                State::Running,
                Some(progress(3, 1500)),
                Some([12.5, 0.0, -1.25]),
                None,
            ),
        ),
        (
            "status_done",
            status(
                State::Done {
                    result: JobResult::Completed,
                },
                Some(progress(6, 4200)),
                None,
                None,
            ),
        ),
        (
            "status_alarm",
            status(State::Alarm, None, None, Some("ALARM:1 Hard limit")),
        ),
        (
            "error",
            Message::Error {
                code: ErrorCode::WrongState,
                message: "Start needs a job that is ready.".into(),
            },
        ),
    ]
}

#[test]
fn every_message_matches_its_golden_file_and_round_trips() {
    for (name, message) in all() {
        let golden = fixture(&format!("messages/{name}.json"));
        assert_eq!(encode(&message).unwrap(), golden, "encoding of {name}");
        assert_eq!(decode(&golden).unwrap(), message, "decoding of {name}");
    }
}

#[test]
fn the_goldens_cover_all_eighteen_types() {
    let mut types: Vec<String> = all()
        .iter()
        .map(|(_, m)| {
            let v: serde_json::Value = serde_json::from_str(&encode(m).unwrap()).unwrap();
            v["type"].as_str().unwrap().to_owned()
        })
        .collect();
    types.sort();
    types.dedup();
    assert_eq!(types.len(), 18);
}

#[test]
fn status_shows_empty_fields_as_null_not_omitted() {
    let golden = fixture("messages/status_idle.json");
    let v: serde_json::Value = serde_json::from_str(&golden).unwrap();
    for key in ["job", "position", "alarm"] {
        assert!(v.get(key).is_some_and(serde_json::Value::is_null), "{key}");
    }
}

#[test]
fn every_error_code_is_fixed_by_the_golden_file() {
    // `codes.json` holds the codes up to slice 4, `codes_0128.json` the ones `specs/0128` added;
    // together, in order, they are every code.
    let (old, added) = ErrorCode::ALL.split_at(9);
    assert_eq!(
        serde_json::to_string(old).unwrap(),
        fixture("errors/codes.json")
    );
    assert_eq!(
        serde_json::to_string(added).unwrap(),
        fixture("errors/codes_0128.json")
    );
}

#[test]
fn bad_frames_give_typed_errors_with_distinct_codes() {
    let malformed = [
        "not json",
        "{\"type\":\"Pause\"",
        "{\"type\":\"Start\"}",
        "{\"type\":\"Start\",\"job_id\":7}",
        "[1]",
        "{}",
        "{\"type\":7}",
    ];
    for text in malformed {
        let err = decode(text).unwrap_err();
        assert!(matches!(err, CodecError::Malformed(_)), "{text}: {err:?}");
        assert_eq!(
            encode(&err.to_error_message()).unwrap(),
            fixture("errors/malformed.json")
        );
    }
    let err = decode("{\"type\":\"Reboot\"}").unwrap_err();
    assert_eq!(err, CodecError::UnknownType("Reboot".to_owned()));
    assert_eq!(
        encode(&err.to_error_message()).unwrap(),
        fixture("errors/unknown_type.json")
    );
}

#[test]
fn a_frame_is_refused_above_one_mebibyte_before_parsing_and_taken_at_exactly_one() {
    let over = "x".repeat(MAX_FRAME_BYTES + 1);
    let err = decode(&over).unwrap_err();
    assert_eq!(
        err,
        CodecError::FrameTooLarge {
            len: MAX_FRAME_BYTES + 1
        }
    );
    assert_eq!(
        encode(&err.to_error_message()).unwrap(),
        fixture("errors/frame_too_large.json")
    );

    let head = "{\"type\":\"Auth\",\"token\":\"";
    let tail = "\"}";
    let token = "a".repeat(MAX_FRAME_BYTES - head.len() - tail.len());
    let frame = format!("{head}{token}{tail}");
    assert_eq!(frame.len(), MAX_FRAME_BYTES);
    assert!(matches!(decode(&frame), Ok(Message::Auth { .. })));
}
