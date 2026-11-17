//! Trust on the app side: a changed key, a man in the middle, a connector that is not there, that
//! does not answer, or that speaks another protocol.
#![allow(clippy::unwrap_used)]

mod common;

use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::time::{Duration, Instant};

use common::{WAIT, rig};
use curvyo_connector_core::Message;
use curvyo_connector_io::client::{CONNECT_TIMEOUT, ClientError, connect, open_paired};
use curvyo_connector_io::frames::{Frame, PATH, RefusalReason};
use curvyo_connector_io::identity::{self, Identity};
use curvyo_connector_io::pairing::PairError;
use curvyo_connector_io::pin_verifier::PinMode;
use curvyo_connector_io::server_tls::server_config;
use rustls::{ServerConnection, StreamOwned};
use tungstenite::WebSocket;

type Server = WebSocket<StreamOwned<ServerConnection, TcpStream>>;

fn accept_one(listener: &TcpListener, identity: &Identity) -> Server {
    let (stream, _) = listener.accept().unwrap();
    stream
        .set_read_timeout(Some(Duration::from_millis(50)))
        .unwrap();
    let connection = ServerConnection::new(server_config(identity).unwrap()).unwrap();
    let mut attempt = tungstenite::accept(StreamOwned::new(connection, stream));
    loop {
        match attempt {
            Ok(socket) => return socket,
            Err(tungstenite::HandshakeError::Interrupted(mid)) => attempt = mid.handshake(),
            Err(e) => panic!("{e}"),
        }
    }
}

fn send(socket: &mut Server, message: &Message) {
    let text = curvyo_connector_io::frames::encode(&Frame::Core(message.clone())).unwrap();
    socket.send(tungstenite::Message::text(text)).unwrap();
}

#[test]
fn q8_a_changed_key_is_refused_before_any_token_is_sent_and_only_a_new_pairing_accepts_it() {
    let old = rig(20);
    let record = old.pair("Laptop");
    let new = rig(20);
    assert_ne!(old.server.spki(), new.server.spki());
    let error = open_paired(&new.address(), record.spki, &record.token, CONNECT_TIMEOUT)
        .err()
        .unwrap();
    assert_eq!(
        error,
        ClientError::KeyChanged {
            stored: old.server.spki(),
            presented: new.server.spki()
        }
    );
    // Pairing anew with the new code is the way on.
    let fresh = new.pair("Laptop");
    assert_eq!(fresh.spki, new.server.spki());
}

#[test]
fn q7_a_man_in_the_middle_is_one_wrong_attempt_and_nothing_is_stored() {
    let connector = rig(20);
    let code = connector.code();
    let proxy_dir = tempfile::tempdir().unwrap();
    let identity = identity::load_or_create(proxy_dir.path()).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let proxy_addr = listener.local_addr().unwrap();
    let upstream = connector.address();
    let relay = std::thread::spawn(move || {
        let mut app = accept_one(&listener, &identity);
        send(
            &mut app,
            &Message::Hello {
                protocol: 0,
                software: "relay".into(),
                version: "0".into(),
            },
        );
        let _hello = app.read().unwrap();
        let mut up = connect(&upstream, PinMode::Capture, CONNECT_TIMEOUT).unwrap();
        send(
            &mut app,
            &Message::Welcome {
                connector_name: up.greeting.name.clone(),
                controller: None,
            },
        );
        loop {
            match app.read() {
                Ok(tungstenite::Message::Text(text)) => {
                    let frame = curvyo_connector_io::frames::decode(text.as_str()).unwrap();
                    up.send_frame(&frame).unwrap();
                    if let Some(answer) = up
                        .try_recv(Instant::now() + Duration::from_secs(2))
                        .unwrap()
                    {
                        let text = curvyo_connector_io::frames::encode(&answer).unwrap();
                        app.send(tungstenite::Message::text(text)).unwrap();
                    }
                }
                Ok(_) => {}
                Err(tungstenite::Error::Io(e)) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(_) => return,
            }
        }
    });
    let mut app = connect(&proxy_addr.to_string(), PinMode::Capture, CONNECT_TIMEOUT).unwrap();
    let error = app
        .pair(&code, "Laptop", Instant::now() + WAIT)
        .unwrap_err();
    assert_eq!(
        error,
        PairError::Refused {
            reason: RefusalReason::WrongCode,
            attempts_left: 2
        }
    );
    drop(app);
    relay.join().unwrap();
    // The connector counted one attempt: a direct wrong code now has one fewer left.
    let wrong = if code.starts_with('0') {
        "1111-1111"
    } else {
        "0000-0000"
    };
    let mut direct = connect(&connector.address(), PinMode::Capture, CONNECT_TIMEOUT).unwrap();
    assert_eq!(
        direct
            .pair(wrong, "Laptop", Instant::now() + WAIT)
            .unwrap_err(),
        PairError::Refused {
            reason: RefusalReason::WrongCode,
            attempts_left: 1
        }
    );
    assert!(
        std::fs::read_to_string(connector.dir.path().join("clients.json")).is_err(),
        "no client is stored"
    );
}

#[test]
fn q11_the_test_reads_the_controller_protocol_and_version() {
    let rig = rig(20);
    let record = rig.pair("Laptop");
    let (connection, status) = rig.open(&record);
    assert_eq!(connection.greeting.protocol, 0);
    assert_eq!(connection.greeting.version, env!("CARGO_PKG_VERSION"));
    assert_eq!(connection.greeting.name, "Test connector");
    assert_eq!(
        connection.greeting.controller.unwrap().name,
        "Fake controller"
    );
    assert!(matches!(status, Message::Status { .. }));
}

#[test]
fn q11_nothing_answers_at_a_closed_port() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap().to_string();
    drop(listener);
    let error = connect(&address, PinMode::Capture, CONNECT_TIMEOUT)
        .err()
        .unwrap();
    assert_eq!(error, ClientError::Unreachable(address));
}

#[test]
fn q11_a_connector_that_never_answers_times_out() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap().to_string();
    let error = connect(&address, PinMode::Capture, Duration::from_millis(300))
        .err()
        .unwrap();
    assert_eq!(error, ClientError::TimedOut);
    drop(listener);
}

#[test]
fn q11_another_protocol_version_is_named() {
    let dir = tempfile::tempdir().unwrap();
    let identity = identity::load_or_create(dir.path()).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap().to_string();
    let server = std::thread::spawn(move || {
        let mut socket = accept_one(&listener, &identity);
        send(
            &mut socket,
            &Message::Hello {
                protocol: 7,
                software: "future".into(),
                version: "9".into(),
            },
        );
        let _ = socket.read();
    });
    let error = connect(&address, PinMode::Capture, CONNECT_TIMEOUT)
        .err()
        .unwrap();
    assert_eq!(error, ClientError::ProtocolMismatch { connector: 7 });
    server.join().unwrap();
}

#[test]
fn another_path_than_the_connector_path_gets_http_404() {
    let rig = rig(20);
    let verifier = curvyo_connector_io::pin_verifier::PinVerifier::new(PinMode::Capture);
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let config = rustls::ClientConfig::builder_with_provider(provider)
        .with_protocol_versions(&[&rustls::version::TLS13])
        .unwrap()
        .dangerous()
        .with_custom_certificate_verifier(verifier)
        .with_no_client_auth();
    let stream = TcpStream::connect(rig.addr()).unwrap();
    let tls = rustls::ClientConnection::new(Arc::new(config), "x".try_into().unwrap()).unwrap();
    let result = tungstenite::client(
        format!("wss://{}{}x", rig.address(), PATH),
        StreamOwned::new(tls, stream),
    );
    match result {
        Err(tungstenite::HandshakeError::Failure(tungstenite::Error::Http(response))) => {
            assert_eq!(response.status(), 404);
        }
        other => panic!("{:?}", other.err()),
    }
}
