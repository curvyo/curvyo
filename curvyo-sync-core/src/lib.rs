//! The relay protocol of cooperative editing (ADR 0031 decision 1): it defines the wire frames,
//! the client's session and the relay's room rules as pure state machines over bytes and
//! passed-in time.
//!
//! Pure and wasm-compatible (`CLAUDE.md` §6): no socket, clock, randomness, filesystem or thread.
//! The caller (the editor's wasm session, the relay server) feeds received bytes, the current
//! time and entropy in and carries the bytes and events that come out. Everything that leaves a
//! client is sealed by `curvyo-crypto-core`; there is no unsealed path (ADR 0008 §1).

#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod client;
mod entropy;
mod frame;
mod invite_link;
mod relay;
mod time;

pub use client::{
    ClientOutput, ClientSession, InviteSecret, SNAPSHOT_BYTES, SNAPSHOT_RECORDS, SessionConfig,
    SessionError, SessionEvent, SessionStart,
};
pub use frame::{
    ClientFrame, ConnectionId, FrameError, MAX_MESSAGE_LEN, PROTOCOL_VERSION, RecordKind,
    RefusalReason, RelayFrame, RoomId,
};
pub use invite_link::{InviteLink, InviteLinkError};
pub use relay::{Relay, RelayOutput, RestoreError, RoomLog, StoreOp, StoredRecord, StoredRoom};
pub use time::Millis;
