//! Tester budget of `specs/0152-library-browser` criterion 22 at the spec's number: a keystroke
//! re-filters and re-ranks 50,000 fetched items within 100 ms of thread CPU time (ADR 0086
//! decision 10, the budget of decision 9). Timed by thread CPU time (`CLAUDE.md` §5), the
//! budget scaled only by `CURVYO_BUDGET_FACTOR` (CI). Run natively in release:
//! `cargo test --release -p curvyo-editor-wasm --test acceptance_0152_budget_tester -- --nocapture`.

#![allow(clippy::unwrap_used, missing_docs)]

mod cpu_time;

use std::time::Duration;

use cpu_time::cpu_time_of;
use curvyo_ui_core::library_browser::{
    BrowseItem, BrowseList, BrowseSource, BrowseState, Browser, ItemKind, Sort,
};

const WORDS: [&str; 12] = [
    "arrow", "heart", "home", "weather", "cloud", "box", "finger", "circle", "left", "sun",
    "account", "lock",
];

fn list(count: usize) -> BrowseList {
    let source = BrowseSource {
        key: "set:api#material-symbols".to_owned(),
        label: "Material Symbols".to_owned(),
        host: "set:api#material-symbols".to_owned(),
        prefix: "material-symbols".to_owned(),
        place: "Iconify › Material Symbols".to_owned(),
        category: Some("Material".to_owned()),
        palette: Some(false),
        ..BrowseSource::default()
    };
    let items = (0..count)
        .map(|i| {
            let a = WORDS[i % WORDS.len()];
            let b = WORDS[(i / WORDS.len()) % WORDS.len()];
            BrowseItem {
                kind: ItemKind::Icon,
                name: format!("{a}-{b}-{i}"),
                source: 0,
                aliases: vec![format!("{a}-outline-{i}")],
                tags: Vec::new(),
                page_names: Vec::new(),
                groups: vec!["Group".to_owned()],
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
        sources: vec![source],
        items,
        cards: Vec::new(),
        groups: Vec::new(),
        search_status: String::new(),
    }
}

#[test]
fn criterion_22_a_keystroke_over_50000_items_costs_at_most_100_ms_of_cpu() {
    let mut browser = Browser::new(list(50_000));
    // Warm the page the way a maker does (one keystroke before the measured one).
    let mut state = BrowseState {
        text: "h".to_owned(),
        sort: Sort::Best,
        own_sort: Sort::Name,
        ..BrowseState::default()
    };
    let _ = browser.query(&state);
    state.text = "hart".to_owned();
    let (answer, cpu) = cpu_time_of(|| browser.query(&state));
    eprintln!(
        "50,000 items, typed hart: {} matches, {cpu:?} CPU",
        answer.count
    );
    assert!(answer.count > 0, "the fuzzy word finds items (heart)");
    assert!(
        cpu <= Duration::from_millis(100),
        "a keystroke must take at most 100 ms of thread CPU, took {cpu:?}"
    );
}
