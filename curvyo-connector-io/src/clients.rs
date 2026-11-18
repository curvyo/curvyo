//! `clients.json`: the paired apps, each by name and the SHA-256 of its token, never the token.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::identity::{make_dir, write_private};
use crate::token::Token;

/// The file name in the state directory.
pub const CLIENTS_FILE: &str = "clients.json";
const FORMAT: u32 = 1;

/// One paired app.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientRecord {
    /// The name the app gave when it paired.
    pub name: String,
    /// SHA-256 of its token, 64 lower-case hex characters.
    pub token_sha256: String,
    /// When it paired, RFC 3339 UTC.
    pub paired_at: String,
}

#[derive(Serialize, Deserialize)]
struct File {
    format: u32,
    clients: Vec<ClientRecord>,
}

/// Why the list could not be read or written; every message names the file.
#[derive(Debug, thiserror::Error)]
pub enum ClientsError {
    /// Reading or writing failed.
    #[error("{}: {source}", .path.display())]
    Io {
        /// The file.
        path: PathBuf,
        /// The cause.
        source: std::io::Error,
    },
    /// The file is not a list this build wrote.
    #[error("{}: damaged ({reason})", .path.display())]
    Damaged {
        /// The file.
        path: PathBuf,
        /// What is wrong.
        reason: String,
    },
}

/// The list of paired apps in one state directory. It re-reads the file on every call, so a
/// `--revoke` from another process counts at once.
#[derive(Debug, Clone)]
pub struct Clients {
    path: PathBuf,
}

impl Clients {
    /// The list in `dir`; the file is made at the first pairing.
    #[must_use]
    pub fn new(dir: &Path) -> Self {
        Self {
            path: dir.join(CLIENTS_FILE),
        }
    }

    /// Every paired app, oldest first; none when there is no file yet.
    ///
    /// # Errors
    /// [`ClientsError`] for an unreadable or damaged file.
    pub fn list(&self) -> Result<Vec<ClientRecord>, ClientsError> {
        let bytes = match fs::read(&self.path) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(source) => {
                return Err(ClientsError::Io {
                    path: self.path.clone(),
                    source,
                });
            }
        };
        let damaged = |reason: String| ClientsError::Damaged {
            path: self.path.clone(),
            reason,
        };
        let file: File = serde_json::from_slice(&bytes).map_err(|e| damaged(e.to_string()))?;
        if file.format != FORMAT {
            return Err(damaged(format!("format {}", file.format)));
        }
        Ok(file.clients)
    }

    fn save(&self, clients: Vec<ClientRecord>) -> Result<(), ClientsError> {
        let io = |source| ClientsError::Io {
            path: self.path.clone(),
            source,
        };
        if let Some(dir) = self.path.parent() {
            make_dir(dir).map_err(io)?;
        }
        let text = serde_json::to_vec_pretty(&File {
            format: FORMAT,
            clients,
        })
        .map_err(|e| ClientsError::Damaged {
            path: self.path.clone(),
            reason: e.to_string(),
        })?;
        write_private(&self.path, &text).map_err(io)
    }

    /// Adds an app; one of the same name is replaced.
    ///
    /// # Errors
    /// [`ClientsError`].
    pub fn add(
        &self,
        name: &str,
        token_sha256: &str,
        paired_at: OffsetDateTime,
    ) -> Result<(), ClientsError> {
        let mut clients = self.list()?;
        clients.retain(|c| c.name != name);
        clients.push(ClientRecord {
            name: name.to_owned(),
            token_sha256: token_sha256.to_owned(),
            paired_at: paired_at.format(&Rfc3339).unwrap_or_default(),
        });
        self.save(clients)
    }

    /// Removes the app called `name`; false when there is none.
    ///
    /// # Errors
    /// [`ClientsError`].
    pub fn remove(&self, name: &str) -> Result<bool, ClientsError> {
        let mut clients = self.list()?;
        let before = clients.len();
        clients.retain(|c| c.name != name);
        if clients.len() == before {
            return Ok(false);
        }
        self.save(clients)?;
        Ok(true)
    }

    /// Whether `token` (64 hex characters) belongs to a paired app. The hash is compared with
    /// every entry in constant time.
    ///
    /// # Errors
    /// [`ClientsError`].
    pub fn accepts(&self, token: &str) -> Result<bool, ClientsError> {
        let Some(token) = Token::from_hex(token) else {
            return Ok(false);
        };
        let hash = token.sha256_hex();
        let found = self.list()?.iter().fold(false, |found, c| {
            same(c.token_sha256.as_bytes(), hash.as_bytes()) | found
        });
        Ok(found)
    }

    /// The name of the app `token` belongs to, for the log (`specs/0127` G33).
    ///
    /// # Errors
    /// [`ClientsError`] for a damaged list.
    pub fn name_of(&self, token: &str) -> Result<Option<String>, ClientsError> {
        let Some(token) = Token::from_hex(token) else {
            return Ok(None);
        };
        let hash = token.sha256_hex();
        Ok(self
            .list()?
            .into_iter()
            .find(|c| same(c.token_sha256.as_bytes(), hash.as_bytes()))
            .map(|c| c.name))
    }
}

/// Equality of two byte strings of public length without an early exit.
fn same(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len()
        && std::hint::black_box(a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y))) == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn time(secs: i64) -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(secs).unwrap()
    }

    #[test]
    fn a_pairing_is_stored_as_a_hash_and_found_again() {
        let dir = tempfile::tempdir().unwrap();
        let clients = Clients::new(dir.path());
        assert_eq!(clients.list().unwrap(), Vec::new());
        let token = Token::generate().unwrap();
        clients
            .add("Studio laptop", &token.sha256_hex(), time(1_700_000_000))
            .unwrap();
        assert!(clients.accepts(&token.to_hex()).unwrap());
        assert!(
            !clients
                .accepts(&Token::generate().unwrap().to_hex())
                .unwrap()
        );
        assert!(!clients.accepts("not hex").unwrap());
        let text = fs::read_to_string(dir.path().join(CLIENTS_FILE)).unwrap();
        assert!(!text.contains(token.to_hex().as_str()));
        assert!(text.contains(&token.sha256_hex()));
    }

    #[test]
    fn the_file_matches_its_golden_file() {
        let dir = tempfile::tempdir().unwrap();
        let clients = Clients::new(dir.path());
        clients
            .add("Studio laptop", &"ab".repeat(32), time(1_700_000_000))
            .unwrap();
        clients
            .add("Workshop tablet", &"cd".repeat(32), time(1_700_000_060))
            .unwrap();
        let text = fs::read_to_string(dir.path().join(CLIENTS_FILE)).unwrap();
        let golden = include_str!("../tests/fixtures/clients.json");
        assert_eq!(text.trim_end(), golden.trim_end());
    }

    #[test]
    fn pairing_again_under_a_name_replaces_and_revoke_removes() {
        let dir = tempfile::tempdir().unwrap();
        let clients = Clients::new(dir.path());
        let one = Token::generate().unwrap();
        let two = Token::generate().unwrap();
        clients.add("laptop", &one.sha256_hex(), time(1)).unwrap();
        clients.add("laptop", &two.sha256_hex(), time(2)).unwrap();
        assert_eq!(clients.list().unwrap().len(), 1);
        assert!(!clients.accepts(&one.to_hex()).unwrap());
        assert!(clients.accepts(&two.to_hex()).unwrap());
        assert!(clients.remove("laptop").unwrap());
        assert!(!clients.remove("laptop").unwrap());
        assert!(!clients.accepts(&two.to_hex()).unwrap());
    }

    #[test]
    fn a_damaged_file_is_an_error_naming_it() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(CLIENTS_FILE), "{").unwrap();
        let error = Clients::new(dir.path()).list().unwrap_err().to_string();
        assert!(error.contains(CLIENTS_FILE), "{error}");
    }
}
