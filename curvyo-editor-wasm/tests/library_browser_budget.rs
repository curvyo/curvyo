//! Thread-CPU budget of the library browser (`specs/0152-library-browser` criteria 11 and 22,
//! ADR 0086 decision 10): with 50,000 fetched items a keystroke re-filters and re-ranks within
//! 100 ms of thread CPU, and building the list's index takes at most 300 ms (Proposal). CPU time,
//! never wall time (`CLAUDE.md` §5); asserted in an optimised build, a smoke test in a debug one.
//! A second test runs 31,722 distinct hyphenated names with aliases (iconmind's size), so the
//! dictionary is as large as a real set's. `cpu_time_of` divides by `CURVYO_BUDGET_FACTOR`.
//! A third test adds the shipped keyword file of `specs/0153-library-keywords` (criteria 4, 5).
//! Run: `cargo test --release -p curvyo-editor-wasm --test library_browser_budget -- --nocapture`.

#![cfg(unix)]
#![allow(clippy::unwrap_used, missing_docs)]

mod cpu_time;

use std::time::Duration;

use cpu_time::cpu_time_of;
use curvyo_ui_core::library_browser::{
    BrowseItem, BrowseList, BrowseSource, BrowseState, Browser, ColourFilter, ItemKind, Sort,
};

const ITEMS: usize = 50_000;
const KEYSTROKE: Duration = Duration::from_millis(100);
const BUILD: Duration = Duration::from_millis(300);

const WORDS: [&str; 48] = [
    "arrow", "home", "heart", "circle", "box", "weather", "account", "alarm", "battery", "bell",
    "book", "calendar", "camera", "chart", "chat", "check", "cloud", "code", "cog", "copy",
    "delete", "download", "edit", "email", "eye", "file", "filter", "flag", "folder", "gift",
    "globe", "image", "key", "link", "lock", "map", "menu", "mic", "music", "pencil", "phone",
    "pin", "print", "search", "send", "star", "sun", "user",
];
const SUFFIXES: [&str; 7] = [
    "",
    "outline",
    "rounded",
    "sharp",
    "outline-rounded",
    "off",
    "filled",
];
const GROUPS: [&str; 17] = [
    "Actions",
    "Activities",
    "Android",
    "Audio & Video",
    "Business",
    "Communicate",
    "Hardware",
    "Home",
    "Household",
    "Images",
    "Maps",
    "Privacy",
    "Social",
    "Text",
    "Transit",
    "Travel",
    "UI Actions",
];

/// 50,000 icons in four sets with realistic names ("arrow-left-outline-rounded"), groups and
/// aliases, as one loaded set of Material Symbols' size three times over.
fn list() -> BrowseList {
    let sources = (0..4)
        .map(|s| BrowseSource {
            key: format!("set:https://api.iconify.design#set{s}"),
            label: format!("Icon set {s}"),
            host: "online:https://api.iconify.design".into(),
            prefix: format!("set{s}"),
            place: format!("Iconify › Icon set {s}"),
            category: Some(["Material", "UI 24px", "Logos", "Emoji"][s].into()),
            palette: Some(s == 3),
            ..BrowseSource::default()
        })
        .collect();
    let items = (0..ITEMS)
        .map(|n| {
            let a = WORDS[n % WORDS.len()];
            let b = WORDS[(n / WORDS.len()) % WORDS.len()];
            let suffix = SUFFIXES[(n / (WORDS.len() * WORDS.len())) % SUFFIXES.len()];
            let mut name = format!("{a}-{b}");
            if !suffix.is_empty() {
                name = format!("{name}-{suffix}");
            }
            let variant = n / (WORDS.len() * WORDS.len() * SUFFIXES.len());
            if variant > 0 {
                name = format!("{name}-{variant}");
            }
            BrowseItem {
                kind: ItemKind::Icon,
                name: name.clone(),
                source: u32::try_from(n % 4).unwrap(),
                aliases: if n % 3 == 0 {
                    vec![format!("{name}-alt")]
                } else {
                    Vec::new()
                },
                tags: Vec::new(),
                page_names: Vec::new(),
                groups: vec![GROUPS[n % GROUPS.len()].into()],
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
        })
        .collect();
    BrowseList {
        sources,
        items,
        cards: Vec::new(),
        groups: Vec::new(),
        search_status: String::new(),
    }
}

#[test]
fn a_keystroke_over_50000_items_stays_within_100_ms() {
    let list = list();
    let (mut browser, build) = cpu_time_of(|| Browser::new(list));
    println!("build: {build:?}");
    let mut worst = Duration::ZERO;
    let mut typed = String::new();
    for key in "arow lft hrat".chars() {
        typed.push(key);
        let state = BrowseState {
            text: typed.clone(),
            sort: Sort::Best,
            ..BrowseState::default()
        };
        let (answer, took) = cpu_time_of(|| browser.query(&state));
        println!(
            "{typed:?}: {} of {} in {took:?}",
            answer.count, answer.total
        );
        worst = worst.max(took);
    }
    for text in ["haert", "wether", "cir lef", "home outline rounded", ""] {
        let state = BrowseState {
            text: text.into(),
            sort: Sort::Best,
            colour: ColourFilter::Mono,
            categories: vec!["home".into(), "maps".into()],
            customizable: true,
            ..BrowseState::default()
        };
        let (answer, took) = cpu_time_of(|| browser.query(&state));
        println!("{text:?} filtered: {} in {took:?}", answer.count);
        assert_eq!(answer.total, ITEMS);
        worst = worst.max(took);
    }
    if !cfg!(debug_assertions) {
        assert!(build <= BUILD, "building the index took {build:?}");
        assert!(worst <= KEYSTROKE, "the slowest keystroke took {worst:?}");
    }
}

/// Pseudo-words from syllables ("ka", "lor", ...): about 2,000 distinct field words, so the
/// dictionary is as large as a real set's and every word is scored on each keystroke.
fn word(n: usize) -> String {
    const SYLLABLES: [&str; 46] = [
        "ka", "lor", "mi", "sen", "tu", "vex", "bra", "dol", "fin", "gar", "hep", "jis", "kum",
        "lan", "mor", "nis", "pol", "quo", "rin", "sta", "tor", "ulm", "var", "wix", "yen", "zor",
        "ark", "bel", "cro", "dun", "elm", "fro", "gli", "hun", "ist", "jar", "kel", "lum", "nor",
        "ost", "pra", "rea", "sol", "tri", "uva", "wen",
    ];
    let a = SYLLABLES[n % SYLLABLES.len()];
    let b = SYLLABLES[(n / SYLLABLES.len()) % SYLLABLES.len()];
    if n < SYLLABLES.len() * SYLLABLES.len() {
        format!("{a}{b}")
    } else {
        format!("{a}{b}{}", SYLLABLES[(n / 2116) % SYLLABLES.len()])
    }
}

/// One set of 31,722 distinct hyphenated names (as iconmind's size, named like material-symbols:
/// "kalor-misen-outline-rounded"), every other one with an alias.
fn distinct_list() -> BrowseList {
    const NAMES: usize = 31_722;
    const WORDS_IN_POOL: usize = 2_000;
    let mut seen = std::collections::HashSet::new();
    let mut items = Vec::with_capacity(NAMES);
    let mut n = 0usize;
    while items.len() < NAMES {
        let a = word(n % WORDS_IN_POOL);
        // 16 partners per first word: 32,000 distinct pairs before any suffix.
        let b = word(((n / WORDS_IN_POOL) * 131 + n * 7 + 13) % WORDS_IN_POOL);
        let suffix = SUFFIXES[(n / 3) % SUFFIXES.len()];
        let name = if suffix.is_empty() {
            format!("{a}-{b}")
        } else {
            format!("{a}-{b}-{suffix}")
        };
        n += 1;
        if !seen.insert(name.clone()) {
            continue;
        }
        let aliases = if n.is_multiple_of(2) {
            vec![format!("{b}-{a}")]
        } else {
            Vec::new()
        };
        items.push(BrowseItem {
            aliases,
            groups: vec![GROUPS[n % GROUPS.len()].into()],
            ..icon(&name)
        });
    }
    BrowseList {
        sources: vec![BrowseSource {
            key: "set:https://api.iconify.design#iconmind".into(),
            label: "Iconmind".into(),
            host: "online:https://api.iconify.design".into(),
            prefix: "iconmind".into(),
            place: "Iconify › Iconmind".into(),
            category: Some("UI Multicolor".into()),
            palette: Some(false),
            ..BrowseSource::default()
        }],
        items,
        cards: Vec::new(),
        groups: Vec::new(),
        search_status: String::new(),
    }
}

fn icon(name: &str) -> BrowseItem {
    BrowseItem {
        kind: ItemKind::Icon,
        name: name.to_owned(),
        source: 0,
        aliases: Vec::new(),
        tags: Vec::new(),
        page_names: Vec::new(),
        groups: Vec::new(),
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

#[test]
fn a_keystroke_over_31722_distinct_names_stays_within_100_ms() {
    let list = distinct_list();
    assert_eq!(list.items.len(), 31_722);
    let (mut browser, build) = cpu_time_of(|| Browser::new(list));
    println!("distinct build: {build:?}");
    let mut worst = Duration::ZERO;
    let mut typed = String::new();
    // Prefixes, a typo ("kalro" for "kalor"), a subsequence and two words.
    for key in "kalro misen hrt".chars() {
        typed.push(key);
        let state = BrowseState {
            text: typed.clone(),
            sort: Sort::Best,
            ..BrowseState::default()
        };
        let (answer, took) = cpu_time_of(|| browser.query(&state));
        println!(
            "{typed:?}: {} of {} in {took:?}",
            answer.count, answer.total
        );
        worst = worst.max(took);
    }
    if !cfg!(debug_assertions) {
        assert!(build <= BUILD, "building the index took {build:?}");
        assert!(worst <= KEYSTROKE, "the slowest keystroke took {worst:?}");
    }
}

/// The shipped keyword file (`specs/0153-library-keywords` criteria 4 and 5).
fn keyword_file() -> Vec<u8> {
    std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../frontend/public/library/keywords.bin.gz"),
    )
    .unwrap()
}

/// 50,000 icons in the sets `lucide`, `tabler`, `mdi` and `ph`: every other one has a real name
/// ("search", "heart"), so the keyword file adds its words to 25,000 items.
fn keyword_list() -> BrowseList {
    let mut list = list();
    for (s, source) in list.sources.iter_mut().enumerate() {
        source.prefix = ["lucide", "tabler", "mdi", "ph"][s].into();
    }
    for (n, item) in list.items.iter_mut().enumerate() {
        if n % 2 == 0 {
            item.name = WORDS[(n / 2) % WORDS.len()].into();
        }
    }
    list
}

/// Spec `0153` criterion 4: decoding within 100 ms; criterion 5: with the file, 50,000 items and
/// four typed words with related words, a keystroke within 100 ms; the rebuild within the
/// `set_items` budget of ADR 0086 decision 10 (ADR 0087 decision 5).
#[test]
fn a_keystroke_with_keywords_and_four_related_words_stays_within_100_ms() {
    use curvyo_ui_core::library_browser::Keywords;
    let bytes = keyword_file();
    let (keywords, decode) = cpu_time_of(|| Keywords::decode(&bytes).unwrap());
    println!("decode: {decode:?}");
    let keywords = std::sync::Arc::new(keywords);
    let mut browser = Browser::new(keyword_list());
    let ((), rebuild) = cpu_time_of(|| browser.set_keywords(std::sync::Arc::clone(&keywords)));
    println!("rebuild with keywords: {rebuild:?}");
    let mut worst = Duration::ZERO;
    for text in [
        "magnifier trash bin cog",
        "search delete settings heart",
        "find remove gear love",
        "magnif garb prefer favo",
    ] {
        let mut typed = String::new();
        for key in text.chars() {
            typed.push(key);
            let state = BrowseState {
                text: typed.clone(),
                sort: Sort::Best,
                ..BrowseState::default()
            };
            let (answer, took) = cpu_time_of(|| browser.query(&state));
            worst = worst.max(took);
            if key == ' ' || typed.len() == text.len() {
                println!("{typed:?}: {} in {took:?}", answer.count);
            }
        }
    }
    let one = BrowseState {
        text: "magnifier".into(),
        sort: Sort::Best,
        ..BrowseState::default()
    };
    assert!(browser.query(&one).count > 0, "related words match");
    if !cfg!(debug_assertions) {
        assert!(decode <= KEYSTROKE, "decoding took {decode:?}");
        assert!(rebuild <= BUILD, "the rebuild took {rebuild:?}");
        assert!(worst <= KEYSTROKE, "the slowest keystroke took {worst:?}");
    }
}
