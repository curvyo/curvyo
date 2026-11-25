//! The browser with the shipped keyword file (spec `0153` criteria 1, 2, 3, 9 and 14, ADR 0087
//! decisions 4 and 5): icons of Lucide, Tabler and MDI match by their published keywords and by
//! related words, a set without keywords by related words only, and the detail bar says why.

#![allow(clippy::unwrap_used, missing_docs)]

use std::path::Path;
use std::sync::Arc;

use curvyo_library_core::{Keywords, Why};
use curvyo_ui_core::library_browser::{
    BrowseItem, BrowseList, BrowseSource, BrowseState, Browser, Sort, why_words,
};

fn keywords() -> Arc<Keywords> {
    let file =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../frontend/public/library/keywords.bin.gz");
    Arc::new(Keywords::decode(&std::fs::read(file).unwrap()).unwrap())
}

/// Sets `lucide`, `tabler`, `mdi` and `ph` (no keywords), each with a few real icon names.
fn list() -> BrowseList {
    let sets: [(&str, &[&str]); 4] = [
        ("lucide", &["search", "trash", "heart", "house"]),
        ("tabler", &["search", "trash", "trash-filled", "heart"]),
        ("mdi", &["magnify", "delete", "heart", "home"]),
        (
            "ph",
            &["search", "trash", "delete", "heart", "not-in-the-data-x"],
        ),
    ];
    let sources: Vec<serde_json::Value> = sets
        .iter()
        .map(|(prefix, _)| {
            serde_json::json!({
                "key": format!("set:https://api.iconify.design#{prefix}"),
                "label": format!("Set {prefix}"),
                "host": "online:https://api.iconify.design",
                "prefix": prefix,
                "place": format!("Iconify › {prefix}"),
            })
        })
        .collect();
    let items: Vec<serde_json::Value> = sets
        .iter()
        .enumerate()
        .flat_map(|(s, (_, names))| {
            names
                .iter()
                .map(move |name| serde_json::json!({"kind": "icon", "name": name, "source": s}))
        })
        .collect();
    serde_json::from_value(serde_json::json!({
        "sources": sources, "items": items, "cards": [], "groups": [], "search_status": ""
    }))
    .unwrap()
}

fn names(browser: &mut Browser, text: &str) -> Vec<String> {
    let list = browser.list().clone();
    let answer = browser.query(&BrowseState {
        text: text.into(),
        sort: Sort::Best,
        ..BrowseState::default()
    });
    answer
        .order
        .iter()
        .map(|i| {
            let item = &list.items[*i as usize];
            format!(
                "{}:{}",
                list.sources[item.source as usize].prefix, item.name
            )
        })
        .collect()
}

#[test]
fn keywords_and_related_words_match_once_the_file_is_set() {
    let mut browser = Browser::new(list());
    assert!(
        names(&mut browser, "magnifier").is_empty(),
        "without the file only names match"
    );
    browser.set_keywords(keywords());
    let found = names(&mut browser, "magnifier");
    for name in ["lucide:search", "tabler:search", "mdi:magnify", "ph:search"] {
        assert!(found.contains(&name.to_owned()), "{name} in {found:?}");
    }
    let bin = names(&mut browser, "bin");
    assert!(
        bin.contains(&"ph:trash".to_owned()),
        "bin finds trash: {bin:?}"
    );
    assert!(bin.contains(&"tabler:trash-filled".to_owned()), "{bin:?}");
    let trash = names(&mut browser, "trash");
    // The customer's example: in a set without keywords, by related words only.
    assert!(
        trash.contains(&"ph:delete".to_owned()),
        "trash finds delete: {trash:?}"
    );
    assert!(
        found.contains(&"ph:search".to_owned()),
        "magnifier finds search"
    );
    assert!(
        trash.contains(&"mdi:delete".to_owned()),
        "trash finds delete"
    );
    assert_eq!(
        names(&mut browser, "magnifier heart"),
        Vec::<String>::new(),
        "every typed word must match"
    );
    // Criterion 3: the names first.
    let search = names(&mut browser, "search");
    assert_eq!(search[..3], ["lucide:search", "tabler:search", "ph:search"]);
    assert_eq!(search[3], "mdi:magnify", "MDI's alias next");
}

#[test]
fn the_browser_made_with_the_file_keeps_it_and_the_detail_bar_says_why() {
    let mut browser = Browser::with_keywords(list(), Some(keywords()));
    let list = browser.list().clone();
    let at = |prefix: &str, name: &str| {
        let index = list
            .items
            .iter()
            .position(|item| {
                item.name == name && list.sources[item.source as usize].prefix == prefix
            })
            .unwrap();
        u32::try_from(index).unwrap()
    };
    let lucide = browser.why(at("lucide", "search"), "magnifer").unwrap();
    assert_eq!(
        lucide.line, "Matches “magnifier” (keyword)",
        "the data's word, not the typed text"
    );
    let ph = browser.why(at("ph", "trash"), "bin").unwrap();
    assert_eq!(ph.line, "Matches “bin” (related to trash)");
    assert_eq!(
        browser.why(at("ph", "search"), "search"),
        None,
        "a name match"
    );
    assert_eq!(
        browser.why(at("ph", "heart"), "magnifier"),
        None,
        "no match"
    );
    let two = browser
        .why(at("lucide", "search"), "search magnifier")
        .unwrap();
    assert_eq!(
        two.line, "Matches “magnifier” (keyword)",
        "the name word gives none"
    );
}

#[test]
fn the_criterion_14_texts_are_reachable_from_outside_the_crate() {
    let words = why_words(&[
        Why::Category("arrows".into()),
        Why::Related {
            head: "bin".into(),
            word: "trash".into(),
        },
    ])
    .unwrap();
    assert_eq!(words.line, "Matches “arrows” (category) and 1 more");
    assert_eq!(why_words(&[Why::Name, Why::Source]), None);
    // A list built in code: an item names only what it needs.
    let list = BrowseList {
        sources: vec![BrowseSource {
            prefix: "ph".into(),
            ..BrowseSource::default()
        }],
        items: vec![BrowseItem {
            name: "trash".into(),
            ..BrowseItem::default()
        }],
        ..BrowseList::default()
    };
    let mut browser = Browser::with_keywords(list, Some(keywords()));
    assert!(browser.keywords().is_some());
    assert_eq!(
        browser.why(0, "bin").unwrap().line,
        "Matches “bin” (related to trash)"
    );
}
