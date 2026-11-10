//! The drag answer contract (`specs/0100-bring-content-in` `adrs.md` note 4, architecture review
//! finding 3): one answer per drop badge, as Rust serialises it for the row drag and the cloning
//! drop, equals `frontend/src/lib/fixtures/drag-answers.json`, which `dragAnswer.test.ts`
//! decodes. A new badge fails to compile here until it is listed. `CURVYO_UPDATE_GOLDEN=1`
//! rewrites the fixture.
#![allow(clippy::unwrap_used, missing_docs)]

use std::path::PathBuf;

use curvyo_editor_wasm::{DropBadge, RowDragAnswer};

/// Every badge, by an exhaustive match: a new variant must be added here.
fn every_badge() -> Vec<DropBadge> {
    let all = [
        DropBadge::None,
        DropBadge::Clone,
        DropBadge::NotOnSketchPage,
        DropBadge::NoSketch,
        DropBadge::Move,
        DropBadge::Copy,
        DropBadge::PlacedPage,
        DropBadge::OnlyOnLaserPage,
    ];
    for badge in all {
        match badge {
            DropBadge::None
            | DropBadge::Clone
            | DropBadge::NotOnSketchPage
            | DropBadge::NoSketch
            | DropBadge::Move
            | DropBadge::Copy
            | DropBadge::PlacedPage
            | DropBadge::OnlyOnLaserPage => {}
        }
    }
    all.to_vec()
}

fn answer(badge: DropBadge) -> RowDragAnswer {
    let refused = matches!(
        badge,
        DropBadge::NotOnSketchPage | DropBadge::OnlyOnLaserPage
    );
    RowDragAnswer {
        badge,
        target: (!matches!(badge, DropBadge::None | DropBadge::NoSketch) && !refused)
            .then(|| "mfg-1-2".to_owned()),
        faint: refused,
        // `0102`: a fit cue is left out of the JSON when there is none.
        fit: None,
    }
}

#[test]
fn every_badge_answer_matches_the_frontend_fixture() {
    let answers: Vec<RowDragAnswer> = every_badge().into_iter().map(answer).collect();
    let text = serde_json::to_string_pretty(&answers).unwrap() + "\n";
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../frontend/src/lib/fixtures/drag-answers.json");
    if std::env::var_os("CURVYO_UPDATE_GOLDEN").is_some() {
        std::fs::write(&path, &text).unwrap();
    }
    let want = std::fs::read_to_string(&path).unwrap();
    assert_eq!(text, want, "the frontend fixture is the Rust payload");
}
