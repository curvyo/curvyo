//! Symmetric epoch keys, their public `key_id` and the document id they are bound to.

use sha2::{Digest, Sha256};
use zeroize::Zeroize;

/// The domain tag of `key_id` (ADR 0031 decision 6).
const KEY_ID_TAG: &[u8] = b"curvyo-epoch-id-v1";

/// The 16-byte id of a shared document; in slice 1 also its room id (ADR 0031 decision 5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DocumentId(pub [u8; 16]);

/// The public 8-byte id of an epoch key: the first 8 bytes of
/// SHA-256(`"curvyo-epoch-id-v1"` ‖ key). Two concurrent rotations to the same epoch number
/// have different ids.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct KeyId(pub [u8; 8]);

/// One epoch's random 256-bit document key (ADR 0010 §6). Wiped on drop; never printed.
#[derive(Clone, PartialEq, Eq)]
pub struct EpochKey([u8; 32]);

impl Drop for EpochKey {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

impl EpochKey {
    /// Wraps 32 bytes of caller-supplied entropy (or an unwrapped or stored key).
    #[must_use]
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// The raw key, for the credential store.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// This key's public id.
    #[must_use]
    pub fn key_id(&self) -> KeyId {
        let digest = Sha256::new()
            .chain_update(KEY_ID_TAG)
            .chain_update(self.0)
            .finalize();
        let mut id = [0; 8];
        id.copy_from_slice(&digest[..8]);
        KeyId(id)
    }
}

impl core::fmt::Debug for EpochKey {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "EpochKey({:?})", self.key_id())
    }
}
