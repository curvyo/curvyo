#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Golden tests of the catalogue index and the Iconify answers (ADR 0085): recorded response
//! shapes in `tests/fixtures/`, no network.

use curvyo_library_core::{
    Catalogue, CatalogueError, IconifyError, VerifyError, icon_svg, parse_collection,
    parse_collections, parse_icons, parse_search, sha256_hex, verify,
};

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(format!(
        "{}/tests/fixtures/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
}

#[test]
fn the_catalogue_keeps_its_good_entries_and_counts_the_rest() {
    let catalogue = Catalogue::parse(&fixture("curvyo-catalogue.json")).unwrap();
    assert_eq!(catalogue.name, "Curvyo templates");
    let paths: Vec<&str> = catalogue
        .templates
        .iter()
        .map(|t| t.path.as_str())
        .collect();
    assert_eq!(
        paths,
        [
            "boxes/finger-box",
            "signs/name-plate",
            "misc/no-licence",
            "big/huge"
        ]
    );
    // A path escape, a CC BY entry without credit, a bad hash and a duplicate path.
    assert_eq!(catalogue.skipped, 4);
}

#[test]
fn an_entry_states_its_licence_credit_and_fonts() {
    let catalogue = Catalogue::parse(&fixture("curvyo-catalogue.json")).unwrap();
    let plate = catalogue.entry("signs/name-plate").unwrap();
    assert_eq!(plate.licence.as_ref().unwrap().name, "CC BY 4.0");
    assert_eq!(
        plate.credit.as_deref(),
        Some("Name plate by Max Mustermann, CC BY 4.0")
    );
    assert!(
        plate.fonts_withheld(),
        "a font without a licence is not fetched"
    );
    assert_eq!(plate.format_version, Some(999));
    let finger = catalogue.entry("boxes/finger-box").unwrap();
    assert!(!finger.fonts_withheld());
    assert!(finger.licence_known());
    let none = catalogue.entry("misc/no-licence").unwrap();
    assert!(
        !none.licence_known(),
        "a missing licence is never read as free"
    );
    assert!(catalogue.entry("big/huge").unwrap().too_large());
}

#[test]
fn the_requests_of_an_entry_sit_below_the_base() {
    let catalogue = Catalogue::parse(&fixture("curvyo-catalogue.json")).unwrap();
    let base = "https://example.org/cat/";
    assert_eq!(
        Catalogue::request(base).url(),
        "https://example.org/cat/curvyo-catalogue.json"
    );
    let entry = catalogue.entry("boxes/finger-box").unwrap();
    assert_eq!(
        entry.template_request(base).url(),
        "https://example.org/cat/boxes/finger-box/template.curvyo"
    );
    assert_eq!(
        entry.thumbnail_request(base).url(),
        "https://example.org/cat/boxes/finger-box/thumbnail.png"
    );
}

#[test]
fn a_newer_or_foreign_or_oversized_index_is_refused() {
    assert_eq!(
        Catalogue::parse(&fixture("catalogue_newer.json")),
        Err(CatalogueError::Newer)
    );
    assert_eq!(
        Catalogue::parse(b"<html>404</html>"),
        Err(CatalogueError::Invalid)
    );
    assert_eq!(
        Catalogue::parse(b"{\"name\": \"x\"}"),
        Err(CatalogueError::Invalid)
    );
    let big = vec![b' '; curvyo_library_core::MAX_INDEX_BYTES + 1];
    assert_eq!(Catalogue::parse(&big), Err(CatalogueError::TooLarge));
}

#[test]
fn a_template_is_checked_against_its_entry() {
    let catalogue = Catalogue::parse(&fixture("curvyo-catalogue.json")).unwrap();
    let entry = catalogue.entry("boxes/finger-box").unwrap();
    assert_eq!(verify(b"abc", Some(entry.size), &entry.sha256), Ok(()));
    assert_eq!(
        verify(b"abd", Some(entry.size), &entry.sha256),
        Err(VerifyError::Hash)
    );
    assert_eq!(sha256_hex(b"abc"), entry.sha256);
}

#[test]
fn collections_carry_the_licence_and_author_of_each_set() {
    let sets = parse_collections(&fixture("iconify_collections.json")).unwrap();
    let prefixes: Vec<&str> = sets.iter().map(|s| s.prefix.as_str()).collect();
    assert_eq!(prefixes, ["mdi", "no-licence"]);
    let mdi = &sets[0];
    assert_eq!(mdi.name, "Material Design Icons");
    assert_eq!(mdi.total, 7447);
    assert_eq!(mdi.author, "Pictogrammers");
    assert_eq!(mdi.licence.as_ref().unwrap().spdx, "Apache-2.0");
    assert_eq!(mdi.samples, ["account", "home", "cog"]);
    assert!(sets[1].licence.is_none());
    assert_eq!(parse_collections(b"[1]"), Err(IconifyError::Invalid));
    assert_eq!(
        parse_collections(&vec![b' '; 8 * 1024 * 1024 + 1]),
        Err(IconifyError::TooLarge)
    );
}

#[test]
fn a_collection_lists_each_icon_once() {
    let names = parse_collection(&fixture("iconify_collection.json")).unwrap();
    assert_eq!(names, ["home", "account", "arrow-left"]);
    assert_eq!(parse_collection(b"{}"), Err(IconifyError::Invalid));
}

#[test]
fn icon_bodies_resolve_aliases_and_build_an_svg() {
    let icons = parse_icons(&fixture("iconify_icons.json")).unwrap();
    let names: Vec<&str> = icons.iter().map(|i| i.name.as_str()).collect();
    assert_eq!(names, ["home", "wide", "house", "house-flipped"]);
    let wide = icons.iter().find(|i| i.name == "wide").unwrap();
    assert_eq!(
        icon_svg(wide),
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"1 2 30 10\" width=\"30\" height=\"10\"><path d=\"M0 0h30v10H0z\"/></svg>"
    );
    let house = icons.iter().find(|i| i.name == "house").unwrap();
    assert_eq!(house.body, icons[0].body);
    let flipped = icons.iter().find(|i| i.name == "house-flipped").unwrap();
    assert!(flipped.h_flip);
    assert!(icon_svg(flipped).contains("scale(-1 1)"));
}

#[test]
fn a_search_answer_keeps_valid_results_and_the_sets_info() {
    let answer = parse_search(&fixture("iconify_search.json")).unwrap();
    let found: Vec<String> = answer
        .icons
        .iter()
        .map(|i| format!("{}:{}", i.prefix, i.name))
        .collect();
    assert_eq!(found, ["mdi:home", "mdi-light:home", "tabler:home-2"]);
    assert_eq!(answer.sets.len(), 2);
    assert_eq!(answer.sets[1].licence.as_ref().unwrap().name, "MIT");
    assert_eq!(parse_search(b"{\"total\": 0}"), Err(IconifyError::Invalid));
}
