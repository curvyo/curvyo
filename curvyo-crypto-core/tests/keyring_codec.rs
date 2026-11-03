//! The keyring entry codec and the `keyring.log` format (ADR 0010 §2, §3; golden file).

#![allow(clippy::unwrap_used)]

mod support;

use curvyo_crypto_core::CryptoError;
use curvyo_crypto_core::keyring::{
    Entry, EpochWrap, Keyring, Payload, Role, decode_log, encode_log,
};
use support::{
    DOC, claim, device, epoch_key, genesis, golden, hash, invite, new_epoch, remove, rewrap, wraps,
};

/// One entry of each of the eight payloads, all well-signed.
fn every_payload() -> Vec<Entry> {
    let (a, b, c) = (device(1), device(2), device(3));
    let g = genesis(&a);
    let add = support::entry(
        &[&g],
        &a,
        Payload::AddMember {
            identity: b.identity(),
            role: Role::Admin,
            wraps: vec![EpochWrap {
                epoch: 0,
                wrap: wraps(&epoch_key(0), 0, &[&b], 3)[0].wrap,
            }],
        },
    );
    let set_role = support::entry(
        &[&add],
        &a,
        Payload::SetRole {
            member: b.identity().member,
            role: Role::Member,
        },
    );
    let inv = invite(&[&set_role], &a, 1, 1_000);
    let cl = claim(&[&inv], &c, 1);
    let rw = rewrap(&[&cl], &a, 0, &epoch_key(0), &c, 1);
    let rm = remove(&[&rw], &a, &b);
    let ne = new_epoch(&[&rm], &a, 1, &epoch_key(1), &[&a, &c]);
    vec![g, add, set_role, inv, cl, rw, rm, ne]
}

#[test]
fn every_payload_round_trips_and_keeps_its_hash_and_signature() {
    for entry in every_payload() {
        let bytes = entry.encode().unwrap();
        let decoded = Entry::decode(&bytes).unwrap();
        assert_eq!(decoded, entry);
        assert_eq!(hash(&decoded), hash(&entry));
        decoded.verify_signature().unwrap();
    }
}

#[test]
fn every_payload_is_accepted_in_its_chain() {
    let entries = every_payload();
    let keyring = Keyring::replay(&entries).unwrap();
    assert_eq!(keyring.rejected(), vec![]);
    assert_eq!(keyring.document_id(), DOC);
}

#[test]
fn malformed_entries_are_refused() {
    let entry = &every_payload()[1];
    let bytes = entry.encode().unwrap();
    assert_eq!(
        Entry::decode(&bytes[..bytes.len() - 1]),
        Err(CryptoError::Malformed)
    );
    let mut trailing = bytes.clone();
    trailing.push(0);
    assert_eq!(Entry::decode(&trailing), Err(CryptoError::Malformed));
    let mut version = bytes.clone();
    version[0] = 9;
    assert_eq!(Entry::decode(&version), Err(CryptoError::UnknownVersion(9)));
    // The payload tag sits after version, count, one parent and the author.
    let mut tag = bytes;
    tag[1 + 2 + 32 + 32] = 99;
    assert_eq!(Entry::decode(&tag), Err(CryptoError::Malformed));
}

#[test]
fn unsorted_parents_are_refused() {
    let entries = every_payload();
    let mut entry = entries[2].clone();
    entry.parents = vec![hash(&entries[1]), hash(&entries[0])];
    entry.parents.sort();
    entry.parents.reverse();
    assert_eq!(
        Entry::decode(&entry.encode().unwrap()),
        Err(CryptoError::Malformed)
    );
}

/// The golden `keyring.log`: the join and the removal of slice 1 plus the two payloads it does
/// not write yet. A change here is a format change.
#[test]
fn the_golden_keyring_log_is_stable_and_replays() {
    let entries = every_payload();
    let bytes = encode_log(&entries).unwrap();
    golden("keyring_v1.log", &bytes);
    let decoded = decode_log(&bytes).unwrap();
    assert_eq!(decoded, entries);
    assert_eq!(Keyring::replay(&decoded).unwrap().rejected(), vec![]);
}

#[test]
fn malformed_logs_are_refused() {
    let bytes = encode_log(&every_payload()).unwrap();
    assert_eq!(decode_log(b"XXXX\x01"), Err(CryptoError::Malformed));
    assert_eq!(decode_log(b"CYKR\x02"), Err(CryptoError::UnknownVersion(2)));
    assert_eq!(
        decode_log(&bytes[..bytes.len() - 3]),
        Err(CryptoError::Malformed)
    );
    assert_eq!(decode_log(b"CYKR\x01").unwrap(), Vec::new());
}
