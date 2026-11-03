//! Fixed test identities and the network simulator shared by the integration tests (ADR 0008
//! §1: tests use fixed keys and seeds, never an unsealed path).

#![allow(
    dead_code,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::missing_panics_doc
)]

pub mod sim;

use curvyo_crypto_core::{DocumentId, IdentityKeys, MemberId};
use curvyo_sync_core::RoomId;

/// The room (and document) of most tests.
pub const ROOM: RoomId = RoomId([0xd0; 16]);
/// A second room of the same maker (criterion 40).
pub const OTHER_ROOM: RoomId = RoomId([0xd1; 16]);

/// Device `n`'s identity keys from fixed entropy.
pub fn device(n: u8) -> IdentityKeys {
    let mut entropy = [0; 64];
    for (i, byte) in entropy.iter_mut().enumerate() {
        *byte = n.wrapping_mul(31).wrapping_add(u8::try_from(i).unwrap());
    }
    IdentityKeys::from_entropy(&entropy)
}

pub fn member(n: u8) -> MemberId {
    device(n).identity().member
}

pub fn doc_id(room: RoomId) -> DocumentId {
    room.document_id()
}

/// A fixed 32-byte seed, distinct per `(n, salt)`.
pub fn seed(n: u8, salt: u8) -> [u8; 32] {
    let mut out = [0; 32];
    for (i, byte) in out.iter_mut().enumerate() {
        *byte = n
            .wrapping_mul(97)
            .wrapping_add(salt.wrapping_mul(13))
            .wrapping_add(u8::try_from(i).unwrap());
    }
    out
}
