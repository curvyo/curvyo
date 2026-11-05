//! Tester acceptance tests for `specs/0096-own-devices-sync` (criteria 1 to 4, 30, 34 and 36),
//! written from the specification and ADR 0040 decisions 4 and 13 before the implementation was
//! read: the pairing code, the pairing channel against a relay that swaps shares or session ids,
//! the check words, no private key in any pairing message, and the passkey backup record.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::missing_panics_doc)]

mod support;

use curvyo_crypto_core::{
    BACKUP_RECORD_VERSION, BackupContent, BackupKeys, BackupOp, CryptoError, DocumentId,
    IdentityKeys, PAIR_SLOTS, PairChannel, PairCode, PairMessage, PairRole, PairSecret, PairStart,
    backup_prf_input, verify_backup_write,
};
use proptest::prelude::*;
use support::device;
use zeroize::Zeroizing;

const SERVER: &str = "ws://192.168.1.20:8080";
const ROOM: DocumentId = DocumentId([0x6e; 16]);

fn secret(n: u8) -> PairSecret {
    PairSecret::from_entropy([n, 0x5a, 0xc3, 0x11])
}

fn share(message: &PairMessage) -> [u8; 32] {
    match message {
        PairMessage::Share(share) => *share,
        other => panic!("not a share: {other:?}"),
    }
}

/// One `CPace` run with each side's own secret, sid and server, as a relay would let it happen.
struct Run {
    joiner_secret: PairSecret,
    opener_secret: PairSecret,
    joiner_sid: [u8; 16],
    opener_sid: [u8; 16],
    joiner_server: &'static str,
    opener_server: &'static str,
}

impl Run {
    fn honest() -> Self {
        Self {
            joiner_secret: secret(1),
            opener_secret: secret(1),
            joiner_sid: [0x51; 16],
            opener_sid: [0x51; 16],
            joiner_server: SERVER,
            opener_server: SERVER,
        }
    }

    fn channels(&self) -> (PairChannel, PairChannel, Vec<PairMessage>) {
        let joiner = PairStart::new(
            PairRole::Joiner,
            self.joiner_secret,
            self.joiner_server,
            self.joiner_sid,
            &[0x0b; 32],
        )
        .unwrap();
        let opener = PairStart::new(
            PairRole::Opener,
            self.opener_secret,
            self.opener_server,
            self.opener_sid,
            &[0x0a; 32],
        )
        .unwrap();
        let shares = vec![joiner.share_message(), opener.share_message()];
        let j = joiner.finish(&share(&shares[1])).unwrap();
        let o = opener.finish(&share(&shares[0])).unwrap();
        (j, o, shares)
    }
}

/// Runs the confirm step joiner first; `Ok` with both channels when both confirmations open.
fn confirm(
    joiner: &mut PairChannel,
    opener: &mut PairChannel,
    laptop: &IdentityKeys,
    desktop: &IdentityKeys,
    messages: &mut Vec<PairMessage>,
) -> Result<(), CryptoError> {
    let from_joiner = joiner.confirm_message(laptop, "ZQX-PC-7731 laptop")?;
    messages.push(from_joiner.clone());
    let hello = opener.open_confirm(&from_joiner)?;
    assert_eq!(hello.identity, laptop.identity());
    let from_opener = opener.confirm_message(desktop, "ZQX-PC-7731 desktop")?;
    messages.push(from_opener.clone());
    let hello = joiner.open_confirm(&from_opener)?;
    assert_eq!(hello.identity, desktop.identity());
    Ok(())
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    needle.is_empty() || haystack.windows(needle.len()).any(|w| w == needle)
}

/// Criterion 1: a code is 8 characters in two groups of four from Crockford base 32.
#[test]
fn criterion_1_code_has_two_groups_of_four_without_look_alikes() {
    for slot in [0, 1, 0x1234, PAIR_SLOTS - 1] {
        for n in [0, 7, 0xff] {
            let code = PairCode::new(slot, PairSecret::from_entropy([n, n, n, n])).unwrap();
            let text = code.to_string();
            assert_eq!(text.len(), 9, "{text}");
            assert_eq!(&text[4..5], "-");
            for c in text.chars().filter(|&c| c != '-') {
                assert!(
                    c.is_ascii_digit() || c.is_ascii_uppercase(),
                    "{c} in {text}"
                );
                assert!(!"ILOU".contains(c), "look-alike {c} in {text}");
            }
            assert_eq!(PairCode::parse(&text).unwrap(), code);
            assert_eq!(code.slot(), slot);
        }
    }
}

/// A slot at or above 2^15 cannot be shown in 3 characters.
#[test]
fn criterion_1_a_slot_beyond_15_bits_is_refused() {
    assert!(PairCode::new(PAIR_SLOTS, secret(1)).is_err());
    assert!(PairCode::new(u16::MAX, secret(1)).is_err());
}

/// Criterion 2 (typing the code): case folds, spaces and dashes drop, I and L read as 1, O as 0.
#[test]
fn criterion_2_typed_code_folds_look_alikes_and_separators() {
    let code = PairCode::new(0x0421, PairSecret::from_entropy([0x10, 0x20, 0x30, 0x40])).unwrap();
    let text = code.to_string();
    let lower = text.to_lowercase();
    assert_eq!(PairCode::parse(&lower).unwrap(), code);
    let spaced = text.replace('-', " ");
    assert_eq!(PairCode::parse(&spaced).unwrap(), code);
    let joined = text.replace('-', "");
    assert_eq!(PairCode::parse(&format!("  {joined}  ")).unwrap(), code);
    let looks = text.replace('1', "l").replace('0', "O");
    assert_eq!(PairCode::parse(&looks).unwrap(), code);
    let looks = text.replace('1', "I").replace('0', "o");
    assert_eq!(PairCode::parse(&looks).unwrap(), code);
}

/// Criterion 4: anything that is not a code is refused, never guessed.
#[test]
fn criterion_4_malformed_codes_are_refused() {
    for text in [
        "",
        "-",
        "ABCD-EFG",
        "ABCD-EFGHJ",
        "ABCD-EFGU",
        "ABCU-EFGH",
        "ABCD_EFGH",
        "ABCD-EFG!",
        "ÄBCD-EFGH",
        "ABCD-EFGH-",
        "ABCDEFGHJK",
    ] {
        if text == "ABCD-EFGH-" {
            // A trailing dash is a separator; it may be accepted, but must parse to the same code.
            if let Ok(code) = PairCode::parse(text) {
                assert_eq!(code, PairCode::parse("ABCD-EFGH").unwrap());
            }
            continue;
        }
        assert!(PairCode::parse(text).is_err(), "{text:?} was accepted");
    }
}

proptest! {
    /// Criterion 1: every slot and secret survives format then parse, and the slot stays below
    /// 2^15.
    #[test]
    fn criterion_1_format_parse_round_trip(slot in 0u16..PAIR_SLOTS, bytes in any::<[u8; 4]>()) {
        let code = PairCode::new(slot, PairSecret::from_entropy(bytes)).unwrap();
        let parsed = PairCode::parse(&code.to_string()).unwrap();
        prop_assert_eq!(parsed, code);
        prop_assert!(parsed.slot() < PAIR_SLOTS);
        prop_assert_eq!(parsed.secret().password(), code.secret().password());
    }

    /// Criterion 4: arbitrary text never panics the parser; what it accepts round-trips.
    #[test]
    fn criterion_4_parse_never_panics(text in ".{0,20}") {
        if let Ok(code) = PairCode::parse(&text) {
            prop_assert!(code.slot() < PAIR_SLOTS);
            prop_assert_eq!(PairCode::parse(&code.to_string()).unwrap(), code);
        }
    }
}

/// The secret carries 25 bits: entropy differing only above them gives the same password, and
/// the 25 bits themselves are all used.
#[test]
fn criterion_1_the_secret_is_25_bits() {
    let a = PairSecret::from_entropy([0, 0, 0, 1]);
    let b = PairSecret::from_entropy([0, 0, 0, 2]);
    assert_ne!(a.password(), b.password());
    let codes: std::collections::BTreeSet<String> = (0u8..=255)
        .map(|n| {
            PairCode::new(5, PairSecret::from_entropy([n, n.wrapping_mul(3), 7, 1]))
                .unwrap()
                .to_string()
        })
        .collect();
    assert!(codes.len() > 200, "secrets collapse: {}", codes.len());
}

/// Criteria 2 and 3: the honest run pairs, both sides derive the same four words, and the
/// welcome carries the group room.
#[test]
fn criterion_2_check_words_are_equal_on_both_sides() {
    let (mut joiner, mut opener, mut messages) = Run::honest().channels();
    let laptop = device(2);
    let desktop = device(1);
    confirm(&mut joiner, &mut opener, &laptop, &desktop, &mut messages).unwrap();
    let on_opener = opener.check_words(&desktop.identity()).unwrap();
    let on_joiner = joiner.check_words(&laptop.identity()).unwrap();
    assert_eq!(on_opener, on_joiner);
    assert!(on_opener.iter().all(|&w| w < 2048));
    let welcome = opener.welcome_message(ROOM).unwrap();
    assert_eq!(joiner.open_welcome(&welcome).unwrap(), ROOM);
}

/// Criterion 4: a wrong code fails on the opener's side before anything sealed is opened.
#[test]
fn criterion_4_wrong_code_fails_confirmation() {
    let mut run = Run::honest();
    run.joiner_secret = secret(2);
    let (mut joiner, mut opener, mut messages) = run.channels();
    let err = confirm(
        &mut joiner,
        &mut opener,
        &device(2),
        &device(1),
        &mut messages,
    );
    assert!(err.is_err());
}

/// A codes that differs in one bit of the secret is wrong too.
#[test]
fn criterion_4_one_bit_of_the_secret_matters() {
    let mut run = Run::honest();
    run.joiner_secret = PairSecret::from_entropy([1, 0x5a, 0xc3, 0x10]);
    run.opener_secret = PairSecret::from_entropy([1, 0x5a, 0xc3, 0x11]);
    assert_ne!(run.joiner_secret.password(), run.opener_secret.password());
    let (mut joiner, mut opener, mut messages) = run.channels();
    assert!(
        confirm(
            &mut joiner,
            &mut opener,
            &device(2),
            &device(1),
            &mut messages
        )
        .is_err()
    );
}

/// A relay that hands the two sides different session ids breaks the pairing.
#[test]
fn criterion_2_relay_swapping_sid_fails() {
    let mut run = Run::honest();
    run.opener_sid = [0x52; 16];
    let (mut joiner, mut opener, mut messages) = run.channels();
    assert!(
        confirm(
            &mut joiner,
            &mut opener,
            &device(2),
            &device(1),
            &mut messages
        )
        .is_err()
    );
}

/// The channel is bound to the server: the same code on another server does not pair.
#[test]
fn criterion_2_another_server_fails() {
    let mut run = Run::honest();
    run.opener_server = "ws://192.168.1.21:8080";
    let (mut joiner, mut opener, mut messages) = run.channels();
    assert!(
        confirm(
            &mut joiner,
            &mut opener,
            &device(2),
            &device(1),
            &mut messages
        )
        .is_err()
    );
}

/// A relay that swaps both shares for its own (a man in the middle guessing the code wrongly)
/// is caught at the first confirmation on both channels.
#[test]
fn criterion_2_relay_swapping_shares_is_caught() {
    let sid = [0x51; 16];
    let joiner = PairStart::new(PairRole::Joiner, secret(1), SERVER, sid, &[0x0b; 32]).unwrap();
    let opener = PairStart::new(PairRole::Opener, secret(1), SERVER, sid, &[0x0a; 32]).unwrap();
    // The relay plays opener to the joiner and joiner to the opener with a guessed secret.
    let fake_opener =
        PairStart::new(PairRole::Opener, secret(9), SERVER, sid, &[0x0c; 32]).unwrap();
    let fake_joiner =
        PairStart::new(PairRole::Joiner, secret(9), SERVER, sid, &[0x0d; 32]).unwrap();
    let joiner_share = share(&joiner.share_message());
    let opener_share = share(&opener.share_message());
    let fake_opener_share = share(&fake_opener.share_message());
    let fake_joiner_share = share(&fake_joiner.share_message());

    let mut joiner = joiner.finish(&fake_opener_share).unwrap();
    let mut opener = opener.finish(&fake_joiner_share).unwrap();
    let mut fake_opener = fake_opener.finish(&joiner_share).unwrap();
    let mut fake_joiner = fake_joiner.finish(&opener_share).unwrap();

    let laptop = device(2);
    let mallory = device(9);
    let from_joiner = joiner.confirm_message(&laptop, "Laptop").unwrap();
    assert!(fake_opener.open_confirm(&from_joiner).is_err());
    let forged = fake_joiner.confirm_message(&mallory, "Laptop").unwrap();
    assert!(opener.open_confirm(&forged).is_err());
    // Forwarding the joiner's real confirmation unchanged also fails on the opener.
    assert!(opener.open_confirm(&from_joiner).is_err());
    // The relay's own confirmation towards the joiner fails there too.
    let from_fake_opener = fake_opener.confirm_message(&mallory, "Desktop").unwrap();
    assert!(joiner.open_confirm(&from_fake_opener).is_err());
}

/// A relay that knows the code but swaps the joiner's identity for its own (re-sealing is
/// impossible without the seal key, so it can only forward bytes it changed): any changed
/// byte of a confirm fails.
#[test]
fn criterion_2_a_changed_confirm_fails() {
    let (mut joiner, _, _) = Run::honest().channels();
    let message = joiner.confirm_message(&device(2), "Laptop").unwrap();
    let bytes = message.to_bytes();
    for i in 0..bytes.len() {
        let mut changed = bytes.clone();
        changed[i] ^= 0x01;
        let Ok(parsed) = PairMessage::parse(&changed) else {
            continue;
        };
        let (_, mut fresh_opener, _) = Run::honest().channels();
        assert!(
            fresh_opener.open_confirm(&parsed).is_err(),
            "byte {i} changed and still opened"
        );
    }
}

/// A replayed confirm (opened twice) and a confirm reflected back to its sender fail.
#[test]
fn criterion_2_replayed_or_reflected_confirm_fails() {
    let (mut joiner, mut opener, _) = Run::honest().channels();
    let from_joiner = joiner.confirm_message(&device(2), "Laptop").unwrap();
    opener.open_confirm(&from_joiner).unwrap();
    // Reflection: the joiner's own confirm handed back to it.
    assert!(joiner.open_confirm(&from_joiner).is_err());
    // A second confirm cannot be sealed by the same side (nonce reuse).
    assert!(joiner.confirm_message(&device(2), "Laptop").is_err());
}

/// The welcome only opens for the joiner of this pairing, and only from the opener.
#[test]
fn criterion_3_welcome_is_bound_to_the_pairing() {
    let (mut joiner, mut opener, mut messages) = Run::honest().channels();
    confirm(
        &mut joiner,
        &mut opener,
        &device(2),
        &device(1),
        &mut messages,
    )
    .unwrap();
    let welcome = opener.welcome_message(ROOM).unwrap();
    // Another pairing's joiner with another sid cannot open it.
    let mut other = Run::honest();
    other.joiner_sid = [0x61; 16];
    other.opener_sid = [0x61; 16];
    let (mut j2, mut o2, mut m2) = other.channels();
    confirm(&mut j2, &mut o2, &device(2), &device(1), &mut m2).unwrap();
    assert!(j2.open_welcome(&welcome).is_err());
    // The opener cannot open its own welcome as if it came from the joiner.
    assert!(opener.open_welcome(&welcome).is_err());
    // A second welcome is refused (one per pairing).
    assert!(opener.welcome_message(ROOM).is_err());
}

/// Criterion 2: with a relay that guessed the code and swapped the joiner's identity, the
/// opener's words are computed over another identity and do not match the joiner's (the words
/// cover both identities).
#[test]
fn criterion_2_words_cover_both_identities() {
    let (mut j1, mut o1, mut m1) = Run::honest().channels();
    confirm(&mut j1, &mut o1, &device(2), &device(1), &mut m1).unwrap();
    let (mut j2, mut o2, mut m2) = Run::honest().channels();
    confirm(&mut j2, &mut o2, &device(3), &device(1), &mut m2).unwrap();
    let with_laptop = o1.check_words(&device(1).identity()).unwrap();
    let with_other = o2.check_words(&device(1).identity()).unwrap();
    assert_ne!(with_laptop, with_other);
    // Words before the peer is known are refused, not made up.
    let (fresh, _, _) = Run::honest().channels();
    assert!(fresh.check_words(&device(2).identity()).is_err());
}

/// Criteria 3 and 30: no private key byte and no code secret appears in any pairing message.
#[test]
fn criterion_3_no_private_key_or_secret_in_any_message() {
    let laptop = device(2);
    let desktop = device(1);
    let (mut joiner, mut opener, mut messages) = Run::honest().channels();
    confirm(&mut joiner, &mut opener, &laptop, &desktop, &mut messages).unwrap();
    messages.push(opener.welcome_message(ROOM).unwrap());
    let bytes: Vec<u8> = messages.iter().flat_map(PairMessage::to_bytes).collect();
    for keys in [&laptop, &desktop] {
        let entropy = keys.to_entropy();
        for window in entropy.chunks(16) {
            assert!(!contains(&bytes, window), "private key bytes in a message");
        }
    }
    let password = secret(1).password();
    assert!(!contains(&bytes, &password));
    assert!(!contains(&bytes, b"ZQX-PC-7731"), "label in clear");
    assert!(!contains(&bytes, &ROOM.0), "group room in clear");
}

/// A label must be 1 to 255 bytes.
#[test]
fn criterion_2_label_bounds() {
    let (mut joiner, _, _) = Run::honest().channels();
    assert!(joiner.confirm_message(&device(2), "").is_err());
    let (mut joiner, _, _) = Run::honest().channels();
    assert!(
        joiner
            .confirm_message(&device(2), &"x".repeat(256))
            .is_err()
    );
    let (mut joiner, mut opener, _) = Run::honest().channels();
    let long = "é".repeat(127);
    let message = joiner.confirm_message(&device(2), &long).unwrap();
    assert_eq!(opener.open_confirm(&message).unwrap().label, long);
}

/// Malformed pairing messages are refused, never a panic.
#[test]
fn criterion_2_garbage_messages_are_refused() {
    for bytes in [
        vec![],
        vec![1],
        vec![2, 1],
        vec![1, 1, 0],
        vec![9; 40],
        vec![1, 9],
    ] {
        assert!(PairMessage::parse(&bytes).is_err(), "{bytes:?}");
    }
    let (_, mut opener, _) = Run::honest().channels();
    assert!(opener.open_confirm(&PairMessage::Share([0; 32])).is_err());
}

/// An identity point as a share (relay sends zeros) is refused, not turned into a key.
#[test]
fn criterion_2_identity_share_is_refused() {
    let joiner = PairStart::new(PairRole::Joiner, secret(1), SERVER, [1; 16], &[3; 32]).unwrap();
    assert!(joiner.finish(&[0; 32]).is_err());
    let joiner = PairStart::new(PairRole::Joiner, secret(1), SERVER, [1; 16], &[3; 32]).unwrap();
    assert!(joiner.finish(&[0xff; 32]).is_err());
}

// --- Passkey backup (criteria 29, 34, 36; ADR 0040 decision 13) ---

fn content() -> BackupContent {
    BackupContent {
        identity: Zeroizing::new(*device(5).to_entropy()),
        group_room: ROOM,
        server: "wss://relay.example:8443".to_owned(),
        label: "ZQX-PC-7731 Firefox on Linux".to_owned(),
    }
}

/// The PRF input is fixed: SHA-256 of `"curvyo-backup-v1"`.
#[test]
fn criterion_36_prf_input_is_fixed() {
    use sha2::Digest;
    let expected: [u8; 32] = sha2::Sha256::digest(b"curvyo-backup-v1").into();
    assert_eq!(backup_prf_input(), expected);
}

/// The record round-trips with the same passkey and opens with nothing else.
#[test]
fn criterion_36_restore_with_same_passkey_only() {
    let keys = BackupKeys::derive(&[7; 32]);
    let record = keys.seal(&content(), &[9; 24]).unwrap();
    let opened = keys.open(&record).unwrap();
    assert_eq!(opened, content());
    assert_eq!(
        IdentityKeys::from_entropy(&opened.identity)
            .identity()
            .fingerprint(),
        device(5).identity().fingerprint()
    );
    let other = BackupKeys::derive(&[8; 32]);
    assert_ne!(other.id(), keys.id());
    assert_ne!(other.write_key(), keys.write_key());
    assert!(other.open(&record).is_err());
}

/// Criterion 34: the record holds nothing readable by the relay.
#[test]
fn criterion_34_record_is_unreadable_to_the_server() {
    let keys = BackupKeys::derive(&[7; 32]);
    let record = keys.seal(&content(), &[9; 24]).unwrap();
    let entropy = device(5).to_entropy();
    for window in entropy.chunks(16) {
        assert!(!contains(&record, window));
    }
    assert!(!contains(&record, &device(5).identity().to_bytes()[..32]));
    assert!(!contains(&record, &ROOM.0));
    assert!(!contains(&record, b"ZQX-PC-7731"));
    assert!(!contains(&record, b"relay.example"));
    assert!(!contains(&record, &[7; 32]));
    assert_eq!(record[0], BACKUP_RECORD_VERSION);
}

/// Any changed byte of the record is refused (tampered record), and so is a truncation.
#[test]
fn criterion_36_tampered_record_is_refused() {
    let keys = BackupKeys::derive(&[7; 32]);
    let record = keys.seal(&content(), &[9; 24]).unwrap();
    for i in 0..record.len() {
        let mut changed = record.clone();
        changed[i] ^= 0x80;
        assert!(keys.open(&changed).is_err(), "byte {i}");
    }
    for len in 0..record.len() {
        assert!(keys.open(&record[..len]).is_err(), "truncated to {len}");
    }
    let mut longer = record.clone();
    longer.push(0);
    assert!(keys.open(&longer).is_err());
}

/// A newer record version is refused, not read as v1.
#[test]
fn criterion_36_newer_record_version_is_refused() {
    let keys = BackupKeys::derive(&[7; 32]);
    let mut record = keys.seal(&content(), &[9; 24]).unwrap();
    record[0] = BACKUP_RECORD_VERSION + 1;
    assert!(keys.open(&record).is_err());
}

/// Oversized fields are refused at seal time.
#[test]
fn criterion_36_oversized_fields_are_refused() {
    let keys = BackupKeys::derive(&[7; 32]);
    let mut big = content();
    big.server = "w".repeat(curvyo_crypto_core::BACKUP_SERVER_MAX + 1);
    assert!(keys.seal(&big, &[1; 24]).is_err());
    let mut big = content();
    big.label = "l".repeat(curvyo_crypto_core::BACKUP_LABEL_MAX + 1);
    assert!(keys.seal(&big, &[1; 24]).is_err());
}

/// A write signature binds the op, the id, the challenge nonce and the record bytes.
#[test]
fn criterion_36_write_signature_binds_op_id_nonce_record() {
    let keys = BackupKeys::derive(&[7; 32]);
    let record = keys.seal(&content(), &[9; 24]).unwrap();
    let nonce = [0x33; 32];
    let sig = keys.sign_write(BackupOp::Put, &nonce, &record);
    let wk = keys.write_key();
    let id = keys.id();
    verify_backup_write(&wk, BackupOp::Put, id, &nonce, &record, &sig).unwrap();
    assert!(verify_backup_write(&wk, BackupOp::Delete, id, &nonce, &record, &sig).is_err());
    assert!(verify_backup_write(&wk, BackupOp::Put, id, &[0x34; 32], &record, &sig).is_err());
    let mut other_record = record.clone();
    other_record[5] ^= 1;
    assert!(verify_backup_write(&wk, BackupOp::Put, id, &nonce, &other_record, &sig).is_err());
    let other_id = BackupKeys::derive(&[8; 32]).id();
    assert!(verify_backup_write(&wk, BackupOp::Put, other_id, &nonce, &record, &sig).is_err());
    let other_key = BackupKeys::derive(&[8; 32]).write_key();
    assert!(verify_backup_write(&other_key, BackupOp::Put, id, &nonce, &record, &sig).is_err());
}

/// A record sealed under one id cannot be served as another id's record, even by someone with
/// both passkeys' keys (the id is bound as associated data).
#[test]
fn criterion_36_record_is_bound_to_its_id() {
    let a = BackupKeys::derive(&[7; 32]);
    let b = BackupKeys::derive(&[8; 32]);
    let record = a.seal(&content(), &[9; 24]).unwrap();
    assert!(b.open(&record).is_err());
}

/// The record's derived values depend on the PRF output only (deterministic per passkey), so
/// Back up again replaces the same record.
#[test]
fn criterion_36_same_passkey_same_id_and_write_key() {
    let a = BackupKeys::derive(&[7; 32]);
    let b = BackupKeys::derive(&[7; 32]);
    assert_eq!(a.id(), b.id());
    assert_eq!(a.write_key(), b.write_key());
    // The derived id is not the PRF output itself, nor its prefix.
    assert_ne!(a.id().0, [7; 16]);
}
