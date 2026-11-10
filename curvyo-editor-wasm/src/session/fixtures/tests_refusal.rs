//! The refusals of an unknown fixture (criterion 32) and the removal notice of a profile pick
//! (criterion 29, ADR 0050 section 6), at session level.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use curvyo_document_core::NodeId;

use super::super::Session;
use super::FixtureKindView;
use super::commands::READ_ONLY_NOTICE;
use super::tests::{laser, mm, view};

/// The unknown-kind golden: a laser page with one fixture of a kind this build does not know.
const UNKNOWN_GOLDEN: &[u8] = include_bytes!(
    "../../../../curvyo-document-core/tests/fixtures/fixtures_unknown_kind_v40.curvyo"
);

fn unknown_page() -> (Session, NodeId) {
    let document = curvyo_document_core::unpack(9_401, UNKNOWN_GOLDEN).unwrap();
    let mut s = Session::open(9_401, UNKNOWN_GOLDEN).unwrap();
    for page in document.pages() {
        if s.activate_page(&page.id) && s.fixtures_view().is_some() {
            break;
        }
    }
    let row = view(&s)
        .rows
        .into_iter()
        .find(|r| r.kind == FixtureKindView::Unknown)
        .expect("the golden has an unknown fixture");
    let id = s.fixture_part_named(&row.id).unwrap();
    (s, id)
}

#[test]
fn criterion_32_an_unknown_fixture_refuses_every_edit_but_rename_and_remove() {
    let (mut s, id) = unknown_page();
    let steps = s.undo_len();
    assert!(!s.set_fixture_active(id, false));
    assert_eq!(s.take_fixture_notice().as_deref(), Some(READ_ONLY_NOTICE));
    assert!(!s.set_fixture_visible(id, false), "the eye is refused too");
    assert!(!s.set_fixture_length(id, "spacing_mm", mm(5.0)));
    assert!(
        !s.open_fixture_entry_for_key('m'),
        "an unknown fixture has no chip"
    );
    assert_eq!(s.undo_len(), steps, "a refused edit is no history step");
    assert!(s.rename_fixture(id, "Turntable A"), "rename stays open");
    assert!(s.delete_fixture(id), "remove stays open");
}

#[test]
fn criterion_29_a_pick_or_reset_posts_the_notice_only_when_fixtures_go() {
    let mut s = laser();
    // The library as the app loads it at start: empty, the built-in Generic laser only.
    s.load_machine_profiles_json("[]");
    // Nothing to remove (the page already holds Generic laser): no notice.
    let _ = s.pick_machine_profile("generic-laser");
    assert_eq!(s.take_fixture_notice(), None);
    // A grid goes with the pick: the removal is named, with its undo.
    s.add_fixture("grid").unwrap();
    assert!(s.pick_machine_profile("generic-laser"));
    assert_eq!(
        s.take_fixture_notice().as_deref(),
        Some("Fixtures removed (undo restores them).")
    );
    // The set is empty now: picking again removes nothing.
    let _ = s.pick_machine_profile("generic-laser");
    assert_eq!(s.take_fixture_notice(), None);
}
