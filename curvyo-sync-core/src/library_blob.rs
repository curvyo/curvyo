//! The plaintext of a font blob in the device group room (ADR 0062 decision 3): its parts as
//! sealed `Asset` records and its entry in the room's framed snapshot.
//!
//! A blob is the bytes of one font file (or one licence file), keyed by its SHA-256. A part is
//! at most [`PART_BYTES`] long: `hash[32] | index u32 | count u32 | total u64 | bytes`, the last
//! part padded with zeros to a multiple of [`PART_PADDING`] like the snapshot's (the relay sees
//! only the padded length, so it cannot tell a known font by its exact size). Every part but
//! the last carries exactly `PART_BYTES - 48` bytes, so the length of the last follows from
//! `total` and the padding is cut off on reading. The hash is checked by the caller against the
//! assembled bytes; here only the shape and the size limit are, so a hostile blob is refused
//! before it is held.

use crate::workspace_stream::{PART_BYTES, PART_PADDING, StreamEntry, StreamError};

/// The stream id of a blob entry in the group room's framed snapshot.
pub const BLOB_STREAM_ID: [u8; 16] = *b"curvyo.fontblob1";
/// The bytes of a blob's SHA-256.
pub const BLOB_HASH_BYTES: usize = 32;
/// The largest blob a part sequence may announce: 16 MiB, the font file budget of ADR 0062
/// decision 6 (a larger announcement is refused with its first part).
pub const MAX_BLOB_BYTES: u64 = 16 * 1024 * 1024;

/// `hash [32], index u32, count u32, total u64`.
const PART_HEADER: usize = BLOB_HASH_BYTES + 4 + 4 + 8;
/// The blob bytes of every part but the last.
const PART_CAPACITY: usize = PART_BYTES - PART_HEADER;

/// One part of a blob as read from a record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlobPart<'a> {
    /// The blob's SHA-256.
    pub hash: [u8; BLOB_HASH_BYTES],
    /// This part's position.
    pub index: u32,
    /// How many parts the blob has.
    pub count: u32,
    /// The blob's length in bytes.
    pub total: u64,
    /// This part's bytes.
    pub bytes: &'a [u8],
}

/// The plaintexts of the parts of the blob `bytes` with SHA-256 `hash`: each at most
/// [`PART_BYTES`] long (the last padded to [`PART_PADDING`]), at least one (an empty blob is one
/// empty part).
///
/// # Panics
/// Does not panic in practice: a blob of 2^32 parts does not fit in memory.
#[must_use]
pub fn encode_blob_parts(hash: &[u8; BLOB_HASH_BYTES], bytes: &[u8]) -> Vec<Vec<u8>> {
    let chunks: Vec<&[u8]> = if bytes.is_empty() {
        vec![&[]]
    } else {
        bytes.chunks(PART_CAPACITY).collect()
    };
    // invariant: see `# Panics`.
    #[allow(clippy::expect_used)]
    let count = u32::try_from(chunks.len()).expect("fewer than 2^32 parts");
    (0_u32..)
        .zip(chunks)
        .map(|(index, chunk)| {
            let mut part = Vec::with_capacity(PART_HEADER + chunk.len());
            part.extend_from_slice(hash);
            part.extend_from_slice(&index.to_le_bytes());
            part.extend_from_slice(&count.to_le_bytes());
            part.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
            part.extend_from_slice(chunk);
            if index + 1 == count {
                part.resize(part.len().div_ceil(PART_PADDING) * PART_PADDING, 0);
            }
            part
        })
        .collect()
}

/// Reads one part.
///
/// # Errors
/// [`StreamError::Malformed`] for a short part, a part index past the count, a part longer than
/// [`PART_BYTES`] or than its place in the blob allows, bytes that do not fill the part its
/// `total` says, a padding that is not zero, or a blob announced over [`MAX_BLOB_BYTES`].
pub fn decode_blob_part(part: &[u8]) -> Result<BlobPart<'_>, StreamError> {
    if part.len() < PART_HEADER || part.len() > PART_BYTES {
        return Err(StreamError::Malformed);
    }
    let (head, bytes) = part.split_at(PART_HEADER);
    let word = |at: usize, len: usize| -> Result<&[u8], StreamError> {
        head.get(at..at + len).ok_or(StreamError::Malformed)
    };
    let hash: [u8; BLOB_HASH_BYTES] = word(0, BLOB_HASH_BYTES)?
        .try_into()
        .map_err(|_| StreamError::Malformed)?;
    let index = u32::from_le_bytes(
        word(32, 4)?
            .try_into()
            .map_err(|_| StreamError::Malformed)?,
    );
    let count = u32::from_le_bytes(
        word(36, 4)?
            .try_into()
            .map_err(|_| StreamError::Malformed)?,
    );
    let total = u64::from_le_bytes(
        word(40, 8)?
            .try_into()
            .map_err(|_| StreamError::Malformed)?,
    );
    if count == 0
        || index >= count
        || total > MAX_BLOB_BYTES
        || u64::from(count) != total.div_ceil(PART_CAPACITY as u64).max(1)
    {
        return Err(StreamError::Malformed);
    }
    // The bytes this part must carry: a full part, or what is left for the last one.
    let before = u64::from(index) * PART_CAPACITY as u64;
    let own = if index + 1 == count {
        total.checked_sub(before).ok_or(StreamError::Malformed)?
    } else {
        PART_CAPACITY as u64
    };
    let own = usize::try_from(own).map_err(|_| StreamError::Malformed)?;
    let (bytes, padding) = bytes.split_at_checked(own).ok_or(StreamError::Malformed)?;
    if padding.iter().any(|byte| *byte != 0) || padding.len() >= PART_PADDING {
        return Err(StreamError::Malformed);
    }
    Ok(BlobPart {
        hash,
        index,
        count,
        total,
        bytes,
    })
}

/// Collects the parts of one blob in order.
#[derive(Debug)]
pub struct BlobAssembler {
    hash: [u8; BLOB_HASH_BYTES],
    count: u32,
    total: u64,
    next: u32,
    bytes: Vec<u8>,
}

impl BlobAssembler {
    /// Starts with the blob's first part, which is taken into the assembly: feed part 1 next.
    /// A blob of one part is complete at once ([`is_complete`](Self::is_complete)).
    ///
    /// # Errors
    /// [`StreamError::OutOfOrder`] when `part` is not part 0; the errors of
    /// [`decode_blob_part`].
    pub fn start(part: &[u8]) -> Result<Self, StreamError> {
        let first = decode_blob_part(part)?;
        if first.index != 0 {
            return Err(StreamError::OutOfOrder);
        }
        Ok(Self {
            hash: first.hash,
            count: first.count,
            total: first.total,
            next: 1,
            bytes: first.bytes.to_vec(),
        })
    }

    /// Whether every part has arrived (a blob of one part, right after [`start`](Self::start)).
    #[must_use]
    pub const fn is_complete(&self) -> bool {
        self.next >= self.count
    }

    /// The index of the part the assembler expects next; a part below it was seen already.
    #[must_use]
    pub const fn next_index(&self) -> u32 {
        self.next
    }

    /// The blob's announced length.
    #[must_use]
    pub const fn total(&self) -> u64 {
        self.total
    }

    /// The whole blob once complete, `None` before; it leaves the assembler.
    pub fn take(&mut self) -> Option<Vec<u8>> {
        self.is_complete().then(|| std::mem::take(&mut self.bytes))
    }

    /// The blob's SHA-256 this assembler collects.
    #[must_use]
    pub const fn hash(&self) -> &[u8; BLOB_HASH_BYTES] {
        &self.hash
    }

    /// Feeds the next part; the whole blob once the last part arrived.
    ///
    /// # Errors
    /// [`StreamError::OutOfOrder`] for a part of another blob or not the next;
    /// [`StreamError::Malformed`] when the bytes do not add up to the announced length.
    pub fn push(&mut self, part: &[u8]) -> Result<Option<Vec<u8>>, StreamError> {
        let part = decode_blob_part(part)?;
        if part.hash != self.hash
            || part.count != self.count
            || part.total != self.total
            || part.index != self.next
        {
            return Err(StreamError::OutOfOrder);
        }
        self.bytes.extend_from_slice(part.bytes);
        if u64::try_from(self.bytes.len()).map_err(|_| StreamError::Malformed)? > self.total {
            return Err(StreamError::Malformed);
        }
        self.next += 1;
        if self.next < self.count {
            return Ok(None);
        }
        if u64::try_from(self.bytes.len()).ok() != Some(self.total) {
            return Err(StreamError::Malformed);
        }
        Ok(Some(std::mem::take(&mut self.bytes)))
    }
}

/// A blob as an entry of the group room's framed snapshot: `hash[32] | bytes`.
#[must_use]
pub fn encode_blob_entry(hash: &[u8; BLOB_HASH_BYTES], bytes: &[u8]) -> StreamEntry {
    let mut entry = Vec::with_capacity(BLOB_HASH_BYTES + bytes.len());
    entry.extend_from_slice(hash);
    entry.extend_from_slice(bytes);
    StreamEntry {
        stream: BLOB_STREAM_ID,
        bytes: entry,
    }
}

/// The inverse of [`encode_blob_entry`]; `None` for another stream, a short entry or a blob over
/// [`MAX_BLOB_BYTES`].
#[must_use]
pub fn decode_blob_entry(entry: &StreamEntry) -> Option<([u8; BLOB_HASH_BYTES], &[u8])> {
    if entry.stream != BLOB_STREAM_ID {
        return None;
    }
    let (hash, bytes) = entry.bytes.split_first_chunk::<BLOB_HASH_BYTES>()?;
    (u64::try_from(bytes.len()).ok()? <= MAX_BLOB_BYTES).then_some((*hash, bytes))
}
