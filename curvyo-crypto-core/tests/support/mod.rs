//! Fixed test keys and a small keyring writer for the integration tests (ADR 0008 §1: tests use
//! fixed keys, never an unsealed path).

#![allow(
    dead_code,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::missing_panics_doc
)]

use std::collections::BTreeSet;
use std::path::PathBuf;

use curvyo_crypto_core::keyring::{
    Entry, EntryHash, EpochRef, Keyring, Payload, RecipientWrap, Role,
};
use curvyo_crypto_core::{
    DocumentId, EpochKey, Identity, IdentityKeys, InviteKey, MemberId, wrap_epoch_key,
};

pub const DOC: DocumentId = DocumentId([0xd0; 16]);
pub const OTHER_DOC: DocumentId = DocumentId([0xd1; 16]);

/// Device `n`'s identity keys from fixed entropy.
pub fn device(n: u8) -> IdentityKeys {
    let mut entropy = [0; 64];
    for (i, byte) in entropy.iter_mut().enumerate() {
        *byte = n.wrapping_mul(31).wrapping_add(u8::try_from(i).unwrap());
    }
    IdentityKeys::from_entropy(&entropy)
}

pub fn epoch_key(n: u8) -> EpochKey {
    EpochKey::from_bytes([n.wrapping_add(0x40); 32])
}

pub fn ephemeral(n: u8, salt: u8) -> [u8; 32] {
    [n ^ salt ^ 0x5a; 32]
}

pub fn invite_key(n: u8) -> InviteKey {
    InviteKey::from_seed(&[n.wrapping_add(0x70); 32]).unwrap()
}

pub fn wraps(key: &EpochKey, epoch: u32, to: &[&IdentityKeys], salt: u8) -> Vec<RecipientWrap> {
    to.iter()
        .enumerate()
        .map(|(i, d)| RecipientWrap {
            member: d.identity().member,
            wrap: wrap_epoch_key(
                key,
                epoch,
                DOC,
                &d.identity(),
                ephemeral(u8::try_from(i).unwrap(), salt),
            )
            .unwrap(),
        })
        .collect()
}

pub fn hash(entry: &Entry) -> EntryHash {
    entry.try_hash().unwrap()
}

pub fn id(d: &IdentityKeys) -> MemberId {
    d.identity().member
}

pub fn identity(d: &IdentityKeys) -> Identity {
    d.identity()
}

/// The genesis of `DOC` with `admin` as the only admin and epoch 0 = `epoch_key(0)`.
pub fn genesis(admin: &IdentityKeys) -> Entry {
    Entry::sign(
        Vec::new(),
        admin,
        Payload::Genesis {
            document_id: DOC,
            admins: vec![admin.identity()],
            epoch0_wraps: wraps(&epoch_key(0), 0, &[admin], 0),
        },
    )
    .unwrap()
}

pub fn entry(parents: &[&Entry], author: &IdentityKeys, payload: Payload) -> Entry {
    Entry::sign(parents.iter().map(|e| hash(e)).collect(), author, payload).unwrap()
}

pub fn invite(parents: &[&Entry], admin: &IdentityKeys, n: u8, expires: u64) -> Entry {
    entry(
        parents,
        admin,
        Payload::AddPendingInvite {
            invite_id: invite_key(n).invite_id(),
            role: Role::Member,
            expires,
        },
    )
}

pub fn claim(parents: &[&Entry], joiner: &IdentityKeys, n: u8) -> Entry {
    let key = invite_key(n);
    entry(
        parents,
        joiner,
        Payload::ClaimInvite {
            invite_id: key.invite_id(),
            identity: joiner.identity(),
            proof: key.claim_proof(DOC, &joiner.identity()),
        },
    )
}

pub fn rewrap(
    parents: &[&Entry],
    author: &IdentityKeys,
    epoch: u32,
    key: &EpochKey,
    to: &IdentityKeys,
    salt: u8,
) -> Entry {
    entry(
        parents,
        author,
        Payload::RewrapEpoch {
            epoch,
            member: to.identity().member,
            wrap: wrap_epoch_key(key, epoch, DOC, &to.identity(), ephemeral(9, salt)).unwrap(),
        },
    )
}

pub fn remove(parents: &[&Entry], author: &IdentityKeys, who: &IdentityKeys) -> Entry {
    entry(
        parents,
        author,
        Payload::RemoveMember {
            member: who.identity().member,
        },
    )
}

pub fn new_epoch(
    parents: &[&Entry],
    admin: &IdentityKeys,
    epoch: u32,
    key: &EpochKey,
    to: &[&IdentityKeys],
) -> Entry {
    entry(
        parents,
        admin,
        Payload::NewEpoch {
            epoch,
            wraps: wraps(
                key,
                epoch,
                to,
                u8::try_from(epoch).unwrap().wrapping_mul(17),
            ),
        },
    )
}

pub fn replay(entries: &[&Entry]) -> Keyring {
    let owned: Vec<Entry> = entries.iter().map(|e| (*e).clone()).collect();
    Keyring::replay(&owned).unwrap()
}

pub fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

/// Compares `bytes` with the golden file, or writes it with `CURVYO_UPDATE_GOLDEN=1`.
pub fn golden(name: &str, bytes: &[u8]) {
    let path = fixture(name);
    if std::env::var_os("CURVYO_UPDATE_GOLDEN").is_some() {
        std::fs::write(&path, bytes).unwrap();
    }
    let stored = std::fs::read(&path).unwrap_or_else(|_| panic!("missing golden {name}"));
    assert_eq!(stored, bytes, "golden {name} differs");
}

pub fn hex(text: &str) -> Vec<u8> {
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap())
        .collect()
}

pub fn epoch(number: u32, key: &EpochKey) -> EpochRef {
    EpochRef {
        number,
        key_id: key.key_id(),
    }
}

pub fn accepted_set(keyring: &Keyring, entries: &[&Entry]) -> BTreeSet<EntryHash> {
    entries
        .iter()
        .map(|e| hash(e))
        .filter(|h| keyring.is_accepted(h))
        .collect()
}

/// Every order of `entries` replays to the same state, and in every ancestor-closed subset
/// each entry is accepted exactly when it is accepted in the whole set (validity depends on an
/// entry's ancestors alone).
pub fn check_exhaustively(entries: &[&Entry]) {
    let full = replay(entries);
    let accepted = accepted_set(&full, entries);
    let n = entries.len();
    // Orders: every permutation up to 5 entries, otherwise every rotation and its reverse
    // (the replay deduplicates and sorts by hash before anything else, so order cannot matter;
    // this guards that).
    let mut orders: Vec<Vec<usize>> = Vec::new();
    if n <= 5 {
        permutations(&mut (0..n).collect(), 0, &mut orders);
    } else {
        for shift in 0..n {
            let rotated: Vec<usize> = (0..n).map(|i| (i + shift) % n).collect();
            orders.push(rotated.iter().rev().copied().collect());
            orders.push(rotated);
        }
    }
    for order in orders {
        let shuffled: Vec<&Entry> = order.iter().map(|&i| entries[i]).collect();
        let again = replay(&shuffled);
        assert_eq!(accepted_set(&again, entries), accepted);
        assert_eq!(again.participants(), full.participants());
        assert_eq!(again.current_epoch(), full.current_epoch());
        assert_eq!(again.heads(), full.heads());
    }
    // Ancestor-closed subsets.
    let hashes: Vec<EntryHash> = entries.iter().map(|e| hash(e)).collect();
    assert!(n <= 14, "subset enumeration would be too large");
    for mask in 0u32..(1 << n) {
        let chosen: Vec<usize> = (0..n).filter(|i| mask & (1 << i) != 0).collect();
        let present: BTreeSet<EntryHash> = chosen.iter().map(|&i| hashes[i]).collect();
        let closed = chosen
            .iter()
            .all(|&i| entries[i].parents.iter().all(|p| present.contains(p)));
        if !closed {
            continue;
        }
        let subset: Vec<Entry> = chosen.iter().map(|&i| entries[i].clone()).collect();
        let Ok(partial) = Keyring::replay(&subset) else {
            continue;
        };
        for &i in &chosen {
            assert_eq!(
                partial.is_accepted(&hashes[i]),
                accepted.contains(&hashes[i]),
                "entry {i} changes validity in subset {mask:b}"
            );
        }
    }
}

fn permutations(items: &mut Vec<usize>, k: usize, out: &mut Vec<Vec<usize>>) {
    if k == items.len() {
        out.push(items.clone());
        return;
    }
    for i in k..items.len() {
        items.swap(k, i);
        permutations(items, k + 1, out);
        items.swap(k, i);
    }
}
