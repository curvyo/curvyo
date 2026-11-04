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

mod backup;
mod client;
mod devices;
mod entropy;
mod frame;
mod invite_link;
mod pairing;
mod relay;
mod server_address;
mod time;
mod workspace_stream;

pub use backup::{BackupExchange, BackupOutcome};
pub use client::{
    ClientOutput, ClientSession, InviteSecret, SNAPSHOT_BYTES, SNAPSHOT_RECORDS, SessionConfig,
    SessionError, SessionEvent, SessionStart,
};
pub use devices::{DeviceAction, Plan, Target, WorkspaceInput, may_readmit, reconcile};
pub use frame::{
    ClientFrame, ConnectionId, FrameError, MAX_MESSAGE_LEN, OLDEST_PROTOCOL_VERSION,
    PROTOCOL_VERSION, PairCloseReason, RecordKind, RefusalReason, RelayFrame, RoomId,
};
pub use invite_link::{InviteKind, InviteLink, InviteLinkError};
pub use pairing::{
    CHECK_WORD_COUNT, PAIR_WRONG_TRIES, PairEvent, PairFailure, PairJoiner, PairOpener, PairOutput,
    PairingError, check_word,
};
pub use relay::{
    BACKUP_RECORD_MAX, Backups, PAIR_JOINS_PER_SLOT, PAIR_MESSAGE_MAX, PAIR_SLOT_LIFETIME_MS,
    PAIR_SLOTS_PER_SERVER, PairSlots, Relay, RelayOutput, RestoreError, RoomLog, StoreOp,
    StoredBackup, StoredRecord, StoredRoom,
};
pub use server_address::normalize_server_address;
pub use time::Millis;
pub use workspace_stream::{
    InnerHeader, Moved, PART_BYTES, PART_PADDING, SNAPSHOT_STREAM_VERSION, SnapshotAssembler,
    StreamEntry, StreamError, decode_inner_record, decode_moved, encode_inner_record, encode_moved,
    encode_snapshot_stream, snapshot_due, split_snapshot,
};
