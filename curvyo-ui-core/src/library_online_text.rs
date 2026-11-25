//! The words of the Online group (`specs/0138-template-library` UX notes 3 to 9).

use curvyo_library_core::{Failure, Licence, SourceKind};

/// What a row says when the source states no licence (ADR 0007 §9: never read as free).
pub const LICENCE_UNKNOWN: &str = "Licence unknown";

/// The notice when `online-sources.toml` was made by a newer Curvyo: Add and Remove are refused so
/// the file is never overwritten.
#[must_use]
pub const fn sources_newer_text() -> &'static str {
    "The online sources list was made by a newer Curvyo. Nothing was changed."
}

/// The credit of an icon whose set licence needs one: "home from Material by Pictogrammers, CC BY 4.0".
#[must_use]
pub fn icon_credit_text(name: &str, set: &str, author: &str, licence: &str) -> String {
    let by = if author.is_empty() {
        String::new()
    } else {
        format!(" by {author}")
    };
    format!("{name} from {set}{by}, {licence}")
}

/// "1 template", "4 templates": `count` and `noun` with the plural s unless it is one.
#[must_use]
pub fn counted(count: usize, noun: &str) -> String {
    format!("{count} {noun}{}", if count == 1 { "" } else { "s" })
}

/// The row a running Enter search shows: `Searching "home"...`.
#[must_use]
pub fn searching_row_text(words: &str) -> String {
    format!("Searching \"{words}\"...")
}

/// The announcement when one source's search ends (UX notes 7.2): "api.iconify.design: 48 icons for
/// home", "...: no icons for home", "...: Offline".
#[must_use]
pub fn search_done_text(source: &str, words: &str, outcome: Result<usize, Failure>) -> String {
    match outcome {
        Ok(0) => format!("{source}: no icons for {words}"),
        Ok(n) => format!("{source}: {n} icons for {words}"),
        Err(Failure::Offline) => format!("{source}: Offline"),
        Err(failure) => format!("{source}: {}", failure_text(SourceKind::Iconify, failure)),
    }
}

/// The words of a failure on a source's list (ADR 0085 decision 6).
#[must_use]
pub fn failure_text(kind: SourceKind, failure: Failure) -> String {
    match failure {
        Failure::Offline => "Offline".to_owned(),
        Failure::NotFound | Failure::Invalid => match kind {
            SourceKind::Git => "No Git catalogue at this address.".to_owned(),
            SourceKind::Iconify => "No Iconify API at this address.".to_owned(),
        },
        Failure::Status(code) => format!("The server answered {code}."),
        Failure::Newer => "Made for a newer Curvyo.".to_owned(),
        Failure::Mismatch => "The index does not match its hash.".to_owned(),
        Failure::TooLarge => "The answer is too large.".to_owned(),
        Failure::Redirect => "The server sent the request elsewhere.".to_owned(),
    }
}

/// The refusal of the Add source form (UX notes 3) for a failed first request.
#[must_use]
pub fn add_failure_text(kind: SourceKind, failure: Failure) -> String {
    match failure {
        Failure::Offline => "Offline. Nothing was sent.".to_owned(),
        Failure::NotFound | Failure::Invalid | Failure::Newer => {
            failure_text(kind, Failure::NotFound)
        }
        Failure::Status(_) | Failure::Redirect | Failure::TooLarge | Failure::Mismatch => {
            "The server did not answer.".to_owned()
        }
    }
}

/// What Add sends, said under the Address field (criteria 1 and 10, UX F6): a statement of what
/// leaves the machine.
#[must_use]
pub const fn add_sent_text(iconify: bool) -> &'static str {
    if iconify {
        "Add sends one request to this address. Enter in the library search sends your search words to it."
    } else {
        "Add sends one request to this address."
    }
}

/// The announcement of the polite region after Enter: "Searching 2 Iconify sources".
#[must_use]
pub fn searching_text(sources: usize) -> String {
    if sources == 1 {
        "Searching 1 Iconify source".to_owned()
    } else {
        format!("Searching {sources} Iconify sources")
    }
}

/// The refusal for an address the form cannot take.
#[must_use]
pub const fn add_address_text() -> &'static str {
    "Enter an address that starts with https://."
}

/// The refusal for an address that is already in the list.
#[must_use]
pub const fn add_duplicate_text() -> &'static str {
    "This address is already in the library."
}

/// The notice of a template that could not be loaded (UX notes 5): the reason, then "Nothing was
/// changed.".
#[must_use]
pub fn load_failure_text(name: &str, failure: Failure) -> String {
    let why = match failure {
        Failure::Mismatch => "does not match the catalogue.".to_owned(),
        Failure::Offline => "offline.".to_owned(),
        Failure::Newer => "made by a newer Curvyo.".to_owned(),
        Failure::TooLarge => "too large.".to_owned(),
        Failure::Status(code) => format!("the server answered {code}."),
        Failure::NotFound | Failure::Invalid | Failure::Redirect => "damaged.".to_owned(),
    };
    format!("{name} cannot be loaded: {why} Nothing was changed.")
}

pub(crate) fn refused_line(failure: Failure) -> String {
    match failure {
        Failure::Mismatch => "Cannot be loaded: does not match the catalogue.".to_owned(),
        Failure::Offline => "Offline".to_owned(),
        Failure::Newer => "Cannot be loaded: made by a newer Curvyo.".to_owned(),
        Failure::TooLarge => "Cannot be loaded: too large.".to_owned(),
        Failure::Status(code) => format!("The server answered {code}."),
        Failure::NotFound | Failure::Invalid | Failure::Redirect => {
            "Cannot be loaded: damaged.".to_owned()
        }
    }
}

/// "24 KB", "1.5 MB".
#[must_use]
pub fn size_text(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * 1024;
    if bytes < KB {
        format!("{bytes} B")
    } else if bytes < MB {
        format!("{} KB", bytes.div_ceil(KB))
    } else {
        // 0.1 MB steps, rounded up.
        let tenths = (bytes * 10).div_ceil(MB);
        format!("{}.{} MB", tenths / 10, tenths % 10)
    }
}

pub(crate) fn size_words(bytes: u64) -> String {
    let text = size_text(bytes);
    text.replace(" KB", " kilobytes")
        .replace(" MB", " megabytes")
        .replace(" B", " bytes")
}

pub(crate) fn icons_text(total: u64) -> String {
    let digits = total.to_string();
    let mut grouped = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            grouped.push(',');
        }
        grouped.push(c);
    }
    format!("{grouped} {}", if total == 1 { "icon" } else { "icons" })
}

/// The licence's name and address as a row states them; "Licence unknown" when the source states none.
pub(crate) fn licence_line(licence: Option<&Licence>) -> (String, String) {
    licence
        .map(|l| {
            (
                if l.name.is_empty() {
                    l.spdx.clone()
                } else {
                    l.name.clone()
                },
                l.url.clone(),
            )
        })
        .filter(|(name, _)| !name.trim().is_empty())
        .unwrap_or_else(|| (LICENCE_UNKNOWN.to_owned(), String::new()))
}
