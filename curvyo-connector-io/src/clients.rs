//! `clients.json`: the paired clients, each by name, role and the SHA-256 of its token, never the
//! token. Format 1 (slice 2) reads with every client as a member; it is written as format 2.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use curvyo_connector_core::Role;

use crate::identity::{make_dir, write_private};
use crate::token::Token;

/// The file name in the state directory.
pub const CLIENTS_FILE: &str = "clients.json";
const FORMAT: u32 = 2;

/// A client's role as the file spells it (lower case, ADR 0071).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileRole {
    /// An owner.
    Owner,
    /// A member.
    Member,
}

impl From<FileRole> for Role {
    fn from(role: FileRole) -> Self {
        match role {
            FileRole::Owner => Role::Owner,
            FileRole::Member => Role::Member,
        }
    }
}

impl From<Role> for FileRole {
    fn from(role: Role) -> Self {
        match role {
            Role::Owner => FileRole::Owner,
            Role::Member => FileRole::Member,
        }
    }
}

/// One paired client.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientRecord {
    /// The name it is recorded under.
    pub name: String,
    /// Its role.
    pub role: FileRole,
    /// SHA-256 of its token, 64 lower-case hex characters.
    pub token_sha256: String,
    /// When it paired, RFC 3339 UTC.
    pub paired_at: String,
}

#[derive(Serialize)]
struct File {
    format: u32,
    clients: Vec<ClientRecord>,
}

/// A record as read: format 1 has no role.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReadRecord {
    name: String,
    #[serde(default)]
    role: Option<FileRole>,
    token_sha256: String,
    paired_at: String,
}

#[derive(Deserialize)]
struct ReadFile {
    format: u32,
    clients: Vec<ReadRecord>,
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
        let file: ReadFile = serde_json::from_slice(&bytes).map_err(|e| damaged(e.to_string()))?;
        if file.format > FORMAT {
            return Err(damaged(format!(
                "format {}, written by a newer connector",
                file.format
            )));
        }
        if file.format == 0 {
            return Err(damaged("format 0".to_owned()));
        }
        file.clients
            .into_iter()
            .map(|r| {
                let role = match (file.format, r.role) {
                    (1, None) => FileRole::Member,
                    (FORMAT, Some(role)) => role,
                    _ => return Err(damaged(format!("the role of {}", r.name))),
                };
                Ok(ClientRecord {
                    name: r.name,
                    role,
                    token_sha256: r.token_sha256,
                    paired_at: r.paired_at,
                })
            })
            .collect()
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

    /// Adds a client; one of the same name is replaced (addendum D7).
    ///
    /// # Errors
    /// [`ClientsError`].
    pub fn add(
        &self,
        name: &str,
        role: Role,
        token_sha256: &str,
        paired_at: OffsetDateTime,
    ) -> Result<(), ClientsError> {
        let mut clients = self.list()?;
        clients.retain(|c| c.name != name);
        clients.push(ClientRecord {
            name: name.to_owned(),
            role: role.into(),
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

    /// Removes every owner: a setup starts from a list without owners (addendum D6).
    ///
    /// # Errors
    /// [`ClientsError`].
    pub fn drop_owners(&self) -> Result<(), ClientsError> {
        let mut clients = self.list()?;
        let before = clients.len();
        clients.retain(|c| c.role != FileRole::Owner);
        if clients.len() == before {
            return Ok(());
        }
        self.save(clients)
    }

    /// The client `token` belongs to (name and role).
    ///
    /// # Errors
    /// [`ClientsError`] for a damaged list.
    pub fn find(&self, token: &str) -> Result<Option<ClientRecord>, ClientsError> {
        let Some(token) = Token::from_hex(token) else {
            return Ok(None);
        };
        let hash = token.sha256_hex();
        let mut found = None;
        for client in self.list()? {
            if same(client.token_sha256.as_bytes(), hash.as_bytes()) {
                found = Some(client);
            }
        }
        Ok(found)
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
            .add(
                "Studio laptop",
                Role::Member,
                &token.sha256_hex(),
                time(1_700_000_000),
            )
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
            .add(
                "Studio laptop",
                Role::Owner,
                &"ab".repeat(32),
                time(1_700_000_000),
            )
            .unwrap();
        clients
            .add(
                "Workshop tablet",
                Role::Member,
                &"cd".repeat(32),
                time(1_700_000_060),
            )
            .unwrap();
        let text = fs::read_to_string(dir.path().join(CLIENTS_FILE)).unwrap();
        let golden = include_str!("../tests/fixtures/clients_v2.json");
        assert_eq!(text.trim_end(), golden.trim_end());
    }

    #[test]
    fn pairing_again_under_a_name_replaces_and_revoke_removes() {
        let dir = tempfile::tempdir().unwrap();
        let clients = Clients::new(dir.path());
        let one = Token::generate().unwrap();
        let two = Token::generate().unwrap();
        clients
            .add("laptop", Role::Member, &one.sha256_hex(), time(1))
            .unwrap();
        clients
            .add("laptop", Role::Owner, &two.sha256_hex(), time(2))
            .unwrap();
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

    #[test]
    fn a8_format_one_reads_as_members_and_is_written_as_format_two_on_the_next_change() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(CLIENTS_FILE);
        fs::write(&path, include_str!("../tests/fixtures/clients.json")).unwrap();
        let clients = Clients::new(dir.path());
        let list = clients.list().unwrap();
        assert_eq!(list.len(), 2);
        assert!(list.iter().all(|c| c.role == FileRole::Member));
        // Reading changes nothing on disk.
        assert!(fs::read_to_string(&path).unwrap().contains("\"format\": 1"));
        clients.drop_owners().unwrap();
        assert!(fs::read_to_string(&path).unwrap().contains("\"format\": 1"));
        clients
            .add("Laptop", Role::Owner, &"ef".repeat(32), time(1_700_000_120))
            .unwrap();
        let text = fs::read_to_string(&path).unwrap();
        assert!(text.contains("\"format\": 2"), "{text}");
        assert_eq!(text.matches("\"role\": \"member\"").count(), 2);
        assert_eq!(text.matches("\"role\": \"owner\"").count(), 1);
    }

    #[test]
    fn a_newer_format_or_a_missing_role_in_format_two_is_damaged() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(CLIENTS_FILE);
        fs::write(&path, r#"{"format":3,"clients":[]}"#).unwrap();
        let error = Clients::new(dir.path()).list().unwrap_err().to_string();
        assert!(error.contains("newer connector"), "{error}");
        fs::write(
            &path,
            r#"{"format":2,"clients":[{"name":"a","token_sha256":"ab","paired_at":"x"}]}"#,
        )
        .unwrap();
        assert!(Clients::new(dir.path()).list().is_err());
    }

    #[test]
    fn a_setup_drops_left_over_owners_and_find_names_the_role() {
        let dir = tempfile::tempdir().unwrap();
        let clients = Clients::new(dir.path());
        let owner = Token::generate().unwrap();
        let member = Token::generate().unwrap();
        clients
            .add("Laptop", Role::Owner, &owner.sha256_hex(), time(1))
            .unwrap();
        clients
            .add("Phone", Role::Member, &member.sha256_hex(), time(2))
            .unwrap();
        let found = clients.find(&owner.to_hex()).unwrap().unwrap();
        assert_eq!(
            (found.name.as_str(), found.role),
            ("Laptop", FileRole::Owner)
        );
        clients.drop_owners().unwrap();
        assert_eq!(clients.find(&owner.to_hex()).unwrap(), None);
        assert_eq!(
            clients.find(&member.to_hex()).unwrap().unwrap().role,
            FileRole::Member
        );
    }
}
