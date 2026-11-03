//! Sealing an epoch key to one recipient's X25519 key (ADR 0010 §6, amended by ADR 0031).
//!
//! A wrap is `key_id [8] ‖ ephemeral public key [32] ‖ sealed key [32] ‖ tag [16]`. The
//! sealing key and nonce come from HKDF-SHA-256 over the ephemeral-static X25519 secret; the
//! associated data is `document_id ‖ epoch ‖ key_id ‖ recipient identity`, so a wrap cannot be
//! moved to another document, epoch, epoch key or recipient.

use hkdf::Hkdf;
use sha2::Sha256;
use zeroize::Zeroizing;

use crate::codec::{Reader, Writer};
use crate::identity::{Identity, IdentityKeys};
use crate::{CryptoError, DocumentId, EpochKey, KeyId, aead, agreement};

/// The length of a wrap in bytes.
pub const WRAP_LEN: usize = 8 + 32 + 32 + aead::TAG_LEN;
const WRAP_INFO: &[u8] = b"curvyo-wrap-v1";

/// One epoch key sealed to one recipient. Its `key_id` is public, so the keyring can tell
/// which epoch key a wrap holds without opening it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Wrap(pub [u8; WRAP_LEN]);

impl Wrap {
    /// The id of the wrapped epoch key.
    #[must_use]
    pub fn key_id(&self) -> KeyId {
        let mut id = [0; 8];
        id.copy_from_slice(&self.0[..8]);
        KeyId(id)
    }
}

/// Wraps `key` (epoch `epoch` of `document_id`) to `recipient`, with 32 bytes of fresh
/// caller-supplied entropy as the ephemeral secret.
///
/// # Errors
/// `BadPublicKey` for a recipient agreement key of low order.
pub fn wrap_epoch_key(
    key: &EpochKey,
    epoch: u32,
    document_id: DocumentId,
    recipient: &Identity,
    ephemeral_secret: [u8; 32],
) -> Result<Wrap, CryptoError> {
    let ephemeral_secret = Zeroizing::new(ephemeral_secret);
    let ephemeral_public = agreement::public_key(&ephemeral_secret);
    let shared = Zeroizing::new(agreement::shared_secret(
        &ephemeral_secret,
        &recipient.agreement,
    )?);
    let key_id = key.key_id();
    let (wrap_key, nonce) = derive(&shared, &ephemeral_public, recipient)?;
    let ad = associated_data(document_id, epoch, key_id, recipient);
    let sealed = aead::seal(&wrap_key, &nonce, &ad, key.as_bytes())?;
    let mut writer = Writer::default();
    writer.raw(&key_id.0);
    writer.raw(&ephemeral_public);
    writer.raw(&sealed);
    let mut bytes = [0; WRAP_LEN];
    bytes.copy_from_slice(&writer.bytes);
    Ok(Wrap(bytes))
}

/// Opens a wrap addressed to `me`.
///
/// # Errors
/// `Unauthentic` when the wrap is not for `me`, this document or this epoch, or was changed;
/// `WrongKey` when the opened key does not have the wrap's `key_id`.
pub fn unwrap_epoch_key(
    wrap: &Wrap,
    epoch: u32,
    document_id: DocumentId,
    me: &IdentityKeys,
) -> Result<EpochKey, CryptoError> {
    let mut reader = Reader::new(&wrap.0);
    let key_id = KeyId(reader.array()?);
    let ephemeral_public: [u8; 32] = reader.array()?;
    let sealed = reader.take(32 + aead::TAG_LEN)?;
    let recipient = me.identity();
    let shared = Zeroizing::new(agreement::shared_secret(
        me.agreement_secret(),
        &ephemeral_public,
    )?);
    let (wrap_key, nonce) = derive(&shared, &ephemeral_public, &recipient)?;
    let ad = associated_data(document_id, epoch, key_id, &recipient);
    let opened = Zeroizing::new(aead::open(&wrap_key, &nonce, &ad, sealed)?);
    let mut key_bytes = [0; 32];
    key_bytes.copy_from_slice(&opened);
    let key = EpochKey::from_bytes(key_bytes);
    if key.key_id() != key_id {
        return Err(CryptoError::WrongKey);
    }
    Ok(key)
}

fn derive(
    shared: &[u8; 32],
    ephemeral_public: &[u8; 32],
    recipient: &Identity,
) -> Result<(Zeroizing<[u8; 32]>, [u8; 24]), CryptoError> {
    let mut info = Vec::with_capacity(WRAP_INFO.len() + 96);
    info.extend_from_slice(WRAP_INFO);
    info.extend_from_slice(ephemeral_public);
    info.extend_from_slice(&recipient.to_bytes());
    let mut okm = Zeroizing::new([0; 56]);
    Hkdf::<Sha256>::new(None, shared)
        .expand(&info, okm.as_mut_slice())
        .map_err(|_| CryptoError::Malformed)?;
    let mut wrap_key = Zeroizing::new([0; 32]);
    wrap_key.copy_from_slice(&okm[..32]);
    let mut nonce = [0; 24];
    nonce.copy_from_slice(&okm[32..]);
    Ok((wrap_key, nonce))
}

fn associated_data(
    document_id: DocumentId,
    epoch: u32,
    key_id: KeyId,
    recipient: &Identity,
) -> Vec<u8> {
    let mut writer = Writer::default();
    writer.raw(&document_id.0);
    writer.u32(epoch);
    writer.raw(&key_id.0);
    writer.raw(&recipient.to_bytes());
    writer.bytes
}
