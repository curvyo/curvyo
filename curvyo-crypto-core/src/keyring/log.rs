//! The `keyring.log` byte format: a magic, a version and length-prefixed entries.

use crate::CryptoError;
use crate::codec::{Reader, Writer};
use crate::keyring::Entry;

const MAGIC: &[u8; 4] = b"CYKR";
/// The keyring log format version this build writes and reads (ADR 0010 consequences).
pub const LOG_VERSION: u8 = 1;

/// Encodes entries, in the given order, as a `keyring.log`.
///
/// # Errors
/// `Malformed` for an entry longer than `u32::MAX` bytes or a list over `u16::MAX`.
pub fn encode_log(entries: &[Entry]) -> Result<Vec<u8>, CryptoError> {
    let mut writer = Writer::default();
    writer.raw(MAGIC);
    writer.u8(LOG_VERSION);
    for entry in entries {
        let bytes = entry.encode()?;
        writer.u32(u32::try_from(bytes.len()).map_err(|_| CryptoError::Malformed)?);
        writer.raw(&bytes);
    }
    Ok(writer.bytes)
}

/// Decodes a `keyring.log`.
///
/// # Errors
/// `Malformed` for a wrong magic, truncation or a malformed entry; `UnknownVersion`.
pub fn decode_log(bytes: &[u8]) -> Result<Vec<Entry>, CryptoError> {
    let mut reader = Reader::new(bytes);
    if reader.take(4)? != MAGIC {
        return Err(CryptoError::Malformed);
    }
    let version = reader.u8()?;
    if version != LOG_VERSION {
        return Err(CryptoError::UnknownVersion(version));
    }
    let mut entries = Vec::new();
    while let Ok(len) = reader.u32() {
        let len = usize::try_from(len).map_err(|_| CryptoError::Malformed)?;
        entries.push(Entry::decode(reader.take(len)?)?);
    }
    reader.finish()?;
    Ok(entries)
}
