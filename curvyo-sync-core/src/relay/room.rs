//! One room's stored state and rules (ADR 0031 decisions 5, 6a, 10): the gapless record
//! sequence, the keyring sent before any record, removal closing connections, and snapshot
//! replace and prune.

use std::collections::BTreeMap;

use curvyo_crypto_core::keyring::{Entry, Keyring};
use curvyo_crypto_core::{EnvelopeHeader, EnvelopeKind, MemberId};

use super::snapshot::{Snapshots, Staged};
use super::{RelayOutput, RestoreError, Stage, StoreOp};
use crate::frame::{ClientFrame, ConnectionId, RecordKind, RefusalReason, RelayFrame, RoomId};

/// One stored record of a room's log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredRecord {
    /// Its gapless sequence, from 1.
    pub seq: u64,
    /// Update or asset.
    pub kind: RecordKind,
    /// The sealed envelope, as pushed.
    pub envelope: Vec<u8>,
}

/// Everything the relay stores of one room: what the server's startup scan reads back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredRoom {
    /// The room.
    pub room: RoomId,
    /// The accepted keyring entries, in the order they were appended.
    pub keyring: Vec<Vec<u8>>,
    /// The newest snapshot: the sequence it covers and its envelope, the first part of a
    /// multi-part snapshot.
    pub snapshot: Option<(u64, Vec<u8>)>,
    /// The parts after the first of a multi-part snapshot (ADR 0037 decision 7), in order; empty
    /// for a single one.
    pub snapshot_rest: Vec<Vec<u8>>,
    /// The records after the snapshot, ascending.
    pub records: Vec<StoredRecord>,
}

/// One room: its keyring, its log after the newest snapshot, and its admitted connections.
pub struct RoomLog {
    room: RoomId,
    entries: Vec<(Entry, Vec<u8>)>,
    keyring: Keyring,
    head_seq: u64,
    snapshots: Snapshots,
    records: Vec<StoredRecord>,
    online: BTreeMap<ConnectionId, MemberId>,
}

impl RoomLog {
    pub(super) fn new(room: RoomId, genesis: Entry, bytes: Vec<u8>, keyring: Keyring) -> Self {
        Self {
            room,
            entries: vec![(genesis, bytes)],
            keyring,
            head_seq: 0,
            snapshots: Snapshots::default(),
            records: Vec::new(),
            online: BTreeMap::new(),
        }
    }

    pub(super) fn restore(stored: StoredRoom) -> Result<Self, RestoreError> {
        let mut entries = Vec::with_capacity(stored.keyring.len());
        for bytes in stored.keyring {
            let entry = Entry::decode(&bytes).map_err(|_| RestoreError::DamagedKeyring)?;
            entries.push((entry, bytes));
        }
        let plain: Vec<Entry> = entries.iter().map(|(e, _)| e.clone()).collect();
        let keyring = Keyring::replay(&plain).map_err(|_| RestoreError::DamagedKeyring)?;
        if keyring.document_id() != stored.room.document_id() {
            return Err(RestoreError::DamagedKeyring);
        }
        let covers = stored.snapshot.as_ref().map_or(0, |(seq, _)| *seq);
        let current = stored.snapshot.map(|(seq, first)| {
            let mut parts = vec![first];
            parts.extend(stored.snapshot_rest);
            (seq, parts)
        });
        let mut head_seq = covers;
        for record in &stored.records {
            if record.seq <= head_seq {
                return Err(RestoreError::DamagedLog);
            }
            head_seq = record.seq;
        }
        Ok(Self {
            room: stored.room,
            entries,
            keyring,
            head_seq,
            snapshots: Snapshots::restored(current),
            records: stored.records,
            online: BTreeMap::new(),
        })
    }

    /// The room's id.
    #[must_use]
    pub fn room(&self) -> RoomId {
        self.room
    }

    /// The replayed keyring the relay holds (its membership check, ADR 0010 §10).
    #[must_use]
    pub fn keyring(&self) -> &Keyring {
        &self.keyring
    }

    /// The newest record sequence.
    #[must_use]
    pub fn head_seq(&self) -> u64 {
        self.head_seq
    }

    /// The sequence the stored snapshot covers, 0 without one.
    #[must_use]
    pub fn snapshot_seq(&self) -> u64 {
        self.snapshots.covers()
    }

    /// The records after the snapshot.
    #[must_use]
    pub fn records(&self) -> &[StoredRecord] {
        &self.records
    }

    /// The members with an open, admitted connection.
    #[must_use]
    pub fn online(&self) -> Vec<MemberId> {
        self.online.values().copied().collect()
    }

    /// Everything stored, as the server writes it.
    #[must_use]
    pub fn stored(&self) -> StoredRoom {
        StoredRoom {
            room: self.room,
            keyring: self.entries.iter().map(|(_, b)| b.clone()).collect(),
            snapshot: self
                .snapshots
                .current()
                .and_then(|(seq, parts)| Some((*seq, parts.first()?.clone()))),
            snapshot_rest: self
                .snapshots
                .current()
                .map(|(_, parts)| parts.iter().skip(1).cloned().collect())
                .unwrap_or_default(),
            records: self.records.clone(),
        }
    }

    /// Admits a connection: `Welcome`, every keyring entry, then the snapshot (for a cursor of
    /// 0 or one before it) and the records after the cursor, then the peers.
    pub(super) fn admit(
        &mut self,
        connection: ConnectionId,
        member: MemberId,
        since_seq: u64,
        out: &mut Vec<RelayOutput>,
    ) {
        let send = |frame| RelayOutput::Send { connection, frame };
        out.push(send(RelayFrame::Welcome {
            head_seq: self.head_seq,
            snapshot_seq: self.snapshot_seq(),
            connection,
        }));
        self.send_keyring(connection, out);
        let mut after = since_seq;
        // A multi-part snapshot goes out as consecutive snapshot records of one sequence.
        if let Some((covers, parts)) = self.snapshots.current()
            && (since_seq == 0 || since_seq < *covers)
        {
            for envelope in parts {
                out.push(send(RelayFrame::Record {
                    seq: *covers,
                    kind: RecordKind::Snapshot,
                    envelope: envelope.clone(),
                }));
            }
            after = *covers;
        }
        for record in self.records.iter().filter(|r| r.seq > after) {
            out.push(send(RelayFrame::Record {
                seq: record.seq,
                kind: record.kind,
                envelope: record.envelope.clone(),
            }));
        }
        for (&other, &other_member) in &self.online {
            out.push(send(RelayFrame::Peer {
                connection: other,
                identity: Some(other_member),
            }));
            out.push(RelayOutput::Send {
                connection: other,
                frame: RelayFrame::Peer {
                    connection,
                    identity: Some(member),
                },
            });
        }
        self.online.insert(connection, member);
    }

    /// Sends every accepted keyring entry to one connection.
    pub(super) fn send_keyring(&self, connection: ConnectionId, out: &mut Vec<RelayOutput>) {
        for (_, bytes) in &self.entries {
            out.push(RelayOutput::Send {
                connection,
                frame: RelayFrame::Keyring {
                    entry: bytes.clone(),
                },
            });
        }
    }

    /// A connection left: the others are told.
    pub(super) fn leave(&mut self, connection: ConnectionId, out: &mut Vec<RelayOutput>) {
        let mut ops = Vec::new();
        self.snapshots.forget(self.room, connection, &mut ops);
        out.extend(ops.into_iter().map(RelayOutput::Store));
        if self.online.remove(&connection).is_none() {
            return;
        }
        self.broadcast(
            Some(connection),
            &RelayFrame::Peer {
                connection,
                identity: None,
            },
            out,
        );
    }

    /// A frame from an admitted connection.
    pub(super) fn handle(
        &mut self,
        connection: ConnectionId,
        frame: ClientFrame,
        out: &mut Vec<RelayOutput>,
    ) -> Result<Option<Stage>, RefusalReason> {
        match frame {
            ClientFrame::Keyring { entry } => {
                let decoded = Entry::decode(&entry).map_err(|_| RefusalReason::Protocol)?;
                // An entry the replay refuses is dropped, not stored (Proposal): every replica
                // replays to the same verdict, so storing it would only cost space.
                self.append_entry(decoded, &entry, Some(connection), out);
            }
            ClientFrame::Push {
                client_id,
                kind,
                envelope,
            } => self.on_push(connection, client_id, kind, envelope, out)?,
            ClientFrame::Snapshot {
                covers_seq,
                envelope,
            } => self.on_snapshot(covers_seq, envelope, out)?,
            ClientFrame::SnapshotPart {
                covers_seq,
                index,
                last,
                envelope,
            } => self.on_snapshot_part(connection, covers_seq, index, last, envelope, out)?,
            ClientFrame::Awareness { envelope } => {
                self.check_envelope(&envelope, EnvelopeKind::Presence)?;
                self.broadcast(
                    Some(connection),
                    &RelayFrame::Awareness {
                        connection,
                        envelope,
                    },
                    out,
                );
            }
            ClientFrame::Ping => out.push(RelayOutput::Send {
                connection,
                frame: RelayFrame::Pong,
            }),
            ClientFrame::Hello { .. } | ClientFrame::Proof { .. } => {
                return Err(RefusalReason::Protocol);
            }
        }
        Ok(None)
    }

    /// Stores a pushed update or asset under the next sequence, acknowledges it to the sender
    /// and relays it to the others.
    fn on_push(
        &mut self,
        connection: ConnectionId,
        client_id: u64,
        kind: RecordKind,
        envelope: Vec<u8>,
        out: &mut Vec<RelayOutput>,
    ) -> Result<(), RefusalReason> {
        let expected = match kind {
            RecordKind::Update => EnvelopeKind::Update,
            RecordKind::Asset => EnvelopeKind::Asset,
            RecordKind::Snapshot => return Err(RefusalReason::Protocol),
        };
        self.check_envelope(&envelope, expected)?;
        self.head_seq += 1;
        let record = StoredRecord {
            seq: self.head_seq,
            kind,
            envelope,
        };
        out.push(RelayOutput::Store(StoreOp::Record {
            room: self.room,
            record: record.clone(),
        }));
        out.push(RelayOutput::Send {
            connection,
            frame: RelayFrame::Ack {
                client_id,
                seq: record.seq,
            },
        });
        let frame = RelayFrame::Record {
            seq: record.seq,
            kind,
            envelope: record.envelope.clone(),
        };
        self.records.push(record);
        self.broadcast(Some(connection), &frame, out);
        Ok(())
    }

    /// Keeps a signed snapshot that covers at least the stored one's records and prunes them.
    fn on_snapshot(
        &mut self,
        covers_seq: u64,
        envelope: Vec<u8>,
        out: &mut Vec<RelayOutput>,
    ) -> Result<(), RefusalReason> {
        let header = self.check_envelope(&envelope, EnvelopeKind::Snapshot)?;
        if !header.signed || covers_seq > self.head_seq {
            return Err(RefusalReason::Protocol);
        }
        if self.snapshots.replace(covers_seq, envelope.clone()) {
            out.push(RelayOutput::Store(StoreOp::Snapshot {
                room: self.room,
                covers_seq,
                envelope,
            }));
            self.records.retain(|r| r.seq > covers_seq);
        }
        Ok(())
    }

    /// Stages one part of a multi-part snapshot; the last swaps the parts in and prunes the
    /// records they cover (ADR 0037 decision 7).
    fn on_snapshot_part(
        &mut self,
        connection: ConnectionId,
        covers_seq: u64,
        index: u32,
        last: bool,
        envelope: Vec<u8>,
        out: &mut Vec<RelayOutput>,
    ) -> Result<(), RefusalReason> {
        let header = self.check_envelope(&envelope, EnvelopeKind::Snapshot)?;
        if !header.signed || covers_seq > self.head_seq {
            return Err(RefusalReason::Protocol);
        }
        let mut ops = Vec::new();
        let staged = self.snapshots.stage(
            self.room, connection, covers_seq, index, last, envelope, &mut ops,
        );
        out.extend(ops.into_iter().map(RelayOutput::Store));
        if let Staged::Swapped { covers } = staged? {
            self.records.retain(|r| r.seq > covers);
        }
        Ok(())
    }

    /// Appends an entry the replay accepts, stores it, relays it to every admitted connection
    /// but `from`, and closes the connections of members it removed. Returns whether it was
    /// appended (false for a refused or known entry).
    pub(super) fn append_entry(
        &mut self,
        entry: Entry,
        bytes: &[u8],
        from: Option<ConnectionId>,
        out: &mut Vec<RelayOutput>,
    ) -> bool {
        let Ok(hash) = entry.try_hash() else {
            return false;
        };
        if self.keyring.is_accepted(&hash) {
            return false;
        }
        let mut all: Vec<Entry> = self.entries.iter().map(|(e, _)| e.clone()).collect();
        all.push(entry.clone());
        let Ok(keyring) = Keyring::replay(&all) else {
            return false;
        };
        if !keyring.is_accepted(&hash) {
            return false;
        }
        self.keyring = keyring;
        self.entries.push((entry, bytes.to_vec()));
        out.push(RelayOutput::Store(StoreOp::Keyring {
            room: self.room,
            entry: bytes.to_vec(),
        }));
        self.broadcast(
            from,
            &RelayFrame::Keyring {
                entry: bytes.to_vec(),
            },
            out,
        );
        self.close_removed(out);
        true
    }

    /// Closes every admitted connection whose member the keyring no longer lists.
    fn close_removed(&mut self, out: &mut Vec<RelayOutput>) {
        let removed: Vec<ConnectionId> = self
            .online
            .iter()
            .filter(|(_, member)| self.keyring.role(member).is_none())
            .map(|(&c, _)| c)
            .collect();
        for connection in removed {
            out.push(RelayOutput::Send {
                connection,
                frame: RelayFrame::Refused {
                    reason: RefusalReason::Removed,
                },
            });
            out.push(RelayOutput::Close { connection });
            self.leave(connection, out);
        }
    }

    /// An envelope of the expected kind, for this room, under an epoch key its keyring names.
    fn check_envelope(
        &self,
        envelope: &[u8],
        kind: EnvelopeKind,
    ) -> Result<EnvelopeHeader, RefusalReason> {
        let header = EnvelopeHeader::parse(envelope).map_err(|_| RefusalReason::Protocol)?;
        let known_key = self
            .keyring
            .epochs()
            .iter()
            .any(|e| e.number == header.epoch && e.key_id == header.key_id);
        if header.kind != kind || header.document_id != self.room.document_id() || !known_key {
            return Err(RefusalReason::Protocol);
        }
        Ok(header)
    }

    fn broadcast(
        &self,
        except: Option<ConnectionId>,
        frame: &RelayFrame,
        out: &mut Vec<RelayOutput>,
    ) {
        for &connection in self.online.keys() {
            if Some(connection) != except {
                out.push(RelayOutput::Send {
                    connection,
                    frame: frame.clone(),
                });
            }
        }
    }
}
