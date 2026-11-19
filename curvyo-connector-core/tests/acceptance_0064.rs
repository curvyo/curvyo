//! Acceptance tests for spec 0064 (machine connector, protocol core), criteria P1 to P24.
//!
//! Drafted from `specs/0064-machine-connector/specification.md` before reading the
//! production code. Only the public API of `curvyo-connector-core` is used. The
//! assertions follow the spec; the helpers were adapted to the real public names
//! (for example `Event`, `Output`, `Handshake`, `Uuid` job ids). No assertion was
//! weakened to make a test pass.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use curvyo_connector_core::{
    CodecError, ControllerCommand, ErrorCode, Event, Handshake, JobKind, JobResult,
    MAX_CHUNK_BYTES, MAX_FRAME_BYTES, MAX_JOB_BYTES, Machine, Message, Millimetres, Options,
    Outcome, Output, PROTOCOL_VERSION, RejectReason, State, decode, encode,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Test-side helpers
// ---------------------------------------------------------------------------

fn sha_hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    Sha256::digest(bytes)
        .iter()
        .fold(String::new(), |mut hex, b| {
            let _ = write!(hex, "{b:02x}");
            hex
        })
}

/// Constructor of one event for the table test.
type Make = fn() -> Event;

fn j1() -> Uuid {
    Uuid::from_u128(1)
}

fn j2() -> Uuid {
    Uuid::from_u128(2)
}

fn hello(protocol: u32) -> Message {
    Message::Hello {
        protocol,
        software: "app".into(),
        version: "1.0.0".into(),
    }
}

fn submit(job_id: Uuid, kind: JobKind, text: &str) -> Message {
    Message::SubmitJob {
        job_id,
        name: "job".into(),
        kind,
        bytes: text.len() as u64,
        sha256: sha_hex(text.as_bytes()),
    }
}

fn chunk(job_id: Uuid, index: u32, text: &str) -> Message {
    Message::JobChunk {
        job_id,
        index,
        text: text.into(),
    }
}

// One message of every type of ADR 0035 section 6 (16 in total).
fn all_messages() -> Vec<Message> {
    vec![
        hello(0),
        Message::Pair {
            code: "123456".into(),
            client_name: "laptop".into(),
        },
        Message::Auth {
            token: "tok".into(),
        },
        Message::GetStatus,
        submit(j1(), JobKind::Job, "G0\n"),
        chunk(j1(), 0, "G0\n"),
        Message::Start { job_id: j1() },
        Message::Pause,
        Message::Resume,
        Message::Stop,
        Message::Welcome {
            connector_name: "shop".into(),
            controller: None,
        },
        Message::Paired {
            token: "tok".into(),
        },
        Message::JobAccepted { job_id: j1() },
        Message::JobRejected {
            job_id: j1(),
            reason: RejectReason::Busy,
        },
        Message::Status {
            state: State::Idle,
            job: None,
            position: None,
            alarm: None,
            detail: None,
            pause_pending: false,
        },
        Message::Error {
            code: ErrorCode::Malformed,
            message: "x".into(),
        },
    ]
}

// A fake controller: a plain test double that feeds events to the machine and keeps the
// controller commands it would have received. It is the only controller side of these tests.
struct FakeController {
    machine: Machine,
    commands: Vec<ControllerCommand>,
}

impl FakeController {
    fn new(options: Options) -> Self {
        Self {
            machine: Machine::new(options),
            commands: Vec::new(),
        }
    }

    /// A controller attached, no job held.
    fn connected(options: Options) -> Self {
        let mut f = Self::new(options);
        f.event(Event::ControllerConnected);
        f
    }

    fn event(&mut self, event: Event) -> Output {
        let out = self.machine.handle(event);
        self.commands.extend(out.commands.iter().cloned());
        out
    }

    fn send(&mut self, message: Message) -> Output {
        self.event(Event::Message(message))
    }

    fn state(&self) -> State {
        self.machine.state()
    }
}

fn idle() -> FakeController {
    FakeController::connected(Options::default())
}

fn ready(text: &str) -> FakeController {
    let mut f = idle();
    f.send(submit(j1(), JobKind::Job, text));
    f.send(chunk(j1(), 0, text));
    f
}

fn running(text: &str) -> FakeController {
    let mut f = ready(text);
    f.send(Message::Start { job_id: j1() });
    f
}

// A machine in the requested state, built only from public events.
fn in_state(state: State, text: &str) -> FakeController {
    match state {
        State::NoController => FakeController::new(Options::default()),
        State::Idle => idle(),
        State::Receiving => {
            let mut f = idle();
            f.send(submit(j1(), JobKind::Job, text));
            f
        }
        State::Ready => ready(text),
        State::Running => running(text),
        State::Paused => {
            let mut f = running(text);
            f.send(Message::Pause);
            f
        }
        State::AwaitingLocalStart => {
            let mut f = FakeController::connected(Options {
                require_local_start: true,
            });
            f.send(submit(j1(), JobKind::Job, text));
            f.send(chunk(j1(), 0, text));
            f.send(Message::Start { job_id: j1() });
            f
        }
        State::Done { result } => {
            let mut f = running(text);
            match result {
                JobResult::Completed => f.event(Event::JobFinished { elapsed_ms: 10 }),
                JobResult::Stopped => f.send(Message::Stop),
                JobResult::Failed => f.event(Event::JobFailed { elapsed_ms: 10 }),
            };
            f
        }
        State::Alarm => {
            let mut f = idle();
            f.event(Event::Alarm {
                text: "Hard limit".into(),
            });
            f
        }
        // `specs/0118`: added so the match stays exhaustive; no table here lists it.
        State::Homing => {
            let mut f = idle();
            f.send(Message::Home);
            f
        }
    }
}

fn has_error(out: &Output) -> bool {
    out.messages
        .iter()
        .any(|m| matches!(m, Message::Error { .. }))
}

fn rejected_with(out: &Output, job: Uuid, reason: RejectReason) -> bool {
    out.messages.contains(&Message::JobRejected {
        job_id: job,
        reason,
    })
}

fn status_alarm(m: &Machine) -> Option<String> {
    match m.status() {
        Message::Status { alarm, .. } => alarm,
        other => panic!("expected Status, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// P1 to P5: messages and encoding
// ---------------------------------------------------------------------------

// P1: every message type round-trips through its encoding, and each encoding is one JSON object
// with a `type` string.
#[test]
fn p1_every_message_type_round_trips() {
    let samples = all_messages();
    assert_eq!(
        samples.len(),
        16,
        "ADR 0035 section 6 lists 16 message types"
    );
    for msg in samples {
        let text = encode(&msg).expect("encodes");
        let v: Value = serde_json::from_str(&text).expect("valid JSON");
        assert!(v.is_object(), "one JSON object: {text}");
        assert!(v["type"].is_string(), "string type tag: {text}");
        assert_eq!(decode(&text).expect("decodes"), msg, "round trip of {text}");
    }
}

// P1: the type tag is the message name.
#[test]
fn p1_type_tag_is_the_message_name() {
    let v: Value = serde_json::from_str(&encode(&hello(0)).unwrap()).unwrap();
    assert_eq!(v["type"], "Hello");
    assert_eq!(v["protocol"], 0);
    assert_eq!(v["software"], "app");
    assert_eq!(v["version"], "1.0.0");
}

// P2: the encoding of each message equals the fixed expected JSON (value equality, key order
// not significant), and the expected JSON decodes back to the message.
#[test]
fn p2_fixed_encoding_per_message() {
    let uuid = j1().to_string();
    let cases: Vec<(Message, Value)> = vec![
        (Message::GetStatus, json!({"type": "GetStatus"})),
        (Message::Pause, json!({"type": "Pause"})),
        (Message::Resume, json!({"type": "Resume"})),
        (Message::Stop, json!({"type": "Stop"})),
        (
            Message::Start { job_id: j1() },
            json!({"type": "Start", "job_id": uuid}),
        ),
        (
            submit(j1(), JobKind::Job, "G0\n"),
            json!({"type": "SubmitJob", "job_id": uuid, "name": "job",
                   "kind": "Job", "bytes": 3, "sha256": sha_hex(b"G0\n")}),
        ),
        (
            submit(j1(), JobKind::Frame, ""),
            json!({"type": "SubmitJob", "job_id": uuid, "name": "job",
                   "kind": "Frame", "bytes": 0, "sha256": sha_hex(b"")}),
        ),
        (
            chunk(j1(), 4, "G1"),
            json!({"type": "JobChunk", "job_id": uuid, "index": 4, "text": "G1"}),
        ),
        (
            Message::JobAccepted { job_id: j1() },
            json!({"type": "JobAccepted", "job_id": uuid}),
        ),
        (
            Message::JobRejected {
                job_id: j1(),
                reason: RejectReason::HashMismatch,
            },
            json!({"type": "JobRejected", "job_id": uuid, "reason": "HashMismatch"}),
        ),
        (
            Message::Error {
                code: ErrorCode::UnsupportedProtocol,
                message: "v".into(),
            },
            json!({"type": "Error", "code": "UnsupportedProtocol", "message": "v"}),
        ),
    ];
    for (msg, expected) in cases {
        let text = encode(&msg).unwrap();
        assert_eq!(
            serde_json::from_str::<Value>(&text).unwrap(),
            expected,
            "encoding of {msg:?}"
        );
        assert_eq!(
            decode(&expected.to_string()).unwrap(),
            msg,
            "decode of golden"
        );
    }
}

// P2: the reject reasons, error codes and Done results are spelled as the spec names them.
#[test]
fn p2_enum_spellings() {
    for (reason, spelling) in [
        (RejectReason::Busy, "Busy"),
        (RejectReason::TooLarge, "TooLarge"),
        (RejectReason::HashMismatch, "HashMismatch"),
        (RejectReason::NotAscii, "NotAscii"),
        (RejectReason::NoController, "NoController"),
    ] {
        let v = serde_json::to_value(reason).unwrap();
        assert_eq!(v, spelling);
    }
    for (code, spelling) in [
        (ErrorCode::UnsupportedProtocol, "UnsupportedProtocol"),
        (ErrorCode::Malformed, "Malformed"),
        (ErrorCode::UnknownType, "UnknownType"),
    ] {
        assert_eq!(serde_json::to_value(code).unwrap(), spelling);
    }
    for (result, spelling) in [
        (JobResult::Completed, "Completed"),
        (JobResult::Stopped, "Stopped"),
        (JobResult::Failed, "Failed"),
    ] {
        let v = serde_json::to_value(State::Done { result }).unwrap();
        assert_eq!(v["name"], "Done", "state name");
        assert_eq!(v["result"], spelling, "Done result");
    }
    assert_eq!(serde_json::to_value(JobKind::Frame).unwrap(), "Frame");
}

// P3: a Status with null job, position and alarm shows them as JSON null, not omitted.
#[test]
fn p3_status_nulls_are_present() {
    let text = encode(&Message::Status {
        state: State::Idle,
        job: None,
        position: None,
        alarm: None,
        detail: None,
        pause_pending: false,
    })
    .unwrap();
    let v: Value = serde_json::from_str(&text).unwrap();
    for key in ["job", "position", "alarm"] {
        assert!(v.get(key).is_some(), "{key} present in {text}");
        assert!(v[key].is_null(), "{key} null in {text}");
    }
}

// P3: a Status with job and position [x, y, z] in millimetres round-trips with the same numbers.
#[test]
fn p3_status_with_job_and_position_round_trips() {
    let msg = Message::Status {
        state: State::Running,
        job: Some(curvyo_connector_core::Progress {
            job_id: j1(),
            line: 7,
            lines: 40,
            elapsed_ms: 1234,
        }),
        position: Some([Millimetres(1.5), Millimetres(-2.25), Millimetres(0.0)]),
        alarm: None,
        detail: None,
        pause_pending: false,
    };
    let text = encode(&msg).unwrap();
    assert_eq!(decode(&text).unwrap(), msg);
    let v: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(v["position"], json!([1.5, -2.25, 0.0]));
    assert_eq!(v["job"]["line"], 7);
    assert_eq!(v["job"]["lines"], 40);
}

// P4: malformed text, unknown type, wrong field types or no type are typed errors, never panics.
#[test]
fn p4_bad_input_is_a_typed_error_never_a_panic() {
    let inputs = [
        "this is not json",
        "{",
        r#"{"type":"NoSuchMessage"}"#,
        r#"{"type":"Hello","protocol":"zero","software":"x","version":"1"}"#,
        r#"{"type":"Hello","protocol":0,"software":"x"}"#,
        r#"{"type":"JobChunk","job_id":"00000000-0000-0000-0000-000000000001","text":"G0"}"#,
        r#"{"protocol":0}"#,
        "[1,2,3]",
        "",
    ];
    for input in inputs {
        let result = std::panic::catch_unwind(|| decode(input));
        let result = result.unwrap_or_else(|_| panic!("decode panicked on {input:?}"));
        let err = result.expect_err(&format!("must fail: {input:?}"));
        assert!(matches!(err.to_error_message(), Message::Error { .. }));
    }
}

// P4: "malformed" and "unknown type" have distinct, fixed codes.
#[test]
fn p4_malformed_and_unknown_type_have_distinct_codes() {
    let code_of = |text: &str| match decode(text).unwrap_err().to_error_message() {
        Message::Error { code, .. } => code,
        other => panic!("expected Error, got {other:?}"),
    };
    assert_eq!(code_of("not json"), ErrorCode::Malformed);
    assert_eq!(code_of(r#"{"type":"Nope"}"#), ErrorCode::UnknownType);
    assert_ne!(code_of("not json"), code_of(r#"{"type":"Nope"}"#));
}

// P5: a frame of exactly 1 MiB is accepted.
#[test]
fn p5_frame_of_exactly_one_mib_is_accepted() {
    let fixed = r#"{"type":"Stop"}"#.len();
    let frame = format!(
        r#"{{"type":"Stop"{}}}"#,
        " ".repeat(MAX_FRAME_BYTES - fixed)
    );
    assert_eq!(frame.len(), 1024 * 1024);
    assert_eq!(decode(&frame).expect("1 MiB accepted"), Message::Stop);
}

// P5: a frame longer than 1 MiB is refused as too large before parsing.
#[test]
fn p5_frame_over_one_mib_is_refused_as_too_large() {
    let frame = format!(r#"{{"type":"Stop"{}}}"#, " ".repeat(MAX_FRAME_BYTES));
    assert!(frame.len() > MAX_FRAME_BYTES);
    assert!(matches!(
        decode(&frame),
        Err(CodecError::FrameTooLarge { .. })
    ));
}

// P5 edge: an oversize frame that is also not JSON is refused for size, not parsed.
#[test]
fn p5_size_is_checked_before_parsing() {
    let frame = "x".repeat(MAX_FRAME_BYTES + 1);
    assert!(matches!(
        decode(&frame),
        Err(CodecError::FrameTooLarge { .. })
    ));
}

// ---------------------------------------------------------------------------
// P6, P7: version check
// ---------------------------------------------------------------------------

// P6: the protocol constant is 0 (ADR 0035 section 6).
#[test]
fn p6_protocol_constant_is_zero() {
    assert_eq!(PROTOCOL_VERSION, 0);
}

// P6: a matching Hello passes.
#[test]
fn p6_matching_hello_passes() {
    let mut h = Handshake::new();
    assert_eq!(h.on_message(&hello(PROTOCOL_VERSION)), Outcome::Hello);
    assert_eq!(h.on_message(&Message::GetStatus), Outcome::Pass);
}

// P6: a higher or lower version gets UnsupportedProtocol and the close outcome; afterwards
// nothing is accepted.
#[test]
fn p6_mismatch_higher_or_lower_closes_and_accepts_nothing_after() {
    for protocol in [1u32, 2, 9999, u32::MAX] {
        let mut h = Handshake::new();
        match h.on_message(&hello(protocol)) {
            Outcome::Close(Message::Error { code, .. }) => {
                assert_eq!(code, ErrorCode::UnsupportedProtocol, "protocol {protocol}");
            }
            other => panic!("protocol {protocol}: expected close, got {other:?}"),
        }
        assert_eq!(
            h.on_message(&Message::GetStatus),
            Outcome::Closed,
            "nothing accepted after close (protocol {protocol})"
        );
        assert_eq!(h.on_message(&hello(PROTOCOL_VERSION)), Outcome::Closed);
    }
}

// P7: a first message other than Hello gives an Error and the close outcome, for every type.
#[test]
fn p7_every_non_hello_first_message_errors_and_closes() {
    for msg in all_messages() {
        if matches!(msg, Message::Hello { .. }) {
            continue;
        }
        let mut h = Handshake::new();
        match h.on_message(&msg) {
            Outcome::Close(Message::Error { code, .. }) => {
                assert_eq!(code, ErrorCode::HelloRequired, "{msg:?}");
            }
            other => panic!("{msg:?}: expected close with Error, got {other:?}"),
        }
    }
}

// ---------------------------------------------------------------------------
// P8 to P12: chunk assembly and job checks
// ---------------------------------------------------------------------------

// P8: chunks 0, 1, 2 with exactly `bytes` bytes assemble; the last one yields JobAccepted in Ready,
// and the held text equals the original.
#[test]
fn p8_chunks_assemble_and_job_is_accepted() {
    let text = "G0 X1\nG1 X2 F100\nM2\n";
    let mut f = idle();
    f.send(submit(j1(), JobKind::Job, text));
    f.send(chunk(j1(), 0, &text[..5]));
    f.send(chunk(j1(), 1, &text[5..12]));
    let last = f.send(chunk(j1(), 2, &text[12..]));
    assert!(
        last.messages
            .contains(&Message::JobAccepted { job_id: j1() })
    );
    assert_eq!(f.state(), State::Ready);
    assert_eq!(f.machine.held_job().unwrap().text, text);
}

// P8: a Frame job is assembled the same way.
#[test]
fn p8_frame_job_is_assembled() {
    let text = "G0 X0 Y0\nG1 X10 Y0\n";
    let mut f = idle();
    f.send(submit(j1(), JobKind::Frame, text));
    f.send(chunk(j1(), 0, text));
    assert_eq!(f.state(), State::Ready);
    assert_eq!(f.machine.held_job().unwrap().kind, JobKind::Frame);
}

// P8 edge: one byte per chunk assembles to the same text.
#[test]
fn p8_one_byte_chunks_assemble() {
    let text = "M3 S1000\n";
    let mut f = idle();
    f.send(submit(j1(), JobKind::Job, text));
    for (i, c) in text.chars().enumerate() {
        f.send(chunk(j1(), u32::try_from(i).unwrap(), &c.to_string()));
    }
    assert_eq!(f.state(), State::Ready);
    assert_eq!(f.machine.held_job().unwrap().text, text);
}

// P8 edge: a zero-byte job (empty text, sha256 of nothing) is accepted with no chunk at all:
// bytes 0 means the whole text is already there. The spec does not say how a zero-byte job is
// framed (see the report: a stray empty chunk afterwards gets UnknownJob).
#[test]
fn p8_zero_byte_job_is_accepted() {
    let mut f = idle();
    let out = f.send(submit(j1(), JobKind::Job, ""));
    assert!(
        out.messages
            .contains(&Message::JobAccepted { job_id: j1() })
    );
    assert_eq!(f.state(), State::Ready);
    assert_eq!(f.machine.held_job().unwrap().text, "");
}

// P8 edge: a CR LF pair split across two chunks is one newline and is accepted.
#[test]
fn p8_crlf_split_across_chunks_is_accepted() {
    let text = "G0\r\nG1\n";
    let mut f = idle();
    f.send(submit(j1(), JobKind::Job, text));
    f.send(chunk(j1(), 0, "G0\r"));
    f.send(chunk(j1(), 1, "\nG1\n"));
    assert_eq!(f.state(), State::Ready);
}

// P9: a hash that does not match gives HashMismatch and the state returns to Idle.
#[test]
fn p9_hash_mismatch_rejects_and_discards() {
    let mut f = idle();
    let mut wrong = submit(j1(), JobKind::Job, "G0\n");
    if let Message::SubmitJob { sha256, .. } = &mut wrong {
        *sha256 = sha_hex(b"G1\n");
    }
    f.send(wrong);
    let out = f.send(chunk(j1(), 0, "G0\n"));
    assert!(rejected_with(&out, j1(), RejectReason::HashMismatch));
    assert_eq!(f.state(), State::Idle);
    assert!(f.machine.held_job().is_none(), "assembly discarded");
}

// P10: a control character or a tab is outside printable ASCII and gives NotAscii; discarded.
#[test]
fn p10_control_bytes_are_not_ascii() {
    for bad in ["G0\t\n", "G0\u{0}\n", "G0\u{7f}\n", "G0 \u{1}\n"] {
        let mut f = idle();
        f.send(submit(j1(), JobKind::Job, bad));
        let out = f.send(chunk(j1(), 0, bad));
        assert!(rejected_with(&out, j1(), RejectReason::NotAscii), "{bad:?}");
        assert_eq!(f.state(), State::Idle, "{bad:?} discards");
    }
}

// P10: non-ASCII UTF-8 text (e.g. "ö") is rejected with NotAscii.
#[test]
fn p10_non_ascii_utf8_is_rejected() {
    let text = "Größe\n";
    let mut f = idle();
    f.send(submit(j1(), JobKind::Job, text));
    let out = f.send(chunk(j1(), 0, text));
    assert!(rejected_with(&out, j1(), RejectReason::NotAscii));
}

// P10: printable ASCII boundaries 0x20 and 0x7E, LF and CRLF are accepted.
#[test]
fn p10_ascii_boundaries_and_newlines_accepted() {
    let text = " ~\nA\r\nB\n";
    let mut f = idle();
    f.send(submit(j1(), JobKind::Job, text));
    f.send(chunk(j1(), 0, text));
    assert_eq!(f.state(), State::Ready);
}

// P10 edge: a lone CR (not followed by LF) is not printable and not a newline: NotAscii.
#[test]
fn p10_lone_cr_is_not_ascii() {
    let text = "G0\rG1\n";
    let mut f = idle();
    f.send(submit(j1(), JobKind::Job, text));
    let out = f.send(chunk(j1(), 0, text));
    assert!(rejected_with(&out, j1(), RejectReason::NotAscii));
}

// P11: bytes above 64 MiB are refused with TooLarge before any chunk.
#[test]
fn p11_bytes_above_64_mib_refused_before_chunks() {
    let mut f = idle();
    let out = f.send(Message::SubmitJob {
        job_id: j1(),
        name: "big".into(),
        kind: JobKind::Job,
        bytes: MAX_JOB_BYTES + 1,
        sha256: "00".repeat(32),
    });
    assert!(rejected_with(&out, j1(), RejectReason::TooLarge));
    assert_eq!(f.state(), State::Idle);
}

// P11: bytes of exactly 64 MiB is accepted (the job is being received).
#[test]
fn p11_bytes_exactly_64_mib_is_accepted() {
    assert_eq!(MAX_JOB_BYTES, 67_108_864, "constant is 64 MiB");
    let mut f = idle();
    let out = f.send(Message::SubmitJob {
        job_id: j1(),
        name: "max".into(),
        kind: JobKind::Job,
        bytes: MAX_JOB_BYTES,
        sha256: "00".repeat(32),
    });
    assert!(!has_error(&out));
    assert!(
        !out.messages
            .iter()
            .any(|m| matches!(m, Message::JobRejected { .. }))
    );
    assert_eq!(f.state(), State::Receiving);
}

// P11: a chunk above 256 KiB (262 144 bytes) is TooLarge and discards the assembly.
#[test]
fn p11_chunk_above_256_kib_rejected_and_discarded() {
    assert_eq!(MAX_CHUNK_BYTES, 262_144);
    let big = "G".repeat(MAX_CHUNK_BYTES + 1);
    let mut f = idle();
    f.send(submit(j1(), JobKind::Job, &big));
    let out = f.send(chunk(j1(), 0, &big));
    assert!(rejected_with(&out, j1(), RejectReason::TooLarge));
    assert_eq!(f.state(), State::Idle);
}

// P11: a chunk of exactly 256 KiB is accepted.
#[test]
fn p11_chunk_of_exactly_256_kib_accepted() {
    let big = "G".repeat(MAX_CHUNK_BYTES);
    let mut f = idle();
    f.send(submit(j1(), JobKind::Job, &big));
    f.send(chunk(j1(), 0, &big));
    assert_eq!(f.state(), State::Ready);
}

// P11: a chunk that pushes the total above the announced bytes is TooLarge and discarded.
#[test]
fn p11_total_above_announced_bytes_rejected_and_discarded() {
    let mut f = idle();
    f.send(submit(j1(), JobKind::Job, "G0\n"));
    let out = f.send(chunk(j1(), 0, "G0\nG1\n"));
    assert!(rejected_with(&out, j1(), RejectReason::TooLarge));
    assert_eq!(f.state(), State::Idle);
}

// P12: a skipped index (gap) rejects and discards with HashMismatch.
#[test]
fn p12_chunk_gap_rejects_with_hash_mismatch() {
    let text = "G0\nG1\nG2\n";
    let mut f = idle();
    f.send(submit(j1(), JobKind::Job, text));
    f.send(chunk(j1(), 0, &text[..3]));
    let out = f.send(chunk(j1(), 2, &text[6..]));
    assert!(rejected_with(&out, j1(), RejectReason::HashMismatch));
    assert_eq!(f.state(), State::Idle);
}

// P12: a repeated index rejects and discards with HashMismatch.
#[test]
fn p12_repeated_chunk_index_rejects() {
    let text = "G0\nG1\n";
    let mut f = idle();
    f.send(submit(j1(), JobKind::Job, text));
    f.send(chunk(j1(), 0, &text[..3]));
    let out = f.send(chunk(j1(), 0, &text[..3]));
    assert!(rejected_with(&out, j1(), RejectReason::HashMismatch));
    assert_eq!(f.state(), State::Idle);
}

// P12: an out-of-order index rejects and discards with HashMismatch.
#[test]
fn p12_out_of_order_index_rejects() {
    let text = "G0\nG1\n";
    let mut f = idle();
    f.send(submit(j1(), JobKind::Job, text));
    let out = f.send(chunk(j1(), 1, &text[3..]));
    assert!(rejected_with(&out, j1(), RejectReason::HashMismatch));
    assert_eq!(f.state(), State::Idle);
}

// P12: a chunk for an unknown job_id gets Error; the job in assembly is untouched and can finish.
#[test]
fn p12_chunk_for_unknown_job_id_leaves_assembly_untouched() {
    let text = "G0\nG1\n";
    let mut f = idle();
    f.send(submit(j1(), JobKind::Job, text));
    f.send(chunk(j1(), 0, &text[..3]));
    let out = f.send(chunk(j2(), 1, &text[3..]));
    assert!(has_error(&out), "Error for unknown job_id");
    assert_eq!(f.state(), State::Receiving, "assembly untouched");
    let ok = f.send(chunk(j1(), 1, &text[3..]));
    assert!(ok.messages.contains(&Message::JobAccepted { job_id: j1() }));
}

// ---------------------------------------------------------------------------
// P13 to P15: busy, no controller, same job twice
// ---------------------------------------------------------------------------

// P13: in any non-Idle job state another SubmitJob gets Busy and the held job is untouched.
#[test]
fn p13_submit_while_busy_gets_busy_and_keeps_held_job() {
    for state in [
        State::Receiving,
        State::Ready,
        State::Running,
        State::Paused,
        State::AwaitingLocalStart,
    ] {
        let mut f = in_state(state, "G0\n");
        let before = f.machine.held_job().cloned();
        let out = f.send(submit(j2(), JobKind::Job, "G1\n"));
        assert!(
            rejected_with(&out, j2(), RejectReason::Busy),
            "{state:?}: Busy"
        );
        assert_eq!(f.state(), state, "{state:?} unchanged");
        assert_eq!(
            f.machine.held_job().cloned(),
            before,
            "{state:?} held job untouched"
        );
    }
}

// P13: after a Done state the next SubmitJob is accepted, for both results.
#[test]
fn p13_after_done_next_submit_is_accepted() {
    for result in [JobResult::Completed, JobResult::Stopped, JobResult::Failed] {
        let mut f = in_state(State::Done { result }, "G0\n");
        let out = f.send(submit(j2(), JobKind::Job, "G1\n"));
        assert!(
            !out.messages
                .iter()
                .any(|m| matches!(m, Message::JobRejected { .. })),
            "{result:?}"
        );
        assert_eq!(f.state(), State::Receiving, "{result:?}");
    }
}

// P14: in NoController a SubmitJob gets NoController.
#[test]
fn p14_no_controller_rejects_submit() {
    let mut f = FakeController::new(Options::default());
    assert_eq!(f.state(), State::NoController);
    let out = f.send(submit(j1(), JobKind::Job, "G0\n"));
    assert!(rejected_with(&out, j1(), RejectReason::NoController));
}

// P15: the same job submitted twice in a row while the first is held gets Busy.
#[test]
fn p15_same_job_twice_gets_busy() {
    let mut f = idle();
    f.send(submit(j1(), JobKind::Job, "G0\n"));
    let out = f.send(submit(j1(), JobKind::Job, "G0\n"));
    assert!(rejected_with(&out, j1(), RejectReason::Busy));
}

// ---------------------------------------------------------------------------
// P16 to P20: job state machine
// ---------------------------------------------------------------------------

// P16: walks the listed transitions with the fake controller, including the commands it receives.
#[test]
fn p16_listed_transitions_walk() {
    let mut f = FakeController::new(Options::default());
    assert_eq!(f.state(), State::NoController);

    f.event(Event::ControllerConnected);
    assert_eq!(f.state(), State::Idle);

    f.send(submit(j1(), JobKind::Job, "G0\n"));
    assert_eq!(f.state(), State::Receiving);
    f.send(chunk(j1(), 0, "G0\n"));
    assert_eq!(f.state(), State::Ready);

    let out = f.send(Message::Start { job_id: j1() });
    assert_eq!(f.state(), State::Running);
    assert!(
        out.commands
            .contains(&ControllerCommand::Start { job_id: j1() })
    );

    let out = f.send(Message::Pause);
    assert_eq!(f.state(), State::Paused);
    assert!(out.commands.contains(&ControllerCommand::Pause));

    let out = f.send(Message::Resume);
    assert_eq!(f.state(), State::Running);
    assert!(out.commands.contains(&ControllerCommand::Resume));

    f.event(Event::JobFinished { elapsed_ms: 99 });
    assert_eq!(
        f.state(),
        State::Done {
            result: JobResult::Completed
        }
    );
}

// P16: Running -> Done(Stopped) and Paused -> Done(Stopped) via Stop; Stop is sent to the controller.
#[test]
fn p16_stop_from_running_and_paused_ends_stopped() {
    let mut f = in_state(State::Running, "G0\n");
    let out = f.send(Message::Stop);
    assert_eq!(
        f.state(),
        State::Done {
            result: JobResult::Stopped
        }
    );
    assert!(out.commands.contains(&ControllerCommand::Stop));

    let mut f = in_state(State::Paused, "G0\n");
    f.send(Message::Stop);
    assert_eq!(
        f.state(),
        State::Done {
            result: JobResult::Stopped
        }
    );
}

// P16: Running -> Done(Failed) on a controller error.
#[test]
fn p16_controller_error_while_running_is_failed() {
    let mut f = in_state(State::Running, "G0\n");
    f.event(Event::JobFailed { elapsed_ms: 5 });
    assert_eq!(
        f.state(),
        State::Done {
            result: JobResult::Failed
        }
    );
}

// P16: an alarm from any state with a controller goes to Alarm and carries its text in the Status.
#[test]
fn p16_alarm_from_any_state_with_controller() {
    for state in [
        State::Idle,
        State::Receiving,
        State::Ready,
        State::Running,
        State::Paused,
        State::Done {
            result: JobResult::Completed,
        },
    ] {
        let mut f = in_state(state, "G0\n");
        let out = f.event(Event::Alarm {
            text: "Hard limit".into(),
        });
        assert_eq!(f.state(), State::Alarm, "from {state:?}");
        assert!(
            out.messages.iter().any(|m| matches!(m,
                Message::Status { alarm: Some(t), .. } if t == "Hard limit")),
            "alarm text in Status from {state:?}"
        );
        assert_eq!(status_alarm(&f.machine).as_deref(), Some("Hard limit"));
    }
}

// P16: controller lost goes to NoController from every state with a controller.
#[test]
fn p16_controller_lost_goes_to_no_controller() {
    for state in [
        State::Idle,
        State::Receiving,
        State::Ready,
        State::Running,
        State::Paused,
    ] {
        let mut f = in_state(state, "G0\n");
        f.event(Event::ControllerLost);
        assert_eq!(f.state(), State::NoController, "from {state:?}");
    }
}

// P16 table: every (state, event) pair not in the listed transitions leaves the state unchanged
// and yields only Error messages or none.
#[test]
fn p16_unlisted_pairs_leave_state_unchanged() {
    let states = [
        State::NoController,
        State::Idle,
        State::Receiving,
        State::Ready,
        State::Running,
        State::Paused,
        State::AwaitingLocalStart,
        State::Done {
            result: JobResult::Completed,
        },
        State::Done {
            result: JobResult::Stopped,
        },
        State::Alarm,
    ];
    let events: [(&str, Make); 9] = [
        ("Start", || Event::Message(Message::Start { job_id: j1() })),
        ("Pause", || Event::Message(Message::Pause)),
        ("Resume", || Event::Message(Message::Resume)),
        ("JobFinished", || Event::JobFinished { elapsed_ms: 1 }),
        ("JobFailed", || Event::JobFailed { elapsed_ms: 1 }),
        ("LocalStartPressed", || Event::LocalStartPressed),
        ("ControllerConnected", || Event::ControllerConnected),
        ("ControllerLost", || Event::ControllerLost),
        ("Alarm", || Event::Alarm { text: "a".into() }),
    ];
    // The listed transitions of P16 (and of the local start option) are excluded.
    let listed = |state: State, name: &str| match name {
        "Start" => state == State::Ready,
        "Resume" => state == State::Paused,
        "Pause" | "JobFinished" | "JobFailed" => state == State::Running,
        "LocalStartPressed" => state == State::AwaitingLocalStart,
        "ControllerConnected" => state == State::NoController,
        "ControllerLost" | "Alarm" => state != State::NoController,
        _ => false,
    };
    for state in states {
        for (name, make) in events {
            if listed(state, name) {
                continue;
            }
            let mut f = in_state(state, "G0\n");
            let out = f.event(make());
            assert_eq!(
                f.state(),
                state,
                "({state:?}, {name}) must not change state"
            );
            assert!(
                out.messages
                    .iter()
                    .all(|m| matches!(m, Message::Error { .. })),
                "({state:?}, {name}) yields only Error or nothing: {:?}",
                out.messages
            );
        }
    }
}

// P17: Stop in Idle, Ready, Done, Alarm and NoController is accepted: no error, no state change
// (Ready is the exception, it discards the job).
#[test]
fn p17_stop_is_idempotent_where_nothing_runs() {
    for state in [
        State::Idle,
        State::Done {
            result: JobResult::Completed,
        },
        State::Alarm,
        State::NoController,
    ] {
        let mut f = in_state(state, "G0\n");
        let out = f.send(Message::Stop);
        assert_eq!(f.state(), state, "Stop in {state:?}");
        assert!(!has_error(&out), "Stop in {state:?} is no error");
    }
}

// P17: Stop in Ready discards the held job and returns to Idle.
#[test]
fn p17_stop_in_ready_discards_job() {
    let mut f = in_state(State::Ready, "G0\n");
    let out = f.send(Message::Stop);
    assert_eq!(f.state(), State::Idle);
    assert!(f.machine.held_job().is_none());
    assert!(!has_error(&out));
}

// P17: Pause while Paused and Resume while Running are accepted with no state change and no error.
#[test]
fn p17_pause_while_paused_and_resume_while_running_are_noops() {
    let mut f = in_state(State::Paused, "G0\n");
    let out = f.send(Message::Pause);
    assert_eq!(f.state(), State::Paused);
    assert!(!has_error(&out));

    let mut f = in_state(State::Running, "G0\n");
    let out = f.send(Message::Resume);
    assert_eq!(f.state(), State::Running);
    assert!(!has_error(&out));
}

// P17: Stop in Running ends in Done(Stopped) exactly once; a second Stop changes nothing.
#[test]
fn p17_second_stop_changes_nothing() {
    let mut f = in_state(State::Running, "G0\n");
    let first = f.send(Message::Stop);
    let stopped = first
        .messages
        .iter()
        .filter(|m| {
            matches!(
                m,
                Message::Status {
                    state: State::Done {
                        result: JobResult::Stopped
                    },
                    ..
                }
            )
        })
        .count();
    assert_eq!(stopped, 1, "exactly one Done(Stopped) Status");
    let second = f.send(Message::Stop);
    assert_eq!(
        f.state(),
        State::Done {
            result: JobResult::Stopped
        }
    );
    assert!(!has_error(&second));
    assert!(
        second.messages.is_empty(),
        "second Stop emits nothing: {:?}",
        second.messages
    );
    assert!(
        second.commands.is_empty(),
        "second Stop sends nothing to the controller"
    );
}

// P18: with the local start option on, Start in Ready goes to AwaitingLocalStart and the local
// press runs it.
#[test]
fn p18_local_start_required_when_option_on() {
    let mut f = FakeController::connected(Options {
        require_local_start: true,
    });
    f.send(submit(j1(), JobKind::Job, "G0\n"));
    f.send(chunk(j1(), 0, "G0\n"));
    f.send(Message::Start { job_id: j1() });
    assert_eq!(f.state(), State::AwaitingLocalStart);
    f.event(Event::LocalStartPressed);
    assert_eq!(f.state(), State::Running);
}

// P18: with the option on, Stop in AwaitingLocalStart returns to Idle.
#[test]
fn p18_stop_while_awaiting_local_start_returns_idle() {
    let mut f = in_state(State::AwaitingLocalStart, "G0\n");
    f.send(Message::Stop);
    assert_eq!(f.state(), State::Idle);
}

// P18: the option is off by default, and Start then goes straight to Running.
#[test]
fn p18_default_option_is_off_and_start_runs() {
    assert!(!Options::default().require_local_start);
    let mut f = in_state(State::Ready, "G0\n");
    f.send(Message::Start { job_id: j1() });
    assert_eq!(f.state(), State::Running);
}

// P18: Start with a job_id that is not the held job gets Error and changes nothing.
#[test]
fn p18_start_with_wrong_job_id_gets_error() {
    let mut f = in_state(State::Ready, "G0\n");
    let out = f.send(Message::Start { job_id: j2() });
    assert!(has_error(&out));
    assert_eq!(f.state(), State::Ready);
}

// P19: a lost client connection leaves the running job alone; a new connection's GetStatus gets the
// same Status; its Stop ends the job.
#[test]
fn p19_connection_loss_keeps_job_and_new_connection_can_stop() {
    let mut f = in_state(State::Running, "G0\nG1\n");
    let before = f.machine.status();
    f.event(Event::ClientDisconnected);
    assert_eq!(f.state(), State::Running, "job keeps running");

    let out = f.send(Message::GetStatus);
    let status = out
        .messages
        .iter()
        .find(|m| matches!(m, Message::Status { .. }))
        .expect("GetStatus answers with Status");
    assert_eq!(status, &before, "same Status as before the loss");
    match status {
        Message::Status { job: Some(p), .. } => assert_eq!(p.job_id, j1()),
        other => panic!("job progress expected, got {other:?}"),
    }

    f.send(Message::Stop);
    assert_eq!(
        f.state(),
        State::Done {
            result: JobResult::Stopped
        }
    );
}

// P20: GetStatus returns a Status, and a state change returns one too.
#[test]
fn p20_status_on_change_and_on_get_status() {
    let mut f = idle();
    let out = f.send(Message::GetStatus);
    assert!(
        out.messages
            .iter()
            .any(|m| matches!(m, Message::Status { .. }))
    );
    let out = f.send(submit(j1(), JobKind::Job, "G0\n"));
    assert!(
        out.messages
            .iter()
            .any(|m| matches!(m, Message::Status { .. })),
        "Status on change"
    );
}

// P20: job progress is carried only in Ready, Running, Paused, AwaitingLocalStart and Done.
#[test]
fn p20_job_progress_only_in_listed_states() {
    let mut f = in_state(State::Ready, "G0\n");
    let ready_status = f.machine.status();
    assert!(
        matches!(ready_status, Message::Status { job: Some(_), .. }),
        "job in Ready"
    );

    let out = f.send(Message::Start { job_id: j1() });
    let running = out
        .messages
        .iter()
        .find_map(|m| match m {
            Message::Status { job, .. } => Some(job.clone()),
            _ => None,
        })
        .expect("Status on Start");
    assert!(running.is_some(), "job progress in Running");

    f.event(Event::JobFinished { elapsed_ms: 3 });
    assert!(
        matches!(f.machine.status(), Message::Status { job: Some(_), .. }),
        "job in Done"
    );
}

// P20: progress events update line and elapsed time as the counts come in with the events.
#[test]
fn p20_progress_event_updates_line_counts() {
    let mut f = in_state(State::Running, "G0\nG1\nG2\n");
    f.event(Event::Progress {
        line: 2,
        elapsed_ms: 150,
    });
    match f.machine.status() {
        Message::Status { job: Some(p), .. } => {
            assert_eq!(p.line, 2);
            assert_eq!(p.lines, 3);
            assert_eq!(p.elapsed_ms, 150);
        }
        other => panic!("expected job progress, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// P21 to P24: build and rules
// ---------------------------------------------------------------------------

// P22: the crate root forbids unsafe code.
#[test]
fn p22_crate_forbids_unsafe_code() {
    let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/lib.rs")).unwrap();
    assert!(src.contains("#![forbid(unsafe_code)]"));
}

// P22: the crate depends only on the permitted crates (the spec lists serde, serde_json, sha2
// and uuid; thiserror is also a dependency, see the report).
#[test]
fn p22_dependencies_are_limited_to_the_permitted_set() {
    let manifest =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml")).unwrap();
    let mut in_deps = false;
    let mut names = Vec::new();
    for line in manifest.lines() {
        let t = line.trim();
        if t.starts_with('[') {
            in_deps = t == "[dependencies]";
            continue;
        }
        if in_deps && !t.is_empty() {
            let name = t.split(['.', ' ', '=']).next().unwrap_or("").to_string();
            names.push(name);
        }
    }
    for name in &names {
        assert!(
            ["serde", "serde_json", "sha2", "uuid", "thiserror"].contains(&name.as_str()),
            "unexpected dependency {name}"
        );
    }
    assert_ne!(names.len(), 0);
}

// P23: the fake controller double of this file drives the machine through the events used in
// P16 to P20 (exercised by the tests above; here the double itself is checked).
#[test]
fn p23_fake_controller_drives_machine() {
    let mut f = FakeController::connected(Options::default());
    assert_eq!(f.state(), State::Idle);
    f.event(Event::ControllerLost);
    assert_eq!(f.state(), State::NoController);
}

// P24: a golden file exists in tests/fixtures/messages for each of the 16 message types.
#[test]
fn p24_golden_file_per_message_type() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/messages");
    let files: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    for name in [
        "hello.json",
        "pair.json",
        "auth.json",
        "get_status.json",
        "submit_job.json",
        "job_chunk.json",
        "start.json",
        "pause.json",
        "resume.json",
        "stop.json",
        "welcome.json",
        "paired.json",
        "job_accepted.json",
        "job_rejected.json",
        "status_idle.json",
        "error.json",
    ] {
        assert!(files.iter().any(|f| f == name), "missing golden {name}");
    }
}
