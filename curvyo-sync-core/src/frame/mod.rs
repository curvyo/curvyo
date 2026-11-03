//! The relay protocol's wire frames (ADR 0031 decision 5): one binary WebSocket message is one
//! frame, a tag byte followed by little-endian fields; a frame's last byte field runs to the end
//! of the message.

mod codec;

use curvyo_crypto_core::{DocumentId, MemberId};

/// The protocol version a client names in its `Hello`: 2 adds `SnapshotPart` (ADR 0037 decision
/// 7). The relay still admits clients of protocol 1, of which 2 is a superset.
pub const PROTOCOL_VERSION: u16 = 2;
/// The oldest protocol version the relay admits.
pub const OLDEST_PROTOCOL_VERSION: u16 = 1;

/// The largest message either side accepts: 64 MiB (ADR 0031 decision 5, Proposal; above the
/// image import ceiling).
pub const MAX_MESSAGE_LEN: usize = 64 * 1024 * 1024;

/// An opaque 16-byte room id. The relay knows rooms, never documents; in slice 1 a room's id is
/// its document's id (COOP-SCOPE A), but nothing assumes one room per server or per user.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RoomId(pub [u8; 16]);

impl From<DocumentId> for RoomId {
    fn from(id: DocumentId) -> Self {
        Self(id.0)
    }
}

impl RoomId {
    /// The document whose envelopes this room carries (slice 1: the same 16 bytes).
    #[must_use]
    pub fn document_id(self) -> DocumentId {
        DocumentId(self.0)
    }
}

/// The relay's number for one connection, unique while the relay runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ConnectionId(pub u64);

/// What a stored record carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RecordKind {
    /// A sealed Loro update.
    Update,
    /// The sealed bytes of an embedded asset.
    Asset,
    /// The room's newest signed snapshot; only the relay sends this kind, as the first record a
    /// client receives when its cursor lies before the snapshot (Proposal).
    Snapshot,
}

/// Why the relay refused a connection or a frame (ADR 0031 decisions 5 and 6a).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RefusalReason {
    /// The invite was already claimed.
    Used,
    /// The invite is past its expiry by the relay's clock.
    Expired,
    /// The identity is not a current member and offered no valid genesis or claim.
    NotAMember,
    /// A message is over [`MAX_MESSAGE_LEN`].
    TooLarge,
    /// A frame was malformed or out of order.
    Protocol,
    /// The identity was removed from the room's keyring.
    Removed,
}

/// A frame a client sends to the relay.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientFrame {
    /// Opens a room session: the protocol version, the room, the last record sequence the
    /// client holds (0: none) and the client's ed25519 identity key.
    Hello {
        /// [`PROTOCOL_VERSION`].
        protocol: u16,
        /// The room.
        room: RoomId,
        /// The client's cursor.
        since_seq: u64,
        /// Who connects.
        identity: MemberId,
    },
    /// The signature over `"curvyo-relay-v1" ‖ room ‖ nonce` answering a `Challenge`.
    Proof {
        /// The ed25519 signature.
        signature: [u8; 64],
    },
    /// One encoded keyring entry.
    Keyring {
        /// The entry's bytes (`Entry::encode`).
        entry: Vec<u8>,
    },
    /// A sealed update or asset to store and relay.
    Push {
        /// The client's own number for it, echoed in the `Ack`.
        client_id: u64,
        /// Update or asset; never snapshot.
        kind: RecordKind,
        /// The envelope.
        envelope: Vec<u8>,
    },
    /// A signed snapshot covering every record up to `covers_seq`.
    Snapshot {
        /// The last record the snapshot holds.
        covers_seq: u64,
        /// The signed envelope.
        envelope: Vec<u8>,
    },
    /// A sealed presence message, relayed and never stored.
    Awareness {
        /// The envelope.
        envelope: Vec<u8>,
    },
    /// Liveness.
    Ping,
    /// One part of a multi-part signed snapshot covering every record up to `covers_seq`
    /// (protocol 2): parts arrive in order from index 0; the relay swaps them in on `last`.
    SnapshotPart {
        /// The last record the snapshot holds.
        covers_seq: u64,
        /// The part's place, from 0.
        index: u32,
        /// Whether it is the snapshot's last part.
        last: bool,
        /// The signed envelope.
        envelope: Vec<u8>,
    },
}

/// A frame the relay sends to a client.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RelayFrame {
    /// The nonce a client signs in its `Proof`.
    Challenge {
        /// 32 bytes of the relay's entropy.
        nonce: [u8; 32],
    },
    /// Admission: the room's newest sequence, its snapshot's `covers_seq` (0: none) and this
    /// connection's number.
    Welcome {
        /// The newest record sequence.
        head_seq: u64,
        /// The sequence the stored snapshot covers.
        snapshot_seq: u64,
        /// This connection.
        connection: ConnectionId,
    },
    /// One accepted keyring entry.
    Keyring {
        /// The entry's bytes.
        entry: Vec<u8>,
    },
    /// One stored record.
    Record {
        /// Its sequence (a snapshot's: its `covers_seq`).
        seq: u64,
        /// What it carries.
        kind: RecordKind,
        /// The envelope.
        envelope: Vec<u8>,
    },
    /// A push was stored under `seq`.
    Ack {
        /// The client's number of the push.
        client_id: u64,
        /// Its sequence.
        seq: u64,
    },
    /// Another connection in the room joined (with its identity) or left (`None`).
    Peer {
        /// The other connection.
        connection: ConnectionId,
        /// Its identity, `None` when it left.
        identity: Option<MemberId>,
    },
    /// Another connection's sealed presence message.
    Awareness {
        /// The sender's connection.
        connection: ConnectionId,
        /// The envelope.
        envelope: Vec<u8>,
    },
    /// The connection is refused and will be closed.
    Refused {
        /// Why.
        reason: RefusalReason,
    },
    /// The answer to a `Ping`.
    Pong,
}

/// Why bytes are not a frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum FrameError {
    /// The message is over [`MAX_MESSAGE_LEN`].
    #[error("the message is over the size limit")]
    TooLarge,
    /// The message is truncated, has trailing bytes, or an unknown tag or field value.
    #[error("the message is not a frame")]
    Malformed,
}
