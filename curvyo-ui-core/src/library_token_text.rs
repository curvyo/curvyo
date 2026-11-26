//! The words of a Git catalogue's access token (`specs/0157-library-git-token` criteria 1 to 16,
//! UX notes 2 to 7): what a refusal says and under which field, the failure line of a source, and
//! the fixed texts of the form. The token itself is never an input here.

use serde::Serialize;

use curvyo_library_core::{Failure, SourceKind};

/// The field a refusal of the Add source or token form is shown under.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RefusalField {
    /// Under Address (also the ones that are about neither field).
    Address,
    /// Under Access token.
    Token,
}

/// A refusal and the field it is about.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Refusal {
    /// Where to show it and where the focus goes.
    pub field: RefusalField,
    /// The sentence.
    pub text: String,
}

fn refused_status(failure: Failure) -> bool {
    matches!(failure, Failure::Status(401 | 403))
}

/// The failure line of a source's list (criterion 13, UX notes 5): with a token a 401, 403 or a 404
/// of the index is "The token was refused."; a source without one that is private says what it can.
#[must_use]
pub fn source_failure_text(kind: SourceKind, token: bool, failure: Failure) -> String {
    match (kind, token, failure) {
        (SourceKind::Git, true, Failure::NotFound | Failure::Status(401 | 403)) => {
            "The token was refused.".to_owned()
        }
        (_, _, Failure::TokenUnreadable) => {
            "The saved token could not be read from the key store.".to_owned()
        }
        (SourceKind::Git, false, Failure::Status(401 | 403)) => {
            "The server refused the request.".to_owned()
        }
        _ => crate::library_online_text::failure_text(kind, failure),
    }
}

/// Whether the failure line of a source is a token or access failure, so the token action stays
/// shown next to Refresh (criterion 13, UX notes 4). Only a Git catalogue has one.
#[must_use]
pub fn token_attention(kind: SourceKind, failure: Option<Failure>) -> bool {
    kind == SourceKind::Git
        && failure.is_some_and(|f| {
            f == Failure::TokenUnreadable || refused_status(f) || f == Failure::NotFound
        })
}

/// The refusal of the Add source or token form for a failed validating request (criteria 2, 3, 5,
/// 15): under the token field when it is about the token, else under Address.
#[must_use]
pub fn add_refusal(kind: SourceKind, token: bool, failure: Failure) -> Refusal {
    const HINT: &str = " For a private repository, add an access token.";
    let under_token = |text: &str| Refusal {
        field: RefusalField::Token,
        text: text.to_owned(),
    };
    let under_address = |text: String| Refusal {
        field: RefusalField::Address,
        text,
    };
    match (kind, token, failure) {
        (SourceKind::Git, true, Failure::Status(401)) => {
            under_token("The server refused the token.")
        }
        (SourceKind::Git, true, Failure::Status(403) | Failure::NotFound) => {
            under_token("This token has no read access to the repository.")
        }
        (SourceKind::Git, true, Failure::Redirect) => {
            under_token("The server sent the request elsewhere. The token was not sent there.")
        }
        (SourceKind::Git, false, Failure::NotFound) => under_address(format!(
            "{}{HINT}",
            crate::library_online_text::failure_text(kind, Failure::NotFound)
        )),
        (SourceKind::Git, false, Failure::Status(401 | 403)) => {
            under_address(format!("The server refused the request.{HINT}"))
        }
        _ => under_address(crate::library_online_text::add_failure_text(kind, failure)),
    }
}

/// The text refusal of a token the form will not send (criterion 3).
#[must_use]
pub const fn token_text_refusal() -> &'static str {
    "Enter a token of at most 1024 characters, without spaces."
}

/// The refusal when the key store is not there (criterion 11; UX notes T9: Change and Add token
/// end "Nothing was changed.").
#[must_use]
pub const fn key_store_text(changing: bool) -> &'static str {
    if changing {
        "Your system's key store is not available, so the token cannot be kept. Nothing was changed."
    } else {
        "Your system's key store is not available, so the token cannot be kept. The source was not added."
    }
}

/// The line of the Online group when a token could not be deleted from the key store (criterion
/// 7): the entry's account and service, as the OS key store lists them.
#[must_use]
pub fn token_delete_failed_text(account: &str, service: &str) -> String {
    format!(
        "The saved token could not be deleted from the key store; remove it there by hand: {account} in {service}."
    )
}

/// The fixed texts of the token field, the Lock and the token form, for the page to draw.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TokenWords {
    /// Label of the field in the Add source form.
    pub add_label: String,
    /// The statement under the field.
    pub statement: String,
    /// Tooltip line 3 of a source with a token.
    pub private_tooltip: String,
    /// Label of the field when a source has a token.
    pub new_label: String,
    /// Label of the field when it has none.
    pub label: String,
    /// The `KeyRound` action of a source with a token.
    pub change: String,
    /// The `KeyRound` action of a source without one.
    pub add: String,
    /// The Remove token button.
    pub remove: String,
    /// The armed Remove token button.
    pub remove_armed: String,
    /// The text refusal.
    pub bad_text: String,
}

/// The fixed texts.
#[must_use]
pub fn token_words() -> TokenWords {
    TokenWords {
        add_label: "Access token (optional)".to_owned(),
        statement: "The token is sent only to this address and kept in your system's key store."
            .to_owned(),
        private_tooltip: "Private, with a token".to_owned(),
        new_label: "New access token".to_owned(),
        label: "Access token".to_owned(),
        change: "Change token…".to_owned(),
        add: "Add token…".to_owned(),
        remove: "Remove token".to_owned(),
        remove_armed: "Press again to remove the token".to_owned(),
        bad_text: token_text_refusal().to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GIT: SourceKind = SourceKind::Git;

    #[test]
    fn the_add_form_words_each_refusal_under_the_field_it_is_about() {
        let r = |token, failure| add_refusal(GIT, token, failure);
        let token = |text: &str| Refusal {
            field: RefusalField::Token,
            text: text.to_owned(),
        };
        assert_eq!(
            r(true, Failure::Status(401)),
            token("The server refused the token.")
        );
        assert_eq!(
            r(true, Failure::Status(403)),
            token("This token has no read access to the repository.")
        );
        assert_eq!(
            r(true, Failure::NotFound),
            token("This token has no read access to the repository.")
        );
        assert_eq!(
            r(true, Failure::Redirect),
            token("The server sent the request elsewhere. The token was not sent there.")
        );
        assert_eq!(
            r(false, Failure::NotFound),
            Refusal {
                field: RefusalField::Address,
                text: "No Git catalogue at this address. For a private repository, add an access token."
                    .to_owned()
            }
        );
        for code in [401, 403] {
            assert_eq!(
                r(false, Failure::Status(code)).text,
                "The server refused the request. For a private repository, add an access token."
            );
        }
        // The 0138 texts stay for the rest, with or without a token.
        for with in [true, false] {
            assert_eq!(r(with, Failure::Offline).text, "Offline. Nothing was sent.");
            assert_eq!(r(with, Failure::Invalid).field, RefusalField::Address);
            assert_eq!(
                r(with, Failure::Status(500)).text,
                "The server did not answer."
            );
        }
        assert_eq!(
            add_refusal(SourceKind::Iconify, false, Failure::Status(401)).text,
            "The server did not answer.",
            "no hint for Iconify"
        );
        assert!(
            !add_refusal(SourceKind::Iconify, false, Failure::NotFound)
                .text
                .contains("token")
        );
    }

    #[test]
    fn a_failure_line_says_the_token_was_refused_only_for_a_source_with_one() {
        let line = |token, failure| source_failure_text(GIT, token, failure);
        for failure in [
            Failure::Status(401),
            Failure::Status(403),
            Failure::NotFound,
        ] {
            assert_eq!(line(true, failure), "The token was refused.");
        }
        assert_eq!(
            line(false, Failure::Status(401)),
            "The server refused the request."
        );
        assert_eq!(
            line(false, Failure::NotFound),
            "No Git catalogue at this address."
        );
        assert!(
            !line(false, Failure::NotFound).contains("private"),
            "no hint in the tree"
        );
        assert_eq!(
            line(true, Failure::TokenUnreadable),
            "The saved token could not be read from the key store."
        );
        assert_eq!(line(true, Failure::Offline), "Offline");
        assert_eq!(
            line(true, Failure::Redirect),
            "The server sent the request elsewhere."
        );
        assert_eq!(line(true, Failure::Status(500)), "The server answered 500.");
    }

    #[test]
    fn the_token_action_stays_shown_for_a_token_or_access_failure_of_a_git_catalogue() {
        assert!(token_attention(GIT, Some(Failure::Status(403))));
        assert!(token_attention(GIT, Some(Failure::TokenUnreadable)));
        assert!(token_attention(GIT, Some(Failure::NotFound)));
        assert!(!token_attention(GIT, Some(Failure::Redirect)));
        assert!(!token_attention(GIT, Some(Failure::Offline)));
        assert!(!token_attention(GIT, None));
        assert!(!token_attention(
            SourceKind::Iconify,
            Some(Failure::NotFound)
        ));
    }

    #[test]
    fn the_fixed_texts_are_the_spec_s() {
        assert_eq!(
            token_delete_failed_text("git-token:https://e.org/c/", "curvyo/abc"),
            "The saved token could not be deleted from the key store; remove it there by hand: git-token:https://e.org/c/ in curvyo/abc."
        );
        assert!(key_store_text(false).ends_with("The source was not added."));
        assert!(key_store_text(true).ends_with("Nothing was changed."));
        let words = token_words();
        assert_eq!(words.change, "Change token…");
        assert_eq!(words.private_tooltip, "Private, with a token");
    }
}
