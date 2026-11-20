//! The owner calls on the paired clients and the logs (A10, A11, A18): codes for a name and
//! role, the client list, revoke with the close of its connections, and the last log lines.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use curvyo_connector_core::{ClientEntry, ErrorCode, LogName, Role, check_client_name};

use super::{Hub, Refused, SessionEntry};
use crate::admin_log::AdminEvent;
use crate::clients::FileRole;
use crate::log_tail::{clamp, last_lines};
use crate::pairing_code::{Grant, VALID_FOR};

fn error(code: ErrorCode, text: String) -> Refused {
    Refused { code, text }
}

impl Hub {
    /// Registers an authenticated connection of `name`; a revoke of that name sets the flag.
    pub fn register_session(&self, name: &str) -> Arc<AtomicBool> {
        let close = Arc::new(AtomicBool::new(false));
        let mut inner = self.lock();
        inner.sessions.retain(|s| s.close.strong_count() > 0);
        inner.sessions.push(SessionEntry {
            name: name.to_owned(),
            close: Arc::downgrade(&close),
        });
        close
    }

    fn owners(&self) -> Vec<String> {
        self.clients
            .list()
            .unwrap_or_default()
            .into_iter()
            .filter(|c| c.role == FileRole::Owner)
            .map(|c| c.name)
            .collect()
    }

    /// `IssueCode { client_name, role }` from the owner `by` (A10): the code as printed and the
    /// seconds it is valid. Recorded in `admin.log` without the code.
    ///
    /// # Errors
    /// `Malformed` for a name that breaks the rules; `WrongState` when the code would turn the
    /// only owner's name into a member (Frage 248 A).
    pub fn issue_code_for(
        &self,
        client_name: &str,
        role: Role,
        by: &str,
        now: Instant,
    ) -> Result<(String, u64), Refused> {
        check_client_name(client_name)
            .map_err(|r| error(ErrorCode::Malformed, format!("client_name: {r}")))?;
        let mut inner = self.lock();
        let owners = self.owners();
        if role == Role::Member && owners == [client_name] {
            return Err(error(
                ErrorCode::WrongState,
                format!("{client_name} is the last owner."),
            ));
        }
        let grant = Grant {
            name: client_name.to_owned(),
            role,
        };
        let code = inner.codes.issue_for(grant, now).ok_or_else(|| {
            error(
                ErrorCode::WrongState,
                "No random bytes are available.".to_owned(),
            )
        })?;
        let event = AdminEvent::CodeIssued {
            name: client_name.to_owned(),
            role,
        };
        self.append_admin(&inner, by, &event);
        Ok((code, VALID_FOR.as_secs()))
    }

    /// `ListClients`: every paired client, oldest first.
    #[must_use]
    pub fn list_clients(&self) -> Vec<ClientEntry> {
        self.clients
            .list()
            .unwrap_or_default()
            .into_iter()
            .map(|c| ClientEntry {
                name: c.name,
                role: c.role.into(),
                paired_at: c.paired_at,
            })
            .collect()
    }

    /// `RevokeClient { name }` from the owner `by` (A11): removes it and closes its connections.
    ///
    /// # Errors
    /// `UnknownName` for a name nobody has; `WrongState` for the last owner (Frage 248 A).
    pub fn revoke(&self, name: &str, by: &str) -> Result<(), Refused> {
        // Under the hub's lock, so two revokes cannot both pass the last-owner check.
        let mut inner = self.lock();
        let clients = self.clients.list().unwrap_or_default();
        let Some(record) = clients.iter().find(|c| c.name == name) else {
            return Err(error(
                ErrorCode::UnknownName,
                format!("No client is called {name}."),
            ));
        };
        if record.role == FileRole::Owner && self.owners().len() == 1 {
            return Err(error(
                ErrorCode::WrongState,
                format!("{name} is the last owner."),
            ));
        }
        let role = record.role.into();
        if let Err(e) = self.clients.remove(name) {
            return Err(error(ErrorCode::WrongState, e.to_string()));
        }
        for session in inner.sessions.iter().filter(|s| s.name == name) {
            if let Some(close) = session.close.upgrade() {
                close.store(true, Ordering::Relaxed);
            }
        }
        inner.sessions.retain(|s| s.name != name);
        let _ = inner.notices.send(format!("Revoked {name}"));
        let event = AdminEvent::Revoked {
            name: name.to_owned(),
            role,
        };
        self.append_admin(&inner, by, &event);
        Ok(())
    }

    /// `GetLog { log, last }`: the raw last lines, `last` clamped to 1 to 1,000.
    #[must_use]
    pub fn log_lines(&self, log: LogName, last: u32) -> Vec<String> {
        let path = match log {
            LogName::Jobs => self.job_log.path(),
            LogName::Admin => self.admin_log.path(),
        };
        last_lines(path, clamp(last)).unwrap_or_default()
    }
}
