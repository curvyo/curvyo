//! The rows of the Online group of the Library tab (`specs/0138-template-library`), built from what
//! the host fetched; nothing here fetches.

use std::collections::{HashMap, HashSet};

use serde::Serialize;

use curvyo_library_core::{
    Catalogue, Failure, IconRef, IconSet, READY_HOST, SetListing, SourceKind, host_of, is_ready,
};

use crate::library_index::IndexedDocument;
use crate::library_online_icons::push_iconify;
use crate::library_online_templates::push_templates;
pub use crate::library_online_text::*;
pub use crate::library_token_text::*;
use crate::library_tree::{LibraryRow, RowKind, plain_row, row_key};

/// The heading of the group.
pub const ONLINE_HEADING: &str = "Online";
/// The ready entry's name.
pub const READY_NAME: &str = "Curvyo catalogue";
/// The key of the ready entry's row.
pub const READY_KEY: &str = "r:ready";
/// Icon cells per row (UX notes 6).
pub const CELLS_PER_ROW: usize = 5;

/// One icon cell of an icon row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IconCell {
    /// The set's prefix.
    pub prefix: String,
    /// The icon's name.
    pub name: String,
    /// The accessible name: "home, icon, Material Symbols, licence Apache 2.0".
    pub label: String,
    /// The tooltip: the name, then the set and licence.
    pub tooltip: String,
}

/// What a source row shows of its access token (`specs/0157` criteria 4, 5, 13).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct TokenMark {
    /// The source has a token: the Lock mark after its name, from the first paint.
    pub private: bool,
    /// The token action's name ("Change token…", "Add token…"); empty for a source that cannot
    /// have a token (Iconify, the Curvyo catalogue).
    pub action: String,
    /// The failure line is a token or access failure: the action stays shown.
    pub attention: bool,
}

/// What an online row adds to a [`LibraryRow`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct OnlineExtra {
    /// `git` or `iconify` for a source row or its child; empty otherwise.
    pub glyph: String,
    /// The host the source's requests go to (tooltip line 2).
    pub host: String,
    /// The right-hand state of a source row: the count, "Offline", or empty.
    pub state: String,
    /// Whether Refresh is offered.
    pub refresh: bool,
    /// The Lock mark and the token action of a source row.
    pub token: TokenMark,
    /// A status row's icon: `loading`, `offline`, `failed` or `search`.
    pub status: String,
    /// A template's or set's author.
    pub author: String,
    /// The licence's name, "Licence unknown" when the source states none.
    pub licence: String,
    /// The licence's address; empty when none.
    pub licence_url: String,
    /// A template's size ("24 KB") or a set's icon count ("1,240 icons").
    pub size: String,
    /// "Font not included" when the template's fonts are withheld.
    pub font_note: String,
    /// Whether a template has a thumbnail to fetch.
    pub thumbnail: bool,
    /// Whether Load is offered.
    pub loadable: bool,
    /// The icon cells of an icon row.
    pub cells: Vec<IconCell>,
    /// Up to three sample icons of a set row.
    pub samples: Vec<IconCell>,
}

/// An Iconify search as sent and answered, for the results rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchView {
    /// The words as sent.
    pub words: String,
    /// The outcome; empty while `running`.
    pub outcome: Result<Vec<IconRef>, Failure>,
    /// Whether the request is still out (the results row says so).
    pub running: bool,
    /// The info of the sets the answer named.
    pub sets: Vec<IconSet>,
}

/// One online source with what this session holds of it.
pub struct OnlineSourceView<'a> {
    /// The source key (`online:<address>`).
    pub key: String,
    /// The name shown.
    pub name: &'a str,
    /// The base address.
    pub address: &'a str,
    /// Git catalogue or Iconify.
    pub kind: SourceKind,
    /// Whether the key store holds an access token for the source (criterion 4).
    pub token: bool,
    /// The failure of the source's last request of its list, if it failed.
    pub failure: Option<Failure>,
    /// The checked index, once fetched.
    pub catalogue: Option<&'a Catalogue>,
    /// The icon sets, once fetched.
    pub sets: Option<&'a [IconSet]>,
    /// The lists of the sets opened, by prefix.
    pub set_icons: &'a HashMap<String, SetListing>,
    /// The sets whose list could not be fetched, by prefix (`specs/0152` set cards).
    pub set_failures: &'a HashMap<String, Failure>,
    /// The last Enter search.
    pub search: Option<&'a SearchView>,
    /// The downloaded and read templates, by template path.
    pub templates: &'a HashMap<String, IndexedDocument>,
    /// The templates whose download or check failed, by template path.
    pub refused: &'a HashMap<String, Failure>,
}

/// What the tree shows of the Online group.
pub struct OnlineView<'a> {
    /// The added sources, in the order they were added.
    pub sources: Vec<OnlineSourceView<'a>>,
    /// Whether the ready Curvyo catalogue is offered (it is not an added source).
    pub ready_offered: bool,
    /// The expanded row keys.
    pub expanded: &'a HashSet<String>,
    /// The search text.
    pub query: &'a str,
}

pub(crate) fn status_row(
    source: &str,
    level: usize,
    key: String,
    text: &str,
    status: &str,
) -> LibraryRow {
    LibraryRow {
        online: Some(OnlineExtra {
            status: status.to_owned(),
            ..OnlineExtra::default()
        }),
        label: text.to_owned(),
        ..plain_row(key, RowKind::Status, level, text, source)
    }
}

/// The rows of the Online group: its heading, the ready entry or the added sources with what is
/// open under them. Empty only while searching and nothing matches.
#[must_use]
pub fn online_rows(view: &OnlineView<'_>) -> Vec<LibraryRow> {
    let searching = !view.query.trim().is_empty();
    let mut rows = vec![LibraryRow {
        key: format!("h:{ONLINE_HEADING}"),
        ..plain_row(String::new(), RowKind::Heading, 0, ONLINE_HEADING, "")
    }];
    if view.ready_offered && !searching {
        rows.push(ready_row());
    }
    for source in &view.sources {
        push_source(&mut rows, source, view);
    }
    if rows.len() == 1 {
        rows.clear();
    }
    rows
}

fn ready_row() -> LibraryRow {
    LibraryRow {
        online: Some(OnlineExtra {
            glyph: "ready".to_owned(),
            host: READY_HOST.to_owned(),
            ..OnlineExtra::default()
        }),
        label: format!("{READY_NAME}, Git catalogue, not added"),
        ..plain_row(READY_KEY.to_owned(), RowKind::Ready, 0, READY_NAME, "")
    }
}

pub(crate) fn kind_glyph(kind: SourceKind) -> &'static str {
    match kind {
        SourceKind::Git => "git",
        SourceKind::Iconify => "iconify",
    }
}

fn push_source(rows: &mut Vec<LibraryRow>, source: &OnlineSourceView<'_>, view: &OnlineView<'_>) {
    let key = row_key(RowKind::Source, &source.key, &[], None);
    let searching = !view.query.trim().is_empty();
    let mut body = Vec::new();
    let mut opened = view.expanded.contains(&key);
    match source.kind {
        SourceKind::Git => push_templates(&mut body, source, view),
        SourceKind::Iconify => push_iconify(&mut body, source, view),
    }
    if searching {
        // A source with nothing fetched is drawn collapsed with its name; one with matches or
        // results opens (UX notes 7).
        opened = body.iter().any(|r| r.kind != RowKind::Status) || source.search.is_some();
    }
    let host = host_of(source.address).unwrap_or_default();
    let (count_word, count) = match source.kind {
        SourceKind::Git => ("template", source.catalogue.map(|c| c.templates.len())),
        SourceKind::Iconify => ("icon set", source.sets.map(<[IconSet]>::len)),
    };
    let state = match (source.failure, count) {
        (Some(Failure::Offline), _) => "Offline".to_owned(),
        (_, Some(n)) => n.to_string(),
        _ => String::new(),
    };
    let kind_word = match source.kind {
        SourceKind::Git if source.token => "Git catalogue, private",
        SourceKind::Git => "Git catalogue",
        SourceKind::Iconify => "icon API",
    };
    let user_git = source.kind == SourceKind::Git && !is_ready(source.address);
    let words = token_words();
    let label = match (source.failure, count) {
        (Some(failure), _) => format!(
            "{}, {kind_word}, {}",
            source.name,
            source_failure_text(source.kind, source.token, failure).trim_end_matches('.')
        ),
        (None, Some(n)) => format!("{}, {kind_word}, {}", source.name, counted(n, count_word)),
        (None, None) => format!("{}, {kind_word}", source.name),
    };
    rows.push(LibraryRow {
        expandable: true,
        expanded: opened,
        removable: true,
        label,
        line: state.clone(),
        online: Some(OnlineExtra {
            glyph: kind_glyph(source.kind).to_owned(),
            host,
            state,
            refresh: true,
            token: TokenMark {
                private: source.token,
                action: match (user_git, source.token) {
                    (false, _) => String::new(),
                    (true, true) => words.change,
                    (true, false) => words.add,
                },
                attention: user_git && token_attention(source.kind, source.failure),
            },
            ..OnlineExtra::default()
        }),
        ..plain_row(key, RowKind::Source, 0, source.name, &source.key)
    });
    if opened {
        rows.extend(body);
    }
}

pub(crate) fn loading(source: &OnlineSourceView<'_>, level: usize) -> LibraryRow {
    status_row(
        &source.key,
        level,
        format!("x:{}#loading", source.key),
        "Loading...",
        "loading",
    )
}

pub(crate) fn failed(source: &OnlineSourceView<'_>, level: usize, failure: Failure) -> LibraryRow {
    let text = source_failure_text(source.kind, source.token, failure);
    let status = if failure == Failure::Offline {
        "offline"
    } else {
        "failed"
    };
    status_row(
        &source.key,
        level,
        format!("x:{}#failed", source.key),
        &text,
        status,
    )
}

#[cfg(test)]
#[path = "library_online_tests.rs"]
mod tests;
