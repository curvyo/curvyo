//! The data the owner calls carry (ADR 0071 decision 4): roles, modes, client entries, log names
//! and the redacted secret.

use serde::{Deserialize, Serialize};

/// What a paired client may do (ADR 0071 decision 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    /// Configures, pairs, confirms starts and resets.
    Owner,
    /// Sends jobs; its starts wait for an owner.
    Member,
}

/// Which log `GetLog` reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogName {
    /// `jobs.log`.
    Jobs,
    /// `admin.log`.
    Admin,
}

/// The connector's mode as `Welcome` shows it (A17).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "name")]
pub enum ConnectorMode {
    /// Not configured: only `Hello`, `ListPorts` and `ApplyConfig` are answered.
    Maintenance {
        /// Seconds until `ApplyConfig` is refused; 0 once the window is closed.
        closes_in_s: u64,
    },
    /// Configured: every call needs a login.
    Configured {
        /// The owner label of the configuration, if it has one.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        owner: Option<String>,
    },
    /// Run with the command-line flags of slices 2 and 4: no owners.
    Foreground,
}

/// One paired client as `Clients` lists it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientEntry {
    /// Its name.
    pub name: String,
    /// Its role.
    pub role: Role,
    /// When it paired, RFC 3339 UTC.
    pub paired_at: String,
}

/// A pairing code or a token on the wire: its `Debug` never shows it, so no log line can print it.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Secret(String);

impl Secret {
    /// Wraps `text`.
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        Self(text.into())
    }

    /// The text itself.
    #[must_use]
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Secret(..)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_secret_hides_itself_in_debug_and_not_on_the_wire() {
        let secret = Secret::new("K7QM-4XTR");
        assert_eq!(format!("{secret:?}"), "Secret(..)");
        assert_eq!(serde_json::to_string(&secret).unwrap(), "\"K7QM-4XTR\"");
        assert_eq!(secret.expose(), "K7QM-4XTR");
    }
}
