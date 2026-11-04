//! The relay's passkey backups on disk (ADR 0040 decision 13): one file per backup in
//! `backups/` under the data directory, named by the id in lowercase hex.
//!
//! A file is version `1` ‖ the 32-byte write key ‖ the sealed record, replaced by a synced
//! temporary file and a rename, so a crash leaves the old or the new file, never a mix. The
//! relay never opens a record.

use std::fs;
use std::path::{Path, PathBuf};

use curvyo_crypto_core::{BackupId, MemberId};
use curvyo_sync_core::StoredBackup;

use crate::store::{StoreError, TEMP_SUFFIX, io_err, replace, sync_dir};

/// The directory under the data directory.
pub(crate) const BACKUPS: &str = "backups";
/// The version byte of a backup file.
const FILE_VERSION: u8 = 1;

/// Every backup under `root`; leftover temporary files are removed.
///
/// # Errors
/// `Io`, or `Damaged` for a file that is not a backup file.
pub(crate) fn load(root: &Path) -> Result<Vec<StoredBackup>, StoreError> {
    let dir = root.join(BACKUPS);
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut found = Vec::new();
    for item in fs::read_dir(&dir).map_err(io_err("read", &dir))? {
        let path = item.map_err(io_err("read", &dir))?.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if name.ends_with(TEMP_SUFFIX) {
            fs::remove_file(&path).map_err(io_err("remove", &path))?;
            continue;
        }
        let Some(id) = id_from_hex(name) else {
            continue;
        };
        let bytes = fs::read(&path).map_err(io_err("read", &path))?;
        found.push(decode(id, &bytes).ok_or_else(|| StoreError::Damaged(path.clone()))?);
    }
    found.sort_by_key(|b| b.id);
    Ok(found)
}

/// Writes `backup`, synced before it returns.
///
/// # Errors
/// `Io`.
pub(crate) fn write(root: &Path, backup: &StoredBackup) -> Result<(), StoreError> {
    let dir = backups_dir(root)?;
    replace(&dir, &id_hex(backup.id), &encode(backup))
}

/// Deletes the backup `id`, synced before it returns.
///
/// # Errors
/// `Io`.
pub(crate) fn delete(root: &Path, id: BackupId) -> Result<(), StoreError> {
    let dir = backups_dir(root)?;
    let path = dir.join(id_hex(id));
    match fs::remove_file(&path) {
        Ok(()) => sync_dir(&dir),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(io_err("remove", &path)(error)),
    }
}

/// The bytes of a backup file.
#[must_use]
pub(crate) fn encode(backup: &StoredBackup) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(1 + 32 + backup.record.len());
    bytes.push(FILE_VERSION);
    bytes.extend_from_slice(&backup.write_key.0);
    bytes.extend_from_slice(&backup.record);
    bytes
}

/// A backup file's bytes read back; `None` for another version or a file without a record.
#[must_use]
pub(crate) fn decode(id: BackupId, bytes: &[u8]) -> Option<StoredBackup> {
    let (&version, rest) = bytes.split_first()?;
    let (write_key, record) = rest.split_first_chunk::<32>()?;
    (version == FILE_VERSION && !record.is_empty()).then(|| StoredBackup {
        id,
        write_key: MemberId(*write_key),
        record: record.to_vec(),
    })
}

fn backups_dir(root: &Path) -> Result<PathBuf, StoreError> {
    let dir = root.join(BACKUPS);
    if !dir.is_dir() {
        fs::create_dir(&dir).map_err(io_err("create", &dir))?;
        sync_dir(root)?;
    }
    Ok(dir)
}

fn id_hex(id: BackupId) -> String {
    crate::store::room_hex(curvyo_sync_core::RoomId(id.0))
}

fn id_from_hex(name: &str) -> Option<BackupId> {
    crate::store::room_from_hex(name).map(|room| BackupId(room.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn backup() -> StoredBackup {
        StoredBackup {
            id: BackupId([0x5b; 16]),
            write_key: MemberId([0x6c; 32]),
            record: vec![1, 2, 3, 4],
        }
    }

    /// The golden backup file v1; regenerate with `CURVYO_UPDATE_GOLDEN=1`.
    #[test]
    fn a_backup_file_matches_its_golden_bytes() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/backup_file_v1.bin");
        let bytes = encode(&backup());
        if std::env::var_os("CURVYO_UPDATE_GOLDEN").is_some() {
            fs::write(&path, &bytes).unwrap();
        }
        assert_eq!(fs::read(&path).unwrap(), bytes);
        assert_eq!(decode(backup().id, &bytes), Some(backup()));
        assert_eq!(decode(backup().id, &bytes[..33]), None);
        let mut newer = bytes.clone();
        newer[0] = 2;
        assert_eq!(decode(backup().id, &newer), None);
    }

    #[test]
    fn backups_are_written_replaced_deleted_and_read_back() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(load(dir.path()).unwrap(), Vec::new());
        write(dir.path(), &backup()).unwrap();
        let mut changed = backup();
        changed.record = vec![9];
        write(dir.path(), &changed).unwrap();
        fs::write(dir.path().join(BACKUPS).join("x.tmp"), b"half").unwrap();
        assert_eq!(load(dir.path()).unwrap(), vec![changed]);
        assert!(!dir.path().join(BACKUPS).join("x.tmp").exists());
        delete(dir.path(), backup().id).unwrap();
        delete(dir.path(), backup().id).unwrap();
        assert_eq!(load(dir.path()).unwrap(), Vec::new());
    }
}
