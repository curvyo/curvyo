//! The passkey backup record v1 (ADR 0040 decision 13, Frage 145 A): the golden record made from
//! a fixed PRF output, the derived id and write key, and refusals of a changed byte, another
//! passkey and a newer version.

#![allow(clippy::unwrap_used, clippy::expect_used)]

mod support;

use curvyo_crypto_core::{
    BACKUP_RECORD_VERSION, BackupContent, BackupKeys, CryptoError, DocumentId, backup_prf_input,
};
use support::{device, golden};
use zeroize::Zeroizing;

const PRF: [u8; 32] = [0x42; 32];

fn content() -> BackupContent {
    BackupContent {
        identity: device(5).to_entropy(),
        group_room: DocumentId([0x6e; 16]),
        server: "wss://sync.example.org".to_owned(),
        label: "Firefox on Linux".to_owned(),
    }
}

/// The golden backup record v1 and the derived id; the format and the derivation are permanent.
#[test]
fn golden_backup_record_is_stable() {
    let keys = BackupKeys::derive(&PRF);
    let record = keys.seal(&content(), &[0x24; 24]).unwrap();
    golden("backup_record_v1.bin", &record);
    let mut derived = keys.id().0.to_vec();
    derived.extend_from_slice(&keys.write_key().0);
    derived.extend_from_slice(&backup_prf_input());
    golden("backup_keys_v1.bin", &derived);
    assert_eq!(keys.open(&record).unwrap(), content());
}

#[test]
fn a_changed_record_another_passkey_or_a_newer_version_does_not_open() {
    let keys = BackupKeys::derive(&PRF);
    let record = keys.seal(&content(), &[0x24; 24]).unwrap();
    for at in [1, 30, record.len() - 1] {
        let mut changed = record.clone();
        changed[at] ^= 1;
        assert_eq!(keys.open(&changed), Err(CryptoError::Unauthentic));
    }
    assert_eq!(
        BackupKeys::derive(&[0x43; 32]).open(&record),
        Err(CryptoError::Unauthentic)
    );
    let mut newer = record.clone();
    newer[0] = BACKUP_RECORD_VERSION + 1;
    assert_eq!(
        keys.open(&newer),
        Err(CryptoError::UnknownVersion(BACKUP_RECORD_VERSION + 1))
    );
    assert_eq!(keys.open(&record[..10]), Err(CryptoError::Malformed));
}

#[test]
fn the_record_holds_no_plaintext() {
    let keys = BackupKeys::derive(&PRF);
    let record = keys.seal(&content(), &[0x24; 24]).unwrap();
    let identity: Zeroizing<[u8; 64]> = device(5).to_entropy();
    for needle in [
        &identity[..32],
        &identity[32..],
        &[0x6e; 16][..],
        b"sync.example.org",
        b"Firefox",
        &PRF[..],
    ] {
        assert!(!record.windows(needle.len()).any(|w| w == needle));
    }
}
