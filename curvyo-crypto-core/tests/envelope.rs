//! The sealed envelope and the nonce stream (`specs/0092` criteria 24, 25, 27, 28, 33;
//! ADR 0008 §1 to §3, ADR 0031 decision 6).

#![allow(clippy::unwrap_used)]

mod support;

use curvyo_crypto_core::{
    CryptoError, DocumentId, EnvelopeHeader, EnvelopeKind, EpochKey, HEADER_LEN, NONCE_LEN,
    NonceStream, Sealing, open, seal,
};
use proptest::prelude::*;
use support::{DOC, OTHER_DOC, device, epoch_key, golden, hex};

const KINDS: [EnvelopeKind; 4] = [
    EnvelopeKind::Update,
    EnvelopeKind::Asset,
    EnvelopeKind::Snapshot,
    EnvelopeKind::Presence,
];

/// The plaintext of the golden envelopes: the marker and colour of criterion 24.
const MARKER: &[u8] = br##"{"text":"ZQX-MARKER-7731","stroke":"#13a7c9"}"##;

fn sealed(kind: EnvelopeKind, key: &EpochKey, epoch: u32, plaintext: &[u8]) -> Vec<u8> {
    let signer = device(1);
    seal(
        &Sealing {
            kind,
            epoch,
            key,
            document_id: DOC,
            sender_peer: 0x1122_3344_5566_7788,
            signer: (kind == EnvelopeKind::Snapshot).then_some(&signer),
        },
        NonceStream::nonce_at(&[3; 32], u64::from(epoch)),
        plaintext,
    )
    .unwrap()
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

/// RFC 8439 §A.1, test vector 1: the all-zero key and IV give this keystream, so nonce 0 of the
/// all-zero seed is its first 24 bytes.
#[test]
fn the_nonce_stream_is_the_chacha20_keystream() {
    assert_eq!(
        NonceStream::nonce_at(&[0; 32], 0).to_vec(),
        hex("76b8e0ada0f13d90405d6ae55386bd28bdd219b8a08ded1a")
    );
    let mut stream = NonceStream::new([5; 32]);
    let first = stream.next_nonce().unwrap();
    let second = stream.next_nonce().unwrap();
    assert_eq!(first, NonceStream::nonce_at(&[5; 32], 0));
    assert_eq!(second, NonceStream::nonce_at(&[5; 32], 1));
    assert_ne!(first, second);
    assert_ne!(NonceStream::nonce_at(&[6; 32], 0), first);
}

#[test]
fn every_kind_round_trips_and_carries_its_header() {
    let key = epoch_key(1);
    for kind in KINDS {
        let envelope = sealed(kind, &key, 4, MARKER);
        let header = EnvelopeHeader::parse(&envelope).unwrap();
        assert_eq!(header.kind, kind);
        assert_eq!(header.epoch, 4);
        assert_eq!(header.key_id, key.key_id());
        assert_eq!(header.document_id, DOC);
        assert_eq!(header.sender_peer, 0x1122_3344_5566_7788);
        assert_eq!(header.signed, kind == EnvelopeKind::Snapshot);
        let opened = open(&envelope, &key, DOC, kind).unwrap();
        assert_eq!(opened.plaintext, MARKER);
        assert_eq!(
            opened.signer,
            (kind == EnvelopeKind::Snapshot).then(|| device(1).identity().member)
        );
    }
}

/// Criterion 24: neither the marker nor the colour appear in a sealed envelope.
#[test]
fn a_sealed_envelope_contains_no_plaintext() {
    for kind in KINDS {
        let envelope = sealed(kind, &epoch_key(1), 0, MARKER);
        assert!(!contains(&envelope, b"ZQX-MARKER-7731"));
        assert!(!contains(&envelope, b"13a7c9"));
    }
}

/// ADR 0010 §11: a snapshot is always signed.
#[test]
fn a_snapshot_without_a_signer_is_refused() {
    let key = epoch_key(1);
    let unsigned = Sealing {
        kind: EnvelopeKind::Snapshot,
        epoch: 0,
        key: &key,
        document_id: DOC,
        sender_peer: 1,
        signer: None,
    };
    assert_eq!(seal(&unsigned, [0; 24], b"x"), Err(CryptoError::Unsigned));
    let mut update = sealed(EnvelopeKind::Update, &key, 0, b"x");
    update[1] = 3;
    assert_eq!(
        open(&update, &key, DOC, EnvelopeKind::Snapshot),
        Err(CryptoError::Unsigned)
    );
}

/// Criteria 25, 27 and 33: another document, kind or key is refused before decrypting.
#[test]
fn a_mismatched_header_is_refused() {
    let key = epoch_key(1);
    let envelope = sealed(EnvelopeKind::Update, &key, 0, MARKER);
    assert_eq!(
        open(&envelope, &key, OTHER_DOC, EnvelopeKind::Update),
        Err(CryptoError::WrongDocument)
    );
    assert_eq!(
        open(&envelope, &key, DOC, EnvelopeKind::Presence),
        Err(CryptoError::WrongKind)
    );
    // An uninvited device, or a removed one holding only an older key, cannot open it.
    assert_eq!(
        open(&envelope, &epoch_key(2), DOC, EnvelopeKind::Update),
        Err(CryptoError::WrongKey)
    );
}

#[test]
fn malformed_and_unknown_headers_are_refused() {
    let envelope = sealed(EnvelopeKind::Update, &epoch_key(1), 0, b"");
    assert_eq!(
        EnvelopeHeader::parse(&envelope[..HEADER_LEN + NONCE_LEN + 15]),
        Err(CryptoError::Malformed)
    );
    let mut version = envelope.clone();
    version[0] = 2;
    assert_eq!(
        EnvelopeHeader::parse(&version),
        Err(CryptoError::UnknownVersion(2))
    );
    let mut kind = envelope.clone();
    kind[1] = 9;
    assert_eq!(EnvelopeHeader::parse(&kind), Err(CryptoError::Malformed));
    let mut flags = envelope;
    flags[2] = 2;
    assert_eq!(EnvelopeHeader::parse(&flags), Err(CryptoError::Malformed));
}

/// The golden envelopes: one per kind, sealed from fixed keys, nonces and the plaintext of
/// criterion 24. The format is permanent; a change here is a format change.
#[test]
fn golden_envelopes_are_stable_and_open() {
    let key = epoch_key(1);
    for (kind, name) in [
        (EnvelopeKind::Update, "envelope_update_v1.bin"),
        (EnvelopeKind::Asset, "envelope_asset_v1.bin"),
        (EnvelopeKind::Snapshot, "envelope_snapshot_signed_v1.bin"),
        (EnvelopeKind::Presence, "envelope_presence_v1.bin"),
    ] {
        let envelope = sealed(kind, &key, 1, MARKER);
        golden(name, &envelope);
        assert_eq!(open(&envelope, &key, DOC, kind).unwrap().plaintext, MARKER);
    }
}

/// Criterion 28: the crate has no switch of any kind (environment, feature, flag) and so no
/// way to disable sealing.
#[test]
fn the_crate_has_no_switch_that_could_disable_sealing() {
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut stack = vec![src];
    while let Some(dir) = stack.pop() {
        for item in std::fs::read_dir(dir).unwrap() {
            let path = item.unwrap().path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            let text = std::fs::read_to_string(&path).unwrap();
            for forbidden in [
                "std::env",
                "env!(",
                "option_env!",
                "cfg(feature",
                "plaintext_mode",
            ] {
                assert!(
                    !text.contains(forbidden),
                    "{} uses {forbidden}",
                    path.display()
                );
            }
        }
    }
    let manifest = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"),
    )
    .unwrap();
    assert!(!manifest.contains("[features]"));
}

/// A changed byte under the AEAD (or, for a signed snapshot, under the signature first).
fn refused_as_unauthentic(result: &Result<curvyo_crypto_core::Opened, CryptoError>) -> bool {
    matches!(
        result,
        Err(CryptoError::Unauthentic | CryptoError::BadSignature)
    )
}

fn arbitrary_kind() -> impl Strategy<Value = EnvelopeKind> {
    prop::sample::select(KINDS.to_vec())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// Criterion 25: any flipped byte is refused, never opened to other plaintext.
    #[test]
    fn any_flipped_byte_is_refused(
        kind in arbitrary_kind(),
        plaintext in prop::collection::vec(any::<u8>(), 0..300),
        position in any::<prop::sample::Index>(),
        bit in 0u8..8,
    ) {
        let key = epoch_key(1);
        let mut envelope = sealed(kind, &key, 7, &plaintext);
        let index = position.index(envelope.len());
        envelope[index] ^= 1 << bit;
        prop_assert!(open(&envelope, &key, DOC, kind).is_err());
    }

    /// Criterion 25: a record moved into another room (document), epoch, `key_id` or kind by
    /// rewriting its cleartext header is refused.
    #[test]
    fn a_rewritten_header_is_refused(
        kind in arbitrary_kind(),
        plaintext in prop::collection::vec(any::<u8>(), 0..100),
        other_epoch in any::<u32>(),
        other_room in any::<[u8; 16]>(),
        other_kind in arbitrary_kind(),
        other_key in any::<[u8; 32]>(),
    ) {
        let key = epoch_key(1);
        let envelope = sealed(kind, &key, 7, &plaintext);
        // Epoch: bytes 3..7.
        if other_epoch != 7 {
            let mut moved = envelope.clone();
            moved[3..7].copy_from_slice(&other_epoch.to_le_bytes());
            prop_assert!(refused_as_unauthentic(&open(&moved, &key, DOC, kind)));
        }
        // Room: bytes 15..31; opened as the other room, and as this one.
        if DocumentId(other_room) != DOC {
            let mut moved = envelope.clone();
            moved[15..31].copy_from_slice(&other_room);
            prop_assert!(refused_as_unauthentic(&open(&moved, &key, DocumentId(other_room), kind)));
            prop_assert_eq!(open(&moved, &key, DOC, kind), Err(CryptoError::WrongDocument));
        }
        // Key id: bytes 7..15, opened with the key it now names.
        let other = EpochKey::from_bytes(other_key);
        if other.key_id() != key.key_id() {
            let mut moved = envelope.clone();
            moved[7..15].copy_from_slice(&other.key_id().0);
            prop_assert!(open(&moved, &other, DOC, kind).is_err());
        }
        // Kind: byte 1.
        if other_kind != kind {
            let mut moved = envelope.clone();
            moved[1] = match other_kind {
                EnvelopeKind::Update => 1,
                EnvelopeKind::Asset => 2,
                EnvelopeKind::Snapshot => 3,
                EnvelopeKind::Presence => 4,
            };
            prop_assert!(open(&moved, &key, DOC, other_kind).is_err());
        }
    }
}
