//! `0064` Q21: the demo path without the UI. A connector on `127.0.0.1:0` with the fake controller,
//! a client that pairs, tests, sends, starts, pauses, resumes and stops, a watcher that survives a
//! restart of the connector, a second client that reads and stops a running job, and a revoke.
//! Waiting is by status and line counts; nothing is decided by a sleep.
#![allow(clippy::unwrap_used)]

mod common;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::channel;
use std::time::{Duration, Instant};

use common::{JOB, WAIT, config, line_of, state_of, wait_for, wait_state};
use curvyo_connector_core::{JobResult, Message, State};
use curvyo_connector_io::client::{CONNECT_TIMEOUT, ClientError, connect, open_paired};
use curvyo_connector_io::clients::Clients;
use curvyo_connector_io::pin_verifier::PinMode;
use curvyo_connector_io::server::RunningServer;
use curvyo_connector_io::token::Token;
use curvyo_connector_io::watch::{WatchEvent, watch};

fn deadline() -> Instant {
    Instant::now() + WAIT
}

#[test]
#[allow(clippy::too_many_lines)] // one story from pairing to revoke
fn the_demo_path_from_pairing_to_revoke() {
    let dir = tempfile::tempdir().unwrap();
    let server = RunningServer::start(&config(dir.path().to_owned(), 1000, None)).unwrap();
    let address = server.addr().to_string();

    // Pair: the first contact pins the key the code authenticated.
    let mut first = connect(&address, PinMode::Capture, CONNECT_TIMEOUT).unwrap();
    let code = server.current_code().unwrap();
    let record = first.pair(&code, "Studio laptop", deadline()).unwrap();
    assert_eq!(record.spki, server.spki());
    assert_eq!(record.connector_name, "Test connector");
    drop(first);

    // Test: Hello, Welcome with the controller, Auth, the first Status.
    let (mut connection, status) =
        open_paired(&address, record.spki, &record.token, CONNECT_TIMEOUT).unwrap();
    assert_eq!(
        connection.greeting.controller.as_ref().unwrap().name,
        "Fake controller"
    );
    assert_eq!(state_of(&status), State::Idle);

    // Send, start, pause, resume, stop.
    let text = JOB.repeat(100_000);
    let job = connection
        .send_job("demo.gcode", &text, deadline())
        .unwrap();
    assert_eq!(job.lines, 600_000);
    wait_state(&mut connection, State::Ready);
    connection
        .send(&Message::Start { job_id: job.job_id })
        .unwrap();
    wait_state(&mut connection, State::Running);
    let moving = wait_for(&mut connection, |s| line_of(s) > 0);
    connection.send(&Message::Pause).unwrap();
    let paused = wait_state(&mut connection, State::Paused);
    assert!(line_of(&paused) >= line_of(&moving));
    connection.send(&Message::Resume).unwrap();
    wait_state(&mut connection, State::Running);
    let before = line_of(&paused);
    wait_for(&mut connection, |s| line_of(s) > before);

    // Reconnect: the app side dies, a watcher and a second client find the job running.
    drop(connection);
    let stop = Arc::new(AtomicBool::new(false));
    let (events, seen) = channel();
    let watcher = {
        let (stop, address, token) = (
            Arc::clone(&stop),
            address.clone(),
            Token::from_hex(&record.token.to_hex()).unwrap(),
        );
        let spki = record.spki;
        std::thread::spawn(move || {
            watch(
                &address,
                spki,
                &token,
                Duration::from_millis(20),
                &stop,
                &|| true,
                |e| {
                    let _ = events.send(e);
                },
            );
        })
    };
    let first_status = loop {
        if let WatchEvent::Status(s) = seen.recv_timeout(WAIT).unwrap() {
            break s;
        }
    };
    assert_eq!(state_of(&first_status), State::Running);
    let (mut second, running) =
        open_paired(&address, record.spki, &record.token, CONNECT_TIMEOUT).unwrap();
    assert_eq!(state_of(&running), State::Running);
    second.send(&Message::Stop).unwrap();
    wait_state(
        &mut second,
        State::Done {
            result: JobResult::Stopped,
        },
    );
    drop(second);

    // The connector restarts on the same state directory and port: the watcher reports the loss,
    // then reads the new connector's status with no pairing again.
    let port = server.addr();
    server.shutdown();
    loop {
        if matches!(seen.recv_timeout(WAIT).unwrap(), WatchEvent::Lost) {
            break;
        }
    }
    let mut again = config(dir.path().to_owned(), 1000, None);
    again.listen = port;
    let server = RunningServer::start(&again).unwrap();
    loop {
        if let WatchEvent::Status(s) = seen.recv_timeout(WAIT).unwrap() {
            assert_eq!(state_of(&s), State::Idle);
            break;
        }
    }

    // Revoke: the next connection is refused, and the watcher ends with the final error.
    assert!(Clients::new(dir.path()).remove("Studio laptop").unwrap());
    let refused = open_paired(&address, record.spki, &record.token, CONNECT_TIMEOUT)
        .err()
        .unwrap();
    assert_eq!(refused, ClientError::NoLongerPaired);
    stop.store(true, Ordering::Relaxed);
    watcher.join().unwrap();
    drop(server);

    let log = std::fs::read_to_string(dir.path().join("jobs.log")).unwrap();
    assert!(log.contains("\tdemo.gcode\tStopped\n"), "{log}");
}
