//! The SPAKE2 exchange of pairing (addendum A6): the code is the password, the connector's key
//! hash is in the transcript, and both sides confirm the session key with an HMAC.

use ring::hmac;
use spake2::{Ed25519Group, Identity, Password, Spake2};
use zeroize::Zeroizing;

use crate::pairing_code;
use crate::spki::SpkiHash;

/// Identity of the app side in the transcript.
const APP_ID: &[u8] = b"curvyo-connector-app/0";
const APP_CONFIRM: &[u8] = b"curvyo-connector pair confirm app";
const CONNECTOR_CONFIRM: &[u8] = b"curvyo-connector pair confirm connector";

/// The exchange failed: the peer's message is not a SPAKE2 message of this group.
#[derive(Debug, thiserror::Error)]
#[error("the pairing message is not valid")]
pub struct SpakeError;

/// One side's state between its message and the peer's.
pub struct Exchange(Spake2<Ed25519Group>);

/// The agreed key; only its two confirmations leave this type.
pub struct SessionKey(Zeroizing<Vec<u8>>);

fn identities(spki: &SpkiHash) -> (Identity, Identity) {
    (Identity::new(APP_ID), Identity::new(spki.as_bytes()))
}

/// The app side: `spki` is the hash of the key the app saw.
#[must_use]
pub fn start_app(code: &pairing_code::Password, spki: &SpkiHash) -> (Exchange, Vec<u8>) {
    let (a, b) = identities(spki);
    let (state, message) = Spake2::<Ed25519Group>::start_a(&Password::new(code.as_ref()), &a, &b);
    (Exchange(state), message)
}

/// The connector side: `spki` is the hash of its own key.
#[must_use]
pub fn start_connector(code: &pairing_code::Password, spki: &SpkiHash) -> (Exchange, Vec<u8>) {
    let (a, b) = identities(spki);
    let (state, message) = Spake2::<Ed25519Group>::start_b(&Password::new(code.as_ref()), &a, &b);
    (Exchange(state), message)
}

impl Exchange {
    /// Combines the peer's message into the session key.
    ///
    /// # Errors
    /// [`SpakeError`] for a message of the wrong length or not on the curve.
    pub fn finish(self, peer: &[u8]) -> Result<SessionKey, SpakeError> {
        self.0
            .finish(peer)
            .map(|key| SessionKey(Zeroizing::new(key)))
            .map_err(|_| SpakeError)
    }
}

impl SessionKey {
    fn tag(&self, label: &[u8]) -> Vec<u8> {
        let key = hmac::Key::new(hmac::HMAC_SHA256, &self.0);
        hmac::sign(&key, label).as_ref().to_vec()
    }

    fn verify(&self, label: &[u8], tag: &[u8]) -> bool {
        let key = hmac::Key::new(hmac::HMAC_SHA256, &self.0);
        hmac::verify(&key, label, tag).is_ok()
    }

    /// The app's confirmation (round 2, app to connector).
    #[must_use]
    pub fn app_tag(&self) -> Vec<u8> {
        self.tag(APP_CONFIRM)
    }

    /// The connector's confirmation (round 2, connector to app).
    #[must_use]
    pub fn connector_tag(&self) -> Vec<u8> {
        self.tag(CONNECTOR_CONFIRM)
    }

    /// Checks the app's confirmation in constant time.
    #[must_use]
    pub fn verify_app_tag(&self, tag: &[u8]) -> bool {
        self.verify(APP_CONFIRM, tag)
    }

    /// Checks the connector's confirmation in constant time.
    #[must_use]
    pub fn verify_connector_tag(&self, tag: &[u8]) -> bool {
        self.verify(CONNECTOR_CONFIRM, tag)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hex;
    use rand_core::{CryptoRng, RngCore};

    fn code(text: &str) -> pairing_code::Password {
        pairing_code::normalise(text).unwrap()
    }

    fn run(
        app_code: &str,
        app_spki: &SpkiHash,
        connector_spki: &SpkiHash,
    ) -> (SessionKey, SessionKey) {
        let (app, app_message) = start_app(&code(app_code), app_spki);
        let (connector, connector_message) = start_connector(&code("K7QM-4XTR"), connector_spki);
        (
            app.finish(&connector_message).unwrap(),
            connector.finish(&app_message).unwrap(),
        )
    }

    #[test]
    fn the_right_code_and_key_give_matching_confirmations() {
        let spki = SpkiHash::of_spki_der(b"key");
        let (app, connector) = run("K7QM-4XTR", &spki, &spki);
        assert!(connector.verify_app_tag(&app.app_tag()));
        assert!(app.verify_connector_tag(&connector.connector_tag()));
        assert!(!connector.verify_app_tag(&app.connector_tag()));
    }

    #[test]
    fn a_wrong_code_or_another_key_fails_the_confirmation() {
        let spki = SpkiHash::of_spki_der(b"key");
        let (app, connector) = run("K7QM-4XTS", &spki, &spki);
        assert!(!connector.verify_app_tag(&app.app_tag()));
        let relay = SpkiHash::of_spki_der(b"relay key");
        let (app, connector) = run("K7QM-4XTR", &relay, &spki);
        assert!(!connector.verify_app_tag(&app.app_tag()));
        assert!(!app.verify_connector_tag(&connector.connector_tag()));
    }

    #[test]
    fn a_message_of_the_wrong_length_is_refused() {
        let spki = SpkiHash::of_spki_der(b"key");
        let (app, _) = start_app(&code("K7QM-4XTR"), &spki);
        assert!(app.finish(&[1, 2, 3]).is_err());
    }

    /// A fixed random source: the byte `n` for the `n`-th byte asked for.
    struct Counter(u8);
    impl RngCore for Counter {
        fn next_u32(&mut self) -> u32 {
            rand_core::impls::next_u32_via_fill(self)
        }
        fn next_u64(&mut self) -> u64 {
            rand_core::impls::next_u64_via_fill(self)
        }
        fn fill_bytes(&mut self, dest: &mut [u8]) {
            for byte in dest {
                self.0 = self.0.wrapping_add(1);
                *byte = self.0;
            }
        }
        fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand_core::Error> {
            self.fill_bytes(dest);
            Ok(())
        }
    }
    impl CryptoRng for Counter {}

    #[test]
    fn known_answer_with_fixed_scalars() {
        let spki = SpkiHash::of_spki_der(b"known answer key");
        let password = Password::new(b"K7QM4XTR");
        let (a, b) = identities(&spki);
        let (app, app_message) =
            Spake2::<Ed25519Group>::start_a_with_rng(&password, &a, &b, Counter(0));
        let (connector, connector_message) =
            Spake2::<Ed25519Group>::start_b_with_rng(&password, &a, &b, Counter(100));
        let app_key = Exchange(app).finish(&connector_message).unwrap();
        let connector_key = Exchange(connector).finish(&app_message).unwrap();
        assert_eq!(app_key.app_tag(), connector_key.app_tag());
        let shown = (
            hex::encode(&app_message),
            hex::encode(&connector_message),
            hex::encode(&app_key.app_tag()),
            hex::encode(&app_key.connector_tag()),
        );
        let expected = (
            "416dd92a96b476aa35c4d20c27e41a2fa384dbd2346f495e6e9c3ebeb27ce14837",
            "42c8f1d85ea94719b18a2adba333d41ae379c6ca9586ccf043883fcd2016f906f5",
            "c8246323239fdf3692bf58c113f8009330e3658aa920c9c0818fc69a691f752d",
            "57c3a7136562d28406b5e8b66eaeecae0d46450218b69aac0ca49885734e3d09",
        );
        assert_eq!(
            shown,
            (
                expected.0.into(),
                expected.1.into(),
                expected.2.into(),
                expected.3.into()
            )
        );
    }
}
