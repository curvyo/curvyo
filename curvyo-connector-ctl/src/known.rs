//! `curvyoctl.toml`: the connectors this computer knows, one entry per address with its name and
//! its pinned key; never a token (A24, A36; addendum D12). `0600` in a `0700` folder.

use std::path::{Path, PathBuf};

use curvyo_connector_io::spki::SpkiHash;
use serde::{Deserialize, Serialize};

use crate::error::CtlError;

/// The file name in `curvyoctl`'s folder.
pub const KNOWN_FILE: &str = "curvyoctl.toml";
const FORMAT: u32 = 1;

/// One known connector.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Known {
    /// The connector's configured name: the handle in commands.
    pub name: String,
    /// Its address as dialled, `host:port`.
    pub address: String,
    /// The full pin: SHA-256 of its key, 64 hex.
    pub spki_sha256: String,
}

impl Known {
    /// The pinned key.
    ///
    /// # Errors
    /// A refusal naming the file when the entry is damaged.
    pub fn spki(&self) -> Result<SpkiHash, CtlError> {
        SpkiHash::from_hex(&self.spki_sha256).ok_or_else(|| {
            CtlError::refused(format!("{KNOWN_FILE}: the key of {} is damaged", self.name))
        })
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    format: u32,
    #[serde(default)]
    connectors: Vec<Known>,
}

/// The list in one folder.
#[derive(Debug, Clone)]
pub struct KnownList {
    path: PathBuf,
}

fn io_error(path: &Path, error: &std::io::Error) -> CtlError {
    CtlError::refused(format!("{}: {error}", path.display()))
}

impl KnownList {
    /// The list in `dir`.
    #[must_use]
    pub fn new(dir: &Path) -> Self {
        Self {
            path: dir.join(KNOWN_FILE),
        }
    }

    /// Every entry; none without a file.
    ///
    /// # Errors
    /// A refusal for an unreadable or damaged file.
    pub fn list(&self) -> Result<Vec<Known>, CtlError> {
        let text = match std::fs::read_to_string(&self.path) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(io_error(&self.path, &e)),
        };
        let file: File = toml::from_str(&text)
            .map_err(|e| CtlError::refused(format!("{KNOWN_FILE}: {}", e.message().trim())))?;
        if file.format != FORMAT {
            return Err(CtlError::refused(format!(
                "{KNOWN_FILE}: format {} is not {FORMAT}",
                file.format
            )));
        }
        Ok(file.connectors)
    }

    fn save(&self, connectors: Vec<Known>) -> Result<(), CtlError> {
        let text = toml::to_string(&File {
            format: FORMAT,
            connectors,
        })
        .map_err(|e| CtlError::refused(format!("{KNOWN_FILE}: {e}")))?;
        write_private(&self.path, text.as_bytes()).map_err(|e| io_error(&self.path, &e))
    }

    /// Adds `entry`, replacing the entry of its address.
    ///
    /// # Errors
    /// A refusal when the file cannot be read or written.
    pub fn put(&self, entry: Known) -> Result<(), CtlError> {
        let mut list = self.list()?;
        list.retain(|k| k.address != entry.address);
        list.push(entry);
        self.save(list)
    }

    /// Removes the entry of `address`.
    ///
    /// # Errors
    /// A refusal when the file cannot be read or written.
    pub fn remove(&self, address: &str) -> Result<(), CtlError> {
        let mut list = self.list()?;
        list.retain(|k| k.address != address);
        self.save(list)
    }
}

/// Writes `bytes` to `path` (`0600`) through a temporary file, its folder made `0700`.
///
/// # Errors
/// The I/O error.
pub fn write_private(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write as _;
    if let Some(dir) = path.parent() {
        make_private_dir(dir)?;
    }
    let temp = path.with_extension("tmp");
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(0o600);
    }
    let mut file = options.open(&temp)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    std::fs::rename(&temp, path)
}

/// Makes `dir` and its parents; `dir` itself is `0700`, also when it existed before.
///
/// # Errors
/// The I/O error.
pub fn make_private_dir(dir: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::{DirBuilderExt as _, PermissionsExt as _};
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(dir)?;
        std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))
    }
    #[cfg(not(unix))]
    {
        std::fs::create_dir_all(dir)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a24_the_list_matches_its_golden_and_is_private() {
        let dir = tempfile::tempdir().unwrap();
        let folder = dir.path().join("curvyoctl");
        let list = KnownList::new(&folder);
        assert_eq!(list.list().unwrap(), Vec::new());
        let entry = |name: &str, address: &str, key: &str| Known {
            name: name.into(),
            address: address.into(),
            spki_sha256: key.repeat(32),
        };
        list.put(entry("workshop-laser", "192.168.1.20:7447", "ab"))
            .unwrap();
        list.put(entry("garage-plotter", "192.168.1.21:7447", "cd"))
            .unwrap();
        list.put(entry("workshop-laser", "192.168.1.20:7447", "ef"))
            .unwrap();
        let text = std::fs::read_to_string(folder.join(KNOWN_FILE)).unwrap();
        assert_eq!(text, include_str!("../tests/fixtures/curvyoctl.toml"));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            let mode = |p: &Path| std::fs::metadata(p).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode(&folder), 0o700);
            assert_eq!(mode(&folder.join(KNOWN_FILE)), 0o600);
        }
        list.remove("192.168.1.21:7447").unwrap();
        assert_eq!(list.list().unwrap().len(), 1);
    }
}
