//! The credential-store record of one paired connector (ADR 0007, addendum A8): the pinned key,
//! the token and the connector's name, as one JSON value kept under `connector/<profile id>`.

use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use crate::spki::SpkiHash;
use crate::token::Token;

const FORMAT: u32 = 1;

/// Why a stored record cannot be used.
#[derive(Debug, thiserror::Error)]
#[error("the stored pairing is not a record of this version")]
pub struct RecordError;

#[derive(Serialize, Deserialize)]
struct Wire {
    format: u32,
    spki_sha256: String,
    token: String,
    connector_name: String,
}

/// What the app keeps of a pairing.
#[derive(Debug)]
pub struct PairingRecord {
    /// The pinned key.
    pub spki: SpkiHash,
    /// The token.
    pub token: Token,
    /// The connector's name, so the row can show it offline.
    pub connector_name: String,
}

impl PairingRecord {
    /// The JSON bytes to store.
    ///
    /// # Panics
    /// Never: the record has only strings and a number.
    #[must_use]
    pub fn to_bytes(&self) -> Zeroizing<Vec<u8>> {
        let wire = Wire {
            format: FORMAT,
            spki_sha256: self.spki.to_hex(),
            token: self.token.to_hex().to_string(),
            connector_name: self.connector_name.clone(),
        };
        // invariant: a struct of strings and a number always serialises.
        #[allow(clippy::expect_used)]
        Zeroizing::new(serde_json::to_vec(&wire).expect("a record serialises"))
    }

    /// Reads stored bytes.
    ///
    /// # Errors
    /// [`RecordError`] for anything but a version 1 record.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, RecordError> {
        let wire: Wire = serde_json::from_slice(bytes).map_err(|_| RecordError)?;
        if wire.format != FORMAT {
            return Err(RecordError);
        }
        Ok(Self {
            spki: SpkiHash::from_hex(&wire.spki_sha256).ok_or(RecordError)?,
            token: Token::from_hex(&wire.token).ok_or(RecordError)?,
            connector_name: wire.connector_name,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_record_round_trips() {
        let token = Token::generate().unwrap();
        let record = PairingRecord {
            spki: SpkiHash::of_spki_der(b"key"),
            token,
            connector_name: "Workshop PC".to_owned(),
        };
        let back = PairingRecord::from_bytes(&record.to_bytes()).unwrap();
        assert_eq!(back.spki, record.spki);
        assert_eq!(back.token.sha256_hex(), record.token.sha256_hex());
        assert_eq!(back.connector_name, "Workshop PC");
        assert!(PairingRecord::from_bytes(b"{}").is_err());
        assert!(
            PairingRecord::from_bytes(
                br#"{"format":2,"spki_sha256":"","token":"","connector_name":""}"#
            )
            .is_err()
        );
    }
}
