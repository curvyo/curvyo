//! The access token of a Git catalogue and its place in the credential store (ADR 0090): parsed,
//! never printed, kept only under `git-token:<address>`.

use std::fmt;

use zeroize::Zeroizing;

use crate::credentials::{CredentialError, CredentialStore};

/// The longest token text (it fits Windows' 2560-byte credential blob).
const MAX_TOKEN_CHARS: usize = 1024;
/// The key store account prefix; the address that follows is not secret.
const ACCOUNT_PREFIX: &str = "git-token:";

/// Why a text is not a token.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("a token is 1 to 1024 visible characters without spaces")]
pub struct TokenError;

/// A read-only access token. `Debug` hides it, there is no `Display`, `Clone` or `Serialize`, and
/// its memory is wiped when it is dropped.
pub struct AccessToken(Zeroizing<String>);

impl fmt::Debug for AccessToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("AccessToken(<redacted>)")
    }
}

impl AccessToken {
    /// The token in `text`: outer white space is dropped, then 1 to 1024 visible ASCII characters
    /// (`0x21..=0x7E`) must remain, so it can never break out of a header.
    ///
    /// # Errors
    /// [`TokenError`] for an empty, too long or not plain text.
    pub fn parse(text: &str) -> Result<Self, TokenError> {
        let text = text.trim();
        let plain = !text.is_empty()
            && text.len() <= MAX_TOKEN_CHARS
            && text.bytes().all(|b| (0x21..=0x7E).contains(&b));
        plain
            .then(|| Self(Zeroizing::new(text.to_owned())))
            .ok_or(TokenError)
    }

    /// The `Authorization` header's value; only the transport reads it.
    pub(crate) fn bearer(&self) -> Zeroizing<String> {
        Zeroizing::new(format!("Bearer {}", self.0.as_str()))
    }
}

fn account(address: &str) -> String {
    format!("{ACCOUNT_PREFIX}{address}")
}

/// The entry the OS key store lists for a source: `(account, service)`, shown to the maker when a
/// delete failed (`specs/0157` criterion 7): the account and the service `curvyo/<slot>`.
#[must_use]
pub fn token_label(address: &str, slot: &str) -> (String, String) {
    (account(address), format!("curvyo/{slot}"))
}

/// The token kept for the source at `address`, `None` when there is none.
///
/// # Errors
/// [`CredentialError`] when the store cannot be read; a stored value that is not a token reads as
/// none.
pub fn load_token(
    store: &CredentialStore,
    address: &str,
) -> Result<Option<AccessToken>, CredentialError> {
    Ok(store.get(&account(address))?.and_then(|bytes| {
        std::str::from_utf8(&bytes)
            .ok()
            .and_then(|text| AccessToken::parse(text).ok())
    }))
}

/// Keeps `token` for the source at `address`, replacing an older one.
///
/// # Errors
/// [`CredentialError`] when the store refuses.
pub fn save_token(
    store: &CredentialStore,
    address: &str,
    token: &AccessToken,
) -> Result<(), CredentialError> {
    store.set(&account(address), token.0.as_bytes())
}

/// Deletes the token of the source at `address`; nothing happens when there is none.
///
/// # Errors
/// [`CredentialError`] when the store refuses.
pub fn forget_token(store: &CredentialStore, address: &str) -> Result<(), CredentialError> {
    store.remove(&account(address))
}

#[cfg(test)]
mod tests {
    use super::*;

    const ADDRESS: &str = "https://example.org/cat/";

    #[test]
    fn a_token_is_one_to_1024_visible_characters_after_its_outer_space_is_dropped() {
        assert!(AccessToken::parse("  github_pat_AbC-123_x\n").is_ok());
        for bad in [
            "",
            "   ",
            "two words",
            "a\r\nX-Evil: 1",
            "tab\tinside",
            "tökén",
            &"x".repeat(1025),
        ] {
            assert_eq!(AccessToken::parse(bad).err(), Some(TokenError), "{bad:?}");
        }
        assert!(AccessToken::parse(&"x".repeat(1024)).is_ok());
    }

    #[test]
    fn debug_never_shows_the_token() {
        let token = AccessToken::parse("SENTINEL-secret-123").unwrap();
        assert_eq!(format!("{token:?}"), "AccessToken(<redacted>)");
        assert!(!format!("{token:#?}").contains("SENTINEL"));
        assert_eq!(token.bearer().as_str(), "Bearer SENTINEL-secret-123");
    }

    #[test]
    fn save_load_replace_and_forget_on_the_key_file_double() {
        let dir = tempfile::tempdir().unwrap();
        let store = CredentialStore::file(dir.path(), "pass").unwrap();
        assert!(load_token(&store, ADDRESS).unwrap().is_none());
        save_token(&store, ADDRESS, &AccessToken::parse("one").unwrap()).unwrap();
        save_token(&store, ADDRESS, &AccessToken::parse("two").unwrap()).unwrap();
        assert_eq!(
            load_token(&store, ADDRESS)
                .unwrap()
                .unwrap()
                .bearer()
                .as_str(),
            "Bearer two"
        );
        assert!(
            load_token(&store, "https://other.example/")
                .unwrap()
                .is_none(),
            "one token per address"
        );
        forget_token(&store, ADDRESS).unwrap();
        forget_token(&store, ADDRESS).unwrap();
        assert!(load_token(&store, ADDRESS).unwrap().is_none());
    }

    #[test]
    fn the_label_names_the_entry_the_key_store_lists() {
        assert_eq!(
            token_label(ADDRESS, "abc"),
            (format!("git-token:{ADDRESS}"), "curvyo/abc".to_owned())
        );
    }
}
