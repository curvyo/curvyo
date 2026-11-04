//! The session's inputs and outputs: how it starts, what it asks the caller to do, the events
//! it reports and why a call is refused.

use curvyo_crypto_core::keyring::Entry;
use curvyo_crypto_core::{CryptoError, EpochKey, IdentityKeys, MemberId};

use crate::frame::{ConnectionId, RecordKind, RefusalReason, RoomId};

/// What an invite link carries after `#` (ADR 0031 decision 8): the room and the one-time seed.
#[derive(Clone, PartialEq, Eq)]
pub struct InviteSecret {
    /// The room (slice 1: the document id).
    pub room: RoomId,
    /// The 32-byte invite seed.
    pub seed: [u8; 32],
}

impl core::fmt::Debug for InviteSecret {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "InviteSecret({:?})", self.room)
    }
}

/// How a session starts.
pub enum SessionStart {
    /// A document already shared: its `keyring.log` entries and the epoch keys from the
    /// credential store.
    Member {
        /// The keyring entries.
        keyring: Vec<Entry>,
        /// The epoch keys held, with their epoch numbers.
        epoch_keys: Vec<(u32, EpochKey)>,
    },
    /// Sharing the document now (criterion 1): the session writes the genesis.
    Share,
    /// Joining by invite link (criterion 3).
    Join(InviteSecret),
}

/// Everything a session needs from its caller at the start.
pub struct SessionConfig {
    /// The room (for a join: the link's room).
    pub room: RoomId,
    /// This device's identity keys.
    pub identity: IdentityKeys,
    /// The session's Loro peer id, the envelopes' sender (ADR 0031 decision 4).
    pub peer_id: u64,
    /// 32 fresh bytes seeding the nonce stream.
    pub nonce_seed: [u8; 32],
    /// 32 further fresh bytes for keys, wrap ephemerals, invite seeds and jitter.
    pub entropy_seed: [u8; 32],
    /// The last record sequence the local file holds (`share.json` `server_seq`).
    pub since_seq: u64,
    /// How it starts.
    pub start: SessionStart,
}

/// What the caller does next.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientOutput {
    /// Open the socket, then call [`connected`](crate::ClientSession::connected) (or
    /// `disconnected`).
    Connect,
    /// Send one binary message.
    Send(Vec<u8>),
    /// Close the socket.
    Close,
    /// Something to apply or show.
    Event(SessionEvent),
}

/// Something the caller applies to its document or shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionEvent {
    /// Connected, caught up and holding the current key.
    Online,
    /// The connection was lost; edits are kept and go out on reconnect (criterion 17).
    Offline,
    /// An opened record: update bytes to import, asset bytes to insert, or snapshot bytes to
    /// open or merge.
    Remote {
        /// Its sequence.
        seq: u64,
        /// What it is.
        kind: RecordKind,
        /// The plaintext.
        bytes: Vec<u8>,
    },
    /// A push was stored under `seq`.
    Acked {
        /// The push's number from [`push`](crate::ClientSession::push).
        client_id: u64,
        /// Its sequence.
        seq: u64,
    },
    /// The keyring changed (participants, roles, epochs): save `keyring.log` and keys.
    KeyringChanged,
    /// Admitted to the room but holding no key yet: "Waiting for a member to let you in".
    WaitingForMember,
    /// A member handed this device the key.
    Admitted,
    /// The invite expired while waiting (criterion 3).
    InviteExpired,
    /// The relay refused this session; it has ended.
    Refused(RefusalReason),
    /// This device was removed (criterion 33): it stops syncing, the file stays local.
    Removed,
    /// Another device connected to the room.
    Peer {
        /// Its connection.
        connection: ConnectionId,
        /// Its identity as the relay reports it.
        member: MemberId,
    },
    /// Another device's presence.
    Presence {
        /// Its connection.
        connection: ConnectionId,
        /// Its identity as the relay reported it, if known.
        member: Option<MemberId>,
        /// The opened payload.
        payload: Vec<u8>,
    },
    /// Another device's presence is gone (left, or silent for 5 s).
    PresenceGone {
        /// Its connection.
        connection: ConnectionId,
    },
    /// A snapshot is due: call
    /// [`upload_snapshot`](crate::ClientSession::upload_snapshot) with the replica's state, which
    /// holds every record up to `covers_seq`.
    SnapshotDue {
        /// The records it covers.
        covers_seq: u64,
    },
    /// A received record failed to open; shown once (criterion 25).
    UpdateRejected,
    /// After a rejected record the session reconnects from the room's snapshot: every record
    /// from the snapshot on arrives again (ADR 0031 decision 3, UX notes §6 "Resyncing").
    Resyncing,
}

/// Why a session call was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SessionError {
    /// Only an admin may do this.
    #[error("only an admin can do this")]
    NotAdmin,
    /// The device named, or this device, is not a current member.
    #[error("not a current member")]
    NotAMember,
    /// The device to add is a member already, or was removed.
    #[error("already a member or removed")]
    AlreadyMember,
    /// The payload is over the message size limit.
    #[error("the message is too large")]
    TooLarge,
    /// This device holds no key of the current epoch yet.
    #[error("no key to seal with")]
    NoKey,
    /// A snapshot goes through [`upload_snapshot`](crate::ClientSession::upload_snapshot), not as
    /// a push.
    #[error("a snapshot is not pushed")]
    SnapshotPushed,
    /// Inviting and removing need a connection.
    #[error("not connected")]
    Offline,
    /// The session has ended (removed, refused, expired).
    #[error("the session has ended")]
    Ended,
    /// A sealing or keyring operation failed.
    #[error(transparent)]
    Crypto(#[from] CryptoError),
}
