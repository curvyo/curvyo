//! The connector's own key and self-signed certificate, kept in one `0600` file.

use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use rcgen::{CertificateParams, KeyPair, PKCS_ECDSA_P256_SHA256, PublicKeyData as _};
use rustls::server::ParsedCertificate;
use rustls_pki_types::pem::PemObject;
use rustls_pki_types::{CertificateDer, PrivatePkcs8KeyDer};
use time::{Duration, OffsetDateTime};

use crate::spki::SpkiHash;

/// The file name in the state directory.
pub const IDENTITY_FILE: &str = "identity.pem";
/// How long the certificate is valid (ADR 0035 §3).
const VALID_DAYS: i64 = 3650;

/// Why the identity file could not be made or used; every message names the file.
#[derive(Debug, thiserror::Error)]
pub enum IdentityError {
    /// Reading or writing failed.
    #[error("{}: {source}", .path.display())]
    Io {
        /// The file or directory.
        path: PathBuf,
        /// The cause.
        source: std::io::Error,
    },
    /// The file is not a key and a certificate that belong together.
    #[error("{}: damaged ({reason}); move it away to make a new key", .path.display())]
    Damaged {
        /// The file.
        path: PathBuf,
        /// What is wrong.
        reason: &'static str,
    },
    /// Others can read the file.
    #[error("{}: readable by others; run chmod 600 on it", .path.display())]
    TooOpen {
        /// The file.
        path: PathBuf,
    },
    /// A new key could not be made.
    #[error("cannot make a key: {0}")]
    Generate(String),
}

/// The key and certificate of one connector.
pub struct Identity {
    /// The certificate.
    pub certificate: CertificateDer<'static>,
    /// The private key.
    pub key: PrivatePkcs8KeyDer<'static>,
    /// SHA-256 of the key's public half, what apps pin.
    pub spki: SpkiHash,
}

impl std::fmt::Debug for Identity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Identity({})", self.spki.fingerprint())
    }
}

/// Reads `identity.pem` in `dir`, or makes it on the first start.
///
/// # Errors
/// [`IdentityError`] for a file that is unreadable, damaged, too open, or a key that does not
/// match its certificate: the connector never replaces such a file by itself.
pub fn load_or_create(dir: &Path) -> Result<Identity, IdentityError> {
    let path = dir.join(IDENTITY_FILE);
    match fs::read(&path) {
        Ok(bytes) => {
            check_mode(&path)?;
            parse(&path, &bytes)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => create(dir, &path),
        Err(source) => Err(IdentityError::Io { path, source }),
    }
}

fn parse(path: &Path, bytes: &[u8]) -> Result<Identity, IdentityError> {
    let damaged = |reason| IdentityError::Damaged {
        path: path.to_owned(),
        reason,
    };
    let certificate = CertificateDer::from_pem_slice(bytes)
        .map_err(|_| damaged("no certificate"))?
        .into_owned();
    let key = PrivatePkcs8KeyDer::from_pem_slice(bytes)
        .map_err(|_| damaged("no private key"))?
        .clone_key();
    let pair = KeyPair::try_from(key.secret_pkcs8_der()).map_err(|_| damaged("unreadable key"))?;
    let parsed =
        ParsedCertificate::try_from(&certificate).map_err(|_| damaged("unreadable certificate"))?;
    let from_certificate = parsed.subject_public_key_info();
    if from_certificate.as_ref() != pair.subject_public_key_info().as_slice() {
        return Err(damaged("the key does not match the certificate"));
    }
    Ok(Identity {
        spki: SpkiHash::of_spki_der(from_certificate.as_ref()),
        certificate,
        key,
    })
}

fn create(dir: &Path, path: &Path) -> Result<Identity, IdentityError> {
    let io = |path: &Path| {
        let path = path.to_owned();
        move |source| IdentityError::Io { path, source }
    };
    let generate = |e: rcgen::Error| IdentityError::Generate(e.to_string());
    make_dir(dir).map_err(io(dir))?;
    let pair = KeyPair::generate_for(&PKCS_ECDSA_P256_SHA256).map_err(generate)?;
    let mut params =
        CertificateParams::new(vec!["curvyo-connector".to_owned()]).map_err(generate)?;
    let now = OffsetDateTime::now_utc();
    params.not_before = now - Duration::days(1);
    params.not_after = now + Duration::days(VALID_DAYS);
    let certificate = params.self_signed(&pair).map_err(generate)?;
    let text = format!("{}{}", pair.serialize_pem(), certificate.pem());
    write_private(path, text.as_bytes()).map_err(io(path))?;
    parse(path, text.as_bytes())
}

#[cfg(unix)]
pub(crate) fn make_dir(dir: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::DirBuilderExt as _;
    fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(dir)
}

#[cfg(not(unix))]
pub(crate) fn make_dir(dir: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dir)
}

/// Writes `bytes` to a new `0600` temp file next to `path` and renames it over `path`.
#[cfg(unix)]
pub(crate) fn write_private(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::os::unix::fs::OpenOptionsExt as _;
    let temp = path.with_extension("tmp");
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(&temp)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    fs::rename(&temp, path)
}

#[cfg(not(unix))]
pub(crate) fn write_private(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let temp = path.with_extension("tmp");
    let mut file = fs::File::create(&temp)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    fs::rename(&temp, path)
}

#[cfg(unix)]
fn check_mode(path: &Path) -> Result<(), IdentityError> {
    use std::os::unix::fs::PermissionsExt as _;
    let mode = fs::metadata(path)
        .map_err(|source| IdentityError::Io {
            path: path.to_owned(),
            source,
        })?
        .permissions()
        .mode();
    if mode & 0o077 != 0 {
        return Err(IdentityError::TooOpen {
            path: path.to_owned(),
        });
    }
    Ok(())
}

#[cfg(not(unix))]
fn check_mode(_path: &Path) -> Result<(), IdentityError> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_start_makes_the_file_and_a_restart_keeps_the_key() {
        let dir = tempfile::tempdir().unwrap();
        let state = dir.path().join("state");
        let first = load_or_create(&state).unwrap();
        let again = load_or_create(&state).unwrap();
        assert_eq!(first.spki, again.spki);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            let mode = |p: &Path| fs::metadata(p).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode(&state), 0o700);
            assert_eq!(mode(&state.join(IDENTITY_FILE)), 0o600);
        }
    }

    #[test]
    fn a_damaged_file_refuses_the_start_and_names_itself() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(IDENTITY_FILE);
        let good = load_or_create(dir.path()).unwrap();
        let text = fs::read_to_string(&path).unwrap();
        let other = tempfile::tempdir().unwrap();
        load_or_create(other.path()).unwrap();
        let other_text = fs::read_to_string(other.path().join(IDENTITY_FILE)).unwrap();
        let cert_of = |t: &str| t[t.find("-----BEGIN CERTIFICATE").unwrap()..].to_owned();
        let key_of = |t: &str| t[..t.find("-----BEGIN CERTIFICATE").unwrap()].to_owned();
        let cases = [
            ("garbage".to_owned(), "no certificate"),
            (key_of(&text), "no certificate"),
            (cert_of(&text), "no private key"),
            (
                format!("{}{}", key_of(&text), cert_of(&other_text)),
                "does not match",
            ),
            (
                text.replace("BEGIN PRIVATE KEY", "BEGIN GARBAGE"),
                "damaged",
            ),
        ];
        for (content, expect) in cases {
            fs::write(&path, content).unwrap();
            let error = load_or_create(dir.path()).err().unwrap().to_string();
            assert!(error.contains(IDENTITY_FILE), "{error}");
            assert!(error.contains(expect), "{error}");
        }
        // The file is left as it was: no silent new key.
        assert_ne!(fs::read_to_string(&path).unwrap(), text);
        drop(good);
    }

    #[cfg(unix)]
    #[test]
    fn a_file_others_can_read_is_refused() {
        use std::os::unix::fs::PermissionsExt as _;
        let dir = tempfile::tempdir().unwrap();
        load_or_create(dir.path()).unwrap();
        let path = dir.path().join(IDENTITY_FILE);
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
        assert!(matches!(
            load_or_create(dir.path()),
            Err(IdentityError::TooOpen { .. })
        ));
    }
}
