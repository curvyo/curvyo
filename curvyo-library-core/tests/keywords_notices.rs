//! The shipped keyword file and its notices (spec `0153` criteria 4 and 10): it decodes, stays
//! within 320 KiB gzipped, and each source's project and version are in its header, in
//! `keywords-LICENSES.txt` beside it and in `docs/third-party-assets.md`.

#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

use std::path::{Path, PathBuf};

use curvyo_library_core::Keywords;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .into()
}

#[test]
fn the_shipped_file_decodes_within_its_size_and_both_notices_name_every_source() {
    let bytes = std::fs::read(root().join("frontend/public/library/keywords.bin.gz")).unwrap();
    assert!(bytes.len() <= 320 * 1024, "{} bytes", bytes.len());
    let keywords = Keywords::decode(&bytes).unwrap();
    let licences =
        std::fs::read_to_string(root().join("frontend/public/library/keywords-LICENSES.txt"))
            .unwrap();
    let assets = std::fs::read_to_string(root().join("docs/third-party-assets.md")).unwrap();
    let section = assets
        .split("\n## ")
        .find(|s| s.starts_with("Search keywords"))
        .expect("a Search keywords section");
    let prefixes: Vec<&str> = keywords
        .sources()
        .iter()
        .map(|s| s.prefix.as_str())
        .collect();
    assert_eq!(prefixes, ["lucide", "tabler", "mdi"]);
    for source in keywords.sources() {
        let line = format!("{} {}", source.project, source.version);
        assert!(licences.contains(&line), "{line} in keywords-LICENSES.txt");
        assert!(
            section.contains(&format!("| {} |", source.project))
                && section.contains(&format!("`{}` {}", source.package, source.version)),
            "{line} in docs/third-party-assets.md"
        );
        assert!(
            licences.contains(&format!("Licence of the file: {}", source.licence)),
            "{}",
            source.licence
        );
    }
    assert!(keywords.of("lucide", "search").is_some());
    assert!(keywords.of("tabler", "trash-filled").is_some());
    assert!(keywords.of("mdi", "magnify").is_some());
}
