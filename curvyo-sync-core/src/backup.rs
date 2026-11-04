//! The client side of one passkey backup exchange with the relay (ADR 0040 decision 13): read,
//! write or delete the record a passkey's PRF output names, over one connection.

use curvyo_crypto_core::{BackupContent, BackupKeys, BackupOp, CryptoError};

use crate::frame::{ClientFrame, RefusalReason, RelayFrame};

/// How one exchange ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackupOutcome {
    /// The record was read and opened.
    Read(BackupContent),
    /// The record is stored (put) or gone (delete).
    Done,
    /// The relay refused: `Unknown` means no backup for this passkey on this server.
    Refused(RefusalReason),
    /// The relay sent a record this passkey does not open, or a frame out of order.
    Damaged,
    /// The connection ended before an answer.
    Failed,
}

#[derive(Debug)]
enum Request {
    Read,
    Put(Vec<u8>),
    Delete,
}

/// One exchange: the first frame to send, the answers to the relay's frames, the outcome.
pub struct BackupExchange {
    keys: BackupKeys,
    request: Request,
    outcome: Option<BackupOutcome>,
}

impl BackupExchange {
    /// Reads the record of the passkey whose PRF output is `prf_output`.
    #[must_use]
    pub fn read(prf_output: &[u8; 32]) -> Self {
        Self::new(prf_output, Request::Read)
    }

    /// Stores `content` sealed with `nonce` (24 fresh bytes) as the passkey's record.
    ///
    /// # Errors
    /// `Malformed` for a server address or name over its limit.
    pub fn write(
        prf_output: &[u8; 32],
        content: &BackupContent,
        nonce: &[u8; 24],
    ) -> Result<Self, CryptoError> {
        let keys = BackupKeys::derive(prf_output);
        let record = keys.seal(content, nonce)?;
        Ok(Self {
            keys,
            request: Request::Put(record),
            outcome: None,
        })
    }

    /// Deletes the passkey's record.
    #[must_use]
    pub fn delete(prf_output: &[u8; 32]) -> Self {
        Self::new(prf_output, Request::Delete)
    }

    fn new(prf_output: &[u8; 32], request: Request) -> Self {
        Self {
            keys: BackupKeys::derive(prf_output),
            request,
            outcome: None,
        }
    }

    /// The frame that opens the connection.
    #[must_use]
    pub fn first_frame(&self) -> Vec<u8> {
        let id = self.keys.id();
        match self.request {
            Request::Read => ClientFrame::BackupGet { id },
            Request::Put(_) | Request::Delete => ClientFrame::BackupBegin { id },
        }
        .encode()
    }

    /// Takes one relay frame; returns the frame to answer with, if any.
    pub fn receive(&mut self, bytes: &[u8]) -> Option<Vec<u8>> {
        if self.outcome.is_some() {
            return None;
        }
        let frame = RelayFrame::decode(bytes);
        let (answer, outcome) = match (&self.request, frame) {
            (_, Ok(RelayFrame::Refused { reason })) => (None, BackupOutcome::Refused(reason)),
            (Request::Read, Ok(RelayFrame::BackupRecord { record })) => {
                let outcome = self
                    .keys
                    .open(&record)
                    .map_or(BackupOutcome::Damaged, BackupOutcome::Read);
                (None, outcome)
            }
            (Request::Put(record), Ok(RelayFrame::Challenge { nonce })) => {
                let frame = ClientFrame::BackupPut {
                    write_key: self.keys.write_key(),
                    signature: self.keys.sign_write(BackupOp::Put, &nonce, record),
                    record: record.clone(),
                };
                return Some(frame.encode());
            }
            (Request::Delete, Ok(RelayFrame::Challenge { nonce })) => {
                let frame = ClientFrame::BackupDelete {
                    signature: self.keys.sign_write(BackupOp::Delete, &nonce, &[]),
                };
                return Some(frame.encode());
            }
            (Request::Put(_) | Request::Delete, Ok(RelayFrame::BackupDone)) => {
                (None, BackupOutcome::Done)
            }
            _ => (None, BackupOutcome::Damaged),
        };
        self.outcome = Some(outcome);
        answer
    }

    /// The connection ended: an exchange without an answer failed.
    pub fn closed(&mut self) {
        self.outcome.get_or_insert(BackupOutcome::Failed);
    }

    /// The outcome once the exchange ended; taken once.
    pub fn take_outcome(&mut self) -> Option<BackupOutcome> {
        self.outcome.take()
    }

    /// Whether the exchange ended.
    #[must_use]
    pub fn finished(&self) -> bool {
        self.outcome.is_some()
    }
}
