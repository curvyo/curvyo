//! Sends one job and follows it until the connector has accepted or lost it, also across a
//! dropped connection (`specs/0125` S11, S19; architect addendum B4).

use std::sync::{Mutex, PoisonError};
use std::time::Instant;

use curvyo_connector_core::{Message, Resolution, SendWatch};

use crate::client::{ClientError, Connection};
use crate::job_send::SendError;

/// A watch shared between the send and the status watcher; `None` once one of them took the
/// answer.
pub type SharedWatch = Mutex<Option<SendWatch>>;

/// How a tracked send ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SendEnd {
    /// This call got the answer.
    Resolved(Resolution),
    /// The watcher got the answer first.
    ResolvedElsewhere,
    /// The connection dropped and the connector could not be asked again; the watcher resolves
    /// the send later.
    Unresolved(ClientError),
}

/// Feeds `message` to the shared watch; the one caller that gets the answer takes the watch.
pub fn observe_shared(watch: &SharedWatch, message: &Message) -> Option<Resolution> {
    let mut guard = watch.lock().unwrap_or_else(PoisonError::into_inner);
    let resolution = guard.as_mut()?.observe(message)?;
    *guard = None;
    Some(resolution)
}

/// The job id the shared watch follows, `None` once it is resolved.
#[must_use]
pub fn watched_job(watch: &SharedWatch) -> Option<uuid::Uuid> {
    watch
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .as_ref()
        .map(SendWatch::job_id)
}

fn end_of(resolution: Option<Resolution>) -> SendEnd {
    resolution.map_or(SendEnd::ResolvedElsewhere, SendEnd::Resolved)
}

/// Opens a connection with `open`, sends `text` as the job `watch` follows and reports how it
/// ended. A connection that drops after the announcement is replaced once by a fresh one, whose
/// first `Status` decides (`Ready` or later with the job id: accepted; anything else: not
/// received).
///
/// # Errors
/// The error of `open` for the first connection: nothing was sent.
pub fn send_tracked(
    open: &mut dyn FnMut() -> Result<(Connection, Message), ClientError>,
    watch: &SharedWatch,
    name: &str,
    text: &str,
    deadline: Instant,
) -> Result<SendEnd, ClientError> {
    let Some(job_id) = watched_job(watch) else {
        return Ok(SendEnd::ResolvedElsewhere);
    };
    let (mut connection, _) = open()?;
    match connection.send_job_with_id(job_id, name, text, deadline) {
        Ok(_) => Ok(end_of(observe_shared(
            watch,
            &Message::JobAccepted { job_id },
        ))),
        Err(SendError::Rejected(reason)) => Ok(end_of(observe_shared(
            watch,
            &Message::JobRejected { job_id, reason },
        ))),
        Err(SendError::Client(error)) => {
            drop(connection);
            if let Some(held) = watch
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .as_mut()
            {
                held.transfer_ended();
            }
            Ok(match open() {
                Ok((_, status)) => end_of(observe_shared(watch, &status)),
                Err(_) => SendEnd::Unresolved(error),
            })
        }
    }
}
