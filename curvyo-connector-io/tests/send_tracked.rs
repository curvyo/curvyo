//! Sending a job and following it by its id: accepted, refused, a connection that dies before the
//! answer (resolved by a fresh status), a transfer cut mid-way (`specs/0125` S11, S19) and the
//! watcher that ends on an idle connector (B5). Waiting is by status, never by sleep.
#![allow(clippy::unwrap_used)]

mod common;

use std::sync::atomic::AtomicBool;
use std::sync::{Mutex, mpsc};
use std::time::{Duration, Instant};

use common::{JOB, WAIT, rig, state_of};
use curvyo_connector_core::{
    JobKind, MAX_CHUNK_BYTES, Message, RejectReason, Resolution, SendWatch, State,
};
use curvyo_connector_io::client::{CONNECT_TIMEOUT, ClientError, Connection, open_paired};
use curvyo_connector_io::job_send::new_job_id;
use curvyo_connector_io::record::PairingRecord;
use curvyo_connector_io::send_tracked::{SendEnd, SharedWatch, observe_shared, send_tracked};
use curvyo_connector_io::token::Token;
use curvyo_connector_io::watch::{WatchEvent, watch};

type Opened = Result<(Connection, Message), ClientError>;

fn deadline() -> Instant {
    Instant::now() + WAIT
}

fn shared(id: uuid::Uuid) -> SharedWatch {
    Mutex::new(Some(SendWatch::new(id)))
}

fn opener<'a>(address: &'a str, record: &'a PairingRecord) -> impl FnMut() -> Opened + 'a {
    move || open_paired(address, record.spki, &record.token, CONNECT_TIMEOUT)
}

#[test]
fn an_accepted_send_resolves_once_and_the_connector_holds_the_job() {
    let rig = rig(20);
    let record = rig.pair("Laptop");
    let address = rig.address();
    let id = new_job_id().unwrap();
    let watch = shared(id);
    let end = send_tracked(
        &mut opener(&address, &record),
        &watch,
        "page",
        JOB,
        deadline(),
    )
    .unwrap();
    assert_eq!(end, SendEnd::Resolved(Resolution::Accepted));
    assert!(watch.lock().unwrap().is_none(), "taken by the answer");
    let (_, status) = rig.open(&record);
    assert_eq!(state_of(&status), State::Ready);
    assert_eq!(
        observe_shared(&watch, &Message::JobAccepted { job_id: id }),
        None,
        "a replayed answer commits nothing again"
    );
}

#[test]
fn a_second_send_is_refused_busy_and_the_held_job_stays() {
    let rig = rig(20);
    let record = rig.pair("Laptop");
    let address = rig.address();
    let first = shared(new_job_id().unwrap());
    send_tracked(
        &mut opener(&address, &record),
        &first,
        "one",
        JOB,
        deadline(),
    )
    .unwrap();
    let second = shared(new_job_id().unwrap());
    let end = send_tracked(
        &mut opener(&address, &record),
        &second,
        "two",
        JOB,
        deadline(),
    )
    .unwrap();
    assert_eq!(
        end,
        SendEnd::Resolved(Resolution::Rejected(RejectReason::Busy))
    );
    let (_, status) = rig.open(&record);
    assert_eq!(state_of(&status), State::Ready);
}

#[test]
fn a_connection_that_died_resolves_by_a_fresh_status_accepted_when_the_job_is_held() {
    let rig = rig(20);
    let record = rig.pair("Laptop");
    let address = rig.address();
    let id = new_job_id().unwrap();
    // The connector holds the job; the app never saw the answer.
    let (mut sender, _) = rig.open(&record);
    sender
        .send_job_with_id(id, "page", JOB, deadline())
        .unwrap();
    drop(sender);
    let watch = shared(id);
    let mut dead = true;
    let mut open = || -> Opened {
        let (mut connection, status) =
            open_paired(&address, record.spki, &record.token, CONNECT_TIMEOUT)?;
        if std::mem::take(&mut dead) {
            connection.close();
        }
        Ok((connection, status))
    };
    let end = send_tracked(&mut open, &watch, "page", JOB, deadline()).unwrap();
    assert_eq!(end, SendEnd::Resolved(Resolution::Accepted));
}

#[test]
fn a_connection_that_died_before_anything_arrived_is_not_received() {
    let rig = rig(20);
    let record = rig.pair("Laptop");
    let address = rig.address();
    let watch = shared(new_job_id().unwrap());
    let mut dead = true;
    let mut open = || -> Opened {
        let (mut connection, status) =
            open_paired(&address, record.spki, &record.token, CONNECT_TIMEOUT)?;
        if std::mem::take(&mut dead) {
            connection.close();
        }
        Ok((connection, status))
    };
    let end = send_tracked(&mut open, &watch, "page", JOB, deadline()).unwrap();
    assert_eq!(end, SendEnd::Resolved(Resolution::NotReceived));
}

#[test]
fn a_dead_connector_after_the_cut_leaves_the_send_unresolved_for_the_watcher() {
    let rig = rig(20);
    let record = rig.pair("Laptop");
    let address = rig.address();
    let id = new_job_id().unwrap();
    let watch = shared(id);
    let mut tries = 0;
    let mut open = || -> Opened {
        tries += 1;
        if tries > 1 {
            return Err(ClientError::Unreachable(address.clone()));
        }
        let (mut connection, status) =
            open_paired(&address, record.spki, &record.token, CONNECT_TIMEOUT)?;
        connection.close();
        Ok((connection, status))
    };
    let end = send_tracked(&mut open, &watch, "page", JOB, deadline()).unwrap();
    assert!(matches!(end, SendEnd::Unresolved(_)), "{end:?}");
    // The watcher's later status decides, once.
    let status = Message::Status {
        state: State::Idle,
        job: None,
        position: None,
        alarm: None,
        detail: None,
        pause_pending: false,
    };
    assert_eq!(
        observe_shared(&watch, &status),
        Some(Resolution::NotReceived)
    );
    assert_eq!(observe_shared(&watch, &status), None);
}

#[test]
fn s19_a_transfer_cut_mid_way_does_not_leave_the_connector_busy() {
    let rig = rig(20);
    let record = rig.pair("Laptop");
    let address = rig.address();
    // The app announces a job of two chunks, sends one and dies.
    let big = "G1 X1 Y1\n".repeat(MAX_CHUNK_BYTES / 9 + 10);
    let (mut cut, _) = rig.open(&record);
    let cut_id = new_job_id().unwrap();
    cut.send(&Message::SubmitJob {
        job_id: cut_id,
        name: "cut".to_owned(),
        kind: JobKind::Job,
        bytes: big.len() as u64,
        sha256: "0".repeat(64),
    })
    .unwrap();
    cut.send(&Message::JobChunk {
        job_id: cut_id,
        index: 0,
        text: big[..MAX_CHUNK_BYTES].to_owned(),
    })
    .unwrap();
    drop(cut);
    // The retry is accepted, not answered "Connector busy".
    let watch = shared(new_job_id().unwrap());
    let end = send_tracked(
        &mut opener(&address, &record),
        &watch,
        "page",
        JOB,
        deadline(),
    )
    .unwrap();
    assert_eq!(end, SendEnd::Resolved(Resolution::Accepted));
    let (_, status) = rig.open(&record);
    assert_eq!(state_of(&status), State::Ready);
}

#[test]
fn b5_an_idle_watcher_reports_the_loss_once_and_ends() {
    let rig = rig(20);
    let record = rig.pair("Laptop");
    let address = rig.address();
    let stop = AtomicBool::new(false);
    let (events, seen) = mpsc::channel();
    let token = Token::from_hex(&record.token.to_hex()).unwrap();
    let rig_server = rig.server;
    std::thread::scope(|scope| {
        let handle = scope.spawn(|| {
            watch(
                &address,
                record.spki,
                &token,
                Duration::from_millis(20),
                &stop,
                &|| false,
                |event| {
                    let _ = events.send(event);
                },
            );
        });
        assert!(matches!(
            seen.recv_timeout(WAIT).unwrap(),
            WatchEvent::Status(_)
        ));
        rig_server.shutdown();
        assert!(matches!(seen.recv_timeout(WAIT).unwrap(), WatchEvent::Lost));
        handle.join().unwrap();
    });
}
