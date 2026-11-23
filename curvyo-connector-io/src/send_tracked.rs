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
    send_tracked_open(open, watch, name, text, deadline).map(|(end, _)| end)
}

/// [`send_tracked`], also handing back the connection that got `JobAccepted`, still open, so the
/// caller can send `Start` for the job on it (`specs/0134` addendum S2); `None` on every other
/// path, also when the acceptance was read from a fresh connection's status.
///
/// # Errors
/// As [`send_tracked`].
pub fn send_tracked_open(
    open: &mut dyn FnMut() -> Result<(Connection, Message), ClientError>,
    watch: &SharedWatch,
    name: &str,
    text: &str,
    deadline: Instant,
) -> Result<(SendEnd, Option<Connection>), ClientError> {
    let Some(job_id) = watched_job(watch) else {
        return Ok((SendEnd::ResolvedElsewhere, None));
    };
    let (mut connection, _) = open()?;
    match connection.send_job_with_id(job_id, name, text, deadline) {
        Ok(sent) => {
            let accepted = Message::JobAccepted {
                job_id,
                position: sent.position,
            };
            // A job behind another one is not started by its sender (`specs/0134` S6).
            let keep = sent.position.is_none().then_some(connection);
            Ok((end_of(observe_shared(watch, &accepted)), keep))
        }
        Err(SendError::Rejected(reason)) => Ok((
            end_of(observe_shared(
                watch,
                &Message::JobRejected { job_id, reason },
            )),
            None,
        )),
        Err(SendError::Client(error)) => {
            drop(connection);
            if let Some(held) = watch
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .as_mut()
            {
                held.transfer_ended();
            }
            Ok((
                match open() {
                    Ok((fresh, status)) => {
                        // The list comes first on a connection: it names a job that waits.
                        let listed = fresh
                            .job_list
                            .as_ref()
                            .and_then(|list| observe_shared(watch, list));
                        end_of(listed.or_else(|| observe_shared(watch, &status)))
                    }
                    Err(_) => SendEnd::Unresolved(error),
                },
                None,
            ))
        }
    }
}
