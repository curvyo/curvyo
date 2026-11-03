//! Keyring entries (ADR 0010 §2, §3): their binary codec, hash and signature.
//!
//! Entry layout, little-endian: `version u8 (1)`, `parent count u16`, parents (32 bytes each,
//! strictly ascending), `author [32]`, `payload tag u8`, payload, `signature [64]`. The hash
//! is SHA-256(`"curvyo-keyring-entry-v1"` ‖ everything before the signature); the signature
//! is the author's ed25519 signature over `"curvyo-keyring-sig-v1"` ‖ hash.

use sha2::{Digest, Sha256};

use crate::codec::{Reader, Writer};
use crate::identity::{Identity, IdentityKeys, MemberId, verify};
use crate::invite::InviteId;
use crate::wrap::{WRAP_LEN, Wrap};
use crate::{CryptoError, DocumentId};

/// The entry format version this build writes and reads.
pub const ENTRY_VERSION: u8 = 1;
const HASH_TAG: &[u8] = b"curvyo-keyring-entry-v1";
const SIGNATURE_TAG: &[u8] = b"curvyo-keyring-sig-v1";

/// The hash of an entry; parents name entries by it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntryHash(pub [u8; 32]);

/// The two roles of ADR 0010 §12. `Member` sorts first: the lower role wins a conflict.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Role {
    /// May edit and rewrap.
    Member,
    /// May also add, remove, change roles, invite and rotate.
    Admin,
}

impl Role {
    fn tag(self) -> u8 {
        match self {
            Self::Admin => 1,
            Self::Member => 2,
        }
    }

    fn from_tag(tag: u8) -> Result<Self, CryptoError> {
        match tag {
            1 => Ok(Self::Admin),
            2 => Ok(Self::Member),
            _ => Err(CryptoError::Malformed),
        }
    }
}

/// A wrap of one epoch's key for the member it names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecipientWrap {
    /// The recipient.
    pub member: MemberId,
    /// The wrapped key.
    pub wrap: Wrap,
}

/// A wrap of the key of epoch `epoch` (for the member an `AddMember` adds).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EpochWrap {
    /// The epoch number; the wrap's `key_id` names which key of that number.
    pub epoch: u32,
    /// The wrapped key.
    pub wrap: Wrap,
}

/// The eight payloads of ADR 0010 §3, and no other.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Payload {
    /// The root: the document, its first admins and epoch 0's wraps.
    Genesis {
        /// The document (and in slice 1 room) id.
        document_id: DocumentId,
        /// The first admins.
        admins: Vec<Identity>,
        /// Epoch 0's key wrapped to each admin.
        epoch0_wraps: Vec<RecipientWrap>,
    },
    /// A member added directly (the verified flow, slice 2).
    AddMember {
        /// Who is added.
        identity: Identity,
        /// With which role.
        role: Role,
        /// The epoch keys they receive.
        wraps: Vec<EpochWrap>,
    },
    /// A member removed by an admin, or leaving by themselves.
    RemoveMember {
        /// Who is removed.
        member: MemberId,
    },
    /// A member's role changed.
    SetRole {
        /// Whose role.
        member: MemberId,
        /// The new role.
        role: Role,
    },
    /// A key rotation.
    NewEpoch {
        /// The new epoch number: the highest known plus one.
        epoch: u32,
        /// The new key wrapped to every current member.
        wraps: Vec<RecipientWrap>,
    },
    /// A one-time invite.
    AddPendingInvite {
        /// The invite key's public half.
        invite_id: InviteId,
        /// The role the claimant gets.
        role: Role,
        /// Expiry in seconds since the Unix epoch; checked by callers with their clock.
        expires: u64,
    },
    /// A joiner binding its identity to an invite.
    ClaimInvite {
        /// The invite claimed.
        invite_id: InviteId,
        /// The joiner.
        identity: Identity,
        /// The invite key's signature over `document_id ‖ invite_id ‖ identity`.
        proof: [u8; 64],
    },
    /// A member handing another member an epoch key it lacks.
    RewrapEpoch {
        /// The epoch number; the wrap's `key_id` names which key.
        epoch: u32,
        /// The recipient.
        member: MemberId,
        /// The wrapped key.
        wrap: Wrap,
    },
}

/// One signed keyring entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// The entries it builds on, strictly ascending; empty only for the genesis.
    pub parents: Vec<EntryHash>,
    /// The author's member id.
    pub author: MemberId,
    /// What it says.
    pub payload: Payload,
    /// The author's signature over the hash.
    pub signature: [u8; 64],
}

impl Entry {
    /// Builds and signs an entry; `parents` are sorted and deduplicated.
    ///
    /// # Errors
    /// `Malformed` for a list longer than `u16::MAX`.
    pub fn sign(
        mut parents: Vec<EntryHash>,
        author: &IdentityKeys,
        payload: Payload,
    ) -> Result<Self, CryptoError> {
        parents.sort_unstable();
        parents.dedup();
        let mut entry = Self {
            parents,
            author: author.identity().member,
            payload,
            signature: [0; 64],
        };
        let hash = entry.try_hash()?;
        entry.signature = author.sign(&signature_message(hash));
        Ok(entry)
    }

    /// The entry's hash.
    ///
    /// # Errors
    /// `Malformed` for a list longer than `u16::MAX` (never for a decoded entry).
    pub fn try_hash(&self) -> Result<EntryHash, CryptoError> {
        let unsigned = self.unsigned_bytes()?;
        Ok(EntryHash(
            Sha256::new()
                .chain_update(HASH_TAG)
                .chain_update(&unsigned)
                .finalize()
                .into(),
        ))
    }

    /// Whether the signature verifies under the author key.
    ///
    /// # Errors
    /// `BadSignature`.
    pub fn verify_signature(&self) -> Result<(), CryptoError> {
        verify(
            &self.author,
            &signature_message(self.try_hash()?),
            &self.signature,
        )
    }

    /// The binary encoding.
    ///
    /// # Errors
    /// `Malformed` for a list longer than `u16::MAX`.
    pub fn encode(&self) -> Result<Vec<u8>, CryptoError> {
        let mut bytes = self.unsigned_bytes()?;
        bytes.extend_from_slice(&self.signature);
        Ok(bytes)
    }

    /// Reads one entry; trailing bytes, unsorted parents and unknown tags are refused.
    ///
    /// # Errors
    /// `Malformed`, `UnknownVersion`.
    pub fn decode(bytes: &[u8]) -> Result<Self, CryptoError> {
        let mut reader = Reader::new(bytes);
        let version = reader.u8()?;
        if version != ENTRY_VERSION {
            return Err(CryptoError::UnknownVersion(version));
        }
        let parent_count = reader.u16()?;
        let mut parents = Vec::with_capacity(usize::from(parent_count));
        for _ in 0..parent_count {
            let parent = EntryHash(reader.array()?);
            if parents.last().is_some_and(|last| *last >= parent) {
                return Err(CryptoError::Malformed);
            }
            parents.push(parent);
        }
        let author = MemberId(reader.array()?);
        let payload = read_payload(&mut reader)?;
        let signature = reader.array()?;
        reader.finish()?;
        Ok(Self {
            parents,
            author,
            payload,
            signature,
        })
    }

    fn unsigned_bytes(&self) -> Result<Vec<u8>, CryptoError> {
        let mut writer = Writer::default();
        writer.u8(ENTRY_VERSION);
        writer.count(self.parents.len())?;
        for parent in &self.parents {
            writer.raw(&parent.0);
        }
        writer.raw(&self.author.0);
        write_payload(&mut writer, &self.payload)?;
        Ok(writer.bytes)
    }
}

fn signature_message(hash: EntryHash) -> Vec<u8> {
    let mut message = Vec::with_capacity(SIGNATURE_TAG.len() + 32);
    message.extend_from_slice(SIGNATURE_TAG);
    message.extend_from_slice(&hash.0);
    message
}

fn write_payload(writer: &mut Writer, payload: &Payload) -> Result<(), CryptoError> {
    match payload {
        Payload::Genesis {
            document_id,
            admins,
            epoch0_wraps,
        } => {
            writer.u8(1);
            writer.raw(&document_id.0);
            writer.count(admins.len())?;
            for admin in admins {
                writer.raw(&admin.to_bytes());
            }
            write_recipient_wraps(writer, epoch0_wraps)?;
        }
        Payload::AddMember {
            identity,
            role,
            wraps,
        } => {
            writer.u8(2);
            writer.raw(&identity.to_bytes());
            writer.u8(role.tag());
            writer.count(wraps.len())?;
            for wrap in wraps {
                writer.u32(wrap.epoch);
                writer.raw(&wrap.wrap.0);
            }
        }
        Payload::RemoveMember { member } => {
            writer.u8(3);
            writer.raw(&member.0);
        }
        Payload::SetRole { member, role } => {
            writer.u8(4);
            writer.raw(&member.0);
            writer.u8(role.tag());
        }
        Payload::NewEpoch { epoch, wraps } => {
            writer.u8(5);
            writer.u32(*epoch);
            write_recipient_wraps(writer, wraps)?;
        }
        Payload::AddPendingInvite {
            invite_id,
            role,
            expires,
        } => {
            writer.u8(6);
            writer.raw(&invite_id.0);
            writer.u8(role.tag());
            writer.u64(*expires);
        }
        Payload::ClaimInvite {
            invite_id,
            identity,
            proof,
        } => {
            writer.u8(7);
            writer.raw(&invite_id.0);
            writer.raw(&identity.to_bytes());
            writer.raw(proof);
        }
        Payload::RewrapEpoch {
            epoch,
            member,
            wrap,
        } => {
            writer.u8(8);
            writer.u32(*epoch);
            writer.raw(&member.0);
            writer.raw(&wrap.0);
        }
    }
    Ok(())
}

fn write_recipient_wraps(writer: &mut Writer, wraps: &[RecipientWrap]) -> Result<(), CryptoError> {
    writer.count(wraps.len())?;
    for wrap in wraps {
        writer.raw(&wrap.member.0);
        writer.raw(&wrap.wrap.0);
    }
    Ok(())
}

fn read_payload(reader: &mut Reader<'_>) -> Result<Payload, CryptoError> {
    Ok(match reader.u8()? {
        1 => {
            let document_id = DocumentId(reader.array()?);
            let count = reader.u16()?;
            let mut admins = Vec::new();
            for _ in 0..count {
                admins.push(Identity::from_bytes(&reader.array()?));
            }
            Payload::Genesis {
                document_id,
                admins,
                epoch0_wraps: read_recipient_wraps(reader)?,
            }
        }
        2 => {
            let identity = Identity::from_bytes(&reader.array()?);
            let role = Role::from_tag(reader.u8()?)?;
            let count = reader.u16()?;
            let mut wraps = Vec::new();
            for _ in 0..count {
                wraps.push(EpochWrap {
                    epoch: reader.u32()?,
                    wrap: Wrap(reader.array::<WRAP_LEN>()?),
                });
            }
            Payload::AddMember {
                identity,
                role,
                wraps,
            }
        }
        3 => Payload::RemoveMember {
            member: MemberId(reader.array()?),
        },
        4 => Payload::SetRole {
            member: MemberId(reader.array()?),
            role: Role::from_tag(reader.u8()?)?,
        },
        5 => Payload::NewEpoch {
            epoch: reader.u32()?,
            wraps: read_recipient_wraps(reader)?,
        },
        6 => Payload::AddPendingInvite {
            invite_id: InviteId(reader.array()?),
            role: Role::from_tag(reader.u8()?)?,
            expires: reader.u64()?,
        },
        7 => Payload::ClaimInvite {
            invite_id: InviteId(reader.array()?),
            identity: Identity::from_bytes(&reader.array()?),
            proof: reader.array()?,
        },
        8 => Payload::RewrapEpoch {
            epoch: reader.u32()?,
            member: MemberId(reader.array()?),
            wrap: Wrap(reader.array::<WRAP_LEN>()?),
        },
        _ => return Err(CryptoError::Malformed),
    })
}

fn read_recipient_wraps(reader: &mut Reader<'_>) -> Result<Vec<RecipientWrap>, CryptoError> {
    let count = reader.u16()?;
    let mut wraps = Vec::new();
    for _ in 0..count {
        wraps.push(RecipientWrap {
            member: MemberId(reader.array()?),
            wrap: Wrap(reader.array::<WRAP_LEN>()?),
        });
    }
    Ok(wraps)
}
