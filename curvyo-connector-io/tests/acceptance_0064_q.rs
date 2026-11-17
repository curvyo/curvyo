//! Acceptance tests for 0064 slice 2, criterion Q2 (pairing code rules), written from the
//! specification before reading the implementation. The clock is an injected `Instant`.
#![allow(clippy::unwrap_used)]

use std::time::{Duration, Instant};

use curvyo_connector_io::pairing_code::{PairingCodes, Refusal, normalise};

#[test]
fn q2_a_code_has_8_characters_from_one_32_symbol_alphabet_without_o_i_l() {
    let now = Instant::now();
    let mut codes = PairingCodes::default();
    let mut seen = std::collections::BTreeSet::new();
    for _ in 0..4000 {
        let shown = codes.issue(now).unwrap();
        let plain: String = shown.chars().filter(|c| *c != '-').collect();
        assert_eq!(plain.chars().count(), 8, "{shown}");
        for c in plain.chars() {
            assert!(c.is_ascii_digit() || c.is_ascii_uppercase(), "{shown}: {c}");
            assert!(!"OIL".contains(c), "look-alike {c} in {shown}");
            seen.insert(c);
        }
    }
    assert!(seen.len() <= 32, "more than 32 symbols: {seen:?}");
    assert!(seen.len() >= 28, "suspiciously few symbols: {seen:?}");
}

#[test]
fn q2_look_alikes_typed_by_the_maker_still_match_and_noise_is_ignored() {
    let now = Instant::now();
    let mut codes = PairingCodes::default();
    let shown = codes.issue_from([0, 0, 0, 0, 0], now);
    let typed = shown.to_lowercase().replace('-', " ").replace('0', "o");
    assert_eq!(normalise(&typed).unwrap(), normalise(&shown).unwrap());
    assert!(normalise("ABC").is_none());
    assert!(normalise("ABCDEFGHJ").is_none());
}

#[test]
fn q2_valid_for_ten_minutes_by_the_injected_clock() {
    let t0 = Instant::now();
    let mut codes = PairingCodes::default();
    codes.issue(t0).unwrap();
    assert!(codes.current(t0 + Duration::from_secs(599)).is_some());
    assert!(codes.current(t0 + Duration::from_secs(600)).is_none());
    let mut late = PairingCodes::default();
    late.issue(t0).unwrap();
    assert_eq!(
        late.begin(t0 + Duration::from_secs(601)).unwrap_err(),
        Refusal::Expired
    );
    let mut ontime = PairingCodes::default();
    ontime.issue(t0).unwrap();
    assert!(ontime.begin(t0 + Duration::from_secs(599)).is_ok());
}

#[test]
fn q2_works_once() {
    let t0 = Instant::now();
    let mut codes = PairingCodes::default();
    codes.issue(t0).unwrap();
    codes.begin(t0).unwrap();
    codes.succeeded();
    assert_eq!(codes.begin(t0).unwrap_err(), Refusal::NoCode);
    assert!(codes.current(t0).is_none());
}

#[test]
fn q2_three_wrong_attempts_spend_the_code() {
    let t0 = Instant::now();
    let mut codes = PairingCodes::default();
    codes.issue(t0).unwrap();
    for left in [2u8, 1, 0] {
        codes.begin(t0).unwrap();
        assert_eq!(codes.failed(), left);
    }
    assert_eq!(codes.begin(t0).unwrap_err(), Refusal::NoCode);
}

#[test]
fn q2_a_second_code_replaces_a_valid_one() {
    let t0 = Instant::now();
    let mut codes = PairingCodes::default();
    let first = codes.issue_from([1, 2, 3, 4, 5], t0);
    let second = codes.issue_from([9, 8, 7, 6, 5], t0);
    assert_ne!(first, second);
    assert_eq!(codes.current(t0).unwrap(), second);
    let password = codes.begin(t0).unwrap();
    assert_eq!(password, normalise(&second).unwrap());
    assert_ne!(password, normalise(&first).unwrap());
}
