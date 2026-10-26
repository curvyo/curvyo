//! The picture bytes of a project (ADR 0017 §2): content-addressed by SHA-256, held once in
//! memory and once in the container's `assets/`, shared by every image node that names the hash.
//! Only the document wipe prunes the store; a save never scans the history.

use std::collections::{BTreeMap, HashSet};
use std::sync::Arc;

use serde::Serialize;
use sha2::{Digest, Sha256};

/// The most bytes of pictures one project holds: 512 MiB (customer decision IMG-TOTAL = A,
/// `specs/0058-bitmap-import` criterion 16).
pub const MAX_PROJECT_ASSET_BYTES: u64 = 512 * 1024 * 1024;

/// The largest single asset read from a container: the file ceiling of the import, 100 MiB
/// (`specs/0058-bitmap-import` criterion 16; `curvyo_image_core::MAX_FILE_BYTES`). A larger
/// member is a missing asset, so a zip bomb is cut off.
pub const MAX_ASSET_BYTES: u64 = 100 * 1024 * 1024;

/// The lowercase hex SHA-256 of an asset's bytes, 64 characters.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct AssetHash(String);

impl AssetHash {
    /// The hash of `bytes`.
    #[must_use]
    pub fn of(bytes: &[u8]) -> Self {
        let digest = Sha256::digest(bytes);
        let mut hex = String::with_capacity(64);
        for byte in digest {
            hex.push(char::from_digit(u32::from(byte >> 4), 16).unwrap_or('0'));
            hex.push(char::from_digit(u32::from(byte & 0x0F), 16).unwrap_or('0'));
        }
        Self(hex)
    }

    /// `text` when it is 64 lowercase hex characters, else `None`.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        (text.len() == 64
            && text
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)))
        .then(|| Self(text.to_owned()))
    }

    /// The 64 hex characters.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The container extension of an asset by its first bytes: `png`, `jpg`, `webp`, or `bin` for
/// anything else (such an asset cannot be decoded and shows as a failed image).
#[must_use]
pub fn asset_extension(bytes: &[u8]) -> &'static str {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        "png"
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        "jpg"
    } else if bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP" {
        "webp"
    } else {
        "bin"
    }
}

/// Why pictures cannot be added to a project.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum AssetError {
    /// The new bytes would take the project past [`MAX_PROJECT_ASSET_BYTES`].
    #[error("the project would hold more than 512 MiB of pictures; these add {adds} bytes")]
    ProjectFull {
        /// The bytes the refused pictures would add.
        adds: u64,
    },
}

/// The asset bytes of one document, sorted by hash so a save is deterministic.
#[derive(Debug, Clone, Default)]
pub(crate) struct AssetStore {
    assets: BTreeMap<AssetHash, Arc<[u8]>>,
}

impl AssetStore {
    /// The bytes named `hash`.
    pub(crate) fn get(&self, hash: &AssetHash) -> Option<Arc<[u8]>> {
        self.assets.get(hash).cloned()
    }

    /// The bytes held, in total.
    pub(crate) fn total_bytes(&self) -> u64 {
        self.assets.values().map(|bytes| bytes.len() as u64).sum()
    }

    /// The bytes the distinct, new ones among `files` would add.
    pub(crate) fn added_bytes(&self, files: &[(AssetHash, &[u8])]) -> u64 {
        let mut seen = HashSet::new();
        files
            .iter()
            .filter(|(hash, _)| !self.assets.contains_key(hash) && seen.insert(hash.clone()))
            .map(|(_, bytes)| bytes.len() as u64)
            .sum()
    }

    /// Adds `bytes` under `hash` (the caller hashed them); nothing when they are held already.
    pub(crate) fn insert(&mut self, hash: AssetHash, bytes: Arc<[u8]>) {
        self.assets.entry(hash).or_insert(bytes);
    }

    /// Keeps only the assets in `alive`.
    pub(crate) fn retain(&mut self, alive: &HashSet<AssetHash>) {
        self.assets.retain(|hash, _| alive.contains(hash));
    }

    /// Every asset in hash order.
    pub(crate) fn iter(&self) -> impl Iterator<Item = (&AssetHash, &Arc<[u8]>)> {
        self.assets.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_hash_is_lowercase_hex_sha256() {
        assert_eq!(
            AssetHash::of(b"abc").as_str(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert!(AssetHash::parse(AssetHash::of(b"").as_str()).is_some());
        assert!(AssetHash::parse(&"A".repeat(64)).is_none());
        assert!(AssetHash::parse("abc").is_none());
    }

    #[test]
    fn the_same_bytes_count_once() {
        let mut store = AssetStore::default();
        let bytes: Arc<[u8]> = Arc::from(&b"\x89PNG\r\n\x1a\n...."[..]);
        let hash = AssetHash::of(&bytes);
        assert_eq!(
            store.added_bytes(&[(hash.clone(), &bytes), (hash.clone(), &bytes)]),
            12
        );
        store.insert(hash.clone(), bytes.clone());
        store.insert(hash.clone(), bytes.clone());
        assert_eq!(store.total_bytes(), 12);
        assert_eq!(store.added_bytes(&[(hash, &bytes)]), 0);
        assert_eq!(asset_extension(&bytes), "png");
    }
}
