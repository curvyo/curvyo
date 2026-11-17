//! The connector server as a client sees it: keys, the TLS rules, the auth gate, pairing codes.
#![allow(clippy::unwrap_used)]

mod common;

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::{Duration, Instant};

use common::{Rig, WAIT, config, rig, state_of};
use curvyo_connector_core::{ErrorCode, Message, State};
use curvyo_connector_io::client::{CONNECT_TIMEOUT, ClientError, connect, open_paired};
use curvyo_connector_io::frames::RefusalReason;
use curvyo_connector_io::pairing::PairError;
use curvyo_connector_io::pin_verifier::PinMode;
use curvyo_connector_io::server::RunningServer;

fn pair_with(rig: &Rig, code: &str) -> Result<(), PairError> {
    let mut connection = connect(&rig.address(), PinMode::Capture, CONNECT_TIMEOUT).unwrap();
    connection
        .pair(code, "Test app", Instant::now() + WAIT)
        .map(|_| ())
}

#[test]
fn q1_the_server_listens_and_offers_a_fingerprint_and_a_code() {
    let rig = rig(20);
    assert_eq!(rig.server.spki().fingerprint().split(' ').count(), 8);
    assert_eq!(rig.code().len(), 9);
}

#[test]
fn q2_a_new_code_replaces_a_valid_one() {
    let rig = rig(20);
    let first = rig.code();
    let second = rig.server.new_code().unwrap();
    assert_ne!(first, second);
    assert_eq!(rig.code(), second);
    assert!(matches!(
        pair_with(&rig, &first),
        Err(PairError::Refused {
            reason: RefusalReason::WrongCode,
            ..
        })
    ));
}

#[test]
fn q3_a_restart_keeps_the_key_and_a_paired_app_needs_no_code() {
    let dir = tempfile::tempdir().unwrap();
    let first = RunningServer::start(&config(dir.path().to_owned(), 20, None)).unwrap();
    let rig = Rig { server: first, dir };
    let record = rig.pair("Laptop");
    let spki = rig.server.spki();
    let Rig { server, dir } = rig;
    server.shutdown();
    let again = RunningServer::start(&config(dir.path().to_owned(), 20, None)).unwrap();
    assert_eq!(again.spki(), spki);
    assert!(
        again.current_code().is_none(),
        "an app is paired: no code at start"
    );
    let (_, status) = open_paired(
        &again.addr().to_string(),
        record.spki,
        &record.token,
        CONNECT_TIMEOUT,
    )
    .unwrap();
    assert_eq!(state_of(&status), State::Idle);
}

#[test]
fn q3_a_damaged_key_file_refuses_the_start_and_names_the_file() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("identity.pem"), "damaged").unwrap();
    let error = RunningServer::start(&config(dir.path().to_owned(), 20, None))
        .err()
        .unwrap();
    assert!(error.to_string().contains("identity.pem"), "{error}");
    assert_eq!(
        std::fs::read_to_string(dir.path().join("identity.pem")).unwrap(),
        "damaged"
    );
}

/// What a socket gets back after it sent `bytes`: nothing a protocol message could be.
fn closed_without_a_message(rig: &Rig, bytes: &[u8]) -> bool {
    let mut stream = TcpStream::connect(rig.addr()).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let _ = stream.write_all(bytes);
    let mut answer = Vec::new();
    let _ = stream.read_to_end(&mut answer);
    // At most a TLS alert record (type 21); never application data (23) or HTTP text.
    answer.is_empty() || answer[0] == 21
}

#[test]
fn q4_plain_tcp_plain_http_and_tls_12_get_no_message() {
    let rig = rig(20);
    let hello12 = std::fs::read(format!(
        "{}/tests/fixtures/tls12_client_hello.bin",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();
    assert!(closed_without_a_message(&rig, b"hello\r\n\r\n"));
    assert!(closed_without_a_message(
        &rig,
        b"GET /connector HTTP/1.1\r\nHost: x\r\nUpgrade: websocket\r\n\r\n"
    ));
    assert!(closed_without_a_message(&rig, &hello12));
    // The server is unharmed.
    let record = rig.pair("Laptop");
    let (_, status) = rig.open(&record);
    assert_eq!(state_of(&status), State::Idle);
}

#[test]
fn q9_an_unpaired_client_gets_pairing_required_and_changes_nothing() {
    let rig = rig(20);
    let mut connection = connect(&rig.address(), PinMode::Capture, CONNECT_TIMEOUT).unwrap();
    let id = uuid::Uuid::nil();
    for message in [
        Message::GetStatus,
        Message::Start { job_id: id },
        Message::Stop,
        Message::Pause,
        Message::JobChunk {
            job_id: id,
            index: 0,
            text: String::new(),
        },
        Message::SubmitJob {
            job_id: id,
            name: "x".to_owned(),
            kind: curvyo_connector_core::JobKind::Job,
            bytes: 0,
            sha256: String::new(),
        },
    ] {
        connection.send(&message).unwrap();
        let answer = connection.recv_message(Instant::now() + WAIT).unwrap();
        assert!(
            matches!(
                answer,
                Message::Error {
                    code: ErrorCode::PairingRequired,
                    ..
                }
            ),
            "{message:?} -> {answer:?}"
        );
    }
    connection
        .send(&Message::Auth {
            token: "00".repeat(32),
        })
        .unwrap();
    assert!(matches!(
        connection.recv_message(Instant::now() + WAIT).unwrap(),
        Message::Error {
            code: ErrorCode::PairingRequired,
            ..
        }
    ));
    // A paired client still sees an idle machine.
    let record = rig.pair("Laptop");
    assert_eq!(state_of(&rig.open(&record).1), State::Idle);
}

#[test]
fn q9_the_state_directory_holds_a_hash_of_the_token_and_never_the_token() {
    let rig = rig(20);
    let record = rig.pair("Laptop");
    let clients = std::fs::read_to_string(rig.dir.path().join("clients.json")).unwrap();
    assert!(clients.contains("Laptop"));
    assert!(clients.contains(&record.token.sha256_hex()));
    assert!(!clients.contains(record.token.to_hex().as_str()));
    for entry in std::fs::read_dir(rig.dir.path()).unwrap() {
        let text = std::fs::read(entry.unwrap().path()).unwrap();
        let hex = record.token.to_hex();
        assert!(!text.windows(64).any(|w| w == hex.as_bytes()));
    }
}

#[test]
fn q9_a_revoked_token_gets_pairing_required_on_the_next_connection() {
    let rig = rig(20);
    let record = rig.pair("Laptop");
    let list = curvyo_connector_io::clients::Clients::new(rig.dir.path());
    assert!(list.remove("Laptop").unwrap());
    let error = open_paired(&rig.address(), record.spki, &record.token, CONNECT_TIMEOUT)
        .err()
        .unwrap();
    assert_eq!(error, ClientError::NoLongerPaired);
}

fn a_wrong_code(code: &str) -> &'static str {
    if code.starts_with('0') {
        "1111-1111"
    } else {
        "0000-0000"
    }
}

#[test]
fn q6_three_wrong_attempts_spend_the_code_even_for_the_right_one() {
    let rig = rig(20);
    let code = rig.code();
    for left in [2, 1, 0] {
        assert_eq!(
            pair_with(&rig, a_wrong_code(&code)).unwrap_err(),
            PairError::Refused {
                reason: RefusalReason::WrongCode,
                attempts_left: left
            }
        );
    }
    assert_eq!(
        pair_with(&rig, &code).unwrap_err(),
        PairError::Refused {
            reason: RefusalReason::NoCode,
            attempts_left: 0
        }
    );
    assert!(rig.server.current_code().is_none());
}

#[test]
fn q5_the_code_works_once_and_a_typed_variant_is_read_like_the_code() {
    let rig = rig(20);
    let typed = rig.code().to_lowercase().replace('-', " ");
    pair_with(&rig, &typed).unwrap();
    assert_eq!(
        pair_with(&rig, &typed).unwrap_err(),
        PairError::Refused {
            reason: RefusalReason::NoCode,
            attempts_left: 0
        }
    );
}

#[test]
fn a_frame_between_one_and_two_mib_gets_an_error_and_the_connection_stays() {
    let rig = rig(20);
    let record = rig.pair("Laptop");
    let (mut connection, _) = rig.open(&record);
    let big = Message::JobChunk {
        job_id: uuid::Uuid::nil(),
        index: 0,
        text: "G".repeat(1024 * 1024 + 10),
    };
    connection.send(&big).unwrap();
    assert!(matches!(
        connection.recv_message(Instant::now() + WAIT).unwrap(),
        Message::Error {
            code: ErrorCode::FrameTooLarge,
            ..
        }
    ));
    connection.send(&Message::GetStatus).unwrap();
    assert!(matches!(
        connection.recv_message(Instant::now() + WAIT).unwrap(),
        Message::Status { .. }
    ));
}
