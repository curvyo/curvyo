//! Independent tester cases for `specs/0077-font-preview/specification.md` on `curvyo-ui-core`:
//! criterion 7 and 8 (the sample is the first line, at most 200 characters) and criterion 23's
//! memory bound for 2,000 families (`adrs.md` decision 2: 256 MiB, the least recently drawn face
//! dropped first, `has` does not count as a draw).
//!
//! Written from the specification, `adrs.md` and the public names before the implementation was
//! read.

#![allow(clippy::unwrap_used, missing_docs)]

use std::path::PathBuf;

use curvyo_ui_core::{MAX_SAMPLE_CHARS, PREVIEW_FACE_BUDGET, PreviewFaces, sample_line};

fn font(file: &str) -> Vec<u8> {
    std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../frontend/public/fonts")
            .join(file),
    )
    .unwrap()
}

/// Criterion 7: the field holds at most 200 characters; Criterion 8: a Windows line break ends
/// the first line too, and a text starting with a line break has an empty first line.
#[test]
fn ac07_ac08_first_line_limits_and_line_breaks() {
    assert_eq!(MAX_SAMPLE_CHARS, 200);
    assert_eq!(sample_line("Anna\r\nJonas"), "Anna");
    assert_eq!(sample_line("\nJonas"), "");
    assert_eq!(sample_line(""), "");
    let long = "ß".repeat(250);
    assert_eq!(sample_line(&long).chars().count(), 200);
    let emoji = "👩‍👩‍👧".repeat(100);
    assert!(sample_line(&emoji).chars().count() <= 200);
}

/// Criterion 23 with `adrs.md` decision 2: the budget is 256 MiB, and a face only asked about
/// with `has` is not marked as drawn, so it is the one dropped.
#[test]
fn ac23_budget_and_has_is_not_a_draw() {
    assert_eq!(PREVIEW_FACE_BUDGET, 256 * 1024 * 1024);
    let a = font("source-sans-3/SourceSans3-Regular.otf");
    let b = font("great-vibes/GreatVibes-Regular.ttf");
    let budget = a.len() + b.len() + 16;
    let mut faces = PreviewFaces::with_budget(budget);
    faces.add("a", a.clone(), 0).unwrap();
    faces.add("b", b, 0).unwrap();
    assert!(faces.has("a"));
    assert!(faces.get("b").is_some());
    faces.add("c", a, 0).unwrap();
    assert!(
        !faces.has("a"),
        "a was only asked about, never drawn since b"
    );
    assert!(faces.has("b") && faces.has("c"));
    assert!(faces.held_bytes() <= budget);
}
