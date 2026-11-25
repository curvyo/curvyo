//! Every word of the library browser (spec `0152` criteria 4, 5, 7, 10, 11, 21 and 23, UX notes
//! sections 4 to 8): subject line, empty states, tiles, tooltips, accessible names, detail bar
//! lines and cards.

use std::fmt::Write as _;

use serde::Serialize;

use curvyo_library_core::Failure;

use crate::library_browser::{BrowseItem, BrowseList, BrowseSource, ItemKind};
use crate::library_online::SearchView;
use crate::library_online_text::{counted, licence_line};

/// The category of an item without one.
pub const UNCATEGORISED: &str = "Uncategorised";

/// `n` with thousands separated by commas: "4,210".
#[must_use]
pub fn grouped(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

fn items_text(n: usize) -> String {
    let word = if n == 1 { "item" } else { "items" };
    format!("{} {word}", grouped(n as u64))
}

/// The subject line (UX notes 4): "4,210 items", "132 of 4,210 items", or at the second level
/// "Box job › Lid · 3 pages".
#[must_use]
pub fn browse_subject(
    count: usize,
    total: usize,
    level: Option<(&BrowseItem, &BrowseSource)>,
) -> String {
    match level {
        Some((item, source)) => {
            let pages = counted(count, "page");
            format!("{} › {} · {pages}", source.label, item.name)
        }
        None if count == total => items_text(total),
        None => format!("{} of {}", grouped(count as u64), items_text(total)),
    }
}

/// The subject line of the Items view narrowed to one source (criterion 27): "Material Symbols ·
/// 600 items", "Material Symbols · 12 of 600 items".
#[must_use]
pub fn narrowed_subject(label: &str, count: usize, in_source: usize) -> String {
    if count == in_source {
        format!("{label} · {}", items_text(count))
    } else {
        format!(
            "{label} · {} of {}",
            grouped(count as u64),
            items_text(in_source)
        )
    }
}

/// The Nothing matches state (criterion 11): the search text, then the active filters by their
/// values, three at most and "and N more".
#[must_use]
pub fn nothing_matches_text(text: &str, values: &[String]) -> String {
    let mut named: Vec<String> = values.iter().take(3).cloned().collect();
    if values.len() > 3 {
        named.push(format!("and {} more", values.len() - 3));
    }
    match (text.is_empty(), named.is_empty()) {
        (true, true) => "Nothing matches.".to_owned(),
        (false, true) => format!("Nothing matches \"{text}\"."),
        (true, false) => format!("Nothing matches in {}.", named.join(", ")),
        (false, false) => format!("Nothing matches \"{text}\" in {}.", named.join(", ")),
    }
}

/// The Enter outcome for the subject line (criterion 21, UX U6): 'Searching "home"…' while one
/// source runs, else '"home": 48 icons', '"home": no icons', '"home": Offline'; several sources
/// sum, and a failed one is named.
#[must_use]
pub fn search_done_status(searches: &[(&str, &SearchView)]) -> String {
    let Some((_, first)) = searches.first() else {
        return String::new();
    };
    let words = &first.words;
    if searches.iter().any(|(_, s)| s.running) {
        return format!("Searching \"{words}\"…");
    }
    let found: usize = searches
        .iter()
        .filter_map(|(_, s)| s.outcome.as_ref().ok().map(Vec::len))
        .sum();
    let failures: Vec<(&str, Failure)> = searches
        .iter()
        .filter_map(|(name, s)| s.outcome.as_ref().err().map(|f| (*name, *f)))
        .collect();
    let reason = |f: Failure| match f {
        Failure::Offline => "Offline".to_owned(),
        Failure::Status(code) => format!("The server answered {code}."),
        _ => "The answer could not be read.".to_owned(),
    };
    if failures.len() == searches.len() && failures.len() == 1 {
        return format!("\"{words}\": {}", reason(failures[0].1));
    }
    let mut text = if found == 0 {
        format!("\"{words}\": no icons")
    } else {
        format!("\"{words}\": {}", counted(found, "icon"))
    };
    for (name, failure) in failures {
        let _ = write!(text, ", {name}: {}", reason(failure));
    }
    text
}

/// What one tile, its tooltip and the detail bar say, and what the page needs to load it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TileWords {
    /// The item's index.
    pub index: u32,
    /// What it is.
    pub kind: ItemKind,
    /// The name on the tile.
    pub name: String,
    /// The meta line's source: set, catalogue or document name.
    pub meta: String,
    /// The licence chip; empty for the maker's own pages.
    pub chip: String,
    /// Two lines: the name, then source, licence, author and facts.
    pub tooltip: String,
    /// The accessible name: "Arrow left, Material Symbols, Apache 2.0, monochrome".
    pub label: String,
    /// The accessible description: the tooltip's second line.
    pub description: String,
    /// The detail bar's place: "Iconify › Material Symbols", "Box job › Lid".
    pub place: String,
    /// The licence's name (the link text); empty for own pages.
    pub licence: String,
    /// The licence's address.
    pub licence_url: String,
    /// The author or credit.
    pub author: String,
    /// The detail bar's facts line.
    pub facts: String,
    /// The detail bar's buttons, none of them loads (H4): `show_pages`, `pick`.
    pub actions: Vec<&'static str>,
    /// The page count chip (0: none).
    pub pages: u32,
    /// Why it cannot be loaded.
    pub refusal: String,
    /// The host's key: the online source key or the local source key.
    pub host: String,
    /// An icon's set prefix.
    pub prefix: String,
    /// A template's or document's path.
    pub path: Vec<String>,
    /// A page's id (the first page of a document).
    pub page_id: String,
    /// The document's name, for Came from.
    pub document: String,
    /// The page's name, for Came from.
    pub page_name: String,
    /// Whether a picture is there to load.
    pub picture: bool,
    /// `true` colour, `false` monochrome, absent unknown.
    pub colour: Option<bool>,
}

fn humane(name: &str) -> String {
    let spaced = name.replace(['-', '_'], " ");
    let mut chars = spaced.chars();
    chars.next().map_or_else(String::new, |c| {
        c.to_uppercase().chain(chars).collect::<String>()
    })
}

fn colour_word(colour: Option<bool>) -> Option<&'static str> {
    colour.map(|c| if c { "colour" } else { "monochrome" })
}

fn page_kind_word(kind: &str) -> String {
    let mut word = humane(kind);
    if !word.is_empty() {
        word.push_str(" page");
    }
    word
}

/// The words of `item` (index `index` of `list`), its colour as refined.
#[must_use]
pub fn tile_words(
    list: &BrowseList,
    index: u32,
    item: &BrowseItem,
    colour: Option<bool>,
) -> TileWords {
    let source = &list.sources[item.source as usize];
    let parent = item.parent.and_then(|p| list.items.get(p as usize));
    let online = !source.local;
    let licence_of = item.licence.as_ref().or(source.licence.as_ref());
    let (licence, licence_url) = if online {
        licence_line(licence_of)
    } else {
        (String::new(), String::new())
    };
    let author = if item.author.is_empty() {
        source.author.clone()
    } else {
        item.author.clone()
    };
    let (meta, place) = match (item.kind, parent) {
        (ItemKind::Page, Some(parent)) if source.local => (
            parent.name.clone(),
            format!("{} › {}", source.label, parent.name),
        ),
        (ItemKind::Page, Some(parent)) => (
            parent.name.clone(),
            format!("{} › {}", source.place, parent.name),
        ),
        _ => (source.label.clone(), source.place.clone()),
    };
    let facts = facts_of(item, colour);
    let description = description_of(item, source.local, &place, &meta, &licence, &author, colour);
    let label = label_of(item, &meta, online.then_some(licence.as_str()), colour);
    let (document, page_name) = match (item.kind, parent) {
        (_, Some(parent)) => (parent.name.clone(), item.name.clone()),
        (ItemKind::Page, None) => (
            item.name.clone(),
            item.page_names.first().cloned().unwrap_or_default(),
        ),
        _ => (item.name.clone(), String::new()),
    };
    TileWords {
        index,
        kind: item.kind,
        name: item.name.clone(),
        chip: licence.clone(),
        tooltip: format!("{}\n{description}", item.name),
        label,
        description,
        meta,
        place,
        licence,
        licence_url,
        author,
        facts,
        actions: actions_of(item),
        pages: item.pages,
        refusal: item.refusal.clone(),
        host: source.host.clone(),
        prefix: source.prefix.clone(),
        path: item.path.clone(),
        page_id: item.page_id.clone(),
        document,
        page_name,
        picture: item.picture,
        colour,
    }
}

/// The tooltip's second line and the accessible description.
fn description_of(
    item: &BrowseItem,
    local: bool,
    place: &str,
    meta: &str,
    licence: &str,
    author: &str,
    colour: Option<bool>,
) -> String {
    let mut second: Vec<String> = Vec::new();
    if local && matches!(item.kind, ItemKind::Page | ItemKind::Document) {
        second.push(format!("{place} › {}", item.name));
    } else {
        second.push(meta.to_owned());
        second.push(licence.to_owned());
        if !author.is_empty() {
            second.push(author.to_owned());
        }
    }
    if !item.size.is_empty() {
        second.push(item.size.clone());
    }
    if !local {
        second.extend(colour_word(colour).map(str::to_owned));
    }
    if item.pages > 1 {
        second.push(counted(item.pages as usize, "page"));
    }
    if !item.refusal.is_empty() {
        second.push(item.refusal.trim_end_matches('.').to_owned());
    }
    second.join(" · ")
}

/// The accessible name: "Arrow left, Material Symbols, Apache 2.0, monochrome".
fn label_of(item: &BrowseItem, meta: &str, licence: Option<&str>, colour: Option<bool>) -> String {
    let mut parts = vec![
        if item.kind == ItemKind::Icon {
            humane(&item.name)
        } else {
            item.name.clone()
        },
        meta.to_owned(),
    ];
    parts.extend(licence.map(str::to_owned));
    parts.extend(colour_word(colour).map(str::to_owned));
    if item.pages > 1 {
        parts.push(counted(item.pages as usize, "page"));
    }
    if !item.refusal.is_empty() {
        parts.push("cannot be loaded".to_owned());
    }
    parts.join(", ")
}

fn facts_of(item: &BrowseItem, colour: Option<bool>) -> String {
    if !item.refusal.is_empty() {
        return item.refusal.clone();
    }
    let mut facts: Vec<String> = Vec::new();
    match item.kind {
        ItemKind::Icon => facts.extend(colour.map(|c| {
            if c {
                "Colour".to_owned()
            } else {
                "Monochrome · takes the object's colour".to_owned()
            }
        })),
        ItemKind::Page => {
            if !item.size.is_empty() {
                facts.push(item.size.clone());
            }
            if !item.page_kind.is_empty() {
                facts.push(page_kind_word(&item.page_kind));
            }
        }
        ItemKind::Template | ItemKind::Document => {
            if item.pages > 1 {
                facts.push(counted(item.pages as usize, "page"));
            }
            if item.kind == ItemKind::Template && !item.size.is_empty() {
                facts.push(item.size.clone());
            }
        }
    }
    if !item.font_note.is_empty() {
        facts.push(item.font_note.clone());
    }
    facts.join(" · ")
}

fn actions_of(item: &BrowseItem) -> Vec<&'static str> {
    // No Load or Import page button (H4): a double-click, Enter or a drag loads.
    match item.kind {
        ItemKind::Template | ItemKind::Document if item.pages > 1 && item.refusal.is_empty() => {
            vec!["show_pages"]
        }
        ItemKind::Page if item.refusal.is_empty() => vec!["pick"],
        _ => Vec::new(),
    }
}
