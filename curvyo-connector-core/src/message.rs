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
    /// The controller's laser mode is off (GRBL `$32=0`, `specs/0127` G34).
    LaserModeOff,
    /// The controller's laser mode could not be read (GRBL without `$32` in `$$`, G34).
    LaserModeUnknown,
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
    /// Started, waiting for the button on the connector's host; a waiting `Home` carries no job.
    AwaitingLocalStart,
    /// The controller runs its homing cycle (`Home`); a kept `Ready` job is carried along.
    Homing,
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ControllerInfo {
    /// The controller's own name ("GRBL", "Fake controller").
    pub name: String,
    /// Its firmware version ("1.1h").
    pub firmware: String,
    /// The serial port the connector uses; absent for the fake controller.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port: Option<String>,
    /// The baud rate the connector uses.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub baud: Option<u32>,
    /// The settings the controller reported.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settings: Option<ControllerSettings>,
}

/// What the controller reported of its settings (GRBL `$$`); `None` for a value it did not report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ControllerSettings {
    /// Laser mode (GRBL `$32`).
    pub laser_mode: Option<bool>,
    /// The S value of full power, rounded (GRBL `$30`).
    pub s_max: Option<u32>,
    /// The travel of X and Y (GRBL `$130`, `$131`).
    pub travel: Option<[Millimetres; 2]>,
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
    /// Leaves the controller's alarm lock (GRBL `$X`); only in `Alarm` (`specs/0127` G14).
    Unlock,
    /// Runs the controller's homing cycle (GRBL `$H`), from `Idle`, `Ready`, `Done` or `Alarm`
    /// (`specs/0118`); never sent on its own by the app or the connector.
    Home,
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
        /// The sentence that explains the state ("Failed at line 12: No feed rate (error 22)",
        /// "Lost the controller at line 120", "Stopping", "Controller state: Door").
        #[serde(default, skip_serializing_if = "Option::is_none")]
        detail: Option<String>,
        /// A `Pause` was taken and waits for the controller's first moment to hold (a job's
        /// `$H` runs, `specs/0133`); left out when false.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        pause_pending: bool,
    },
    /// A refusal that keeps the connection open, except where stated.
    Error {
        /// The code.
        code: ErrorCode,
        /// A sentence for the log.
        message: String,
    },
}
