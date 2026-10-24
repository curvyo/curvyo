//! The glyph table's search budget (`specs/0082-glyph-table` criterion 16, `adrs.md` decision
//! 3): one keystroke over 3,000 listed characters. Measured in release (`cargo test --release -p
//! curvyo-ui-core --test glyph_search_budget -- --nocapture`); asserted only in release and off
//! CI, printed always.
#![allow(clippy::unwrap_used)]

use std::time::{Duration, Instant};

use curvyo_text_core::{ListedChar, UnicodeNames};
use curvyo_ui_core::glyph_table;

#[test]
fn a_search_keystroke_over_3000_characters() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let bytes = std::fs::read(dir.join("../frontend/public/unicode/names-17.0.0.bin")).unwrap();
    let names = UnicodeNames::parse(&bytes).unwrap();
    let list: Vec<ListedChar> = (0x20..0x3000_u32)
        .filter_map(char::from_u32)
        .filter(|c| names.name_of(*c).is_some())
        .take(3_000)
        .map(|c| ListedChar {
            c,
            glyph: 1,
            name: Some("uni".into()),
        })
        .collect();
    assert_eq!(list.len(), 3_000);
    let mut worst = Duration::ZERO;
    for query in [
        "l",
        "la",
        "lat",
        "latin small",
        "a diaeresis",
        "U+00E4",
        "arrows",
        "xyz",
    ] {
        let start = Instant::now();
        let table = glyph_table(&list, query, Some(&names));
        worst = worst.max(start.elapsed());
        assert!(table.shown <= 3_000);
    }
    println!("worst search keystroke over 3000: {worst:?} (budget 30 ms)");
    if !cfg!(debug_assertions) && std::env::var_os("CI").is_none() {
        assert!(worst <= Duration::from_millis(30), "{worst:?}");
    }
}
