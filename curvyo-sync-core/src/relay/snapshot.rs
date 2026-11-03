//! A room's newest snapshot, kept as one or more parts, and the parts staged for the next one
//! (ADR 0031 decision 10, ADR 0037 decision 7).

use crate::frame::{ConnectionId, RefusalReason, RoomId};

use super::StoreOp;

/// What a room keeps of its snapshots.
#[derive(Debug, Default)]
pub(super) struct Snapshots {
    /// The newest complete snapshot: the sequence it covers and its parts in order.
    current: Option<(u64, Vec<Vec<u8>>)>,
    /// The snapshot being uploaded, by one connection at a time.
    staged: Option<Staging>,
    /// Connections whose upload is being dropped because another one staged first.
    dropping: Vec<ConnectionId>,
}

/// The parts of the snapshot being uploaded.
#[derive(Debug)]
struct Staging {
    /// The connection uploading it; parts of other connections are dropped meanwhile.
    owner: ConnectionId,
    /// Its `covers_seq`.
    covers: u64,
    /// The parts so far.
    parts: Vec<Vec<u8>>,
}

/// What a staged part led to.
pub(super) enum Staged {
    /// Kept; more parts follow.
    Waiting,
    /// The last part completed a snapshot that replaces the stored one: prune up to `covers`.
    Swapped { covers: u64 },
    /// The last part completed a snapshot older than the stored one; it was dropped.
    Dropped,
    /// The part belongs to an upload that started while another connection's was under way;
    /// it was dropped.
    Ignored,
}

impl Snapshots {
    pub(super) fn restored(current: Option<(u64, Vec<Vec<u8>>)>) -> Self {
        Self {
            current,
            staged: None,
            dropping: Vec::new(),
        }
    }

    /// The sequence the stored snapshot covers, 0 without one.
    pub(super) fn covers(&self) -> u64 {
        self.current.as_ref().map_or(0, |(seq, _)| *seq)
    }

    /// The stored snapshot.
    pub(super) fn current(&self) -> Option<&(u64, Vec<Vec<u8>>)> {
        self.current.as_ref()
    }

    /// A single-envelope snapshot (`ClientFrame::Snapshot`): kept when it covers at least the
    /// stored one's records, since a rotation re-seals the same state under a new key (ADR 0031
    /// decision 6a). Returns whether it was kept.
    pub(super) fn replace(&mut self, covers_seq: u64, envelope: Vec<u8>) -> bool {
        if covers_seq < self.covers() {
            return false;
        }
        self.current = Some((covers_seq, vec![envelope]));
        true
    }

    /// One part of a multi-part snapshot from `owner`: its part 0 starts its staging over; the
    /// others must follow in order for the same `covers_seq`; the last swaps the parts in. While
    /// another connection stages, the part is dropped without a refusal: two members may reach
    /// the threshold at once, and the one that loses uploads again at its next threshold.
    #[allow(
        clippy::too_many_arguments,
        reason = "the room, the sender and the fields of one SnapshotPart frame"
    )]
    pub(super) fn stage(
        &mut self,
        room: RoomId,
        owner: ConnectionId,
        covers_seq: u64,
        index: u32,
        last: bool,
        envelope: Vec<u8>,
        out: &mut Vec<StoreOp>,
    ) -> Result<Staged, RefusalReason> {
        let busy = self.staged.as_ref().is_some_and(|s| s.owner != owner);
        if busy && index == 0 && !self.dropping.contains(&owner) {
            self.dropping.push(owner);
        }
        if self.dropping.contains(&owner) {
            if last || !busy && index == 0 {
                self.dropping.retain(|c| *c != owner);
            }
            if busy || index != 0 {
                return Ok(Staged::Ignored);
            }
        }
        if index == 0 {
            self.staged = Some(Staging {
                owner,
                covers: covers_seq,
                parts: Vec::new(),
            });
        }
        let Some(staging) = self.staged.as_mut() else {
            return Err(RefusalReason::Protocol);
        };
        let next = u32::try_from(staging.parts.len()).map_err(|_| RefusalReason::Protocol)?;
        if staging.covers != covers_seq || next != index {
            self.staged = None;
            out.push(StoreOp::DropStagedParts { room });
            return Err(RefusalReason::Protocol);
        }
        out.push(StoreOp::SnapshotPart {
            room,
            covers_seq,
            index,
            envelope: envelope.clone(),
        });
        staging.parts.push(envelope);
        if !last {
            return Ok(Staged::Waiting);
        }
        let Some(Staging { covers, parts, .. }) = self.staged.take() else {
            return Err(RefusalReason::Protocol);
        };
        if covers < self.covers() {
            out.push(StoreOp::DropStagedParts { room });
            return Ok(Staged::Dropped);
        }
        out.push(StoreOp::SnapshotParts {
            room,
            covers_seq: covers,
            count: index + 1,
        });
        self.current = Some((covers, parts));
        Ok(Staged::Swapped { covers })
    }

    /// `connection` left: its unfinished upload is dropped.
    pub(super) fn forget(
        &mut self,
        room: RoomId,
        connection: ConnectionId,
        out: &mut Vec<StoreOp>,
    ) {
        self.dropping.retain(|c| *c != connection);
        if self.staged.as_ref().is_some_and(|s| s.owner == connection) {
            self.staged = None;
            out.push(StoreOp::DropStagedParts { room });
        }
    }
}
