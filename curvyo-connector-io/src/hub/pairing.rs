//! The hub's side of pairing: the pairing codes, the attempts and the list of paired apps.

use std::time::Instant;

use time::OffsetDateTime;

use super::Hub;
use crate::clients::ClientsError;
use crate::pairing_code::{Password, Refusal};

impl Hub {
    /// Whether `token` belongs to a paired app. The list is read from the file every time.
    #[must_use]
    pub fn accepts(&self, token: &str) -> bool {
        self.clients.accepts(token).unwrap_or(false)
    }

    /// The name of the paired app `token` belongs to.
    #[must_use]
    pub fn client_name(&self, token: &str) -> Option<String> {
        self.clients.name_of(token).ok().flatten()
    }

    /// Whether any app is paired.
    ///
    /// # Errors
    /// [`ClientsError`] for a damaged list.
    pub fn has_clients(&self) -> Result<bool, ClientsError> {
        Ok(!self.clients.list()?.is_empty())
    }

    /// Issues a new code, replacing a valid one.
    pub fn issue_code(&self, now: Instant) -> Option<String> {
        self.lock().codes.issue(now)
    }

    /// The valid code as printed.
    #[must_use]
    pub fn current_code(&self, now: Instant) -> Option<String> {
        self.lock().codes.current(now)
    }

    /// Begins a pairing attempt.
    ///
    /// # Errors
    /// [`Refusal`] when there is no valid code or an exchange is running.
    pub fn begin_pair(&self, now: Instant) -> Result<Password, Refusal> {
        self.lock().codes.begin(now)
    }

    /// The running attempt failed or was abandoned: the attempts left.
    pub fn pair_failed(&self) -> u8 {
        self.lock().codes.failed()
    }

    /// The running attempt verified: the code is spent and the app is stored.
    ///
    /// # Errors
    /// [`ClientsError`] when the list cannot be written; the code stays spent.
    pub fn pair_succeeded(&self, name: &str, token_sha256: &str) -> Result<(), ClientsError> {
        self.lock().codes.succeeded();
        self.clients
            .add(name, token_sha256, OffsetDateTime::now_utc())
    }
}
