//! `connector.toml`: the configuration on disk in TOML, read, validated and written (A16, A31;
//! addendum D8). Written to a temporary file and renamed; it is the commit point of a setup.

use std::path::{Path, PathBuf};

use curvyo_connector_core::{ConfigError, ConnectorConfig};

use crate::identity::{make_dir, write_private};

/// The file name in the state directory.
pub const CONFIG_FILE: &str = "connector.toml";

/// Why a configuration file could not be used.
#[derive(Debug, thiserror::Error)]
pub enum ConfigFileError {
    /// Reading or writing failed.
    #[error("{}: {source}", .path.display())]
    Io {
        /// The file.
        path: PathBuf,
        /// The cause.
        source: std::io::Error,
    },
    /// Not TOML of a configuration (syntax, an unknown or missing field).
    #[error("line {line}: {message}")]
    Syntax {
        /// The line the parser names, from 1.
        line: usize,
        /// The parser's words.
        message: String,
    },
    /// A field breaks a rule of A16.
    #[error(transparent)]
    Invalid(#[from] ConfigError),
}

/// Reads and validates a configuration from TOML text.
///
/// # Errors
/// [`ConfigFileError::Syntax`] or [`ConfigFileError::Invalid`].
pub fn parse(text: &str) -> Result<ConnectorConfig, ConfigFileError> {
    let config: ConnectorConfig = toml::from_str(text).map_err(|e| {
        let at = e.span().map_or(0, |s| s.start);
        let line = text
            .get(..at)
            .map_or(1, |head| head.matches('\n').count() + 1);
        ConfigFileError::Syntax {
            line,
            message: e.message().trim().to_owned(),
        }
    })?;
    config.validate()?;
    Ok(config)
}

/// The configuration as TOML, as `connector.toml` holds it and `curvyoctl get-config` prints it.
///
/// # Panics
/// Never: a configuration is strings, numbers and one table.
#[must_use]
pub fn to_text(config: &ConnectorConfig) -> String {
    // invariant: a configuration is strings, numbers and one table, which TOML always writes.
    #[allow(clippy::expect_used)]
    toml::to_string(config).expect("a configuration is TOML")
}

/// The configuration in the state directory `dir`; `None` when there is no file (maintenance).
///
/// # Errors
/// [`ConfigFileError`]: an unreadable, damaged or invalid file refuses the start (D8).
pub fn load(dir: &Path) -> Result<Option<ConnectorConfig>, ConfigFileError> {
    let path = dir.join(CONFIG_FILE);
    match std::fs::read_to_string(&path) {
        Ok(text) => parse(&text).map(Some),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(ConfigFileError::Io { path, source }),
    }
}

/// Writes the configuration into `dir` (temporary file, sync, rename, sync of the directory).
///
/// # Errors
/// [`ConfigFileError::Io`].
pub fn store(dir: &Path, config: &ConnectorConfig) -> Result<(), ConfigFileError> {
    let path = dir.join(CONFIG_FILE);
    let io = |source| ConfigFileError::Io {
        path: path.clone(),
        source,
    };
    make_dir(dir).map_err(io)?;
    write_private(&path, to_text(config).as_bytes()).map_err(io)?;
    sync_dir(dir).map_err(io)
}

/// Syncs the directory entry, so a rename survives a power loss.
///
/// # Errors
/// The I/O error.
pub fn sync_dir(dir: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        std::fs::File::open(dir)?.sync_all()
    }
    #[cfg(not(unix))]
    {
        let _ = dir;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use curvyo_connector_core::ControllerConfig;

    fn fixture(name: &str) -> String {
        let path = format!(
            "{}/tests/fixtures/config/{name}",
            env!("CARGO_MANIFEST_DIR")
        );
        std::fs::read_to_string(path).unwrap()
    }

    #[test]
    fn a16_the_toml_spellings_match_their_goldens() {
        let grbl = parse(&fixture("connector_grbl.toml")).unwrap();
        assert_eq!(
            grbl.controller,
            ControllerConfig::GrblSerial {
                port: "/dev/serial/by-id/usb-1a86_USB_Serial-if00-port0".into(),
                baud: 115_200
            }
        );
        assert_eq!(to_text(&grbl), fixture("connector_grbl.toml"));
        let fake = parse(&fixture("connector_fake.toml")).unwrap();
        assert_eq!(to_text(&fake), fixture("connector_fake.toml"));
    }

    #[test]
    fn a_fault_names_the_line_or_the_field() {
        let text = "format = 1\nname = \"a\"\n[controller]\nkind = \"grbl-serial\"\nport = \"/dev/ttyUSB0\"\nbaud = 0\n";
        assert_eq!(
            parse(text).unwrap_err().to_string(),
            "controller.baud: 0 is not from 1 to 4,000,000"
        );
        let text = "format = 1\nname = \"a\"\ncolour = \"red\"\n[controller]\nkind = \"fake\"\n";
        match parse(text).unwrap_err() {
            ConfigFileError::Syntax { line, message } => {
                assert_eq!(line, 3, "{message}");
                assert!(message.contains("colour"), "{message}");
            }
            other => panic!("{other:?}"),
        }
        assert!(matches!(
            parse("name = [").unwrap_err(),
            ConfigFileError::Syntax { line: 1, .. }
        ));
    }

    #[test]
    fn a_stored_file_is_private_and_reads_back() {
        let dir = tempfile::tempdir().unwrap();
        assert!(load(dir.path()).unwrap().is_none());
        let config = parse(&fixture("connector_fake.toml")).unwrap();
        store(dir.path(), &config).unwrap();
        assert_eq!(load(dir.path()).unwrap(), Some(config));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            let mode = std::fs::metadata(dir.path().join(CONFIG_FILE))
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(mode & 0o777, 0o600);
        }
        std::fs::write(dir.path().join(CONFIG_FILE), "format = 1\n").unwrap();
        assert!(load(dir.path()).is_err());
    }
}
