//! The pairing channel after `CPace` (ADR 0040 decision 4): its key schedule, the confirmation
//! tags, the check words and the pairing messages v1 that travel inside the relay's `PairMsg`.
//!
//! Pairing messages v1, each `version u8 = 1 ‖ kind u8 ‖ body`:
//! - kind 1 `Share`: the sender's `CPace` share (32 bytes);
//! - kind 2 `Confirm`: the sender's confirmation tag (32 bytes) ‖ sealed hello;
//! - kind 3 `Welcome`: sealed group room id (opener only).
//!
//! A sealed body is XChaCha20-Poly1305 under the sender's seal key with the nonce
//! `kind ‖ sender role ‖ 22 zero bytes` and the associated data `version ‖ kind ‖
//! "curvyo-pair-v1" ‖ sid ‖ sender role`. The keys are fresh per pairing and each kind is sealed
//! at most once per sender, so a nonce is never reused. A hello is `identity (64) ‖ label length
//! u8 ‖ label (UTF-8, 1 to 255 bytes) ‖ ed25519 signature (64)` over `"curvyo-pair-v1 sig" ‖
//! sender role ‖ sid ‖ transcript ‖ identity ‖ label`.

use hkdf::Hkdf;
use sha2::Sha256;
use subtle::ConstantTimeEq;
use zeroize::Zeroizing;

use crate::codec::{Reader, Writer};
use crate::identity::{Identity, IdentityKeys, verify};
use crate::pair_code::PairSecret;
use crate::pake::{CpaceParty, lv_cat, transcript_ir};
use crate::{CryptoError, DocumentId, aead};

/// The context string that binds every pairing message to this protocol version.
const CONTEXT: &[u8] = b"curvyo-pair-v1";
/// The format version of pairing messages.
pub const PAIR_MESSAGE_VERSION: u8 = 1;

const KIND_SHARE: u8 = 1;
const KIND_CONFIRM: u8 = 2;
const KIND_WELCOME: u8 = 3;

/// Which side of a pairing a device is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairRole {
    /// The device that shows the code (A); it admits the other one.
    Opener,
    /// The device that types the code (B); the `CPace` initiator.
    Joiner,
}

impl PairRole {
    fn byte(self) -> u8 {
        match self {
            Self::Opener => 1,
            Self::Joiner => 2,
        }
    }

    fn peer(self) -> Self {
        match self {
            Self::Opener => Self::Joiner,
            Self::Joiner => Self::Opener,
        }
    }
}

/// One pairing message v1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PairMessage {
    /// A `CPace` share.
    Share([u8; 32]),
    /// A confirmation tag and the sender's sealed hello.
    Confirm {
        /// The sender's confirmation tag.
        tag: [u8; 32],
        /// The sealed hello.
        sealed: Vec<u8>,
    },
    /// The opener's sealed group room id.
    Welcome {
        /// The sealed room id.
        sealed: Vec<u8>,
    },
}

impl PairMessage {
    /// The message's bytes.
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut writer = Writer::default();
        writer.u8(PAIR_MESSAGE_VERSION);
        match self {
            Self::Share(share) => {
                writer.u8(KIND_SHARE);
                writer.raw(share);
            }
            Self::Confirm { tag, sealed } => {
                writer.u8(KIND_CONFIRM);
                writer.raw(tag);
                writer.raw(sealed);
            }
            Self::Welcome { sealed } => {
                writer.u8(KIND_WELCOME);
                writer.raw(sealed);
            }
        }
        writer.bytes
    }

    /// Reads a message.
    ///
    /// # Errors
    /// `UnknownVersion` for another version; `Malformed` for an unknown kind or a wrong length.
    pub fn parse(bytes: &[u8]) -> Result<Self, CryptoError> {
        let mut reader = Reader::new(bytes);
        let version = reader.u8()?;
        if version != PAIR_MESSAGE_VERSION {
            return Err(CryptoError::UnknownVersion(version));
        }
        let kind = reader.u8()?;
        let rest = &bytes[2..];
        match kind {
            KIND_SHARE => {
                let share = reader.array()?;
                reader.finish()?;
                Ok(Self::Share(share))
            }
            KIND_CONFIRM => {
                let tag = reader.array()?;
                Ok(Self::Confirm {
                    tag,
                    sealed: rest[32..].to_vec(),
                })
            }
            KIND_WELCOME => Ok(Self::Welcome {
                sealed: rest.to_vec(),
            }),
            _ => Err(CryptoError::Malformed),
        }
    }
}

/// What a peer said about itself in its hello, after its tag, seal and signature checked out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairHello {
    /// The peer's public identity.
    pub identity: Identity,
    /// The device name the peer chose.
    pub label: String,
}

/// The first step of a pairing: this side's `CPace` share.
pub struct PairStart {
    role: PairRole,
    sid: [u8; 16],
    party: CpaceParty,
}

impl PairStart {
    /// Starts `CPace` with the code's secret as password, the relay's `sid` and the channel
    /// identifier `lv_cat("curvyo-pair-v1", server_url)`; the scalar comes from 32 passed-in
    /// bytes.
    ///
    /// # Errors
    /// `Malformed` for entropy that gives the zero scalar.
    pub fn new(
        role: PairRole,
        secret: PairSecret,
        server_url: &str,
        sid: [u8; 16],
        entropy: &[u8; 32],
    ) -> Result<Self, CryptoError> {
        let channel_id = lv_cat(&[CONTEXT, server_url.as_bytes()]);
        let party = CpaceParty::new(&secret.password(), &channel_id, &sid, entropy)?;
        Ok(Self { role, sid, party })
    }

    /// The `Share` message to send.
    #[must_use]
    pub fn share_message(&self) -> PairMessage {
        PairMessage::Share(self.party.share())
    }

    /// Completes `CPace` with the peer's share and derives the channel keys.
    ///
    /// # Errors
    /// `BadPublicKey` for a share that does not decode or gives the neutral element.
    pub fn finish(self, peer_share: &[u8; 32]) -> Result<PairChannel, CryptoError> {
        let own = self.party.share();
        let (joiner, opener) = match self.role {
            PairRole::Joiner => (own, *peer_share),
            PairRole::Opener => (*peer_share, own),
        };
        let transcript = transcript_ir(&joiner, b"", &opener, b"");
        let isk = Zeroizing::new(self.party.isk(&self.sid, peer_share, &transcript)?);
        let hkdf = Hkdf::<Sha256>::new(Some(&self.sid), isk.as_slice());
        let derive = |parts: &[&[u8]]| -> Result<Zeroizing<[u8; 32]>, CryptoError> {
            let mut key = Zeroizing::new([0; 32]);
            hkdf.expand_multi_info(parts, key.as_mut_slice())
                .map_err(|_| CryptoError::Malformed)?;
            Ok(key)
        };
        Ok(PairChannel {
            role: self.role,
            sid: self.sid,
            confirm_opener: derive(&[CONTEXT, b" confirm opener", &transcript])?,
            confirm_joiner: derive(&[CONTEXT, b" confirm joiner", &transcript])?,
            seal_opener: derive(&[CONTEXT, b" seal opener"])?,
            seal_joiner: derive(&[CONTEXT, b" seal joiner"])?,
            words_key: derive(&[CONTEXT, b" words key"])?,
            transcript,
            sealed_confirm: false,
            sealed_welcome: false,
            peer: None,
        })
    }
}

/// The keyed channel of one pairing after `CPace`.
pub struct PairChannel {
    role: PairRole,
    sid: [u8; 16],
    transcript: Vec<u8>,
    confirm_opener: Zeroizing<[u8; 32]>,
    confirm_joiner: Zeroizing<[u8; 32]>,
    seal_opener: Zeroizing<[u8; 32]>,
    seal_joiner: Zeroizing<[u8; 32]>,
    words_key: Zeroizing<[u8; 32]>,
    sealed_confirm: bool,
    sealed_welcome: bool,
    /// The peer's identity, once its `Confirm` checked out.
    peer: Option<Identity>,
}

impl PairChannel {
    fn confirm_tag(&self, role: PairRole) -> &[u8; 32] {
        match role {
            PairRole::Opener => &self.confirm_opener,
            PairRole::Joiner => &self.confirm_joiner,
        }
    }

    fn seal_key(&self, role: PairRole) -> &[u8; 32] {
        match role {
            PairRole::Opener => &self.seal_opener,
            PairRole::Joiner => &self.seal_joiner,
        }
    }

    fn nonce(kind: u8, sender: PairRole) -> [u8; 24] {
        let mut nonce = [0; 24];
        nonce[0] = kind;
        nonce[1] = sender.byte();
        nonce
    }

    fn associated_data(&self, kind: u8, sender: PairRole) -> Vec<u8> {
        let mut data = vec![PAIR_MESSAGE_VERSION, kind];
        data.extend_from_slice(CONTEXT);
        data.extend_from_slice(&self.sid);
        data.push(sender.byte());
        data
    }

    fn signed_message(&self, sender: PairRole, identity: &Identity, label: &[u8]) -> Vec<u8> {
        let mut message = Vec::with_capacity(self.transcript.len() + 160);
        message.extend_from_slice(CONTEXT);
        message.extend_from_slice(b" sig");
        message.push(sender.byte());
        message.extend_from_slice(&self.sid);
        message.extend_from_slice(&self.transcript);
        message.extend_from_slice(&identity.to_bytes());
        message.extend_from_slice(label);
        message
    }

    /// This side's `Confirm` message: its tag and its sealed, signed hello. Made once per
    /// pairing.
    ///
    /// # Errors
    /// `Malformed` for an empty label, one over 255 bytes, or a second call.
    pub fn confirm_message(
        &mut self,
        keys: &IdentityKeys,
        label: &str,
    ) -> Result<PairMessage, CryptoError> {
        let label = label.as_bytes();
        let label_len = u8::try_from(label.len()).map_err(|_| CryptoError::Malformed)?;
        if label.is_empty() || self.sealed_confirm {
            return Err(CryptoError::Malformed);
        }
        self.sealed_confirm = true;
        let identity = keys.identity();
        let signature = keys.sign(&self.signed_message(self.role, &identity, label));
        let mut hello = Writer::default();
        hello.raw(&identity.to_bytes());
        hello.u8(label_len);
        hello.raw(label);
        hello.raw(&signature);
        let sealed = aead::seal(
            self.seal_key(self.role),
            &Self::nonce(KIND_CONFIRM, self.role),
            &self.associated_data(KIND_CONFIRM, self.role),
            &hello.bytes,
        )?;
        Ok(PairMessage::Confirm {
            tag: *self.confirm_tag(self.role),
            sealed,
        })
    }

    /// Checks the peer's `Confirm` message: its tag first (a wrong code fails here), then the
    /// seal and the hello's signature. On success the channel keeps the peer's identity for
    /// [`PairChannel::check_words`] and [`PairChannel::welcome_message`].
    ///
    /// # Errors
    /// `Unauthentic` for a wrong tag or a changed seal (a wrong code, a swapped share);
    /// `BadSignature` for a hello not signed by the identity it names; `Malformed` for another
    /// kind of message or a malformed hello.
    pub fn open_confirm(&mut self, message: &PairMessage) -> Result<PairHello, CryptoError> {
        let PairMessage::Confirm { tag, sealed } = message else {
            return Err(CryptoError::Malformed);
        };
        let peer = self.role.peer();
        if !bool::from(tag.ct_eq(self.confirm_tag(peer))) {
            return Err(CryptoError::Unauthentic);
        }
        let hello = aead::open(
            self.seal_key(peer),
            &Self::nonce(KIND_CONFIRM, peer),
            &self.associated_data(KIND_CONFIRM, peer),
            sealed,
        )?;
        let mut reader = Reader::new(&hello);
        let identity = Identity::from_bytes(&reader.array()?);
        let label_len = usize::from(reader.u8()?);
        let label = reader.take(label_len)?;
        let signature = reader.array()?;
        reader.finish()?;
        if label.is_empty() {
            return Err(CryptoError::Malformed);
        }
        verify(
            &identity.member,
            &self.signed_message(peer, &identity, label),
            &signature,
        )?;
        let label = String::from_utf8(label.to_vec()).map_err(|_| CryptoError::Malformed)?;
        self.peer = Some(identity);
        Ok(PairHello { identity, label })
    }

    /// The opener's `Welcome` message naming the group room. Made once per pairing, and only
    /// after the joiner's `Confirm` checked out.
    ///
    /// # Errors
    /// `Malformed` on the joiner's side, before a checked `Confirm` or on a second call.
    pub fn welcome_message(&mut self, room: DocumentId) -> Result<PairMessage, CryptoError> {
        if self.role != PairRole::Opener || self.sealed_welcome || self.peer.is_none() {
            return Err(CryptoError::Malformed);
        }
        self.sealed_welcome = true;
        let sealed = aead::seal(
            self.seal_key(PairRole::Opener),
            &Self::nonce(KIND_WELCOME, PairRole::Opener),
            &self.associated_data(KIND_WELCOME, PairRole::Opener),
            &room.0,
        )?;
        Ok(PairMessage::Welcome { sealed })
    }

    /// Opens the opener's `Welcome` message on the joiner's side.
    ///
    /// # Errors
    /// `Unauthentic` for a changed seal; `Malformed` on the opener's side, for another kind of
    /// message or a wrong length.
    pub fn open_welcome(&self, message: &PairMessage) -> Result<DocumentId, CryptoError> {
        let PairMessage::Welcome { sealed } = message else {
            return Err(CryptoError::Malformed);
        };
        if self.role != PairRole::Joiner {
            return Err(CryptoError::Malformed);
        }
        let room = aead::open(
            self.seal_key(PairRole::Opener),
            &Self::nonce(KIND_WELCOME, PairRole::Opener),
            &self.associated_data(KIND_WELCOME, PairRole::Opener),
            sealed,
        )?;
        let room: [u8; 16] = room.try_into().map_err(|_| CryptoError::Malformed)?;
        Ok(DocumentId(room))
    }

    /// The four check words as indices below 2,048 (11 bits each, up to 44 bits): derived over
    /// the transcript, `own` identity and the peer identity whose `Confirm` checked out, so both
    /// screens show the same words only if both sides hold the same keys and see the same two
    /// identities.
    ///
    /// # Errors
    /// `Malformed` before the peer's `Confirm` checked out.
    pub fn check_words(&self, own: &Identity) -> Result<[u16; 4], CryptoError> {
        let peer = self.peer.as_ref().ok_or(CryptoError::Malformed)?;
        let (joiner, opener) = match self.role {
            PairRole::Joiner => (own, peer),
            PairRole::Opener => (peer, own),
        };
        let mut bits = [0; 8];
        Hkdf::<Sha256>::new(None, self.words_key.as_slice())
            .expand_multi_info(
                &[
                    CONTEXT,
                    b" words",
                    &self.transcript,
                    &joiner.to_bytes(),
                    &opener.to_bytes(),
                ],
                &mut bits[2..],
            )
            .map_err(|_| CryptoError::Malformed)?;
        let value = u64::from_be_bytes(bits);
        let mut indices = [0; 4];
        for (place, index) in indices.iter_mut().enumerate() {
            let shift = 48 - 11 * (place + 1);
            // The mask keeps the value below 2,048, so the conversion never falls back.
            *index = u16::try_from((value >> shift) & 0x7ff).unwrap_or_default();
        }
        Ok(indices)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn channels() -> (PairChannel, PairChannel) {
        let secret = PairSecret::from_entropy([1, 2, 3, 4]);
        let server = "ws://relay.test";
        let joiner =
            PairStart::new(PairRole::Joiner, secret, server, [7; 16], &[0x0b; 32]).unwrap();
        let opener =
            PairStart::new(PairRole::Opener, secret, server, [7; 16], &[0x0a; 32]).unwrap();
        let PairMessage::Share(joiner_share) = joiner.share_message() else {
            unreachable!()
        };
        let PairMessage::Share(opener_share) = opener.share_message() else {
            unreachable!()
        };
        (
            joiner.finish(&opener_share).unwrap(),
            opener.finish(&joiner_share).unwrap(),
        )
    }

    /// A `Welcome` sealed under the joiner's key, nonce or associated data (a reflection) is
    /// refused by the joiner.
    #[test]
    fn a_welcome_sealed_under_the_joiners_role_is_refused() {
        let (joiner, _opener) = channels();
        let room = [0x6e; 16];
        let as_joiner = aead::seal(
            joiner.seal_key(PairRole::Joiner),
            &PairChannel::nonce(KIND_WELCOME, PairRole::Joiner),
            &joiner.associated_data(KIND_WELCOME, PairRole::Joiner),
            &room,
        )
        .unwrap();
        assert_eq!(
            joiner.open_welcome(&PairMessage::Welcome { sealed: as_joiner }),
            Err(CryptoError::Unauthentic)
        );
        let joiner_key_opener_frame = aead::seal(
            joiner.seal_key(PairRole::Joiner),
            &PairChannel::nonce(KIND_WELCOME, PairRole::Opener),
            &joiner.associated_data(KIND_WELCOME, PairRole::Opener),
            &room,
        )
        .unwrap();
        assert_eq!(
            joiner.open_welcome(&PairMessage::Welcome {
                sealed: joiner_key_opener_frame
            }),
            Err(CryptoError::Unauthentic)
        );
    }
}
