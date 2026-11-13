//! Writes the derived operations of a test grid (ADR 0057 §2, §3): their registers, their place
//! in the list and the sweep of those whose grid is gone.

use std::collections::HashMap;

use loro::{LoroMap, LoroTree, TreeID, TreeParentId};

use crate::manufacturing_codec::write_plain_map;
use crate::node_registers::KEY_NAME;
use crate::operation::operation_nodes;
use crate::operation_codec::{
    KEY_OP_GRID_PART, KEY_OP_KIND, KEY_OP_OPTIONS, KEY_OP_OUTPUT, KEY_OP_SETTINGS, KEY_OP_SHOW,
    MEMBER_PREFIX, SHAPE_OPERATION, member_key,
};
use crate::shape_codec::KEY_SHAPE;
use crate::test_grid_codec::{
    KEY_GRID_CELL, KEY_GRID_LABEL, descendants, encode_grid, grid_part_of, plain,
};
use crate::test_grid_commands::{TestGridError, insert};

type Plain = serde_json::Map<String, serde_json::Value>;

/// The cell items of a grid by `{ column, row }` and its label items, in tree order.
pub(crate) type GridItemNodes = (HashMap<(u8, u8), TreeID>, Vec<TreeID>);

/// The part of a grid a derived operation runs (`op_grid_part` without its `grid`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DerivedPart {
    /// Every label item.
    Labels,
    /// The cell item tagged `{ column, row }`.
    Cell {
        /// Its column, from 0.
        column: u8,
        /// Its row, from 0.
        row: u8,
    },
}

impl DerivedPart {
    /// The `op_grid_part` map for the group `grid`.
    fn to_plain(self, grid: TreeID) -> Plain {
        let mut map = Plain::new();
        map.insert("grid".to_owned(), encode_grid(grid).into());
        match self {
            Self::Labels => {
                map.insert("part".to_owned(), "labels".into());
            }
            Self::Cell { column, row } => {
                map.insert("part".to_owned(), "cell".into());
                map.insert("column".to_owned(), column.into());
                map.insert("row".to_owned(), row.into());
            }
        }
        map
    }
}

/// One derived operation as the laser extension resolved it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DerivedRow<'a> {
    /// What it runs; its members follow from it.
    pub part: DerivedPart,
    /// Its kind text.
    pub kind: &'a str,
    /// Its name.
    pub name: &'a str,
    /// Its `op_settings`.
    pub settings: &'a Plain,
    /// Its `op_options`; `None` writes no register.
    pub options: Option<&'a Plain>,
    /// Where it goes when it is new and its grid has no row yet: before the operation at this
    /// index of the list as it was before the command, or last.
    pub at: usize,
}

/// The cell items of `top` by `{ column, row }` and its label items, in tree order.
pub(crate) fn grid_items(tree: &LoroTree, top: TreeID) -> GridItemNodes {
    let (mut cells, mut labels) = (HashMap::new(), Vec::new());
    for item in descendants(tree, top) {
        let Ok(meta) = tree.get_meta(item) else {
            continue;
        };
        if meta.get(KEY_GRID_LABEL).is_some() {
            labels.push(item);
        }
        let Some(serde_json::Value::Object(tag)) = plain(&meta, KEY_GRID_CELL) else {
            continue;
        };
        let index = |key: &str| u8::try_from(tag.get(key)?.as_u64()?).ok();
        if let (Some(column), Some(row)) = (index("column"), index("row")) {
            cells.entry((column, row)).or_insert(item);
        }
    }
    (cells, labels)
}

/// Overwrites every register of the derived operation `meta` by `row` (Frage 204 A): kind, name,
/// Show and Output on, settings, options, part and members. Uncommitted.
pub(crate) fn fill_row(meta: &LoroMap, top: TreeID, row: &DerivedRow<'_>, items: &GridItemNodes) {
    insert(meta, KEY_SHAPE, SHAPE_OPERATION);
    insert(meta, KEY_OP_KIND, row.kind);
    insert(meta, KEY_NAME, row.name);
    for key in [KEY_OP_SHOW, KEY_OP_OUTPUT] {
        if meta.get(key).is_some() {
            let _ = meta.delete(key);
        }
    }
    write_plain_map(meta, KEY_OP_SETTINGS, row.settings);
    match row.options {
        Some(options) => write_plain_map(meta, KEY_OP_OPTIONS, options),
        None if meta.get(KEY_OP_OPTIONS).is_some() => {
            let _ = meta.delete(KEY_OP_OPTIONS);
        }
        None => {}
    }
    write_plain_map(meta, KEY_OP_GRID_PART, &row.part.to_plain(top));
    let mut old = Vec::new();
    meta.for_each(|key, _| {
        if key.starts_with(MEMBER_PREFIX) {
            old.push(key.to_owned());
        }
    });
    for key in old {
        let _ = meta.delete(&key);
    }
    let members: Vec<TreeID> = match row.part {
        DerivedPart::Labels => items.1.clone(),
        DerivedPart::Cell { column, row } => {
            items.0.get(&(column, row)).copied().into_iter().collect()
        }
    };
    for member in members {
        insert(meta, &member_key(&[member]), true);
    }
}

/// Deletes the derived operation `row`, uncommitted. An operation node is a child of its
/// manufacturing, never of a group, so no group is emptied.
pub(crate) fn delete_row(tree: &LoroTree, row: TreeID) {
    let _ = LoroTree::delete(tree, row);
}

/// A new operation node under `manufacturing`, placed by `place`. Uncommitted.
pub(crate) fn new_row(
    tree: &LoroTree,
    manufacturing: TreeID,
    place: impl FnOnce(TreeID) -> Result<(), loro::LoroError>,
) -> Result<TreeID, TestGridError> {
    tree.enable_fractional_index(0);
    let node = tree
        .create(TreeParentId::Node(manufacturing))
        .map_err(|_| TestGridError::NoSuchManufacturing)?;
    place(node).map_err(|_| TestGridError::NoSuchManufacturing)?;
    Ok(node)
}

/// Places `node` before the operation at `at` of `before` (the list before the command), or
/// after the last operation of `manufacturing` now.
pub(crate) fn place_at(
    tree: &LoroTree,
    manufacturing: TreeID,
    before: &[TreeID],
    at: usize,
    node: TreeID,
) -> Result<(), loro::LoroError> {
    if let Some(&anchor) = before.get(at) {
        return tree.mov_before(node, anchor);
    }
    let last = operation_nodes(tree, manufacturing)
        .into_iter()
        .rfind(|&op| op != node);
    last.map_or(Ok(()), |last| tree.mov_after(node, last))
}

/// Deletes, uncommitted, every derived operation whose `op_grid_part` names a node that is no
/// live group with a recipe register (ADR 0057 §3): the grid was deleted, cut or ungrouped in
/// this step, or a merge removed it. Called by every local step.
pub(crate) fn sweep_orphan_derived_rows(tree: &LoroTree) {
    for manufacturing in crate::manufacturing_codec::manufacturing_nodes(tree) {
        for op in operation_nodes(tree, manufacturing) {
            let orphan = tree
                .get_meta(op)
                .ok()
                .and_then(|meta| grid_part_of(&meta))
                .is_some_and(|(group, _)| !crate::test_grid_codec::is_grid_group(tree, group));
            if orphan {
                delete_row(tree, op);
            }
        }
    }
}
