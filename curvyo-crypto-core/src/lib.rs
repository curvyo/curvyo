//! The sealing and keyring layer of cooperative editing (ADR 0008, ADR 0010, ADR 0031): every
//! byte that leaves a device for the relay passes through here.
//!
//! Pure and wasm-compatible (`CLAUDE.md` §6): no randomness, clock, filesystem or network.
//! Callers pass in every byte of entropy (identity keys, epoch keys, wrap ephemerals, invite
//! seeds, the nonce seed) and the current time where an expiry is checked. There is no
//! unsealed path and no flag that disables sealing (ADR 0008 §1).

#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod aead;
mod agreement;
mod codec;
mod envelope;
mod error;
mod identity;
mod invite;
pub mod keyring;
mod keys;
mod nonce;
mod wrap;

pub use envelope::{
    ENVELOPE_VERSION, EnvelopeHeader, EnvelopeKind, HEADER_LEN, NONCE_LEN, Opened, Sealing, open,
    seal,
};
pub use error::CryptoError;
pub use identity::{Identity, IdentityKeys, MemberId, verify};
pub use invite::{InviteId, InviteKey, verify_claim_proof};
pub use keys::{DocumentId, EpochKey, KeyId};
pub use nonce::NonceStream;
pub use wrap::{WRAP_LEN, Wrap, unwrap_epoch_key, wrap_epoch_key};
