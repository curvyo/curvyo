//! The recovery copies on disk (`specs/0136-autosave-and-restore` criteria 2, 3, 9 and 11, ADR
//! 0079 decisions 2, 3, 6 and 7): one folder per session key with the copy, its assets written
//! once each, the record, and a lock file held while the session runs. Bytes only: the record
//! and the container are read by the caller.
//!
//! The folder lives in the app's local data folder, never in a synced one, so the lock file of
//! ADR 0004 §7's "no lock files" does not apply here.

use std::fs::{self, File, OpenOptions, TryLockError};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use crate::filesystem::{FsError, read_optional, write_atomic};

/// The copy's container.
const COPY_FILE: &str = "copy.curvyo";
/// The record beside it, written last.
const RECORD_FILE: &str = "recovery.json";
/// The lock a running session holds.
const LOCK_FILE: &str = "lock";
/// The folder of assets, written before the copy that refers to them.
const ASSETS_DIR: &str = "assets";

/// Why a recovery operation failed.
#[derive(Debug, thiserror::Error)]
pub enum RecoveryError {
    /// A key or asset name that is not what the app makes.
    #[error("not a recovery key or asset name")]
    BadName,
    /// Another running session holds this key.
    #[error("the recovery copy is held by a running session")]
    Held,
    /// A file operation failed.
    #[error(transparent)]
    Fs(#[from] FsError),
    /// A raw I/O error (listing, deleting, locking).
    #[error("recovery folder: {0}")]
    Io(#[from] std::io::Error),
}

/// A held lock: the session runs while it lives. Dropping it releases the folder.
#[derive(Debug)]
pub struct SessionLock {
    _file: File,
}

/// One folder as the listing finds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListedCopy {
    /// The session key.
    pub key: String,
    /// `recovery.json`, `None` when it is missing.
    pub record: Option<Vec<u8>>,
    /// Size of `copy.curvyo` and the assets.
    pub bytes: u64,
    /// Modification time of `recovery.json` or the copy, ms since the epoch (0 when unknown).
    pub modified_ms: u64,
    /// A running session holds it.
    pub locked: bool,
}

/// The recovery folder.
#[derive(Debug, Clone)]
pub struct RecoveryFolder {
    root: PathBuf,
}

/// A key is the 32 hex digits the page makes.
fn check_key(key: &str) -> Result<(), RecoveryError> {
    if key.len() == 32 && key.bytes().all(|b| b.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(RecoveryError::BadName)
    }
}

/// An asset name is `<name>` or `derived/<name>` with name characters only.
fn check_asset(name: &str) -> Result<(), RecoveryError> {
    let plain = |part: &str| {
        !part.is_empty()
            && !part.starts_with('.')
            && part
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'_' || b == b'-')
    };
    let ok = match name.split_once('/') {
        Some(("derived", rest)) => plain(rest),
        Some(_) => false,
        None => plain(name),
    };
    if ok {
        Ok(())
    } else {
        Err(RecoveryError::BadName)
    }
}

fn modified_ms(path: &Path) -> u64 {
    fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(0))
}

/// The size of all files below `dir`.
fn tree_bytes(dir: &Path) -> u64 {
    let Ok(entries) = fs::read_dir(dir) else {
        return 0;
    };
    entries
        .flatten()
        .map(|entry| match entry.metadata() {
            Ok(meta) if meta.is_dir() => tree_bytes(&entry.path()),
            Ok(meta) => meta.len(),
            Err(_) => 0,
        })
        .sum()
}

fn tree_files(dir: &Path, prefix: &str, out: &mut Vec<(String, PathBuf)>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let path = entry.path();
        if path.is_dir() {
            tree_files(&path, &format!("{prefix}{name}/"), out);
        } else {
            out.push((format!("{prefix}{name}"), path));
        }
    }
}

impl RecoveryFolder {
    /// The recovery folder at `root` (made on first write).
    #[must_use]
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    /// Where the copies are kept, for the help line.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.root
    }

    fn dir(&self, key: &str) -> Result<PathBuf, RecoveryError> {
        check_key(key)?;
        Ok(self.root.join(key))
    }

    /// Starts a session's copy folder and takes its lock (Restore takes the lock of the copy it
    /// restores).
    ///
    /// # Errors
    /// [`RecoveryError::Held`] when a running session holds the key; the I/O error otherwise.
    pub fn begin(&self, key: &str) -> Result<SessionLock, RecoveryError> {
        let dir = self.dir(key)?;
        fs::create_dir_all(&dir)?;
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(dir.join(LOCK_FILE))?;
        match file.try_lock() {
            Ok(()) => Ok(SessionLock { _file: file }),
            Err(TryLockError::WouldBlock) => Err(RecoveryError::Held),
            Err(TryLockError::Error(error)) => Err(error.into()),
        }
    }

    /// Writes one asset once: an asset that is there is left alone (assets are content
    /// addressed).
    ///
    /// # Errors
    /// [`RecoveryError::BadName`] or the write error.
    pub fn write_asset(&self, key: &str, name: &str, bytes: &[u8]) -> Result<(), RecoveryError> {
        check_asset(name)?;
        let path = self.dir(key)?.join(ASSETS_DIR).join(name);
        if path.is_file() {
            return Ok(());
        }
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        write_atomic(&path, bytes)?;
        Ok(())
    }

    /// Writes the copy, then the record: each atomic, so the copy is always one complete write
    /// and the record names a copy that exists.
    ///
    /// # Errors
    /// The write error; the previous copy stays.
    pub fn write_copy(&self, key: &str, copy: &[u8], record: &[u8]) -> Result<(), RecoveryError> {
        let dir = self.dir(key)?;
        fs::create_dir_all(&dir)?;
        write_atomic(&dir.join(COPY_FILE), copy)?;
        write_atomic(&dir.join(RECORD_FILE), record)?;
        Ok(())
    }

    /// Deletes the record, the copy and the assets, and keeps the lock (Save: the session runs
    /// on under the same key).
    ///
    /// # Errors
    /// The I/O error of a file that exists and cannot be removed.
    pub fn delete_copy(&self, key: &str) -> Result<(), RecoveryError> {
        let dir = self.dir(key)?;
        remove_if_there(&dir.join(RECORD_FILE))?;
        remove_if_there(&dir.join(COPY_FILE))?;
        match fs::remove_dir_all(dir.join(ASSETS_DIR)) {
            Err(error) if error.kind() != ErrorKind::NotFound => Err(error.into()),
            _ => Ok(()),
        }
    }

    /// Deletes the whole folder (Discard, a clean Close); the caller dropped the lock first.
    ///
    /// # Errors
    /// The I/O error of a folder that exists and cannot be removed.
    pub fn delete_all(&self, key: &str) -> Result<(), RecoveryError> {
        match fs::remove_dir_all(self.dir(key)?) {
            Err(error) if error.kind() != ErrorKind::NotFound => Err(error.into()),
            _ => Ok(()),
        }
    }

    /// Every folder: its record, size and lock state. An unlocked folder without a record is a
    /// leftover of a clean session and is removed silently, never listed (ADR 0079 §7).
    #[must_use]
    pub fn list(&self) -> Vec<ListedCopy> {
        let Ok(entries) = fs::read_dir(&self.root) else {
            return Vec::new();
        };
        let mut listed = Vec::new();
        for entry in entries.flatten() {
            let key = entry.file_name().to_string_lossy().into_owned();
            let Ok(dir) = self.dir(&key) else { continue };
            if !dir.is_dir() {
                continue;
            }
            let locked = is_locked(&dir);
            let record = read_optional(&dir.join(RECORD_FILE)).ok().flatten();
            if record.is_none() && !locked {
                let _ = fs::remove_dir_all(&dir);
                continue;
            }
            let modified =
                modified_ms(&dir.join(RECORD_FILE)).max(modified_ms(&dir.join(COPY_FILE)));
            let bytes = tree_bytes(&dir.join(ASSETS_DIR))
                + fs::metadata(dir.join(COPY_FILE)).map_or(0, |m| m.len());
            listed.push(ListedCopy {
                key,
                record,
                bytes,
                modified_ms: modified,
                locked,
            });
        }
        listed
    }

    /// The copy's container bytes.
    ///
    /// # Errors
    /// The read error (a missing copy included).
    pub fn read_copy(&self, key: &str) -> Result<Vec<u8>, RecoveryError> {
        Ok(fs::read(self.dir(key)?.join(COPY_FILE))?)
    }

    /// The names of the copy's assets below `assets/` (`<name>` or `derived/<name>`).
    ///
    /// # Errors
    /// [`RecoveryError::BadName`] for a bad key.
    pub fn asset_names(&self, key: &str) -> Result<Vec<String>, RecoveryError> {
        let mut files = Vec::new();
        tree_files(&self.dir(key)?.join(ASSETS_DIR), "", &mut files);
        Ok(files.into_iter().map(|(name, _)| name).collect())
    }

    /// One asset's bytes.
    ///
    /// # Errors
    /// [`RecoveryError::BadName`] or the read error; the caller opens the project without it.
    pub fn read_asset(&self, key: &str, name: &str) -> Result<Vec<u8>, RecoveryError> {
        check_asset(name)?;
        Ok(fs::read(self.dir(key)?.join(ASSETS_DIR).join(name))?)
    }
}

fn remove_if_there(path: &Path) -> Result<(), RecoveryError> {
    match fs::remove_file(path) {
        Err(error) if error.kind() != ErrorKind::NotFound => Err(error.into()),
        _ => Ok(()),
    }
}

/// Whether a running session holds the folder's lock.
fn is_locked(dir: &Path) -> bool {
    let Ok(file) = OpenOptions::new().write(true).open(dir.join(LOCK_FILE)) else {
        return false;
    };
    matches!(file.try_lock(), Err(TryLockError::WouldBlock))
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: &str = "00112233445566778899aabbccddeeff";
    const OTHER: &str = "ffeeddccbbaa99887766554433221100";

    fn folder() -> (tempfile::TempDir, RecoveryFolder) {
        let dir = tempfile::tempdir().unwrap();
        let folder = RecoveryFolder::new(dir.path().join("recovery"));
        (dir, folder)
    }

    #[test]
    fn a_copy_is_written_listed_and_read_back_with_its_assets() {
        let (_dir, folder) = folder();
        let lock = folder.begin(KEY).unwrap();
        folder.write_asset(KEY, "abc.png", b"picture").unwrap();
        folder
            .write_asset(KEY, "derived/def.png", b"derived")
            .unwrap();
        folder.write_copy(KEY, b"copy-1", b"{\"r\":1}").unwrap();
        drop(lock);
        let listed = folder.list();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].key, KEY);
        assert_eq!(listed[0].record.as_deref(), Some(&b"{\"r\":1}"[..]));
        assert_eq!(listed[0].bytes, 6 + 7 + 7);
        assert!(!listed[0].locked);
        assert_eq!(folder.read_copy(KEY).unwrap(), b"copy-1");
        let mut names = folder.asset_names(KEY).unwrap();
        names.sort();
        assert_eq!(
            names,
            vec!["abc.png".to_owned(), "derived/def.png".to_owned()]
        );
        assert_eq!(
            folder.read_asset(KEY, "derived/def.png").unwrap(),
            b"derived"
        );
    }

    #[test]
    fn a_new_write_replaces_the_copy_whole_and_an_asset_is_written_once() {
        let (_dir, folder) = folder();
        let _lock = folder.begin(KEY).unwrap();
        folder.write_asset(KEY, "abc.png", b"first").unwrap();
        folder.write_asset(KEY, "abc.png", b"second").unwrap();
        folder.write_copy(KEY, b"one", b"{}").unwrap();
        folder.write_copy(KEY, b"two", b"{}").unwrap();
        assert_eq!(folder.read_copy(KEY).unwrap(), b"two");
        assert_eq!(folder.read_asset(KEY, "abc.png").unwrap(), b"first");
        let leftovers: Vec<_> = fs::read_dir(folder.path().join(KEY))
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        assert!(
            leftovers.iter().all(|n| !n.contains("tmp")),
            "{leftovers:?}"
        );
    }

    #[test]
    fn a_failed_write_is_an_error_and_leaves_no_partial_file() {
        let (_dir, folder) = folder();
        let _lock = folder.begin(KEY).unwrap();
        folder.write_copy(KEY, b"good", b"{}").unwrap();
        // A non-empty directory where the record goes makes its rename fail.
        let record = folder.path().join(KEY).join(RECORD_FILE);
        fs::remove_file(&record).unwrap();
        fs::create_dir_all(record.join("blocker")).unwrap();
        assert!(folder.write_copy(KEY, b"newer", b"{}").is_err());
        assert_eq!(
            folder.read_copy(KEY).unwrap(),
            b"newer",
            "the copy is one complete write"
        );
        let names: Vec<_> = fs::read_dir(folder.path().join(KEY))
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        assert!(names.iter().all(|n| !n.contains("tmp")), "{names:?}");
    }

    #[test]
    fn a_running_session_is_listed_as_locked_and_a_second_begin_is_refused() {
        let (_dir, folder) = folder();
        let lock = folder.begin(KEY).unwrap();
        folder.write_copy(KEY, b"c", b"{}").unwrap();
        assert!(folder.list()[0].locked);
        assert!(matches!(folder.begin(KEY), Err(RecoveryError::Held)));
        drop(lock);
        assert!(!folder.list()[0].locked);
        assert!(
            folder.begin(KEY).is_ok(),
            "a released lock can be taken again (Restore)"
        );
    }

    #[test]
    fn save_deletes_the_copy_and_keeps_the_lock_file() {
        let (_dir, folder) = folder();
        let _lock = folder.begin(KEY).unwrap();
        folder.write_asset(KEY, "abc.png", b"p").unwrap();
        folder.write_copy(KEY, b"c", b"{}").unwrap();
        folder.delete_copy(KEY).unwrap();
        let names: Vec<_> = fs::read_dir(folder.path().join(KEY))
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, vec!["lock".to_owned()]);
        folder.delete_copy(KEY).unwrap();
    }

    #[test]
    fn an_unlocked_folder_without_a_record_is_a_leftover_and_goes_silently() {
        let (_dir, folder) = folder();
        drop(folder.begin(KEY).unwrap());
        let _running = folder.begin(OTHER).unwrap();
        let listed = folder.list();
        assert_eq!(
            listed.len(),
            1,
            "the running session's folder stays, listed as locked"
        );
        assert_eq!(listed[0].key, OTHER);
        assert!(!folder.path().join(KEY).exists());
    }

    #[test]
    fn discard_removes_the_folder_and_names_are_checked() {
        let (_dir, folder) = folder();
        drop(folder.begin(KEY).unwrap());
        folder.write_copy(KEY, b"c", b"{}").unwrap();
        folder.delete_all(KEY).unwrap();
        assert_eq!(folder.list(), vec![]);
        folder.delete_all(KEY).unwrap();
        assert!(matches!(folder.begin("../x"), Err(RecoveryError::BadName)));
        assert!(matches!(
            folder.write_asset(KEY, "a/b/c", b""),
            Err(RecoveryError::BadName)
        ));
        assert!(matches!(
            folder.write_asset(KEY, "derived/../x", b""),
            Err(RecoveryError::BadName)
        ));
    }
}
