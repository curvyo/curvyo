//! The relay's side of the protocol: the connection handshake and the routing of frames to rooms
//! (ADR 0031 decision 5). The server is an I/O shell around [`Relay`]: it feeds each received
//! message in and carries the [`RelayOutput`]s out (sends, closes and store writes).

mod admission;
mod room;
mod snapshot;

use std::collections::BTreeMap;

use curvyo_crypto_core::MemberId;

pub use room::{RoomLog, StoredRecord, StoredRoom};

use crate::frame::{ClientFrame, ConnectionId, FrameError, RefusalReason, RelayFrame, RoomId};
use crate::time::Millis;

/// One effect of the relay's decision, for the server to carry out in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RelayOutput {
    /// Send a frame on a connection.
    Send {
        /// To whom.
        connection: ConnectionId,
        /// What.
        frame: RelayFrame,
    },
    /// Close a connection (after the frames sent before it).
    Close {
        /// Which.
        connection: ConnectionId,
    },
    /// Write to the room's store before the sends that follow it are carried out.
    Store(StoreOp),
}

/// A write to the relay's store (ADR 0031 decision 11: `keyring.log`, `log`, `snapshot`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreOp {
    /// Append an accepted keyring entry; the first one creates the room.
    Keyring {
        /// The room.
        room: RoomId,
        /// The entry's bytes.
        entry: Vec<u8>,
    },
    /// Append a record to the room's log.
    Record {
        /// The room.
        room: RoomId,
        /// The record.
        record: StoredRecord,
    },
    /// Replace the room's snapshot and drop every record up to `covers_seq`.
    Snapshot {
        /// The room.
        room: RoomId,
        /// The last record it covers.
        covers_seq: u64,
        /// The signed envelope.
        envelope: Vec<u8>,
    },
    /// Stage one part of a multi-part snapshot (`snapshot.part/`); part 0 discards any staged
    /// parts first (ADR 0037 decision 7).
    SnapshotPart {
        /// The room.
        room: RoomId,
        /// The last record the snapshot covers.
        covers_seq: u64,
        /// The part's place.
        index: u32,
        /// The signed envelope.
        envelope: Vec<u8>,
    },
    /// The staged `count` parts replace the room's snapshot; every record up to `covers_seq` is
    /// dropped.
    SnapshotParts {
        /// The room.
        room: RoomId,
        /// The last record they cover.
        covers_seq: u64,
        /// How many parts are staged.
        count: u32,
    },
    /// Discard the staged parts: a complete snapshot older than the stored one.
    DropStagedParts {
        /// The room.
        room: RoomId,
    },
}

/// Why stored rooms cannot be restored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum RestoreError {
    /// A stored keyring entry does not decode or the keyring has no valid genesis.
    #[error("a stored keyring is damaged")]
    DamagedKeyring,
    /// The stored records are not in ascending sequence order.
    #[error("a stored log is out of order")]
    DamagedLog,
}

/// Where a connection stands in the handshake.
#[derive(Debug, Clone, Copy)]
enum Stage {
    /// Waiting for `Hello`.
    Fresh,
    /// `Challenge` sent, waiting for `Proof`.
    Challenged {
        room: RoomId,
        member: MemberId,
        since_seq: u64,
        nonce: [u8; 32],
    },
    /// Identity proven but not a member: waiting for a genesis or a claim.
    Proven {
        room: RoomId,
        member: MemberId,
        since_seq: u64,
    },
    /// In the room.
    Admitted { room: RoomId },
}

/// Every room this relay holds and every open connection.
#[derive(Default)]
pub struct Relay {
    rooms: BTreeMap<RoomId, RoomLog>,
    connections: BTreeMap<ConnectionId, Stage>,
    next_connection: u64,
}

impl Relay {
    /// A relay without rooms.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// A relay holding the stored rooms (the startup scan of the server, criterion 20).
    ///
    /// # Errors
    /// See [`RestoreError`].
    pub fn restore(rooms: Vec<StoredRoom>) -> Result<Self, RestoreError> {
        let mut relay = Self::new();
        for stored in rooms {
            let room = RoomLog::restore(stored)?;
            relay.rooms.insert(room.room(), room);
        }
        Ok(relay)
    }

    /// The room with this id, if the relay holds it.
    #[must_use]
    pub fn room(&self, room: &RoomId) -> Option<&RoomLog> {
        self.rooms.get(room)
    }

    /// The ids of every room, ascending.
    #[must_use]
    pub fn room_ids(&self) -> Vec<RoomId> {
        self.rooms.keys().copied().collect()
    }

    /// Registers a new connection and returns its number.
    pub fn connect(&mut self) -> ConnectionId {
        self.next_connection += 1;
        let connection = ConnectionId(self.next_connection);
        self.connections.insert(connection, Stage::Fresh);
        connection
    }

    /// Forgets a closed connection and tells its room's other connections.
    pub fn disconnect(&mut self, connection: ConnectionId) -> Vec<RelayOutput> {
        let mut out = Vec::new();
        if let Some(Stage::Admitted { room }) = self.connections.remove(&connection)
            && let Some(log) = self.rooms.get_mut(&room)
        {
            log.leave(connection, &mut out);
        }
        out
    }

    /// Handles one message from `connection`. `now` is the relay's clock (invite expiry);
    /// `nonce` is 32 fresh bytes of the server's entropy, used if a `Challenge` is due.
    pub fn receive(
        &mut self,
        connection: ConnectionId,
        bytes: &[u8],
        now: Millis,
        nonce: [u8; 32],
    ) -> Vec<RelayOutput> {
        let mut out = Vec::new();
        let Some(stage) = self.connections.get(&connection).copied() else {
            return out;
        };
        let frame = match ClientFrame::decode(bytes) {
            Ok(frame) => frame,
            Err(FrameError::TooLarge) => {
                self.refuse(connection, RefusalReason::TooLarge, &mut out);
                return out;
            }
            Err(FrameError::Malformed) => {
                self.refuse(connection, RefusalReason::Protocol, &mut out);
                return out;
            }
        };
        let verdict = self.dispatch(stage, frame, connection, now, nonce, &mut out);
        match verdict {
            Ok(Some(next)) => {
                self.connections.insert(connection, next);
            }
            Ok(None) => {}
            Err(reason) => self.refuse(connection, reason, &mut out),
        }
        self.close_removed(&mut out);
        out
    }

    /// The frame's verdict at the connection's stage: the next stage, or a refusal.
    fn dispatch(
        &mut self,
        stage: Stage,
        frame: ClientFrame,
        connection: ConnectionId,
        now: Millis,
        nonce: [u8; 32],
        out: &mut Vec<RelayOutput>,
    ) -> Result<Option<Stage>, RefusalReason> {
        match stage {
            Stage::Fresh => admission::hello(&frame, nonce, connection, out),
            Stage::Challenged {
                room,
                member,
                since_seq,
                nonce,
            } => admission::proof(
                &frame,
                &admission::Claimant {
                    room,
                    member,
                    since_seq,
                },
                &nonce,
                &mut self.rooms,
                connection,
                out,
            ),
            Stage::Proven {
                room,
                member,
                since_seq,
            } => admission::genesis_or_claim(
                &frame,
                &admission::Claimant {
                    room,
                    member,
                    since_seq,
                },
                now,
                &mut self.rooms,
                connection,
                out,
            ),
            Stage::Admitted { room } => match self.rooms.get_mut(&room) {
                Some(log) => log.handle(connection, frame, out),
                None => Err(RefusalReason::Protocol),
            },
        }
    }

    /// Sends `Refused`, closes and forgets the connection.
    fn refuse(
        &mut self,
        connection: ConnectionId,
        reason: RefusalReason,
        out: &mut Vec<RelayOutput>,
    ) {
        out.push(RelayOutput::Send {
            connection,
            frame: RelayFrame::Refused { reason },
        });
        out.push(RelayOutput::Close { connection });
        out.extend(self.disconnect(connection));
    }

    /// Forgets connections a room closed (removed members, ADR 0031 decision 6a).
    fn close_removed(&mut self, out: &mut [RelayOutput]) {
        for output in out.iter() {
            if let RelayOutput::Close { connection } = output {
                self.connections.remove(connection);
            }
        }
    }
}

pub(crate) use admission::relay_proof_message;
