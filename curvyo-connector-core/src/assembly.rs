//! Assembles a checked job from its chunks (size, order, ASCII, hash).

use std::fmt::Write as _;

use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::message::{JobKind, RejectReason};

/// The largest job, in bytes (ADR 0035 §5).
pub const MAX_JOB_BYTES: u64 = 64 * 1024 * 1024;
/// The largest chunk text, in bytes.
pub const MAX_CHUNK_BYTES: usize = 256 * 1024;

/// A job that passed every check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Job {
    /// The job.
    pub job_id: Uuid,
    /// The name sent with it.
    pub name: String,
    /// Job or frame.
    pub kind: JobKind,
    /// The G-code text.
    pub text: String,
}

/// Why a chunk was not taken.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ChunkError {
    /// The chunk belongs to another job: answer with `Error`, keep the assembly.
    UnknownJob,
    /// The job is refused and the assembly is discarded.
    Reject(RejectReason),
}

/// A job whose chunks are arriving.
#[derive(Debug, Clone)]
pub(crate) struct Assembly {
    job_id: Uuid,
    name: String,
    kind: JobKind,
    bytes: u64,
    sha256: String,
    next_index: u32,
    text: String,
    pending_cr: bool,
}

impl Assembly {
    /// Starts an assembly for an announced job.
    ///
    /// # Errors
    /// [`RejectReason::TooLarge`] when `bytes` is above [`MAX_JOB_BYTES`].
    pub fn begin(
        job_id: Uuid,
        name: String,
        kind: JobKind,
        bytes: u64,
        sha256: String,
    ) -> Result<Self, RejectReason> {
        if bytes > MAX_JOB_BYTES {
            return Err(RejectReason::TooLarge);
        }
        Ok(Self {
            job_id,
            name,
            kind,
            bytes,
            sha256,
            next_index: 0,
            text: String::new(),
            pending_cr: false,
        })
    }

    /// The job being assembled.
    #[must_use]
    pub fn job_id(&self) -> Uuid {
        self.job_id
    }

    /// True once the announced number of bytes has arrived.
    #[must_use]
    pub fn is_full(&self) -> bool {
        self.text.len() as u64 == self.bytes
    }

    /// Takes the next chunk.
    ///
    /// # Errors
    /// [`ChunkError::UnknownJob`] for another `job_id`; otherwise a rejection (gap or repeat is
    /// `HashMismatch`, size is `TooLarge`, a bad byte is `NotAscii`).
    pub fn push(&mut self, job_id: Uuid, index: u32, text: &str) -> Result<(), ChunkError> {
        if job_id != self.job_id {
            return Err(ChunkError::UnknownJob);
        }
        if index != self.next_index {
            return Err(ChunkError::Reject(RejectReason::HashMismatch));
        }
        if text.len() > MAX_CHUNK_BYTES || (self.text.len() + text.len()) as u64 > self.bytes {
            return Err(ChunkError::Reject(RejectReason::TooLarge));
        }
        let Some(next) = self.next_index.checked_add(1) else {
            return Err(ChunkError::Reject(RejectReason::TooLarge));
        };
        self.check_ascii(text)?;
        self.text.push_str(text);
        self.next_index = next;
        Ok(())
    }

    fn check_ascii(&mut self, text: &str) -> Result<(), ChunkError> {
        let bad = ChunkError::Reject(RejectReason::NotAscii);
        for byte in text.bytes() {
            if self.pending_cr && byte != b'\n' {
                return Err(bad);
            }
            self.pending_cr = byte == b'\r';
            if !(byte == b'\n' || byte == b'\r' || (0x20..=0x7E).contains(&byte)) {
                return Err(bad);
            }
        }
        Ok(())
    }

    /// Checks the whole text and hands the job over.
    ///
    /// # Errors
    /// `NotAscii` for a lone `\r` at the end, `HashMismatch` when the text is short or its hash
    /// differs from the announced one.
    pub fn finish(self) -> Result<Job, RejectReason> {
        if self.pending_cr {
            return Err(RejectReason::NotAscii);
        }
        let hash = Sha256::digest(self.text.as_bytes())
            .iter()
            .fold(String::new(), |mut hex, b| {
                let _ = write!(hex, "{b:02x}");
                hex
            });
        if !self.is_full() || hash != self.sha256 {
            return Err(RejectReason::HashMismatch);
        }
        Ok(Job {
            job_id: self.job_id,
            name: self.name,
            kind: self.kind,
            text: self.text,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_chunk_index_that_would_overflow_is_too_large() {
        let mut assembly =
            Assembly::begin(Uuid::nil(), "a".to_owned(), JobKind::Job, 10, String::new()).unwrap();
        assembly.next_index = u32::MAX;
        assert_eq!(
            assembly.push(Uuid::nil(), u32::MAX, "G0"),
            Err(ChunkError::Reject(RejectReason::TooLarge))
        );
    }
}
