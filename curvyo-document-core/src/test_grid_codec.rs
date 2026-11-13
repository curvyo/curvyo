//! The stored shape of a laser test grid (ADR 0054 §4, ADR 0057 §1, §2, §6): the keys on its top
//! group, items and derived operations, read back as plain values for the laser extension.

use loro::{LoroMap, LoroTree, LoroValue, TreeID, TreeParentId, ValueOrContainer};

use crate::document::{Document, OBJECTS_TREE};
use crate::group_tree::{is_group_node, node_id};
use crate::history::lineage::{decode_node, encode_node};
use crate::manufacturing_codec::{is_manufacturing_node, json_of, manufacturing_nodes, string};
use crate::operation::operation_nodes;
use crate::operation_codec::{
    KEY_OP_GRID_PART, MEMBER_PREFIX, SHAPE_OPERATION, decode_path, grid_switch,
};
use crate::path_codec::node_exists;
use crate::path_model::NodeId;
use crate::paths::tree_id_of;
use crate::shape_codec::KEY_SHAPE;
use crate::test_grid_object::DerivedPart;

/// The recipe on the top group, one atomic map.
pub(crate) const KEY_TEST_GRID: &str = "x.curvyo.laser.test_grid";
/// The Worked mark `{ column, row }` on the top group; absent when nothing is marked.
pub(crate) const KEY_TEST_GRID_WORKED: &str = "x.curvyo.laser.test_grid_worked";
/// The Unix milliseconds of the output that lasered the grid (ADR 0057 §1); absent is Not
/// lasered.
pub(crate) const KEY_TEST_GRID_LASERED: &str = "x.curvyo.laser.test_grid_lasered";
/// The tag of a cell item: `{ column, row }` (ADR 0057 §1), `{ column, row, power_pct,
/// speed_mm_min }` in a format-43 grid.
pub(crate) const KEY_GRID_CELL: &str = "x.curvyo.laser.grid_cell";
/// The tag of a label item: `"id"`, `"column_title"`, `"row_title"`, `"column_value"` or
/// `"row_value"` (ADR 0057 §1); `"power"`, `"speed"` or `"unit"` in a format-43 grid.
pub(crate) const KEY_GRID_LABEL: &str = "x.curvyo.laser.grid_label";

/// One tagged item of a grid.
#[derive(Debug, Clone, PartialEq)]
pub struct GridItem {
    /// The item.
    pub id: NodeId,
    /// Its tag as plain values (`null` when it holds no plain value).
    pub tag: serde_json::Value,
}

/// A test grid as stored: the top group's registers and its tagged items, uninterpreted.
#[derive(Debug, Clone, PartialEq)]
pub struct TestGridSnapshot {
    /// The top group.
    pub group: NodeId,
    /// The recipe as plain values (`null` when it holds no plain value).
    pub recipe: serde_json::Value,
    /// The Worked mark, `None` when absent.
    pub worked: Option<serde_json::Value>,
    /// The descendants tagged as cells, in tree order.
    pub cells: Vec<GridItem>,
    /// The descendants tagged as labels, in tree order.
    pub labels: Vec<GridItem>,
    /// The grid rows whose membership names this group (one, unless a merge left more).
    pub rows: Vec<NodeId>,
    /// The Lasered register: `None` when absent or not a non-negative integer (Not lasered).
    pub lasered: Option<u64>,
    /// The derived operations whose `op_grid_part` names this group, in list order.
    pub derived: Vec<NodeId>,
}

/// The group and part a derived operation's `op_grid_part` names: `None` when the register is
/// absent, no map, its `grid` is no encoded node id or its part does not read (the row is then
/// ordinary, ADR 0057 §6). The same reading as the laser extension's `GridPart::read`.
pub(crate) fn grid_part_of(meta: &LoroMap) -> Option<(TreeID, DerivedPart)> {
    let ValueOrContainer::Value(LoroValue::Map(map)) = meta.get(KEY_OP_GRID_PART)? else {
        return None;
    };
    let LoroValue::String(grid) = map.get("grid")? else {
        return None;
    };
    let index = |key: &str| match map.get(key) {
        Some(LoroValue::I64(n)) => u8::try_from(*n).ok(),
        _ => None,
    };
    let part = match map.get("part")? {
        LoroValue::String(part) if part.as_str() == "labels" && map.len() == 2 => {
            DerivedPart::Labels
        }
        LoroValue::String(part) if part.as_str() == "cell" && map.len() == 4 => DerivedPart::Cell {
            column: index("column")?,
            row: index("row")?,
        },
        _ => return None,
    };
    Some((decode_node(grid)?, part))
}

/// The text a derived operation's `op_grid_part.grid` holds for the group `group`.
pub(crate) fn encode_grid(group: TreeID) -> String {
    encode_node(group)
}

/// The derived operations of every manufacturing whose `op_grid_part` names `group`, in list
/// order.
pub(crate) fn derived_rows_naming(tree: &LoroTree, group: TreeID) -> Vec<TreeID> {
    manufacturing_nodes(tree)
        .into_iter()
        .flat_map(|manufacturing| operation_nodes(tree, manufacturing))
        .filter(|&op| {
            tree.get_meta(op)
                .is_ok_and(|meta| grid_part_of(&meta).is_some_and(|(of, _)| of == group))
        })
        .collect()
}

/// The plain values of the register `key`: `None` when absent, `null` when not plain.
pub(crate) fn plain(meta: &LoroMap, key: &str) -> Option<serde_json::Value> {
    match meta.get(key)? {
        ValueOrContainer::Value(value) => Some(json_of(&value).unwrap_or(serde_json::Value::Null)),
        ValueOrContainer::Container(_) => Some(serde_json::Value::Null),
    }
}

/// Whether `node` is a group carrying a recipe register.
pub(crate) fn is_grid_group(tree: &LoroTree, node: TreeID) -> bool {
    node_exists(tree, node)
        && is_group_node(tree, node)
        && tree
            .get_meta(node)
            .is_ok_and(|meta| meta.get(KEY_TEST_GRID).is_some())
}

/// The descendants of `node`, depth first in tree order.
pub(crate) fn descendants(tree: &LoroTree, node: TreeID) -> Vec<TreeID> {
    let mut out = Vec::new();
    let mut stack = vec![node];
    while let Some(at) = stack.pop() {
        let children = tree.children(TreeParentId::Node(at)).unwrap_or_default();
        for &child in children.iter().rev() {
            stack.push(child);
        }
        if at != node {
            out.push(at);
        }
    }
    out
}

/// The grid rows of every manufacturing with a membership key whose path is `[group]`, with the
/// key, live or not.
pub(crate) fn grid_rows_naming(tree: &LoroTree, group: TreeID) -> Vec<TreeID> {
    let mut rows = Vec::new();
    for manufacturing in manufacturing_nodes(tree) {
        for op in tree
            .children(TreeParentId::Node(manufacturing))
            .unwrap_or_default()
        {
            let Ok(meta) = tree.get_meta(op) else {
                continue;
            };
            if string(&meta, KEY_SHAPE).as_deref() != Some(SHAPE_OPERATION)
                || grid_switch(&meta) != Some(true)
            {
                continue;
            }
            let mut names = false;
            meta.for_each(|key, _| {
                let path = key.strip_prefix(MEMBER_PREFIX).and_then(decode_path);
                names |= path.as_deref() == Some(&[group]);
            });
            if names {
                rows.push(op);
            }
        }
    }
    rows
}

impl Document {
    /// The test grid whose top group is `group`, `None` when `group` is no live group with a
    /// recipe register. Interprets nothing: the laser extension reads the values.
    #[must_use]
    pub fn test_grid(&self, group: NodeId) -> Option<TestGridSnapshot> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let node = tree_id_of(group);
        if !is_grid_group(&tree, node) {
            return None;
        }
        let meta = tree.get_meta(node).ok()?;
        let (mut cells, mut labels) = (Vec::new(), Vec::new());
        for item in descendants(&tree, node) {
            let Ok(item_meta) = tree.get_meta(item) else {
                continue;
            };
            if let Some(tag) = plain(&item_meta, KEY_GRID_CELL) {
                cells.push(GridItem {
                    id: node_id(item),
                    tag,
                });
            } else if let Some(tag) = plain(&item_meta, KEY_GRID_LABEL) {
                labels.push(GridItem {
                    id: node_id(item),
                    tag,
                });
            }
        }
        Some(TestGridSnapshot {
            group,
            recipe: plain(&meta, KEY_TEST_GRID).unwrap_or(serde_json::Value::Null),
            worked: plain(&meta, KEY_TEST_GRID_WORKED),
            cells,
            labels,
            rows: grid_rows_naming(&tree, node)
                .into_iter()
                .map(node_id)
                .collect(),
            lasered: plain(&meta, KEY_TEST_GRID_LASERED).and_then(|value| value.as_u64()),
            derived: derived_rows_naming(&tree, node)
                .into_iter()
                .map(node_id)
                .collect(),
        })
    }

    /// The top group a grid row names, `None` for a row whose group is gone ("no grid") or an id
    /// that is no grid row.
    #[must_use]
    pub fn test_grid_of_row(&self, row: NodeId) -> Option<NodeId> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let node = tree_id_of(row);
        let in_manufacturing = matches!(
            tree.parent(node),
            Some(TreeParentId::Node(parent)) if is_manufacturing_node(&tree, parent)
        );
        if !node_exists(&tree, node) || !in_manufacturing {
            return None;
        }
        let meta = tree.get_meta(node).ok()?;
        if grid_switch(&meta) != Some(true) {
            return None;
        }
        let mut group = None;
        meta.for_each(|key, value| {
            let path = key.strip_prefix(MEMBER_PREFIX).and_then(decode_path);
            if let (Some([one]), ValueOrContainer::Value(LoroValue::Bool(true))) =
                (path.as_deref(), value)
                && is_grid_group(&tree, *one)
            {
                group = Some(node_id(*one));
            }
        });
        group
    }

    /// The grid group the derived operation `row` belongs to: `None` when `row` is no
    /// operation, its `op_grid_part` does not read or names no live group with a recipe register.
    #[must_use]
    pub fn test_grid_of_derived_row(&self, row: NodeId) -> Option<NodeId> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let node = tree_id_of(row);
        let in_manufacturing = matches!(
            tree.parent(node),
            Some(TreeParentId::Node(parent)) if is_manufacturing_node(&tree, parent)
        );
        if !node_exists(&tree, node) || !in_manufacturing {
            return None;
        }
        let (group, _) = grid_part_of(&tree.get_meta(node).ok()?)?;
        is_grid_group(&tree, group).then(|| node_id(group))
    }

    /// The grid tags of the item `id`: its cell tag and its label tag as plain values.
    #[must_use]
    pub fn grid_item_tags(
        &self,
        id: NodeId,
    ) -> (Option<serde_json::Value>, Option<serde_json::Value>) {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let node = tree_id_of(id);
        if !node_exists(&tree, node) {
            return (None, None);
        }
        tree.get_meta(node).map_or((None, None), |meta| {
            (plain(&meta, KEY_GRID_CELL), plain(&meta, KEY_GRID_LABEL))
        })
    }
}
