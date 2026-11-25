//! The machine connector protocol (ADR 0035, ADR 0071): its messages, the version check, who may
//! send what, the configuration, job assembly from chunks and the job state machine. Pure data and logic, no I/O, no clock, builds for wasm32.

#![forbid(unsafe_code)]
#![cfg_attr(
    test,
    allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)
)]

mod access;
mod admin;
mod assembly;
mod codec;
mod config;
mod count;
mod focus;
mod frame_text;
mod handshake;
mod job_list;
mod jog;
mod machine;
mod message;
mod millimetres;
mod send_watch;

pub use access::{Mode, admit, refusal};
pub use admin::{ClientEntry, ConnectorMode, LogName, Role, Secret};
pub use assembly::{Job, MAX_CHUNK_BYTES, MAX_JOB_BYTES};
pub use codec::{CodecError, MAX_FRAME_BYTES, decode, encode};
pub use config::{
    CONFIG_FORMAT, ConfigError, ConnectorConfig, ControllerConfig, DEFAULT_BAUD,
    DEFAULT_LINES_PER_SECOND, MAX_BAUD, MAX_CLIENT_NAME, MAX_LABEL, check_client_name, check_text,
};
pub use count::count;
pub use focus::{
    ARRIVED, MAX_TARGET, PULSE_GRACE, Pulse, PulseLength, PulsePower, arrived, clamp_target,
};
pub use frame_text::{FrameMoves, NotAFrame, check as check_frame};
pub use handshake::{Handshake, Outcome, PROTOCOL_VERSION};
pub use job_list::{
    Chain, Dropped, End, Entry, Item, JobList, MAX_BYTES, MAX_ENTRIES, Restored, StoredCurrent,
    StoredList, next as chain_next, restore,
};
pub use jog::{EDGE, Feed, JOG_DEAD_MAN, JogAxis, JogDirection, JogStep, TravelBox, clamp};
pub use machine::{
    Confirmation, Confirms, ControllerCommand, Event, ListLog, Machine, Options, Output, Peer,
    People, StartRule,
};
pub use message::{
    ControllerInfo, ControllerSettings, ErrorCode, JobKind, JobResult, ListEntry, Message,
    Progress, RejectReason, State,
};
pub use millimetres::Millimetres;
pub use send_watch::{Resolution, SendWatch};
/// The id type of jobs and frames, so callers need no `uuid` dependency of their own.
pub use uuid::Uuid;
