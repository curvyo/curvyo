//! The credential store of this device's keys (ADR 0007 §1, ADR 0031 decision 12): the operating
//! system's keychain where one is available, otherwise an encrypted file under a passphrase the
//! maker sets, never a plaintext file and never a silent downgrade.
//!
//! Each installation of the app keeps its secrets under its own slot, a random id kept in its data
//! directory, so two instances with their own data directories are two devices on one machine
//! (`specs/0092-cooperative-editing`, "Acceptance criteria").

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use chacha20poly1305::aead::{AeadInOut, KeyInit};
use chacha20poly1305::{Key, Tag, XChaCha20Poly1305, XNonce};
use zeroize::Zeroizing;

use crate::filesystem::{read_optional, write_atomic};

/// The first bytes of the encrypted key file.
const FILE_MAGIC: &[u8; 8] = b"CVYKEYS1";
/// The key file format this build writes and reads (ADR 0031 decision 12).
const FILE_FORMAT: u8 = 1;
/// Argon2id memory cost in KiB, a fixed constant of this build (never the crate's default).
const ARGON2_MEMORY_KIB: u32 = 19_456;
/// Argon2id passes.
const ARGON2_PASSES: u32 = 2;
/// Argon2id lanes.
const ARGON2_LANES: u32 = 1;
/// The largest memory cost a key file may ask for (1 GiB), so a damaged header cannot exhaust memory.
const ARGON2_MAX_MEMORY_KIB: u32 = 1 << 20;
/// The largest pass count a key file may ask for.
const ARGON2_MAX_PASSES: u32 = 64;
/// The largest lane count a key file may ask for.
const ARGON2_MAX_LANES: u32 = 16;
/// `format u8, version u32, m_cost u32, t_cost u32, p_cost u32`.
const PARAMS_LEN: usize = 1 + 4 * 4;
const SALT_LEN: usize = 16;
/// Magic, parameters and salt: the authenticated header in front of the nonce.
const HEADER_LEN: usize = FILE_MAGIC.len() + PARAMS_LEN + SALT_LEN;
const NONCE_LEN: usize = 24;
const TAG_LEN: usize = 16;
/// The name of the slot file in the data directory.
const SLOT_FILE: &str = "credential-slot";
/// The name of the encrypted key file in the data directory.
pub const KEY_FILE: &str = "device-keys.bin";

/// Why the credential store could not do what was asked.
#[derive(Debug, thiserror::Error)]
pub enum CredentialError {
    /// This system has no usable keychain; the maker may set a passphrase for the key file.
    #[error("this computer has no credential store")]
    NoKeychain,
    /// The passphrase does not open the key file.
    #[error("the passphrase does not open the key file")]
    WrongPassphrase,
    /// The key file is not one this build wrote.
    #[error("the key file is damaged")]
    Damaged,
    /// The keychain refused.
    #[error("the credential store failed: {0}")]
    Keychain(String),
    /// Reading or writing a file failed.
    #[error(transparent)]
    Fs(#[from] crate::FsError),
    /// No random bytes could be drawn.
    #[error("no random bytes are available")]
    Random,
}

/// Where this device's secrets are kept.
pub enum CredentialStore {
    /// The operating system's keychain, under one service name per installation.
    Keychain {
        /// `curvyo/<slot>`.
        service: String,
    },
    /// The encrypted file in the data directory.
    File {
        /// The file.
        path: PathBuf,
        /// The key derived from the passphrase.
        key: Zeroizing<[u8; 32]>,
        /// The Argon2id parameters it was derived with, as the file's header holds them.
        params: KdfParams,
        /// The salt it was derived with.
        salt: [u8; SALT_LEN],
    },
}

/// The Argon2id parameters written into and read back from the key file's header.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KdfParams {
    /// Memory cost in KiB.
    memory_kib: u32,
    /// Passes.
    passes: u32,
    /// Lanes.
    lanes: u32,
}

impl KdfParams {
    /// The parameters this build writes into a new key file.
    const CURRENT: Self = Self {
        memory_kib: ARGON2_MEMORY_KIB,
        passes: ARGON2_PASSES,
        lanes: ARGON2_LANES,
    };

    /// `format u8 = 1, version u32 = 0x13, m_cost u32, t_cost u32, p_cost u32`, little-endian.
    fn encode(self) -> [u8; PARAMS_LEN] {
        let mut out = [0; PARAMS_LEN];
        out[0] = FILE_FORMAT;
        for (i, value) in [0x13, self.memory_kib, self.passes, self.lanes]
            .into_iter()
            .enumerate()
        {
            out[1 + 4 * i..5 + 4 * i].copy_from_slice(&value.to_le_bytes());
        }
        out
    }

    /// The parameters of a header this build reads; `None` for another format, another Argon2
    /// version or costs out of bounds.
    fn decode(bytes: &[u8]) -> Option<Self> {
        let word = |i: usize| -> Option<u32> {
            Some(u32::from_le_bytes(
                bytes.get(1 + 4 * i..5 + 4 * i)?.try_into().ok()?,
            ))
        };
        if *bytes.first()? != FILE_FORMAT || word(0)? != 0x13 {
            return None;
        }
        let params = Self {
            memory_kib: word(1)?,
            passes: word(2)?,
            lanes: word(3)?,
        };
        (params.memory_kib <= ARGON2_MAX_MEMORY_KIB
            && (1..=ARGON2_MAX_PASSES).contains(&params.passes)
            && (1..=ARGON2_MAX_LANES).contains(&params.lanes))
        .then_some(params)
    }
}

const HEX: &[u8; 16] = b"0123456789abcdef";

/// 16 random bytes as hex.
fn random_hex() -> Result<String, CredentialError> {
    let mut bytes = [0; 16];
    getrandom::fill(&mut bytes).map_err(|_| CredentialError::Random)?;
    Ok(bytes
        .iter()
        .flat_map(|b| [HEX[usize::from(b >> 4)], HEX[usize::from(b & 15)]])
        .map(char::from)
        .collect())
}

/// This installation's slot id, created on first use.
///
/// # Errors
/// `Fs` when the slot file cannot be read or written; `Random`.
pub fn credential_slot(data_dir: &Path) -> Result<String, CredentialError> {
    let path = data_dir.join(SLOT_FILE);
    if let Some(bytes) = read_optional(&path)? {
        let text = String::from_utf8_lossy(&bytes).trim().to_owned();
        if text.len() == 32 && text.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Ok(text);
        }
    }
    let slot = random_hex()?;
    crate::filesystem::ensure_parent_dir(&path)?;
    write_atomic(&path, slot.as_bytes())?;
    Ok(slot)
}

fn derive_key(
    passphrase: &str,
    params: KdfParams,
    salt: &[u8; SALT_LEN],
) -> Result<Zeroizing<[u8; 32]>, CredentialError> {
    use argon2::{Algorithm, Argon2, Params, Version};
    let params = Params::new(params.memory_kib, params.passes, params.lanes, Some(32))
        .map_err(|_| CredentialError::Damaged)?;
    let mut key = Zeroizing::new([0; 32]);
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
        .hash_password_into(passphrase.as_bytes(), salt, key.as_mut())
        .map_err(|_| CredentialError::Damaged)?;
    Ok(key)
}

impl CredentialStore {
    /// The keychain under this installation's slot, if the system has one that answers.
    ///
    /// # Errors
    /// `NoKeychain` when there is none or it cannot be reached.
    pub fn keychain(slot: &str) -> Result<Self, CredentialError> {
        let service = format!("curvyo/{slot}");
        let probe =
            keyring::Entry::new(&service, "probe").map_err(|_| CredentialError::NoKeychain)?;
        match probe.get_secret() {
            Ok(_) | Err(keyring::Error::NoEntry) => Ok(Self::Keychain { service }),
            Err(_) => Err(CredentialError::NoKeychain),
        }
    }

    /// The encrypted key file in `data_dir`, opened with `passphrase`, or created with it when
    /// there is none.
    ///
    /// # Errors
    /// `WrongPassphrase`, `Damaged`, `Fs`, `Random`.
    pub fn file(data_dir: &Path, passphrase: &str) -> Result<Self, CredentialError> {
        let path = data_dir.join(KEY_FILE);
        if let Some(bytes) = read_optional(&path)? {
            if bytes.get(..FILE_MAGIC.len()) != Some(&FILE_MAGIC[..]) {
                return Err(CredentialError::Damaged);
            }
            let params =
                KdfParams::decode(&bytes[FILE_MAGIC.len()..]).ok_or(CredentialError::Damaged)?;
            let salt: [u8; SALT_LEN] = bytes
                .get(FILE_MAGIC.len() + PARAMS_LEN..HEADER_LEN)
                .and_then(|s| s.try_into().ok())
                .ok_or(CredentialError::Damaged)?;
            let store = Self::File {
                path,
                key: derive_key(passphrase, params, &salt)?,
                params,
                salt,
            };
            store.read_file()?;
            return Ok(store);
        }
        let mut salt = [0; SALT_LEN];
        getrandom::fill(&mut salt).map_err(|_| CredentialError::Random)?;
        let params = KdfParams::CURRENT;
        let store = Self::File {
            path,
            key: derive_key(passphrase, params, &salt)?,
            params,
            salt,
        };
        store.write_file(&BTreeMap::new())?;
        Ok(store)
    }

    /// Whether the encrypted key file exists in `data_dir`.
    #[must_use]
    pub fn file_exists(data_dir: &Path) -> bool {
        data_dir.join(KEY_FILE).is_file()
    }

    /// The secret called `name`, `None` when none is stored.
    ///
    /// # Errors
    /// `Keychain`, `WrongPassphrase`, `Damaged`, `Fs`.
    pub fn get(&self, name: &str) -> Result<Option<Zeroizing<Vec<u8>>>, CredentialError> {
        match self {
            Self::Keychain { service } => {
                let entry = keyring::Entry::new(service, name)
                    .map_err(|e| CredentialError::Keychain(e.to_string()))?;
                match entry.get_secret() {
                    Ok(secret) => Ok(Some(Zeroizing::new(secret))),
                    Err(keyring::Error::NoEntry) => Ok(None),
                    Err(e) => Err(CredentialError::Keychain(e.to_string())),
                }
            }
            Self::File { .. } => Ok(self.read_file()?.remove(name)),
        }
    }

    /// Stores `secret` under `name`, replacing what was there.
    ///
    /// # Errors
    /// `Keychain`, `WrongPassphrase`, `Damaged`, `Fs`, `Random`.
    pub fn set(&self, name: &str, secret: &[u8]) -> Result<(), CredentialError> {
        match self {
            Self::Keychain { service } => keyring::Entry::new(service, name)
                .and_then(|entry| entry.set_secret(secret))
                .map_err(|e| CredentialError::Keychain(e.to_string())),
            Self::File { .. } => {
                let mut all = self.read_file()?;
                all.insert(name.to_owned(), Zeroizing::new(secret.to_vec()));
                self.write_file(&all)
            }
        }
    }

    /// This device's 64 identity bytes (ADR 0010 §1), drawn from the system's random source and
    /// stored on first use.
    ///
    /// # Errors
    /// As [`CredentialStore::get`] and [`CredentialStore::set`].
    pub fn device_identity(&self) -> Result<Zeroizing<[u8; 64]>, CredentialError> {
        const NAME: &str = "device-identity";
        if let Some(bytes) = self.get(NAME)?
            && let Ok(identity) = <[u8; 64]>::try_from(bytes.as_slice())
        {
            return Ok(Zeroizing::new(identity));
        }
        let mut fresh = Zeroizing::new([0; 64]);
        getrandom::fill(fresh.as_mut()).map_err(|_| CredentialError::Random)?;
        self.set(NAME, fresh.as_ref())?;
        Ok(fresh)
    }

    fn read_file(&self) -> Result<BTreeMap<String, Zeroizing<Vec<u8>>>, CredentialError> {
        let Self::File {
            path,
            key,
            params,
            salt,
        } = self
        else {
            return Ok(BTreeMap::new());
        };
        let bytes = read_optional(path)?.ok_or(CredentialError::Damaged)?;
        let header = HEADER_LEN + NONCE_LEN;
        if bytes.len() < header + TAG_LEN || bytes[..HEADER_LEN] != file_header(*params, salt) {
            return Err(CredentialError::Damaged);
        }
        let nonce: [u8; NONCE_LEN] = bytes[HEADER_LEN..header]
            .try_into()
            .map_err(|_| CredentialError::Damaged)?;
        let (body, tag) = bytes[header..].split_at(bytes.len() - header - TAG_LEN);
        let tag = Tag::try_from(tag).map_err(|_| CredentialError::Damaged)?;
        let mut plain = Zeroizing::new(body.to_vec());
        XChaCha20Poly1305::new(&Key::from(**key))
            .decrypt_inout_detached(
                &XNonce::from(nonce),
                &bytes[..HEADER_LEN],
                plain.as_mut_slice().into(),
                &tag,
            )
            .map_err(|_| CredentialError::WrongPassphrase)?;
        decode_entries(&plain).ok_or(CredentialError::Damaged)
    }

    fn write_file(
        &self,
        entries: &BTreeMap<String, Zeroizing<Vec<u8>>>,
    ) -> Result<(), CredentialError> {
        let Self::File {
            path,
            key,
            params,
            salt,
        } = self
        else {
            return Ok(());
        };
        let mut nonce = [0; NONCE_LEN];
        getrandom::fill(&mut nonce).map_err(|_| CredentialError::Random)?;
        let mut out = file_header(*params, salt);
        let mut plain = encode_entries(entries);
        let tag = XChaCha20Poly1305::new(&Key::from(**key))
            .encrypt_inout_detached(&XNonce::from(nonce), &out, plain.as_mut_slice().into())
            .map_err(|_| CredentialError::Damaged)?;
        out.extend_from_slice(&nonce);
        out.extend_from_slice(&plain);
        out.extend_from_slice(&tag);
        crate::filesystem::ensure_parent_dir(path)?;
        write_atomic(path, &out)?;
        Ok(())
    }
}

/// `CVYKEYS1`, the parameters and the salt: the authenticated header of the key file.
fn file_header(params: KdfParams, salt: &[u8; SALT_LEN]) -> Vec<u8> {
    let mut out = Vec::with_capacity(HEADER_LEN);
    out.extend_from_slice(FILE_MAGIC);
    out.extend_from_slice(&params.encode());
    out.extend_from_slice(salt);
    out
}

/// `count u32, (name_len u16, name, len u32, bytes)*`, little-endian.
fn encode_entries(entries: &BTreeMap<String, Zeroizing<Vec<u8>>>) -> Zeroizing<Vec<u8>> {
    let mut out = Zeroizing::new(Vec::new());
    let count = u32::try_from(entries.len()).unwrap_or(u32::MAX);
    out.extend_from_slice(&count.to_le_bytes());
    for (name, secret) in entries {
        let name_len = u16::try_from(name.len()).unwrap_or(u16::MAX);
        out.extend_from_slice(&name_len.to_le_bytes());
        out.extend_from_slice(&name.as_bytes()[..usize::from(name_len)]);
        let len = u32::try_from(secret.len()).unwrap_or(u32::MAX);
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(secret);
    }
    out
}

fn decode_entries(bytes: &[u8]) -> Option<BTreeMap<String, Zeroizing<Vec<u8>>>> {
    let mut at = 0;
    let mut take = |n: usize| -> Option<&[u8]> {
        let part = bytes.get(at..at + n)?;
        at += n;
        Some(part)
    };
    let count = u32::from_le_bytes(take(4)?.try_into().ok()?);
    let mut entries = BTreeMap::new();
    for _ in 0..count {
        let name_len = u16::from_le_bytes(take(2)?.try_into().ok()?);
        let name = String::from_utf8(take(usize::from(name_len))?.to_vec()).ok()?;
        let len = u32::from_le_bytes(take(4)?.try_into().ok()?);
        let secret = take(usize::try_from(len).ok()?)?.to_vec();
        entries.insert(name, Zeroizing::new(secret));
    }
    Some(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_key_file_round_trips_and_refuses_another_passphrase() {
        let dir = tempfile::tempdir().unwrap();
        let store = CredentialStore::file(dir.path(), "correct horse").unwrap();
        assert!(store.get("device-identity").unwrap().is_none());
        store.set("device-identity", &[7; 64]).unwrap();
        store.set("room-ab", &[1, 2, 3]).unwrap();
        let again = CredentialStore::file(dir.path(), "correct horse").unwrap();
        assert_eq!(
            again.get("device-identity").unwrap().unwrap().as_slice(),
            &[7; 64]
        );
        assert_eq!(
            again.get("room-ab").unwrap().unwrap().as_slice(),
            &[1, 2, 3]
        );
        assert!(matches!(
            CredentialStore::file(dir.path(), "wrong"),
            Err(CredentialError::WrongPassphrase)
        ));
    }

    #[test]
    fn the_key_file_holds_no_secret_in_plain_text() {
        let dir = tempfile::tempdir().unwrap();
        let store = CredentialStore::file(dir.path(), "pass").unwrap();
        store.set("device-identity", b"ZQX-SECRET-MARKER").unwrap();
        let bytes = std::fs::read(dir.path().join(KEY_FILE)).unwrap();
        assert!(bytes.starts_with(FILE_MAGIC));
        assert!(!bytes.windows(10).any(|w| w == b"ZQX-SECRET"));
        assert!(!bytes.windows(15).any(|w| w == b"device-identity"));
    }

    #[test]
    fn a_damaged_key_file_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        CredentialStore::file(dir.path(), "pass")
            .unwrap()
            .set("a", b"b")
            .unwrap();
        let path = dir.path().join(KEY_FILE);
        let mut bytes = std::fs::read(&path).unwrap();
        let last = bytes.len() - 1;
        bytes[last] ^= 1;
        std::fs::write(&path, &bytes).unwrap();
        assert!(matches!(
            CredentialStore::file(dir.path(), "pass"),
            Err(CredentialError::WrongPassphrase)
        ));
        std::fs::write(&path, b"CVYKEYS1short").unwrap();
        assert!(matches!(
            CredentialStore::file(dir.path(), "pass"),
            Err(CredentialError::Damaged)
        ));
    }

    #[test]
    fn each_data_directory_has_its_own_stable_slot() {
        let one = tempfile::tempdir().unwrap();
        let two = tempfile::tempdir().unwrap();
        let a = credential_slot(one.path()).unwrap();
        assert_eq!(a, credential_slot(one.path()).unwrap());
        assert_ne!(a, credential_slot(two.path()).unwrap());
        assert_eq!(a.len(), 32);
    }
}
