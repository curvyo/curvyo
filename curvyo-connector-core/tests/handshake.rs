//! P6 and P7: `Hello` first, the version check and the close outcome.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]

mod common;

use common::fixture;
use curvyo_connector_core::{Handshake, Message, Outcome, PROTOCOL_VERSION, encode};

fn hello(protocol: u32) -> Message {
    Message::Hello {
        protocol,
        software: "curvyo-app".to_owned(),
        version: "0.1.0".to_owned(),
    }
}

fn close_text(outcome: &Outcome) -> String {
    match outcome {
        Outcome::Close(message) => encode(message).unwrap(),
        other => panic!("expected Close, got {other:?}"),
    }
}

#[test]
fn the_constant_is_zero_and_matching_hello_passes() {
    assert_eq!(PROTOCOL_VERSION, 0);
    let mut handshake = Handshake::new();
    assert_eq!(handshake.on_message(&hello(0)), Outcome::Hello);
    assert_eq!(handshake.on_message(&Message::GetStatus), Outcome::Pass);
}

#[test]
fn a_different_version_closes_and_nothing_is_accepted_after() {
    for protocol in [1, 7, u32::MAX] {
        let mut handshake = Handshake::new();
        let outcome = handshake.on_message(&hello(protocol));
        assert_eq!(
            close_text(&outcome),
            fixture("errors/unsupported_protocol.json")
        );
        assert_eq!(handshake.on_message(&hello(0)), Outcome::Closed);
        assert_eq!(handshake.on_message(&Message::GetStatus), Outcome::Closed);
    }
}

#[test]
fn another_first_message_closes() {
    for first in [
        Message::GetStatus,
        Message::Stop,
        Message::Auth {
            token: "t".to_owned(),
        },
    ] {
        let mut handshake = Handshake::new();
        let outcome = handshake.on_message(&first);
        assert_eq!(close_text(&outcome), fixture("errors/hello_required.json"));
        assert_eq!(handshake.on_message(&hello(0)), Outcome::Closed);
    }
}
