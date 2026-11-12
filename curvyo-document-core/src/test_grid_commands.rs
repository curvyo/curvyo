//! The commands that write a laser test grid and its row (ADR 0054 §4, §5; `specs/0028` criteria
//! 3, 4, 6, 9 to 11, 13): make, rebuild, mark, and the delete and copy cases; the laser extension
//! lays the grid out and decides every rule.

use loro::{LoroMap, LoroTree, LoroValue, TreeID, TreeParentId};

use crate::document::{Document, OBJECTS_TREE};
use crate::drawing_content::DrawingText;
use crate::drawing_insert::{DrawingNode, DrawingRegisters, Writer};
use crate::group_tree::node_id;
use crate::manufacturing_codec::{is_manufacturing_node, write_plain_map};
use crate::node_registers::KEY_NAME;
use crate::operation_codec::{
    KEY_OP_GRID, KEY_OP_KIND, KEY_OP_OUTPUT, KEY_OP_SHOW, SHAPE_OPERATION, member_key,
};
use crate::path_codec::node_exists;
use crate::path_model::NodeId;
use crate::paths::tree_id_of;
use crate::primitive_model::Shape;
use crate::shape_codec::KEY_SHAPE;
use crate::style_model::Style;
use crate::test_grid_codec::{
    KEY_GRID_CELL, KEY_GRID_LABEL, KEY_TEST_GRID, KEY_TEST_GRID_WORKED, descendants,
    grid_rows_naming, is_grid_group,
};
use crate::units::Angle;

/// The label of Add test grid (criterion 3).
pub const ADD_TEST_GRID_COMMIT_LABEL: &str = "add_test_grid";
/// The label of a grid field edit (criterion 9).
pub const EDIT_TEST_GRID_COMMIT_LABEL: &str = "edit_test_grid";
/// The label of Rebuild (criterion 10).
pub const REBUILD_TEST_GRID_COMMIT_LABEL: &str = "rebuild_test_grid";
/// The label of marking a cell as Worked (criterion 13).
pub const MARK_TEST_CELL_COMMIT_LABEL: &str = "mark_test_cell";
/// The label of removing the Worked mark (criterion 13).
pub const UNMARK_TEST_CELL_COMMIT_LABEL: &str = "unmark_test_cell";
/// The label of a grid row rename (criterion 6).
pub const RENAME_TEST_GRID_COMMIT_LABEL: &str = "rename_test_grid";
/// The label of a grid row delete; the items stay (criterion 6).
pub const DELETE_TEST_GRID_ROW_COMMIT_LABEL: &str = "delete_test_grid_row";

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

/// What Add test grid writes.
#[derive(Debug, Clone, PartialEq)]
pub struct NewTestGrid<'a> {
    /// The manufacturing whose list gets the row.
    pub manufacturing: NodeId,
    /// The name of the top group and of its row, `Test grid <N>`.
    pub name: &'a str,
    /// The grid's kind text (the row's `op_kind`).
    pub kind: &'a str,
    /// The recipe as plain values.
    pub recipe: &'a Plain,
    /// The items.
    pub items: GridItems,
    /// Where the row goes in the operation list (or last).
    pub row_index: usize,
}

/// The nodes Add test grid made.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MadeTestGrid {
    /// The top group.
    pub group: NodeId,
    /// The grid row.
    pub row: NodeId,
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
}

/// A field edit of a grid (criteria 8 to 10).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TestGridEdit<'a> {
    /// The grid's top group.
    pub group: NodeId,
    /// The new recipe as plain values.
    pub recipe: &'a Plain,
    /// The mark after the edit; `None` removes it.
    pub worked: Option<&'a Plain>,
    /// The rebuilt items; `None` for a hand-edited grid, whose items stay.
    pub items: Option<&'a GridItems>,
    /// With a Kind change: where the rows go and what they become.
    pub row: Option<RowMove<'a>>,
}

/// A Kind change of a grid's rows: the new kind text and the index in the operation list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RowMove<'a> {
    /// The rows' new kind text.
    pub kind: &'a str,
    /// Their index in the operation list.
    pub index: usize,
}

/// Writes the mark `worked` on the top group, or removes it, uncommitted.
fn write_mark(meta: &LoroMap, worked: Option<&Plain>) {
    match worked {
        Some(mark) => write_plain_map(meta, KEY_TEST_GRID_WORKED, mark),
        None => {
            let _ = meta.delete(KEY_TEST_GRID_WORKED);
        }
    }
}

fn insert(meta: &LoroMap, key: &str, value: impl Into<LoroValue>) {
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
    fn write_group(&self, parent: TreeParentId, name: Option<&str>) -> TreeID {
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
    fn write_grid_items(&self, top: TreeID, items: &GridItems) {
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

    fn grid_group(&self, group: NodeId) -> Result<(LoroTree, TreeID), TestGridError> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let node = tree_id_of(group);
        is_grid_group(&tree, node)
            .then_some((tree, node))
            .ok_or(TestGridError::NoSuchGrid)
    }

    /// Add test grid (criteria 3, 4, 6): the top group "Test grid `<N>`" with its recipe, a cells
    /// group, a labels group (with labels) and the grid row with `op_grid` and the one membership
    /// of the top group, in one history step. The group is made under the creation parent.
    ///
    /// # Errors
    /// [`TestGridError::NoSuchManufacturing`]; [`TestGridError::BadLabel`].
    pub fn add_test_grid(&self, new: &NewTestGrid<'_>) -> Result<MadeTestGrid, TestGridError> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let manufacturing = tree_id_of(new.manufacturing);
        if !node_exists(&tree, manufacturing) || !is_manufacturing_node(&tree, manufacturing) {
            return Err(TestGridError::NoSuchManufacturing);
        }
        if new.items.labels.iter().any(|label| !label.text.is_valid()) {
            return Err(TestGridError::BadLabel);
        }
        let top = self.write_group(self.creation_parent(&tree), Some(new.name));
        let top_meta = tree.get_meta(top).map_err(|_| TestGridError::NoSuchGrid)?;
        write_plain_map(&top_meta, KEY_TEST_GRID, new.recipe);
        self.write_grid_items(top, &new.items);
        let row = self.insert_grid_row(&tree, manufacturing, new.row_index, top, |meta| {
            insert(meta, KEY_OP_KIND, new.kind);
            insert(meta, KEY_NAME, new.name);
        })?;
        self.commit_with_label(ADD_TEST_GRID_COMMIT_LABEL);
        Ok(MadeTestGrid {
            group: node_id(top),
            row: node_id(row),
        })
    }

    /// Creates a grid row for `top` at `index` of the operation list of `manufacturing`,
    /// uncommitted; `fill` writes its kind and name.
    fn insert_grid_row(
        &self,
        tree: &LoroTree,
        manufacturing: TreeID,
        index: usize,
        top: TreeID,
        fill: impl FnOnce(&LoroMap),
    ) -> Result<TreeID, TestGridError> {
        let others: Vec<TreeID> = tree
            .children(TreeParentId::Node(manufacturing))
            .unwrap_or_default()
            .into_iter()
            .filter(|&child| {
                tree.get_meta(child).is_ok_and(|m| {
                    crate::manufacturing_codec::string(&m, KEY_SHAPE).as_deref()
                        == Some(SHAPE_OPERATION)
                })
            })
            .collect();
        tree.enable_fractional_index(0);
        let row = tree
            .create(TreeParentId::Node(manufacturing))
            .map_err(|_| TestGridError::NoSuchManufacturing)?;
        let placed = match (others.get(index), others.last()) {
            (Some(&before), _) => tree.mov_before(row, before),
            (None, Some(&after)) => tree.mov_after(row, after),
            (None, None) => Ok(()),
        };
        placed.map_err(|_| TestGridError::NoSuchManufacturing)?;
        let meta = tree
            .get_meta(row)
            .map_err(|_| TestGridError::NoSuchManufacturing)?;
        insert(&meta, KEY_SHAPE, SHAPE_OPERATION);
        insert(&meta, KEY_OP_GRID, true);
        fill(&meta);
        insert(&meta, &member_key(&[top]), true);
        self.member_keys_may_exist.set(true);
        Ok(row)
    }

    /// Replaces the tagged items of the grid `top` by `items` in a new cells group and labels
    /// group (an emptied group goes; untagged items stay), uncommitted.
    fn replace_grid_items(&self, tree: &LoroTree, top: TreeID, items: &GridItems) {
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

    /// Rebuild of a hand-edited grid (criterion 10): the tagged items of the grid `group` are
    /// replaced by `items` in a new cells group and labels group (an emptied group goes; untagged
    /// items stay), and the mark is set to `worked` or removed, in one history step. The row keeps its Show and Output.
    ///
    /// # Errors
    /// [`TestGridError::NoSuchGrid`]; [`TestGridError::BadLabel`].
    pub fn rebuild_test_grid(
        &self,
        group: NodeId,
        worked: Option<&Plain>,
        items: &GridItems,
    ) -> Result<(), TestGridError> {
        let (tree, top) = self.grid_group(group)?;
        if items.labels.iter().any(|label| !label.text.is_valid()) {
            return Err(TestGridError::BadLabel);
        }
        self.replace_grid_items(&tree, top, items);
        let meta = tree.get_meta(top).map_err(|_| TestGridError::NoSuchGrid)?;
        write_mark(&meta, worked);
        self.commit_with_label(REBUILD_TEST_GRID_COMMIT_LABEL);
        Ok(())
    }

    /// A field edit of the grid `group` (criteria 8 to 10), one history step "Edit test grid":
    /// the recipe and the mark; the items replaced by `edit.items` unless that is `None` (a
    /// hand-edited grid keeps its items until Rebuild); with `edit.row` the grid rows get the new
    /// kind text and move to that index of the operation list (a Kind change).
    ///
    /// # Errors
    /// [`TestGridError::NoSuchGrid`]; [`TestGridError::BadLabel`].
    pub fn edit_test_grid(&self, edit: &TestGridEdit<'_>) -> Result<(), TestGridError> {
        let (tree, top) = self.grid_group(edit.group)?;
        if edit
            .items
            .is_some_and(|items| items.labels.iter().any(|label| !label.text.is_valid()))
        {
            return Err(TestGridError::BadLabel);
        }
        if let Some(items) = edit.items {
            self.replace_grid_items(&tree, top, items);
        }
        let meta = tree.get_meta(top).map_err(|_| TestGridError::NoSuchGrid)?;
        write_plain_map(&meta, KEY_TEST_GRID, edit.recipe);
        write_mark(&meta, edit.worked);
        if let Some(RowMove { kind, index }) = edit.row {
            for row in grid_rows_naming(&tree, top) {
                let Some(TreeParentId::Node(manufacturing)) = tree.parent(row) else {
                    continue;
                };
                if let Ok(row_meta) = tree.get_meta(row) {
                    insert(&row_meta, KEY_OP_KIND, kind);
                }
                let others: Vec<TreeID> = crate::operation::operation_nodes(&tree, manufacturing)
                    .into_iter()
                    .filter(|&node| node != row)
                    .collect();
                tree.enable_fractional_index(0);
                crate::operation::place_in_list(&tree, row, &others, index);
            }
        }
        self.commit_with_label(EDIT_TEST_GRID_COMMIT_LABEL);
        Ok(())
    }

    /// Marks the cell `worked` of the grid `group` (replacing any other mark) or, with `None`,
    /// removes the mark, in one history step "Mark cell" or "Unmark cell" (criterion 13).
    /// `Ok(false)` when the mark already reads so.
    ///
    /// # Errors
    /// [`TestGridError::NoSuchGrid`].
    pub fn set_test_grid_mark(
        &self,
        group: NodeId,
        worked: Option<&Plain>,
    ) -> Result<bool, TestGridError> {
        let (tree, top) = self.grid_group(group)?;
        let meta = tree.get_meta(top).map_err(|_| TestGridError::NoSuchGrid)?;
        let now = crate::test_grid_codec::plain(&meta, KEY_TEST_GRID_WORKED);
        let wanted = worked.map(|mark| serde_json::Value::Object(mark.clone()));
        if now == wanted {
            return Ok(false);
        }
        if let Some(mark) = worked {
            write_plain_map(&meta, KEY_TEST_GRID_WORKED, mark);
            self.commit_with_label(MARK_TEST_CELL_COMMIT_LABEL);
        } else {
            let _ = meta.delete(KEY_TEST_GRID_WORKED);
            self.commit_with_label(UNMARK_TEST_CELL_COMMIT_LABEL);
        }
        Ok(true)
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

#[cfg(test)]
#[path = "test_grid_commands_tests.rs"]
mod tests;
