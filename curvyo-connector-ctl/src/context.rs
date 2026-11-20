//! What a run of `curvyoctl` works with: its folder, the credential store (opened on first use)
//! and the computer's name (addendum D2, D12).

use std::path::PathBuf;

use curvyo_connector_core::MAX_CLIENT_NAME;
use curvyo_storage_io::CredentialStore;

use crate::error::CtlError;
use crate::known::KnownList;
use crate::secrets::open_store;

/// The folder, the store and the name of this computer.
pub struct Context {
    /// `curvyoctl`'s folder: `curvyoctl.toml` and, without a keychain, the key file.
    pub dir: PathBuf,
    /// This computer's host name.
    pub host_name: String,
    store: Option<CredentialStore>,
    open: fn(&std::path::Path) -> Result<CredentialStore, CtlError>,
}

/// The computer's host name; `CURVYOCTL_HOST_NAME` replaces it so that tests can run several
/// computers on one host.
fn host_name() -> String {
    if let Some(name) = std::env::var("CURVYOCTL_HOST_NAME")
        .ok()
        .filter(|n| !n.is_empty())
    {
        return name;
    }
    std::fs::read_to_string("/proc/sys/kernel/hostname")
        .ok()
        .map(|h| h.trim().to_owned())
        .filter(|h| !h.is_empty())
        .unwrap_or_else(|| "computer".to_owned())
}

impl Context {
    /// This computer's: `<config dir>/curvyoctl/`, the store opened when a command needs it.
    ///
    /// # Errors
    /// A refusal when the system has no config directory.
    pub fn system() -> Result<Self, CtlError> {
        let dir = dirs::config_dir()
            .ok_or_else(|| CtlError::refused("no config directory on this computer"))?
            .join("curvyoctl");
        Ok(Self {
            dir,
            host_name: host_name(),
            store: None,
            open: open_store,
        })
    }

    /// A context with a given folder, host name and store (tests).
    #[must_use]
    pub fn with_store(dir: PathBuf, host_name: &str, store: CredentialStore) -> Self {
        Self {
            dir,
            host_name: host_name.to_owned(),
            store: Some(store),
            open: open_store,
        }
    }

    /// A context on a computer without a credential store (tests).
    #[must_use]
    pub fn without_store(dir: PathBuf, host_name: &str) -> Self {
        Self {
            dir,
            host_name: host_name.to_owned(),
            store: None,
            open: |_| Err(CtlError::refused("no credential store on this computer")),
        }
    }

    /// The credential store, opened on first use.
    ///
    /// # Errors
    /// "no credential store on this computer", or the store's failure.
    pub fn store(&mut self) -> Result<&CredentialStore, CtlError> {
        if self.store.is_none() {
            self.store = Some((self.open)(&self.dir)?);
        }
        self.store
            .as_ref()
            .ok_or_else(|| CtlError::refused("no credential store on this computer"))
    }

    /// The list of known connectors.
    #[must_use]
    pub fn known(&self) -> KnownList {
        KnownList::new(&self.dir)
    }

    /// `curvyoctl@<host name>`, cut to the client name limit (UX notes V6, A16).
    #[must_use]
    pub fn client_name(&self) -> String {
        let name = format!("curvyoctl@{}", self.host_name);
        name.chars()
            .filter(|c| !c.is_control())
            .take(MAX_CLIENT_NAME)
            .collect()
    }
}
