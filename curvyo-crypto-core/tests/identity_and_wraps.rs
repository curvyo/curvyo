//! Identity keys, fingerprints, epoch key ids, wraps and invite proofs (`specs/0092`
//! criteria 1 to 3, 27, 33; ADR 0010 §1, §6, ADR 0031 decisions 6 and 8).

#![allow(clippy::unwrap_used)]

mod support;

use curvyo_crypto_core::{
    CryptoError, EpochKey, IdentityKeys, InviteKey, MemberId, unwrap_epoch_key, verify,
    verify_claim_proof, wrap_epoch_key,
};
use support::{DOC, OTHER_DOC, device, epoch_key, hex};

fn seed_entropy(seed_hex: &str) -> [u8; 64] {
    let mut entropy = [0; 64];
    entropy[..32].copy_from_slice(&hex(seed_hex));
    entropy
}

/// RFC 8032 §7.1, tests 1 and 2.
#[test]
fn ed25519_identity_keys_match_rfc_8032() {
    let cases = [
        (
            "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60",
            "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a",
            "",
            "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b",
        ),
        (
            "4ccd089b28ff96da9db6c346ec114e0f5b8a319f35aba624da8cf6ed4fb8a6fb",
            "3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c",
            "72",
            "92a009a9f0d4cab8720e820b5f642540a2b27b5416503f8fb3762223ebdb69da085ac1e43e15996e458f3613d0f11d8c387b2eaeb4302aeeb00d291612bb0c00",
        ),
    ];
    for (secret, public, message, signature) in cases {
        let keys = IdentityKeys::from_entropy(&seed_entropy(secret));
        let member = keys.identity().member;
        assert_eq!(member.0.to_vec(), hex(public));
        let message = hex(message);
        let signed = keys.sign(&message);
        assert_eq!(signed.to_vec(), hex(signature));
        verify(&member, &message, &signed).unwrap();
        let mut forged = signed;
        forged[0] ^= 1;
        assert_eq!(
            verify(&member, &message, &forged),
            Err(CryptoError::BadSignature)
        );
    }
}

/// The X25519 half of an identity is RFC 7748's public key of its secret (§6.1, Alice).
#[test]
fn x25519_identity_key_matches_rfc_7748() {
    let mut entropy = [0; 64];
    entropy[32..].copy_from_slice(&hex(
        "77076d0a7318a57d3c16c17251b26645df4c2f87ebc0992ab177fba51db92c2a",
    ));
    let identity = IdentityKeys::from_entropy(&entropy).identity();
    assert_eq!(
        identity.agreement.to_vec(),
        hex("8520f0098930a754748b7ddcb43ef75a0dbf3a0d26381af4eba4a98eaa9b4e6a")
    );
}

#[test]
fn identity_keys_round_trip_through_their_entropy() {
    let keys = device(3);
    let again = IdentityKeys::from_entropy(&keys.to_entropy());
    assert_eq!(again.identity(), keys.identity());
}

/// Criterion 1 (c): every participant shows a fingerprint: 16 base32 symbols in groups of four.
#[test]
fn fingerprints_are_four_groups_of_four_base32_symbols() {
    let a = device(1).identity().fingerprint();
    let b = device(2).identity().fingerprint();
    assert_ne!(a, b);
    assert_eq!(a, device(1).identity().fingerprint());
    let groups: Vec<&str> = a.split(' ').collect();
    assert_eq!(groups.len(), 4, "{a}");
    assert!(groups.iter().all(|g| {
        g.len() == 4
            && g.chars()
                .all(|c| c.is_ascii_uppercase() || ('2'..='7').contains(&c))
    }));
}

/// ADR 0031 decision 6: `key_id` tells two keys of the same epoch number apart.
#[test]
fn key_ids_differ_per_key_and_are_stable() {
    assert_eq!(epoch_key(1).key_id(), epoch_key(1).key_id());
    assert_ne!(epoch_key(1).key_id(), epoch_key(2).key_id());
}

/// Criteria 3 and 33: a wrap opens only for its recipient, document, epoch and key.
#[test]
fn a_wrap_opens_only_for_its_recipient_document_and_epoch() {
    let (a, b) = (device(1), device(2));
    let key = epoch_key(5);
    let wrap = wrap_epoch_key(&key, 2, DOC, &b.identity(), [9; 32]).unwrap();
    assert_eq!(wrap.key_id(), key.key_id());
    assert_eq!(unwrap_epoch_key(&wrap, 2, DOC, &b).unwrap(), key);
    assert_eq!(
        unwrap_epoch_key(&wrap, 2, DOC, &a),
        Err(CryptoError::Unauthentic)
    );
    assert_eq!(
        unwrap_epoch_key(&wrap, 3, DOC, &b),
        Err(CryptoError::Unauthentic)
    );
    assert_eq!(
        unwrap_epoch_key(&wrap, 2, OTHER_DOC, &b),
        Err(CryptoError::Unauthentic)
    );
    let mut moved = wrap;
    moved.0[..8].copy_from_slice(&epoch_key(6).key_id().0);
    assert_eq!(
        unwrap_epoch_key(&moved, 2, DOC, &b),
        Err(CryptoError::Unauthentic)
    );
    for index in 8..wrap.0.len() {
        let mut flipped = wrap;
        flipped.0[index] ^= 0x01;
        assert!(
            unwrap_epoch_key(&flipped, 2, DOC, &b).is_err(),
            "byte {index}"
        );
    }
}

#[test]
fn a_wrap_to_a_low_order_key_is_refused() {
    let mut victim = device(2).identity();
    victim.agreement = [0; 32];
    assert_eq!(
        wrap_epoch_key(&EpochKey::from_bytes([1; 32]), 0, DOC, &victim, [9; 32]),
        Err(CryptoError::BadPublicKey)
    );
}

/// Criterion 2: the link's seed derives the invite key; only its public half is in the
/// keyring, and the claim proof binds document, invite and joiner.
#[test]
fn an_invite_proof_binds_document_invite_and_joiner() {
    let key = InviteKey::from_seed(&[7; 32]).unwrap();
    assert_eq!(
        key.invite_id(),
        InviteKey::from_seed(&[7; 32]).unwrap().invite_id()
    );
    assert_ne!(key.invite_id().0, [7; 32]);
    let joiner = device(2).identity();
    let proof = key.claim_proof(DOC, &joiner);
    verify_claim_proof(key.invite_id(), DOC, &joiner, &proof).unwrap();
    assert!(verify_claim_proof(key.invite_id(), OTHER_DOC, &joiner, &proof).is_err());
    assert!(verify_claim_proof(key.invite_id(), DOC, &device(3).identity(), &proof).is_err());
    let other = InviteKey::from_seed(&[8; 32]).unwrap();
    assert!(verify_claim_proof(other.invite_id(), DOC, &joiner, &proof).is_err());
}

#[test]
fn a_verification_under_an_invalid_point_is_refused() {
    let member = MemberId([0xff; 32]);
    assert_eq!(
        verify(&member, b"x", &[0; 64]),
        Err(CryptoError::BadSignature)
    );
}
