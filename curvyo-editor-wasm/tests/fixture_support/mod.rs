//! The set add bar of `specs/0113-fixture-sets-and-pins` criterion 7 for tests that were written
//! against the per-kind add buttons of `0102` (superseded by `0113`, Part C): a fixture of `kind`
//! is added into the active laser page's first set, "Set 1" made first when the page has none.

use curvyo_document_core::NodeId;
use curvyo_editor_wasm::Session;

/// Adds a fixture of `kind` (`grid`, `holder`, `clamp`, `stop`, `screw`, `pin`) into the first set
/// of the active laser page; `None` where the set add bar adds none.
pub fn add_fixture(s: &mut Session, kind: &str) -> Option<NodeId> {
    editable_sets(s);
    let first = s
        .fixtures_view()?
        .sets
        .first()
        .and_then(|set| s.fixture_set_named(&set.id));
    let set = match first {
        Some(set) => set,
        None => s.add_fixture_set()?,
    };
    s.add_set_fixture(set, kind)
}

/// Makes the active laser page's sets editable: a page on Generic laser is saved as a profile
/// first (`0113` rework, criterion 3: sets live in a machine profile).
pub fn editable_sets(s: &mut Session) {
    if !s.fixture_sets_editable() {
        s.save_machine_profile([7; 16], 1_000);
    }
}
