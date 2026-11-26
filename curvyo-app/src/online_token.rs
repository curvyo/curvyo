//! The access token of a Git catalogue in the host (ADR 0090 decisions 3, 5 and 7): where the key
//! store is opened, how a request gets its header, and what Change token and Remove token do.
//! The token is read from the key store per request batch, never kept, never returned.

use curvyo_library_core::{Failure, SourceEntry, SourceKind, Sources, is_ready};
use curvyo_storage_io::{
    AccessToken, Authorized, CredentialError, CredentialStore, Transport, fetch_catalogue,
    forget_token, load_token, save_token, token_label,
};
use curvyo_ui_core::library_online::{
    Refusal, RefusalField, add_refusal, key_store_text, token_delete_failed_text,
    token_text_refusal,
};

use crate::online_state::OnlineState;

/// Opens the system key store (never the key file, ADR 0090 decision 3).
pub(crate) type KeyOpener = Box<dyn Fn() -> Result<CredentialStore, CredentialError> + Send + Sync>;

/// How the host reaches the key store: its opener and this installation's slot.
pub(crate) struct Keys {
    open: KeyOpener,
    slot: String,
}

impl Keys {
    pub(crate) fn new(slot: String, open: KeyOpener) -> Self {
        Self { open, slot }
    }
}

/// A refusal under the token field.
pub(crate) fn under_token(text: &str) -> Refusal {
    Refusal {
        field: RefusalField::Token,
        text: text.to_owned(),
    }
}

/// Whether a source may carry a token: a user-added Git catalogue (criterion 17).
pub(crate) fn may_have_token(entry: &SourceEntry) -> bool {
    entry.kind == SourceKind::Git && !is_ready(&entry.address)
}

impl OnlineState {
    /// Installs the key store this host uses; the first call wins.
    pub(crate) fn use_keys(&self, keys: Keys) {
        let _ = self.keys.set(keys);
    }

    /// The system key store, opened for this command.
    pub(crate) fn open_keys(&self) -> Result<CredentialStore, CredentialError> {
        self.keys
            .get()
            .map_or(Err(CredentialError::NoKeychain), |keys| (keys.open)())
    }

    /// Runs `run` with the transport `entry`'s requests go through: the plain one, or for a source
    /// with a token the one that adds it. A flagged source whose token cannot be read sends
    /// nothing (criterion 12, no silent anonymous request).
    pub(crate) fn sent<R>(
        &self,
        transport: &dyn Transport,
        entry: &SourceEntry,
        run: impl FnOnce(&dyn Transport) -> Result<R, Failure>,
    ) -> Result<R, Failure> {
        if !(entry.token && may_have_token(entry)) {
            return run(transport);
        }
        let token = self
            .open_keys()
            .ok()
            .and_then(|store| load_token(&store, &entry.address).ok().flatten())
            .ok_or(Failure::TokenUnreadable)?;
        run(&Authorized::new(transport, &token))
    }

    /// Change token or Add token (criteria 5, 6): the new token is checked with one GET of the
    /// index; only on success it replaces the stored one and the flag is set. A failure changes
    /// nothing, not even what this session holds of the source.
    pub(crate) fn set_token(
        &self,
        transport: &dyn Transport,
        sources: &mut Sources,
        key: &str,
        text: &str,
    ) -> Result<(), Refusal> {
        let entry = sources
            .find(key)
            .filter(|entry| may_have_token(entry))
            .cloned()
            .ok_or_else(|| under_token(key_store_text(true)))?;
        let token = AccessToken::parse(text).map_err(|_| under_token(token_text_refusal()))?;
        let store = self
            .open_keys()
            .map_err(|_| under_token(key_store_text(true)))?;
        let catalogue = fetch_catalogue(&Authorized::new(transport, &token), &entry.address)
            .map_err(|failure| add_refusal(SourceKind::Git, true, failure))?;
        save_token(&store, &entry.address, &token)
            .map_err(|_| under_token(key_store_text(true)))?;
        // invariant: the entry was found above and may carry a token.
        let _ = sources.set_token(key, true);
        self.install_catalogue(key, catalogue);
        Ok(())
    }

    /// Deletes the stored token of the source at `address`; the line of the Online group when the
    /// key store would not (criterion 7), `None` when it is gone.
    pub(crate) fn delete_token(&self, address: &str) -> Option<String> {
        let failed = self
            .open_keys()
            .and_then(|store| forget_token(&store, address))
            .is_err();
        failed.then(|| {
            let slot = self.keys.get().map_or("", |keys| keys.slot.as_str());
            let (account, service) = token_label(address, slot);
            token_delete_failed_text(&account, &service)
        })
    }
}
