//! The text of `online-sources.toml` (ADR 0085 decision 1): the online sources the maker added,
//! in the order they were added. Machine-local, no credential.

use serde::{Deserialize, Serialize};

use crate::error::{RequestError, SourcesError};
use crate::request::{SourceKind, base_address, is_ready};

/// The file's name in the app data folder.
pub const SOURCES_FILE: &str = "online-sources.toml";

const FORMAT: u32 = 1;

/// A moment as Unix milliseconds in UTC; the caller reads the clock, this crate has none.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Millis(pub i64);

/// One added source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceEntry {
    /// Git catalogue or Iconify.
    pub kind: SourceKind,
    /// The normalised base address.
    pub address: String,
    /// The name shown.
    pub name: String,
    /// When it was added, Unix milliseconds.
    pub added: Millis,
    /// Whether the key store holds an access token for this source (ADR 0090 decision 4); the
    /// token itself is never in this file. Only a user-added Git catalogue may have one.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub token: bool,
}

impl SourceEntry {
    /// The key of the source in the library tree and the host commands.
    #[must_use]
    pub fn key(&self) -> String {
        format!("online:{}", self.address)
    }
}

/// Why a source was not added.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddRefusal {
    /// The address is not a plain `https` address.
    Address(RequestError),
    /// The address is already in the list.
    AlreadyIn,
}

/// A token was asked for a source that cannot have one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TokenRefusal;

#[derive(Serialize, Deserialize)]
struct Wire {
    format: u32,
    #[serde(default, rename = "source")]
    sources: Vec<SourceEntry>,
}

/// The sources in the order they were added.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Sources {
    entries: Vec<SourceEntry>,
}

impl Sources {
    /// Reads the file's text.
    ///
    /// # Errors
    /// [`SourcesError::Damaged`] for text that is not the file, [`SourcesError::Newer`] for a
    /// newer `format`.
    pub fn parse(text: &str) -> Result<Self, SourcesError> {
        let wire: Wire = toml::from_str(text).map_err(|_| SourcesError::Damaged)?;
        if wire.format > FORMAT {
            return Err(SourcesError::Newer);
        }
        Ok(Self {
            entries: wire.sources,
        })
    }

    /// The file's text.
    #[must_use]
    pub fn to_text(&self) -> String {
        let wire = Wire {
            format: FORMAT,
            sources: self.entries.clone(),
        };
        // Plain strings and numbers always serialise; an empty text would read as no sources.
        toml::to_string(&wire).unwrap_or_default()
    }

    /// The sources in order.
    #[must_use]
    pub fn entries(&self) -> &[SourceEntry] {
        &self.entries
    }

    /// The source with `key`.
    #[must_use]
    pub fn find(&self, key: &str) -> Option<&SourceEntry> {
        self.entries.iter().find(|entry| entry.key() == key)
    }

    /// Appends a source; its address is normalised first.
    ///
    /// # Errors
    /// [`AddRefusal`] for a bad or already listed address.
    pub fn add(
        &mut self,
        kind: SourceKind,
        address: &str,
        name: &str,
        added: Millis,
    ) -> Result<String, AddRefusal> {
        let address = base_address(kind, address).map_err(AddRefusal::Address)?;
        if self.entries.iter().any(|entry| entry.address == address) {
            return Err(AddRefusal::AlreadyIn);
        }
        let entry = SourceEntry {
            kind,
            address,
            name: name.to_owned(),
            added,
            token: false,
        };
        let key = entry.key();
        self.entries.push(entry);
        Ok(key)
    }

    /// Sets whether the source with `key` has a token in the key store. Only a Git catalogue
    /// that is not the ready entry may have one (ADR 0090 decision 4); `false` always works.
    ///
    /// # Errors
    /// [`TokenRefusal`] when the source is missing, not a Git catalogue, or the ready entry.
    pub fn set_token(&mut self, key: &str, token: bool) -> Result<(), TokenRefusal> {
        let entry = self
            .entries
            .iter_mut()
            .find(|entry| entry.key() == key)
            .ok_or(TokenRefusal)?;
        if token && (entry.kind != SourceKind::Git || is_ready(&entry.address)) {
            return Err(TokenRefusal);
        }
        entry.token = token;
        Ok(())
    }

    /// Removes the source with `key`; whether it was there.
    pub fn remove(&mut self, key: &str) -> bool {
        let before = self.entries.len();
        self.entries.retain(|entry| entry.key() != key);
        self.entries.len() != before
    }

    /// The Iconify sources, the only ones Enter in the search is sent to (criterion 10).
    pub fn iconify(&self) -> impl Iterator<Item = &SourceEntry> {
        self.entries
            .iter()
            .filter(|entry| entry.kind == SourceKind::Iconify)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sources_keep_their_order_and_round_trip() {
        let mut sources = Sources::default();
        sources
            .add(
                SourceKind::Iconify,
                "https://api.iconify.design/",
                "api.iconify.design",
                Millis(5),
            )
            .unwrap();
        sources
            .add(SourceKind::Git, "https://example.org/cat", "Cat", Millis(6))
            .unwrap();
        let again = Sources::parse(&sources.to_text()).unwrap();
        assert_eq!(again, sources);
        assert_eq!(again.entries()[0].address, "https://api.iconify.design");
        assert_eq!(again.entries()[1].address, "https://example.org/cat/");
        assert_eq!(again.iconify().count(), 1);
    }

    #[test]
    fn a_duplicate_or_plain_http_address_is_refused() {
        let mut sources = Sources::default();
        sources
            .add(
                SourceKind::Git,
                "https://example.org/cat/",
                "Cat",
                Millis(1),
            )
            .unwrap();
        assert_eq!(
            sources.add(
                SourceKind::Git,
                "HTTPS://EXAMPLE.org/cat/",
                "Again",
                Millis(2)
            ),
            Err(AddRefusal::AlreadyIn)
        );
        assert_eq!(
            sources.add(SourceKind::Git, "http://example.org/x/", "Plain", Millis(3)),
            Err(AddRefusal::Address(RequestError::NotHttps))
        );
        assert_eq!(sources.entries().len(), 1);
    }

    #[test]
    fn the_token_flag_round_trips_defaults_false_and_is_never_a_secret() {
        let mut sources = Sources::default();
        let git = sources
            .add(SourceKind::Git, "https://example.org/cat", "Cat", Millis(1))
            .unwrap();
        let icons = sources
            .add(
                SourceKind::Iconify,
                "https://api.iconify.design",
                "I",
                Millis(2),
            )
            .unwrap();
        let ready = sources
            .add(SourceKind::Git, crate::READY_ADDRESS, "Ready", Millis(3))
            .unwrap();
        assert!(!sources.to_text().contains("token"), "false is not written");
        sources.set_token(&git, true).unwrap();
        assert_eq!(sources.set_token(&icons, true), Err(TokenRefusal));
        assert_eq!(sources.set_token(&ready, true), Err(TokenRefusal));
        assert_eq!(sources.set_token("online:nope", true), Err(TokenRefusal));
        let text = sources.to_text();
        assert!(text.contains("token = true"));
        let again = Sources::parse(&text).unwrap();
        assert!(again.find(&git).unwrap().token);
        assert!(!again.find(&icons).unwrap().token);
        // An older file without the field reads as a source without a token.
        let old = "format = 1\n[[source]]\nkind = \"git\"\naddress = \"https://e.org/\"\nname = \"E\"\nadded = 1\n";
        assert!(!Sources::parse(old).unwrap().entries()[0].token);
        sources.set_token(&git, false).unwrap();
        assert!(!sources.find(&git).unwrap().token);
    }

    #[test]
    fn remove_and_unreadable_text() {
        let mut sources = Sources::default();
        let key = sources
            .add(SourceKind::Git, "https://example.org/", "E", Millis(1))
            .unwrap();
        assert!(sources.remove(&key));
        assert!(!sources.remove(&key));
        assert_eq!(Sources::parse("nonsense ="), Err(SourcesError::Damaged));
        assert_eq!(Sources::parse("format = 2\n"), Err(SourcesError::Newer));
        assert_eq!(Sources::parse("format = 1\n").unwrap().entries().len(), 0);
    }
}
