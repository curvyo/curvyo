//! The commands that write a test grid object with its derived operations (ADR 0057 §2 to §5,
//! `specs/0112` criteria 2, 8, 11 to 14, 28); the laser extension resolves every value.

use std::collections::HashMap;

use loro::{LoroMap, LoroTree, TreeID, TreeParentId};

use crate::document::{Document, OBJECTS_TREE};
use crate::group_tree::node_id;
use crate::manufacturing_codec::{is_manufacturing_node, write_plain_map};
use crate::node_registers::KEY_NAME;
use crate::operation::operation_nodes;
use crate::path_codec::node_exists;
use crate::path_model::NodeId;
use crate::paths::tree_id_of;
use crate::test_grid_codec::{
    KEY_GRID_LABEL, KEY_TEST_GRID, KEY_TEST_GRID_LASERED, derived_rows_naming, grid_part_of, plain,
};
use crate::test_grid_commands::{
    ADD_TEST_GRID_COMMIT_LABEL, EDIT_TEST_GRID_COMMIT_LABEL, GridItems, TestGridError, insert,
};
pub use crate::test_grid_rows::{DerivedPart, DerivedRow};
use crate::test_grid_rows::{GridItemNodes, delete_row, fill_row, grid_items, new_row, place_at};

/// The label of Place on a page from Materials (criterion 28).
pub const PLACE_TEST_GRID_COMMIT_LABEL: &str = "place_test_grid";
/// The label of the Lasered test hook (ADR 0057 §5); no UI writes it.
pub const MARK_TEST_GRID_LASERED_COMMIT_LABEL: &str = "mark_test_grid_lasered";
/// The label of a material pick in the bar (UX G8).
const USE_MATERIAL_LABEL: &str = crate::operation_settings_commands::USE_MATERIAL_COMMIT_LABEL;

type Plain = serde_json::Map<String, serde_json::Value>;

/// The step a grid write is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GridStep {
    /// Add test grid (the tool's press, a copy).
    Add,
    /// Place test grid (recreated from Materials).
    Place,
    /// Edit test grid (a bar commit).
    Edit,
    /// Use material (a material pick in the bar).
    UseMaterial,
}

impl GridStep {
    const fn label(self) -> &'static str {
        match self {
            Self::Add => ADD_TEST_GRID_COMMIT_LABEL,
            Self::Place => PLACE_TEST_GRID_COMMIT_LABEL,
            Self::Edit => EDIT_TEST_GRID_COMMIT_LABEL,
            Self::UseMaterial => USE_MATERIAL_LABEL,
        }
    }
}

/// Which id change a grid write allows (ADR 0057 §3, §4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IdRule {
    /// A bar commit: a Lasered grid whose recipe changes needs a new id, any other keeps it.
    Edit,
    /// A copy: always a new id, never Lasered.
    Copy,
}

/// What a grid write writes: the recipe, the items and the derived operations.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TestGridWrite<'a> {
    /// The manufacturing whose list gets new rows.
    pub manufacturing: NodeId,
    /// The recipe as plain values; its `id` is a text.
    pub recipe: &'a Plain,
    /// The items, laid out by the extension.
    pub items: &'a GridItems,
    /// The derived operations in list order.
    pub rows: &'a [DerivedRow<'a>],
}

/// A new grid object.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NewGridObject<'a> {
    /// The name of its group.
    pub name: &'a str,
    /// What it holds.
    pub write: TestGridWrite<'a>,
    /// The Lasered register (a grid recreated from a Lasered test), `None` for Not lasered.
    pub lasered: Option<u64>,
    /// [`GridStep::Add`] or [`GridStep::Place`].
    pub step: GridStep,
}

fn recipe_id(recipe: &Plain) -> Result<&str, TestGridError> {
    recipe
        .get("id")
        .and_then(serde_json::Value::as_str)
        .ok_or(TestGridError::NoId)
}

fn check_labels(items: &GridItems) -> Result<(), TestGridError> {
    if items.labels.iter().any(|label| !label.text.is_valid()) {
        return Err(TestGridError::BadLabel);
    }
    Ok(())
}

fn grid_manufacturing(tree: &LoroTree, id: NodeId) -> Result<TreeID, TestGridError> {
    let node = tree_id_of(id);
    (node_exists(tree, node) && is_manufacturing_node(tree, node))
        .then_some(node)
        .ok_or(TestGridError::NoSuchManufacturing)
}

/// Whether the Lasered register of `meta` reads (a non-negative integer).
fn is_lasered(meta: &LoroMap) -> bool {
    plain(meta, KEY_TEST_GRID_LASERED).is_some_and(|v| v.as_u64().is_some())
}

impl Document {
    /// Add test grid, a copy, or Place test grid (criteria 2, 8, 28): the group `name` under the
    /// creation parent with the recipe, the Lasered register when given, the items and the
    /// derived operations, in one history step. Returns the group.
    ///
    /// # Errors
    /// [`TestGridError::NoSuchManufacturing`]; [`TestGridError::BadLabel`];
    /// [`TestGridError::NoId`] when the recipe has no text `id`.
    pub fn place_test_grid(&self, new: &NewGridObject<'_>) -> Result<NodeId, TestGridError> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let write = &new.write;
        let manufacturing = grid_manufacturing(&tree, write.manufacturing)?;
        check_labels(write.items)?;
        recipe_id(write.recipe)?;
        let before = operation_nodes(&tree, manufacturing);
        let top = self.write_group(self.creation_parent(&tree), Some(new.name));
        let meta = tree.get_meta(top).map_err(|_| TestGridError::NoSuchGrid)?;
        write_plain_map(&meta, KEY_TEST_GRID, write.recipe);
        if let Some(at) = new.lasered {
            insert(
                &meta,
                KEY_TEST_GRID_LASERED,
                i64::try_from(at).unwrap_or(i64::MAX),
            );
        }
        self.write_grid_items(top, write.items);
        let items = grid_items(&tree, top);
        for row in write.rows {
            let node = new_row(&tree, manufacturing, |node| {
                place_at(&tree, manufacturing, &before, row.at, node)
            })?;
            let row_meta = tree.get_meta(node).map_err(|_| TestGridError::NoSuchGrid)?;
            fill_row(&row_meta, top, row, &items);
        }
        self.member_keys_may_exist.set(true);
        self.commit_with_label(new.step.label());
        Ok(node_id(top))
    }

    /// Regeneration (criterion 14, ADR 0057 §2): writes the recipe, replaces the items and
    /// rewrites the grid's derived operations by part in place (node id and place kept,
    /// everything else overwritten); a missing part goes after the grid's last row (or at its
    /// `at` when the grid has none), a surplus row is deleted, in one history step `step`. A
    /// Lasered grid whose recipe changes loses its Lasered register; the caller gives the new id
    /// (criterion 11).
    ///
    /// # Errors
    /// [`TestGridError::NoSuchGrid`]; [`TestGridError::NoSuchManufacturing`];
    /// [`TestGridError::BadLabel`]; [`TestGridError::NoId`]; [`TestGridError::IdKept`] when a
    /// Lasered grid's recipe changes under its old id; [`TestGridError::IdChanged`] when a grid
    /// that is not Lasered would change its id. Nothing is written then.
    pub fn regenerate_test_grid(
        &self,
        group: NodeId,
        write: &TestGridWrite<'_>,
        step: GridStep,
    ) -> Result<(), TestGridError> {
        self.write_grid(group, write, step.label(), IdRule::Edit)
    }

    /// The copy rule (criterion 8, ADR 0057 §3): the copy `copy` that Duplicate or Paste just
    /// made gets the caller's new id, no Lasered register, its own items and derived rows, as one
    /// step "Add test grid" or inside the caller's [`Document::batch`] of the copy.
    ///
    /// # Errors
    /// As [`Document::regenerate_test_grid`]; [`TestGridError::IdKept`] when the id is the old
    /// one.
    pub fn copy_test_grid(
        &self,
        copy: NodeId,
        write: &TestGridWrite<'_>,
    ) -> Result<(), TestGridError> {
        self.write_grid(copy, write, ADD_TEST_GRID_COMMIT_LABEL, IdRule::Copy)
    }

    fn write_grid(
        &self,
        group: NodeId,
        write: &TestGridWrite<'_>,
        label: &str,
        rule: IdRule,
    ) -> Result<(), TestGridError> {
        let (tree, top) = self.grid_group(group)?;
        let manufacturing = grid_manufacturing(&tree, write.manufacturing)?;
        check_labels(write.items)?;
        let meta = tree.get_meta(top).map_err(|_| TestGridError::NoSuchGrid)?;
        let stored = plain(&meta, KEY_TEST_GRID);
        let lasered = is_lasered(&meta);
        let new_id = recipe_id(write.recipe)?;
        let old_id = stored
            .as_ref()
            .and_then(|recipe| recipe.get("id")?.as_str().map(str::to_owned));
        let changed = stored.as_ref() != Some(&serde_json::Value::Object(write.recipe.clone()));
        let renews = lasered && changed;
        check_id_rule(rule, renews, old_id.as_deref(), new_id)?;
        if rule == IdRule::Copy || renews {
            let _ = meta.delete(KEY_TEST_GRID_LASERED);
        }
        write_plain_map(&meta, KEY_TEST_GRID, write.recipe);
        self.replace_grid_items(&tree, top, write.items);
        let items = grid_items(&tree, top);
        sync_rows(&tree, top, manufacturing, write.rows, &items)?;
        self.member_keys_may_exist.set(true);
        self.commit_with_label(label);
        Ok(())
    }

    /// A new test id for a Lasered grid (criterion 11, ADR 0057 §4): writes `new_id` into the
    /// recipe, removes the Lasered register, rewrites the `id` label item and the old id in the
    /// names of its derived operations, as one step "Edit test grid" or inside the caller's
    /// [`Document::batch`] (a hand edit of a derived row's settings, options or members).
    /// `Ok(false)`, writing nothing, when the grid is not Lasered: it keeps its id.
    ///
    /// # Errors
    /// [`TestGridError::NoSuchGrid`]; [`TestGridError::NoId`] when the stored recipe has no text
    /// `id`; [`TestGridError::IdKept`] when `new_id` is the old id.
    pub fn renew_test_id(&self, group: NodeId, new_id: &str) -> Result<bool, TestGridError> {
        let (tree, top) = self.grid_group(group)?;
        let meta = tree.get_meta(top).map_err(|_| TestGridError::NoSuchGrid)?;
        if !is_lasered(&meta) {
            return Ok(false);
        }
        let Some(serde_json::Value::Object(mut recipe)) = plain(&meta, KEY_TEST_GRID) else {
            return Err(TestGridError::NoId);
        };
        let old_id = recipe_id(&recipe)?.to_owned();
        if old_id == new_id {
            return Err(TestGridError::IdKept);
        }
        recipe.insert("id".to_owned(), new_id.into());
        write_plain_map(&meta, KEY_TEST_GRID, &recipe);
        let _ = meta.delete(KEY_TEST_GRID_LASERED);
        for item in grid_items(&tree, top).1 {
            let Ok(item_meta) = tree.get_meta(item) else {
                continue;
            };
            if plain(&item_meta, KEY_GRID_LABEL)
                .as_ref()
                .and_then(|v| v.as_str())
                == Some("id")
                && let Some(text) = crate::text_codec::string_container(&item_meta)
            {
                crate::text_codec::apply_string_diff(&text, new_id);
            }
        }
        for row in derived_rows_naming(&tree, top) {
            let Ok(row_meta) = tree.get_meta(row) else {
                continue;
            };
            let name = crate::node_registers::read_registers(&row_meta).name;
            if let Some(name) = name.filter(|name| name.contains(&old_id)) {
                insert(&row_meta, KEY_NAME, name.replace(&old_id, new_id).as_str());
            }
        }
        self.commit_with_label(EDIT_TEST_GRID_COMMIT_LABEL);
        Ok(true)
    }

    /// Removes the derived operations of the grid `group` (criterion 13: the grid moved off its
    /// laser page), inside the caller's [`Document::batch`] of the move, or as its own step
    /// `translate_objects`. Returns how many went.
    pub fn remove_test_grid_rows(&self, group: NodeId) -> usize {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let rows = derived_rows_naming(&tree, tree_id_of(group));
        for &row in &rows {
            delete_row(&tree, row);
        }
        if !rows.is_empty() {
            self.commit_with_label("translate_objects");
        }
        rows.len()
    }

    /// The Lasered test hook (ADR 0057 §5): writes `test_grid_lasered` = `at_ms` on the grid
    /// `group` in one step. Only `0029`'s output and the acceptance tests write it.
    ///
    /// # Errors
    /// [`TestGridError::NoSuchGrid`].
    #[doc(hidden)]
    pub fn mark_test_grid_lasered_for_tests(
        &self,
        group: NodeId,
        at_ms: u64,
    ) -> Result<(), TestGridError> {
        let (tree, top) = self.grid_group(group)?;
        let meta = tree.get_meta(top).map_err(|_| TestGridError::NoSuchGrid)?;
        insert(
            &meta,
            KEY_TEST_GRID_LASERED,
            i64::try_from(at_ms).unwrap_or(i64::MAX),
        );
        self.commit_with_label(MARK_TEST_GRID_LASERED_COMMIT_LABEL);
        Ok(())
    }
}

/// Refuses a write whose test id breaks `rule` (criterion 11, ADR 0057 §4).
fn check_id_rule(
    rule: IdRule,
    renews: bool,
    old_id: Option<&str>,
    new_id: &str,
) -> Result<(), TestGridError> {
    let same_id = old_id == Some(new_id);
    match rule {
        IdRule::Copy if same_id => Err(TestGridError::IdKept),
        IdRule::Edit if renews && same_id => Err(TestGridError::IdKept),
        IdRule::Edit if !renews && !same_id && old_id.is_some() => Err(TestGridError::IdChanged),
        _ => Ok(()),
    }
}

/// Reuses, creates and deletes the derived operation rows of the grid `top` so they match `rows`.
fn sync_rows(
    tree: &LoroTree,
    top: TreeID,
    manufacturing: TreeID,
    rows_wanted: &[DerivedRow<'_>],
    items: &GridItemNodes,
) -> Result<(), TestGridError> {
    let mut existing: HashMap<DerivedPart, TreeID> = HashMap::new();
    let mut surplus = Vec::new();
    let rows = derived_rows_naming(tree, top);
    for &row in &rows {
        let part = tree
            .get_meta(row)
            .ok()
            .and_then(|m| grid_part_of(&m))
            .map(|(_, part)| part);
        match part {
            Some(part) if !existing.contains_key(&part) => {
                existing.insert(part, row);
            }
            _ => surplus.push(row),
        }
    }
    let before = operation_nodes(tree, manufacturing);
    let mut cursor = rows
        .iter()
        .rev()
        .find(|&&row| tree.parent(row) == Some(TreeParentId::Node(manufacturing)))
        .copied();
    for row in rows_wanted {
        let node = if let Some(node) = existing.remove(&row.part) {
            node
        } else {
            let node = new_row(tree, manufacturing, |node| match cursor {
                Some(after) => tree.mov_after(node, after),
                None => place_at(tree, manufacturing, &before, row.at, node),
            })?;
            cursor = Some(node);
            node
        };
        let row_meta = tree.get_meta(node).map_err(|_| TestGridError::NoSuchGrid)?;
        fill_row(&row_meta, top, row, items);
    }
    for node in surplus.into_iter().chain(existing.into_values()) {
        delete_row(tree, node);
    }
    Ok(())
}

#[cfg(test)]
#[path = "test_grid_object_tests.rs"]
mod tests;
