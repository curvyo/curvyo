//! The fixture commands of the Fixtures block and of the canvas handles (`specs/0102-laser-fixtures`
//! criteria 2 to 6, 13 to 15, 18, 22, 23, 37, 38): each one history step, values set to their
//! nearest limit with a notice, never refused for range.

use curvyo_document_core::{
    Affine, FixtureAdd, FixtureParams, FixtureResize, FixtureTurn, Length, NewFixture, NodeId, Vec2,
};
use curvyo_laser_core::FixtureKind;
use serde_json::Value;

use super::super::Session;
use super::edit::FixtureEdit;
use super::params::{accept_major_every, fixture_field, new_params, with_number};
use super::placed::{PageFixtures, PlacedFixture};

/// The kind a button or the `0093` manifest names: `grid` or `engraved_grid`, `holder`, `clamp`,
/// `stop`, `screw`.
fn kind_named(name: &str) -> Option<(FixtureKind, FixtureAdd)> {
    match name {
        "grid" | "engraved_grid" => Some((FixtureKind::EngravedGrid, FixtureAdd::Grid)),
        "holder" => Some((FixtureKind::Holder, FixtureAdd::Holder)),
        "clamp" => Some((FixtureKind::Clamp, FixtureAdd::Clamp)),
        "stop" => Some((FixtureKind::Stop, FixtureAdd::Stop)),
        "screw" => Some((FixtureKind::Screw, FixtureAdd::Screw)),
        _ => None,
    }
}

/// The stored tape names in the order of the Tapes on group.
const TAPE_EDGES: [&str; 4] = ["bottom", "left", "top", "right"];

/// The refusal of an edit on an unknown fixture (criterion 32): it can only be renamed or removed.
pub(crate) const READ_ONLY_NOTICE: &str =
    "Read-only: an unknown fixture can only be renamed or removed.";

impl Session {
    /// One of the five add buttons (criterion 2): a fixture "`Kind` N", Active, centred on the
    /// work area at its Proposal size, selected, in one step "Add `kind`". `None` for an unknown
    /// kind, off a laser page, or for a second grid (the Grid button is hidden then).
    pub fn add_fixture(&mut self, kind: &str) -> Option<NodeId> {
        let (kind, add) = kind_named(kind)?;
        let fixtures = self.active_fixtures()?;
        if kind == FixtureKind::EngravedGrid && fixtures.has_grid() {
            return None;
        }
        let stored = kind.stored()?;
        let params = new_params(kind);
        let matrix = (kind != FixtureKind::EngravedGrid).then(|| centred(&fixtures, kind));
        let id = self.recording_unguarded(|this| {
            this.document
                .add_fixture(
                    fixtures.manufacturing,
                    &NewFixture {
                        add,
                        kind: stored,
                        label: kind.word(),
                        params: &params,
                        matrix,
                    },
                )
                .ok()
        })?;
        self.select_fixture(id);
        Some(id)
    }

    /// Renames the fixture `id` (criterion 3): cut to 40 characters; one step "Rename fixture".
    pub fn rename_fixture(&mut self, id: NodeId, name: &str) -> bool {
        if self.page_fixture(id).is_none() {
            return false;
        }
        self.recording_unguarded(|this| this.document.rename_fixture(id, name).unwrap_or(false))
    }

    /// Removes the fixture `id` with its slots (criterion 4): one step "Remove fixture".
    pub fn delete_fixture(&mut self, id: NodeId) -> bool {
        if self.page_fixture(id).is_none() {
            return false;
        }
        let deleted = self.recording_unguarded(|this| this.document.delete_fixture(id).is_ok());
        if deleted && self.fixtures.selected.is_some_and(|s| s.fixture == id) {
            self.clear_fixture_selection();
        }
        deleted
    }

    /// Switches the fixture `id` on or off the bed (criterion 5); several may be Active.
    pub fn set_fixture_active(&mut self, id: NodeId, active: bool) -> bool {
        if self.editable(id).is_none() {
            return false;
        }
        self.recording_unguarded(|this| {
            this.document
                .set_fixture_active(id, active)
                .unwrap_or(false)
        })
    }

    /// The eye of the fixture `id` (criterion 6): one step "Show or hide fixture".
    pub fn set_fixture_visible(&mut self, id: NodeId, visible: bool) -> bool {
        if self.editable(id).is_none() {
            return false;
        }
        self.recording_unguarded(|this| {
            this.document
                .set_fixture_visible(id, visible)
                .unwrap_or(false)
        })
    }

    /// Sets the length field `key` of the grid `id` (spacing and origin, criterion 15), set to the
    /// nearest limit with a notice. `false` for a key the kind does not have (holder, clamp, stop
    /// and screw have no block field: their handles size them), or a value already stored.
    pub fn set_fixture_length(&mut self, id: NodeId, key: &str, value: Length) -> bool {
        let Some((_, fixture)) = self.editable(id) else {
            return false;
        };
        let Some(field) = fixture_field(fixture.kind, key) else {
            return false;
        };
        let (kept, notice) = field.accept(value);
        self.fixtures.notice = notice;
        let params = with_number(&self.stored_params(id), field.key, kept);
        let label = if field.key == "spacing_mm" {
            curvyo_document_core::SET_GRID_SPACING_COMMIT_LABEL
        } else {
            curvyo_document_core::SET_GRID_ORIGIN_COMMIT_LABEL
        };
        self.write_fixture_params(id, &params, label)
    }

    /// Sets the grid's Major every (criterion 15): a whole count of 2 to 20 lines.
    pub fn set_grid_major_every(&mut self, id: NodeId, count: f64) -> bool {
        if !self.is_grid(id) {
            return false;
        }
        let (kept, notice) = accept_major_every(count);
        self.fixtures.notice = notice;
        // A whole number: laser-core reads the count as an unsigned integer.
        let mut params = self.stored_params(id);
        params.insert("major_every".to_owned(), Value::from(kept));
        self.write_fixture_params(
            id,
            &params,
            curvyo_document_core::SET_GRID_MAJOR_COMMIT_LABEL,
        )
    }

    /// Switches the grid's tape on `edge` (`bottom`, `left`, `top`, `right`; criterion 15).
    pub fn set_grid_tape(&mut self, id: NodeId, edge: &str, on: bool) -> bool {
        if !self.is_grid(id) || !TAPE_EDGES.contains(&edge) {
            return false;
        }
        let Some((_, fixture)) = self.page_fixture(id) else {
            return false;
        };
        let super::placed::Shape::Grid(spec) = fixture.shape else {
            return false;
        };
        let tapes = spec.tapes;
        let now = [tapes.bottom, tapes.left, tapes.top, tapes.right];
        let names: Vec<Value> = TAPE_EDGES
            .iter()
            .zip(now)
            .filter(|(name, was)| if **name == edge { on } else { *was })
            .map(|(name, _)| Value::from(*name))
            .collect();
        let mut params = self.stored_params(id);
        params.insert("tapes".to_owned(), Value::Array(names));
        self.write_fixture_params(
            id,
            &params,
            curvyo_document_core::SET_TAPE_EDGE_COMMIT_LABEL,
        )
    }

    /// The fixture `id` of the active laser page with its page's fixtures.
    pub(super) fn page_fixture(&self, id: NodeId) -> Option<(PageFixtures, PlacedFixture)> {
        let fixtures = self.active_fixtures()?;
        let fixture = fixtures.get(id)?.clone();
        Some((fixtures, fixture))
    }

    /// The fixture `id` for an edit other than rename and remove: an unknown kind is read-only
    /// (criterion 32), so it is refused with the notice `READ_ONLY_NOTICE`.
    fn editable(&mut self, id: NodeId) -> Option<(PageFixtures, PlacedFixture)> {
        let (fixtures, fixture) = self.page_fixture(id)?;
        if fixture.kind == FixtureKind::Unknown {
            self.fixtures.notice = Some(READ_ONLY_NOTICE.to_owned());
            return None;
        }
        Some((fixtures, fixture))
    }

    /// Writes `edit` of the fixture `id` as one step labelled `label` (`move_fixture`,
    /// `rotate_fixture` or `resize_fixture`): a move or a turn writes the matrix alone; a resize
    /// writes the sizes, the matrix and the moved slots' centres together (`adrs.md` §6a.5).
    pub(super) fn write_fixture_edit(
        &mut self,
        id: NodeId,
        edit: &FixtureEdit,
        label: &str,
    ) -> bool {
        if label != curvyo_document_core::RESIZE_FIXTURE_COMMIT_LABEL {
            let turn = if label == curvyo_document_core::ROTATE_FIXTURE_COMMIT_LABEL {
                FixtureTurn::Rotate
            } else {
                FixtureTurn::Move
            };
            return self.write_fixture_matrix(id, edit.matrix, turn);
        }
        let Some(stored) = self.document.fixture(id) else {
            return false;
        };
        let mut params = stored.params.clone();
        for (key, value) in edit.shape.params() {
            params.insert(key.to_owned(), Value::from(value));
        }
        let slots: Vec<(NodeId, FixtureParams)> = edit
            .slots
            .iter()
            .filter_map(|(slot, centre)| {
                let mut values = stored.slots.iter().find(|s| s.id == *slot)?.params.clone();
                values.insert("cx_mm".to_owned(), Value::from(centre.x));
                values.insert("cy_mm".to_owned(), Value::from(centre.y));
                Some((*slot, values))
            })
            .collect();
        let matrix = (stored.matrix != Some(edit.matrix)).then_some(edit.matrix);
        let resize = FixtureResize {
            params: &params,
            matrix,
            slots: &slots,
        };
        self.recording_unguarded(|this| this.document.resize_fixture(id, &resize).unwrap_or(false))
    }

    pub(super) fn write_fixture_matrix(
        &mut self,
        id: NodeId,
        m: Affine,
        turn: FixtureTurn,
    ) -> bool {
        self.recording_unguarded(|this| {
            this.document
                .set_fixture_matrix(id, m, turn)
                .unwrap_or(false)
        })
    }

    fn is_grid(&self, id: NodeId) -> bool {
        self.page_fixture(id)
            .is_some_and(|(_, f)| f.kind == FixtureKind::EngravedGrid)
    }

    fn stored_params(&self, id: NodeId) -> FixtureParams {
        self.document
            .fixture(id)
            .map(|f| f.params)
            .unwrap_or_default()
    }

    fn write_fixture_params(&mut self, id: NodeId, params: &FixtureParams, label: &str) -> bool {
        self.recording_unguarded(|this| {
            this.document
                .set_fixture_params_labelled(id, params, label)
                .unwrap_or(false)
        })
    }
}

/// Whether the fixture moves: holders, clamps, stops and screws (criteria 13, 14).
pub(super) fn movable(fixture: &PlacedFixture) -> bool {
    matches!(
        fixture.kind,
        FixtureKind::Holder | FixtureKind::Clamp | FixtureKind::Stop | FixtureKind::Screw
    )
}

/// The matrix that centres a new fixture of `kind` on the work area (a screw's frame origin is
/// its centre).
fn centred(fixtures: &PageFixtures, kind: FixtureKind) -> Affine {
    let size = fixtures.page.size;
    let (w, h) = match kind {
        FixtureKind::Screw => (0.0, 0.0),
        FixtureKind::Holder => super::placed::HOLDER_SIZE_MM,
        FixtureKind::Clamp => super::placed::CLAMP_SIZE_MM,
        _ => {
            let leg = curvyo_laser_core::stop_leg_range().default.as_mm();
            (leg, leg)
        }
    };
    Affine::translation(Vec2::new(
        (size.width.as_mm() - w) / 2.0,
        (size.height.as_mm() - h) / 2.0,
    ))
}
