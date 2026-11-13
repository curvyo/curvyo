//! The items of a laser test grid and the copy case of an old grid row (ADR 0054 §5, ADR 0057
//! §2): the grid's group and tagged items as the laser extension laid them out.

use loro::{LoroMap, LoroTree, LoroValue, TreeID, TreeParentId};

use crate::document::{Document, OBJECTS_TREE};
use crate::drawing_content::DrawingText;
use crate::drawing_insert::{DrawingNode, DrawingRegisters, Writer};
use crate::manufacturing_codec::write_plain_map;
use crate::node_registers::KEY_NAME;
use crate::operation_codec::{KEY_OP_GRID, KEY_OP_KIND, KEY_OP_OUTPUT, KEY_OP_SHOW, member_key};
use crate::path_codec::node_exists;
use crate::path_model::NodeId;
use crate::paths::tree_id_of;
use crate::primitive_model::Shape;
use crate::shape_codec::KEY_SHAPE;
use crate::style_model::Style;
use crate::test_grid_codec::{
    KEY_GRID_CELL, KEY_GRID_LABEL, KEY_TEST_GRID_WORKED, descendants, grid_rows_naming,
    is_grid_group,
};
use crate::units::Angle;

/// The label of Add test grid (criterion 3).
pub const ADD_TEST_GRID_COMMIT_LABEL: &str = "add_test_grid";
/// The label of a grid field edit (criterion 9).
pub const EDIT_TEST_GRID_COMMIT_LABEL: &str = "edit_test_grid";

type Plain = serde_json::Map<String, serde_json::Value>;

/// One cell of a grid as the laser extension laid it out.
#[derive(Debug, Clone, PartialEq)]
pub struct NewGridCell {
    /// Its square, a rectangle primitive.
    pub shape: Shape,
    /// Its style (outline for Cut and Score, filled for Engrave).
    pub style: Style,
    /// Its `grid_cell` tag.
    pub tag: Plain,
}

/// One label of a grid as the laser extension laid it out.
#[derive(Debug, Clone, PartialEq)]
pub struct NewGridLabel {
    /// Its text item.
    pub text: DrawingText,
    /// Its `grid_label` tag (`power`, `speed`, `unit`).
    pub tag: String,
}

/// The items of a grid: its cells and its labels (none with Labels off).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct GridItems {
    /// The cells, row by row.
    pub cells: Vec<NewGridCell>,
    /// The labels.
    pub labels: Vec<NewGridLabel>,
}

/// A refused grid command; it writes nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum TestGridError {
    /// The id names no live manufacturing.
    #[error("no such manufacturing")]
    NoSuchManufacturing,
    /// The id names no live test grid group.
    #[error("no such test grid")]
    NoSuchGrid,
    /// A label text the document cannot hold (empty, too long, a size out of range).
    #[error("a label text out of range")]
    BadLabel,
    /// The recipe has no text `id` (ADR 0057 §1).
    #[error("a recipe without an id")]
    NoId,
    /// A Lasered grid's recipe changes under its old id (ADR 0057 §4).
    #[error("a lasered grid needs a new id")]
    IdKept,
    /// A grid that is not Lasered would change its id (ADR 0057 §4).
    #[error("a grid that is not lasered keeps its id")]
    IdChanged,
}

pub(crate) fn insert(meta: &LoroMap, key: &str, value: impl Into<LoroValue>) {
    // invariant: inserting a plain value under a known key on an attached map cannot fail.
    #[allow(clippy::unwrap_used)]
    meta.insert(key, value).unwrap();
}

impl Document {
    fn grid_writer(&self) -> Writer<'_, std::iter::Empty<crate::path_model::AnchorId>> {
        Writer {
            document: self,
            tree: self.loro().get_tree(OBJECTS_TREE),
            ids: std::iter::empty(),
            leaves: Vec::new(),
            clones: Vec::new(),
            links: Vec::new(),
            assets: &[],
        }
    }

    /// Writes a new group named `name` under `parent`, uncommitted.
    pub(crate) fn write_group(&self, parent: TreeParentId, name: Option<&str>) -> TreeID {
        let registers = DrawingRegisters {
            name: name.map(str::to_owned),
            ..DrawingRegisters::default()
        };
        self.grid_writer().write_one(
            parent,
            &DrawingNode::Group {
                registers,
                layer: false,
                children: Vec::new(),
            },
        )
    }

    /// Writes the cells group and the labels group of `items` as the last children of `top`.
    pub(crate) fn write_grid_items(&self, top: TreeID, items: &GridItems) {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let cells = self.write_group(TreeParentId::Node(top), Some("Cells"));
        for cell in &items.cells {
            let leaf = self.grid_writer().write_one(
                TreeParentId::Node(cells),
                &DrawingNode::Primitive {
                    registers: DrawingRegisters::default(),
                    shape: cell.shape,
                    rotation: Angle::from_radians(0.0),
                    style: cell.style.clone(),
                },
            );
            if let Ok(meta) = tree.get_meta(leaf) {
                write_plain_map(&meta, KEY_GRID_CELL, &cell.tag);
            }
        }
        if items.labels.is_empty() {
            return;
        }
        let labels = self.write_group(TreeParentId::Node(top), Some("Labels"));
        for label in &items.labels {
            let leaf = self.write_drawing_text(TreeParentId::Node(labels), &label.text);
            if let Ok(meta) = tree.get_meta(leaf) {
                insert(&meta, KEY_GRID_LABEL, label.tag.as_str());
            }
        }
    }

    pub(crate) fn grid_group(&self, group: NodeId) -> Result<(LoroTree, TreeID), TestGridError> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let node = tree_id_of(group);
        is_grid_group(&tree, node)
            .then_some((tree, node))
            .ok_or(TestGridError::NoSuchGrid)
    }

    /// Replaces the tagged items of the grid `top` by `items` in a new cells group and labels
    /// group (an emptied group goes; untagged items stay), uncommitted.
    pub(crate) fn replace_grid_items(&self, tree: &LoroTree, top: TreeID, items: &GridItems) {
        let doomed: Vec<TreeID> = descendants(tree, top)
            .into_iter()
            .filter(|&item| {
                tree.get_meta(item).is_ok_and(|meta| {
                    meta.get(KEY_GRID_CELL).is_some() || meta.get(KEY_GRID_LABEL).is_some()
                })
            })
            .collect();
        // A linked clone of a replaced cell or label follows it as for any delete (`0049`).
        self.repair_clones_of_removed(tree, &doomed);
        let mut parents = vec![Some(top)];
        for item in doomed {
            if node_exists(tree, item) {
                parents.push(crate::group_tree::group_parent(tree, item));
                let _ = tree.delete(item);
            }
        }
        // The new items first, so the top group is never emptied while the old groups go.
        self.write_grid_items(top, items);
        crate::groups::remove_emptied_groups(tree, &parents);
    }
}

/// After a Ctrl copy, Duplicate or Paste (ADR 0054 §5, amends ADR 0046 §4): a copied top group
/// loses its mark and gets a row of its own, a copy of each row of the original with the copy's
/// key and placed after it, in the commit that creates the copy. Uncommitted.
pub(crate) fn give_copied_grids_rows(
    document: &Document,
    tree: &LoroTree,
    made: &[(TreeID, TreeID)],
) {
    for &(source, copy) in made {
        if !is_grid_group(tree, copy) {
            continue;
        }
        if let Ok(meta) = tree.get_meta(copy) {
            let _ = meta.delete(KEY_TEST_GRID_WORKED);
            // ADR 0057 §3: a copy is never Lasered; its derived rows come from the caller, who
            // gives it a new id in the same step.
            let _ = meta.delete(crate::test_grid_codec::KEY_TEST_GRID_LASERED);
        }
        for row in grid_rows_naming(tree, source) {
            let Some(TreeParentId::Node(manufacturing)) = tree.parent(row) else {
                continue;
            };
            let Ok(original) = tree.get_meta(row) else {
                continue;
            };
            let Ok(new_row) = tree.create(TreeParentId::Node(manufacturing)) else {
                continue;
            };
            tree.enable_fractional_index(0);
            let _ = tree.mov_after(new_row, row);
            let Ok(meta) = tree.get_meta(new_row) else {
                continue;
            };
            for key in [
                KEY_SHAPE,
                KEY_OP_GRID,
                KEY_OP_KIND,
                KEY_NAME,
                KEY_OP_SHOW,
                KEY_OP_OUTPUT,
            ] {
                if let Some(loro::ValueOrContainer::Value(value)) = original.get(key) {
                    insert(&meta, key, value);
                }
            }
            insert(&meta, &member_key(&[copy]), true);
            document.member_keys_may_exist.set(true);
        }
    }
}
