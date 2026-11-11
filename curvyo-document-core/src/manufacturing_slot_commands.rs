//! The commands that write the slots of a manufacturing's fixtures (ADR 0049 §3, `specs/0102`
//! criteria 19 and 20, `specs/0103` criterion 11), each one history step with its own label.

use loro::{LoroMap, TreeID, TreeParentId};

use crate::document::{Document, OBJECTS_TREE};
use crate::group_tree::node_id;
use crate::manufacturing::{FixtureParams, ManufacturingError, next_numbered};
use crate::manufacturing_codec::{KEY_SLOT_PARAMS, SHAPE_FIXTURE_SLOT, string, write_plain_map};
use crate::manufacturing_fixtures::{
    MAX_SLOTS_PER_FIXTURE, SlotRecord, fixture_name, read_whole_map, write_slot_registers,
};
use crate::node_registers::{KEY_NAME, read_registers};
use crate::path_codec::node_exists;
use crate::path_model::NodeId;
use crate::paths::tree_id_of;
use crate::shape_codec::KEY_SHAPE;

/// The label of a slot's rename.
pub const RENAME_SLOT_COMMIT_LABEL: &str = "rename_slot";
/// The label of a slot's Shape switch.
pub const SET_SLOT_SHAPE_COMMIT_LABEL: &str = "set_slot_shape";
/// The label of a slot's Centre X or Y.
pub const SET_SLOT_POSITION_COMMIT_LABEL: &str = "set_slot_position";
/// The label of a slot's width, height or diameter.
pub const SET_SLOT_SIZE_COMMIT_LABEL: &str = "set_slot_size";
/// The label of Add slot.
pub const ADD_SLOT_COMMIT_LABEL: &str = "add_slot";
/// The label of a slot's removal.
pub const REMOVE_SLOT_COMMIT_LABEL: &str = "remove_slot";
/// The label of a slot field edit.
pub const SET_SLOT_COMMIT_LABEL: &str = "set_slot";

impl Document {
    /// The fixture node that holds the slot `id`.
    pub(crate) fn slot_holder(&self, id: NodeId) -> Option<NodeId> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        match tree.parent(tree_id_of(id)) {
            Some(TreeParentId::Node(parent)) => Some(node_id(parent)),
            _ => None,
        }
    }

    /// Adds a slot "`label` N" with `params` at the end of the fixture `fixture` (criterion 19).
    ///
    /// # Errors
    /// [`ManufacturingError::NoSuchFixture`] when `fixture` is not a live fixture;
    /// [`ManufacturingError::TooManySlots`] when it holds 50 slots already.
    pub fn add_slot(
        &self,
        fixture: NodeId,
        label: &str,
        params: &FixtureParams,
    ) -> Result<NodeId, ManufacturingError> {
        self.fixture_meta(fixture)?;
        let slots = self.fixture(fixture).map(|f| f.slots).unwrap_or_default();
        if slots.len() >= MAX_SLOTS_PER_FIXTURE {
            return Err(ManufacturingError::TooManySlots);
        }
        let taken: Vec<String> = slots.into_iter().map(|slot| slot.name).collect();
        let record = SlotRecord {
            name: next_numbered(label, &taken, false),
            params: params.clone(),
        };
        let node = self.insert_slot(tree_id_of(fixture), &record)?;
        self.commit_with_label(ADD_SLOT_COMMIT_LABEL);
        Ok(node_id(node))
    }

    /// Removes the slot `id` in one step, with the ticks of its fixture's jobs on it
    /// (`specs/0103` criterion 11).
    ///
    /// # Errors
    /// [`ManufacturingError::NoSuchSlot`] when `id` is not a live slot of a fixture.
    pub fn remove_slot(&self, id: NodeId) -> Result<(), ManufacturingError> {
        self.slot_meta(id)?;
        if let Some(holder) = self.slot_holder(id) {
            self.remove_slot_ticks(tree_id_of(holder), tree_id_of(id));
        }
        let tree = self.loro().get_tree(OBJECTS_TREE);
        tree.delete(tree_id_of(id))
            .map_err(|_| ManufacturingError::NoSuchSlot)?;
        crate::groups::remove_emptied_groups(&tree, &[None]);
        self.commit_with_label(REMOVE_SLOT_COMMIT_LABEL);
        Ok(())
    }

    /// Writes the slot's name (cleaned, cut to 40 characters; an empty one keeps the old) and its
    /// `slot_params` whole in one step. `Ok(false)` when both are unchanged.
    ///
    /// # Errors
    /// [`ManufacturingError::NoSuchSlot`] when `id` is not a live slot of a fixture.
    pub fn set_slot(
        &self,
        id: NodeId,
        name: &str,
        params: &FixtureParams,
    ) -> Result<bool, ManufacturingError> {
        self.set_slot_labelled(id, name, params, SET_SLOT_COMMIT_LABEL)
    }

    /// [`Document::set_slot`] as the step `label`: one label per slot field (`rename_slot`,
    /// `set_slot_shape`, `set_slot_position`, `set_slot_size`; `specs/0102` adrs.md §4).
    ///
    /// # Errors
    /// [`ManufacturingError::NoSuchSlot`] when `id` is not a live slot of a fixture.
    pub fn set_slot_labelled(
        &self,
        id: NodeId,
        name: &str,
        params: &FixtureParams,
        label: &str,
    ) -> Result<bool, ManufacturingError> {
        let meta = self.slot_meta(id)?;
        let name = fixture_name(name).filter(|n| read_registers(&meta).name.as_ref() != Some(n));
        let same_params = read_whole_map(&meta, KEY_SLOT_PARAMS).as_ref() == Some(params);
        if name.is_none() && same_params {
            return Ok(false);
        }
        if let Some(name) = name {
            meta.insert(KEY_NAME, name.as_str())
                .map_err(|_| ManufacturingError::NoSuchSlot)?;
        }
        if !same_params {
            write_plain_map(&meta, KEY_SLOT_PARAMS, params);
        }
        self.commit_with_label(label);
        Ok(true)
    }

    /// Creates, uncommitted, a slot node from `record` at the end of the fixture `parent`.
    pub(crate) fn insert_slot(
        &self,
        parent: TreeID,
        record: &SlotRecord,
    ) -> Result<TreeID, ManufacturingError> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        tree.enable_fractional_index(0);
        let node = tree
            .create(TreeParentId::Node(parent))
            .map_err(|_| ManufacturingError::NoSuchFixture)?;
        let meta = tree
            .get_meta(node)
            .map_err(|_| ManufacturingError::NoSuchFixture)?;
        write_slot_registers(&meta, record);
        Ok(node)
    }

    pub(crate) fn slot_meta(&self, id: NodeId) -> Result<LoroMap, ManufacturingError> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let node = tree_id_of(id);
        let parent = match tree.parent(node) {
            Some(TreeParentId::Node(parent)) if node_exists(&tree, node) => parent,
            _ => return Err(ManufacturingError::NoSuchSlot),
        };
        self.fixture_meta(node_id(parent))
            .map_err(|_| ManufacturingError::NoSuchSlot)?;
        let meta = tree
            .get_meta(node)
            .map_err(|_| ManufacturingError::NoSuchSlot)?;
        if string(&meta, KEY_SHAPE).as_deref() != Some(SHAPE_FIXTURE_SLOT) {
            return Err(ManufacturingError::NoSuchSlot);
        }
        Ok(meta)
    }
}
