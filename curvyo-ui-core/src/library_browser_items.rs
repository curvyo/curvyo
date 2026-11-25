//! The browser's list built from what the Library tab holds (spec `0152` criteria 5, 8 to 10, 12
//! and 21): local documents and pages, catalogue templates, loaded Iconify sets, the Enter
//! results, and the cards of what is not fetched; nothing here fetches.

use std::collections::HashMap;

use curvyo_library_core::{IconSet, SourceKind};

use crate::library_browser::{BrowseItem, BrowseList, BrowseSource, ItemKind};
use crate::library_browser_text::search_done_status;
use crate::library_index::{DocumentState, IndexedDocument, LibraryIndex};
use crate::library_index_text::page_size_text;
use crate::library_online::{OnlineSourceView, OnlineView};
use crate::library_online_templates::refusal;
use crate::library_online_text::{refused_line, size_text};
use crate::library_tree::LibrarySource;

/// The tag that makes a template customizable (Frage 371 A).
const CUSTOMIZABLE_TAG: &str = "customizable";

/// The local part of the library: the sources in order and the index.
#[derive(Clone, Copy)]
pub struct LocalView<'a> {
    /// The sources, in source order.
    pub sources: &'a [LibrarySource],
    /// The records of their documents.
    pub index: &'a LibraryIndex,
}

fn item(kind: ItemKind, name: &str, source: usize) -> BrowseItem {
    BrowseItem {
        kind,
        name: name.to_owned(),
        // invariant: the sources are a few hundred at most
        source: u32::try_from(source).unwrap_or(u32::MAX),
        aliases: Vec::new(),
        tags: Vec::new(),
        groups: Vec::new(),
        page_names: Vec::new(),
        path: Vec::new(),
        page_id: String::new(),
        page_kind: String::new(),
        size: String::new(),
        modified: 0,
        pages: 0,
        parent: None,
        picture: false,
        tagged_customizable: false,
        refusal: String::new(),
        font_note: String::new(),
        licence: None,
        author: String::new(),
    }
}

fn index_of(list: &BrowseList) -> u32 {
    // invariant: a list holds far fewer than 2^32 items (50,000 budgeted)
    u32::try_from(list.items.len()).unwrap_or(u32::MAX)
}

/// The pages of `document` as children of item `parent` (or the document's one page as item).
fn push_pages(list: &mut BrowseList, document: &IndexedDocument, source: usize, parent: u32) {
    for page in &document.pages {
        list.items.push(BrowseItem {
            path: document.path.clone(),
            page_id: page.id.clone(),
            page_kind: page.kind.clone(),
            size: page_size_text(page.size),
            modified: document.modified.get(),
            parent: Some(parent),
            picture: page.picture,
            ..item(ItemKind::Page, &page.name, source)
        });
    }
}

fn push_local(list: &mut BrowseList, local: LocalView<'_>) {
    for source in local.sources.iter().filter(|s| s.error.is_none()) {
        let at = list.sources.len();
        list.sources.push(BrowseSource {
            key: format!("local:{}", source.key),
            label: source.name.clone(),
            local: true,
            host: source.key.clone(),
            place: source.name.clone(),
            ..BrowseSource::default()
        });
        for document in local
            .index
            .documents()
            .iter()
            .filter(|d| d.source == source.key)
        {
            push_document(list, document, at);
        }
    }
}

fn push_document(list: &mut BrowseList, document: &IndexedDocument, source: usize) {
    let refusal = match document.state {
        DocumentState::Readable if !document.pages.is_empty() => "",
        DocumentState::Readable | DocumentState::NotCurvyo => return,
        DocumentState::Newer => "Made by a newer Curvyo.",
        DocumentState::Damaged => "Cannot be read: damaged.",
    };
    let modified = document.modified.get();
    let base = BrowseItem {
        path: document.path.clone(),
        modified,
        refusal: refusal.to_owned(),
        ..item(ItemKind::Document, document.name(), source)
    };
    if let [page] = document.pages.as_slice() {
        // A single-page document is its page (criterion 8): the document's name, the page's data.
        list.items.push(BrowseItem {
            kind: ItemKind::Page,
            page_id: page.id.clone(),
            page_kind: page.kind.clone(),
            size: page_size_text(page.size),
            picture: page.picture,
            page_names: vec![page.name.clone()],
            ..base
        });
        return;
    }
    let parent = index_of(list);
    let first = document.pages.first();
    list.items.push(BrowseItem {
        pages: u32::try_from(document.pages.len()).unwrap_or(u32::MAX),
        page_id: first.map(|p| p.id.clone()).unwrap_or_default(),
        picture: first.is_some_and(|p| p.picture),
        // The page names find the document (spec criterion 8).
        page_names: document.pages.iter().map(|p| p.name.clone()).collect(),
        ..base
    });
    if refusal.is_empty() {
        push_pages(list, document, source, parent);
    }
}

fn set_source(set: &IconSet, view: &OnlineSourceView<'_>, key: String) -> BrowseSource {
    BrowseSource {
        key,
        label: set.name.clone(),
        host: view.key.clone(),
        prefix: set.prefix.clone(),
        place: format!("Iconify › {}", set.name),
        category: set.category.clone(),
        palette: set.palette,
        licence: set.licence.clone(),
        author: set.author.clone(),
        local: false,
    }
}

fn push_templates(list: &mut BrowseList, view: &OnlineSourceView<'_>) {
    let Some(catalogue) = view.catalogue else {
        return;
    };
    let at = list.sources.len();
    list.sources.push(BrowseSource {
        key: view.key.clone(),
        label: view.name.to_owned(),
        host: view.key.clone(),
        place: view.name.to_owned(),
        ..BrowseSource::default()
    });
    for entry in &catalogue.templates {
        let path: Vec<String> = entry.path.split('/').map(str::to_owned).collect();
        let held = view.templates.get(&entry.path);
        let parent = index_of(list);
        let pages = held.map_or(0, |d| d.pages.len());
        list.items.push(BrowseItem {
            tags: entry.tags.clone(),
            tagged_customizable: entry
                .tags
                .iter()
                .any(|t| t.trim().eq_ignore_ascii_case(CUSTOMIZABLE_TAG)),
            path: path.clone(),
            size: size_text(entry.size),
            pages: u32::try_from(pages).unwrap_or(u32::MAX),
            picture: entry.thumbnail_sha256.is_some(),
            refusal: refusal(view, entry).map(refused_line).unwrap_or_default(),
            font_note: if entry.fonts_withheld() {
                "Font not included".to_owned()
            } else {
                String::new()
            },
            licence: entry.licence.clone(),
            author: entry.author.clone(),
            ..item(ItemKind::Template, &entry.name, at)
        });
        if let Some(document) = held.filter(|_| pages > 1) {
            push_pages(list, document, at, parent);
            // The pages of a template keep its tags, licence and author for the filters.
            let tags = entry.tags.clone();
            for page in list.items.iter_mut().skip(parent as usize + 1) {
                page.tags.clone_from(&tags);
                page.licence.clone_from(&entry.licence);
                page.author.clone_from(&entry.author);
            }
        }
    }
}

fn push_sets(list: &mut BrowseList, view: &OnlineSourceView<'_>) {
    let Some(sets) = view.sets else {
        return;
    };
    for set in sets {
        let Some(listing) = view.set_icons.get(&set.prefix) else {
            continue;
        };
        let at = list.sources.len();
        let key = format!("set:{}#{}", view.address, set.prefix);
        list.sources.push(set_source(set, view, key));
        let aliases = listing.aliases_by_name();
        for (n, name) in listing.names.iter().enumerate() {
            list.items.push(BrowseItem {
                aliases: aliases[n].iter().map(|a| (*a).to_owned()).collect(),
                groups: listing.groups_of(n).map(str::to_owned).collect(),
                ..item(ItemKind::Icon, name, at)
            });
        }
    }
}

fn push_search(list: &mut BrowseList, views: &[OnlineSourceView<'_>]) {
    let mut sources: HashMap<(String, String), usize> = HashMap::new();
    for view in views {
        let Some(Ok(icons)) = view.search.map(|s| &s.outcome) else {
            continue;
        };
        for icon in icons {
            let key = (view.key.clone(), icon.prefix.clone());
            let at = *sources.entry(key).or_insert_with(|| {
                let set = view
                    .search
                    .and_then(|s| s.sets.iter().find(|s| s.prefix == icon.prefix))
                    .or_else(|| {
                        view.sets
                            .unwrap_or_default()
                            .iter()
                            .find(|s| s.prefix == icon.prefix)
                    });
                let source = set.map_or_else(
                    || BrowseSource {
                        label: icon.prefix.clone(),
                        host: view.key.clone(),
                        prefix: icon.prefix.clone(),
                        place: format!("Iconify › {}", icon.prefix),
                        ..BrowseSource::default()
                    },
                    |set| set_source(set, view, String::new()),
                );
                list.sources.push(BrowseSource {
                    key: "search".to_owned(),
                    ..source
                });
                list.sources.len() - 1
            });
            list.items.push(item(ItemKind::Icon, &icon.name, at));
        }
    }
}

/// The Enter outcome of every Iconify source: running while one runs, else the sum, a failure by
/// its source's name.
fn search_status(online: &OnlineView<'_>) -> String {
    let searches: Vec<(&str, &crate::library_online::SearchView)> = online
        .sources
        .iter()
        .filter_map(|v| v.search.map(|s| (v.name, s)))
        .collect();
    search_done_status(&searches)
}

/// The browser's list (criteria 9 and 10): what the tab holds, never more.
#[must_use]
pub fn browse_list(local: Option<LocalView<'_>>, online: &OnlineView<'_>) -> BrowseList {
    let mut list = BrowseList::default();
    if let Some(local) = local {
        push_local(&mut list, local);
    }
    for view in online.sources.iter().filter(|v| v.kind == SourceKind::Git) {
        push_templates(&mut list, view);
    }
    for view in online
        .sources
        .iter()
        .filter(|v| v.kind == SourceKind::Iconify)
    {
        push_sets(&mut list, view);
    }
    push_search(&mut list, &online.sources);
    crate::library_browser_cards::push_cards(&mut list, local, online);
    list.search_status = search_status(online);
    list
}
