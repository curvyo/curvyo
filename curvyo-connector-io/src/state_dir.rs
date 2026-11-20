//! The state directory as a whole (A19 to A21; addendum D10): the lock one connector holds, the
//! wipe of a reset and the fingerprint of its key.

use std::fs::{File, OpenOptions, TryLockError};
use std::path::{Path, PathBuf};

use crate::config_file::{CONFIG_FILE, sync_dir};
use crate::identity::{self, IDENTITY_FILE, make_dir};
use crate::spki::SpkiHash;

/// The lock file's name.
pub const LOCK_FILE: &str = "lock";

/// Why the state directory could not be used.
#[derive(Debug, thiserror::Error)]
pub enum StateDirError {
    /// Another connector holds the lock.
    #[error("a running connector holds {}", .0.display())]
    Locked(PathBuf),
    /// A file could not be read, made or deleted.
    #[error("{}: {source}", .path.display())]
    Io {
        /// The file.
        path: PathBuf,
        /// The cause.
        source: std::io::Error,
    },
    /// The key file.
    #[error(transparent)]
    Identity(#[from] identity::IdentityError),
}

fn io(path: &Path) -> impl FnOnce(std::io::Error) -> StateDirError + '_ {
    move |source| StateDirError::Io {
        path: path.to_owned(),
        source,
    }
}

/// The held lock of one state directory; released when dropped.
#[derive(Debug)]
pub struct Lock {
    _file: File,
}

/// Takes `<dir>/lock` (made `0600`), creating the directory `0700` if needed.
///
/// # Errors
/// [`StateDirError::Locked`] when another process holds it.
pub fn lock(dir: &Path) -> Result<Lock, StateDirError> {
    make_dir(dir).map_err(io(dir))?;
    let path = dir.join(LOCK_FILE);
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(0o600);
    }
    let file = options.open(&path).map_err(io(&path))?;
    match file.try_lock() {
        Ok(()) => Ok(Lock { _file: file }),
        Err(TryLockError::WouldBlock) => Err(StateDirError::Locked(path)),
        Err(TryLockError::Error(source)) => Err(StateDirError::Io { path, source }),
    }
}

/// Deletes everything in `dir` but the directory itself: `connector.toml` first (a half-done
/// wipe is maintenance mode), then the rest, `lock` last.
///
/// # Errors
/// The first file that could not be deleted.
pub fn empty(dir: &Path) -> Result<(), StateDirError> {
    let remove = |path: &Path| match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(StateDirError::Io {
            path: path.to_owned(),
            source,
        }),
    };
    remove(&dir.join(CONFIG_FILE))?;
    let _ = sync_dir(dir);
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(source) => return Err(io(dir)(source)),
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if entry.file_name() == LOCK_FILE {
            continue;
        }
        if path.is_dir() {
            std::fs::remove_dir_all(&path).map_err(io(&path))?;
        } else {
            remove(&path)?;
        }
    }
    remove(&dir.join(LOCK_FILE))?;
    let _ = sync_dir(dir);
    Ok(())
}

/// `--reset`: takes the lock (refused while a connector runs), then empties the directory.
///
/// # Errors
/// [`StateDirError::Locked`] or a failed deletion.
pub fn reset(dir: &Path) -> Result<(), StateDirError> {
    if !dir.exists() {
        return Ok(());
    }
    let held = lock(dir)?;
    empty(dir)?;
    drop(held);
    Ok(())
}

/// `--fingerprint`: the fingerprint of an existing key; nothing is created.
///
/// # Errors
/// [`StateDirError::Io`] when there is no key file, or a damaged key.
pub fn fingerprint(dir: &Path) -> Result<SpkiHash, StateDirError> {
    let path = dir.join(IDENTITY_FILE);
    if !path.exists() {
        return Err(StateDirError::Io {
            path,
            source: std::io::ErrorKind::NotFound.into(),
        });
    }
    Ok(identity::load_or_create(dir)?.spki)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a21_one_lock_per_directory_and_reset_refused_while_held() {
        let dir = tempfile::tempdir().unwrap();
        let held = lock(dir.path()).unwrap();
        assert!(matches!(lock(dir.path()), Err(StateDirError::Locked(_))));
        std::fs::write(dir.path().join(CONFIG_FILE), "x").unwrap();
        let error = reset(dir.path()).unwrap_err().to_string();
        assert!(error.contains("a running connector holds"), "{error}");
        assert!(dir.path().join(CONFIG_FILE).exists());
        drop(held);
        reset(dir.path()).unwrap();
        assert!(dir.path().exists());
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
    }

    #[test]
    fn a19_empty_deletes_everything_but_the_directory() {
        let dir = tempfile::tempdir().unwrap();
        for name in [
            CONFIG_FILE,
            "clients.json",
            "jobs.log",
            "admin.log",
            LOCK_FILE,
        ] {
            std::fs::write(dir.path().join(name), "x").unwrap();
        }
        identity::load_or_create(dir.path()).unwrap();
        empty(dir.path()).unwrap();
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
    }

    #[test]
    fn a21_the_fingerprint_reads_the_key_and_creates_nothing() {
        let dir = tempfile::tempdir().unwrap();
        assert!(fingerprint(dir.path()).is_err());
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
        let key = identity::load_or_create(dir.path()).unwrap();
        assert_eq!(fingerprint(dir.path()).unwrap(), key.spki);
    }
}
