//! The message types of protocol version 0 (ADR 0035 §6) as plain data.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::admin::{ClientEntry, ConnectorMode, LogName, Role, Secret};
use crate::config::ConnectorConfig;
use crate::jog::{JogAxis, JogDirection, JogStep};
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
    /// The job list holds 20 jobs or 256 MiB (`specs/0132`).
    ListFull,
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
    /// The controller moves the head for a jog (`specs/0118`); a kept `Ready` job is carried along.
    Jogging,
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

/// One job of the connector's job list (`specs/0132`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListEntry {
    /// The job.
    pub job_id: Uuid,
    /// The name sent with it.
    pub name: String,
    /// Its lines.
    pub lines: u32,
    /// Its size in bytes.
    pub bytes: u64,
    /// The client that sent it.
    pub sent_by: String,
    /// 1-based place among the waiting jobs; 0 for the current job and an interrupted one.
    pub position: u32,
    /// Marked "Run next automatically"; left out when false.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub run_next: bool,
    /// A job that ran when the connector stopped: the line it had reached.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interrupted_at: Option<u32>,
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
    /// The connector is in maintenance mode and does not take this message (A3).
    Maintenance,
    /// The setup window of maintenance mode is closed (A5).
    MaintenanceClosed,
    /// `ApplyConfig` carried a configuration that breaks a rule; the message names the field.
    InvalidConfig,
    /// Only an owner may send this, or the connector is configured already (A7, A9).
    OwnerRequired,
    /// No client or connector is called that (A11, A19).
    UnknownName,
}

impl ErrorCode {
    /// Every code, in the order of the golden file `errors/codes.json`.
    pub const ALL: [ErrorCode; 14] = [
        ErrorCode::UnsupportedProtocol,
        ErrorCode::Malformed,
        ErrorCode::UnknownType,
        ErrorCode::FrameTooLarge,
        ErrorCode::HelloRequired,
        ErrorCode::PairingRequired,
        ErrorCode::UnknownJob,
        ErrorCode::WrongState,
        ErrorCode::UnexpectedMessage,
        ErrorCode::Maintenance,
        ErrorCode::MaintenanceClosed,
        ErrorCode::InvalidConfig,
        ErrorCode::OwnerRequired,
        ErrorCode::UnknownName,
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
    /// Moves the head one step (or, with `hold`, until `JogKeep` stops coming) with the laser off;
    /// an owner only (`specs/0118` criteria 16, 18).
    Jog {
        /// The machine axis.
        axis: JogAxis,
        /// The way along it.
        direction: JogDirection,
        /// The distance of one press.
        step: JogStep,
        /// A held press: the head keeps moving while `JogKeep` arrives; left out when false.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        hold: bool,
    },
    /// Keeps a held jog alive; the connector stops it when none arrives for 300 ms.
    JogKeep,
    /// Ends a jog at once (a released press); not `Stop`: outside a jog it does nothing.
    JogStop,
    /// Moves a waiting job from place `from` to place `to` (both 1-based; owner).
    MoveJob {
        /// The job.
        job_id: Uuid,
        /// The place the app saw it at.
        from: u32,
        /// The place it moves to.
        to: u32,
    },
    /// Removes a waiting job (its sender, or an owner).
    RemoveJob {
        /// The job.
        job_id: Uuid,
    },
    /// Sets or clears "Run next automatically" on a waiting job (its sender, or an owner).
    SetRunNext {
        /// The job.
        job_id: Uuid,
        /// The new value.
        on: bool,
    },
    /// Asks for the serial ports of the connector's host (maintenance or owner).
    ListPorts,
    /// Configures the connector: in maintenance mode the sender becomes its owner; in configured
    /// mode an owner replaces the configuration.
    ApplyConfig {
        /// The configuration.
        config: ConnectorConfig,
        /// The sender's name, recorded as the owner in maintenance mode.
        client_name: String,
    },
    /// Asks for the configuration (owner).
    GetConfig,
    /// Issues a pairing code for a client of this name and role (owner).
    IssueCode {
        /// The name the pairing is recorded under.
        client_name: String,
        /// The role it gets.
        role: Role,
    },
    /// Asks for the paired clients (owner).
    ListClients,
    /// Unpairs a client and closes its connections (owner).
    RevokeClient {
        /// Its name.
        name: String,
    },
    /// Asks for the last lines of a log (owner).
    GetLog {
        /// Which log.
        log: LogName,
        /// How many lines, clamped to 1 to 1,000.
        last: u32,
    },
    /// Stops a running job, deletes the connector's state and ends it (owner).
    Reset {
        /// The connector's name, typed by the owner as the confirmation.
        connector_name: String,
    },
    /// Answer to `Hello` once accepted.
    Welcome {
        /// The connector's name.
        connector_name: String,
        /// The controller, if one is attached.
        controller: Option<ControllerInfo>,
        /// Maintenance, configured or foreground; absent from a connector before slice 5.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        mode: Option<ConnectorMode>,
    },
    /// After `Paired` and after a valid `Auth`, before the `Status`: who this client is.
    Authenticated {
        /// The client's name in `clients.json`.
        client_name: String,
        /// Its role.
        role: Role,
    },
    /// Answer to `ListPorts`: device paths only.
    Ports {
        /// The paths, `/dev/serial/by-id/` names first.
        paths: Vec<String>,
    },
    /// Answer to `ApplyConfig`: in maintenance mode with the new owner's token.
    ConfigApplied {
        /// The owner's token, for a first configuration only.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        token: Option<Secret>,
    },
    /// Answer to `GetConfig`.
    Config {
        /// The configuration.
        config: ConnectorConfig,
    },
    /// Answer to `IssueCode`.
    CodeIssued {
        /// The code as printed (`XXXX-XXXX`).
        code: Secret,
        /// Seconds it stays valid.
        expires_in_s: u64,
    },
    /// Answer to `ListClients`.
    Clients {
        /// Every paired client, oldest first.
        clients: Vec<ClientEntry>,
    },
    /// Answer to `RevokeClient`.
    ClientRevoked {
        /// The removed client.
        name: String,
    },
    /// Answer to `GetLog`: raw lines, oldest first.
    LogLines {
        /// The lines without their line ends.
        lines: Vec<String>,
    },
    /// Answer to `Reset`, sent before the connector deletes its state and ends.
    ResetDone,
    /// Answer to `Pair`.
    Paired {
        /// The token for later `Auth`.
        token: String,
    },
    /// The job is assembled: `Ready`, or in the job list at `position`.
    JobAccepted {
        /// The job.
        job_id: Uuid,
        /// The 1-based place among the waiting jobs; left out when the job became current.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        position: Option<u32>,
    },
    /// The job list, pushed on every change and before a connection's first `Status`.
    JobList {
        /// The current job, if the connector holds one.
        current: Option<ListEntry>,
        /// An interrupted job first, then the waiting jobs in order.
        waiting: Vec<ListEntry>,
        /// "Chain stopped: `<state>`" until the next start.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        note: Option<String>,
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
        /// In `AwaitingLocalStart` under the owner rule: the member whose start or resume waits.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        requested_by: Option<String>,
        /// Under the owner rule (configured mode): the held job's name.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        job_name: Option<String>,
        /// Under the owner rule: the client that sent the held job.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        sent_by: Option<String>,
        /// The controller has been homed since the connector identified it (`specs/0118` 13);
        /// left out when false.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        homed: bool,
        /// The head's machine position, when the controller reported it.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        machine_position: Option<[Millimetres; 3]>,
        /// The controller's work coordinate offset, when the controller reported it.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        work_offset: Option<[Millimetres; 3]>,
    },
    /// A refusal that keeps the connection open, except where stated.
    Error {
        /// The code.
        code: ErrorCode,
        /// A sentence for the log.
        message: String,
    },
}
