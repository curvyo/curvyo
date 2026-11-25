#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Acceptance tests of spec `0152` for `curvyo-library-core`: the fuzzy matcher (criteria 18 to
//! 21), the Iconify parser fields (criteria 13, 14, 25) and the tile colour rule (criterion 6).
//! Written from the specification before the implementation was read (playbook: tester).

use curvyo_library_core::{
    IconBody, Rgb8, Scratch, SearchIndex, WEIGHT_ALIAS, WEIGHT_CATEGORY, WEIGHT_NAME,
    WEIGHT_SOURCE, has_fixed_colour, icon_tile_svg, parse_collections, parse_set_listing,
};

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(format!(
        "{}/tests/fixtures/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
}

/// One index over `items`; each item is its fields with their weights.
fn index(items: &[&[(&str, u8)]]) -> SearchIndex {
    let mut index = SearchIndex::new();
    for fields in items {
        index.push(fields);
    }
    index
}

/// The matching item indices of `query`, best first.
fn found(index: &SearchIndex, query: &str) -> Vec<u32> {
    let mut scratch = Scratch::default();
    let mut out = Vec::new();
    index.rank(query, &mut scratch, &mut out);
    out.iter().map(|(_, item)| *item).collect()
}

/// The names of the test set: heart, box, weather, arrow-left-circle, circle.
fn names() -> SearchIndex {
    index(&[
        &[("heart", WEIGHT_NAME)],
        &[("box", WEIGHT_NAME)],
        &[("weather", WEIGHT_NAME)],
        &[("arrow-left-circle", WEIGHT_NAME)],
        &[("circle", WEIGHT_NAME)],
    ])
}

// Criterion 20: the examples the spec names.

#[test]
fn criterion_20_hart_and_haert_find_heart() {
    let index = names();
    assert_eq!(found(&index, "hart"), [0]);
    assert_eq!(found(&index, "haert"), [0]);
}

#[test]
fn criterion_20_a_subsequence_of_three_letters_finds_heart() {
    assert_eq!(found(&names(), "hrt"), [0]);
}

#[test]
fn criterion_20_bx_finds_nothing_and_box_or_bxo_finds_box() {
    let index = names();
    assert!(
        found(&index, "bx").is_empty(),
        "two letters allow no edit and no subsequence"
    );
    assert_eq!(found(&index, "box"), [1]);
    assert_eq!(
        found(&index, "bxo"),
        [1],
        "a swap of two neighbours at three letters"
    );
}

#[test]
fn criterion_20_cir_lef_finds_arrow_left_circle_by_prefix_words() {
    let index = names();
    assert_eq!(
        found(&index, "cir lef"),
        [3],
        "cir and lef are prefixes of circle and left"
    );
}

#[test]
fn criterion_20_wether_finds_weather() {
    assert_eq!(found(&names(), "wether"), [2]);
}

#[test]
fn criterion_20_kreis_does_not_find_circle() {
    assert!(
        found(&names(), "kreis").is_empty(),
        "no translation (Out of scope)"
    );
}

#[test]
fn criterion_20_words_under_three_letters_allow_no_typo() {
    let index = index(&[&[("ox", WEIGHT_NAME)]]);
    assert_eq!(found(&index, "ax"), Vec::<u32>::new());
    assert_eq!(found(&index, "ox"), [0]);
}

#[test]
fn criterion_20_one_edit_at_four_to_seven_letters_and_not_two() {
    // "wxyther" is two substitutions away from "weather" (seven letters, one edit allowed).
    let index = names();
    assert_eq!(found(&index, "wxyther"), Vec::<u32>::new());
    // "hxrt" is not an ordered subsequence of "heart" and two edits away.
    assert_eq!(found(&index, "hxrt"), Vec::<u32>::new());
}

#[test]
fn criterion_20_two_edits_at_eight_letters_and_more() {
    // "prallelogrm" is two deletions from "parallelogram" (twelve letters, two edits allowed).
    let index = index(&[&[("parallelogram", WEIGHT_NAME)]]);
    assert_eq!(found(&index, "prallelogrm"), [0]);
}

// Criterion 19: fields, words, case and diacritics.

#[test]
fn criterion_19_words_split_at_dash_underscore_dot_and_space() {
    let index = index(&[
        &[("arrow-left-circle", WEIGHT_NAME)],
        &[("arrow_left.circle", WEIGHT_NAME)],
    ]);
    assert_eq!(found(&index, "left"), [0, 1]);
    assert_eq!(
        found(&index, "arrow left"),
        [0, 1],
        "all words, each in any field"
    );
    assert_eq!(found(&index, "circle"), [0, 1]);
}

#[test]
fn criterion_19_all_words_must_match_and_fields_combine_by_and() {
    let index = index(&[
        &[("heart", WEIGHT_NAME), ("Shapes", WEIGHT_CATEGORY)],
        &[("heart", WEIGHT_NAME), ("Plants", WEIGHT_CATEGORY)],
        &[("arrow", WEIGHT_NAME), ("Shapes", WEIGHT_CATEGORY)],
    ]);
    assert_eq!(
        found(&index, "heart shapes"),
        [0],
        "a word may match the category field"
    );
    assert!(
        found(&index, "heart arrow").is_empty(),
        "every word must match"
    );
}

#[test]
fn criterion_19_matching_ignores_case() {
    assert_eq!(found(&names(), "HEART"), [0]);
    assert_eq!(found(&names(), "Heart"), [0]);
}

#[test]
fn criterion_19_aliases_and_source_fields_are_searched() {
    let index = index(&[
        &[("home", WEIGHT_NAME), ("house", WEIGHT_ALIAS)],
        &[("sun", WEIGHT_NAME), ("Material Symbols", WEIGHT_SOURCE)],
    ]);
    assert_eq!(found(&index, "house"), [0], "an alias is a field");
    assert_eq!(
        found(&index, "symbols"),
        [1],
        "the source name is a field, the lightest one"
    );
}

// Criterion 19 and 22: folding (fold table, ADR 0086 decision 3).

#[test]
fn criterion_19_a_matches_umlaut_a_but_ae_does_not() {
    let index = index(&[&[("ä", WEIGHT_NAME)]]);
    assert_eq!(found(&index, "a"), [0], "\"a\" matches \"ä\"");
    assert!(
        found(&index, "ae").is_empty(),
        "\"ae\" does not equal \"ä\""
    );
}

#[test]
fn criterion_19_sharp_s_folds_to_ss() {
    let index = index(&[&[("straße", WEIGHT_NAME)]]);
    assert_eq!(found(&index, "strasse"), [0]);
}

// Criterion 18 and 17 (score order): ranking.

#[test]
fn criterion_18_an_exact_name_ranks_above_a_typo() {
    let index = index(&[&[("hart", WEIGHT_NAME)], &[("heart", WEIGHT_NAME)]]);
    assert_eq!(found(&index, "heart"), [1, 0]);
}

#[test]
fn criterion_19_a_name_match_outweighs_a_source_match() {
    let index = index(&[&[("box", WEIGHT_SOURCE)], &[("box", WEIGHT_NAME)]]);
    assert_eq!(found(&index, "box"), [1, 0]);
}

#[test]
fn criterion_17_equal_scores_keep_item_order() {
    let index = index(&[
        &[("box", WEIGHT_NAME)],
        &[("box", WEIGHT_NAME)],
        &[("box", WEIGHT_NAME)],
    ]);
    assert_eq!(found(&index, "box"), [0, 1, 2]);
}

// Criterion 14 and 13 (parser fields): the Iconify collection list, from the recorded answers.

#[test]
fn criterion_13_14_25_the_set_list_carries_category_and_palette() {
    let sets = parse_collections(&fixture("iconify_collections.json")).unwrap();
    let mdi = sets
        .iter()
        .find(|set| set.prefix == "mdi")
        .expect("mdi is listed");
    assert_eq!(mdi.category.as_deref(), Some("Material"));
    assert_eq!(mdi.palette, Some(false), "a monochrome set");
    let bare = sets
        .iter()
        .find(|set| set.prefix == "no-licence")
        .expect("listed");
    assert_eq!(
        bare.category, None,
        "a set without category is Uncategorised"
    );
    assert_eq!(
        bare.palette, None,
        "a set without palette flag has no colour data"
    );
    assert!(
        sets.iter().all(|set| set.prefix != "Bad Prefix"),
        "a set with a bad prefix is skipped"
    );
}

#[test]
fn criterion_13_14_25_live_answer_palette_true_is_colour() {
    let sets = parse_collections(&fixture("iconify_live_collections.json")).unwrap();
    let noto = sets
        .iter()
        .find(|set| set.prefix == "noto")
        .expect("noto is listed");
    assert_eq!(
        noto.palette,
        Some(true),
        "its icons carry their own colours"
    );
    assert_eq!(noto.category.as_deref(), Some("Emoji"));
    let whh = sets
        .iter()
        .find(|set| set.prefix == "whh")
        .expect("whh is listed");
    assert_eq!(whh.category, None, "no category in the live answer");
    assert_eq!(whh.palette, Some(false));
    let tabler = sets
        .iter()
        .find(|set| set.prefix == "tabler")
        .expect("tabler is listed");
    assert_eq!(tabler.category.as_deref(), Some("UI 24px"));
}

// Criterion 13 and 25: a set's own groups and aliases, from `/collection`.

#[test]
fn criterion_13_the_listing_keeps_groups_per_name_and_aliases() {
    let listing = parse_set_listing(&fixture("iconify_collection.json")).unwrap();
    assert!(listing.groups.iter().any(|group| group == "Arrows"));
    assert!(listing.names.iter().any(|name| name == "arrow-left"));
    let home = listing
        .names
        .iter()
        .filter(|name| name.as_str() == "home")
        .count();
    assert_eq!(
        home, 1,
        "a name is listed once, even when it sits in two groups"
    );
    let arrows = listing
        .groups
        .iter()
        .position(|group| group == "Arrows")
        .unwrap();
    let home_index = listing
        .names
        .iter()
        .position(|name| name == "home")
        .unwrap();
    assert!(
        listing.name_groups[home_index].contains(&(u16::try_from(arrows).unwrap())),
        "home sits in Arrows"
    );
    assert!(
        listing
            .aliases
            .contains(&("house".to_owned(), "home".to_owned())),
        "the alias house points to home"
    );
}

#[test]
fn criterion_13_hidden_names_and_their_aliases_are_dropped() {
    let listing = parse_set_listing(&fixture("iconify_live_collection.json")).unwrap();
    assert!(
        listing
            .names
            .iter()
            .all(|name| name != "astrophotography-off"),
        "a hidden name is not listed"
    );
    assert!(
        listing
            .aliases
            .iter()
            .all(|(_, target)| target != "astrophotography-off"),
        "an alias of a hidden name is dropped"
    );
    assert!(listing.groups.iter().any(|group| group == "Home"));
}

// Criterion 6: an icon's tile colour (ADR 0086 decision 1).

fn body(text: &str) -> IconBody {
    IconBody {
        name: "arrow-left".to_owned(),
        body: text.to_owned(),
        left: 0.0,
        top: 0.0,
        width: 24.0,
        height: 24.0,
        rotate: 0,
        h_flip: false,
        v_flip: false,
    }
}

#[test]
fn criterion_6_a_monochrome_body_has_no_fixed_colour() {
    assert!(!has_fixed_colour(
        r#"<path fill="currentColor" d="M0 0h24v24H0z"/>"#
    ));
    assert!(!has_fixed_colour(
        r#"<path fill="none" stroke="currentColor"/>"#
    ));
}

#[test]
fn criterion_14_a_body_with_its_own_colours_has_a_fixed_colour() {
    assert!(has_fixed_colour(
        r##"<path fill="#e53935" d="M0 0h1v1z"/>"##
    ));
    assert!(has_fixed_colour(r#"<path style="fill: red" d="M0 0"/>"#));
    assert!(
        has_fixed_colour(r#"<image href="x.png"/>"#),
        "an image is a fixed colour"
    );
}

#[test]
fn criterion_6_the_tile_colours_the_text_colour_through_the_root() {
    let svg = icon_tile_svg(
        &body(r#"<path fill="currentColor" d="M0 0h24"/>"#),
        Rgb8(16, 32, 48),
    );
    assert!(
        svg.contains("color=\"#102030\""),
        "the root carries the colour: {svg}"
    );
    assert!(!svg.contains("<script"), "a body never runs script");
}
