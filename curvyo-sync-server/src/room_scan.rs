//! Reads one room of the relay's store back at startup (criteria 20, 38): the keyring, the
//! snapshot and the log, with ends cut short by a crash truncated and temporary files removed.

use std::fs::{self, OpenOptions};
use std::path::Path;

use curvyo_sync_core::{RoomId, StoredRoom};

use crate::snapshot_file::{self, SNAPSHOT};
use crate::store::{
    KEYRING, LOG, StoreError, TEMP_SUFFIX, decode_records, io_err, keyring_header, read_or_empty,
};

/// Truncates `path` to `len` bytes when a crash left a cut-short end after it.
fn truncate_to(path: &Path, len: usize, total: usize) -> Result<(), StoreError> {
    if len == total {
        return Ok(());
    }
    let file = OpenOptions::new()
        .write(true)
        .open(path)
        .map_err(io_err("open", path))?;
    file.set_len(len as u64).map_err(io_err("truncate", path))?;
    file.sync_all().map_err(io_err("sync", path))
}

/// Reads one room back; `None` for a directory a crash left without a complete keyring entry.
pub(crate) fn scan_room(dir: &Path, room: RoomId) -> Result<Option<StoredRoom>, StoreError> {
    for name in [SNAPSHOT, LOG] {
        let temp = dir.join(format!("{name}{TEMP_SUFFIX}"));
        if temp.exists() {
            fs::remove_file(&temp).map_err(io_err("remove", &temp))?;
        }
    }
    snapshot_file::drop_staged(dir)?;
    let keyring_path = dir.join(KEYRING);
    let bytes = read_or_empty(&keyring_path)?;
    let (keyring, kept) = decode_keyring(&bytes, &keyring_path)?;
    truncate_to(&keyring_path, kept, bytes.len())?;
    if keyring.is_empty() {
        return Ok(None);
    }

    let snapshot = snapshot_file::read(dir)?;
    let covers = snapshot.as_ref().map_or(0, |(seq, _)| *seq);
    let (snapshot, snapshot_rest) = match snapshot {
        Some((seq, parts)) => {
            let mut parts = parts.into_iter();
            (parts.next().map(|first| (seq, first)), parts.collect())
        }
        None => (None, Vec::new()),
    };

    let log_path = dir.join(LOG);
    let bytes = read_or_empty(&log_path)?;
    let (records, kept) = decode_records(&bytes, &log_path)?;
    truncate_to(&log_path, kept, bytes.len())?;
    // A crash between the snapshot's rename and the log's prune leaves covered records behind.
    let records = records.into_iter().filter(|r| r.seq > covers).collect();
    Ok(Some(StoredRoom {
        room,
        keyring,
        snapshot,
        snapshot_rest,
        records,
    }))
}

/// The complete keyring entries and the length of the bytes holding them.
fn decode_keyring(bytes: &[u8], path: &Path) -> Result<(Vec<Vec<u8>>, usize), StoreError> {
    let header = keyring_header();
    if bytes.len() < header.len() {
        return Ok((Vec::new(), 0));
    }
    if bytes[..header.len()] != header[..] {
        return Err(StoreError::Damaged(path.to_owned()));
    }
    let mut at = header.len();
    let mut entries = Vec::new();
    while let Some(len) = bytes.get(at..at + 4) {
        let mut word = [0; 4];
        word.copy_from_slice(len);
        let len = u32::from_le_bytes(word) as usize;
        let Some(entry) = bytes.get(at + 4..at + 4 + len) else {
            break;
        };
        entries.push(entry.to_vec());
        at += 4 + len;
    }
    Ok((entries, at))
}
