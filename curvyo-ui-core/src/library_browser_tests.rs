//! Tests of the library browser's list, filters, facets, sorts and words.
use std::collections::{HashMap, HashSet};

use super::*;
use crate::library_browser_sets::SetsState;
use crate::library_index::{DocumentState, IndexedDocument, IndexedPage, LibraryIndex};
use crate::library_online::{OnlineSourceView, OnlineView, SearchView};
use crate::library_tree::{LibrarySource, SourceKind as LocalKind};
use curvyo_document_core::{DocumentSize, UnixMillis};
use curvyo_library_core::{
    Catalogue, Failure, IconRef, IconSet, SetListing, SourceKind, parse_collections,
    parse_set_listing,
};

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(format!(
        "{}/../curvyo-library-core/tests/fixtures/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
}

fn document(source: &str, name: &str, pages: &[&str], modified: i64) -> IndexedDocument {
    IndexedDocument {
        source: source.into(),
        path: vec![format!("{name}.curvyo")],
        size: 1,
        modified: UnixMillis::new(modified),
        state: DocumentState::Readable,
        pages: pages
            .iter()
            .enumerate()
            .map(|(i, page)| IndexedPage {
                id: format!("{name}-{i}"),
                name: (*page).to_owned(),
                kind: "drawing".into(),
                size: DocumentSize::from_mm(210.0, 297.0),
                picture: true,
            })
            .collect(),
    }
}

/// What a session holds: two local sources, the catalogue fixture, the live Iconify sets with
/// Material Symbols loaded, and optionally an Enter answer.
struct Held {
    sources: Vec<LibrarySource>,
    index: LibraryIndex,
    catalogue: Catalogue,
    sets: Vec<IconSet>,
    listings: HashMap<String, SetListing>,
    failures: HashMap<String, Failure>,
    search: Option<SearchView>,
    empty: HashMap<String, IndexedDocument>,
    refused: HashMap<String, Failure>,
}

impl Held {
    fn new() -> Self {
        let mut index = LibraryIndex::default();
        index.put(document("ws", "Box job", &["Lid", "Front", "Back"], 300));
        index.put(document("ws", "Coaster", &["Round"], 100));
        index.put(document("/logos", "Star 10", &["One"], 200));
        index.put(document("/logos", "Star 2", &["One"], 50));
        let mut listings = HashMap::new();
        listings.insert(
            "material-symbols".to_owned(),
            parse_set_listing(&fixture("iconify_live_collection.json")).unwrap(),
        );
        Self {
            sources: vec![
                LibrarySource {
                    key: "ws".into(),
                    name: "Workshop".into(),
                    kind: LocalKind::OpenWorkspace,
                    error: None,
                },
                LibrarySource {
                    key: "/logos".into(),
                    name: "Logos".into(),
                    kind: LocalKind::Folder,
                    error: None,
                },
            ],
            index,
            catalogue: Catalogue::parse(&fixture("curvyo-catalogue.json")).unwrap(),
            sets: parse_collections(&fixture("iconify_live_collections.json")).unwrap(),
            listings,
            failures: HashMap::new(),
            search: None,
            empty: HashMap::new(),
            refused: HashMap::new(),
        }
    }

    fn list(&self, ready: bool) -> BrowseList {
        let views = vec![
            OnlineSourceView {
                key: "online:https://example.org/cat/".into(),
                name: "Curvyo catalogue",
                address: "https://example.org/cat/",
                kind: SourceKind::Git,
                failure: None,
                catalogue: Some(&self.catalogue),
                sets: None,
                set_icons: &self.listings,
                set_failures: &self.failures,
                search: None,
                templates: &self.empty,
                refused: &self.refused,
            },
            OnlineSourceView {
                key: "online:https://api.iconify.design".into(),
                name: "api.iconify.design",
                address: "https://api.iconify.design",
                kind: SourceKind::Iconify,
                failure: None,
                catalogue: None,
                sets: Some(&self.sets),
                set_icons: &self.listings,
                set_failures: &self.failures,
                search: self.search.as_ref(),
                templates: &self.empty,
                refused: &self.refused,
            },
        ];
        let expanded = HashSet::new();
        browse_list(
            Some(LocalView {
                sources: &self.sources,
                index: &self.index,
            }),
            &OnlineView {
                sources: views,
                ready_offered: ready,
                expanded: &expanded,
                query: "",
            },
        )
    }
}

fn names(browser: &Browser, answer: &Answer) -> Vec<String> {
    answer
        .order
        .iter()
        .map(|i| browser.list().items[*i as usize].name.clone())
        .collect()
}

fn state(text: &str) -> BrowseState {
    BrowseState {
        text: text.into(),
        sort: if text.is_empty() {
            Sort::Name
        } else {
            Sort::Best
        },
        ..BrowseState::default()
    }
}

#[test]
fn the_list_holds_what_the_tab_fetched_and_cards_for_the_rest() {
    let list = Held::new().list(true);
    let kinds = |k: ItemKind| list.items.iter().filter(|i| i.kind == k).count();
    assert_eq!(kinds(ItemKind::Icon), 6, "the loaded set's names");
    assert_eq!(kinds(ItemKind::Template), 4);
    assert_eq!(kinds(ItemKind::Document), 1, "Box job has three pages");
    assert_eq!(
        kinds(ItemKind::Page),
        3 + 3,
        "its pages and the single-page documents"
    );
    // The Sets view's cards: every folder, the ready entry, the catalogue and every set.
    let cards: Vec<(&str, CardKind, bool)> = list
        .cards
        .iter()
        .map(|c| (c.name.as_str(), c.kind, c.loaded))
        .collect();
    assert_eq!(
        cards,
        [
            ("Workshop", CardKind::Folder, true),
            ("Logos", CardKind::Folder, true),
            ("Curvyo catalogue", CardKind::Ready, false),
            ("Curvyo catalogue", CardKind::Catalogue, true),
            ("Material Symbols", CardKind::Set, true),
            ("Noto Emoji", CardKind::Set, false),
            ("Tabler Icons", CardKind::Set, false),
            ("WebHostingHub Glyphs", CardKind::Set, false),
        ]
    );
    let groups: Vec<(&str, u8)> = list
        .groups
        .iter()
        .map(|g| (g.label.as_str(), g.rank))
        .collect();
    assert_eq!(
        groups,
        [
            ("My documents", 0),
            ("Curvyo catalogue", 1),
            ("Curvyo catalogue", 2),
            ("api.iconify.design", 3)
        ]
    );
    let zone = list
        .items
        .iter()
        .find(|i| i.name == "activity-zone")
        .unwrap();
    assert_eq!(zone.aliases, ["activity-zone-rounded"]);
    assert_eq!(zone.groups, ["Home"]);
}

#[test]
fn the_top_level_hides_pages_and_the_second_level_shows_them() {
    let mut browser = Browser::new(Held::new().list(false));
    let top = browser.query(&state(""));
    assert_eq!(top.total, 4 + 4 + 6, "documents, templates, icons");
    assert!(!names(&browser, &top).contains(&"Lid".to_owned()));
    let box_job = browser
        .list()
        .items
        .iter()
        .position(|i| i.name == "Box job")
        .unwrap();
    let level = BrowseState {
        level: Some(u32::try_from(box_job).unwrap()),
        ..state("")
    };
    let pages = browser.query(&level);
    assert_eq!(names(&browser, &pages), ["Back", "Front", "Lid"]);
    assert_eq!(pages.subject, "Workshop › Box job · 3 pages");
    // A document found by a page name shows that page at its second level (criterion 8).
    let found = browser.query(&BrowseState {
        text: "lid".into(),
        ..level.clone()
    });
    assert_eq!(names(&browser, &found), ["Lid"]);
    let top = browser.query(&state("lid"));
    assert_eq!(names(&browser, &top), ["Box job"]);
}

#[test]
fn name_sort_is_natural_and_newest_puts_local_pages_first() {
    let mut browser = Browser::new(Held::new().list(false));
    let local = BrowseState {
        sources: vec!["local".into()],
        ..state("")
    };
    let by_name = browser.query(&local);
    assert_eq!(
        names(&browser, &by_name),
        ["Box job", "Coaster", "Star 2", "Star 10"]
    );
    let newest = browser.query(&BrowseState {
        sort: Sort::Newest,
        ..BrowseState::default()
    });
    assert_eq!(
        names(&browser, &newest)[..4],
        ["Box job", "Star 10", "Coaster", "Star 2"]
    );
}

#[test]
fn fuzzy_search_ranks_and_best_match_keeps_the_own_sort_on_ties() {
    let mut browser = Browser::new(Held::new().list(false));
    let answer = browser.query(&state("360"));
    assert_eq!(names(&browser, &answer), ["360", "360-rounded"]);
    // An alias finds its icon: "outline" is in "360-outline", an alias of "360".
    let answer = browser.query(&state("360 outline"));
    assert!(names(&browser, &answer).contains(&"360".to_owned()));
    assert_eq!(browser.query(&state("kreis")).count, 0);
    let answer = browser.query(&state("zzzz"));
    assert_eq!(answer.empty, "Nothing matches \"zzzz\".");
}

#[test]
fn the_source_filter_and_its_counts() {
    let mut browser = Browser::new(Held::new().list(false));
    let answer = browser.query(&state(""));
    let rows: Vec<(&str, usize, bool)> = answer
        .sources
        .iter()
        .map(|r| (r.label.as_str(), r.count, r.indent))
        .collect();
    assert_eq!(
        rows,
        [
            ("My documents", 4, false),
            ("Workshop", 2, true),
            ("Logos", 2, true),
            ("Curvyo catalogue", 4, false),
            ("Material Symbols", 6, false),
        ]
    );
    let logos = browser.query(&BrowseState {
        sources: vec!["local:/logos".into()],
        ..state("")
    });
    assert_eq!(names(&browser, &logos), ["Star 2", "Star 10"]);
    assert_eq!(
        logos.subject, "2 of 14 items",
        "a selection in the list is not a place"
    );
    assert_eq!(logos.active, 1);
}

#[test]
fn categories_merge_set_category_groups_folders_and_matching_tags() {
    let mut browser = Browser::new(Held::new().list(false));
    let answer = browser.query(&state(""));
    let rows: Vec<(&str, usize)> = answer
        .categories
        .iter()
        .map(|r| (r.label.as_str(), r.count))
        .collect();
    // The cards' set categories are rows too (the cards follow the filter).
    for want in [
        ("Home", 3),
        ("Logos", 2),
        ("Maps", 3),
        ("Material", 6),
        ("Workshop", 2),
    ] {
        assert!(rows.contains(&want), "{want:?} in {rows:?}");
    }
    assert_eq!(rows.last().unwrap().0, "Uncategorised");
    let material = browser.query(&BrowseState {
        categories: vec!["material".into()],
        ..state("")
    });
    assert_eq!(material.count, 6, "a set's category, not only its groups");
    let home = browser.query(&BrowseState {
        categories: vec!["home".into()],
        ..state("")
    });
    assert_eq!(home.count, 3);
    assert!(answer.has_categories);
}

#[test]
fn colour_and_customizable_follow_the_set_flag_and_the_tag() {
    let mut held = Held::new();
    held.catalogue.templates[0].tags.push("Customizable".into());
    let mut browser = Browser::new(held.list(false));
    let mono = browser.query(&BrowseState {
        colour: ColourFilter::Mono,
        ..state("")
    });
    assert_eq!(mono.count, 6, "templates and pages have no colour data");
    let colour = browser.query(&BrowseState {
        colour: ColourFilter::Colour,
        ..state("")
    });
    assert_eq!(colour.count, 0);
    let sets = browser.query_sets(&SetsState {
        colour: ColourFilter::Colour,
        ..SetsState::default()
    });
    assert_eq!(
        sets.count, 1,
        "the Noto set card follows the Sets view's own filter"
    );
    let custom = browser.query(&BrowseState {
        customizable: true,
        ..state("")
    });
    assert_eq!(
        custom.count,
        6 + 1,
        "monochrome icons and the tagged template"
    );
    assert!(custom.has_colour && custom.has_customizable);
}

#[test]
fn a_fixed_colour_body_refines_on_the_next_change_only() {
    let mut browser = Browser::new(Held::new().list(false));
    let mono = BrowseState {
        colour: ColourFilter::Mono,
        ..state("")
    };
    assert_eq!(browser.query(&mono).count, 6);
    let first = browser.query(&mono).order[0];
    browser.note_body(first, true);
    assert_eq!(browser.query(&mono).count, 6, "not while the maker looks");
    let changed = BrowseState {
        text: "a".into(),
        ..mono.clone()
    };
    browser.query(&changed);
    assert_eq!(browser.query(&mono).count, 5);
}

#[test]
fn tiles_and_cards_are_worded() {
    let mut browser = Browser::new(Held::new().list(true));
    let answer = browser.query(&state("activity-zone"));
    let tile = browser.tile(answer.order[0]).unwrap();
    assert_eq!(tile.name, "activity-zone");
    assert_eq!(tile.chip, "Apache 2.0");
    assert_eq!(
        tile.label,
        "Activity zone, Material Symbols, Apache 2.0, monochrome"
    );
    assert_eq!(tile.place, "Iconify › Material Symbols");
    assert_eq!(tile.facts, "Monochrome · takes the object's colour");
    assert!(tile.actions.is_empty(), "no Load button (H4)");
    assert_eq!(tile.prefix, "material-symbols");
    let page = browser
        .list()
        .items
        .iter()
        .position(|i| i.name == "Coaster")
        .unwrap();
    let page = browser.tile(u32::try_from(page).unwrap()).unwrap();
    assert_eq!(page.chip, "", "own pages carry no licence chip");
    assert_eq!(page.tooltip, "Coaster\nWorkshop › Coaster · 210 × 297 mm");
    assert_eq!(page.actions, ["pick"]);
    assert_eq!(
        (page.document.as_str(), page.page_name.as_str()),
        ("Coaster", "Round")
    );
    let noto = browser
        .list()
        .cards
        .iter()
        .position(|c| c.name == "Noto Emoji")
        .unwrap();
    let card = browser.card(u32::try_from(noto).unwrap()).unwrap();
    assert_eq!(card.line, "3,729 icons · Emoji");
    assert_eq!(card.facts, "3,729 icons · Emoji · Colour · Not loaded");
    assert_eq!(card.narrow, "set:https://api.iconify.design#noto");
    assert_eq!(card.state, "");
    let material = browser
        .list()
        .cards
        .iter()
        .position(|c| c.name == "Material Symbols")
        .unwrap();
    assert_eq!(
        browser
            .card(u32::try_from(material).unwrap())
            .unwrap()
            .state,
        "Loaded"
    );
    let ready = browser.card(2).unwrap();
    assert_eq!(
        (ready.line.as_str(), ready.narrow.as_str()),
        ("Not added", "")
    );
    let folder = browser.card(0).unwrap();
    assert_eq!(folder.line, "2 documents · 4 pages");
}

#[test]
fn enter_results_join_as_search_results_with_their_outcome() {
    let mut held = Held::new();
    held.search = Some(SearchView {
        words: "home".into(),
        outcome: Ok(vec![IconRef {
            prefix: "tabler".into(),
            name: "home".into(),
        }]),
        running: false,
        sets: Vec::new(),
    });
    let mut browser = Browser::new(held.list(false));
    let answer = browser.query(&BrowseState {
        sources: vec!["search".into()],
        ..state("home")
    });
    assert_eq!(names(&browser, &answer), ["home"]);
    assert_eq!(answer.sources.last().unwrap().label, "Search results");
    assert_eq!(answer.subject, "1 of 15 items · \"home\": 1 icon");
    let tile = browser.tile(answer.order[0]).unwrap();
    assert_eq!(tile.meta, "Tabler Icons");
    assert_eq!(tile.chip, "MIT");
}

#[test]
fn the_list_crosses_as_json_and_back() {
    let list = Held::new().list(true);
    let text = serde_json::to_string(&list).unwrap();
    let back: BrowseList = serde_json::from_str(&text).unwrap();
    assert_eq!(back, list);
}

#[test]
fn nothing_matches_names_three_filters_and_more() {
    let values: Vec<String> = ["A", "B", "C", "D", "E"].map(str::to_owned).to_vec();
    assert_eq!(
        crate::library_browser_text::nothing_matches_text("hrat", &values),
        "Nothing matches \"hrat\" in A, B, C, and 2 more."
    );
    assert_eq!(
        crate::library_browser_text::nothing_matches_text("", &values[..1]),
        "Nothing matches in A."
    );
}

#[test]
fn the_natural_key_sorts_as_the_natural_comparison() {
    let mut names = vec![
        "lid 10", "lid 2", "lid", "lid 02", "a-b", "a b", "a1", "a", "z", "10", "9", "é",
        "box 007x", "box 7y",
    ];
    let mut by_key = names.clone();
    by_key.sort_by_cached_key(|n| crate::library_browser_tables::natural_key(n));
    names.sort_by(|a, b| crate::workspace_rows::natural_cmp(a, b));
    assert_eq!(by_key, names);
}

#[test]
fn an_empty_items_view_offers_show_sets_when_there_is_a_set_to_open() {
    let expanded = HashSet::new();
    let online = |ready: bool| OnlineView {
        sources: Vec::new(),
        ready_offered: ready,
        expanded: &expanded,
        query: "",
    };
    let mut ready_only = Browser::new(browse_list(None, &online(true)));
    let answer = ready_only.query(&state(""));
    assert_eq!(answer.empty, "No items yet.");
    assert!(answer.show_sets);
    let mut nothing = Browser::new(browse_list(None, &online(false)));
    let answer = nothing.query(&state(""));
    assert_eq!(answer.empty, "Nothing in the library yet.");
    assert!(!answer.show_sets);
    assert_eq!(
        nothing.query_sets(&SetsState::default()).empty,
        "Nothing in the library yet."
    );
}

#[test]
fn the_sets_view_groups_cards_by_source_and_filters_with_its_own_facets() {
    let mut browser = Browser::new(Held::new().list(true));
    let all = browser.query_sets(&SetsState::default());
    let headings: Vec<&str> = all.groups.iter().map(|g| g.heading.as_str()).collect();
    assert_eq!(
        headings,
        [
            "My documents · 2 folders",
            "Curvyo catalogue",
            "Curvyo catalogue",
            "api.iconify.design · 4 sets · 1 loaded"
        ]
    );
    let iconify = &all.groups[3];
    let names: Vec<&str> = iconify
        .cards
        .iter()
        .map(|c| browser.list().cards[*c as usize].name.as_str())
        .collect();
    assert_eq!(
        names,
        [
            "Material Symbols",
            "Noto Emoji",
            "Tabler Icons",
            "WebHostingHub Glyphs"
        ],
        "A to Z, loaded and not loaded mixed"
    );
    assert_eq!(
        all.subject, "4 sets · 2 loaded",
        "sets only; loaded as the cards' state (the set and the catalogue)"
    );
    let emoji = browser.query_sets(&SetsState {
        categories: vec!["emoji".into()],
        ..SetsState::default()
    });
    assert_eq!(emoji.count, 1);
    assert_eq!(emoji.subject, "1 of 4 sets");
    let found = browser.query_sets(&SetsState {
        text: "tablr".into(),
        ..SetsState::default()
    });
    assert_eq!(found.count, 1, "the fuzzy search over names");
    let none = browser.query_sets(&SetsState {
        text: "hrat".into(),
        ..SetsState::default()
    });
    assert_eq!(none.empty, "No set matches \"hrat\".");
    let local = browser.query_sets(&SetsState {
        sources: vec!["local".into()],
        ..SetsState::default()
    });
    assert_eq!(local.count, 2);
    assert_eq!(all.sources[0].count, 2);
}

#[test]
fn a_narrowed_items_view_names_its_source_and_counts_within_it() {
    let mut browser = Browser::new(Held::new().list(false));
    // Opened from the Sets view (criterion 27).
    let set = narrowed_to("set:https://api.iconify.design#material-symbols");
    assert_eq!(browser.query(&set).subject, "Material Symbols · 6 items");
    let searched = BrowseState {
        text: "360".into(),
        sort: Sort::Best,
        ..set
    };
    assert_eq!(
        browser.query(&searched).subject,
        "Material Symbols · 2 of 6 items"
    );
}
