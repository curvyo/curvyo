//! The parts of a fixture as a fixture job's Parts list names them (`specs/0103-fixture-jobs`
//! criteria 8, 9, 12; UX notes 5.2): the row name, the stored key and the node "Select" selects.

use curvyo_document_core::{FixturePartKey, NodeId};
use curvyo_laser_core::FixturePart;

use super::super::placed::PlacedFixture;

/// The row name of `part` of `fixture` (UX notes 5.2): a slot is named as the slot.
pub(super) fn part_name(fixture: &PlacedFixture, part: FixturePart) -> String {
    match part {
        FixturePart::Board => "Board".to_owned(),
        FixturePart::Slot(index) => fixture
            .slots
            .get(index)
            .map_or_else(String::new, |s| s.name.clone()),
        FixturePart::Clamp => "Clamp outline".to_owned(),
        FixturePart::Stop => "Stop".to_owned(),
        FixturePart::GridLines => "Grid lines".to_owned(),
        FixturePart::GridTapes => "Tapes".to_owned(),
        FixturePart::ScrewHead => "Head".to_owned(),
        FixturePart::ScrewThread => "Thread".to_owned(),
    }
}

/// The node "Select" selects for `part`: the slot, else the fixture (UX notes 5.3, J5).
pub(super) fn part_node(fixture: &PlacedFixture, part: FixturePart) -> Option<NodeId> {
    match part {
        FixturePart::Slot(index) => fixture.slots.get(index).map(|s| s.id),
        _ => Some(fixture.id),
    }
}

/// The stored key of `part` of `fixture`: its node and its suffix (ADR 0052 §2).
pub(super) fn part_key(fixture: &PlacedFixture, part: FixturePart) -> Option<FixturePartKey> {
    let target = part.target();
    Some(FixturePartKey {
        node: part_node(fixture, part)?,
        suffix: target.suffix,
    })
}

/// The part the token `token` names on `fixture` (`board`, `slot:0`, `grid.lines`, ...).
pub(super) fn part_of_token(fixture: &PlacedFixture, token: &str) -> Option<FixturePart> {
    FixturePart::from_token(fixture.kind, fixture.slots.len(), token)
}

/// The parts of `fixture` in Parts-list order (criterion 8).
pub(super) fn parts_of(fixture: &PlacedFixture) -> Vec<FixturePart> {
    FixturePart::of_kind(fixture.kind, fixture.slots.len())
}
