//! The relay's durable file store (ADR 0031 decision 11): one subdirectory per room under the
//! data directory, holding `keyring.log`, the append-only `log` and the `snapshot`.
//!
//! Layout of a room directory (named by the room id in lowercase hex):
//! - `keyring.log`: the `keyring.log` format of `curvyo-crypto-core` (magic, version, then each
//!   entry as a little-endian `u32` length and its bytes), appended one entry at a time.
//! - `log`: records after the snapshot, each `seq: u64`, `kind: u8` (0 update, 1 asset),
//!   `len: u32`, then the envelope; little-endian.
//! - `snapshot` and the staged `snapshot.part/`: see `crate::snapshot_file`.
//!
//! Every append is synced before the relay's sends that follow it go out, so an acknowledged
//! record is on disk. A record or entry cut short by a crash (never acknowledged) is dropped by the
//! startup scan. The store holds only sealed bytes; it parses nothing it does not have to.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use curvyo_sync_core::{RecordKind, RoomId, StoreOp, StoredRecord, StoredRoom};

use crate::snapshot_file;

pub(crate) const KEYRING: &str = "keyring.log";
pub(crate) const LOG: &str = "log";
pub(crate) const TEMP_SUFFIX: &str = ".tmp";
/// `seq`, `kind` and `len` of a log record.
const RECORD_HEAD: usize = 8 + 1 + 4;

/// Why the store cannot be read or written.
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    /// The filesystem refused an operation.
    #[error("{action} {path}: {source}")]
    Io {
        /// What was tried.
        action: &'static str,
        /// On which path.
        path: PathBuf,
        /// The cause.
        source: io::Error,
    },
    /// A stored file is not in the store's format (not merely cut short at its end).
    #[error("{0} is damaged")]
    Damaged(PathBuf),
}

pub(crate) fn io_err(
    action: &'static str,
    path: &Path,
) -> impl FnOnce(io::Error) -> StoreError + use<> {
    let path = path.to_owned();
    move |source| StoreError::Io {
        action,
        path,
        source,
    }
}

/// The data directory and every room in it.
pub struct Store {
    root: PathBuf,
}

impl Store {
    /// Opens the data directory (creating it if missing) and reads every room back: the startup
    /// scan (criteria 20, 38). Ends cut short by a crash are truncated; leftover temporary files
    /// are removed; directories that are not rooms are left alone.
    ///
    /// # Errors
    /// See [`StoreError`].
    pub fn open(root: &Path) -> Result<(Self, Vec<StoredRoom>), StoreError> {
        fs::create_dir_all(root).map_err(io_err("create", root))?;
        let mut rooms = Vec::new();
        for item in fs::read_dir(root).map_err(io_err("read", root))? {
            let child = item.map_err(io_err("read", root))?;
            let path = child.path();
            let Some(id) = child.file_name().to_str().and_then(room_from_hex) else {
                continue;
            };
            if !path.is_dir() {
                continue;
            }
            if let Some(found) = crate::room_scan::scan_room(&path, id)? {
                rooms.push(found);
            }
        }
        rooms.sort_by_key(|r| r.room);
        Ok((
            Self {
                root: root.to_owned(),
            },
            rooms,
        ))
    }

    /// Carries out one write of the relay, synced to disk before it returns.
    ///
    /// # Errors
    /// See [`StoreError`].
    pub fn apply(&mut self, op: &StoreOp) -> Result<(), StoreError> {
        match op {
            StoreOp::Keyring { room, entry } => {
                let dir = self.room_dir(*room)?;
                let path = dir.join(KEYRING);
                let mut bytes = Vec::with_capacity(entry.len() + 4);
                if file_len(&path)? == 0 {
                    bytes.extend_from_slice(&keyring_header());
                }
                bytes.extend_from_slice(&len_u32(entry.len(), &path)?.to_le_bytes());
                bytes.extend_from_slice(entry);
                append(&path, &bytes)
            }
            StoreOp::Record { room, record } => {
                let dir = self.room_dir(*room)?;
                let path = dir.join(LOG);
                let bytes = encode_record(record, &path)?;
                append(&path, &bytes)
            }
            StoreOp::Snapshot {
                room,
                covers_seq,
                envelope,
            } => snapshot_file::write(
                &self.room_dir(*room)?,
                *covers_seq,
                std::slice::from_ref(envelope),
            ),
            StoreOp::SnapshotPart {
                room,
                index,
                envelope,
                ..
            } => snapshot_file::stage(&self.room_dir(*room)?, *index, envelope),
            StoreOp::SnapshotParts {
                room,
                covers_seq,
                count,
            } => snapshot_file::commit_staged(&self.room_dir(*room)?, *covers_seq, *count),
            StoreOp::DropStagedParts { room } => snapshot_file::drop_staged(&self.room_dir(*room)?),
        }
    }

    /// The room's directory, created (and its creation synced) if new.
    fn room_dir(&self, room: RoomId) -> Result<PathBuf, StoreError> {
        let dir = self.root.join(room_hex(room));
        if !dir.is_dir() {
            fs::create_dir(&dir).map_err(io_err("create", &dir))?;
            sync_dir(&self.root)?;
        }
        Ok(dir)
    }
}

/// The 32 lowercase hex digits naming a room's directory.
#[must_use]
pub fn room_hex(room: RoomId) -> String {
    use std::fmt::Write as _;
    room.0.iter().fold(String::with_capacity(32), |mut hex, b| {
        let _ = write!(hex, "{b:02x}");
        hex
    })
}

fn room_from_hex(name: &str) -> Option<RoomId> {
    if name.len() != 32 || !name.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')) {
        return None;
    }
    let mut id = [0; 16];
    for (i, byte) in id.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&name[2 * i..2 * i + 2], 16).ok()?;
    }
    Some(RoomId(id))
}

/// The magic and version that open every `keyring.log`.
pub(crate) fn keyring_header() -> Vec<u8> {
    // invariant: an empty entry list always encodes (it is the magic and the version byte).
    curvyo_crypto_core::keyring::encode_log(&[]).unwrap_or_default()
}

fn len_u32(len: usize, path: &Path) -> Result<u32, StoreError> {
    u32::try_from(len).map_err(|_| StoreError::Damaged(path.to_owned()))
}

fn kind_byte(kind: RecordKind, path: &Path) -> Result<u8, StoreError> {
    match kind {
        RecordKind::Update => Ok(0),
        RecordKind::Asset => Ok(1),
        RecordKind::Snapshot => Err(StoreError::Damaged(path.to_owned())),
    }
}

fn encode_record(record: &StoredRecord, path: &Path) -> Result<Vec<u8>, StoreError> {
    let mut bytes = Vec::with_capacity(RECORD_HEAD + record.envelope.len());
    bytes.extend_from_slice(&record.seq.to_le_bytes());
    bytes.push(kind_byte(record.kind, path)?);
    bytes.extend_from_slice(&len_u32(record.envelope.len(), path)?.to_le_bytes());
    bytes.extend_from_slice(&record.envelope);
    Ok(bytes)
}

fn file_len(path: &Path) -> Result<u64, StoreError> {
    match fs::metadata(path) {
        Ok(meta) => Ok(meta.len()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(0),
        Err(e) => Err(io_err("read", path)(e)),
    }
}

pub(crate) fn read_or_empty(path: &Path) -> Result<Vec<u8>, StoreError> {
    match fs::read(path) {
        Ok(bytes) => Ok(bytes),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(io_err("read", path)(e)),
    }
}

/// Appends `bytes` to `path` and syncs the data.
fn append(path: &Path, bytes: &[u8]) -> Result<(), StoreError> {
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(io_err("open", path))?;
    file.write_all(bytes).map_err(io_err("write", path))?;
    file.sync_data().map_err(io_err("sync", path))
}

/// Replaces `dir/name` with `bytes` by writing a synced temporary file and renaming it.
pub(crate) fn replace(dir: &Path, name: &str, bytes: &[u8]) -> Result<(), StoreError> {
    let temp = dir.join(format!("{name}{TEMP_SUFFIX}"));
    let target = dir.join(name);
    let mut file = File::create(&temp).map_err(io_err("create", &temp))?;
    file.write_all(bytes).map_err(io_err("write", &temp))?;
    file.sync_all().map_err(io_err("sync", &temp))?;
    drop(file);
    fs::rename(&temp, &target).map_err(io_err("rename", &temp))?;
    sync_dir(dir)
}

/// Drops every log record up to `covers_seq` (the snapshot holds them).
pub(crate) fn prune_log(dir: &Path, covers_seq: u64) -> Result<(), StoreError> {
    let path = dir.join(LOG);
    let (records, _) = decode_records(&read_or_empty(&path)?, &path)?;
    let mut bytes = Vec::new();
    for record in records.iter().filter(|r| r.seq > covers_seq) {
        bytes.extend_from_slice(&encode_record(record, &path)?);
    }
    replace(dir, LOG, &bytes)
}

/// Makes a directory's entries (a created file or directory, a rename) durable.
pub(crate) fn sync_dir(dir: &Path) -> Result<(), StoreError> {
    #[cfg(unix)]
    File::open(dir)
        .and_then(|d| d.sync_all())
        .map_err(io_err("sync", dir))?;
    #[cfg(not(unix))]
    let _ = dir;
    Ok(())
}

/// The complete log records and the length of the bytes holding them.
pub(crate) fn decode_records(
    bytes: &[u8],
    path: &Path,
) -> Result<(Vec<StoredRecord>, usize), StoreError> {
    let mut at = 0;
    let mut records = Vec::new();
    while let Some(head) = bytes.get(at..at + RECORD_HEAD) {
        let mut seq = [0; 8];
        seq.copy_from_slice(&head[..8]);
        let kind = match head[8] {
            0 => RecordKind::Update,
            1 => RecordKind::Asset,
            _ => return Err(StoreError::Damaged(path.to_owned())),
        };
        let mut len = [0; 4];
        len.copy_from_slice(&head[9..]);
        let len = u32::from_le_bytes(len) as usize;
        let Some(envelope) = bytes.get(at + RECORD_HEAD..at + RECORD_HEAD + len) else {
            break;
        };
        records.push(StoredRecord {
            seq: u64::from_le_bytes(seq),
            kind,
            envelope: envelope.to_vec(),
        });
        at += RECORD_HEAD + len;
    }
    Ok((records, at))
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROOM: RoomId = RoomId([0xab; 16]);

    fn record(seq: u64, kind: RecordKind, fill: u8) -> StoreOp {
        StoreOp::Record {
            room: ROOM,
            record: StoredRecord {
                seq,
                kind,
                envelope: vec![fill; 40],
            },
        }
    }

    fn keyring(fill: u8) -> StoreOp {
        StoreOp::Keyring {
            room: ROOM,
            entry: vec![fill; 70],
        }
    }

    fn reopen(root: &Path) -> Vec<StoredRoom> {
        Store::open(root).unwrap().1
    }

    #[test]
    fn writes_read_back_in_order() {
        let dir = tempfile::tempdir().unwrap();
        let (mut store, rooms) = Store::open(dir.path()).unwrap();
        assert_eq!(rooms.len(), 0);
        for op in [
            keyring(1),
            keyring(2),
            record(1, RecordKind::Update, 3),
            record(2, RecordKind::Asset, 4),
        ] {
            store.apply(&op).unwrap();
        }
        let rooms = reopen(dir.path());
        assert_eq!(rooms.len(), 1);
        assert_eq!(rooms[0].room, ROOM);
        assert_eq!(rooms[0].keyring, vec![vec![1; 70], vec![2; 70]]);
        assert_eq!(rooms[0].snapshot, None);
        assert_eq!(rooms[0].records.len(), 2);
        assert_eq!(rooms[0].records[1].kind, RecordKind::Asset);
        let keyring_file = fs::read(dir.path().join(room_hex(ROOM)).join(KEYRING)).unwrap();
        assert!(keyring_file.starts_with(&keyring_header()));
    }

    #[test]
    fn a_snapshot_replaces_the_old_one_and_prunes_the_log() {
        let dir = tempfile::tempdir().unwrap();
        let (mut store, _) = Store::open(dir.path()).unwrap();
        store.apply(&keyring(1)).unwrap();
        for seq in 1..=4 {
            store.apply(&record(seq, RecordKind::Update, 9)).unwrap();
        }
        for covers_seq in [2, 3] {
            store
                .apply(&StoreOp::Snapshot {
                    room: ROOM,
                    covers_seq,
                    envelope: vec![7; 30],
                })
                .unwrap();
        }
        let rooms = reopen(dir.path());
        assert_eq!(rooms[0].snapshot, Some((3, vec![7; 30])));
        let seqs: Vec<u64> = rooms[0].records.iter().map(|r| r.seq).collect();
        assert_eq!(seqs, vec![4]);
    }

    /// `specs/0094-workspace` criterion 48: a multi-part snapshot is staged, swapped in on its
    /// last part, read back after a restart as its parts; a staging cut short by a restart is
    /// dropped and the stored snapshot stays.
    #[test]
    fn snapshot_parts_survive_a_restart_and_a_cut_staging_is_dropped() {
        let dir = tempfile::tempdir().unwrap();
        let (mut store, _) = Store::open(dir.path()).unwrap();
        store.apply(&keyring(1)).unwrap();
        for seq in 1..=3 {
            store.apply(&record(seq, RecordKind::Update, 9)).unwrap();
        }
        for (index, fill) in [(0, 5_u8), (1, 6)] {
            store
                .apply(&StoreOp::SnapshotPart {
                    room: ROOM,
                    covers_seq: 2,
                    index,
                    envelope: vec![fill; 20],
                })
                .unwrap();
        }
        store
            .apply(&StoreOp::SnapshotParts {
                room: ROOM,
                covers_seq: 2,
                count: 2,
            })
            .unwrap();
        store
            .apply(&StoreOp::SnapshotPart {
                room: ROOM,
                covers_seq: 3,
                index: 0,
                envelope: vec![8; 20],
            })
            .unwrap();
        let rooms = reopen(dir.path());
        assert_eq!(rooms[0].snapshot, Some((2, vec![5; 20])));
        assert_eq!(rooms[0].snapshot_rest, vec![vec![6; 20]]);
        let seqs: Vec<u64> = rooms[0].records.iter().map(|r| r.seq).collect();
        assert_eq!(seqs, vec![3]);
        assert!(
            !dir.path()
                .join(room_hex(ROOM))
                .join("snapshot.part")
                .exists()
        );
    }

    #[test]
    fn a_cut_short_end_is_dropped_and_truncated() {
        let dir = tempfile::tempdir().unwrap();
        let (mut store, _) = Store::open(dir.path()).unwrap();
        store.apply(&keyring(1)).unwrap();
        store.apply(&record(1, RecordKind::Update, 3)).unwrap();
        let room = dir.path().join(room_hex(ROOM));
        let good_log = fs::read(room.join(LOG)).unwrap();
        let good_keyring = fs::read(room.join(KEYRING)).unwrap();
        append(
            &room.join(LOG),
            &[2, 0, 0, 0, 0, 0, 0, 0, 0, 40, 0, 0, 0, 5, 5],
        )
        .unwrap();
        append(&room.join(KEYRING), &[70, 0, 0, 0, 1, 2]).unwrap();
        let rooms = reopen(dir.path());
        assert_eq!(rooms[0].records.len(), 1);
        assert_eq!(rooms[0].keyring.len(), 1);
        assert_eq!(fs::read(room.join(LOG)).unwrap(), good_log);
        assert_eq!(fs::read(room.join(KEYRING)).unwrap(), good_keyring);
        // Appending after the repair continues a well-formed log.
        let (mut store, _) = Store::open(dir.path()).unwrap();
        store.apply(&record(2, RecordKind::Update, 4)).unwrap();
        assert_eq!(reopen(dir.path())[0].records.len(), 2);
    }

    #[test]
    fn records_a_crash_left_behind_a_snapshot_are_skipped() {
        let dir = tempfile::tempdir().unwrap();
        let (mut store, _) = Store::open(dir.path()).unwrap();
        store.apply(&keyring(1)).unwrap();
        for seq in 1..=3 {
            store.apply(&record(seq, RecordKind::Update, 9)).unwrap();
        }
        let room = dir.path().join(room_hex(ROOM));
        let mut snapshot = 2_u64.to_le_bytes().to_vec();
        snapshot.extend_from_slice(&[7; 10]);
        fs::write(room.join(snapshot_file::SNAPSHOT), snapshot).unwrap();
        fs::write(room.join("log.tmp"), b"half").unwrap();
        let rooms = reopen(dir.path());
        let seqs: Vec<u64> = rooms[0].records.iter().map(|r| r.seq).collect();
        assert_eq!(seqs, vec![3]);
        assert!(!room.join("log.tmp").exists());
    }

    #[test]
    fn a_damaged_log_is_an_error_and_foreign_entries_are_ignored() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("lost+found")).unwrap();
        fs::write(dir.path().join(".healthz"), b"").unwrap();
        let (mut store, rooms) = Store::open(dir.path()).unwrap();
        assert_eq!(rooms.len(), 0);
        store.apply(&keyring(1)).unwrap();
        let room = dir.path().join(room_hex(ROOM));
        fs::write(room.join(LOG), [1, 0, 0, 0, 0, 0, 0, 0, 9, 0, 0, 0, 0]).unwrap();
        assert!(matches!(
            Store::open(dir.path()),
            Err(StoreError::Damaged(_))
        ));
    }

    #[test]
    fn a_room_without_a_complete_keyring_entry_is_not_a_room() {
        let dir = tempfile::tempdir().unwrap();
        let room = dir.path().join(room_hex(ROOM));
        fs::create_dir(&room).unwrap();
        fs::write(room.join(KEYRING), &keyring_header()[..2]).unwrap();
        assert_eq!(reopen(dir.path()).len(), 0);
        let (mut store, _) = Store::open(dir.path()).unwrap();
        store.apply(&keyring(5)).unwrap();
        assert_eq!(reopen(dir.path())[0].keyring, vec![vec![5; 70]]);
    }

    #[test]
    fn room_names_are_hex() {
        assert_eq!(room_hex(RoomId([0x0f; 16])), "0f".repeat(16));
        assert_eq!(room_from_hex(&room_hex(ROOM)), Some(ROOM));
        assert_eq!(room_from_hex("AB".repeat(16).as_str()), None);
        assert_eq!(room_from_hex("lost+found"), None);
    }
}
