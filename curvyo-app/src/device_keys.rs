//! The one command pair that reads and writes a shared document's keys (ADR 0031 decision 12):
//! this device's identity and the document's epoch keys, kept in the credential store of
//! `curvyo-storage-io` and handed to the frontend's wasm session when a shared document opens.
//!
//! Nothing here parses a key: the bytes cross as hex. The passphrase of the key-file fallback is
//! asked only when the system has no keychain (ADR 0007 §1: told, never a silent downgrade).

use std::path::PathBuf;
use std::sync::Mutex;

use curvyo_storage_io::{
    CredentialError, CredentialStore, credential_slot, decode_room_keys, encode_room_keys,
};
use tauri::{AppHandle, Manager, State};

/// The store, once opened in this run (the passphrase is asked once).
#[derive(Default)]
pub(crate) struct OpenStore(Mutex<Option<CredentialStore>>);

/// What the frontend gets back.
#[derive(serde::Serialize)]
#[serde(tag = "status", rename_all = "kebab-case")]
pub(crate) enum KeysAnswer {
    /// The identity (128 hex digits) and the room's epoch keys (`[epoch, hex]`).
    Ok {
        identity: String,
        keys: Vec<(u32, String)>,
    },
    /// No keychain: the maker sets or enters the key file's passphrase.
    NeedsPassphrase { exists: bool },
    /// The passphrase does not open the key file.
    WrongPassphrase,
    /// Anything else, in words.
    Failed { message: String },
}

fn data_dir(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map_err(|error| format!("the data folder is not available ({error})"))
}

fn hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    bytes
        .iter()
        .flat_map(|b| [HEX[usize::from(b >> 4)], HEX[usize::from(b & 15)]])
        .map(char::from)
        .collect()
}

fn unhex(text: &str) -> Option<Vec<u8>> {
    if !text.len().is_multiple_of(2) {
        return None;
    }
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(text.get(i..i + 2)?, 16).ok())
        .collect()
}

/// The room's keys as stored (`curvyo_storage_io::decode_room_keys`), as `[epoch, hex]`; none
/// for a value of another version.
fn decode_keys(bytes: &[u8]) -> Vec<(u32, String)> {
    decode_room_keys(bytes)
        .unwrap_or_default()
        .iter()
        .map(|(epoch, key)| (*epoch, hex(key.as_ref())))
        .collect()
}

/// Opens the store: the keychain if there is one, else the key file with `passphrase`.
fn open_store(
    dir: &std::path::Path,
    passphrase: Option<&str>,
) -> Result<CredentialStore, KeysAnswer> {
    let failed = |e: CredentialError| KeysAnswer::Failed {
        message: e.to_string(),
    };
    let slot = credential_slot(dir).map_err(failed)?;
    match CredentialStore::keychain(&slot) {
        Ok(store) => Ok(store),
        Err(_) => match passphrase.filter(|p| !p.is_empty()) {
            None => Err(KeysAnswer::NeedsPassphrase {
                exists: CredentialStore::file_exists(dir),
            }),
            Some(passphrase) => CredentialStore::file(dir, passphrase).map_err(|e| match e {
                CredentialError::WrongPassphrase => KeysAnswer::WrongPassphrase,
                other => failed(other),
            }),
        },
    }
}

/// The answer for `room` (hex, empty for none) from an opened store; the identity is created on
/// first use from the system's random source.
fn answer(store: &CredentialStore, room: &str) -> Result<KeysAnswer, CredentialError> {
    let identity = hex(store.device_identity()?.as_ref());
    let keys = if room.is_empty() {
        Vec::new()
    } else {
        store
            .get(&format!("room-{room}"))?
            .map(|b| decode_keys(&b))
            .unwrap_or_default()
    };
    Ok(KeysAnswer::Ok { identity, keys })
}

/// This device's identity and the epoch keys of `room` (hex room id, empty for none).
/// `passphrase` is only read when the system has no keychain.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn read_device_keys(
    app: AppHandle,
    open: State<'_, OpenStore>,
    room: String,
    passphrase: Option<String>,
) -> KeysAnswer {
    let Ok(mut guard) = open.0.lock() else {
        return KeysAnswer::Failed {
            message: "the credential store is busy".to_owned(),
        };
    };
    if guard.is_none() {
        let dir = match data_dir(&app) {
            Ok(dir) => dir,
            Err(message) => return KeysAnswer::Failed { message },
        };
        match open_store(&dir, passphrase.as_deref()) {
            Ok(store) => *guard = Some(store),
            Err(answer) => return answer,
        }
    }
    let Some(store) = guard.as_ref() else {
        return KeysAnswer::Failed {
            message: "the credential store is not open".to_owned(),
        };
    };
    answer(store, &room).unwrap_or_else(|e| KeysAnswer::Failed {
        message: e.to_string(),
    })
}

/// Stores the epoch keys of `room` (`[epoch, hex]`), replacing what was there.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn write_document_keys(
    open: State<'_, OpenStore>,
    room: String,
    keys: Vec<(u32, String)>,
) -> Result<(), String> {
    let guard = open
        .0
        .lock()
        .map_err(|_| "the credential store is busy".to_owned())?;
    let store = guard
        .as_ref()
        .ok_or_else(|| "the credential store is not open".to_owned())?;
    let mut parsed = Vec::with_capacity(keys.len());
    for (epoch, key) in &keys {
        let key: [u8; 32] = unhex(key)
            .and_then(|k| k.try_into().ok())
            .ok_or("not a key")?;
        parsed.push((*epoch, key));
    }
    let bytes = encode_room_keys(&parsed);
    if room.is_empty() || !room.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("not a room".to_owned());
    }
    store
        .set(&format!("room-{room}"), &bytes)
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_identity_is_made_once_and_room_keys_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let store = CredentialStore::file(dir.path(), "pass").unwrap();
        let KeysAnswer::Ok { identity, keys } = answer(&store, "ab").unwrap() else {
            panic!("keys");
        };
        assert_eq!(identity.len(), 128);
        assert_eq!(keys, Vec::<(u32, String)>::new());
        store
            .set("room-ab", &encode_room_keys(&[(3, [9; 32])]))
            .unwrap();
        let KeysAnswer::Ok {
            identity: again,
            keys,
        } = answer(&store, "ab").unwrap()
        else {
            panic!("keys");
        };
        assert_eq!(again, identity, "the same device");
        assert_eq!(keys, vec![(3, "09".repeat(32))]);
    }
}
