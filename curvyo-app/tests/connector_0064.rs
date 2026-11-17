//! `0064` Q10 and A8: the credential record has a fixed shape, and neither the project file nor a
//! machine profile record holds the pinned key, the token or the connector's name.
#![cfg(target_os = "linux")]
#![allow(clippy::unwrap_used)]

use curvyo_connector_io::record::PairingRecord;
use curvyo_connector_io::spki::SpkiHash;
use curvyo_connector_io::token::Token;

fn record() -> PairingRecord {
    PairingRecord {
        spki: SpkiHash::from_hex(&"ab".repeat(32)).unwrap(),
        token: Token::from_hex(&"cd".repeat(32)).unwrap(),
        connector_name: "Workshop PC".to_owned(),
    }
}

#[test]
fn the_credential_record_matches_its_golden_file() {
    let golden = std::fs::read(format!(
        "{}/tests/fixtures/connector_record.json",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();
    assert_eq!(record().to_bytes().as_slice(), golden.trim_ascii_end());
}

fn contains(haystack: &[u8], needle: &str) -> bool {
    haystack
        .windows(needle.len())
        .any(|w| w == needle.as_bytes())
}

#[test]
fn q10_neither_the_project_nor_the_profile_record_holds_pin_token_or_name() {
    let record = record();
    let secrets = [
        record.token.to_hex().to_string(),
        record.spki.to_hex(),
        record.spki.fingerprint(),
        record.connector_name.clone(),
    ];
    let mut profile = curvyo_laser_core::MachineProfile::generic_laser();
    profile.connection.connector_address = "192.168.1.20:7447".to_owned();
    let profile_text = curvyo_laser_core::write_record(&profile);
    assert!(profile_text.contains("192.168.1.20:7447"));
    let document = curvyo_document_core::Document::new(1);
    let project = curvyo_document_core::pack(&document, "0.1.0").unwrap();
    for secret in &secrets {
        assert!(!contains(profile_text.as_bytes(), secret), "{secret}");
        assert!(!contains(&project, secret), "{secret}");
    }
}
