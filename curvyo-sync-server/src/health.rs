//! The readiness answer of `GET /healthz` (criterion 37): ready once the startup scan is done
//! and the data directory is writable; it reveals no room data.

use std::fs;
use std::io;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

/// The probe file written and removed in the data directory by each check.
const PROBE: &str = ".healthz";

/// The server's readiness.
pub struct Health {
    scanned: AtomicBool,
    data: PathBuf,
}

impl Health {
    /// Not ready: the startup scan of `data` has not finished.
    #[must_use]
    pub fn new(data: PathBuf) -> Self {
        Self {
            scanned: AtomicBool::new(false),
            data,
        }
    }

    /// The startup scan has finished.
    pub fn scanned(&self) {
        self.scanned.store(true, Ordering::Release);
    }

    /// Whether the server is ready: scanned, and a file can be created and removed in the data
    /// directory right now.
    #[must_use]
    pub fn is_ready(&self) -> bool {
        self.scanned.load(Ordering::Acquire) && self.data_writable()
    }

    fn data_writable(&self) -> bool {
        let probe = self.data.join(PROBE);
        if fs::write(&probe, b"").is_err() {
            return false;
        }
        match fs::remove_file(&probe) {
            Ok(()) => true,
            Err(e) => e.kind() == io::ErrorKind::NotFound,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Criterion 37: 503 until the startup scan is done, then 200.
    #[test]
    fn not_ready_until_scanned() {
        let dir = tempfile::tempdir().unwrap();
        let health = Health::new(dir.path().to_owned());
        assert!(!health.is_ready());
        health.scanned();
        assert!(health.is_ready());
        assert!(
            !dir.path().join(PROBE).exists(),
            "the probe leaves nothing behind"
        );
    }

    #[test]
    fn not_ready_without_a_data_directory() {
        let dir = tempfile::tempdir().unwrap();
        let health = Health::new(dir.path().join("missing"));
        health.scanned();
        assert!(!health.is_ready());
    }
}
