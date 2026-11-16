//! The stored fixture sets of a manufacturing (ADR 0059 §1, §3): the set nodes and the fixtures in
//! listing order as read field by field, with a count of the values that did not read.

use loro::{LoroMap, LoroTree, TreeID, TreeParentId};
use serde::Serialize;

use crate::group_tree::node_id;
use crate::manufacturing_codec::{
    KEY_SET_LOCKED, KEY_SET_ON_PAGE, KEY_SET_UID, SHAPE_FIXTURE_SET, has_shape, string,
};
use crate::manufacturing_fixtures::{FixtureSnapshot, read_fixtures, read_flag};
use crate::node_registers::{KEY_NAME, read_registers};
use crate::path_model::NodeId;
use crate::shape_codec::KEY_SHAPE;

/// The most sets a page takes from "+ Set" (`0113` criterion 17); a file may hold more, all read.
pub const MAX_FIXTURE_SETS: usize = 16;
/// The most job fixtures a page takes from its add bar (`0113` criterion 17); a file may hold
/// more, all read.
pub const MAX_JOB_FIXTURES: usize = 64;

/// A named machine set of a laser page as read (ADR 0059 §1). Its fixtures are those of the
/// manufacturing's `fixtures` whose `set` is this set's id.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FixtureSetSnapshot {
    /// Its node.
    pub id: NodeId,
    /// Its name; a missing name reads "Set n" (n its 1-based place) and counts one damaged value.
    pub name: String,
    /// Its On this page check (absent reads on, a value that is not a boolean reads on and counts).
    pub on_page: bool,
    /// The record set's id (`set_uid`, ADR 0059 §1 amended 2026-11-16); `None` when absent or
    /// not a text, which counts one damaged value.
    pub uid: Option<String>,
    /// Its Lock (`set_locked`, absent reads off, a value that is not a boolean reads off and
    /// counts): it locks every fixture in it.
    pub locked: bool,
    /// How many of the set's own values did not read (its fixtures count their own).
    pub damaged: usize,
}

/// The sets among `children` of a manufacturing and every fixture in listing order: the sets in
/// child order, each with its fixtures in child order, then the job fixtures (ADR 0059 §3). A
/// child of a set that is not a fixture is kept and never read, as is a second set with the
/// profile id of an earlier one (ADR 0059 §1 amended 2026-11-16).
pub(crate) fn read_sets_and_fixtures(
    tree: &LoroTree,
    children: &[TreeID],
) -> (Vec<FixtureSetSnapshot>, Vec<FixtureSnapshot>) {
    let mut sets: Vec<FixtureSetSnapshot> = Vec::new();
    let mut fixtures = Vec::new();
    for &child in children {
        let Some(meta) = tree
            .get_meta(child)
            .ok()
            .filter(|meta| has_shape(meta, SHAPE_FIXTURE_SET))
        else {
            continue;
        };
        let set = read_set(child, &meta, sets.len() + 1);
        if set.uid.is_some() && sets.iter().any(|earlier| earlier.uid == set.uid) {
            continue;
        }
        sets.push(set);
        let members = tree.children(TreeParentId::Node(child)).unwrap_or_default();
        fixtures.extend(read_fixtures(tree, &members, Some(child)));
    }
    fixtures.extend(read_fixtures(tree, children, None));
    (sets, fixtures)
}

fn read_set(node: TreeID, meta: &LoroMap, place: usize) -> FixtureSetSnapshot {
    let (on_page, on_page_damage) = read_flag(meta, KEY_SET_ON_PAGE, true);
    let (locked, locked_damage) = read_flag(meta, KEY_SET_LOCKED, false);
    let name = read_registers(meta).name.filter(|name| !name.is_empty());
    let uid = string(meta, KEY_SET_UID).filter(|uid| !uid.is_empty());
    FixtureSetSnapshot {
        id: node_id(node),
        damaged: on_page_damage
            + locked_damage
            + usize::from(name.is_none())
            + usize::from(uid.is_none()),
        name: name.unwrap_or_else(|| format!("Set {place}")),
        on_page,
        uid,
        locked,
    }
}

/// What a new set node holds: its profile id (none when empty), name, Lock and On this page.
pub(crate) struct NewSetRegisters<'a> {
    pub(crate) uid: &'a str,
    pub(crate) name: &'a str,
    pub(crate) locked: bool,
    pub(crate) on_page: bool,
}

/// Writes a new set node's registers: its shape, profile id, name, Lock (when on) and On this
/// page.
pub(crate) fn write_set_registers(meta: &LoroMap, new: &NewSetRegisters<'_>) {
    // invariant: plain values under known keys on an attached map cannot fail.
    #[allow(clippy::unwrap_used)]
    {
        meta.insert(KEY_SHAPE, SHAPE_FIXTURE_SET).unwrap();
        if !new.uid.is_empty() {
            meta.insert(KEY_SET_UID, new.uid).unwrap();
        }
        meta.insert(KEY_NAME, new.name).unwrap();
        if new.locked {
            meta.insert(KEY_SET_LOCKED, true).unwrap();
        }
        meta.insert(KEY_SET_ON_PAGE, new.on_page).unwrap();
    }
}
