//! The passkey backup of a browser device (ADR 0040 decision 13): the keys a passkey's PRF output
//! gives, and the sealed backup record v1 the relay keeps under an opaque id.
//!
//! The browser asks its passkey to evaluate the PRF on [`backup_prf_input`]; the 32-byte output
//! is all [`BackupKeys::derive`] needs. Nothing here touches `WebAuthn`: the relay sees the record
//! id, the write key, the sealed record and its size, never the passkey or what it seals.

use std::fmt;

use hkdf::Hkdf;
use sha2::{Digest, Sha256, Sha512};
use zeroize::Zeroizing;

use crate::codec::{Reader, Writer};
use crate::{CryptoError, DocumentId, IdentityKeys, MemberId, aead, verify};

/// The version byte of a sealed backup record.
pub const BACKUP_RECORD_VERSION: u8 = 1;
/// The longest server address a backup record holds, in bytes.
pub const BACKUP_SERVER_MAX: usize = 1024;
/// The longest device name a backup record holds, in bytes.
pub const BACKUP_LABEL_MAX: usize = 256;

/// The HKDF salt and the associated data's prefix of backup record v1.
const CONTEXT: &[u8] = b"curvyo-backup-v1";
/// The signing context of a backup write; never the relay's `"curvyo-relay-v1"`, so neither
/// signature passes as the other.
const WRITE_CONTEXT: &[u8] = b"curvyo-backup-write-v1";
const NONCE_LEN: usize = 24;

/// The input every browser asks its passkey's PRF to evaluate: SHA-256 of `"curvyo-backup-v1"`.
/// It must never change, or no passkey finds its backup again.
#[must_use]
pub fn backup_prf_input() -> [u8; 32] {
    Sha256::digest(CONTEXT).into()
}

/// The opaque id the relay stores a backup under.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BackupId(pub [u8; 16]);

/// What a backup write does; signed, so a put never passes as a delete.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackupOp {
    /// Store or replace the record.
    Put,
    /// Delete the record.
    Delete,
}

/// What a browser backs up: its identity and where its device group lives. Epoch keys are not
/// in it: a member gets them back from the keyring wraps on connecting (ADR 0010 §6).
#[derive(Clone, PartialEq, Eq)]
pub struct BackupContent {
    /// The identity entropy (`IdentityKeys::from_entropy`).
    pub identity: Zeroizing<[u8; 64]>,
    /// The device group room.
    pub group_room: DocumentId,
    /// The relay's address the group lives on.
    pub server: String,
    /// This device's name in the devices list.
    pub label: String,
}

impl fmt::Debug for BackupContent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BackupContent")
            .field("identity", &"[secret]")
            .field("group_room", &self.group_room)
            .field("server", &self.server)
            .field("label", &self.label)
            .finish()
    }
}

/// The keys one passkey's PRF output gives: the record's wrap key, its id and its write key.
pub struct BackupKeys {
    wrap: Zeroizing<[u8; 32]>,
    id: BackupId,
    write: IdentityKeys,
}

impl BackupKeys {
    /// HKDF-SHA-256 over the PRF output with salt `"curvyo-backup-v1"` and one info string per
    /// key (`wrap`, `id`, `write`); the same passkey always gives the same keys.
    #[must_use]
    pub fn derive(prf_output: &[u8; 32]) -> Self {
        let hkdf = Hkdf::<Sha256>::new(Some(CONTEXT), prf_output);
        let mut wrap = Zeroizing::new([0; 32]);
        let mut id = [0; 16];
        let mut write = Zeroizing::new([0; 64]);
        // invariant: every length here is far below HKDF-SHA-256's 8,160-byte limit.
        for (info, out) in [
            (&b"wrap"[..], wrap.as_mut_slice()),
            (&b"id"[..], id.as_mut_slice()),
            (&b"write"[..], write.as_mut_slice()),
        ] {
            if hkdf.expand(info, out).is_err() {
                unreachable!("HKDF output lengths are fixed and small");
            }
        }
        Self {
            wrap,
            id: BackupId(id),
            write: IdentityKeys::from_entropy(&write),
        }
    }

    /// The record id.
    #[must_use]
    pub fn id(&self) -> BackupId {
        self.id
    }

    /// The public write key the relay pins on the first put.
    #[must_use]
    pub fn write_key(&self) -> MemberId {
        self.write.identity().member
    }

    /// Seals `content` as backup record v1: version ‖ nonce ‖ XChaCha20-Poly1305 ciphertext,
    /// the version and the record id bound as associated data.
    ///
    /// # Errors
    /// `Malformed` for a server address or name over its limit.
    pub fn seal(&self, content: &BackupContent, nonce: &[u8; 24]) -> Result<Vec<u8>, CryptoError> {
        if content.server.len() > BACKUP_SERVER_MAX || content.label.len() > BACKUP_LABEL_MAX {
            return Err(CryptoError::Malformed);
        }
        let mut plain = Writer::default();
        plain.raw(content.identity.as_slice());
        plain.raw(&content.group_room.0);
        plain.count(content.server.len())?;
        plain.raw(content.server.as_bytes());
        plain.count(content.label.len())?;
        plain.raw(content.label.as_bytes());
        let plain = Zeroizing::new(plain.bytes);
        let sealed = aead::seal(&self.wrap, nonce, &self.associated_data(), &plain)?;
        let mut out = Vec::with_capacity(1 + NONCE_LEN + sealed.len());
        out.push(BACKUP_RECORD_VERSION);
        out.extend_from_slice(nonce);
        out.extend_from_slice(&sealed);
        Ok(out)
    }

    /// Opens a backup record sealed by [`BackupKeys::seal`].
    ///
    /// # Errors
    /// `UnknownVersion` for another version; `Unauthentic` for another passkey or a changed
    /// byte; `Malformed` for a record that opens but does not parse.
    pub fn open(&self, record: &[u8]) -> Result<BackupContent, CryptoError> {
        let mut reader = Reader::new(record);
        let version = reader.u8()?;
        if version != BACKUP_RECORD_VERSION {
            return Err(CryptoError::UnknownVersion(version));
        }
        let nonce: [u8; NONCE_LEN] = reader.array()?;
        let sealed = &record[1 + NONCE_LEN..];
        let plain = Zeroizing::new(aead::open(
            &self.wrap,
            &nonce,
            &self.associated_data(),
            sealed,
        )?);
        let mut reader = Reader::new(&plain);
        let identity = Zeroizing::new(reader.array::<64>()?);
        let group_room = DocumentId(reader.array()?);
        let server = text(&mut reader, BACKUP_SERVER_MAX)?;
        let label = text(&mut reader, BACKUP_LABEL_MAX)?;
        reader.finish()?;
        Ok(BackupContent {
            identity,
            group_room,
            server,
            label,
        })
    }

    /// The write key's signature over `op` on this record with the relay's `nonce` and the
    /// record bytes (empty for a delete).
    #[must_use]
    pub fn sign_write(&self, op: BackupOp, nonce: &[u8; 32], record: &[u8]) -> [u8; 64] {
        self.write.sign(&write_message(op, self.id, nonce, record))
    }

    fn associated_data(&self) -> Vec<u8> {
        let mut data = Vec::with_capacity(CONTEXT.len() + 1 + 16);
        data.extend_from_slice(CONTEXT);
        data.push(BACKUP_RECORD_VERSION);
        data.extend_from_slice(&self.id.0);
        data
    }
}

/// Checks a backup write's signature: `write_key` signed `op` on record `id` with the relay's
/// `nonce` and these record bytes.
///
/// # Errors
/// `BadSignature`.
pub fn verify_backup_write(
    write_key: &MemberId,
    op: BackupOp,
    id: BackupId,
    nonce: &[u8; 32],
    record: &[u8],
    signature: &[u8; 64],
) -> Result<(), CryptoError> {
    verify(write_key, &write_message(op, id, nonce, record), signature)
}

/// `"curvyo-backup-write-v1"` ‖ op ‖ id ‖ nonce ‖ SHA-512(record).
fn write_message(op: BackupOp, id: BackupId, nonce: &[u8; 32], record: &[u8]) -> Vec<u8> {
    let mut message = Vec::with_capacity(WRITE_CONTEXT.len() + 1 + 16 + 32 + 64);
    message.extend_from_slice(WRITE_CONTEXT);
    message.push(match op {
        BackupOp::Put => 1,
        BackupOp::Delete => 2,
    });
    message.extend_from_slice(&id.0);
    message.extend_from_slice(nonce);
    message.extend_from_slice(&Sha512::digest(record));
    message
}

fn text(reader: &mut Reader<'_>, max: usize) -> Result<String, CryptoError> {
    let len = usize::from(reader.u16()?);
    if len > max {
        return Err(CryptoError::Malformed);
    }
    String::from_utf8(reader.take(len)?.to_vec()).map_err(|_| CryptoError::Malformed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn content() -> BackupContent {
        BackupContent {
            identity: Zeroizing::new([7; 64]),
            group_room: DocumentId([9; 16]),
            server: "wss://sync.example.org".to_owned(),
            label: "Firefox on Linux".to_owned(),
        }
    }

    #[test]
    fn a_record_opens_with_the_same_passkey_only() {
        let keys = BackupKeys::derive(&[1; 32]);
        let record = keys.seal(&content(), &[2; 24]).unwrap();
        assert_eq!(keys.open(&record).unwrap(), content());
        let other = BackupKeys::derive(&[3; 32]);
        assert_eq!(other.open(&record), Err(CryptoError::Unauthentic));
    }

    #[test]
    fn the_keys_differ_from_each_other() {
        let keys = BackupKeys::derive(&[1; 32]);
        assert_ne!(keys.id().0.as_slice(), &keys.wrap[..16]);
        assert_ne!(keys.write_key().0, *keys.wrap);
    }

    #[test]
    fn a_put_signature_is_no_delete_signature() {
        let keys = BackupKeys::derive(&[1; 32]);
        let signature = keys.sign_write(BackupOp::Put, &[4; 32], b"record");
        let check = |op, nonce: &[u8; 32], record: &[u8]| {
            verify_backup_write(&keys.write_key(), op, keys.id(), nonce, record, &signature)
        };
        assert_eq!(check(BackupOp::Put, &[4; 32], b"record"), Ok(()));
        assert!(check(BackupOp::Delete, &[4; 32], b"record").is_err());
        assert!(check(BackupOp::Put, &[5; 32], b"record").is_err());
        assert!(check(BackupOp::Put, &[4; 32], b"other").is_err());
    }

    #[test]
    fn over_long_texts_are_refused() {
        let keys = BackupKeys::derive(&[1; 32]);
        let mut long = content();
        long.label = "x".repeat(BACKUP_LABEL_MAX + 1);
        assert_eq!(keys.seal(&long, &[0; 24]), Err(CryptoError::Malformed));
    }

    #[test]
    fn debug_hides_the_identity() {
        assert!(!format!("{:?}", content()).contains("7, 7"));
    }
}
