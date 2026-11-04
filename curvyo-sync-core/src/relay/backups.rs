//! The relay's passkey backups (ADR 0040 decision 13): sealed records under opaque ids, read by
//! id and written only with the write key the first put pinned, each write signed over a fresh
//! challenge so a captured write cannot be replayed.
//!
//! A backup connection sends one request and is closed after the answer. The relay never opens a
//! record: it holds the id, the write key, the bytes and their size.

use std::collections::BTreeMap;

use curvyo_crypto_core::{BackupId, BackupOp, MemberId, verify_backup_write};

use super::{RelayOutput, StoreOp};
use crate::frame::{ClientFrame, ConnectionId, FrameError, RefusalReason, RelayFrame};

/// The largest backup record the relay keeps: 4 KiB, far above backup record v1 (Proposal).
pub const BACKUP_RECORD_MAX: usize = 4096;

/// One stored backup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredBackup {
    /// The opaque id.
    pub id: BackupId,
    /// The write key the first put pinned.
    pub write_key: MemberId,
    /// The sealed record.
    pub record: Vec<u8>,
}

/// Every backup of one relay and the writes waiting for their signed frame.
#[derive(Debug, Default)]
pub struct Backups {
    stored: BTreeMap<BackupId, StoredBackup>,
    /// A write's id and the challenge nonce it must sign, by connection.
    writes: BTreeMap<ConnectionId, (BackupId, [u8; 32])>,
}

impl Backups {
    /// The relay's backups as the store read them at start.
    #[must_use]
    pub fn restore(stored: Vec<StoredBackup>) -> Self {
        Self {
            stored: stored.into_iter().map(|b| (b.id, b)).collect(),
            writes: BTreeMap::new(),
        }
    }

    /// The number of stored backups.
    #[must_use]
    pub fn count(&self) -> usize {
        self.stored.len()
    }

    /// Handles one message of a backup connection; `entropy` is the challenge nonce if this
    /// message opens a write.
    pub fn receive(
        &mut self,
        connection: ConnectionId,
        bytes: &[u8],
        entropy: [u8; 32],
    ) -> Vec<RelayOutput> {
        let mut out = Vec::new();
        let frame = match ClientFrame::decode(bytes) {
            Ok(frame) => frame,
            Err(FrameError::TooLarge) => return self.refuse(connection, RefusalReason::TooLarge),
            Err(FrameError::Malformed) => return self.refuse(connection, RefusalReason::Protocol),
        };
        let write = self.writes.remove(&connection);
        let verdict = match (write, frame) {
            (None, ClientFrame::BackupGet { id }) => match self.stored.get(&id) {
                Some(found) => {
                    let record = found.record.clone();
                    finish(connection, RelayFrame::BackupRecord { record }, &mut out);
                    Ok(())
                }
                None => Err(RefusalReason::Unknown),
            },
            (None, ClientFrame::BackupBegin { id }) => {
                self.writes.insert(connection, (id, entropy));
                out.push(RelayOutput::Send {
                    connection,
                    frame: RelayFrame::Challenge { nonce: entropy },
                });
                Ok(())
            }
            (
                Some((id, nonce)),
                ClientFrame::BackupPut {
                    write_key,
                    signature,
                    record,
                },
            ) => self
                .put(id, &nonce, write_key, &signature, record)
                .map(|op| {
                    out.push(RelayOutput::Store(op));
                    finish(connection, RelayFrame::BackupDone, &mut out);
                }),
            (Some((id, nonce)), ClientFrame::BackupDelete { signature }) => {
                self.delete(id, &nonce, &signature).map(|op| {
                    out.push(RelayOutput::Store(op));
                    finish(connection, RelayFrame::BackupDone, &mut out);
                })
            }
            _ => Err(RefusalReason::Protocol),
        };
        match verdict {
            Ok(()) => out,
            Err(reason) => self.refuse(connection, reason),
        }
    }

    /// Forgets a closed connection's pending write.
    pub fn disconnect(&mut self, connection: ConnectionId) {
        self.writes.remove(&connection);
    }

    /// Checks and keeps a put; the store op to carry out.
    fn put(
        &mut self,
        id: BackupId,
        nonce: &[u8; 32],
        write_key: MemberId,
        signature: &[u8; 64],
        record: Vec<u8>,
    ) -> Result<StoreOp, RefusalReason> {
        if record.len() > BACKUP_RECORD_MAX {
            return Err(RefusalReason::TooLarge);
        }
        if self
            .stored
            .get(&id)
            .is_some_and(|b| b.write_key != write_key)
        {
            return Err(RefusalReason::Forbidden);
        }
        verify_backup_write(&write_key, BackupOp::Put, id, nonce, &record, signature)
            .map_err(|_| RefusalReason::Forbidden)?;
        let stored = StoredBackup {
            id,
            write_key,
            record,
        };
        self.stored.insert(id, stored.clone());
        Ok(StoreOp::Backup(stored))
    }

    /// Checks and carries out a delete; the store op to carry out.
    fn delete(
        &mut self,
        id: BackupId,
        nonce: &[u8; 32],
        signature: &[u8; 64],
    ) -> Result<StoreOp, RefusalReason> {
        let stored = self.stored.get(&id).ok_or(RefusalReason::Unknown)?;
        verify_backup_write(
            &stored.write_key,
            BackupOp::Delete,
            id,
            nonce,
            &[],
            signature,
        )
        .map_err(|_| RefusalReason::Forbidden)?;
        self.stored.remove(&id);
        Ok(StoreOp::DropBackup { id })
    }

    fn refuse(&mut self, connection: ConnectionId, reason: RefusalReason) -> Vec<RelayOutput> {
        self.writes.remove(&connection);
        vec![
            RelayOutput::Send {
                connection,
                frame: RelayFrame::Refused { reason },
            },
            RelayOutput::Close { connection },
        ]
    }
}

/// The answer to a backup request, then the close.
fn finish(connection: ConnectionId, frame: RelayFrame, out: &mut Vec<RelayOutput>) {
    out.push(RelayOutput::Send { connection, frame });
    out.push(RelayOutput::Close { connection });
}
