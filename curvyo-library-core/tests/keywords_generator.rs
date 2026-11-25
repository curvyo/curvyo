//! The keyword generator on the checked-in samples (spec `0153` criteria 6 to 8): it runs
//! `tools/library-keywords/generate.py` with `python3 -I`, twice for identical bytes, compares the
//! inflated payload, the related words and the licence text with the goldens, stops on damaged
//! files without writing anything, and the golden file passes the searches of criteria 1 to 3
//! (criterion 7d). The gate never fetches: only the samples are read.

#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use curvyo_library_core::{
    Keywords, Scratch, SearchIndex, WEIGHT_CATEGORY, WEIGHT_KEYWORD, WEIGHT_NAME, sha256_hex,
};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .into()
}

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/keywords")
}

fn scratch_dir(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("keywords")
        .join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn generate(upstream: &Path, pins: &Path, out: &Path) -> Output {
    Command::new("python3")
        .arg("-I")
        .arg(root().join("tools/library-keywords/generate.py"))
        .arg(upstream)
        .arg("--pins")
        .arg(pins)
        .arg("--out")
        .arg(out.join("public"))
        .arg("--related")
        .arg(out.join("related-words.txt"))
        .output()
        .expect("python3 is needed for the keyword generator test")
}

fn inflate(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    flate2::read::GzDecoder::new(bytes)
        .read_to_end(&mut out)
        .unwrap();
    out
}

#[test]
fn the_samples_give_the_golden_file_twice_byte_for_byte() {
    let one = scratch_dir("one");
    let two = scratch_dir("two");
    let upstream = fixtures().join("upstream");
    let pins = fixtures().join("pins.toml");
    for out in [&one, &two] {
        let run = generate(&upstream, &pins, out);
        assert!(
            run.status.success(),
            "{}",
            String::from_utf8_lossy(&run.stderr)
        );
    }
    let read = |dir: &Path, file: &str| std::fs::read(dir.join(file)).unwrap();
    for file in [
        "public/keywords.bin.gz",
        "public/keywords-LICENSES.txt",
        "related-words.txt",
    ] {
        assert_eq!(read(&one, file), read(&two, file), "{file} is reproducible");
    }
    let expected = fixtures().join("expected");
    assert_eq!(
        inflate(&read(&one, "public/keywords.bin.gz")),
        read(&expected, "keywords.bin"),
        "the inflated payload (criterion 7a)"
    );
    assert_eq!(
        String::from_utf8(read(&one, "related-words.txt")).unwrap(),
        String::from_utf8(read(&expected, "related-words.txt")).unwrap(),
        "the related words (criteria 7b and 8)"
    );
    assert_eq!(
        read(&one, "public/keywords-LICENSES.txt"),
        read(&expected, "keywords-LICENSES.txt")
    );
}

/// The sample upstream folder with `file` replaced by `damaged` and its pin updated.
fn damaged_upstream(name: &str, file: &str, damaged: &str) -> (PathBuf, PathBuf) {
    let dir = scratch_dir(name);
    let upstream = dir.join("upstream");
    for package in ["lucide-static", "tabler-icons", "mdi-svg"] {
        let from = fixtures().join("upstream").join(package);
        std::fs::create_dir_all(upstream.join(package)).unwrap();
        for entry in std::fs::read_dir(from).unwrap() {
            let entry = entry.unwrap();
            std::fs::copy(entry.path(), upstream.join(package).join(entry.file_name())).unwrap();
        }
    }
    let before = std::fs::read(upstream.join(file)).unwrap();
    let after = std::fs::read(fixtures().join("damaged").join(damaged)).unwrap();
    std::fs::write(upstream.join(file), &after).unwrap();
    let pins = std::fs::read_to_string(fixtures().join("pins.toml"))
        .unwrap()
        .replace(&sha256_hex(&before), &sha256_hex(&after));
    std::fs::write(dir.join("pins.toml"), pins).unwrap();
    (dir, upstream)
}

#[test]
fn a_damaged_file_stops_the_generator_with_file_and_key_and_writes_nothing() {
    let cases = [
        (
            "not-json",
            "lucide-static/tags.json",
            "not-json.json",
            "lucide-static/tags.json: not JSON",
        ),
        (
            "bad-tag",
            "tabler-icons/icons.json",
            "bad-tag.json",
            "tabler-icons/icons.json: search: a tag is neither",
        ),
        (
            "no-name",
            "mdi-svg/meta.json",
            "no-name.json",
            "mdi-svg/meta.json: entry 1: no name",
        ),
    ];
    for (name, file, damaged, message) in cases {
        let (dir, upstream) = damaged_upstream(name, file, damaged);
        let run = generate(&upstream, &dir.join("pins.toml"), &dir);
        let stderr = String::from_utf8_lossy(&run.stderr);
        assert!(!run.status.success(), "{name}");
        assert!(stderr.contains(message), "{name}: {stderr}");
        assert!(!dir.join("public").exists(), "{name}: nothing written");
        assert!(!dir.join("related-words.txt").exists(), "{name}");
    }
}

#[test]
fn a_file_with_another_checksum_is_refused() {
    let (dir, upstream) = damaged_upstream("checksum", "mdi-svg/meta.json", "no-name.json");
    // The original pins: the replaced file no longer has its checksum.
    let run = generate(&upstream, &fixtures().join("pins.toml"), &dir);
    let stderr = String::from_utf8_lossy(&run.stderr);
    assert!(!run.status.success());
    assert!(stderr.contains("mdi-svg/meta.json: SHA-256"), "{stderr}");
    assert!(!dir.join("public").exists());
}

/// The sample icons of `prefix` as the browser indexes them, plus the set `nokw` without
/// keywords; returns the index and the item names.
fn browser_like(keywords: &Keywords) -> (SearchIndex, Vec<String>) {
    let sets: [(&str, &[&str]); 4] = [
        (
            "lucide",
            &[
                "search",
                "search-check",
                "trash",
                "delete",
                "settings",
                "heart",
                "house",
            ],
        ),
        (
            "tabler",
            &[
                "search",
                "zoom",
                "zoom-in",
                "trash",
                "trash-filled",
                "settings",
                "heart",
                "home",
            ],
        ),
        (
            "mdi",
            &[
                "magnify",
                "delete",
                "delete-outline",
                "trash-can",
                "cog",
                "heart",
                "home",
            ],
        ),
        ("nokw", &["search", "trash", "settings", "pencil"]),
    ];
    let mut index = SearchIndex::new();
    let mut names = Vec::new();
    for (prefix, icons) in sets {
        for name in icons {
            let words = keywords.of(prefix, name);
            let mut fields: Vec<(&str, u8)> = vec![(name, WEIGHT_NAME)];
            if let Some(words) = &words {
                fields.extend(words.keywords().map(|w| (w, WEIGHT_KEYWORD)));
                fields.extend(words.categories().map(|w| (w, WEIGHT_CATEGORY)));
            }
            index.push(&fields);
            names.push(format!("{prefix}:{name}"));
        }
    }
    index.set_related(keywords);
    (index, names)
}

fn found(index: &SearchIndex, names: &[String], query: &str) -> Vec<String> {
    let mut out = Vec::new();
    index.rank(query, &mut Scratch::default(), &mut out);
    out.into_iter()
        .map(|(_, i)| names[i as usize].clone())
        .collect()
}

#[test]
fn the_golden_file_passes_the_searches_of_criteria_1_to_3() {
    let keywords =
        Keywords::decode(&std::fs::read(fixtures().join("expected/keywords.bin")).unwrap())
            .unwrap();
    let (index, names) = browser_like(&keywords);
    let has = |query: &str, name: &str| found(&index, &names, query).contains(&name.to_owned());
    // Criterion 1: published keywords, never applied across sets.
    assert!(has("magnifier", "lucide:search"), "a Lucide tag");
    assert!(has("garbage", "tabler:trash"), "a Tabler tag");
    assert!(has("garbage", "tabler:trash-filled"), "the filled variant");
    assert!(has("rubbish", "mdi:delete"), "an MDI alias");
    assert!(
        keywords
            .of("mdi", "heart")
            .unwrap()
            .keywords()
            .all(|w| w != "love"),
        "mdi:heart has no Lucide tags applied"
    );
    // Criterion 2: related words, also in a set without keywords.
    for name in [
        "lucide:search",
        "tabler:search",
        "mdi:magnify",
        "nokw:search",
    ] {
        assert!(has("magnifier", name), "magnifier finds {name}");
    }
    assert!(has("trash", "nokw:trash"));
    assert!(has("trash", "lucide:delete"), "trash finds delete");
    assert!(has("bin", "nokw:trash"), "bin finds trash");
    assert!(!has("magnifier", "nokw:pencil"));
    // Criterion 3: name above keyword above related word.
    let order = found(&index, &names, "search");
    let at = |name: &str| order.iter().position(|n| n == name).unwrap();
    assert!(
        at("nokw:search") < at("mdi:magnify"),
        "name above keyword: {order:?}"
    );
    assert!(
        at("mdi:magnify") < at("tabler:zoom-in"),
        "keyword above related: {order:?}"
    );
    let order = found(&index, &names, "lookup");
    let at = |name: &str| order.iter().position(|n| n == name).unwrap();
    assert!(at("lucide:search") < at("nokw:search"), "{order:?}");
    // AND: every typed word must still match.
    assert_eq!(
        found(&index, &names, "magnifier pencil"),
        Vec::<String>::new()
    );
    assert!(!has("magnifier heart", "lucide:search"));
}
