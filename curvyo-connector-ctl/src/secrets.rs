//! The tokens in the credential store (ADR 0007, addendum D12): one record per connector key,
//! `curvyoctl/<spki_sha256>`, and how the store is opened on this computer.

use std::path::Path;

use curvyo_connector_io::spki::SpkiHash;
use curvyo_connector_io::token::Token;
use curvyo_storage_io::{CredentialError, CredentialStore, credential_slot};
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use crate::error::CtlError;

#[derive(Serialize, Deserialize)]
struct Record {
    format: u32,
    token: String,
}

fn record_name(spki: &SpkiHash) -> String {
    format!("curvyoctl/{}", spki.to_hex())
}

fn store_error(error: &CredentialError) -> CtlError {
    match error {
        CredentialError::NoKeychain => CtlError::refused("no credential store on this computer"),
        other => CtlError::refused(format!("the credential store failed: {other}")),
    }
}

/// Opens this computer's store for `dir`: the keychain, else the passphrase file when stdin is a
/// terminal, else none (never plaintext).
///
/// # Errors
/// "no credential store on this computer", or the store's failure.
pub fn open_store(dir: &Path) -> Result<CredentialStore, CtlError> {
    crate::known::make_private_dir(dir)
        .map_err(|e| CtlError::refused(format!("{}: {e}", dir.display())))?;
    let slot = credential_slot(dir).map_err(|e| store_error(&e))?;
    match CredentialStore::keychain(&slot) {
        Ok(store) => Ok(store),
        Err(CredentialError::NoKeychain) => {
            let passphrase = crate::passphrase::read()
                .ok_or_else(|| store_error(&CredentialError::NoKeychain))?;
            CredentialStore::file(dir, &passphrase).map_err(|e| store_error(&e))
        }
        Err(e) => Err(store_error(&e)),
    }
}

/// Stores the token of the connector with key `spki`.
///
/// # Errors
/// The store's failure.
pub fn put(store: &CredentialStore, spki: &SpkiHash, token: &Token) -> Result<(), CtlError> {
    let record = Record {
        format: 1,
        token: token.to_hex().to_string(),
    };
    let bytes =
        Zeroizing::new(serde_json::to_vec(&record).map_err(|e| CtlError::refused(e.to_string()))?);
    store
        .set(&record_name(spki), &bytes)
        .map_err(|e| store_error(&e))
}

/// The token of the connector with key `spki`.
///
/// # Errors
/// "no token for <name> in the credential store", or the store's failure.
pub fn get(store: &CredentialStore, spki: &SpkiHash, name: &str) -> Result<Token, CtlError> {
    let missing = || CtlError::refused(format!("no token for {name} in the credential store"));
    let bytes = store
        .get(&record_name(spki))
        .map_err(|e| store_error(&e))?
        .ok_or_else(missing)?;
    let record: Record = serde_json::from_slice(&bytes).map_err(|_| missing())?;
    Token::from_hex(&record.token).ok_or_else(missing)
}

/// Removes the token of the connector with key `spki`; nothing when there is none.
///
/// # Errors
/// The store's failure.
pub fn remove(store: &CredentialStore, spki: &SpkiHash) -> Result<(), CtlError> {
    store
        .remove(&record_name(spki))
        .map_err(|e| store_error(&e))
}
