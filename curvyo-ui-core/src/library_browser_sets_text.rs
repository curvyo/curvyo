//! The words of the library browser's Sets view (spec `0152` criteria 26 to 28, UX notes 0.2):
//! cards, group headings, the subject line and the empty state.

use serde::Serialize;

use crate::library_browser::{BrowseCard, CardKind};
use crate::library_browser_text::{UNCATEGORISED, grouped};
use crate::library_online_text::{counted, licence_line};

/// "196 sets · 4 loaded", "12 of 196 sets".
#[must_use]
pub fn sets_subject(count: usize, total: usize, loaded: usize) -> String {
    let all = format!(
        "{} {}",
        grouped(total as u64),
        if total == 1 { "set" } else { "sets" }
    );
    if count == total {
        format!("{all} · {} loaded", grouped(loaded as u64))
    } else {
        format!("{} of {all}", grouped(count as u64))
    }
}

/// 'No set matches "hrat".' (H14); with no search text 'No set matches.'.
#[must_use]
pub fn no_set_matches_text(text: &str) -> String {
    if text.is_empty() {
        "No set matches.".to_owned()
    } else {
        format!("No set matches \"{text}\".")
    }
}

/// A group's heading: "My documents · 3 folders", "api.iconify.design · 280 sets · 4 loaded",
/// a catalogue's name.
#[must_use]
pub fn group_heading(label: &str, cards: &[&BrowseCard]) -> String {
    let sets = cards.iter().filter(|c| c.kind == CardKind::Set).count();
    let folders = cards.iter().filter(|c| c.kind == CardKind::Folder).count();
    if sets > 0 {
        let loaded = cards
            .iter()
            .filter(|c| c.kind == CardKind::Set && c.loaded)
            .count();
        format!(
            "{label} · {} · {} loaded",
            counted(sets, "set"),
            grouped(loaded as u64)
        )
    } else if folders > 0 {
        format!("{label} · {}", counted(folders, "folder"))
    } else {
        label.to_owned()
    }
}

/// What one card and the detail bar say of it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CardWords {
    /// The card's index.
    pub index: u32,
    /// What it is.
    pub kind: CardKind,
    /// The name.
    pub name: String,
    /// Line 2: "2,500 icons · Material", "12 templates", "14 documents · 38 pages", the host,
    /// "Not added".
    pub line: String,
    /// A set's licence chip.
    pub chip: String,
    /// A set's licence address.
    pub licence_url: String,
    /// A set's author.
    pub author: String,
    /// The state slot: "Loaded" or empty.
    pub state: String,
    /// The detail bar's place: "Iconify › api.iconify.design", "My documents".
    pub place: String,
    /// The detail bar's facts: "2,500 icons · Material · Monochrome · Loaded".
    pub facts: String,
    /// The accessible name.
    pub label: String,
    /// The failure line; empty when none.
    pub failure: String,
    /// Whether the failure is Offline.
    pub offline: bool,
    /// The online source key, or the local source key.
    pub key: String,
    /// A set's prefix.
    pub prefix: String,
    /// The Items view's Source key opening narrows to; empty when opening stays in Sets.
    pub narrow: String,
    /// A set's sample icon names.
    pub samples: Vec<String>,
    /// `git`, `iconify`, `folder` or `ready`.
    pub glyph: String,
    /// Whether the set's names or the catalogue's list are held.
    pub loaded: bool,
}

fn plural(n: u64, word: &str) -> String {
    format!("{} {word}{}", grouped(n), if n == 1 { "" } else { "s" })
}

/// The words of `card`.
#[must_use]
pub fn card_words(index: u32, card: &BrowseCard) -> CardWords {
    let line = match card.kind {
        CardKind::Set => format!(
            "{} · {}",
            plural(card.total, "icon"),
            card.category.as_deref().unwrap_or(UNCATEGORISED)
        ),
        CardKind::Catalogue => plural(card.count, "template"),
        CardKind::Folder => format!(
            "{} · {}",
            plural(card.count, "document"),
            plural(card.pages, "page")
        ),
        CardKind::Source => card.host.clone(),
        CardKind::Ready => "Not added".to_owned(),
    };
    let (chip, licence_url) = if card.kind == CardKind::Set {
        licence_line(card.licence.as_ref())
    } else {
        (String::new(), String::new())
    };
    let place = match card.kind {
        CardKind::Folder => "My documents".to_owned(),
        CardKind::Set => format!("Iconify › {}", card.host),
        CardKind::Source if card.glyph == "iconify" => format!("Iconify › {}", card.host),
        _ => format!("Git catalogue › {}", card.host),
    };
    let loaded_word = match card.kind {
        CardKind::Set | CardKind::Catalogue | CardKind::Source if card.loaded => "Loaded",
        CardKind::Set | CardKind::Catalogue | CardKind::Source => "Not loaded",
        _ => "",
    };
    let mut facts = vec![line.clone()];
    facts.extend(card.palette.map(|c| {
        if c {
            "Colour".to_owned()
        } else {
            "Monochrome".to_owned()
        }
    }));
    if !loaded_word.is_empty() {
        facts.push(loaded_word.to_owned());
    }
    if !card.failure.is_empty() {
        facts.push(card.failure.trim_end_matches('.').to_owned());
    }
    let mut label = vec![card.name.clone(), line.clone()];
    if !chip.is_empty() {
        label.push(chip.clone());
    }
    if !loaded_word.is_empty() {
        label.push(loaded_word.to_lowercase());
    }
    if !card.failure.is_empty() {
        label.push(card.failure.trim_end_matches('.').to_owned());
    }
    CardWords {
        index,
        kind: card.kind,
        name: card.name.clone(),
        line,
        chip,
        licence_url,
        author: card.author.clone(),
        state: if card.loaded && card.kind != CardKind::Folder {
            "Loaded".to_owned()
        } else {
            String::new()
        },
        place,
        facts: facts.join(" · "),
        label: label.join(", "),
        failure: card.failure.clone(),
        offline: card.offline,
        key: card.key.clone(),
        prefix: card.prefix.clone(),
        narrow: card.narrow.clone(),
        samples: card.samples.clone(),
        glyph: card.glyph.clone(),
        loaded: card.loaded,
    }
}
