//! The invite key a link's secret derives, and the claim proof made with it (ADR 0031
//! decision 8).

use ed25519_dalek::{Signer, SigningKey};
use hkdf::Hkdf;
use sha2::Sha256;
use zeroize::Zeroizing;

use crate::identity::{Identity, MemberId, signing_key_from_seed, verify};
use crate::{CryptoError, DocumentId};

const INVITE_INFO: &[u8] = b"curvyo-invite-key-v1";

/// The public half of an invite key: what `AddPendingInvite` and `ClaimInvite` name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InviteId(pub [u8; 32]);

/// The ed25519 key derived (HKDF-SHA-256) from a link's 32-byte invite seed. Only the holder
/// of the link can make it; the keyring sees only its public half.
pub struct InviteKey {
    signing: SigningKey,
}

impl InviteKey {
    /// Derives the invite key from the link's seed.
    ///
    /// # Errors
    /// Never in practice (HKDF output of 32 bytes); kept as an error, not a panic.
    pub fn from_seed(seed: &[u8; 32]) -> Result<Self, CryptoError> {
        let mut key_seed = Zeroizing::new([0; 32]);
        Hkdf::<Sha256>::new(None, seed)
            .expand(INVITE_INFO, key_seed.as_mut_slice())
            .map_err(|_| CryptoError::Malformed)?;
        Ok(Self {
            signing: signing_key_from_seed(&key_seed),
        })
    }

    /// The invite's public id.
    #[must_use]
    pub fn invite_id(&self) -> InviteId {
        InviteId(self.signing.verifying_key().to_bytes())
    }

    /// The `ClaimInvite` proof: the invite key's signature over
    /// `document_id ‖ invite_id ‖ identity`.
    #[must_use]
    pub fn claim_proof(&self, document_id: DocumentId, identity: &Identity) -> [u8; 64] {
        self.signing
            .sign(&claim_message(document_id, self.invite_id(), identity))
            .to_bytes()
    }
}

/// Checks a `ClaimInvite` proof.
///
/// # Errors
/// `BadSignature` when it does not verify.
pub fn verify_claim_proof(
    invite_id: InviteId,
    document_id: DocumentId,
    identity: &Identity,
    proof: &[u8; 64],
) -> Result<(), CryptoError> {
    verify(
        &MemberId(invite_id.0),
        &claim_message(document_id, invite_id, identity),
        proof,
    )
}

fn claim_message(document_id: DocumentId, invite_id: InviteId, identity: &Identity) -> Vec<u8> {
    let mut message = Vec::with_capacity(16 + 32 + 64);
    message.extend_from_slice(&document_id.0);
    message.extend_from_slice(&invite_id.0);
    message.extend_from_slice(&identity.to_bytes());
    message
}
