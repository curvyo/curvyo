//! The commands that write the named fixture sets of a laser page (ADR 0059 §1, §6; `specs/0113`
//! criteria 3 to 9), each one history step with its own label.

use loro::{LoroMap, TreeID, TreeParentId};

use crate::document::{Document, OBJECTS_TREE};
use crate::group_tree::node_id;
use crate::manufacturing::{ManufacturingError, next_numbered};
use crate::manufacturing_codec::{
    KEY_SET_LOCKED, KEY_SET_ON_PAGE, SHAPE_FIXTURE_SET, has_shape, set_manufacturing,
};
use crate::manufacturing_fixture_commands::NewFixture;
use crate::manufacturing_fixture_sets::{MAX_FIXTURE_SETS, NewSetRegisters, write_set_registers};
use crate::manufacturing_fixtures::{FixtureSetRecord, fixture_name, read_flag};
use crate::node_registers::{KEY_NAME, read_registers};
use crate::path_model::NodeId;
use crate::paths::tree_id_of;

/// The label of "+ Set".
pub const ADD_FIXTURE_SET_COMMIT_LABEL: &str = "add_fixture_set";
/// The label of a set's rename.
pub const RENAME_FIXTURE_SET_COMMIT_LABEL: &str = "rename_fixture_set";
/// The label of a set's removal with its fixtures.
pub const REMOVE_FIXTURE_SET_COMMIT_LABEL: &str = "remove_fixture_set";
/// The label of a set's reorder.
pub const MOVE_FIXTURE_SET_COMMIT_LABEL: &str = "move_fixture_set";
/// The label of a set's On this page switched on.
pub const SET_FIXTURE_SET_ON_PAGE_COMMIT_LABEL: &str = "set_fixture_set_on_page";
/// The label of a set's On this page switched off.
pub const SET_FIXTURE_SET_OFF_PAGE_COMMIT_LABEL: &str = "set_fixture_set_off_page";
/// The label of a fixture added into a set (`0113` UX M1).
pub const ADD_FIXTURE_COMMIT_LABEL: &str = "add_fixture";
/// The label of a set's Lock switched on (`0113` criterion 32, amended 2026-11-16).
pub const LOCK_FIXTURE_SET_COMMIT_LABEL: &str = "lock_fixture_set";
/// The label of a set's Lock switched off.
pub const UNLOCK_FIXTURE_SET_COMMIT_LABEL: &str = "unlock_fixture_set";

/// The word a new set's name starts with ("Set 1").
const SET_LABEL: &str = "Set";

impl Document {
    /// The set nodes of the manufacturing `manufacturing`, in child order.
    pub(crate) fn set_nodes(&self, manufacturing: TreeID) -> Vec<TreeID> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        tree.children(TreeParentId::Node(manufacturing))
            .unwrap_or_default()
            .into_iter()
            .filter(|&child| {
                tree.get_meta(child)
                    .is_ok_and(|meta| has_shape(&meta, SHAPE_FIXTURE_SET))
            })
            .collect()
    }

    /// The set `id` with its meta map and its manufacturing.
    fn set_meta(&self, id: NodeId) -> Result<(LoroMap, TreeID), ManufacturingError> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let node = tree_id_of(id);
        let manufacturing = set_manufacturing(&tree, node).ok_or(ManufacturingError::NoSuchSet)?;
        let meta = tree
            .get_meta(node)
            .map_err(|_| ManufacturingError::NoSuchSet)?;
        Ok((meta, manufacturing))
    }

    /// Creates, uncommitted, a set node holding `new` after the manufacturing's last set (sets
    /// come first among its children, ADR 0059 §1).
    pub(crate) fn insert_set(
        &self,
        manufacturing: TreeID,
        new: &NewSetRegisters<'_>,
    ) -> Result<TreeID, ManufacturingError> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        tree.enable_fractional_index(0);
        let children = tree
            .children(TreeParentId::Node(manufacturing))
            .unwrap_or_default();
        let at = self
            .set_nodes(manufacturing)
            .last()
            .and_then(|last| children.iter().position(|child| child == last))
            .map_or(0, |index| index + 1);
        let node = tree
            .create_at(TreeParentId::Node(manufacturing), at)
            .map_err(|_| ManufacturingError::NotFound)?;
        let meta = tree
            .get_meta(node)
            .map_err(|_| ManufacturingError::NotFound)?;
        write_set_registers(&meta, new);
        Ok(node)
    }

    /// Adds an empty unlocked set "Set N" (N the lowest number no set of the page uses) with the
    /// profile id `uid` (the caller makes it, ADR 0059 §6 amended 2026-11-16), On this page,
    /// after the page's sets, in one step "Add fixture set" (criterion 3).
    ///
    /// # Errors
    /// [`ManufacturingError::NotFound`] for an unknown manufacturing;
    /// [`ManufacturingError::TooManySets`] when the page holds 16 sets already.
    pub fn add_fixture_set(
        &self,
        manufacturing: NodeId,
        uid: &str,
    ) -> Result<NodeId, ManufacturingError> {
        let snapshot = self
            .manufacturing(manufacturing)
            .ok_or(ManufacturingError::NotFound)?;
        if snapshot.sets.len() >= MAX_FIXTURE_SETS {
            return Err(ManufacturingError::TooManySets);
        }
        let taken: Vec<String> = snapshot.sets.into_iter().map(|set| set.name).collect();
        let name = next_numbered(SET_LABEL, &taken, false);
        let new = NewSetRegisters {
            uid,
            name: &name,
            locked: false,
            on_page: true,
        };
        let node = self.insert_set(tree_id_of(manufacturing), &new)?;
        self.commit_with_label(ADD_FIXTURE_SET_COMMIT_LABEL);
        Ok(node_id(node))
    }

    /// Renames the set `id`, cleaned and cut to 40 characters; names need not be unique
    /// (criterion 4). `Ok(false)` when the name is unchanged or nothing is left of it.
    ///
    /// # Errors
    /// [`ManufacturingError::NoSuchSet`] when `id` is not a live set.
    pub fn rename_fixture_set(&self, id: NodeId, name: &str) -> Result<bool, ManufacturingError> {
        let (meta, _) = self.set_meta(id)?;
        let Some(name) = fixture_name(name) else {
            return Ok(false);
        };
        if read_registers(&meta).name.as_deref() == Some(name.as_str()) {
            return Ok(false);
        }
        meta.insert(KEY_NAME, name.as_str())
            .map_err(|_| ManufacturingError::NoSuchSet)?;
        self.commit_with_label(RENAME_FIXTURE_SET_COMMIT_LABEL);
        Ok(true)
    }

    /// Removes the set `id` with its fixtures, their slots and jobs in one tree delete, one step;
    /// undo restores them exactly (criterion 5).
    ///
    /// # Errors
    /// [`ManufacturingError::NoSuchSet`] when `id` is not a live set.
    pub fn remove_fixture_set(&self, id: NodeId) -> Result<(), ManufacturingError> {
        self.set_meta(id)?;
        let tree = self.loro().get_tree(OBJECTS_TREE);
        tree.delete(tree_id_of(id))
            .map_err(|_| ManufacturingError::NoSuchSet)?;
        // The rule of `specs/0023-groups`: a set holds no group, so this finds nothing.
        crate::groups::remove_emptied_groups(&tree, &[None]);
        self.commit_with_label(REMOVE_FIXTURE_SET_COMMIT_LABEL);
        Ok(())
    }

    /// Moves the set `id` to `index` among the page's sets (or last), one tree move in one step;
    /// its fixtures move with it (criterion 6). `Ok(false)` when it already sits there.
    ///
    /// # Errors
    /// [`ManufacturingError::NoSuchSet`] when `id` is not a live set.
    pub fn move_fixture_set(&self, id: NodeId, index: usize) -> Result<bool, ManufacturingError> {
        let (_, manufacturing) = self.set_meta(id)?;
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let node = tree_id_of(id);
        let list = self.set_nodes(manufacturing);
        let others: Vec<TreeID> = list.iter().copied().filter(|&n| n != node).collect();
        if list.iter().position(|&n| n == node) == Some(index.min(others.len())) {
            return Ok(false);
        }
        tree.enable_fractional_index(0);
        let moved = match (others.get(index), others.last()) {
            (Some(&before), _) => tree.mov_before(node, before),
            (None, Some(&after)) => tree.mov_after(node, after),
            (None, None) => Ok(()),
        };
        moved.map_err(|_| ManufacturingError::NoSuchSet)?;
        self.commit_with_label(MOVE_FIXTURE_SET_COMMIT_LABEL);
        Ok(true)
    }

    /// Switches the set `id` on or off this page (criteria 2, 8). `Ok(false)` when it already
    /// reads so.
    ///
    /// # Errors
    /// [`ManufacturingError::NoSuchSet`] when `id` is not a live set.
    pub fn set_fixture_set_on_page(
        &self,
        id: NodeId,
        on_page: bool,
    ) -> Result<bool, ManufacturingError> {
        let (meta, _) = self.set_meta(id)?;
        let (now, damage) = read_flag(&meta, KEY_SET_ON_PAGE, true);
        if now == on_page && damage == 0 {
            return Ok(false);
        }
        meta.insert(KEY_SET_ON_PAGE, on_page)
            .map_err(|_| ManufacturingError::NoSuchSet)?;
        self.commit_with_label(if on_page {
            SET_FIXTURE_SET_ON_PAGE_COMMIT_LABEL
        } else {
            SET_FIXTURE_SET_OFF_PAGE_COMMIT_LABEL
        });
        Ok(true)
    }

    /// Locks or unlocks the set `id` (`set_locked`): a locked set locks every fixture in it
    /// (criterion 32). One step "Lock fixture set" or "Unlock fixture set"; `Ok(false)` when it
    /// already reads so.
    ///
    /// # Errors
    /// [`ManufacturingError::NoSuchSet`] when `id` is not a live set.
    pub fn set_fixture_set_locked(
        &self,
        id: NodeId,
        locked: bool,
    ) -> Result<bool, ManufacturingError> {
        let (meta, _) = self.set_meta(id)?;
        let (now, damage) = read_flag(&meta, KEY_SET_LOCKED, false);
        if now == locked && damage == 0 {
            return Ok(false);
        }
        meta.insert(KEY_SET_LOCKED, locked)
            .map_err(|_| ManufacturingError::NoSuchSet)?;
        self.commit_with_label(if locked {
            LOCK_FIXTURE_SET_COMMIT_LABEL
        } else {
            UNLOCK_FIXTURE_SET_COMMIT_LABEL
        });
        Ok(true)
    }

    /// Adds a fixture "`label` N" (N the smallest number no fixture of the same kind on this page
    /// uses) into the set `set`, Active, at the end of the set, in one step "Add fixture"; a pin
    /// or a screw gets its two role jobs (criteria 7, 27).
    ///
    /// # Errors
    /// [`ManufacturingError::NoSuchSet`] when `set` is not a live set;
    /// [`ManufacturingError::OutOfRange`] for a matrix that is not finite or passes ±1e7.
    pub fn add_set_fixture(
        &self,
        set: NodeId,
        new: &NewFixture<'_>,
    ) -> Result<NodeId, ManufacturingError> {
        let (_, manufacturing) = self.set_meta(set)?;
        let snapshot = self
            .manufacturing(node_id(manufacturing))
            .ok_or(ManufacturingError::NotFound)?;
        let taken: Vec<String> = snapshot
            .fixtures
            .into_iter()
            .filter(|fixture| fixture.kind == new.kind)
            .map(|fixture| fixture.name)
            .collect();
        let record = new.record(next_numbered(new.label, &taken, false))?;
        let node = self.insert_fixture(tree_id_of(set), &record)?;
        self.commit_with_label(ADD_FIXTURE_COMMIT_LABEL);
        Ok(node_id(node))
    }

    /// Replaces, uncommitted, every set of the manufacturing `manufacturing` by a fresh mirror of
    /// `records`: one new set node per record, in order, with the record's ids and locks, every
    /// set On this page, every fixture On the bed and every job Output off, with fresh node ids:
    /// the set part of Pick profile (ADR 0059 §6 amended 2026-11-16). No record (Generic laser)
    /// deletes the mirror. Job fixtures and every other child are left as they are.
    pub(crate) fn replace_sets(
        &self,
        manufacturing: NodeId,
        records: &[FixtureSetRecord],
    ) -> Result<(), ManufacturingError> {
        let parent = tree_id_of(manufacturing);
        let tree = self.loro().get_tree(OBJECTS_TREE);
        for set in self.set_nodes(parent) {
            tree.delete(set)
                .map_err(|_| ManufacturingError::NoSuchSet)?;
        }
        crate::groups::remove_emptied_groups(&tree, &[None]);
        for record in records {
            let new = NewSetRegisters {
                uid: &record.id,
                name: &record.name,
                locked: record.locked,
                on_page: true,
            };
            let set = self.insert_set(parent, &new)?;
            for fixture in &record.fixtures {
                self.insert_fixture(set, fixture)?;
            }
        }
        Ok(())
    }
}
