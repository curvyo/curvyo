//! The message types of protocol version 0 (ADR 0035 §6) as plain data.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::millimetres::Millimetres;

/// What a job is: the work itself or its outline at low power.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobKind {
    /// The job's G-code.
    Job,
    /// The frame: the job's outline at low power.
    Frame,
}

/// Why a submitted job is refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RejectReason {
    /// A job is already held.
    Busy,
    /// The announced or sent size is above a limit.
    TooLarge,
    /// The text does not hash to the announced value, or the chunks are out of order.
    HashMismatch,
    /// The text has a byte outside printable ASCII and newline.
    NotAscii,
    /// No controller is attached.
    NoController,
}

/// How a job ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobResult {
    /// The controller ran all lines.
    Completed,
    /// The maker stopped it.
    Stopped,
    /// The controller reported an error.
    Failed,
}

/// The connector's job state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "name")]
pub enum State {
    /// No controller is attached.
    NoController,
    /// A controller is attached and no job is held.
    Idle,
    /// A job's chunks are arriving.
    Receiving,
    /// A job is assembled and waits for `Start`.
    Ready,
    /// Started, waiting for the button on the connector's host.
    AwaitingLocalStart,
    /// The controller is running the job.
    Running,
    /// The job is paused.
    Paused,
    /// The job ended.
    Done {
        /// How it ended.
        result: JobResult,
    },
    /// The controller reported an alarm.
    Alarm,
}

/// Progress of the held job.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Progress {
    /// The job.
    pub job_id: Uuid,
    /// Lines sent so far.
    pub line: u32,
    /// Lines in the job.
    pub lines: u32,
    /// Milliseconds since the start, counted by the caller.
    pub elapsed_ms: u64,
}

/// The controller the connector talks to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControllerInfo {
    /// Port or device name.
    pub name: String,
    /// Firmware banner.
    pub firmware: String,
}

/// The code of an `Error` message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorCode {
    /// `Hello` carried another protocol version; the connection closes.
    UnsupportedProtocol,
    /// Not JSON, or a known type with a missing or wrongly typed field.
    Malformed,
    /// A `type` this version does not know.
    UnknownType,
    /// A frame above 1 MiB.
    FrameTooLarge,
    /// The first message was not `Hello`; the connection closes.
    HelloRequired,
    /// The client is not paired or authenticated.
    PairingRequired,
    /// The `job_id` is not the job held or assembled.
    UnknownJob,
    /// The command does not fit the current state.
    WrongState,
    /// A message only the connector sends, or one the job machine does not take.
    UnexpectedMessage,
}

impl ErrorCode {
    /// Every code, in the order of the golden file `errors/codes.json`.
    pub const ALL: [ErrorCode; 9] = [
        ErrorCode::UnsupportedProtocol,
        ErrorCode::Malformed,
        ErrorCode::UnknownType,
        ErrorCode::FrameTooLarge,
        ErrorCode::HelloRequired,
        ErrorCode::PairingRequired,
        ErrorCode::UnknownJob,
        ErrorCode::WrongState,
        ErrorCode::UnexpectedMessage,
    ];
}

/// One protocol message, one JSON object `{ "type": "<Name>", ... }`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Message {
    /// First message from either side.
    Hello {
        /// Protocol version.
        protocol: u32,
        /// Name of the sending program.
        software: String,
        /// Its version.
        version: String,
    },
    /// Pairing with a code.
    Pair {
        /// The pairing code.
        code: String,
        /// A name for the client.
        client_name: String,
    },
    /// Authentication with the pairing token.
    Auth {
        /// The token.
        token: String,
    },
    /// Asks for a `Status`.
    GetStatus,
    /// Announces a job; `JobChunk`s follow.
    SubmitJob {
        /// The job.
        job_id: Uuid,
        /// A name for the log.
        name: String,
        /// Job or frame.
        kind: JobKind,
        /// Size of the whole text in bytes.
        bytes: u64,
        /// SHA-256 of the text, lower-case hex.
        sha256: String,
    },
    /// A piece of the job text.
    JobChunk {
        /// The job.
        job_id: Uuid,
        /// Counts from 0.
        index: u32,
        /// The text.
        text: String,
    },
    /// Starts the held job.
    Start {
        /// The job.
        job_id: Uuid,
    },
    /// Pauses.
    Pause,
    /// Resumes.
    Resume,
    /// Stops; idempotent.
    Stop,
    /// Answer to `Hello` once accepted.
    Welcome {
        /// The connector's name.
        connector_name: String,
        /// The controller, if one is attached.
        controller: Option<ControllerInfo>,
    },
    /// Answer to `Pair`.
    Paired {
        /// The token for later `Auth`.
        token: String,
    },
    /// The job is assembled and `Ready`.
    JobAccepted {
        /// The job.
        job_id: Uuid,
    },
    /// The job is refused.
    JobRejected {
        /// The job.
        job_id: Uuid,
        /// Why.
        reason: RejectReason,
    },
    /// The state, pushed on every change.
    Status {
        /// The state.
        state: State,
        /// Progress, in states that carry it.
        job: Option<Progress>,
        /// Position in millimetres.
        position: Option<[Millimetres; 3]>,
        /// The controller's alarm text.
        alarm: Option<String>,
    },
    /// A refusal that keeps the connection open, except where stated.
    Error {
        /// The code.
        code: ErrorCode,
        /// A sentence for the log.
        message: String,
    },
}
