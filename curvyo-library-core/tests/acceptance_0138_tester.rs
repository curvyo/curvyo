//! Acceptance tests of `specs/0138-template-library` for the protocols of `curvyo-library-core`,
//! written from the specification before the implementation was read (black box, public API).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

use curvyo_library_core::{
    AddRefusal, Catalogue, CatalogueError, IconifyError, Millimetres, Millis, READY_ADDRESS,
    RequestError, SourceKind, Sources, VerifyError, base_address, collection_request,
    collections_request, font_licence_is_known, icon_svg, icon_svg_mm, icons_request, is_ready,
    needs_credit, parse_collection, parse_collections, parse_icons, parse_search, search_request,
    search_words, sha256_hex, verify,
};

const BODY_A: &[u8] = b"abc";

fn index_with(templates: &str) -> String {
    format!(r#"{{"catalogue_format": 1, "name": "Test templates", "templates": [{templates}]}}"#)
}

fn finger_box(sha: &str) -> String {
    format!(
        r#"{{"path": "boxes/finger-box", "name": "Finger box", "author": "Jane Doe",
            "tags": ["box", "laser"],
            "licence": {{"spdx": "CC0-1.0", "name": "CC0 1.0",
                         "url": "https://creativecommons.org/publicdomain/zero/1.0/"}},
            "fonts": [], "size": 3, "sha256": "{sha}", "format_version": 58}}"#
    )
}

// Criterion 1: the ready Curvyo catalogue is the GitHub raw files of curvyo/templates (Frage 334 A).
#[test]
fn ac1_ready_entry_is_the_curvyo_templates_raw_files_only() {
    assert_eq!(
        READY_ADDRESS,
        "https://raw.githubusercontent.com/curvyo/templates/main/"
    );
    assert!(is_ready(READY_ADDRESS));
    assert!(!is_ready("https://example.com/curvyo/templates/main/"));
    assert!(!is_ready(
        "http://raw.githubusercontent.com/curvyo/templates/main/"
    ));
    assert!(!is_ready(
        "https://raw.githubusercontent.com/someone/else/main/"
    ));
}

// Criterion 1: a source added by address; a duplicate or a non-https address is refused.
#[test]
fn ac1_add_source_refuses_plain_http_and_duplicates() {
    let mut sources = Sources::default();
    let key = sources
        .add(
            SourceKind::Iconify,
            "https://api.iconify.design/",
            "Iconify",
            Millis(1),
        )
        .expect("an https Iconify address is added");
    assert_ne!(key, "");
    assert_eq!(sources.entries().len(), 1);

    assert_eq!(
        sources.add(
            SourceKind::Iconify,
            "https://api.iconify.design/",
            "Iconify",
            Millis(2)
        ),
        Err(AddRefusal::AlreadyIn)
    );
    assert!(matches!(
        sources.add(
            SourceKind::Git,
            "http://example.com/catalogue/",
            "Plain",
            Millis(3)
        ),
        Err(AddRefusal::Address(RequestError::NotHttps))
    ));
    assert_eq!(sources.entries().len(), 1);
}

// Criterion 1 and 2: the sources list round-trips as `online-sources.toml`; nothing else is stored.
#[test]
fn ac2_sources_file_round_trips_and_keeps_order() {
    let mut sources = Sources::default();
    sources
        .add(
            SourceKind::Git,
            "https://example.com/catalogue/",
            "First",
            Millis(10),
        )
        .unwrap();
    sources
        .add(
            SourceKind::Iconify,
            "https://api.example.org/",
            "Second",
            Millis(20),
        )
        .unwrap();
    let text = sources.to_text();
    let back = Sources::parse(&text).expect("the written text parses");
    assert_eq!(back, sources);
    let names: Vec<&str> = back.entries().iter().map(|e| e.name.as_str()).collect();
    assert_eq!(names, ["First", "Second"]);
}

// Criterion 2: a damaged sources file is refused, not silently emptied; a newer one too.
#[test]
fn ac2_damaged_or_newer_sources_file_is_refused() {
    assert!(Sources::parse("this is = [ not toml").is_err());
    assert!(Sources::parse("format = 999\n").is_err());
}

// Criterion 2 and 10: an added Iconify source can be removed by its key.
#[test]
fn ac1_remove_source_by_key() {
    let mut sources = Sources::default();
    let key = sources
        .add(
            SourceKind::Iconify,
            "https://api.iconify.design/",
            "Iconify",
            Millis(1),
        )
        .unwrap();
    assert!(sources.find(&key).is_some());
    assert!(sources.remove(&key));
    assert!(sources.find(&key).is_none());
    assert!(!sources.remove(&key));
}

// Criterion 2: every request is GET over https and carries only the address (no query beyond the
// protocol's own, no user name, no fragment).
#[test]
fn ac2_request_guard_refuses_non_https_and_extra_parts() {
    // Every request is built below a base address the guard accepted.
    assert!(
        base_address(
            SourceKind::Git,
            "https://raw.githubusercontent.com/curvyo/templates/main/"
        )
        .is_ok()
    );
    assert_eq!(
        base_address(SourceKind::Git, "http://raw.githubusercontent.com/x").unwrap_err(),
        RequestError::NotHttps
    );
    assert!(base_address(SourceKind::Git, "https://user:pw@example.com/x").is_err());
    assert!(base_address(SourceKind::Git, "https://example.com/x?id=42").is_err());
    assert!(base_address(SourceKind::Git, "https://example.com/x#frag").is_err());
}

// Criterion 2 and 1: a bare host normalises to an https base; a Git base keeps its folder.
#[test]
fn ac2_base_address_normalises_per_kind() {
    let git = base_address(SourceKind::Git, "https://Example.COM/catalogue").unwrap();
    assert!(git.starts_with("https://example.com/"), "{git}");
    assert!(base_address(SourceKind::Iconify, "ftp://example.com").is_err());
}

// Criterion 6: a plain catalogue index parses; the index lists each template with its hash.
#[test]
fn ac6_catalogue_index_parses_templates_with_hash_and_licence() {
    let hash = sha256_hex(BODY_A);
    let text = index_with(&finger_box(&hash));
    let catalogue = Catalogue::parse(text.as_bytes()).expect("a valid index parses");
    assert_eq!(catalogue.templates.len(), 1);
    let entry = catalogue
        .entry("boxes/finger-box")
        .expect("the template is listed");
    assert_eq!(entry.name, "Finger box");
    assert_eq!(entry.author, "Jane Doe");
    assert_eq!(entry.sha256, hash);
    assert_eq!(
        entry.licence.as_ref().map(|l| l.spdx.as_str()),
        Some("CC0-1.0")
    );
    assert_eq!(entry.size, 3);
    assert!(entry.licence_known());
}

// Criterion 6: Curvyo reads an index and writes nothing: the requests are GET of the index file.
#[test]
fn ac6_catalogue_request_is_a_get_of_the_index_file() {
    let request = Catalogue::request("https://raw.githubusercontent.com/curvyo/templates/main/");
    assert_eq!(
        request.url(),
        "https://raw.githubusercontent.com/curvyo/templates/main/curvyo-catalogue.json"
    );
}

// Criterion 6 and 0001 criterion 7 style: a newer catalogue format is refused with "made for a
// newer Curvyo"; text that is not an index is refused as not an index.
#[test]
fn ac6_newer_and_invalid_index_are_refused() {
    let newer = r#"{"catalogue_format": 99, "name": "X", "templates": []}"#;
    assert_eq!(
        Catalogue::parse(newer.as_bytes()).unwrap_err(),
        CatalogueError::Newer
    );
    assert_eq!(
        Catalogue::parse(b"<html>not an index</html>").unwrap_err(),
        CatalogueError::Invalid
    );
}

// Criterion 6 and 8: a template path that leaves its folder is not listed.
#[test]
fn ac8_template_path_outside_the_repository_is_skipped() {
    let hash = sha256_hex(BODY_A);
    let evil = finger_box(&hash).replace("boxes/finger-box", "../../etc/passwd");
    let text = index_with(&evil);
    let catalogue = Catalogue::parse(text.as_bytes()).expect("the index itself is valid");
    assert!(
        catalogue.templates.is_empty(),
        "a traversing path must not be listed"
    );
    assert!(catalogue.skipped >= 1);
}

// Criterion 4 and 6: a licence that needs attribution needs a credit text; without one the item is
// not listed as usable.
#[test]
fn ac4_attribution_licence_without_credit_is_not_listed() {
    let hash = sha256_hex(BODY_A);
    let no_credit = format!(
        r#"{{"path": "signs/plate", "name": "Plate", "author": "Max",
            "tags": [], "licence": {{"spdx": "CC-BY-4.0", "name": "CC BY 4.0",
            "url": "https://creativecommons.org/licenses/by/4.0/"}}, "fonts": [],
            "size": 3, "sha256": "{hash}", "format_version": 58}}"#
    );
    let catalogue = Catalogue::parse(index_with(&no_credit).as_bytes()).unwrap();
    assert!(catalogue.entry("signs/plate").is_none());
    assert!(catalogue.skipped >= 1);
}

// Criterion 4: which licences need the credit text (CC BY and similar), and which do not.
#[test]
fn ac4_needs_credit_for_attribution_licences_only() {
    assert!(needs_credit("CC-BY-4.0"));
    assert!(needs_credit("CC-BY-SA-4.0"));
    assert!(!needs_credit("CC0-1.0"));
    assert!(!needs_credit("MIT"));
}

// Criterion 9: a font whose licence is unknown is not fetched; known open licences are.
#[test]
fn ac9_font_licence_rule() {
    assert!(font_licence_is_known("OFL-1.1"));
    assert!(!font_licence_is_known(""));
    assert!(!font_licence_is_known("Mystery-Licence"));
}

// Criterion 8: the size and SHA-256 are checked before the bytes are used.
#[test]
fn ac8_verify_accepts_matching_bytes_and_refuses_others() {
    let hash = sha256_hex(BODY_A);
    assert_eq!(hash.len(), 64);
    assert!(verify(BODY_A, Some(3), &hash).is_ok());
    assert_eq!(verify(b"abd", Some(3), &hash), Err(VerifyError::Hash));
    assert_eq!(verify(b"abcd", Some(3), &hash), Err(VerifyError::Size));
}

// Criterion 8: a changed template fails with the entry's wording path, never read.
#[test]
fn ac8_changed_bytes_are_refused_before_any_read() {
    let original = sha256_hex(BODY_A);
    let changed = b"abX";
    assert_eq!(verify(changed, Some(3), &original), Err(VerifyError::Hash));
}

// Criterion 7: Iconify collections answer parses into sets with licence and author.
#[test]
fn ac7_iconify_collections_parse_with_licence_per_set() {
    let bytes = br#"{"mdi": {"name": "Material Design Icons", "total": 7000,
        "author": {"name": "Pictogrammers", "url": "https://pictogrammers.com"},
        "license": {"title": "Apache 2.0", "spdx": "Apache-2.0", "url": "https://www.apache.org/licenses/LICENSE-2.0"},
        "samples": ["home"], "height": 24}}"#;
    let sets = parse_collections(bytes).expect("the collections answer parses");
    assert_eq!(sets.len(), 1);
    assert_eq!(sets[0].prefix, "mdi");
    assert_eq!(sets[0].author, "Pictogrammers");
    assert!(sets[0].licence.is_some());
}

// Criterion 7: a non-Iconify answer is refused as not an Iconify API.
#[test]
fn ac7_non_iconify_answer_is_invalid() {
    assert_eq!(
        parse_collections(b"<html></html>").unwrap_err(),
        IconifyError::Invalid
    );
    assert_eq!(parse_icons(b"[1,2,3]").unwrap_err(), IconifyError::Invalid);
}

// Criterion 7: the icons of a collection parse into names.
#[test]
fn ac7_collection_answer_lists_icon_names() {
    let bytes = br#"{"prefix": "mdi", "uncategorized": ["home", "account"],
        "categories": {"Navigation": ["menu"]}, "hidden": [], "aliases": {}}"#;
    let names = parse_collection(bytes).expect("parses");
    for expected in ["home", "account", "menu"] {
        assert!(
            names.iter().any(|n| n == expected),
            "missing {expected}: {names:?}"
        );
    }
}

// Criterion 3: an icon becomes an SVG whose width is 24 mm (the import size) and that carries the
// icon's own path data.
#[test]
fn ac3_icon_svg_is_24_mm_wide_and_keeps_the_path() {
    let bytes = br#"{"prefix": "mdi", "icons": {"home": {"body": "<path d=\"M0 0h24v24H0z\"/>"}},
        "width": 24, "height": 24}"#;
    let icons = parse_icons(bytes).expect("parses");
    assert_eq!(icons.len(), 1);
    let svg = icon_svg(&icons[0]);
    assert!(svg.contains("<path"), "{svg}");
    assert!(svg.contains("M0 0h24v24H0z"), "{svg}");
    let mm = icon_svg_mm(&icons[0], Millimetres(24.0));
    assert!(mm.contains("24mm"), "the import width is 24 mm: {mm}");
}

// Criterion 3: the icon request asks for the names only, never anything of the project.
#[test]
fn ac3_icons_request_names_only() {
    let request = icons_request(
        "https://api.iconify.design",
        "mdi",
        &["home".to_owned(), "account".to_owned()],
    )
    .expect("two plain names make a request");
    let url = request.url();
    assert!(
        url.starts_with("https://api.iconify.design/mdi.json"),
        "{url}"
    );
    assert!(url.contains("home") && url.contains("account"), "{url}");
    assert!(icons_request("https://api.iconify.design/", "mdi", &[]).is_none());
}

// Criterion 10: the search words are trimmed, inner spaces collapsed, cut at 100 characters, and an
// empty text sends nothing.
#[test]
fn ac10_search_words_are_trimmed_collapsed_and_cut() {
    assert_eq!(
        search_words("   home    garden  ").as_deref(),
        Some("home garden")
    );
    assert_eq!(search_words("   \t  "), None);
    assert_eq!(search_words("").as_deref(), None);
    let long = "a".repeat(250);
    let cut = search_words(&long).expect("a long text still has words");
    assert!(
        cut.chars().count() <= 100,
        "cut at 100 characters, got {}",
        cut.chars().count()
    );
}

// Criterion 10: the search request carries the words only and is a GET to the Iconify search path.
#[test]
fn ac10_search_request_carries_words_only() {
    let request = search_request("https://api.iconify.design", "home garden");
    let url = request.url();
    assert!(
        url.starts_with("https://api.iconify.design/search?"),
        "{url}"
    );
    assert!(
        url.contains("query=home%20garden") || url.contains("query=home+garden"),
        "{url}"
    );
    assert!(!url.contains("project") && !url.contains("user"), "{url}");
}

// Criterion 10: the answer of a search parses into icon references with prefix and name.
#[test]
fn ac10_search_answer_parses_icon_references() {
    let bytes = br#"{"icons": ["mdi:home", "mdi:home-outline"], "total": 2,
        "collections": {"mdi": {"name": "Material Design Icons", "total": 7000}}}"#;
    let answer = parse_search(bytes).expect("parses");
    assert_eq!(answer.icons.len(), 2);
    assert_eq!(answer.icons[0].prefix, "mdi");
    assert_eq!(answer.icons[0].name, "home");
}

// Criterion 7 (collections request is the one GET of the catalogue of sets).
#[test]
fn ac7_collections_request_is_a_get() {
    let request = collections_request("https://api.iconify.design");
    assert_eq!(request.url(), "https://api.iconify.design/collections");
    assert!(collection_request("https://api.iconify.design", "mdi").is_some());
}

// Criterion 7 and 10: the address the maker typed with a trailing slash is normalised when stored,
// and the requests built from the stored address have one slash between host and path.
#[test]
fn ac7_stored_address_with_trailing_slash_gives_single_slash_requests() {
    let mut sources = Sources::default();
    let key = sources
        .add(
            SourceKind::Iconify,
            "https://api.iconify.design/",
            "Iconify",
            Millis(1),
        )
        .unwrap();
    let stored = sources.find(&key).unwrap().address.clone();
    let urls = [
        collections_request(&stored).url().to_owned(),
        search_request(&stored, "home").url().to_owned(),
        icons_request(&stored, "mdi", &["home".to_owned()])
            .unwrap()
            .url()
            .to_owned(),
    ];
    for url in urls {
        assert!(
            !url.replacen("https://", "", 1).contains("//"),
            "double slash in {url}"
        );
    }
}
