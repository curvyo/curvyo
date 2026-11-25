#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Acceptance tests of spec `0152-library-browser` Part G (criteria 26 to 33) and the changed
//! criteria 7, 8, 10, 23 for `curvyo-ui-core`: the Items and Sets views, opening a card or set,
//! the place history with Back, the facet selection rules, the tile actions without Load buttons,
//! and the hosting guide. Written from the specification before the implementation was read
//! (playbook: tester; the seams are the public signatures only).

use curvyo_library_core::Catalogue;
use curvyo_ui_core::library_browser::{
    BrowseCard, BrowseGroup, BrowseItem, BrowseList, BrowseSource, BrowseState, Browser, CardKind,
    ColourFilter, EXAMPLE_REPOSITORY, ItemKind, PUBLIC_ONLY_TEXT, Place, PlaceHistory, PlaceView,
    SetsState, Sort, back_name, hosting_guide, narrowed_to, restored, select_rows,
};

/// The spec's Back history limit (criterion 30).
const MAX_PLACES: usize = 50;

fn source(key: &str, label: &str) -> BrowseSource {
    BrowseSource {
        key: key.to_owned(),
        label: label.to_owned(),
        host: key.to_owned(),
        place: label.to_owned(),
        ..BrowseSource::default()
    }
}

fn item(kind: ItemKind, name: &str, source: u32) -> BrowseItem {
    BrowseItem {
        kind,
        name: name.to_owned(),
        source,
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

fn card(kind: CardKind, name: &str, key: &str, author: &str, group: &str) -> BrowseCard {
    BrowseCard {
        kind,
        name: name.to_owned(),
        key: key.to_owned(),
        host: "api".to_owned(),
        glyph: "iconify".to_owned(),
        prefix: String::new(),
        samples: Vec::new(),
        total: 0,
        category: None,
        palette: None,
        licence: None,
        author: author.to_owned(),
        failure: String::new(),
        offline: false,
        loaded: false,
        group: group.to_owned(),
        narrow: String::new(),
        count: 0,
        pages: 0,
    }
}

/// A set whose icon names are loaded: its card carries the set's category and palette.
fn loaded_set(name: &str, key: &str) -> BrowseCard {
    let mut set = card(CardKind::Set, name, key, "Google", "iconify");
    set.loaded = true;
    set.category = Some("Material".to_owned());
    set.palette = Some(false);
    set.total = 3;
    set
}

/// Sources 0 My documents, 1 Curvyo catalogue (loaded, 2 items), 2 Material Symbols (loaded,
/// 3 icons); cards: Tabler (not loaded, author "Pawel Kuna") and the Git catalogue "Community".
fn list() -> BrowseList {
    let mut mono = source("set:api#material-symbols", "Material Symbols");
    mono.category = Some("Material".to_owned());
    mono.palette = Some(false);
    let mut local = source("local:work", "Work");
    local.local = true;
    let mut heart = item(ItemKind::Icon, "heart", 2);
    heart.groups = vec!["Shapes".to_owned()];
    BrowseList {
        sources: vec![local, source("online:curvyo", "Curvyo catalogue"), mono],
        items: vec![
            item(ItemKind::Icon, "arrow-left", 2),
            heart,
            item(ItemKind::Icon, "sun", 2),
            item(ItemKind::Template, "finger-box", 1),
            item(ItemKind::Template, "plain-box", 1),
        ],
        groups: vec![
            BrowseGroup {
                key: "iconify".to_owned(),
                label: "api.iconify.design".to_owned(),
                rank: 1,
            },
            BrowseGroup {
                key: "git".to_owned(),
                label: "Git catalogues".to_owned(),
                rank: 2,
            },
        ],
        cards: vec![
            loaded_set("Material Symbols", "set:api#material-symbols"),
            card(
                CardKind::Set,
                "Tabler",
                "set:api#tabler",
                "Pawel Kuna",
                "iconify",
            ),
            card(CardKind::Source, "Community", "online:git", "", "git"),
        ],
        search_status: String::new(),
    }
}

fn place(view: PlaceView, items: BrowseState, sets: SetsState, current: Option<u32>) -> Place {
    Place {
        view,
        items,
        sets,
        current,
        anchor: None,
        opened: None,
    }
}

// Criterion 26: Items holds the tiles and no cards; Sets holds one card per set or source.

#[test]
fn criterion_26_items_hold_tiles_and_never_cards() {
    let answer = Browser::new(list()).query(&BrowseState::default());
    assert_eq!(answer.total, 5, "the five fetched tiles, no card");
    assert!(
        answer.order.iter().all(|i| *i < 5),
        "a card index never appears in the Items order: {:?}",
        answer.order
    );
}

#[test]
fn criterion_26_sets_hold_one_card_per_source_or_set_grouped_by_source() {
    let mut browser = Browser::new(list());
    let answer = browser.query_sets(&SetsState::default());
    let cards: Vec<u32> = answer.groups.iter().flat_map(|g| g.cards.clone()).collect();
    assert!(
        cards.contains(&0),
        "the Tabler card is in the Sets view: {:?}",
        answer.groups
    );
    assert!(
        cards.contains(&1),
        "the Community card is in the Sets view: {:?}",
        answer.groups
    );
}

#[test]
fn criterion_26_sets_view_has_sources_colour_and_category_facets() {
    let mut browser = Browser::new(list());
    let answer = browser.query_sets(&SetsState::default());
    assert!(!answer.sources.is_empty(), "the Sources facet has rows");
    assert!(
        answer.categories.iter().any(|r| r.label == "Material"),
        "the Category facet lists the set category: {:?}",
        answer.categories
    );
}

// Criterion 26 and 27: the Sets search filters the cards by name, category and author.

#[test]
fn criterion_27_the_sets_search_matches_name_and_author() {
    let mut browser = Browser::new(list());
    let by_name = browser.query_sets(&SetsState {
        text: "tabler".to_owned(),
        ..SetsState::default()
    });
    assert!(
        by_name.groups.iter().any(|g| g.cards.contains(&1)),
        "Tabler by name"
    );
    let by_author = browser.query_sets(&SetsState {
        text: "kuna".to_owned(),
        ..SetsState::default()
    });
    assert!(
        by_author.groups.iter().any(|g| g.cards.contains(&1)),
        "Tabler by author"
    );
    assert!(
        !by_author.groups.iter().any(|g| g.cards.contains(&2)),
        "the Community card has no such author"
    );
}

#[test]
fn criterion_27_no_set_matches_names_the_search() {
    let answer = Browser::new(list()).query_sets(&SetsState {
        text: "hrat".to_owned(),
        ..SetsState::default()
    });
    assert_eq!(answer.empty, "No set matches \"hrat\".");
}

// Criterion 27: Items empty state says so and offers the Sets view.

#[test]
fn criterion_27_items_with_nothing_fetched_say_no_items_yet_and_offer_sets() {
    let empty = BrowseList {
        sources: Vec::new(),
        items: Vec::new(),
        cards: vec![card(
            CardKind::Set,
            "Tabler",
            "set:api#tabler",
            "",
            "iconify",
        )],
        groups: Vec::new(),
        search_status: String::new(),
    };
    let answer = Browser::new(empty).query(&BrowseState::default());
    assert!(answer.show_sets, "the Show sets button is offered");
    assert!(
        answer.empty.contains("No items yet."),
        "the empty text: {:?}",
        answer.empty
    );
}

// Criterion 27: opening a set, catalogue or folder narrows the Items view to it alone.

#[test]
fn criterion_27_opening_a_set_narrows_items_to_that_set_alone() {
    let state = narrowed_to("set:api#material-symbols");
    assert_eq!(state.sources, ["set:api#material-symbols"]);
    assert!(state.text.is_empty(), "search empty");
    assert!(state.categories.is_empty(), "Category none");
    assert_eq!(state.colour, ColourFilter::All);
    assert!(!state.customizable, "Customizable off");
    assert_eq!(state.sort, Sort::Name, "sort Name");
    let answer = Browser::new(list()).query(&state);
    assert_eq!(
        answer.order,
        [0, 1, 2],
        "the three icons of that set, by name"
    );
}

// Criterion 30: the history of places, no Forward, max 50, Back restores the place.

#[test]
fn criterion_30_the_history_keeps_at_most_fifty_places() {
    let mut history = PlaceHistory::default();
    for i in 0..(MAX_PLACES + 10) {
        history.push(
            place(
                PlaceView::Items,
                BrowseState::default(),
                SetsState::default(),
                Some(u32::try_from(i).unwrap()),
            ),
            false,
        );
    }
    assert_eq!(history.len(), MAX_PLACES, "the oldest entries are dropped");
    assert!(!history.is_empty());
}

#[test]
fn criterion_30_back_returns_the_newest_place_and_then_nothing() {
    let mut history = PlaceHistory::default();
    history.push(
        place(
            PlaceView::Items,
            BrowseState::default(),
            SetsState::default(),
            Some(1),
        ),
        false,
    );
    history.push(
        place(
            PlaceView::Sets,
            BrowseState::default(),
            SetsState::default(),
            Some(0),
        ),
        true,
    );
    assert_eq!(history.peek().map(|p| p.view), Some(PlaceView::Sets));
    let first = history.back().expect("a place to go back to");
    assert_eq!(
        first.view,
        PlaceView::Sets,
        "the newest place comes back first"
    );
    let second = history.back().expect("the older place");
    assert_eq!(second.current, Some(1));
    assert!(
        history.back().is_none(),
        "no Forward and nothing older: back stops"
    );
}

#[test]
fn criterion_30_escape_goes_back_only_when_the_newest_entry_was_left_by_opening() {
    let mut history = PlaceHistory::default();
    history.push(
        place(
            PlaceView::Items,
            BrowseState::default(),
            SetsState::default(),
            None,
        ),
        true,
    );
    assert!(history.escape_goes_back(), "left by opening a set");
    let mut switched = PlaceHistory::default();
    switched.push(
        place(
            PlaceView::Items,
            BrowseState::default(),
            SetsState::default(),
            None,
        ),
        false,
    );
    assert!(
        !switched.escape_goes_back(),
        "left by a view switch: Escape closes"
    );
}

#[test]
fn criterion_30_back_restores_the_view_search_and_current_item() {
    let list = list();
    let saved = place(
        PlaceView::Items,
        BrowseState {
            text: "hart".to_owned(),
            ..BrowseState::default()
        },
        SetsState::default(),
        Some(1),
    );
    let back = restored(&list, saved.clone());
    assert_eq!(back.view, PlaceView::Items);
    assert_eq!(back.items.text, "hart", "the search is restored");
    assert_eq!(back.current, Some(1), "the current item is restored");
    let name = back_name(&list, &saved);
    assert!(!name.is_empty(), "the Back button names its place");
}

#[test]
fn criterion_30_back_restores_the_sets_view_and_its_search() {
    let saved = place(
        PlaceView::Sets,
        BrowseState::default(),
        SetsState {
            text: "tab".to_owned(),
            ..SetsState::default()
        },
        Some(0),
    );
    let back = restored(&list(), saved);
    assert_eq!(back.view, PlaceView::Sets);
    assert_eq!(back.sets.text, "tab");
}

// Criterion 31: facet lists are multi-select; a plain press replaces, a press on the only
// selected row clears, Shift adds or removes.

fn keys(list: &[&str]) -> Vec<String> {
    list.iter().map(|k| (*k).to_owned()).collect()
}

fn sorted(mut list: Vec<String>) -> Vec<String> {
    list.sort();
    list
}

#[test]
fn criterion_31_a_plain_press_replaces_the_selection() {
    assert_eq!(select_rows(&keys(&["a"]), "b", &[], false), keys(&["b"]));
}

#[test]
fn criterion_31_a_press_on_the_only_selected_row_clears_it() {
    assert_eq!(
        select_rows(&keys(&["a"]), "a", &[], false),
        Vec::<String>::new()
    );
}

#[test]
fn criterion_31_shift_press_adds_a_row() {
    assert_eq!(
        sorted(select_rows(&keys(&["a"]), "b", &[], true)),
        keys(&["a", "b"])
    );
}

#[test]
fn criterion_31_shift_press_removes_a_selected_row() {
    assert_eq!(
        sorted(select_rows(&keys(&["a", "b"]), "a", &[], true)),
        keys(&["b"])
    );
}

#[test]
fn criterion_31_a_plain_press_on_a_row_among_several_selects_only_that_row() {
    assert_eq!(
        select_rows(&keys(&["a", "b"]), "a", &[], false),
        keys(&["a"])
    );
}

// Criterion 29: the tile has no Load or Import page button; the detail bar keeps only Show
// pages and Pick elements….

#[test]
fn criterion_29_tile_actions_are_only_show_pages_and_pick_elements() {
    let mut local = list();
    local.sources.push(BrowseSource {
        local: true,
        ..source("local:box", "Box job")
    });
    let mut doc = item(ItemKind::Document, "Box job", 3);
    doc.pages = 2;
    doc.page_names = vec!["Lid".to_owned(), "Base".to_owned()];
    let mut page = item(ItemKind::Page, "Lid", 3);
    page.parent = Some(u32::try_from(local.items.len()).unwrap());
    local.items.push(doc);
    local.items.push(page);
    let browser = Browser::new(local.clone());
    for index in 0..u32::try_from(local.items.len()).unwrap() {
        let tile = browser.tile(index).expect("every item has tile words");
        for action in &tile.actions {
            assert!(
                *action == "show_pages" || *action == "pick",
                "tile {index} has the action {action:?}: no Load or Import page button"
            );
        }
    }
}

#[test]
fn criterion_29_a_template_with_pages_shows_no_load_button() {
    let browser = Browser::new(list());
    let tile = browser.tile(3).expect("the template tile");
    assert!(
        tile.actions
            .iter()
            .all(|a| !a.contains("Load") && !a.contains("Import")),
        "actions: {:?}",
        tile.actions
    );
}

// Criterion 32 and 33: the hosting guide, its example, and the public-only rule.

#[test]
fn criterion_32_the_guide_example_parses_as_a_catalogue() {
    let guide = hosting_guide();
    let catalogue = Catalogue::parse(guide.example.as_bytes())
        .expect("the example catalogue is accepted by the catalogue parser");
    assert_eq!(catalogue.skipped, 0, "no entry of the example is skipped");
    assert!(
        !catalogue.templates.is_empty(),
        "the example lists templates"
    );
}

#[test]
fn criterion_32_the_guide_names_the_layout_steps_fields_and_the_example_repository() {
    let guide = hosting_guide();
    assert!(
        guide.intro.contains("curvyo-catalogue.json"),
        "the index file is named"
    );
    assert!(!guide.layout.is_empty(), "the folder layout is shown");
    assert!(!guide.steps.is_empty(), "the steps are shown");
    assert!(!guide.fields.is_empty(), "the field rules are shown");
    assert!(!guide.rules.is_empty(), "the limits are shown");
    assert_eq!(guide.example_url, EXAMPLE_REPOSITORY);
    assert_eq!(EXAMPLE_REPOSITORY, "https://github.com/curvyo/templates");
}

#[test]
fn criterion_32_the_guide_does_not_name_an_index_script_that_does_not_exist() {
    let guide = hosting_guide();
    let mut text = guide.intro.clone();
    for step in &guide.steps {
        text.push_str(step);
    }
    assert!(
        !text.to_lowercase().contains("script"),
        "H12: no script is named yet: {text}"
    );
}

#[test]
fn criterion_33_the_public_only_text_is_the_one_the_form_and_the_guide_show() {
    assert_eq!(PUBLIC_ONLY_TEXT, "The repository must be public.");
    assert!(
        hosting_guide().private.contains(PUBLIC_ONLY_TEXT),
        "the guide says it: {:?}",
        hosting_guide().private
    );
}

// Criterion 7, 8, 10 and 23 in the browser: a press selects and loads nothing; a page of a
// document is a second level.

#[test]
fn criterion_8_a_document_opens_its_pages_as_a_second_level() {
    let mut local = list();
    let mut doc = item(ItemKind::Document, "Box job", 0);
    doc.pages = 2;
    let doc_index = u32::try_from(local.items.len()).unwrap();
    local.items.push(doc);
    let mut lid = item(ItemKind::Page, "Lid", 0);
    lid.parent = Some(doc_index);
    let mut base = item(ItemKind::Page, "Base", 0);
    base.parent = Some(doc_index);
    local.items.push(lid);
    local.items.push(base);
    let mut browser = Browser::new(local);
    let top = browser.query(&BrowseState::default());
    assert!(top.order.contains(&doc_index), "the document is a tile");
    let second = browser.query(&BrowseState {
        level: Some(doc_index),
        ..BrowseState::default()
    });
    assert_eq!(second.order.len(), 2, "its two pages");
}
