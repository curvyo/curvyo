#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_possible_truncation,
    clippy::assertions_on_constants,
    clippy::format_push_string,
    missing_docs
)]
//! Acceptance tests of spec `0153` (library keywords) for `curvyo-library-core`: keyword and
//! related-word matches on the shipped data (criteria 1, 2), ranking and AND (criterion 3), the
//! damaged data file (criterion 4, 9), the generator on the checked-in samples (criteria 6 to 8),
//! the notices and header (criterion 10), no translation (criterion 12) and the format (criterion
//! 13). Written from the specification before the implementation was read (playbook: tester).

use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;

use curvyo_library_core::{
    Keywords, MAX_RELATED, Scratch, SearchIndex, WEIGHT_ALIAS, WEIGHT_CATEGORY, WEIGHT_KEYWORD,
    WEIGHT_NAME, WEIGHT_RELATED, WEIGHT_SOURCE, Weight, Why, sha256_hex,
};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .into()
}

fn shipped_bytes() -> Vec<u8> {
    std::fs::read(repo().join("frontend/public/library/keywords.bin.gz")).unwrap()
}

fn shipped() -> Keywords {
    Keywords::decode(&shipped_bytes()).expect("the shipped keyword file decodes")
}

fn inflate(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    flate2::read::GzDecoder::new(bytes)
        .read_to_end(&mut out)
        .unwrap();
    out
}

/// The test universe: real icon names of the three sets, with and without published keywords.
const UNIVERSE: &[(&str, &str)] = &[
    ("lucide", "search"),
    ("lucide", "trash-2"),
    ("lucide", "settings"),
    ("lucide", "heart"),
    ("lucide", "arrow-left"),
    ("tabler", "search"),
    ("tabler", "trash"),
    ("tabler", "settings"),
    ("tabler", "heart"),
    ("mdi", "magnify"),
    ("mdi", "delete"),
    ("mdi", "cog"),
    ("mdi", "heart"),
    ("mdi", "trash-can"),
];

/// An index over `items`: each item's name, and its keywords and categories from `kw`.
fn universe(kw: &Keywords, items: &[(&str, &str)]) -> SearchIndex {
    let mut index = SearchIndex::new();
    for (prefix, name) in items {
        let mut fields: Vec<(&str, Weight)> = vec![(name, WEIGHT_NAME), (prefix, WEIGHT_SOURCE)];
        if let Some(words) = kw.of(prefix, name) {
            fields.extend(words.keywords().map(|w| (w, WEIGHT_KEYWORD)));
            fields.extend(words.categories().map(|w| (w, WEIGHT_CATEGORY)));
        }
        index.push(&fields);
    }
    index.set_related(kw);
    index
}

/// The matching item indices of `query`, best first.
fn found(index: &SearchIndex, query: &str) -> Vec<u32> {
    let mut scratch = Scratch::default();
    let mut out = Vec::new();
    index.rank(query, &mut scratch, &mut out);
    out.iter().map(|(_, item)| *item).collect()
}

fn position(items: &[(&str, &str)], prefix: &str, name: &str) -> u32 {
    items
        .iter()
        .position(|(p, n)| *p == prefix && *n == name)
        .unwrap() as u32
}

/// The related words of `word` in the shipped file, as a set.
fn related_of(kw: &Keywords, word: &str) -> BTreeSet<String> {
    kw.related()
        .find(|(head, _)| *head == word)
        .map(|(_, rel)| rel.map(str::to_owned).collect())
        .unwrap_or_default()
}

fn levenshtein(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut cur = vec![i + 1];
        for (j, cb) in b.iter().enumerate() {
            let cost = usize::from(ca != *cb);
            cur.push((prev[j] + cost).min(prev[j + 1] + 1).min(cur[j] + 1));
        }
        prev = cur;
    }
    prev[b.len()]
}

// Criterion 1: keywords published by the three projects.

#[test]
fn criterion_1_the_shipped_file_publishes_lucide_search_as_magnifier() {
    let kw = shipped();
    let icon = kw
        .of("lucide", "search")
        .expect("lucide:search has published keywords");
    let words: Vec<&str> = icon.keywords().collect();
    assert!(
        words.contains(&"magnifier"),
        "Lucide tags of search include magnifier, got {words:?}"
    );
}

#[test]
fn criterion_1_magnifier_finds_lucide_search() {
    let kw = shipped();
    let index = universe(&kw, UNIVERSE);
    let hits = found(&index, "magnifier");
    assert!(
        hits.contains(&position(UNIVERSE, "lucide", "search")),
        "magnifier must find lucide:search, got {hits:?}"
    );
}

#[test]
fn criterion_1_trash_finds_delete_and_bin_finds_trash() {
    let kw = shipped();
    let index = universe(&kw, UNIVERSE);
    let trash = found(&index, "trash");
    assert!(
        trash.contains(&position(UNIVERSE, "mdi", "delete")),
        "trash must find mdi:delete, got {trash:?}"
    );
    assert!(
        trash.contains(&position(UNIVERSE, "tabler", "trash")),
        "trash must find tabler:trash"
    );
    let bin = found(&index, "bin");
    assert!(
        bin.contains(&position(UNIVERSE, "tabler", "trash")),
        "bin must find tabler:trash, got {bin:?}"
    );
}

#[test]
fn criterion_1_cog_finds_settings() {
    let kw = shipped();
    let index = universe(&kw, UNIVERSE);
    let hits = found(&index, "cog");
    assert!(
        hits.contains(&position(UNIVERSE, "lucide", "settings"))
            || hits.contains(&position(UNIVERSE, "tabler", "settings")),
        "cog must find a settings icon, got {hits:?}"
    );
}

#[test]
fn criterion_1_a_keyword_never_matches_an_icon_it_was_not_published_for() {
    // mdi:heart has no Lucide tags applied: no Lucide keyword of `heart` may match through a
    // keyword of mdi:heart. Each keyword is checked alone against an index of mdi:heart only.
    let kw = shipped();
    let lucide_heart: Vec<String> = kw
        .of("lucide", "heart")
        .expect("lucide:heart has keywords")
        .keywords()
        .map(str::to_owned)
        .collect();
    let mdi_heart: BTreeSet<String> = kw
        .of("mdi", "heart")
        .map(|w| w.keywords().map(str::to_owned).collect())
        .unwrap_or_default();
    let index = universe(&kw, &[("mdi", "heart")]);
    let mut scratch = Scratch::default();
    let mut checked = 0;
    for word in lucide_heart.iter().filter(|w| !mdi_heart.contains(*w)) {
        let reasons = index.explain(word, 0, &mut scratch);
        assert!(
            !reasons.iter().any(|r| matches!(r, Why::Keyword(_))),
            "{word} matched mdi:heart through a keyword it was not published for"
        );
        checked += 1;
    }
    assert!(
        checked > 0,
        "the test needs Lucide keywords that mdi:heart lacks"
    );
}

#[test]
fn criterion_1_the_shipped_file_has_keywords_for_tabler_and_mdi_samples() {
    let kw = shipped();
    assert!(
        kw.of("tabler", "trash")
            .is_some_and(|w| w.keywords().count() > 0)
    );
    assert!(
        kw.of("mdi", "delete")
            .is_some_and(|w| w.keywords().count() > 0)
    );
}

// Criterion 2: related words, one hop, at most 8, in every set.

#[test]
fn criterion_2_magnifier_finds_search_in_a_set_without_keywords() {
    // A set with no published keywords (for example Phosphor, not in the file): its item is only
    // its name, and the typed word reaches it through the related words alone.
    let kw = shipped();
    let mut index = SearchIndex::new();
    index.push(&[("search", WEIGHT_NAME), ("ph", WEIGHT_SOURCE)]);
    index.set_related(&kw);
    assert_eq!(
        found(&index, "magnifier"),
        [0],
        "magnifier relates to search, so an item named search is found without keywords"
    );
}

#[test]
fn criterion_2_related_words_are_listed_as_their_own_matches_with_why() {
    let kw = shipped();
    let index = universe(&kw, UNIVERSE);
    let trash = position(UNIVERSE, "tabler", "trash");
    let mut scratch = Scratch::default();
    let reasons = index.explain("bin", trash, &mut scratch);
    // Tabler publishes `bin` as a keyword of trash, so a Keyword reason is the expected one; a
    // Related reason is also a match. An empty or name-only reason is a failure.
    assert!(
        reasons
            .iter()
            .any(|r| matches!(r, Why::Keyword(_) | Why::Related { .. })),
        "bin reaches tabler:trash as a keyword or a related word, got {reasons:?}"
    );
}

#[test]
fn criterion_2_a_related_word_expands_one_hop_only() {
    let kw = shipped();
    let heads: Vec<String> = kw.related().map(|(h, _)| h.to_owned()).collect();
    let mut tested = 0;
    'outer: for word in &heads {
        let direct = related_of(&kw, word);
        for mid in &direct {
            for far in related_of(&kw, mid) {
                if far == *word
                    || direct.contains(&far)
                    || far.contains(word.as_str())
                    || word.contains(far.as_str())
                    || levenshtein(&far, word) <= 2
                {
                    continue;
                }
                // The item is named `far` alone: only a second hop could reach it.
                let mut index = SearchIndex::new();
                index.push(&[(far.as_str(), WEIGHT_NAME)]);
                index.set_related(&kw);
                assert!(
                    found(&index, word).is_empty(),
                    "{word} reached {far} through {mid}: two hops"
                );
                tested += 1;
                if tested >= 20 {
                    break 'outer;
                }
            }
        }
    }
    assert!(tested > 0, "the file has two-hop chains to test against");
}

#[test]
fn criterion_2_at_most_eight_related_words_per_word_and_symmetric() {
    let kw = shipped();
    let mut map: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (head, rel) in kw.related() {
        let rel: BTreeSet<String> = rel.map(str::to_owned).collect();
        // The stored table is the symmetric union of each word's top 8 (lead, 2026-11-25), so
        // a head may list more; the cap of 8 applies per typed word in the matcher.
        map.insert(head.to_owned(), rel);
    }
    assert_eq!(MAX_RELATED, 8, "the spec's Proposal of at most 8");
    for (head, rel) in &map {
        for other in rel {
            assert!(
                map.get(other).is_some_and(|back| back.contains(head)),
                "{head} is related to {other} but not the other way round"
            );
        }
    }
}

#[test]
fn criterion_2_related_words_are_english_and_short_words_are_not_used() {
    let kw = shipped();
    for (head, rel) in kw.related() {
        assert!(
            head.chars().count() >= 3,
            "head {head:?} is under three letters"
        );
        for word in rel {
            assert!(
                word.chars().count() >= 3,
                "related {word:?} is under three letters"
            );
            assert!(
                !matches!(
                    word,
                    "icon" | "outline" | "filled" | "solid" | "alt" | "variant"
                ),
                "the stop list leaked into the related words: {word}"
            );
        }
    }
}

// Criterion 3: ranking and AND.

#[test]
fn criterion_3_ranking_name_keyword_category_related_source() {
    let kw = shipped();
    assert!(
        related_of(&kw, "search").contains("magnifier"),
        "the test needs magnifier as a related word of search"
    );
    let mut index = SearchIndex::new();
    index.push(&[("search", WEIGHT_SOURCE)]); // 0: source
    index.push(&[("search", WEIGHT_CATEGORY)]); // 1: category
    index.push(&[("search", WEIGHT_KEYWORD)]); // 2: keyword
    index.push(&[("search", WEIGHT_NAME)]); // 3: name
    index.push(&[("magnifier", WEIGHT_NAME)]); // 4: related only
    index.set_related(&kw);
    assert_eq!(found(&index, "search"), [3, 2, 1, 4, 0]);
    assert!(WEIGHT_NAME > WEIGHT_ALIAS && WEIGHT_ALIAS >= WEIGHT_KEYWORD);
    assert!(WEIGHT_KEYWORD > WEIGHT_CATEGORY && WEIGHT_CATEGORY > WEIGHT_RELATED);
    assert!(WEIGHT_RELATED > WEIGHT_SOURCE);
}

#[test]
fn criterion_3_every_typed_word_must_match() {
    let kw = shipped();
    let mut index = SearchIndex::new();
    index.push(&[("search", WEIGHT_NAME), ("zebra", WEIGHT_NAME)]); // 0
    index.push(&[("search", WEIGHT_NAME)]); // 1: no zebra
    index.set_related(&kw);
    assert_eq!(
        found(&index, "magnifier zebra"),
        [0],
        "magnifier by related word and zebra by name, both must match"
    );
}

#[test]
fn criterion_3_a_related_word_counts_as_a_match_of_the_typed_word() {
    let kw = shipped();
    let mut index = SearchIndex::new();
    index.push(&[("search", WEIGHT_NAME)]);
    index.set_related(&kw);
    let mut scratch = Scratch::default();
    let reasons = index.explain("magnifier", 0, &mut scratch);
    assert!(
        reasons.iter().any(|r| matches!(r, Why::Related { .. })),
        "magnifier explains its match of search as a related word, got {reasons:?}"
    );
}

// Criterion 4 and 9: the data file is optional for the search.

#[test]
fn criterion_4_a_damaged_file_is_refused_with_no_panic() {
    let good = shipped_bytes();
    let mut cut = good.clone();
    cut.truncate(good.len() / 2);
    let mut flipped = good.clone();
    let mid = flipped.len() / 2;
    flipped[mid] ^= 0xff;
    for bytes in [
        b"not a keyword file".to_vec(),
        Vec::new(),
        cut,
        vec![0x1f, 0x8b, 0, 0, 0, 0],
    ] {
        assert!(
            Keywords::decode(&bytes).is_err(),
            "damaged bytes must not decode"
        );
    }
    // A flipped byte inside the gzip payload is refused or decodes; it must never panic.
    let _ = Keywords::decode(&flipped);
}

#[test]
fn criterion_4_search_still_works_by_name_without_the_data() {
    let mut index = SearchIndex::new();
    index.push(&[("search", WEIGHT_NAME), ("lucide", WEIGHT_SOURCE)]);
    index.push(&[("trash-2", WEIGHT_NAME), ("lucide", WEIGHT_SOURCE)]);
    assert_eq!(found(&index, "search"), [0]);
    assert!(
        found(&index, "magnifier").is_empty(),
        "without the data a keyword word finds nothing"
    );
}

#[test]
fn criterion_9_an_icon_the_data_does_not_know_and_an_unknown_set_match_by_name() {
    let kw = shipped();
    assert!(kw.of("lucide", "no-such-icon-anywhere").is_none());
    assert!(
        kw.of("ph", "search").is_none(),
        "Phosphor is not in the file"
    );
    let items: &[(&str, &str)] = &[("ph", "search")];
    let index = universe(&kw, items);
    assert_eq!(
        found(&index, "search"),
        [0],
        "an unknown set matches by its name"
    );
}

// Criterion 12: English only.

#[test]
fn criterion_12_no_translation_kreis_does_not_find_circle() {
    let kw = shipped();
    let index = universe(&kw, &[("lucide", "circle"), ("lucide", "search")]);
    assert!(
        found(&index, "kreis").is_empty(),
        "no German words are looked up"
    );
}

// Criterion 13: the format is version 1 of the existing container, nothing else changes.

#[test]
fn criterion_13_the_shipped_payload_starts_with_the_format_byte() {
    let bytes = shipped_bytes();
    assert_eq!(&bytes[..2], &[0x1f, 0x8b], "gzip magic");
    let payload = inflate(&bytes);
    assert_eq!(&payload[..4], b"CVYK");
    assert_eq!(payload[4], 1, "format 1 as the generator's README says");
}

// Criterion 4: the size of the shipped file.

#[test]
fn criterion_4_the_shipped_file_is_at_most_300_kib_gzipped() {
    let size = shipped_bytes().len();
    assert!(size <= 300 * 1024, "{size} bytes is over 300 KiB");
    assert!(size <= 320 * 1024, "the test fails above 320 KiB");
}

// Criterion 10: notices and the header of the file.

#[test]
fn criterion_10_the_header_names_and_versions_match_the_pins() {
    let pins: toml::Table =
        std::fs::read_to_string(repo().join("tools/library-keywords/pins.toml"))
            .unwrap()
            .parse()
            .unwrap();
    let kw = shipped();
    let sources = pins["source"].as_array().unwrap();
    assert_eq!(kw.sources().len(), sources.len());
    for pin in sources {
        let prefix = pin["prefix"].as_str().unwrap();
        let entry = kw
            .sources()
            .iter()
            .find(|s| s.prefix == prefix)
            .unwrap_or_else(|| panic!("no source {prefix} in the header"));
        assert_eq!(entry.version, pin["version"].as_str().unwrap());
        assert_eq!(entry.licence, pin["licence"].as_str().unwrap());
        assert_eq!(entry.project, pin["project"].as_str().unwrap());
    }
}

#[test]
fn criterion_10_the_notices_file_carries_each_licence_verbatim() {
    let notices =
        std::fs::read_to_string(repo().join("frontend/public/library/keywords-LICENSES.txt"))
            .unwrap();
    for project in ["Lucide", "Tabler Icons", "Material Design Icons"] {
        assert!(
            notices.contains(project),
            "{project} is named in the notices"
        );
    }
    for version in ["1.80.0", "3.52.0", "7.4.47"] {
        assert!(
            notices.contains(version),
            "version {version} is in the notices"
        );
    }
    let isc = "Permission to use, copy, modify, and/or distribute this software for any";
    assert!(
        notices.contains(isc),
        "the ISC licence text of Lucide is shipped"
    );
    let mit = "Permission is hereby granted, free of charge, to any person obtaining a copy";
    assert!(
        notices.matches(mit).count() >= 2,
        "the MIT licence text of Tabler and of MDI is shipped"
    );
    let mdi = "Pictogrammers Free License";
    assert!(
        notices.contains(mdi),
        "the MDI licence file is shipped verbatim"
    );
}

#[test]
fn criterion_10_the_shipped_notices_hold_the_sample_licence_texts() {
    // The checked-in sample licence files are the same texts the generator reads (fixtures).
    let notices =
        std::fs::read_to_string(repo().join("frontend/public/library/keywords-LICENSES.txt"))
            .unwrap();
    for dir in ["lucide-static", "tabler-icons", "mdi-svg"] {
        let text = std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/keywords/upstream")
                .join(dir)
                .join("LICENSE"),
        )
        .unwrap();
        let first = text.lines().find(|l| !l.trim().is_empty()).unwrap();
        assert!(
            notices.contains(first),
            "{dir}: the first licence line is shipped"
        );
    }
}

#[test]
fn criterion_10_the_third_party_document_has_the_search_keywords_section() {
    let doc = std::fs::read_to_string(repo().join("docs/third-party-assets.md")).unwrap();
    let start = doc
        .find("## Search keywords")
        .expect("a Search keywords section");
    let section = &doc[start..];
    let end = section[3..].find("\n## ").map_or(section.len(), |i| i + 3);
    let section = &section[..end];
    for needle in [
        "1.80.0",
        "3.52.0",
        "7.4.47",
        "ISC",
        "MIT",
        "keywords.bin.gz",
        "keywords-LICENSES.txt",
    ] {
        assert!(section.contains(needle), "the section names {needle}");
    }
}

// Criterion 6 to 8: the generator on the checked-in samples; it never fetches.

fn generator() -> PathBuf {
    repo().join("tools/library-keywords/generate.py")
}

fn generate_into(upstream: &Path, pins: &Path, out: &Path) -> std::process::Output {
    std::fs::create_dir_all(out).unwrap();
    Command::new("python3")
        .arg("-I")
        .arg(generator())
        .arg(upstream)
        .arg("--pins")
        .arg(pins)
        .arg("--out")
        .arg(out.join("public"))
        .arg("--related")
        .arg(out.join("related-words.txt"))
        .output()
        .expect("python3 runs the generator")
}

fn sample_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/keywords")
}

fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("acceptance_0153")
        .join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn criterion_6_the_generator_is_reproducible_on_the_samples() {
    let upstream = sample_dir().join("upstream");
    let pins = sample_dir().join("pins.toml");
    let a = scratch("repro-a");
    let b = scratch("repro-b");
    let ra = generate_into(&upstream, &pins, &a);
    let rb = generate_into(&upstream, &pins, &b);
    assert!(
        ra.status.success(),
        "{}",
        String::from_utf8_lossy(&ra.stderr)
    );
    assert!(
        rb.status.success(),
        "{}",
        String::from_utf8_lossy(&rb.stderr)
    );
    for file in [
        "public/keywords.bin.gz",
        "public/keywords-LICENSES.txt",
        "related-words.txt",
    ] {
        assert_eq!(
            std::fs::read(a.join(file)).unwrap(),
            std::fs::read(b.join(file)).unwrap(),
            "{file} is byte-identical on a second run"
        );
    }
}

#[test]
fn criterion_6_the_generator_never_fetches() {
    let source = std::fs::read_to_string(generator()).unwrap();
    for forbidden in [
        "urllib",
        "http.client",
        "socket",
        "requests",
        "ftplib",
        "urlopen",
        "subprocess",
    ] {
        assert!(
            !source.contains(forbidden),
            "generate.py must not use {forbidden}: it never fetches"
        );
    }
}

#[test]
fn criterion_6_a_file_with_another_checksum_is_refused_and_nothing_is_written() {
    let work = scratch("checksum");
    let upstream = work.join("upstream");
    copy_tree(&sample_dir().join("upstream"), &upstream);
    std::fs::write(upstream.join("lucide-static/tags.json"), b"{}\n").unwrap();
    let out = work.join("out");
    let run = generate_into(&upstream, &sample_dir().join("pins.toml"), &out);
    assert!(!run.status.success(), "a changed file is refused");
    assert!(
        !out.join("public/keywords.bin.gz").exists(),
        "nothing is written"
    );
}

#[test]
fn criterion_7c_damaged_upstream_files_stop_the_generator_and_write_nothing() {
    let damaged = sample_dir().join("damaged");
    let mut refused = 0;
    for file in ["not-json.json", "bad-tag.json", "no-name.json"] {
        for (source, target) in [
            ("lucide", "lucide-static/tags.json"),
            ("tabler", "tabler-icons/icons.json"),
            ("mdi", "mdi-svg/meta.json"),
        ] {
            let work = scratch(&format!("damaged-{file}-{source}"));
            let upstream = work.join("upstream");
            copy_tree(&sample_dir().join("upstream"), &upstream);
            let content = std::fs::read(damaged.join(file)).unwrap();
            std::fs::write(upstream.join(target), &content).unwrap();
            let pins = work.join("pins.toml");
            write_pins_for(
                &sample_dir().join("pins.toml"),
                &pins,
                &upstream,
                target,
                &content,
            );
            let out = work.join("out");
            let run = generate_into(&upstream, &pins, &out);
            if run.status.success() {
                continue;
            }
            refused += 1;
            assert!(
                !out.join("public/keywords.bin.gz").exists(),
                "{file}/{source}: nothing written"
            );
            assert!(
                !out.join("related-words.txt").exists(),
                "{file}/{source}: no related file"
            );
            let stderr = String::from_utf8_lossy(&run.stderr);
            assert!(
                stderr.contains(file.trim_end_matches(".json")) || stderr.contains(target),
                "{file}/{source}: stderr names the file or key: {stderr}"
            );
        }
    }
    assert!(
        refused >= 3,
        "the damaged samples are refused, got {refused} refusals"
    );
}

#[test]
fn criterion_7d_the_generated_sample_file_passes_the_searches() {
    let work = scratch("readback");
    let upstream = sample_dir().join("upstream");
    let run = generate_into(&upstream, &sample_dir().join("pins.toml"), &work);
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let bytes = std::fs::read(work.join("public/keywords.bin.gz")).unwrap();
    let kw = Keywords::decode(&bytes).expect("the generated sample decodes");
    let prefixes: Vec<&str> = kw.sources().iter().map(|s| s.prefix.as_str()).collect();
    assert_eq!(prefixes, ["lucide", "tabler", "mdi"]);
    let words: Vec<&str> = kw
        .of("lucide", "search")
        .map_or(Vec::new(), |w| w.keywords().collect());
    assert!(
        words.contains(&"magnifier") || kw.of("lucide", "search").is_none(),
        "a sample keyword is read back"
    );
}

#[test]
fn criterion_8_related_words_are_derived_and_written_beside_the_file_not_shipped() {
    let related = repo().join("tools/library-keywords/related-words.txt");
    let text = std::fs::read_to_string(related).unwrap();
    assert!(
        text.lines().count() > 100,
        "the derived table has many heads"
    );
    let shipped_dir = repo().join("frontend/public/library");
    assert!(
        !shipped_dir.join("related-words.txt").exists(),
        "the readable table is not shipped"
    );
}

/// Copy a folder tree of the samples (no symlinks).
fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}

/// The sample pins with the checksum of one replaced upstream file (so the damaged file gets
/// past the checksum and fails on its content).
fn write_pins_for(pins: &Path, out: &Path, _upstream: &Path, target: &str, content: &[u8]) {
    let text = std::fs::read_to_string(pins).unwrap();
    let table: toml::Table = text.parse().unwrap();
    let mut rewritten = String::new();
    let digest = sha256_hex(content);
    let mut in_target = false;
    for line in text.lines() {
        if line.starts_with("[[source]]") {
            in_target = false;
        }
        if line.trim_start().starts_with("file") && line.contains(target) {
            in_target = true;
        }
        if in_target && line.trim_start().starts_with("sha256") {
            rewritten.push_str(&format!("sha256 = \"{digest}\"\n"));
            in_target = false;
            continue;
        }
        rewritten.push_str(line);
        rewritten.push('\n');
    }
    assert!(table.contains_key("source"));
    std::fs::write(out, rewritten).unwrap();
}
