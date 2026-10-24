//! The glyph table's engine budgets (`specs/0082-glyph-table` criteria 10 and 21, `adrs.md`
//! decision 3): listing a face of 3,000 and 20,000 characters, one screen of cells. Measured in
//! release (`cargo test --release -p curvyo-text-core --test glyph_budget -- --nocapture`);
//! asserted only in release and off CI, printed always.
#![allow(clippy::unwrap_used)]

mod common;

use std::sync::Arc;
use std::time::{Duration, Instant};

use common::{SANS, font_bytes, sfnt};
use curvyo_text_core::{Face, FaceStore, glyph_cell};

fn face_of(bytes: Vec<u8>) -> Arc<Face> {
    let mut store = FaceStore::new();
    let key = store.add(bytes, 0).unwrap();
    store.get(&key).unwrap().clone()
}

/// A face mapping `count` characters from U+4E00 on, one square glyph each.
fn wide_face(count: u32) -> Vec<u8> {
    let names: Vec<&'static str> = (0..count).map(|_| "g").collect();
    sfnt::build(&sfnt::TestFace {
        glyphs: names.into_iter().map(|name| (name, true)).collect(),
        subtables: vec![sfnt::Subtable {
            platform: 3,
            encoding: 10,
            format: 12,
            map: (0..count)
                .map(|i| (0x4E00 + i, u16::try_from(i + 1).unwrap()))
                .collect(),
        }],
        post_names: false,
    })
}

fn check(label: &str, took: Duration, budget: Duration) {
    println!("{label}: {took:?} (budget {budget:?})");
    if !cfg!(debug_assertions) && std::env::var_os("CI").is_none() {
        assert!(took <= budget, "{label}: {took:?} over {budget:?}");
    }
}

#[test]
fn listing_3000_and_20000_characters() {
    for (count, budget) in [(3_000, 20), (20_000, 150)] {
        let face = face_of(wide_face(count));
        let start = Instant::now();
        let list = face.listed_chars();
        let took = start.elapsed();
        assert_eq!(list.len(), usize::try_from(count).unwrap());
        check(
            &format!("list {count}"),
            took,
            Duration::from_millis(budget),
        );
    }
}

#[test]
fn one_screen_of_cells() {
    let face = face_of(font_bytes(SANS));
    let list = face.listed_chars();
    let start = Instant::now();
    let drawn = list
        .iter()
        .skip(100)
        .take(200)
        .filter_map(|listed| glyph_cell(&face, listed.c))
        .count();
    let took = start.elapsed();
    assert_eq!(drawn, 200);
    check("200 cells", took, Duration::from_millis(100));
}
