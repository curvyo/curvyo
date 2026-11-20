//! The hub's side of pairing and login: the pairing codes, the attempts and the paired clients.

use std::time::Instant;

use curvyo_connector_core::Role;
use time::OffsetDateTime;

use super::Hub;
use crate::admin_log::AdminEvent;
use crate::clients::ClientsError;
use crate::pairing_code::{Password, Refusal};

/// The start of the name `curvyoctl login` pairs under.
const CTL_PREFIX: &str = "curvyoctl@";

impl Hub {
    /// Whether `token` belongs to a paired app. The list is read from the file every time.
    #[must_use]
    pub fn accepts(&self, token: &str) -> bool {
        self.clients.accepts(token).unwrap_or(false)
    }

    /// The name and role of the client `token` belongs to. The list is read every time, so a
    /// revoke or a role change from another process counts from the next login (addendum D4).
    #[must_use]
    pub fn login(&self, token: &str) -> Option<(String, Role)> {
        self.clients
            .find(token)
            .ok()
            .flatten()
            .map(|c| (c.name, c.role.into()))
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

    /// The running attempt verified: the code is spent and the client is stored under the name
    /// and role the code was issued for, else under `app_name` as a member (addendum D7).
    /// Returns the name and role it is stored under.
    ///
    /// # Errors
    /// [`ClientsError`] when the list cannot be written; the code stays spent.
    pub fn pair_succeeded(
        &self,
        app_name: &str,
        token_sha256: &str,
    ) -> Result<(String, Role), ClientsError> {
        let inner = &mut *self.lock();
        // `curvyoctl login` names itself `curvyoctl@<host>` (A35); an owner code gives it the role.
        let (name, role) = inner.codes.grant().map_or_else(
            || (app_name.to_owned(), Role::Member),
            |g| {
                let own = g.role == Role::Owner && app_name.starts_with(CTL_PREFIX);
                (if own { app_name.to_owned() } else { g.name }, g.role)
            },
        );
        inner.codes.succeeded();
        self.clients
            .add(&name, role, token_sha256, OffsetDateTime::now_utc())?;
        self.append_admin(inner, &name, &AdminEvent::Paired { role });
        Ok((name, role))
    }
}
