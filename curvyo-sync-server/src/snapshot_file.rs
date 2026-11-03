//! A room's `snapshot` file and the parts staged beside it in `snapshot.part/` (ADR 0031
//! decision 11, ADR 0037 decision 7).
//!
//! - `snapshot` version 2: `"CVYSNAP2"`, `covers_seq: u64`, `count: u32`, then per part `len:
//!   u32` and the signed envelope; little-endian; replaced by write-temp-then-rename. A version 1
//!   file (`covers_seq: u64` and one envelope) reads as one part.
//! - `snapshot.part/<index>`: one staged envelope per file, named by its index in eight decimal
//!   digits; part 0 clears the directory first; the startup scan removes it (an upload cut short
//!   is sent again by its client).

use std::fs::{self, File};
use std::io::{self, Write};
use std::path::Path;

use crate::store::{StoreError, io_err, prune_log, read_or_empty, replace, sync_dir};

pub(crate) const SNAPSHOT: &str = "snapshot";
const STAGING: &str = "snapshot.part";
const MAGIC: &[u8; 8] = b"CVYSNAP2";

/// A room's newest snapshot: the sequence it covers and its parts.
pub(crate) type Snapshot = (u64, Vec<Vec<u8>>);

/// Writes a snapshot of `parts` covering `covers_seq` and prunes the log it covers.
pub(crate) fn write(dir: &Path, covers_seq: u64, parts: &[Vec<u8>]) -> Result<(), StoreError> {
    let path = dir.join(SNAPSHOT);
    replace(dir, SNAPSHOT, &encode(covers_seq, parts, &path)?)?;
    prune_log(dir, covers_seq)
}

/// Stages part `index`; part 0 discards earlier staged parts.
pub(crate) fn stage(dir: &Path, index: u32, envelope: &[u8]) -> Result<(), StoreError> {
    let staging = dir.join(STAGING);
    if index == 0 {
        drop_staged(dir)?;
    }
    if !staging.is_dir() {
        fs::create_dir(&staging).map_err(io_err("create", &staging))?;
        sync_dir(dir)?;
    }
    let path = staging.join(format!("{index:08}"));
    let mut file = File::create(&path).map_err(io_err("create", &path))?;
    file.write_all(envelope).map_err(io_err("write", &path))?;
    file.sync_all().map_err(io_err("sync", &path))?;
    sync_dir(&staging)
}

/// Makes the `count` staged parts the snapshot covering `covers_seq`.
pub(crate) fn commit_staged(dir: &Path, covers_seq: u64, count: u32) -> Result<(), StoreError> {
    let staging = dir.join(STAGING);
    let mut parts = Vec::new();
    for index in 0..count {
        let path = staging.join(format!("{index:08}"));
        parts.push(fs::read(&path).map_err(io_err("read", &path))?);
    }
    write(dir, covers_seq, &parts)?;
    drop_staged(dir)
}

/// Removes the staged parts, if any.
pub(crate) fn drop_staged(dir: &Path) -> Result<(), StoreError> {
    let staging = dir.join(STAGING);
    match fs::remove_dir_all(&staging) {
        Ok(()) => sync_dir(dir),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(io_err("remove", &staging)(e)),
    }
}

/// The room's snapshot, `None` without one.
pub(crate) fn read(dir: &Path) -> Result<Option<Snapshot>, StoreError> {
    let path = dir.join(SNAPSHOT);
    decode(&read_or_empty(&path)?, &path)
}

fn encode(covers_seq: u64, parts: &[Vec<u8>], path: &Path) -> Result<Vec<u8>, StoreError> {
    let damaged = || StoreError::Damaged(path.to_owned());
    let mut bytes =
        Vec::with_capacity(8 + 8 + 4 + parts.iter().map(|p| 4 + p.len()).sum::<usize>());
    bytes.extend_from_slice(MAGIC);
    bytes.extend_from_slice(&covers_seq.to_le_bytes());
    let count = u32::try_from(parts.len()).map_err(|_| damaged())?;
    bytes.extend_from_slice(&count.to_le_bytes());
    for part in parts {
        let len = u32::try_from(part.len()).map_err(|_| damaged())?;
        bytes.extend_from_slice(&len.to_le_bytes());
        bytes.extend_from_slice(part);
    }
    Ok(bytes)
}

fn decode(bytes: &[u8], path: &Path) -> Result<Option<Snapshot>, StoreError> {
    let damaged = || StoreError::Damaged(path.to_owned());
    if bytes.is_empty() {
        return Ok(None);
    }
    let Some(rest) = bytes.strip_prefix(MAGIC) else {
        // Version 1: one envelope.
        let (covers, envelope) = bytes.split_first_chunk::<8>().ok_or_else(damaged)?;
        return Ok(Some((u64::from_le_bytes(*covers), vec![envelope.to_vec()])));
    };
    let (covers, rest) = rest.split_first_chunk::<8>().ok_or_else(damaged)?;
    let (count, mut rest) = rest.split_first_chunk::<4>().ok_or_else(damaged)?;
    let mut parts = Vec::new();
    for _ in 0..u32::from_le_bytes(*count) {
        let (len, tail) = rest.split_first_chunk::<4>().ok_or_else(damaged)?;
        let len = usize::try_from(u32::from_le_bytes(*len)).map_err(|_| damaged())?;
        let part = tail.get(..len).ok_or_else(damaged)?;
        parts.push(part.to_vec());
        rest = &tail[len..];
    }
    if !rest.is_empty() || parts.is_empty() {
        return Err(damaged());
    }
    Ok(Some((u64::from_le_bytes(*covers), parts)))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    /// The golden version 2 file; regenerate with `CURVYO_UPDATE_GOLDEN=1`.
    #[test]
    fn golden_snapshot_v2() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/snapshot_v2.bin");
        let bytes = encode(7, &[vec![1; 5], vec![2; 3]], &path).unwrap();
        if std::env::var_os("CURVYO_UPDATE_GOLDEN").is_some() {
            fs::write(&path, &bytes).unwrap();
        }
        assert_eq!(fs::read(&path).unwrap(), bytes, "snapshot_v2.bin differs");
        assert_eq!(
            decode(&bytes, &path).unwrap(),
            Some((7, vec![vec![1; 5], vec![2; 3]]))
        );
    }

    /// A version 1 file reads as one part; a cut-short or empty-part file is damaged.
    #[test]
    fn version_1_reads_as_one_part_and_damage_is_refused() {
        let path = Path::new("snapshot");
        let mut v1 = 4_u64.to_le_bytes().to_vec();
        v1.extend_from_slice(&[9; 6]);
        assert_eq!(decode(&v1, path).unwrap(), Some((4, vec![vec![9; 6]])));
        assert!(decode(&[1, 2], path).is_err());
        let v2 = encode(4, &[vec![1; 5]], path).unwrap();
        assert!(decode(&v2[..v2.len() - 1], path).is_err());
        assert!(decode(&encode(4, &[], path).unwrap(), path).is_err());
    }

    /// Staged parts become the snapshot on commit and the staging is gone; part 0 restarts it.
    #[test]
    fn staged_parts_are_committed_in_order() {
        let dir = tempfile::tempdir().unwrap();
        stage(dir.path(), 0, b"stale").unwrap();
        stage(dir.path(), 0, b"one").unwrap();
        stage(dir.path(), 1, b"two").unwrap();
        commit_staged(dir.path(), 3, 2).unwrap();
        assert_eq!(
            read(dir.path()).unwrap(),
            Some((3, vec![b"one".to_vec(), b"two".to_vec()]))
        );
        assert!(!dir.path().join(STAGING).exists());
        stage(dir.path(), 0, b"later").unwrap();
        drop_staged(dir.path()).unwrap();
        assert!(!dir.path().join(STAGING).exists());
    }
}
