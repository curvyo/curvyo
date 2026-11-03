//! The server's log sink: one line per lifecycle event, never an IP address or a message body
//! (ADR 0031 decision 11, criterion 36).

use std::io::Write;
use std::sync::{Arc, Mutex, PoisonError};

/// Where log lines go: stdout in the binary, a buffer in tests. Cheap to clone.
#[derive(Clone)]
pub struct Log(Arc<Mutex<Box<dyn Write + Send>>>);

impl Log {
    /// A log writing to `out`.
    pub fn new(out: impl Write + Send + 'static) -> Self {
        Self(Arc::new(Mutex::new(Box::new(out))))
    }

    /// A log writing to the process's stdout (the container's log stream).
    #[must_use]
    pub fn stdout() -> Self {
        Self::new(std::io::stdout())
    }

    /// Writes one line. Callers pass fixed text and counts or addresses of the server itself,
    /// never a client's address or a received byte.
    pub fn line(&self, message: &str) {
        let mut out = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        // A log that cannot be written must not stop the relay.
        let _ = writeln!(out, "curvyo-sync-server: {message}");
        let _ = out.flush();
    }
}
