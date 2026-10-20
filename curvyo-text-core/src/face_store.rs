//! The faces of a session, keyed by the content hash of their file and their index in it.

use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

use sha2::{Digest, Sha256};

use crate::error::TextError;
use crate::face::Face;

/// The SHA-256 of a font **file**'s bytes (ADR 0020 §1, `face_hash`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FaceHash(pub [u8; 32]);

impl FaceHash {
    /// The hash of `bytes`.
    #[must_use]
    pub fn of(bytes: &[u8]) -> Self {
        Self(Sha256::digest(bytes).into())
    }

    /// The 64 lower-case hex digits the document stores.
    #[must_use]
    pub fn to_hex(&self) -> String {
        use fmt::Write as _;
        self.0
            .iter()
            .fold(String::with_capacity(64), |mut out, byte| {
                // invariant: writing into a String never fails.
                let _ = write!(out, "{byte:02x}");
                out
            })
    }

    /// Reads 64 hex digits (either case); anything else is `None`.
    #[must_use]
    pub fn from_hex(text: &str) -> Option<Self> {
        let bytes = text.as_bytes();
        if bytes.len() != 64 {
            return None;
        }
        let mut out = [0u8; 32];
        for (slot, pair) in out.iter_mut().zip(bytes.as_chunks::<2>().0) {
            let pair = std::str::from_utf8(pair).ok()?;
            *slot = u8::from_str_radix(pair, 16).ok()?;
        }
        Some(Self(out))
    }
}

impl fmt::Display for FaceHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_hex())
    }
}

/// One face: its file's hash and its index in that file (0 unless the file is a collection).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FaceKey {
    /// The file's content hash.
    pub hash: FaceHash,
    /// The index in a `.ttc`, else 0.
    pub index: u32,
}

impl fmt::Display for FaceKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}#{}", self.hash, self.index)
    }
}

/// The faces handed in during a session, each parsed once (ADR 0001 §5: bytes cross once per
/// face per session, not per frame).
#[derive(Debug, Default, Clone)]
pub struct FaceStore {
    faces: BTreeMap<FaceKey, Arc<Face>>,
}

impl FaceStore {
    /// An empty store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Parses face `index` of a font file and keeps it; a face already held is not parsed again.
    ///
    /// # Errors
    /// [`TextError::FaceTooLarge`] or [`TextError::Unreadable`] for bytes that hold no usable face.
    pub fn add(&mut self, bytes: Vec<u8>, index: u32) -> Result<FaceKey, TextError> {
        if bytes.len() > crate::limits::MAX_FACE_BYTES {
            return Err(TextError::FaceTooLarge);
        }
        let key = FaceKey {
            hash: FaceHash::of(&bytes),
            index,
        };
        if let std::collections::btree_map::Entry::Vacant(slot) = self.faces.entry(key) {
            slot.insert(Arc::new(Face::new(Arc::new(bytes), index)?));
        }
        Ok(key)
    }

    /// The face under `key`, if it was added.
    #[must_use]
    pub fn get(&self, key: &FaceKey) -> Option<&Arc<Face>> {
        self.faces.get(key)
    }

    /// The keys of every face held, in key order.
    pub fn keys(&self) -> impl Iterator<Item = &FaceKey> {
        self.faces.keys()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_round_trip_and_refusal() {
        let hash = FaceHash::of(b"abc");
        assert_eq!(
            hash.to_hex(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            FaceHash::from_hex(&hash.to_hex().to_uppercase()),
            Some(hash)
        );
        assert_eq!(FaceHash::from_hex("ab"), None);
        assert_eq!(FaceHash::from_hex(&"g".repeat(64)), None);
    }
}
