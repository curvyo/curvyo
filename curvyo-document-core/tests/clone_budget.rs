//! The cost of the clone read and the size of clones in a file (`specs/0049-linked-clone`
//! criteria 43, 46 and 63): 10,000 clones of one path of 100 nodes, and 1,250 chains of depth 8
//! on it, read with every clone resolved within 300 ms (a second read of the same version is the
//! session's cached read, pinned by `object_cache.rs`); the bytes a clone adds to the file are
//! printed for the PR. Budgets are asserted in release builds only; a debug build runs a tenth of
//! the size and prints the numbers.
//! Run: `cargo test --release -p curvyo-document-core --test clone_budget -- --nocapture`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::cast_precision_loss)]

use std::time::{Duration, Instant};

use curvyo_document_core::{AnchorId, Document, NewAnchor, NodeId, Point, Vec2, pack};

const CLONES: usize = if cfg!(debug_assertions) {
    1_000
} else {
    10_000
};
const READ_BUDGET: Duration = Duration::from_millis(300);

fn check(label: &str, took: Duration, budget: Duration) {
    println!("{label}: {took:?} (budget {budget:?})");
    if !cfg!(debug_assertions) {
        assert!(took <= budget, "{label}: {took:?} is over {budget:?}");
    }
}

/// A closed path of 100 nodes on a circle.
fn root(document: &Document) -> NodeId {
    let anchors: Vec<NewAnchor> = (0..100_u64)
        .map(|k| {
            let angle = k as f64 / 100.0 * std::f64::consts::TAU;
            NewAnchor::corner(
                AnchorId::new(1, k),
                Point::new(20.0 + 10.0 * angle.cos(), 20.0 + 10.0 * angle.sin()),
            )
        })
        .collect();
    document.create_path(&anchors, true)
}

fn direct_clones() -> Document {
    let document = Document::new(1);
    let original = root(&document);
    for k in 0..CLONES {
        let clone = document.linked_clone(&[original], &[]).unwrap()[0];
        document
            .translate_objects(
                &[clone],
                Vec2::new((k % 100) as f64 * 25.0, (k / 100) as f64 * 25.0),
            )
            .unwrap();
    }
    document
}

fn chains() -> Document {
    let document = Document::new(1);
    let original = root(&document);
    for _ in 0..CLONES / 8 {
        let mut last = original;
        for _ in 0..8 {
            last = document.linked_clone(&[last], &[]).unwrap()[0];
        }
    }
    document
}

#[test]
fn ac43_63_reading_ten_thousand_clones_direct_or_in_chains() {
    for (label, document) in [
        ("direct clones", direct_clones()),
        ("chains of 8", chains()),
    ] {
        let started = Instant::now();
        let read = document.read_objects();
        check(
            &format!("read of {CLONES} {label}"),
            started.elapsed(),
            READ_BUDGET,
        );
        assert_eq!(read.objects.len(), CLONES + 1);
        assert_eq!(read.orphans, Vec::<NodeId>::new());
    }
}

#[test]
fn ac46_the_bytes_a_clone_adds_to_the_file() {
    let with = pack(&direct_clones(), "0.1.0").unwrap().len();
    let without = {
        let document = Document::new(1);
        root(&document);
        pack(&document, "0.1.0").unwrap().len()
    };
    let per_clone = (with - without) as f64 / CLONES as f64;
    println!(
        "{CLONES} clones add {} bytes: {per_clone:.0} bytes per clone (each with one move)",
        with - without
    );
    assert!(
        per_clone < 1_000.0,
        "a few hundred bytes per clone, not {per_clone:.0}"
    );
}
