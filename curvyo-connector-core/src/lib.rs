//! The machine connector protocol (ADR 0035): its messages, the version check, job assembly from
//! chunks and the job state machine. Pure data and logic, no I/O, no clock, builds for wasm32.

#![forbid(unsafe_code)]
#![cfg_attr(
    test,
    allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)
)]

mod assembly;
mod codec;
mod handshake;
mod machine;
mod message;
mod millimetres;

pub use assembly::{Job, MAX_CHUNK_BYTES, MAX_JOB_BYTES};
pub use codec::{CodecError, MAX_FRAME_BYTES, decode, encode};
pub use handshake::{Handshake, Outcome, PROTOCOL_VERSION};
pub use machine::{ControllerCommand, Event, Machine, Options, Output};
pub use message::{
    ControllerInfo, ErrorCode, JobKind, JobResult, Message, Progress, RejectReason, State,
};
pub use millimetres::Millimetres;
