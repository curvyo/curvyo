//! Sending a job and the job controls over an authenticated connection.

use std::time::{Duration, Instant};

use curvyo_connector_core::{
    ErrorCode, JobKind, MAX_CHUNK_BYTES, MAX_JOB_BYTES, Message, RejectReason, State,
};
use ring::digest;
use ring::rand::{SecureRandom, SystemRandom};
use uuid::Uuid;

use crate::client::{ClientError, Connection};
use crate::frames::Frame;
use crate::hex;

/// How long the connector gets to answer a job command.
const COMMAND_WAIT: Duration = Duration::from_millis(600);
/// How long to look for an immediate rejection after the announcement.
const REJECTION_WAIT: Duration = Duration::from_millis(60);

/// Why a job was not sent.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SendError {
    /// The connector refused the job.
    #[error("the connector refused the job: {0:?}")]
    Rejected(RejectReason),
    /// The connection failed.
    #[error(transparent)]
    Client(#[from] ClientError),
}

/// A job the connector holds and shows as `Ready`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SentJob {
    /// Its id, needed for `Start`.
    pub job_id: Uuid,
    /// Its lines.
    pub lines: u32,
    /// Its size in bytes.
    pub bytes: u64,
}

/// A fresh random job id.
///
/// # Errors
/// [`ClientError::Io`] when the system has no random bytes.
pub fn new_job_id() -> Result<Uuid, ClientError> {
    let mut bytes = [0; 16];
    SystemRandom::new()
        .fill(&mut bytes)
        .map_err(|_| ClientError::Io("no random bytes are available".to_owned()))?;
    Ok(uuid::Builder::from_random_bytes(bytes).into_uuid())
}

fn chunks(text: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut rest = text;
    while !rest.is_empty() {
        let mut end = rest.len().min(MAX_CHUNK_BYTES);
        while !rest.is_char_boundary(end) {
            end -= 1;
        }
        let (part, tail) = rest.split_at(end);
        parts.push(part);
        rest = tail;
    }
    parts
}

fn rejection(message: &Message, job_id: Uuid) -> Option<RejectReason> {
    match message {
        Message::JobRejected { job_id: id, reason } if *id == job_id => Some(*reason),
        _ => None,
    }
}

impl Connection {
    /// Sends `text` as a job called `name`; returns when the connector says `Ready`.
    ///
    /// # Errors
    /// [`SendError::Rejected`] with the connector's reason, or a connection error.
    pub fn send_job(
        &mut self,
        name: &str,
        text: &str,
        deadline: Instant,
    ) -> Result<SentJob, SendError> {
        self.send_job_with_id(new_job_id()?, name, text, deadline)
    }

    /// [`Self::send_job`] with the id chosen by the caller, who follows the job by it
    /// (`specs/0124` B4).
    ///
    /// # Errors
    /// As [`Self::send_job`].
    pub fn send_job_with_id(
        &mut self,
        job_id: Uuid,
        name: &str,
        text: &str,
        deadline: Instant,
    ) -> Result<SentJob, SendError> {
        let bytes = text.len() as u64;
        let sha256 = hex::encode(digest::digest(&digest::SHA256, text.as_bytes()).as_ref());
        self.send(&Message::SubmitJob {
            job_id,
            name: name.to_owned(),
            kind: JobKind::Job,
            bytes,
            sha256,
        })?;
        if let Some(reason) = self.early_rejection(job_id)? {
            return Err(SendError::Rejected(reason));
        }
        for (index, part) in chunks(text).into_iter().enumerate() {
            self.send(&Message::JobChunk {
                job_id,
                index: u32::try_from(index).unwrap_or(u32::MAX),
                text: part.to_owned(),
            })?;
        }
        loop {
            let message = self.recv_message(deadline)?;
            if let Some(reason) = rejection(&message, job_id) {
                return Err(SendError::Rejected(reason));
            }
            if message == (Message::JobAccepted { job_id }) {
                let lines = u32::try_from(text.lines().count()).unwrap_or(u32::MAX);
                return Ok(SentJob {
                    job_id,
                    lines,
                    bytes,
                });
            }
        }
    }

    /// Announces a job of `size` bytes that is not sent (a file above the limit): the connector
    /// refuses it and logs the refusal.
    ///
    /// # Errors
    /// [`SendError::Rejected`], normally with `TooLarge`.
    pub fn announce_oversized(
        &mut self,
        name: &str,
        size: u64,
        deadline: Instant,
    ) -> Result<(), SendError> {
        debug_assert!(size > MAX_JOB_BYTES);
        let job_id = new_job_id()?;
        self.send(&Message::SubmitJob {
            job_id,
            name: name.to_owned(),
            kind: JobKind::Job,
            bytes: size,
            sha256: String::new(),
        })?;
        loop {
            let message = self.recv_message(deadline)?;
            if let Some(reason) = rejection(&message, job_id) {
                return Err(SendError::Rejected(reason));
            }
        }
    }

    fn early_rejection(&mut self, job_id: Uuid) -> Result<Option<RejectReason>, ClientError> {
        let until = Instant::now() + REJECTION_WAIT;
        while let Some(frame) = self.try_recv(until)? {
            if let Frame::Core(message) = frame
                && let Some(reason) = rejection(&message, job_id)
            {
                return Ok(Some(reason));
            }
        }
        Ok(None)
    }

    /// Sends one job command (`Start`, `Pause`, `Resume`, `Stop`) and waits briefly for what it
    /// changed: the new `Status`, the unchanged one when nothing changed (`Stop` is idempotent),
    /// or the connector's `Error`.
    ///
    /// # Errors
    /// [`ClientError::Protocol`] with the connector's error code in words.
    pub fn command(
        &mut self,
        message: &Message,
        before: State,
    ) -> Result<Option<Message>, ClientError> {
        self.send(message)?;
        let until = Instant::now() + COMMAND_WAIT;
        while let Some(frame) = self.try_recv(until)? {
            match frame {
                Frame::Core(Message::Error { code, .. }) => {
                    return Err(ClientError::Protocol(format!("{code:?}")));
                }
                Frame::Core(status @ Message::Status { state, .. }) if state != before => {
                    return Ok(Some(status));
                }
                _ => {}
            }
        }
        Ok(None)
    }
}

/// Whether the error code names a command that does not fit the state.
#[must_use]
pub fn is_wrong_state(error: &ClientError) -> bool {
    matches!(error, ClientError::Protocol(text) if text == &format!("{:?}", ErrorCode::WrongState))
}
