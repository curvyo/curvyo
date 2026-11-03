//! The plaintext formats inside a workspace room's sealed records (ADR 0037 decisions 5, 7 and
//! 10): the inner record header, the snapshot stream cut into fixed-size parts, the `Moved`
//! payload, and the rule that says when a workspace snapshot is due.
//!
//! Everything here is sealed before it leaves a client; the relay sees only the envelopes, their
//! sizes and their number.

/// The snapshot stream version this build writes and reads.
pub const SNAPSHOT_STREAM_VERSION: u8 = 1;
/// The size of every snapshot part's plaintext but the last: 4 MiB **(Proposal)**.
pub const PART_BYTES: usize = 4 * 1024 * 1024;
/// The last part's plaintext is padded to a multiple of this: 64 KiB **(Proposal)**.
pub const PART_PADDING: usize = 64 * 1024;

/// `snapshot_id [16], index u32, count u32, length u32`: the plaintext header of a part; `length`
/// is the stream bytes the part carries before its padding.
const PART_HEADER: usize = 16 + 4 + 4 + 4;
/// `stream [16], peer u64`.
const INNER_HEADER: usize = 16 + 8;
/// `stream [16], length u64`.
const ENTRY_HEADER: usize = 16 + 8;
/// `format_version u8, count u32`.
const STREAM_HEADER: usize = 1 + 4;
/// The first bytes of a `Moved` payload; an update's first four bytes are a short epoch length,
/// so the two never meet.
const MOVED_MAGIC: &[u8; 8] = b"CVMOVED1";

/// Why workspace stream bytes were refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum StreamError {
    /// Truncated, with trailing bytes, or otherwise not the format.
    #[error("the workspace stream is malformed")]
    Malformed,
    /// A part of another snapshot, out of order, or after the last.
    #[error("the snapshot part is out of order")]
    OutOfOrder,
    /// A stream of a newer version.
    #[error("the workspace stream is of a newer version")]
    FormatTooNew,
}

/// The inner header of a workspace update or asset record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InnerHeader {
    /// The document id, or the workspace id for the list.
    pub stream: [u8; 16],
    /// The Loro peer that wrote the record.
    pub peer: u64,
}

/// The plaintext of a workspace record: the inner header, then `payload`.
#[must_use]
pub fn encode_inner_record(stream: [u8; 16], peer: u64, payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(INNER_HEADER + payload.len());
    out.extend_from_slice(&stream);
    out.extend_from_slice(&peer.to_le_bytes());
    out.extend_from_slice(payload);
    out
}

/// The inverse of [`encode_inner_record`].
///
/// # Errors
/// [`StreamError::Malformed`] for bytes shorter than the header.
pub fn decode_inner_record(bytes: &[u8]) -> Result<(InnerHeader, &[u8]), StreamError> {
    let (stream, rest) = bytes
        .split_first_chunk::<16>()
        .ok_or(StreamError::Malformed)?;
    let (peer, payload) = rest
        .split_first_chunk::<8>()
        .ok_or(StreamError::Malformed)?;
    Ok((
        InnerHeader {
            stream: *stream,
            peer: u64::from_le_bytes(*peer),
        },
        payload,
    ))
}

/// One stream of a workspace snapshot: the list's snapshot or one document's container bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamEntry {
    /// The document id, or the workspace id for the list.
    pub stream: [u8; 16],
    /// Its bytes.
    pub bytes: Vec<u8>,
}

/// The snapshot stream: `format_version u8, count u32`, then per entry `stream [16], length u64,
/// bytes`. The list comes first by the caller's order.
///
/// # Panics
/// Does not panic in practice: more than `u32::MAX` entries do not fit in memory.
#[must_use]
pub fn encode_snapshot_stream(entries: &[StreamEntry]) -> Vec<u8> {
    let size = STREAM_HEADER
        + entries
            .iter()
            .map(|entry| ENTRY_HEADER + entry.bytes.len())
            .sum::<usize>();
    let mut out = Vec::with_capacity(size);
    out.push(SNAPSHOT_STREAM_VERSION);
    // invariant: a workspace holds far fewer than 2^32 documents.
    #[allow(clippy::expect_used)]
    let count = u32::try_from(entries.len()).expect("fewer than 2^32 entries");
    out.extend_from_slice(&count.to_le_bytes());
    for entry in entries {
        out.extend_from_slice(&entry.stream);
        out.extend_from_slice(&(entry.bytes.len() as u64).to_le_bytes());
        out.extend_from_slice(&entry.bytes);
    }
    out
}

/// The plaintexts of the parts of `stream`: each [`PART_BYTES`] long but the last, which is
/// padded with zeros to a multiple of [`PART_PADDING`]. Part boundaries do not follow entries.
///
/// # Panics
/// Does not panic in practice: a stream of 2^32 parts (16 PiB) does not fit in memory.
#[must_use]
pub fn split_snapshot(stream: &[u8], snapshot_id: [u8; 16]) -> Vec<Vec<u8>> {
    let capacity = PART_BYTES - PART_HEADER;
    let chunks: Vec<&[u8]> = if stream.is_empty() {
        vec![&[]]
    } else {
        stream.chunks(capacity).collect()
    };
    // invariant: see `# Panics`.
    #[allow(clippy::expect_used)]
    let count = u32::try_from(chunks.len()).expect("fewer than 2^32 parts");
    let mut parts = Vec::with_capacity(chunks.len());
    for (index, chunk) in (0_u32..).zip(&chunks) {
        let mut part = Vec::with_capacity(PART_BYTES);
        part.extend_from_slice(&snapshot_id);
        part.extend_from_slice(&index.to_le_bytes());
        part.extend_from_slice(&count.to_le_bytes());
        // invariant: a chunk is shorter than PART_BYTES.
        #[allow(clippy::cast_possible_truncation)]
        part.extend_from_slice(&(chunk.len() as u32).to_le_bytes());
        part.extend_from_slice(chunk);
        let padded = if index + 1 == count {
            part.len().div_ceil(PART_PADDING) * PART_PADDING
        } else {
            PART_BYTES
        };
        part.resize(padded, 0);
        parts.push(part);
    }
    parts
}

/// Reads the parts of one snapshot in order and hands out each entry as soon as its bytes are
/// complete (a join writes each document then, criterion 28).
#[derive(Debug, Default)]
pub struct SnapshotAssembler {
    /// The snapshot, its part count and the next part's index, once part 0 arrived.
    position: Option<([u8; 16], u32, u32)>,
    /// Stream bytes not yet consumed.
    pending: Vec<u8>,
    /// Entries still to read, once the stream header was read.
    remaining: Option<u32>,
}

impl SnapshotAssembler {
    /// Feeds the plaintext of the next part; returns the entries it completed.
    ///
    /// # Errors
    /// [`StreamError::OutOfOrder`], [`StreamError::Malformed`], [`StreamError::FormatTooNew`].
    pub fn push_part(&mut self, part: &[u8]) -> Result<Vec<StreamEntry>, StreamError> {
        let (id, rest) = part
            .split_first_chunk::<16>()
            .ok_or(StreamError::Malformed)?;
        let (index, rest) = read_u32(rest)?;
        let (count, rest) = read_u32(rest)?;
        let (length, rest) = read_u32(rest)?;
        let data = rest
            .get(..usize::try_from(length).map_err(|_| StreamError::Malformed)?)
            .ok_or(StreamError::Malformed)?;
        match self.position {
            None if index == 0 && count > 0 => self.position = Some((*id, count, 1)),
            Some((snapshot, total, next)) if snapshot == *id && total == count && next == index => {
                self.position = Some((snapshot, total, next + 1));
            }
            _ => return Err(StreamError::OutOfOrder),
        }
        self.pending.extend_from_slice(data);
        let entries = self.take_entries()?;
        if self.is_last_part() && (self.remaining != Some(0) || !self.pending.is_empty()) {
            return Err(StreamError::Malformed);
        }
        Ok(entries)
    }

    /// Whether every part arrived.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.is_last_part()
    }

    fn is_last_part(&self) -> bool {
        matches!(self.position, Some((_, total, next)) if next == total)
    }

    fn take_entries(&mut self) -> Result<Vec<StreamEntry>, StreamError> {
        let mut at = 0;
        if self.remaining.is_none() {
            let Some(header) = self.pending.get(..STREAM_HEADER) else {
                return Ok(Vec::new());
            };
            if header[0] > SNAPSHOT_STREAM_VERSION {
                return Err(StreamError::FormatTooNew);
            }
            if header[0] != SNAPSHOT_STREAM_VERSION {
                return Err(StreamError::Malformed);
            }
            self.remaining = Some(read_u32(&header[1..])?.0);
            at = STREAM_HEADER;
        }
        let mut entries = Vec::new();
        while self.remaining.is_some_and(|left| left > 0) {
            let Some(header) = self.pending.get(at..at + ENTRY_HEADER) else {
                break;
            };
            let (stream, length) = header.split_at(16);
            let length = u64::from_le_bytes(length.try_into().map_err(|_| StreamError::Malformed)?);
            let length = usize::try_from(length).map_err(|_| StreamError::Malformed)?;
            let start = at + ENTRY_HEADER;
            let Some(bytes) = self.pending.get(start..start.saturating_add(length)) else {
                break;
            };
            entries.push(StreamEntry {
                stream: stream.try_into().map_err(|_| StreamError::Malformed)?,
                bytes: bytes.to_vec(),
            });
            at = start + length;
            self.remaining = self.remaining.map(|left| left - 1);
        }
        self.pending.drain(..at);
        Ok(entries)
    }
}

fn read_u32(bytes: &[u8]) -> Result<(u32, &[u8]), StreamError> {
    let (value, rest) = bytes
        .split_first_chunk::<4>()
        .ok_or(StreamError::Malformed)?;
    Ok((u32::from_le_bytes(*value), rest))
}

/// The sealed payload a take-in appends to a document's old room: the document now lives in the
/// workspace (ADR 0037 decision 10).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Moved {
    /// The workspace that took the document in.
    pub workspace: [u8; 16],
    /// The document's id there.
    pub document: [u8; 16],
}

/// `"CVMOVED1" ‖ workspace ‖ document`.
#[must_use]
pub fn encode_moved(moved: &Moved) -> Vec<u8> {
    let mut out = Vec::with_capacity(MOVED_MAGIC.len() + 32);
    out.extend_from_slice(MOVED_MAGIC);
    out.extend_from_slice(&moved.workspace);
    out.extend_from_slice(&moved.document);
    out
}

/// The `Moved` payload `bytes` hold, `None` for any other record.
#[must_use]
pub fn decode_moved(bytes: &[u8]) -> Option<Moved> {
    let rest = bytes.strip_prefix(MOVED_MAGIC)?;
    let (workspace, document) = rest.split_first_chunk::<16>()?;
    Some(Moved {
        workspace: *workspace,
        document: (*document).try_into().ok()?,
    })
}

/// Whether a workspace snapshot is due: the tail since the last snapshot has reached
/// max(4 MiB, half the last snapshot's bytes) or max(500, the last snapshot's bytes / 8 KiB)
/// records **(Proposal)**. Upload stays about twice the bytes edited; a join reads at most about
/// 1.5 times the workspace.
#[must_use]
pub fn snapshot_due(tail_bytes: u64, tail_records: u64, last_snapshot_bytes: u64) -> bool {
    const MIN_BYTES: u64 = 4 * 1024 * 1024;
    const MIN_RECORDS: u64 = 500;
    const BYTES_PER_RECORD: u64 = 8 * 1024;
    tail_bytes >= MIN_BYTES.max(last_snapshot_bytes / 2)
        || tail_records >= MIN_RECORDS.max(last_snapshot_bytes / BYTES_PER_RECORD)
}
