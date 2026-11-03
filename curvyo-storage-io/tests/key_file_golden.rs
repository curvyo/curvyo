//! Golden files of the encrypted key file and of a room-key value (ADR 0031 decision 12): a key
//! file written by format 1 opens with its passphrase in every later build.

#![allow(clippy::unwrap_used)]
use curvyo_storage_io::{
    CredentialError, CredentialStore, KEY_FILE, decode_room_keys, encode_room_keys,
};

const KEY_FILE_V1: &[u8] = include_bytes!("fixtures/device_keys_v1.bin");
const ROOM_KEYS_V1: &[u8] = include_bytes!("fixtures/room_keys_v1.bin");
const PASSPHRASE: &str = "correct horse battery staple";

/// `CVYKEYS1`, `format = 1`, Argon2 `version = 0x13`, `m = 19456 KiB`, `t = 2`, `p = 1`.
const HEADER_V1: [u8; 25] = [
    b'C', b'V', b'Y', b'K', b'E', b'Y', b'S', b'1', 1, 0x13, 0, 0, 0, 0x00, 0x4c, 0, 0, 2, 0, 0, 0,
    1, 0, 0, 0,
];

fn copy_of_the_golden_key_file() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join(KEY_FILE), KEY_FILE_V1).unwrap();
    dir
}

#[test]
fn the_golden_key_file_header_names_format_and_argon2id_parameters() {
    assert_eq!(&KEY_FILE_V1[..HEADER_V1.len()], &HEADER_V1);
}

#[test]
fn the_golden_key_file_opens_with_its_passphrase() {
    let dir = copy_of_the_golden_key_file();
    let store = CredentialStore::file(dir.path(), PASSPHRASE).unwrap();
    assert_eq!(
        store.get("device-identity").unwrap().unwrap().as_slice(),
        &[7; 64]
    );
    let room = store.get("room-ab").unwrap().unwrap();
    let keys = decode_room_keys(&room).unwrap();
    assert_eq!(keys.len(), 1);
    assert_eq!((keys[0].0, *keys[0].1), (3, [9; 32]));
    assert!(matches!(
        CredentialStore::file(dir.path(), "wrong"),
        Err(CredentialError::WrongPassphrase)
    ));
}

#[test]
fn a_key_file_of_another_format_or_parameters_is_damaged() {
    for (at, value) in [(8, 2), (9, 0x10), (16, 0xff), (17, 0)] {
        let dir = copy_of_the_golden_key_file();
        let mut bytes = KEY_FILE_V1.to_vec();
        bytes[at] = value;
        std::fs::write(dir.path().join(KEY_FILE), &bytes).unwrap();
        assert!(
            matches!(
                CredentialStore::file(dir.path(), PASSPHRASE),
                Err(CredentialError::Damaged)
            ),
            "byte {at}"
        );
    }
}

#[test]
fn a_new_key_file_writes_the_same_header() {
    let dir = tempfile::tempdir().unwrap();
    CredentialStore::file(dir.path(), PASSPHRASE).unwrap();
    let bytes = std::fs::read(dir.path().join(KEY_FILE)).unwrap();
    assert_eq!(&bytes[..HEADER_V1.len()], &HEADER_V1);
}

#[test]
fn the_golden_room_key_value_decodes_and_encodes_byte_for_byte() {
    let keys = decode_room_keys(ROOM_KEYS_V1).unwrap();
    let plain: Vec<(u32, [u8; 32])> = keys.iter().map(|(e, k)| (*e, **k)).collect();
    assert_eq!(plain, vec![(3, [9; 32]), (4, [0xa5; 32])]);
    assert_eq!(encode_room_keys(&plain).as_slice(), ROOM_KEYS_V1);
}
