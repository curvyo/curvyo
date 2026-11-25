#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Acceptance tests of spec `0152` for `curvyo-ui-core::library_browser`: the filters by AND with
//! their counts and disabled states (criteria 11 to 16), the sorts (criterion 17), the search over
//! the fetched items (criteria 18, 19), the second level (criterion 8), the Not loaded cards
//! (criterion 10), and the colour refinement (criterion 14). Written from the specification
//! before the implementation was read (playbook: tester). The keystroke budget is in
//! `curvyo-editor-wasm/tests/acceptance_0152_budget_tester.rs`.

use curvyo_ui_core::library_browser::{
    BrowseCard, BrowseItem, BrowseList, BrowseSource, BrowseState, Browser, CardKind, ColourFilter,
    ItemKind, Sort,
};

const MONO: &str = "set:api#material-symbols";
const COLOUR: &str = "set:api#noto";
const CATALOGUE: &str = "online:curvyo";
const LOCAL: &str = "local:work";

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

/// Sources: 0 My documents (local), 1 Curvyo catalogue, 2 Material Symbols (monochrome, category
/// "Material"), 3 Noto Emoji (colour, category "Emoji").
fn sources() -> Vec<BrowseSource> {
    let mut mono = source(MONO, "Material Symbols");
    "material-symbols".clone_into(&mut mono.prefix);
    mono.category = Some("Material".to_owned());
    mono.palette = Some(false);
    let mut colour = source(COLOUR, "Noto Emoji");
    "noto".clone_into(&mut colour.prefix);
    colour.category = Some("Emoji".to_owned());
    colour.palette = Some(true);
    let mut local = source(LOCAL, "Work");
    local.local = true;
    vec![local, source(CATALOGUE, "Curvyo catalogue"), mono, colour]
}

/// Items 0 to 7: arrow-left (mono icon, group Arrows), heart (mono), sun (mono, group Weather),
/// smile (colour), finger-box (template tagged customizable), plain-box (template), Lid and Base
/// (local pages, Base older).
fn main_list() -> BrowseList {
    let mut arrow = item(ItemKind::Icon, "arrow-left", 2);
    arrow.groups = vec!["Arrows".to_owned()];
    let heart = item(ItemKind::Icon, "heart", 2);
    let mut sun = item(ItemKind::Icon, "sun", 2);
    sun.groups = vec!["Weather".to_owned()];
    let smile = item(ItemKind::Icon, "smile", 3);
    let mut finger = item(ItemKind::Template, "finger-box", 1);
    finger.tags = vec!["customizable".to_owned()];
    finger.tagged_customizable = true;
    let plain = item(ItemKind::Template, "plain-box", 1);
    let mut lid = item(ItemKind::Page, "Lid", 0);
    lid.modified = 500;
    let mut base = item(ItemKind::Page, "Base", 0);
    base.modified = 100;
    BrowseList {
        sources: sources(),
        items: vec![arrow, heart, sun, smile, finger, plain, lid, base],
        cards: Vec::new(),
        groups: Vec::new(),
        search_status: String::new(),
    }
}

fn ids(list: BrowseList, state: &BrowseState) -> Vec<u32> {
    Browser::new(list).query(state).order
}

fn state() -> BrowseState {
    BrowseState::default()
}

fn with(state: BrowseState, edit: impl FnOnce(&mut BrowseState)) -> BrowseState {
    let mut state = state;
    edit(&mut state);
    state
}

fn row<'a>(
    rows: &'a [curvyo_ui_core::library_browser::FacetRow],
    label: &str,
) -> &'a curvyo_ui_core::library_browser::FacetRow {
    rows.iter()
        .find(|row| row.label == label)
        .unwrap_or_else(|| panic!("no row {label:?} in {rows:?}"))
}

// Criterion 17 and 4: the default sort is Name A to Z, case-insensitive.

#[test]
fn criterion_17_the_default_sort_is_name_a_to_z_case_insensitive() {
    let answer = Browser::new(main_list()).query(&state());
    assert_eq!(
        answer.order,
        [0, 7, 4, 1, 6, 5, 3, 2],
        "arrow-left, Base, finger-box, heart, Lid, plain-box, smile, sun"
    );
    assert_eq!(answer.total, 8);
    assert_eq!(answer.count, 8);
    assert!(
        answer.empty.is_empty(),
        "something is shown, so no empty text"
    );
}

#[test]
fn criterion_17_digits_sort_as_numbers_and_ties_keep_item_order() {
    let mut list = main_list();
    list.items = vec![
        item(ItemKind::Icon, "Item 10", 2),
        item(ItemKind::Icon, "item 2", 2),
        item(ItemKind::Icon, "Apple", 2),
        item(ItemKind::Icon, "Sun", 2),
        item(ItemKind::Icon, "Sun", 3),
    ];
    let order = ids(list, &state());
    assert_eq!(
        order,
        [2, 1, 0, 3, 4],
        "Apple, item 2, Item 10, then the two Suns in item order"
    );
}

#[test]
fn criterion_17_source_then_name_orders_by_source_first() {
    let mut list = main_list();
    list.items = vec![
        item(ItemKind::Icon, "Item 10", 2),
        item(ItemKind::Icon, "item 2", 2),
        item(ItemKind::Icon, "Apple", 3),
        item(ItemKind::Icon, "Sun", 2),
        item(ItemKind::Icon, "Sun", 3),
    ];
    let order = ids(list, &with(state(), |s| s.sort = Sort::Source));
    assert_eq!(
        order,
        [1, 0, 3, 2, 4],
        "source 2 first (item 2, Item 10, Sun), then source 3"
    );
}

#[test]
fn criterion_17_newest_puts_local_pages_first_by_change_time_then_the_rest_by_name() {
    let order = ids(main_list(), &with(state(), |s| s.sort = Sort::Newest));
    assert_eq!(
        order,
        [6, 7, 0, 4, 1, 5, 3, 2],
        "Lid (500), Base (100), then arrow-left, finger-box, heart, plain-box, smile, sun"
    );
}

#[test]
fn criterion_17_best_match_orders_by_score_and_ties_keep_the_own_sort() {
    let answer = Browser::new(main_list()).query(&with(state(), |s| {
        s.text = "box".to_owned();
        s.sort = Sort::Best;
        s.own_sort = Sort::Name;
    }));
    assert_eq!(
        answer.order,
        [4, 5],
        "finger-box and plain-box tie on the name word box; Name order breaks the tie"
    );
}

#[test]
fn criterion_17_best_match_puts_the_exact_word_before_a_typo() {
    let mut list = main_list();
    list.items = vec![
        item(ItemKind::Icon, "hart", 2),
        item(ItemKind::Icon, "heart", 2),
    ];
    let order = ids(
        list,
        &with(state(), |s| {
            s.text = "heart".to_owned();
            s.sort = Sort::Best;
        }),
    );
    assert_eq!(order, [1, 0]);
}

// Criterion 18 to 19 in the browser: typing filters live and locally; words are AND.

#[test]
fn criterion_18_a_typo_finds_the_item_and_bx_finds_nothing() {
    let mut list = main_list();
    list.items = vec![
        item(ItemKind::Icon, "heart", 2),
        item(ItemKind::Icon, "box", 2),
    ];
    assert_eq!(
        ids(list.clone(), &with(state(), |s| s.text = "hart".to_owned())),
        [0]
    );
    assert_eq!(
        ids(list.clone(), &with(state(), |s| s.text = "bxo".to_owned())),
        [1]
    );
    let answer = Browser::new(list).query(&with(state(), |s| s.text = "bx".to_owned()));
    assert_eq!(answer.order, Vec::<u32>::new());
}

#[test]
fn criterion_19_every_word_must_match_and_a_category_word_counts() {
    let mut list = main_list();
    list.items = vec![
        item(ItemKind::Template, "finger-box", 1),
        item(ItemKind::Icon, "heart", 2),
    ];
    let answer =
        Browser::new(list.clone()).query(&with(state(), |s| s.text = "finger box".to_owned()));
    assert_eq!(answer.order, [0]);
    let answer = Browser::new(list).query(&with(state(), |s| s.text = "finger heart".to_owned()));
    assert_eq!(answer.order, Vec::<u32>::new());
}

#[test]
fn criterion_11_nothing_matches_names_the_filters_and_never_shows_a_blank_grid() {
    let answer = Browser::new(main_list()).query(&with(state(), |s| {
        s.text = "hrat".to_owned();
        s.colour = ColourFilter::Mono;
    }));
    assert_eq!(answer.order, Vec::<u32>::new());
    assert!(
        answer.empty.contains("Nothing matches"),
        "text: {:?}",
        answer.empty
    );
    assert!(
        answer.empty.contains("hrat"),
        "it names the search: {:?}",
        answer.empty
    );
    assert!(
        answer.empty.contains("Monochrome"),
        "it names the active filter: {:?}",
        answer.empty
    );
    assert_eq!(answer.active, 1, "one filter is active");
}

// Criterion 11 and 12: the Source filter, AND across filters, OR within one.

#[test]
fn criterion_12_two_sources_are_one_or_in_the_source_filter() {
    let answer = Browser::new(main_list()).query(&with(state(), |s| {
        s.sources = vec![MONO.to_owned(), CATALOGUE.to_owned()];
    }));
    assert_eq!(
        answer.order,
        [0, 4, 1, 5, 2],
        "the monochrome icons and the template files, sorted by name"
    );
}

#[test]
fn criterion_11_filters_combine_by_and() {
    let answer = Browser::new(main_list()).query(&with(state(), |s| {
        s.sources = vec![MONO.to_owned(), CATALOGUE.to_owned()];
        s.customizable = true;
    }));
    assert_eq!(
        answer.order,
        [0, 4, 1, 2],
        "the monochrome icons of the two sources are customizable (criterion 15): arrow-left, finger-box, heart, sun"
    );
    assert_eq!(answer.active, 2, "two filters are active");
}

#[test]
fn criterion_11_subject_names_the_count_of_the_items_shown() {
    let mut list = main_list();
    list.items.clear();
    for i in 0..4210 {
        let source = if i < 132 { 3 } else { 2 };
        list.items
            .push(item(ItemKind::Icon, &format!("icon-{i:05}"), source));
    }
    let answer = Browser::new(list).query(&with(state(), |s| s.sources = vec![COLOUR.to_owned()]));
    assert_eq!(answer.total, 4210);
    assert_eq!(answer.count, 132);
    assert_eq!(
        answer.subject, "132 of 4,210 items",
        "the subject line of criterion 4"
    );
}

// Criterion 14: the Colour filter and its data.

#[test]
fn criterion_14_monochrome_keeps_mono_icons_and_hides_items_without_colour_data() {
    let answer = Browser::new(main_list()).query(&with(state(), |s| s.colour = ColourFilter::Mono));
    assert_eq!(
        answer.order,
        [0, 1, 2],
        "arrow-left, heart, sun; the templates and pages have no colour data"
    );
}

#[test]
fn criterion_14_colour_keeps_icons_with_their_own_colours() {
    let answer =
        Browser::new(main_list()).query(&with(state(), |s| s.colour = ColourFilter::Colour));
    assert_eq!(answer.order, [3]);
}

#[test]
fn criterion_14_no_colour_data_disables_the_colour_filter() {
    let mut list = main_list();
    list.items = vec![item(ItemKind::Page, "Lid", 0)];
    let answer = Browser::new(list).query(&state());
    assert!(!answer.has_colour, "no fetched item has colour data");
    assert!(!answer.has_customizable, "no fetched item is customizable");
    assert!(
        answer.has_categories,
        "a local page has its workspace as category"
    );
}

#[test]
fn criterion_14_the_colour_data_of_the_main_list_is_present() {
    let answer = Browser::new(main_list()).query(&state());
    assert!(answer.has_colour);
    assert!(answer.has_customizable);
    assert!(answer.has_categories);
}

// Criterion 15: Customizable is a monochrome icon or a template tagged customizable.

#[test]
fn criterion_15_customizable_is_a_mono_icon_or_a_tagged_template() {
    let answer = Browser::new(main_list()).query(&with(state(), |s| s.customizable = true));
    assert_eq!(
        answer.order,
        [0, 4, 1, 2],
        "arrow-left, finger-box, heart, sun; not plain-box, not smile, not the pages"
    );
}

// Criterion 13: Category, from the set's category and the icon's group; Uncategorised last.

#[test]
fn criterion_13_categories_count_the_fetched_items_and_put_uncategorised_last() {
    let answer = Browser::new(main_list()).query(&state());
    let labels: Vec<&str> = answer.categories.iter().map(|r| r.label.as_str()).collect();
    assert_eq!(
        labels.last().copied(),
        Some("Uncategorised"),
        "last: {labels:?}"
    );
    assert_eq!(
        row(&answer.categories, "Material").count,
        3,
        "arrow-left, heart, sun carry the set's category"
    );
    assert_eq!(
        row(&answer.categories, "Arrows").count,
        1,
        "the group of arrow-left"
    );
    assert_eq!(
        row(&answer.categories, "Weather").count,
        1,
        "the group of sun"
    );
    assert_eq!(row(&answer.categories, "Emoji").count, 1);
    assert_eq!(
        row(&answer.categories, "Uncategorised").count,
        2,
        "the two templates have no category"
    );
}

#[test]
fn criterion_13_facet_counts_pass_the_other_filters() {
    let answer = Browser::new(main_list()).query(&with(state(), |s| s.colour = ColourFilter::Mono));
    assert_eq!(row(&answer.categories, "Material").count, 3);
    assert_eq!(
        row(&answer.categories, "Emoji").count,
        0,
        "smile is not monochrome"
    );
}

#[test]
fn criterion_13_a_category_filter_narrows_by_and_with_colour() {
    let answer = Browser::new(main_list()).query(&with(state(), |s| {
        s.categories = vec![
            row(
                &Browser::new(main_list()).query(&state()).categories,
                "Emoji",
            )
            .key
            .clone(),
        ];
        s.colour = ColourFilter::Mono;
    }));
    assert!(
        answer.order.is_empty(),
        "the Emoji category has no monochrome item"
    );
    assert!(answer.empty.contains("Nothing matches"));
}

#[test]
fn criterion_13_without_categories_the_answer_says_so() {
    let mut list = main_list();
    list.items = vec![item(ItemKind::Template, "plain-box", 1)];
    let answer = Browser::new(list).query(&state());
    assert!(
        !answer.has_categories,
        "no fetched item has a category: one muted line instead of rows"
    );
    assert!(
        answer.categories.is_empty(),
        "no rows while nothing has a category: {:?}",
        answer.categories
    );
}

// Criterion 16: a change scrolls to the top (the order starts again) and the chips count.

#[test]
fn criterion_16_active_filters_are_counted() {
    let answer = Browser::new(main_list()).query(&with(state(), |s| {
        s.colour = ColourFilter::Mono;
        s.customizable = true;
        s.sources = vec![MONO.to_owned()];
    }));
    assert_eq!(answer.active, 3, "Source, Colour and Customizable");
}

// Criterion 8: a second level for documents; the search matches the page names too.

fn document_list() -> BrowseList {
    let mut doc = item(ItemKind::Document, "Box job", 0);
    doc.pages = 2;
    doc.page_names = vec!["Lid".to_owned(), "Base".to_owned()];
    let mut lid = item(ItemKind::Page, "Lid", 0);
    lid.parent = Some(0);
    let mut base = item(ItemKind::Page, "Base", 0);
    base.parent = Some(0);
    BrowseList {
        sources: sources(),
        items: vec![doc, lid, base, item(ItemKind::Icon, "heart", 2)],
        cards: Vec::new(),
        groups: Vec::new(),
        search_status: String::new(),
    }
}

#[test]
fn criterion_8_the_top_level_shows_the_document_not_its_pages() {
    let order = ids(document_list(), &state());
    assert_eq!(
        order,
        [0, 3],
        "Box job and heart; the pages are the second level"
    );
}

#[test]
fn criterion_8_the_second_level_shows_the_pages_of_the_document() {
    let mut s = state();
    s.level = Some(0);
    let answer = Browser::new(document_list()).query(&s);
    assert_eq!(answer.order.len(), 2, "two pages");
    assert!(
        answer.order.iter().all(|i| *i == 1 || *i == 2),
        "only the pages: {:?}",
        answer.order
    );
}

#[test]
fn criterion_8_a_search_for_a_page_name_matches_the_document() {
    let mut s = state();
    s.text = "lid".to_owned();
    let answer = Browser::new(document_list()).query(&s);
    assert!(
        answer.order.contains(&0),
        "the document matches by its page name: {:?}",
        answer.order
    );
}

// Criterion 10 (revised by Part G, criteria 26 to 28): cards live in the Sets view only.

fn card(kind: CardKind, name: &str, key: &str) -> BrowseCard {
    BrowseCard {
        kind,
        name: name.to_owned(),
        key: key.to_owned(),
        host: "api".to_owned(),
        glyph: "git".to_owned(),
        prefix: String::new(),
        samples: Vec::new(),
        total: 0,
        category: None,
        palette: None,
        licence: None,
        author: String::new(),
        failure: String::new(),
        offline: false,
        loaded: false,
        group: "g".to_owned(),
        narrow: String::new(),
        count: 0,
        pages: 0,
    }
}

#[test]
fn criterion_10_cards_are_not_items_and_live_in_the_sets_view() {
    let mut list = main_list();
    list.cards = vec![
        card(CardKind::Source, "Community templates", "online:git"),
        card(CardKind::Set, "Tabler", "set:api#tabler"),
    ];
    list.groups = vec![curvyo_ui_core::library_browser::BrowseGroup {
        key: "g".to_owned(),
        label: "api".to_owned(),
        rank: 3,
    }];
    let mut browser = Browser::new(list);
    let answer = browser.query(&state());
    assert_eq!(answer.order.len(), 8, "the cards are not items");
    let sets = browser.query_sets(&curvyo_ui_core::library_browser::SetsState::default());
    assert_eq!(sets.count, 2, "both cards are in the Sets view");
}

// Criterion 14 (refinement, Proposal): the refined colour applies from the next change only.

#[test]
fn criterion_14_a_loaded_body_with_fixed_colours_refines_only_on_the_next_change() {
    let mut browser = Browser::new(main_list());
    let mut mono = state();
    mono.colour = ColourFilter::Mono;
    assert!(
        browser.query(&mono).order.contains(&0),
        "arrow-left is monochrome before its body is read"
    );
    browser.note_body(0, true);
    assert!(
        browser.query(&mono).order.contains(&0),
        "the same state does not refine while the maker looks at the tile"
    );
    let mut searched = mono.clone();
    searched.text = "arrow".to_owned();
    let answer = browser.query(&searched);
    assert!(
        !answer.order.contains(&0),
        "the next change applies the refinement: {:?}",
        answer.order
    );
    let mut colour = state();
    colour.colour = ColourFilter::Colour;
    assert!(
        browser.query(&colour).order.contains(&0),
        "it now counts as Colour"
    );
}
