//! The sealed envelope of every record, snapshot and presence message (ADR 0031 decision 6).
//!
//! Layout: header (39 bytes) ‖ nonce (24) ‖ ciphertext ‖ tag (16) [‖ signer (32) ‖
//! signature (64) when the signed flag is set]. Header: `version u8, kind u8, flags u8,
//! epoch u32, key_id [8], document_id [16], sender_peer u64`, little-endian, all of it
//! associated data. The signature covers `"curvyo-envelope-sig-v1"` ‖ everything before it
//! ‖ the signer key. There is no unsealed variant (ADR 0008 §1).

use crate::codec::{Reader, Writer};
use crate::identity::{IdentityKeys, MemberId, verify};
use crate::{CryptoError, DocumentId, EpochKey, KeyId, aead};

/// The envelope format version this build writes and reads.
pub const ENVELOPE_VERSION: u8 = 1;
/// The header length in bytes.
pub const HEADER_LEN: usize = 39;
/// The nonce length in bytes.
pub const NONCE_LEN: usize = 24;
const SIGNED_FLAG: u8 = 1;
const SIGNATURE_BLOCK_LEN: usize = 32 + 64;
const SIGNATURE_TAG: &[u8] = b"curvyo-envelope-sig-v1";

/// What an envelope carries; part of the associated data, so one kind cannot be replayed as
/// another.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EnvelopeKind {
    /// A Loro update record.
    Update,
    /// The bytes of an embedded asset.
    Asset,
    /// A full snapshot; always signed.
    Snapshot,
    /// A presence message; relayed, never stored.
    Presence,
}

impl EnvelopeKind {
    fn tag(self) -> u8 {
        match self {
            Self::Update => 1,
            Self::Asset => 2,
            Self::Snapshot => 3,
            Self::Presence => 4,
        }
    }

    fn from_tag(tag: u8) -> Result<Self, CryptoError> {
        match tag {
            1 => Ok(Self::Update),
            2 => Ok(Self::Asset),
            3 => Ok(Self::Snapshot),
            4 => Ok(Self::Presence),
            _ => Err(CryptoError::Malformed),
        }
    }
}

/// The cleartext header: what the relay may route on and what the AEAD binds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnvelopeHeader {
    /// What the envelope carries.
    pub kind: EnvelopeKind,
    /// Whether a signer and signature follow the ciphertext.
    pub signed: bool,
    /// The key epoch number.
    pub epoch: u32,
    /// The id of the epoch key it is sealed under.
    pub key_id: KeyId,
    /// The document it belongs to.
    pub document_id: DocumentId,
    /// The sender's Loro peer id (ADR 0031 decision 4).
    pub sender_peer: u64,
}

impl EnvelopeHeader {
    fn to_bytes(self) -> Vec<u8> {
        let mut writer = Writer::default();
        writer.u8(ENVELOPE_VERSION);
        writer.u8(self.kind.tag());
        writer.u8(if self.signed { SIGNED_FLAG } else { 0 });
        writer.u32(self.epoch);
        writer.raw(&self.key_id.0);
        writer.raw(&self.document_id.0);
        writer.u64(self.sender_peer);
        writer.bytes
    }

    /// Reads the header of an envelope without opening it (the relay's and the router's view).
    ///
    /// # Errors
    /// `Malformed` for a short envelope, unknown kind or unknown flag; `UnknownVersion` for
    /// another version.
    pub fn parse(envelope: &[u8]) -> Result<Self, CryptoError> {
        let mut reader = Reader::new(envelope);
        let version = reader.u8()?;
        if version != ENVELOPE_VERSION {
            return Err(CryptoError::UnknownVersion(version));
        }
        let kind = EnvelopeKind::from_tag(reader.u8()?)?;
        let flags = reader.u8()?;
        if flags & !SIGNED_FLAG != 0 {
            return Err(CryptoError::Malformed);
        }
        let header = Self {
            kind,
            signed: flags & SIGNED_FLAG != 0,
            epoch: reader.u32()?,
            key_id: KeyId(reader.array()?),
            document_id: DocumentId(reader.array()?),
            sender_peer: reader.u64()?,
        };
        let min_len = HEADER_LEN
            + NONCE_LEN
            + aead::TAG_LEN
            + if header.signed {
                SIGNATURE_BLOCK_LEN
            } else {
                0
            };
        if envelope.len() < min_len {
            return Err(CryptoError::Malformed);
        }
        Ok(header)
    }
}

/// Where and under which key an envelope is sealed.
pub struct Sealing<'a> {
    /// What it carries.
    pub kind: EnvelopeKind,
    /// The epoch number of `key`.
    pub epoch: u32,
    /// The epoch key.
    pub key: &'a EpochKey,
    /// The document it belongs to.
    pub document_id: DocumentId,
    /// The sender's Loro peer id.
    pub sender_peer: u64,
    /// Who signs it; required for a snapshot, optional (and unset in slice 1) otherwise.
    pub signer: Option<&'a IdentityKeys>,
}

/// Seals `plaintext` with a nonce from the caller's `NonceStream`.
///
/// # Errors
/// `Unsigned` for a snapshot without a signer.
pub fn seal(
    sealing: &Sealing<'_>,
    nonce: [u8; 24],
    plaintext: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    if sealing.kind == EnvelopeKind::Snapshot && sealing.signer.is_none() {
        return Err(CryptoError::Unsigned);
    }
    let header = EnvelopeHeader {
        kind: sealing.kind,
        signed: sealing.signer.is_some(),
        epoch: sealing.epoch,
        key_id: sealing.key.key_id(),
        document_id: sealing.document_id,
        sender_peer: sealing.sender_peer,
    }
    .to_bytes();
    let mut envelope = header.clone();
    envelope.extend_from_slice(&nonce);
    envelope.extend_from_slice(&aead::seal(
        sealing.key.as_bytes(),
        &nonce,
        &header,
        plaintext,
    )?);
    if let Some(signer) = sealing.signer {
        let member = signer.identity().member;
        let signature = signer.sign(&signature_message(&envelope, &member));
        envelope.extend_from_slice(&member.0);
        envelope.extend_from_slice(&signature);
    }
    Ok(envelope)
}

/// An opened envelope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Opened {
    /// Its header.
    pub header: EnvelopeHeader,
    /// The plaintext.
    pub plaintext: Vec<u8>,
    /// Who signed it, if it is signed; the caller checks that the signer is a member.
    pub signer: Option<MemberId>,
}

/// Opens an envelope expected to be of `kind` and to belong to `document_id`, with the key the
/// caller looked up by the header's epoch and `key_id`.
///
/// # Errors
/// `WrongDocument`, `WrongKind`, `WrongKey` for a header that does not match;
/// `Unsigned` for an unsigned snapshot; `BadSignature`; `Unauthentic` for any changed byte.
pub fn open(
    envelope: &[u8],
    key: &EpochKey,
    document_id: DocumentId,
    kind: EnvelopeKind,
) -> Result<Opened, CryptoError> {
    let header = EnvelopeHeader::parse(envelope)?;
    if header.document_id != document_id {
        return Err(CryptoError::WrongDocument);
    }
    if header.kind != kind {
        return Err(CryptoError::WrongKind);
    }
    if header.key_id != key.key_id() {
        return Err(CryptoError::WrongKey);
    }
    if kind == EnvelopeKind::Snapshot && !header.signed {
        return Err(CryptoError::Unsigned);
    }
    let (body, signer) = if header.signed {
        let (body, block) = envelope.split_at(envelope.len() - SIGNATURE_BLOCK_LEN);
        let mut reader = Reader::new(block);
        let member = MemberId(reader.array()?);
        let signature: [u8; 64] = reader.array()?;
        verify(&member, &signature_message(body, &member), &signature)?;
        (body, Some(member))
    } else {
        (envelope, None)
    };
    let (associated_data, rest) = body.split_at(HEADER_LEN);
    let (nonce, sealed) = rest.split_at(NONCE_LEN);
    let mut nonce_bytes = [0; NONCE_LEN];
    nonce_bytes.copy_from_slice(nonce);
    let plaintext = aead::open(key.as_bytes(), &nonce_bytes, associated_data, sealed)?;
    Ok(Opened {
        header,
        plaintext,
        signer,
    })
}

fn signature_message(body: &[u8], signer: &MemberId) -> Vec<u8> {
    let mut message = Vec::with_capacity(SIGNATURE_TAG.len() + body.len() + 32);
    message.extend_from_slice(SIGNATURE_TAG);
    message.extend_from_slice(body);
    message.extend_from_slice(&signer.0);
    message
}
