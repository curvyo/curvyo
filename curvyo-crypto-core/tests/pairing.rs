//! Pairing by code (ADR 0040 decision 4, `specs/0096-own-devices-sync` criteria 1 to 4): the
//! code, `CPace` between opener and joiner, confirmation, sealed hellos, the welcome and the
//! check words; golden pairing messages v1.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::missing_panics_doc)]

mod support;

use curvyo_crypto_core::{
    CryptoError, DocumentId, PAIR_SLOTS, PairChannel, PairCode, PairMessage, PairRole, PairSecret,
    PairStart,
};
use proptest::prelude::*;
use support::{device, golden};

const SERVER: &str = "ws://192.168.1.20:8080";
const SID: [u8; 16] = [0x51; 16];
const ROOM: DocumentId = DocumentId([0x6e; 16]);

fn secret(n: u8) -> PairSecret {
    PairSecret::from_entropy([n, 0x22, 0x33, 0x44])
}

fn share_of(message: &PairMessage) -> [u8; 32] {
    match message {
        PairMessage::Share(share) => *share,
        other => panic!("not a share: {other:?}"),
    }
}

/// Runs `CPace` between a joiner and an opener and returns both channels.
fn channels(joiner_secret: PairSecret, opener_secret: PairSecret) -> (PairChannel, PairChannel) {
    let joiner = PairStart::new(PairRole::Joiner, joiner_secret, SERVER, SID, &[0x0b; 32]).unwrap();
    let opener = PairStart::new(PairRole::Opener, opener_secret, SERVER, SID, &[0x0a; 32]).unwrap();
    let joiner_share = share_of(&joiner.share_message());
    let opener_share = share_of(&opener.share_message());
    (
        joiner.finish(&opener_share).unwrap(),
        opener.finish(&joiner_share).unwrap(),
    )
}

#[test]
fn opener_and_joiner_pair_and_see_the_same_words() {
    let (mut joiner, mut opener) = channels(secret(1), secret(1));
    let laptop = device(2);
    let desktop = device(1);

    let from_joiner = joiner.confirm_message(&laptop, "Laptop").unwrap();
    let hello = opener.open_confirm(&from_joiner).unwrap();
    assert_eq!(hello.identity, laptop.identity());
    assert_eq!(hello.label, "Laptop");

    let from_opener = opener.confirm_message(&desktop, "Desktop").unwrap();
    let hello = joiner.open_confirm(&from_opener).unwrap();
    assert_eq!(hello.identity, desktop.identity());
    assert_eq!(hello.label, "Desktop");

    let words_on_opener = opener.check_words(&desktop.identity()).unwrap();
    let words_on_joiner = joiner.check_words(&laptop.identity()).unwrap();
    assert_eq!(words_on_opener, words_on_joiner);
    assert!(words_on_opener.iter().all(|&index| index < 2048));

    let welcome = opener.welcome_message(ROOM).unwrap();
    assert_eq!(joiner.open_welcome(&welcome).unwrap(), ROOM);
}

/// Criterion 4: a wrong code fails at the opener's tag check, before anything is opened.
#[test]
fn a_wrong_code_fails_the_confirmation() {
    let (mut joiner, mut opener) = channels(secret(2), secret(1));
    let from_joiner = joiner.confirm_message(&device(2), "Laptop").unwrap();
    assert_eq!(
        opener.open_confirm(&from_joiner),
        Err(CryptoError::Unauthentic)
    );
}

/// A relay that swaps both shares for its own runs two `CPace` sessions with a guessed
/// password; with a wrong guess the forwarded confirmation fails at the opener.
#[test]
fn a_relay_swapping_the_shares_fails_the_confirmation() {
    let start = |role, secret, entropy| PairStart::new(role, secret, SERVER, SID, entropy).unwrap();
    let joiner = start(PairRole::Joiner, secret(1), &[0x0b; 32]);
    let opener = start(PairRole::Opener, secret(1), &[0x0a; 32]);
    let relay_as_opener = start(PairRole::Opener, secret(9), &[0x0c; 32]);
    let relay_as_joiner = start(PairRole::Joiner, secret(9), &[0x0d; 32]);
    let mut joiner = joiner
        .finish(&share_of(&relay_as_opener.share_message()))
        .unwrap();
    let mut opener = opener
        .finish(&share_of(&relay_as_joiner.share_message()))
        .unwrap();
    let from_joiner = joiner.confirm_message(&device(2), "Laptop").unwrap();
    assert_eq!(
        opener.open_confirm(&from_joiner),
        Err(CryptoError::Unauthentic)
    );
}

/// Another server address or another slot session id is another channel: the confirmation
/// fails.
#[test]
fn the_server_address_and_sid_are_bound() {
    for (server, sid) in [("ws://192.168.1.21:8080", SID), (SERVER, [0x52; 16])] {
        let joiner = PairStart::new(PairRole::Joiner, secret(1), SERVER, SID, &[0x0b; 32]).unwrap();
        let opener = PairStart::new(PairRole::Opener, secret(1), server, sid, &[0x0a; 32]).unwrap();
        let joiner_share = share_of(&joiner.share_message());
        let mut joiner = joiner.finish(&share_of(&opener.share_message())).unwrap();
        let mut opener = opener.finish(&joiner_share).unwrap();
        let from_joiner = joiner.confirm_message(&device(2), "Laptop").unwrap();
        assert_eq!(
            opener.open_confirm(&from_joiner),
            Err(CryptoError::Unauthentic)
        );
    }
}

/// An invalid share (the neutral element, or bytes that do not decode) aborts `CPace`.
#[test]
fn invalid_shares_abort() {
    for share in [[0; 32], [0xff; 32]] {
        let opener = PairStart::new(PairRole::Opener, secret(1), SERVER, SID, &[0x0a; 32]).unwrap();
        assert!(matches!(
            opener.finish(&share),
            Err(CryptoError::BadPublicKey)
        ));
    }
}

/// A changed byte of the sealed hello, a reflected confirmation and a hello signed by another
/// identity are all refused.
#[test]
fn tampered_reflected_or_missigned_hellos_are_refused() {
    let (mut joiner, mut opener) = channels(secret(1), secret(1));
    let from_joiner = joiner.confirm_message(&device(2), "Laptop").unwrap();
    let PairMessage::Confirm { tag, mut sealed } = from_joiner.clone() else {
        panic!("not a confirm");
    };
    let last = sealed.len() - 1;
    sealed[last] ^= 1;
    assert_eq!(
        opener.open_confirm(&PairMessage::Confirm { tag, sealed }),
        Err(CryptoError::Unauthentic)
    );

    // The opener's own confirmation reflected back to it.
    let from_opener = opener.confirm_message(&device(1), "Desktop").unwrap();
    assert_eq!(
        opener.open_confirm(&from_opener),
        Err(CryptoError::Unauthentic)
    );
    assert_eq!(
        opener.open_confirm(&PairMessage::Share([1; 32])),
        Err(CryptoError::Malformed)
    );
    assert!(opener.open_confirm(&from_joiner).is_ok());
}

#[test]
fn labels_must_be_1_to_255_bytes_and_each_message_is_sealed_once() {
    let (mut joiner, mut opener) = channels(secret(1), secret(1));
    assert_eq!(
        joiner.confirm_message(&device(2), ""),
        Err(CryptoError::Malformed)
    );
    assert_eq!(
        joiner.confirm_message(&device(2), &"x".repeat(256)),
        Err(CryptoError::Malformed)
    );
    let long = "ö".repeat(127);
    let message = joiner.confirm_message(&device(2), &long).unwrap();
    assert_eq!(joiner.welcome_message(ROOM), Err(CryptoError::Malformed));
    // No welcome before the joiner's confirmation checked out.
    assert_eq!(opener.welcome_message(ROOM), Err(CryptoError::Malformed));
    assert_eq!(opener.open_confirm(&message).unwrap().label, long);
    assert_eq!(
        joiner.confirm_message(&device(2), "Again"),
        Err(CryptoError::Malformed)
    );

    let welcome = opener.welcome_message(ROOM).unwrap();
    assert_eq!(opener.welcome_message(ROOM), Err(CryptoError::Malformed));
    assert_eq!(opener.open_welcome(&welcome), Err(CryptoError::Malformed));
}

/// The words cover both identities: a peer seen under another identity gives other words, and
/// no words exist before the peer's confirmation checked out.
#[test]
fn the_words_depend_on_both_identities() {
    let (mut joiner, mut opener) = channels(secret(1), secret(1));
    assert_eq!(
        opener.check_words(&device(1).identity()),
        Err(CryptoError::Malformed)
    );
    let from_joiner = joiner.confirm_message(&device(2), "Laptop").unwrap();
    opener.open_confirm(&from_joiner).unwrap();
    let from_opener = opener.confirm_message(&device(1), "Desktop").unwrap();
    joiner.open_confirm(&from_opener).unwrap();
    let real = opener.check_words(&device(1).identity()).unwrap();
    assert_eq!(joiner.check_words(&device(2).identity()).unwrap(), real);

    let (mut joiner, mut opener) = channels(secret(1), secret(1));
    let from_other = joiner.confirm_message(&device(3), "Laptop").unwrap();
    opener.open_confirm(&from_other).unwrap();
    assert_ne!(opener.check_words(&device(1).identity()).unwrap(), real);
}

/// Criterion 3: no message carries a byte string of either private key.
#[test]
fn no_message_carries_a_private_key() {
    let (mut joiner, mut opener) = channels(secret(1), secret(1));
    let laptop = device(2);
    let desktop = device(1);
    let from_joiner = joiner.confirm_message(&laptop, "Laptop").unwrap();
    opener.open_confirm(&from_joiner).unwrap();
    let messages = [
        from_joiner,
        opener.confirm_message(&desktop, "Desktop").unwrap(),
        opener.welcome_message(ROOM).unwrap(),
    ];
    for keys in [&laptop, &desktop] {
        let entropy = keys.to_entropy();
        for secret_part in [&entropy[..32], &entropy[32..]] {
            for message in &messages {
                let bytes = message.to_bytes();
                assert!(
                    !bytes.windows(32).any(|window| window == secret_part),
                    "a private key travels"
                );
            }
        }
    }
}

#[test]
fn messages_round_trip_and_refuse_bad_bytes() {
    let (mut joiner, mut opener) = channels(secret(1), secret(1));
    let confirm = joiner.confirm_message(&device(2), "Laptop").unwrap();
    opener.open_confirm(&confirm).unwrap();
    for message in [
        PairMessage::Share([7; 32]),
        confirm,
        opener.welcome_message(ROOM).unwrap(),
    ] {
        assert_eq!(PairMessage::parse(&message.to_bytes()).unwrap(), message);
    }
    assert_eq!(PairMessage::parse(&[]), Err(CryptoError::Malformed));
    assert_eq!(
        PairMessage::parse(&[2, 1]),
        Err(CryptoError::UnknownVersion(2))
    );
    assert_eq!(PairMessage::parse(&[1, 9]), Err(CryptoError::Malformed));
    assert_eq!(PairMessage::parse(&[1, 1, 0]), Err(CryptoError::Malformed));
    let mut long_share = vec![1, 1];
    long_share.extend_from_slice(&[0; 33]);
    assert_eq!(PairMessage::parse(&long_share), Err(CryptoError::Malformed));
    assert_eq!(PairMessage::parse(&[1, 2, 0]), Err(CryptoError::Malformed));
}

/// The golden pairing messages v1, made from fixed entropy, keys and labels. The format is
/// permanent; a change here is a format change.
#[test]
fn golden_pairing_messages_are_stable() {
    let joiner = PairStart::new(PairRole::Joiner, secret(1), SERVER, SID, &[0x0b; 32]).unwrap();
    let opener = PairStart::new(PairRole::Opener, secret(1), SERVER, SID, &[0x0a; 32]).unwrap();
    golden("pairing_share_v1.bin", &joiner.share_message().to_bytes());
    let opener_share = share_of(&opener.share_message());
    let joiner_share = share_of(&joiner.share_message());
    let mut joiner = joiner.finish(&opener_share).unwrap();
    let mut opener = opener.finish(&joiner_share).unwrap();
    let confirm = joiner.confirm_message(&device(2), "Laptop").unwrap();
    golden("pairing_confirm_v1.bin", &confirm.to_bytes());
    opener.open_confirm(&confirm).unwrap();
    let opener_confirm = opener.confirm_message(&device(1), "Desktop").unwrap();
    golden("pairing_confirm_opener_v1.bin", &opener_confirm.to_bytes());
    joiner.open_confirm(&opener_confirm).unwrap();
    let welcome = opener.welcome_message(ROOM).unwrap();
    golden("pairing_welcome_v1.bin", &welcome.to_bytes());
    let words = opener.check_words(&device(1).identity()).unwrap();
    assert_eq!(joiner.check_words(&device(2).identity()).unwrap(), words);
    golden(
        "pairing_words_v1.bin",
        &words
            .iter()
            .flat_map(|index| index.to_le_bytes())
            .collect::<Vec<_>>(),
    );
}

#[test]
fn slot_and_secret_fill_the_code() {
    let code = PairCode::new(PAIR_SLOTS - 1, PairSecret::from_entropy([0xff; 4])).unwrap();
    assert_eq!(code.to_string(), "ZZZZ-ZZZZ");
    assert_eq!(code.slot(), PAIR_SLOTS - 1);
}

proptest! {
    /// Criterion 1: every code shows as `XXXX-XXXX` and reads back as itself, also typed in
    /// lower case without the dash.
    #[test]
    fn codes_format_and_parse_back(slot in 0..PAIR_SLOTS, entropy in any::<[u8; 4]>()) {
        let code = PairCode::new(slot, PairSecret::from_entropy(entropy)).unwrap();
        let shown = code.to_string();
        prop_assert_eq!(shown.len(), 9);
        prop_assert_eq!(&shown[4..5], "-");
        prop_assert_eq!(PairCode::parse(&shown).unwrap(), code);
        prop_assert_eq!(PairCode::parse(&shown.replace('-', "").to_lowercase()).unwrap(), code);
    }
}
