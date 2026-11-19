//! The app's standing connection while the Machine block is shown: pushes of the connector's
//! status, and a new try every few seconds after the connection drops (Q16).

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use curvyo_connector_core::{ControllerInfo, Message, State};

use crate::client::{CONNECT_TIMEOUT, ClientError, open_paired};
use crate::frames::Frame;
use crate::spki::SpkiHash;
use crate::token::Token;

/// What the watcher reports.
#[derive(Debug, Clone, PartialEq)]
pub enum WatchEvent {
    /// A `Status`, the first one right after connecting.
    Status(Message),
    /// The controller changed (identified, lost): the `Welcome` the connector sent again.
    Controller(Option<ControllerInfo>),
    /// The connection dropped; the watcher will try again.
    Lost,
    /// A try failed; the watcher will try again unless the error is final.
    Failed(ClientError),
}

fn is_final(error: &ClientError) -> bool {
    matches!(
        error,
        ClientError::KeyChanged { .. }
            | ClientError::NoLongerPaired
            | ClientError::ProtocolMismatch { .. }
    )
}

fn pause(period: Duration, stop: &AtomicBool) {
    let until = Instant::now() + period;
    while Instant::now() < until && !stop.load(Ordering::Relaxed) {
        std::thread::sleep(Duration::from_millis(20).min(period));
    }
}

/// Whether `status` is a state in which a job or a homing is in flight (`Receiving` to `Paused`,
/// `Homing`).
fn is_active(status: &Message) -> bool {
    matches!(
        status,
        Message::Status {
            state: State::Receiving
                | State::Ready
                | State::AwaitingLocalStart
                | State::Homing
                | State::Running
                | State::Paused,
            ..
        }
    )
}

/// Watches the connector at `address` until `stop` is set or a final error (changed key, revoked
/// token, other protocol) is reported. `retry` is the pause between tries. It tries again only
/// while the last status it saw had a job in flight or `send_pending` says a send awaits its
/// answer (`specs/0125` B5); otherwise a drop or a failed read is reported once and the watcher
/// ends, so an idle connector is never polled.
pub fn watch(
    address: &str,
    spki: SpkiHash,
    token: &Token,
    retry: Duration,
    stop: &AtomicBool,
    send_pending: &dyn Fn() -> bool,
    mut on_event: impl FnMut(WatchEvent),
) {
    let mut active = false;
    while !stop.load(Ordering::Relaxed) {
        match open_paired(address, spki, token, CONNECT_TIMEOUT) {
            Ok((mut connection, status)) => {
                active = is_active(&status);
                on_event(WatchEvent::Status(status));
                let ended = loop {
                    if stop.load(Ordering::Relaxed) {
                        return;
                    }
                    match connection.try_recv(Instant::now() + Duration::from_millis(100)) {
                        Ok(Some(Frame::Core(status @ Message::Status { .. }))) => {
                            active = is_active(&status);
                            on_event(WatchEvent::Status(status));
                        }
                        Ok(Some(Frame::Core(Message::Welcome { controller, .. }))) => {
                            on_event(WatchEvent::Controller(controller));
                        }
                        Ok(_) => {}
                        Err(e) => break e,
                    }
                };
                let _ = ended;
                on_event(WatchEvent::Lost);
            }
            Err(e) => {
                let end = is_final(&e);
                on_event(WatchEvent::Failed(e));
                if end {
                    return;
                }
            }
        }
        if !(active || send_pending()) {
            return;
        }
        pause(retry, stop);
    }
}
