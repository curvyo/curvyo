use super::*;
use crate::keywords::Keywords;

fn index(names: &[&str]) -> SearchIndex {
    let mut index = SearchIndex::new();
    for name in names {
        index.push(&[(name, WEIGHT_NAME)]);
    }
    index
}

fn found(index: &SearchIndex, query: &str) -> Vec<u32> {
    let mut out = Vec::new();
    index.rank(query, &mut Scratch::default(), &mut out);
    out.into_iter().map(|(_, i)| i).collect()
}

fn finds(name: &str, query: &str) -> bool {
    !found(&index(&[name]), query).is_empty()
}

#[test]
fn the_examples_of_criterion_20_hold() {
    assert!(finds("heart", "hart"), "a deletion");
    assert!(finds("heart", "haert"), "a swap");
    assert!(finds("heart", "hrt"), "an ordered subsequence");
    assert!(!finds("box", "bx"), "no edit under 3 letters");
    assert!(finds("box", "box"));
    assert!(finds("box", "bxo"), "one swap at 3 letters");
    assert!(!finds("box", "bix"), "no substitution at 3 letters");
    assert!(finds("weather", "wether"), "one insertion at 6 letters");
    assert!(
        finds("arrow-left-circle", "cir lef"),
        "prefixes of two words"
    );
    assert!(!finds("circle", "kreis"), "no translation");
}

#[test]
fn the_typo_budget_grows_with_the_word() {
    assert!(
        finds("weather", "wethe"),
        "a typo while typing matches a prefix"
    );
    assert!(finds("rectangle", "rectnagel"), "two edits at 8 letters");
    assert!(!finds("rectangle", "rxctnagel"), "three edits are too many");
    assert!(
        !finds("house", "hxuxe"),
        "two edits at 5 letters are too many"
    );
}

#[test]
fn case_and_diacritics_are_folded_and_names_split() {
    assert!(finds("Äpfel", "apfel"));
    assert!(finds("Apfel", "äpfel"), "a matches ä both ways");
    let mut out = Vec::new();
    let one = index(&["Äpfel"]);
    one.rank("aepfel", &mut Scratch::default(), &mut out);
    assert_eq!(
        out,
        [(32, 0)],
        "ae is not ä: only a typo, not an exact match"
    );
    assert!(finds("arrow_left.circle", "left"));
    assert!(finds("ARROW", "arrow"));
}

#[test]
fn every_word_must_match_and_strength_orders_the_result() {
    let index = index(&["home", "homepage", "my-home", "hme-x", "garden"]);
    assert_eq!(
        found(&index, "home"),
        [0, 2, 1, 3],
        "exact, exact, prefix, typo"
    );
    assert_eq!(found(&index, "home garden"), Vec::<u32>::new());
    assert_eq!(found(&index, ""), [0, 1, 2, 3, 4], "no words: everything");
}

#[test]
fn heavier_fields_score_higher_and_equal_scores_keep_item_order() {
    let mut index = SearchIndex::new();
    index.push(&[("circle", WEIGHT_NAME)]);
    index.push(&[("ring", WEIGHT_NAME), ("circle", WEIGHT_ALIAS)]);
    index.push(&[("disc", WEIGHT_NAME), ("Circle shapes", WEIGHT_SOURCE)]);
    index.push(&[("circle", WEIGHT_NAME)]);
    let mut out = Vec::new();
    index.rank("circle", &mut Scratch::default(), &mut out);
    assert_eq!(out, [(256, 0), (256, 3), (128, 1), (32, 2)]);
}

#[test]
fn the_query_is_cut_at_100_characters() {
    let index = index(&["a"]);
    let long = format!("{} zzzz", "a ".repeat(60));
    assert_eq!(
        found(&index, &long),
        [0],
        "the words past 100 characters are not read"
    );
}

#[test]
fn the_prefix_distance_counts_edits_to_the_closest_prefix() {
    let chars = |s: &str| s.chars().collect::<Vec<_>>();
    assert_eq!(prefix_distance(&chars("haert"), &chars("heart")), 1);
    assert_eq!(prefix_distance(&chars("wethe"), &chars("weather")), 1);
    assert_eq!(prefix_distance(&chars("abc"), &chars("xyz")), 3);
}

#[test]
fn one_swap_at_three_letters_is_found_anywhere_in_the_word() {
    assert!(finds("box", "obx"));
    assert!(finds("box", "bxo"));
    assert!(!finds("box", "oxb"), "two swaps");
    assert!(!finds("box", "bxx"));
}

/// Items: 0 `search` (name), 1 `glass` with keyword "search", 2 `lens` with keyword "magnifier",
/// 3 `trash` (name), 4 `delete` with keyword "bin", 5 `folder` with category "search".
fn keyword_index() -> SearchIndex {
    let mut index = SearchIndex::new();
    index.push(&[("search", WEIGHT_NAME)]);
    index.push(&[("glass", WEIGHT_NAME), ("search", WEIGHT_KEYWORD)]);
    index.push(&[("lens", WEIGHT_NAME), ("magnifier", WEIGHT_KEYWORD)]);
    index.push(&[("trash", WEIGHT_NAME)]);
    index.push(&[("delete", WEIGHT_NAME), ("bin", WEIGHT_KEYWORD)]);
    index.push(&[("folder", WEIGHT_NAME), ("search", WEIGHT_CATEGORY)]);
    index.set_related(&Keywords::with_related(&[
        ("magnifier", &["search", "zoom"]),
        ("search", &["magnifier", "find"]),
        ("bin", &["trash", "delete"]),
        ("trash", &["bin", "delete"]),
        ("delete", &["trash"]),
    ]));
    index
}

#[test]
fn name_ranks_above_keyword_above_related_word() {
    let mut out = Vec::new();
    keyword_index().rank("search", &mut Scratch::default(), &mut out);
    // name 16 x 16, keyword 6 x 16, related "magnifier" 3 x 16, category 4 x 16.
    assert_eq!(out, [(256, 0), (96, 1), (64, 5), (48, 2)]);
}

#[test]
fn a_related_word_matches_one_hop_and_counts_for_its_typed_word() {
    let index = keyword_index();
    assert_eq!(
        found(&index, "magnifier"),
        [2, 0, 1],
        "lens by keyword, search by related"
    );
    assert_eq!(found(&index, "bin"), [4, 3], "bin finds trash (related)");
    assert_eq!(
        found(&index, "trash"),
        [3, 4],
        "trash finds delete (related bin, delete)"
    );
    assert_eq!(
        found(&index, "magnifier trash"),
        Vec::<u32>::new(),
        "AND holds"
    );
    assert_eq!(
        found(&index, "zoom"),
        Vec::<u32>::new(),
        "zoom is no head: no hop back"
    );
    assert!(
        !found(&index, "magnifier").contains(&5),
        "a related word never counts on a category word"
    );
}

#[test]
fn heads_match_by_exact_prefix_or_substring_only() {
    let index = keyword_index();
    assert_eq!(found(&index, "magnif"), [2, 0, 1], "a prefix of the head");
    assert_eq!(
        found(&index, "agnifie"),
        [2, 0, 1],
        "a substring of the head"
    );
    assert_eq!(
        found(&index, "magnifeir"),
        [2],
        "a typo reaches the keyword, not the head"
    );
    assert_eq!(
        found(&index, "bi"),
        [4],
        "under 3 letters only the keyword, no related word"
    );
}

#[test]
fn at_most_8_related_words_per_typed_word_exact_head_first() {
    let mut index = SearchIndex::new();
    let names: Vec<String> = (0..12).map(|n| format!("w{n:02}x")).collect();
    for name in &names {
        index.push(&[(name, WEIGHT_NAME)]);
    }
    let first: Vec<&str> = names[..4].iter().map(String::as_str).collect();
    let second: Vec<&str> = names[4..].iter().map(String::as_str).collect();
    index.set_related(&Keywords::with_related(&[
        ("abcd", &second),
        ("abc", &first),
    ]));
    let hits = found(&index, "abc");
    assert_eq!(hits.len(), 8);
    assert!(
        (0..4).all(|i| hits.contains(&i)),
        "the exact head's words come first: {hits:?}"
    );
}

#[test]
fn explain_names_the_data_word_and_the_head() {
    let index = keyword_index();
    let mut scratch = Scratch::default();
    assert_eq!(index.explain("search", 0, &mut scratch), [Why::Name]);
    assert_eq!(
        index.explain("search", 1, &mut scratch),
        [Why::Keyword("search".into())]
    );
    assert_eq!(
        index.explain("magnifer", 2, &mut scratch),
        [Why::Keyword("magnifier".into())],
        "the data's word, not the typed text"
    );
    assert_eq!(
        index.explain("bin", 3, &mut scratch),
        [Why::Related {
            head: "bin".into(),
            word: "trash".into()
        }]
    );
    assert_eq!(
        index.explain("search", 5, &mut scratch),
        [Why::Category("search".into())]
    );
    assert_eq!(index.explain("zebra", 0, &mut scratch), Vec::<Why>::new());
    assert_eq!(index.explain("search", 99, &mut scratch), Vec::<Why>::new());
}

#[test]
fn prefix_heads_come_before_substring_heads() {
    let mut index = SearchIndex::new();
    let names: Vec<String> = (0..10).map(|n| format!("w{n:02}x")).collect();
    for name in &names {
        index.push(&[(name, WEIGHT_NAME)]);
    }
    let block: Vec<&str> = names[..8].iter().map(String::as_str).collect();
    let lock: Vec<&str> = names[8..].iter().map(String::as_str).collect();
    // "block" comes first in table order, but "loc" is a prefix of "lock".
    index.set_related(&Keywords::with_related(&[
        ("block", &block),
        ("lock", &lock),
    ]));
    let hits = found(&index, "loc");
    assert_eq!(hits.len(), 8);
    assert!(
        hits.contains(&8) && hits.contains(&9),
        "lock's words first: {hits:?}"
    );
    let why = index.explain("loc", 8, &mut Scratch::default());
    assert_eq!(
        why,
        [Why::Related {
            head: "lock".into(),
            word: "w08x".into()
        }]
    );
}
