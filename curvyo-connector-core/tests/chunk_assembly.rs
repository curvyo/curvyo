//! P8 to P12: assembling a job from chunks, the hash, ASCII and limits.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]

mod common;

use common::Rig;
use common::{JOB_TEXT, chunk, fixture, job_id, sha256_hex, submit};
use curvyo_connector_core::{
    JobKind, MAX_CHUNK_BYTES, MAX_JOB_BYTES, Message, RejectReason, State,
};

fn rejected(out: &curvyo_connector_core::Output) -> Option<RejectReason> {
    out.messages.iter().find_map(|m| match m {
        Message::JobRejected { reason, .. } => Some(*reason),
        _ => None,
    })
}

fn accepted(out: &curvyo_connector_core::Output) -> bool {
    out.messages.contains(&Message::JobAccepted {
        job_id: job_id(),
        position: None,
    })
}

fn announce(rig: &mut Rig, text: &str) {
    rig.app(submit(text, JobKind::Job));
    assert_eq!(rig.machine.state(), State::Receiving);
}

#[test]
fn chunks_assemble_to_the_original_and_the_job_is_ready() {
    // A frame assembles in its own slot (`frame_0118.rs`), not as the held job.
    for kind in [JobKind::Job] {
        let mut rig = Rig::plugged();
        rig.app(submit(JOB_TEXT, kind));
        let mut last = None;
        for (i, piece) in JOB_TEXT.as_bytes().chunks(10).enumerate() {
            let text = std::str::from_utf8(piece).unwrap();
            last = Some(rig.app(chunk(i.try_into().unwrap(), text)));
        }
        assert!(accepted(&last.unwrap()));
        assert_eq!(rig.machine.state(), State::Ready);
        let job = rig.machine.held_job().unwrap();
        assert_eq!(job.text.as_ref(), JOB_TEXT);
        assert_eq!(job.kind, kind);
        assert_eq!(sha256_hex(&job.text), sha256_hex(JOB_TEXT));
    }
}

#[test]
fn a_wrong_hash_is_rejected_and_discarded() {
    let mut rig = Rig::plugged();
    let mut message = submit(JOB_TEXT, JobKind::Job);
    if let Message::SubmitJob { sha256, .. } = &mut message {
        *sha256 = sha256_hex("other");
    }
    rig.app(message);
    let out = rig.app(chunk(0, JOB_TEXT));
    assert_eq!(rejected(&out), Some(RejectReason::HashMismatch));
    assert_eq!(rig.machine.state(), State::Idle);
    assert!(rig.machine.held_job().is_none());
}

#[test]
fn bytes_outside_printable_ascii_are_rejected_but_newlines_pass() {
    for bad in [
        "G1 X1\u{e4}\n",
        "G1\tX1\n",
        "G1 X1\r",
        "G1 X1\rG0\n",
        "G1\u{7f}\n",
        "G1\u{0}\n",
    ] {
        let mut rig = Rig::plugged();
        announce(&mut rig, bad);
        let out = rig.app(chunk(0, bad));
        assert_eq!(rejected(&out), Some(RejectReason::NotAscii), "{bad:?}");
        assert_eq!(rig.machine.state(), State::Idle);
    }
    let mut rig = Rig::plugged();
    let crlf = "G21\r\nG90\r\n";
    announce(&mut rig, crlf);
    assert!(accepted(&rig.app(chunk(0, crlf))));
}

#[test]
fn a_crlf_split_between_two_chunks_is_still_a_newline() {
    let mut rig = Rig::plugged();
    announce(&mut rig, "G21\r\nG90\n");
    rig.app(chunk(0, "G21\r"));
    assert!(accepted(&rig.app(chunk(1, "\nG90\n"))));
}

#[test]
fn sizes_above_the_limits_are_rejected_and_the_limits_themselves_pass() {
    let mut rig = Rig::plugged();
    let mut big = submit(JOB_TEXT, JobKind::Job);
    if let Message::SubmitJob { bytes, .. } = &mut big {
        *bytes = MAX_JOB_BYTES + 1;
    }
    let out = rig.app(big);
    assert_eq!(rejected(&out), Some(RejectReason::TooLarge));
    assert_eq!(rig.machine.state(), State::Idle);

    let mut exact = submit(JOB_TEXT, JobKind::Job);
    if let Message::SubmitJob { bytes, .. } = &mut exact {
        *bytes = MAX_JOB_BYTES;
    }
    rig.app(exact);
    assert_eq!(rig.machine.state(), State::Receiving);

    // a chunk above 256 KiB
    let mut rig = Rig::plugged();
    let text = "a".repeat(MAX_CHUNK_BYTES + 1);
    announce(&mut rig, &text);
    assert_eq!(
        rejected(&rig.app(chunk(0, &text))),
        Some(RejectReason::TooLarge)
    );
    assert_eq!(rig.machine.state(), State::Idle);

    // a chunk of exactly 256 KiB is taken
    let mut rig = Rig::plugged();
    let text = "a".repeat(MAX_CHUNK_BYTES);
    announce(&mut rig, &text);
    assert!(accepted(&rig.app(chunk(0, &text))));

    // more text than announced
    let mut rig = Rig::plugged();
    announce(&mut rig, "G21\n");
    assert_eq!(
        rejected(&rig.app(chunk(0, "G21\nG90\n"))),
        Some(RejectReason::TooLarge)
    );
    assert_eq!(rig.machine.state(), State::Idle);
}

#[test]
fn an_empty_job_with_the_empty_hash_is_accepted_at_once() {
    let mut rig = Rig::plugged();
    assert!(accepted(&rig.app(submit("", JobKind::Job))));
    assert_eq!(rig.machine.state(), State::Ready);
}

fn other_job_chunk() -> Message {
    Message::JobChunk {
        job_id: "00000000-0000-4000-8000-000000000001".parse().unwrap(),
        index: 0,
        text: "G21\n".to_owned(),
    }
}

/// The chosen mapping of P12, written out as the golden file `chunk_failures.txt`.
#[test]
fn chunk_failures_follow_the_golden_mapping() {
    let mut lines = Vec::new();
    let mut case = |name: &str, setup: &dyn Fn(&mut Rig), send: Message| {
        let mut rig = Rig::plugged();
        setup(&mut rig);
        let out = rig.app(send);
        let reply = if let Some(reason) = rejected(&out) {
            format!("JobRejected {reason:?}")
        } else if let Some(Message::Error { code, .. }) = out
            .messages
            .iter()
            .find(|m| matches!(m, Message::Error { .. }))
        {
            format!("Error {code:?}")
        } else {
            "nothing".to_owned()
        };
        lines.push(format!(
            "{name} -> {reply}, state {:?}",
            rig.machine.state()
        ));
    };
    let text = "G21\nG90\n";
    let begin = |rig: &mut Rig| {
        rig.app(submit(text, JobKind::Job));
    };
    let one = |rig: &mut Rig| {
        begin(rig);
        rig.app(chunk(0, "G21\n"));
    };
    case("chunk with a gap in the index", &begin, chunk(1, "G21\n"));
    case("chunk repeating an index", &one, chunk(0, "G90\n"));
    case("chunk for another job_id", &begin, other_job_chunk());
    case("chunk with no job in assembly", &|_| {}, chunk(0, "G21\n"));
    case(
        "chunk above 262144 bytes",
        &begin,
        chunk(0, &"a".repeat(MAX_CHUNK_BYTES + 1)),
    );
    case("chunks above announced bytes", &one, chunk(1, "G90\nG91\n"));
    case(
        "byte outside printable ASCII",
        &begin,
        chunk(0, "G21\u{e4}\n"),
    );
    case(
        "assembled text with another hash",
        &begin,
        chunk(0, "G21\nG91\n"),
    );
    let expected = fixture("chunk_failures.txt");
    assert_eq!(format!("{}\n", lines.join("\n")), expected);
}

#[test]
fn a_foreign_chunk_leaves_the_job_in_assembly_untouched() {
    let mut rig = Rig::plugged();
    announce(&mut rig, JOB_TEXT);
    rig.app(other_job_chunk());
    assert_eq!(rig.machine.state(), State::Receiving);
    let (a, b) = JOB_TEXT.split_at(20);
    rig.app(chunk(0, a));
    assert!(accepted(&rig.app(chunk(1, b))));
}
