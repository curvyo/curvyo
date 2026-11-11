//! The commands that write a manufacturing's fixtures (ADR 0049, `specs/0102` criteria 2 to 5,
//! 14, 15, 18, 23 and 37 to 39), each one history step with its own label; the slots' commands
//! are in `manufacturing_slot_commands`.

use loro::{LoroMap, TreeID, TreeParentId};

use crate::affine::Affine;
use crate::document::{Document, OBJECTS_TREE};
use crate::fixture_job::insert_job;
use crate::fixture_job_codec::FixturePartKey;
use crate::group_tree::node_id;
use crate::manufacturing::{FixtureParams, ManufacturingError, next_numbered};
use crate::manufacturing_codec::{
    KEY_FIXTURE_ACTIVE, KEY_FIXTURE_PARAMS, KEY_SLOT_PARAMS, SHAPE_FIXTURE, is_manufacturing_node,
    string, write_plain_map,
};
use crate::manufacturing_fixtures::{
    FixtureRecord, fixture_name, read_active, read_whole_map, write_fixture_registers,
    write_matrix, write_visible,
};
use crate::node_registers::{KEY_NAME, read_registers};
use crate::path_codec::node_exists;
use crate::path_model::NodeId;
use crate::paths::tree_id_of;
use crate::shape_codec::KEY_SHAPE;

/// The label of Add grid.
pub const ADD_GRID_COMMIT_LABEL: &str = "add_grid";
/// The label of Add holder.
pub const ADD_HOLDER_COMMIT_LABEL: &str = "add_holder";
/// The label of Add clamp.
pub const ADD_CLAMP_COMMIT_LABEL: &str = "add_clamp";
/// The label of Add stop.
pub const ADD_STOP_COMMIT_LABEL: &str = "add_stop";
/// The label of Add screw.
pub const ADD_SCREW_COMMIT_LABEL: &str = "add_screw";
/// The label of Remove fixture (with its slots).
pub const DELETE_FIXTURE_COMMIT_LABEL: &str = "delete_fixture";
/// The label of Rename fixture.
pub const RENAME_FIXTURE_COMMIT_LABEL: &str = "rename_fixture";
/// The label of a fixture's hide or show (the eye).
pub const SET_FIXTURE_VISIBLE_COMMIT_LABEL: &str = "set_fixture_visible";
/// The label of switching a fixture's Active on.
pub const SET_FIXTURE_ACTIVE_COMMIT_LABEL: &str = "set_fixture_active";
/// The label of switching a fixture's Active off.
pub const SET_FIXTURE_INACTIVE_COMMIT_LABEL: &str = "set_fixture_inactive";
/// The label of a fixture parameter edit.
pub const SET_FIXTURE_PARAMS_COMMIT_LABEL: &str = "set_fixture_params";
/// The label of the grid's Spacing field.
pub const SET_GRID_SPACING_COMMIT_LABEL: &str = "set_grid_spacing";
/// The label of the grid's Major every field.
pub const SET_GRID_MAJOR_COMMIT_LABEL: &str = "set_grid_major";
/// The label of a grid tape switch.
pub const SET_TAPE_EDGE_COMMIT_LABEL: &str = "set_tape_edge";
/// The label of the grid's Origin X or Y field.
pub const SET_GRID_ORIGIN_COMMIT_LABEL: &str = "set_grid_origin";
/// The label of a fixture's move.
pub const MOVE_FIXTURE_COMMIT_LABEL: &str = "move_fixture";
/// The label of a fixture's turn.
pub const ROTATE_FIXTURE_COMMIT_LABEL: &str = "rotate_fixture";
/// The label of a fixture's resize by a handle or a chip: its sizes, its matrix and its slots'
/// centres in one step.
pub const RESIZE_FIXTURE_COMMIT_LABEL: &str = "resize_fixture";

/// Which Add button made the fixture: the history name is fixed per label (ADR 0049 §9).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixtureAdd {
    /// Add grid.
    Grid,
    /// Add holder.
    Holder,
    /// Add clamp.
    Clamp,
    /// Add stop.
    Stop,
    /// Add screw.
    Screw,
}

impl FixtureAdd {
    /// The commit label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Grid => ADD_GRID_COMMIT_LABEL,
            Self::Holder => ADD_HOLDER_COMMIT_LABEL,
            Self::Clamp => ADD_CLAMP_COMMIT_LABEL,
            Self::Stop => ADD_STOP_COMMIT_LABEL,
            Self::Screw => ADD_SCREW_COMMIT_LABEL,
        }
    }
}

/// Whether a new matrix is a move or a turn: two undo names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixtureTurn {
    /// A move (drag, nudge, typed X and Y).
    Move,
    /// A turn (rotate handle).
    Rotate,
}

/// What a resize writes in one step: the fixture's whole parameter values, its matrix, and the
/// slots whose centre moved because the board's frame origin did (`specs/0102` adrs.md §6a.5).
#[derive(Debug, Clone, Copy)]
pub struct FixtureResize<'a> {
    /// The parameter values, written whole.
    pub params: &'a FixtureParams,
    /// The new matrix; `None` keeps the stored one.
    pub matrix: Option<Affine>,
    /// Slots and their new `slot_params`, written whole.
    pub slots: &'a [(NodeId, FixtureParams)],
}

/// The keys of a screw's two diameters (ADR 0049 §2).
const SCREW_THREAD_KEY: &str = "thread_diameter_mm";
const SCREW_HEAD_KEY: &str = "head_diameter_mm";

/// `params` with a screw's thread diameter set to its head where it is larger, so no writer stores
/// a thread above its head (adrs.md §6a.6). Other values are kept as they are.
fn with_thread_within_head(params: &FixtureParams) -> std::borrow::Cow<'_, FixtureParams> {
    let number = |key: &str| params.get(key).and_then(serde_json::Value::as_f64);
    match (number(SCREW_THREAD_KEY), number(SCREW_HEAD_KEY)) {
        (Some(thread), Some(head)) if thread > head => {
            let mut clamped = params.clone();
            clamped.insert(SCREW_THREAD_KEY.to_owned(), serde_json::Value::from(head));
            std::borrow::Cow::Owned(clamped)
        }
        _ => std::borrow::Cow::Borrowed(params),
    }
}

/// What Add fixture writes.
#[derive(Debug, Clone, Copy)]
pub struct NewFixture<'a> {
    /// The button, which names the step.
    pub add: FixtureAdd,
    /// The stored kind, opaque here (`curvyo.laser/holder`).
    pub kind: &'a str,
    /// The word the name starts with ("Holder"): the fixture is "Holder N".
    pub label: &'a str,
    /// The parameter values.
    pub params: &'a FixtureParams,
    /// The matrix into the page; `None` for the grid, which has none.
    pub matrix: Option<Affine>,
}

impl Document {
    /// Adds a fixture "`label` N" (N the smallest number no fixture of the same kind on this page
    /// uses), Active, at the end of the setup, in one step labelled by its button (criterion 2).
    ///
    /// # Errors
    /// [`ManufacturingError::NotFound`] for an unknown manufacturing;
    /// [`ManufacturingError::OutOfRange`] for a matrix that is not finite or passes ±1e7.
    pub fn add_fixture(
        &self,
        manufacturing: NodeId,
        new: &NewFixture<'_>,
    ) -> Result<NodeId, ManufacturingError> {
        let snapshot = self
            .manufacturing(manufacturing)
            .ok_or(ManufacturingError::NotFound)?;
        if new
            .matrix
            .is_some_and(|m| !m.is_finite() || !m.is_within_limit())
        {
            return Err(ManufacturingError::OutOfRange);
        }
        let taken: Vec<String> = snapshot
            .fixtures
            .into_iter()
            .filter(|fixture| fixture.kind == new.kind)
            .map(|fixture| fixture.name)
            .collect();
        let record = FixtureRecord {
            kind: new.kind.to_owned(),
            name: next_numbered(new.label, &taken, false),
            active: true,
            params: with_thread_within_head(new.params).into_owned(),
            matrix: new.matrix,
            slots: Vec::new(),
            jobs: Vec::new(),
        };
        let node = self.insert_fixture(tree_id_of(manufacturing), &record)?;
        self.commit_with_label(new.add.label());
        Ok(node_id(node))
    }

    /// Removes the fixture `id` with its slots in one step; undo restores both (criterion 4).
    ///
    /// # Errors
    /// [`ManufacturingError::NoSuchFixture`] when `id` is not a live fixture.
    pub fn delete_fixture(&self, id: NodeId) -> Result<(), ManufacturingError> {
        self.fixture_meta(id)?;
        let tree = self.loro().get_tree(OBJECTS_TREE);
        tree.delete(tree_id_of(id))
            .map_err(|_| ManufacturingError::NoSuchFixture)?;
        // The rule of `specs/0023-groups`: a fixture holds no group, so this finds nothing.
        crate::groups::remove_emptied_groups(&tree, &[None]);
        self.commit_with_label(DELETE_FIXTURE_COMMIT_LABEL);
        Ok(())
    }

    /// Renames the fixture `id`, cleaned and cut to 40 characters (criterion 3). `Ok(false)` when
    /// the name is unchanged or nothing is left of it.
    ///
    /// # Errors
    /// [`ManufacturingError::NoSuchFixture`] when `id` is not a live fixture.
    pub fn rename_fixture(&self, id: NodeId, name: &str) -> Result<bool, ManufacturingError> {
        let meta = self.fixture_meta(id)?;
        let Some(name) = fixture_name(name) else {
            return Ok(false);
        };
        if read_registers(&meta).name.as_deref() == Some(name.as_str()) {
            return Ok(false);
        }
        meta.insert(KEY_NAME, name.as_str())
            .map_err(|_| ManufacturingError::NoSuchFixture)?;
        self.commit_with_label(RENAME_FIXTURE_COMMIT_LABEL);
        Ok(true)
    }

    /// Hides or shows the fixture `id` through its eye, the `visible` register (criterion 6).
    /// `Ok(false)` when it already is.
    ///
    /// # Errors
    /// [`ManufacturingError::NoSuchFixture`] when `id` is not a live fixture.
    pub fn set_fixture_visible(
        &self,
        id: NodeId,
        visible: bool,
    ) -> Result<bool, ManufacturingError> {
        let meta = self.fixture_meta(id)?;
        if !write_visible(&meta, visible) {
            return Ok(false);
        }
        self.commit_with_label(SET_FIXTURE_VISIBLE_COMMIT_LABEL);
        Ok(true)
    }

    /// Switches the fixture `id` on or off the bed, the `active` register (criterion 5).
    /// `Ok(false)` when it already is.
    ///
    /// # Errors
    /// [`ManufacturingError::NoSuchFixture`] when `id` is not a live fixture.
    pub fn set_fixture_active(&self, id: NodeId, active: bool) -> Result<bool, ManufacturingError> {
        let meta = self.fixture_meta(id)?;
        if read_active(&meta) == active {
            return Ok(false);
        }
        meta.insert(KEY_FIXTURE_ACTIVE, active)
            .map_err(|_| ManufacturingError::NoSuchFixture)?;
        self.commit_with_label(if active {
            SET_FIXTURE_ACTIVE_COMMIT_LABEL
        } else {
            SET_FIXTURE_INACTIVE_COMMIT_LABEL
        });
        Ok(true)
    }

    /// Writes the fixture's parameter values whole, so a merge never pairs one peer's spacing with
    /// another's origin (ADR 0033 §2). `Ok(false)` when they are unchanged.
    ///
    /// # Errors
    /// [`ManufacturingError::NoSuchFixture`] when `id` is not a live fixture.
    pub fn set_fixture_params(
        &self,
        id: NodeId,
        params: &FixtureParams,
    ) -> Result<bool, ManufacturingError> {
        self.set_fixture_params_labelled(id, params, SET_FIXTURE_PARAMS_COMMIT_LABEL)
    }

    /// [`Document::set_fixture_params`] as the step `label`: one label per field of the grid
    /// (`specs/0102` adrs.md §4, `set_grid_spacing` ...).
    ///
    /// # Errors
    /// [`ManufacturingError::NoSuchFixture`] when `id` is not a live fixture.
    pub fn set_fixture_params_labelled(
        &self,
        id: NodeId,
        params: &FixtureParams,
        label: &str,
    ) -> Result<bool, ManufacturingError> {
        let meta = self.fixture_meta(id)?;
        let params = with_thread_within_head(params);
        if read_whole_map(&meta, KEY_FIXTURE_PARAMS).as_ref() == Some(&*params) {
            return Ok(false);
        }
        write_plain_map(&meta, KEY_FIXTURE_PARAMS, &params);
        self.commit_with_label(label);
        Ok(true)
    }

    /// Resizes the fixture `id`: its parameter values, its matrix and the given slots' values, in
    /// one step "Resize fixture" (criteria 18, 22, 23, 38; adrs.md §6a.5). A screw's thread is
    /// set to its head where it is larger. `Ok(false)` when nothing changes.
    ///
    /// # Errors
    /// [`ManufacturingError::NoSuchFixture`] when `id` is not a live fixture;
    /// [`ManufacturingError::NoSuchSlot`] when a slot is not one of its live slots;
    /// [`ManufacturingError::OutOfRange`] for a matrix that is not finite or passes ±1e7.
    pub fn resize_fixture(
        &self,
        id: NodeId,
        resize: &FixtureResize<'_>,
    ) -> Result<bool, ManufacturingError> {
        let meta = self.fixture_meta(id)?;
        if resize
            .matrix
            .is_some_and(|m| !m.is_finite() || !m.is_within_limit())
        {
            return Err(ManufacturingError::OutOfRange);
        }
        let mut slot_metas = Vec::with_capacity(resize.slots.len());
        for (slot, params) in resize.slots {
            let slot_meta = self.slot_meta(*slot)?;
            if self.slot_holder(*slot) != Some(id) {
                return Err(ManufacturingError::NoSuchSlot);
            }
            if read_whole_map(&slot_meta, KEY_SLOT_PARAMS).as_ref() != Some(params) {
                slot_metas.push((slot_meta, params));
            }
        }
        let params = with_thread_within_head(resize.params);
        let new_params = read_whole_map(&meta, KEY_FIXTURE_PARAMS).as_ref() != Some(&*params);
        let stored = self.fixture(id).and_then(|fixture| fixture.matrix);
        let new_matrix = resize.matrix.filter(|m| stored != Some(*m));
        if !new_params && new_matrix.is_none() && slot_metas.is_empty() {
            return Ok(false);
        }
        if new_params {
            write_plain_map(&meta, KEY_FIXTURE_PARAMS, &params);
        }
        if let Some(matrix) = new_matrix {
            write_matrix(&meta, matrix);
        }
        for (slot_meta, params) in slot_metas {
            write_plain_map(&slot_meta, KEY_SLOT_PARAMS, params);
        }
        self.commit_with_label(RESIZE_FIXTURE_COMMIT_LABEL);
        Ok(true)
    }

    /// Moves or turns the fixture `id`: its `fixture_matrix` register alone, so a move never races
    /// a size edit (criterion 14, ADR 0049 §2). `Ok(false)` when the matrix is unchanged.
    ///
    /// # Errors
    /// [`ManufacturingError::NoSuchFixture`] when `id` is not a live fixture;
    /// [`ManufacturingError::OutOfRange`] for a matrix that is not finite or passes ±1e7.
    pub fn set_fixture_matrix(
        &self,
        id: NodeId,
        matrix: Affine,
        turn: FixtureTurn,
    ) -> Result<bool, ManufacturingError> {
        let meta = self.fixture_meta(id)?;
        if !matrix.is_finite() || !matrix.is_within_limit() {
            return Err(ManufacturingError::OutOfRange);
        }
        let stored = self.fixture(id).and_then(|fixture| fixture.matrix);
        if stored == Some(matrix) {
            return Ok(false);
        }
        write_matrix(&meta, matrix);
        self.commit_with_label(match turn {
            FixtureTurn::Move => MOVE_FIXTURE_COMMIT_LABEL,
            FixtureTurn::Rotate => ROTATE_FIXTURE_COMMIT_LABEL,
        });
        Ok(true)
    }

    /// Replaces, uncommitted, every fixture of the manufacturing `manufacturing` by one new node
    /// per record, in order, with fresh ids: the fixture part of Pick and Reset from profile
    /// (ADR 0050 §4). Orphan slots are left as they are.
    pub(crate) fn replace_fixtures(
        &self,
        manufacturing: NodeId,
        records: &[FixtureRecord],
    ) -> Result<(), ManufacturingError> {
        let snapshot = self
            .manufacturing(manufacturing)
            .ok_or(ManufacturingError::NotFound)?;
        let tree = self.loro().get_tree(OBJECTS_TREE);
        for fixture in &snapshot.fixtures {
            tree.delete(tree_id_of(fixture.id))
                .map_err(|_| ManufacturingError::NoSuchFixture)?;
        }
        crate::groups::remove_emptied_groups(&tree, &[None]);
        for record in records {
            self.insert_fixture(tree_id_of(manufacturing), record)?;
        }
        Ok(())
    }

    /// Creates, uncommitted, a fixture node from `record` with its slots and its jobs (Output
    /// off, ticks mapped to the new slots by index, ADR 0052 §7) at the end of `parent`.
    fn insert_fixture(
        &self,
        parent: TreeID,
        record: &FixtureRecord,
    ) -> Result<TreeID, ManufacturingError> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        tree.enable_fractional_index(0);
        let node = tree
            .create(TreeParentId::Node(parent))
            .map_err(|_| ManufacturingError::NotFound)?;
        let meta = tree
            .get_meta(node)
            .map_err(|_| ManufacturingError::NotFound)?;
        write_fixture_registers(&meta, record);
        let mut slots = Vec::with_capacity(record.slots.len());
        for slot in &record.slots {
            slots.push(self.insert_slot(node, slot)?);
        }
        for job in &record.jobs {
            let ticks: Vec<FixturePartKey> = job
                .targets
                .iter()
                .filter_map(|target| {
                    let part = match target.slot {
                        None => node,
                        Some(index) => *slots.get(index)?,
                    };
                    Some(FixturePartKey {
                        node: node_id(part),
                        suffix: target.suffix.clone(),
                    })
                })
                .collect();
            insert_job(&tree, node, job, &ticks).map_err(|_| ManufacturingError::NotFound)?;
        }
        Ok(node)
    }

    /// The fixture `id` as read, `None` when it is not a live fixture.
    #[must_use]
    pub fn fixture(&self, id: NodeId) -> Option<crate::FixtureSnapshot> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let Some(TreeParentId::Node(parent)) = tree.parent(tree_id_of(id)) else {
            return None;
        };
        self.manufacturing(node_id(parent))?
            .fixtures
            .into_iter()
            .find(|fixture| fixture.id == id)
    }

    pub(crate) fn fixture_meta(&self, id: NodeId) -> Result<LoroMap, ManufacturingError> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let node = tree_id_of(id);
        let parent = match tree.parent(node) {
            Some(TreeParentId::Node(parent)) if node_exists(&tree, node) => parent,
            _ => return Err(ManufacturingError::NoSuchFixture),
        };
        let meta = tree
            .get_meta(node)
            .map_err(|_| ManufacturingError::NoSuchFixture)?;
        if !is_manufacturing_node(&tree, parent)
            || string(&meta, KEY_SHAPE).as_deref() != Some(SHAPE_FIXTURE)
        {
            return Err(ManufacturingError::NoSuchFixture);
        }
        Ok(meta)
    }
}
