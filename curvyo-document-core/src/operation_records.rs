//! The records an operation keeps for one of its items (`specs/0139` adrs L3, L4): the
//! `tabs.<key>` registers, keyed by the item's membership key text and opaque here.

use std::collections::BTreeMap;

use loro::{LoroTree, TreeID, ValueOrContainer};

use crate::document::{Document, OBJECTS_TREE};
use crate::manufacturing_codec::{json_of, manufacturing_nodes, write_plain_map};
use crate::operation::{OperationError, delete_key, operation_nodes};
use crate::operation_codec::{
    MEMBER_PREFIX, decode_path, grid_switch, member_key_of, path_is_live,
};
use crate::path_model::NodeId;

/// The prefix of an item's own tabs in one operation, `tabs.<key>` with `<key>` the text after
/// `member.` of the item's membership key.
pub(crate) const TABS_PREFIX: &str = "tabs.";

/// The record prefixes that live and die with a membership key (slice 2 adds its own).
const RECORD_PREFIXES: [&str; 1] = [TABS_PREFIX];

/// The label of a tab added by the Tabs tool (`specs/0139` criterion 12).
pub const ADD_TAB_COMMIT_LABEL: &str = "add_tab";
/// The label of a tab handle drag or key slide.
pub const MOVE_TAB_COMMIT_LABEL: &str = "move_tab";
/// The label of a tab deleted.
pub const DELETE_TAB_COMMIT_LABEL: &str = "delete_tab";
/// The label of a tab's own width typed or reset.
pub const TAB_WIDTH_COMMIT_LABEL: &str = "tab_width";
/// The label of the Own press of the Tabs pill.
pub const OWN_TABS_COMMIT_LABEL: &str = "own_tabs";
/// The label of the Operation's press of the Tabs pill.
pub const OPERATIONS_TABS_COMMIT_LABEL: &str = "operations_tabs";

/// The membership key text of the target `path` (what follows `member.`), the `<key>` of its
/// records in an operation.
#[must_use]
pub fn member_key_text(path: &[NodeId]) -> String {
    member_key_of(path)
        .strip_prefix(MEMBER_PREFIX)
        .unwrap_or_default()
        .to_owned()
}

impl Document {
    /// The own tabs records of the operation `id`, by key text, as plain values (a register that
    /// holds no plain value reads as `null`, damaged for the laser extension).
    ///
    /// # Errors
    /// [`OperationError::NoSuchOperation`].
    pub fn operation_tabs(
        &self,
        id: NodeId,
    ) -> Result<BTreeMap<String, serde_json::Value>, OperationError> {
        let meta = self.operation_meta(id)?;
        let mut out = BTreeMap::new();
        meta.for_each(|key, value| {
            let Some(text) = key.strip_prefix(TABS_PREFIX) else {
                return;
            };
            let value = match value {
                ValueOrContainer::Value(value) => {
                    json_of(&value).unwrap_or(serde_json::Value::Null)
                }
                ValueOrContainer::Container(_) => serde_json::Value::Null,
            };
            out.insert(text.to_owned(), value);
        });
        Ok(out)
    }

    /// Writes (`Some`) or removes (`None`) the own tabs record of the item with membership key
    /// text `key` in the operation `id`, one history step `label`. `Ok(false)` and nothing
    /// written when it is already so.
    ///
    /// # Errors
    /// [`OperationError::NoSuchOperation`]; [`OperationError::GridRow`] for a test grid row;
    /// [`OperationError::NoSuchTarget`] when the operation holds no membership `member.<key>`.
    pub fn write_operation_tabs(
        &self,
        id: NodeId,
        key: &str,
        value: Option<&serde_json::Map<String, serde_json::Value>>,
        label: &str,
    ) -> Result<bool, OperationError> {
        self.write_operation_tabs_many(id, &[(key.to_owned(), value.cloned())], label)
    }

    /// [`Document::write_operation_tabs`] for several items of one operation in one step (the
    /// Tabs pill of several selected items).
    ///
    /// # Errors
    /// As [`Document::write_operation_tabs`]; nothing is written then.
    pub fn write_operation_tabs_many(
        &self,
        id: NodeId,
        writes: &[(String, Option<serde_json::Map<String, serde_json::Value>>)],
        label: &str,
    ) -> Result<bool, OperationError> {
        let meta = self.operation_meta(id)?;
        if grid_switch(&meta) != Some(false) {
            return Err(OperationError::GridRow);
        }
        if writes
            .iter()
            .any(|(key, _)| meta.get(&format!("{MEMBER_PREFIX}{key}")).is_none())
        {
            return Err(OperationError::NoSuchTarget);
        }
        let mut changed = false;
        for (key, value) in writes {
            let record = format!("{TABS_PREFIX}{key}");
            let stored = if let Some(ValueOrContainer::Value(value)) = meta.get(&record) {
                json_of(&value)
            } else {
                None
            };
            if let Some(map) = value {
                if stored.as_ref().and_then(serde_json::Value::as_object) == Some(map) {
                    continue;
                }
                write_plain_map(&meta, &record, map);
            } else {
                if meta.get(&record).is_none() {
                    continue;
                }
                meta.delete(&record)
                    .map_err(|_| OperationError::NoSuchOperation)?;
            }
            changed = true;
        }
        if changed {
            self.member_keys_may_exist.set(true);
            self.commit_with_label(label);
        }
        Ok(changed)
    }

    /// Every item record of every operation whose key reads, with its operation and path.
    pub(crate) fn record_entries(&self) -> Vec<(TreeID, String, Vec<TreeID>)> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let mut out = Vec::new();
        for manufacturing in manufacturing_nodes(&tree) {
            for op in operation_nodes(&tree, manufacturing) {
                let Ok(meta) = tree.get_meta(op) else {
                    continue;
                };
                meta.for_each(|key, _| {
                    let path = RECORD_PREFIXES
                        .iter()
                        .find_map(|prefix| key.strip_prefix(prefix))
                        .and_then(decode_path);
                    if let Some(path) = path {
                        out.push((op, key.to_owned(), path));
                    }
                });
            }
        }
        out
    }
}

/// Removes, uncommitted, the records of operation `op` that belong to the membership key text
/// `key` (an unassign, ADR 0046 §3 amended by `specs/0139` L4).
pub(crate) fn delete_records_of(tree: &LoroTree, op: TreeID, key: &str) {
    for prefix in RECORD_PREFIXES {
        delete_key(tree, op, &format!("{prefix}{key}"));
    }
}

/// Removes, uncommitted, every record of `entries` whose path is no longer live.
pub(crate) fn sweep_dead_records(tree: &LoroTree, entries: Vec<(TreeID, String, Vec<TreeID>)>) {
    for (op, key, path) in entries {
        if !path_is_live(tree, &path) {
            delete_key(tree, op, &key);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DocumentSize, NewOperation, NewOutputPage, Point, RectBounds, pack, unpack};
    use serde_json::json;

    fn page_with_cut() -> (Document, NodeId, NodeId) {
        let d = Document::new(1);
        let laser = d
            .add_output_page(NewOutputPage {
                process: "curvyo.laser",
                label: "Laser",
                profile: None,
                work_area: DocumentSize::from_mm(400.0, 300.0),
            })
            .unwrap();
        let m = laser.kind.manufacturing().unwrap();
        let rect = d.create_rect(RectBounds::from_corners(
            Point::new(10.0, 10.0),
            Point::new(50.0, 50.0),
        ));
        let op = d
            .add_operation(
                m,
                &NewOperation {
                    kind: "cut",
                    name: "Cut 1",
                    settings: None,
                },
                0,
                &[vec![rect]],
            )
            .unwrap();
        (d, op, rect)
    }

    fn undo(d: &Document) {
        let last = d.own_steps().last().unwrap().step;
        d.undo_step(last).unwrap();
    }

    fn record() -> serde_json::Map<String, serde_json::Value> {
        json!({ "0": [{ "at": 250_000_000 }] })
            .as_object()
            .unwrap()
            .clone()
    }

    #[test]
    fn a_record_is_written_read_back_saved_and_undone_as_one_step() {
        let (d, op, rect) = page_with_cut();
        let key = member_key_text(&[rect]);
        assert!(
            d.write_operation_tabs(op, &key, Some(&record()), ADD_TAB_COMMIT_LABEL)
                .unwrap()
        );
        assert!(
            !d.write_operation_tabs(op, &key, Some(&record()), ADD_TAB_COMMIT_LABEL)
                .unwrap(),
            "the same value writes nothing"
        );
        assert_eq!(
            d.operation_tabs(op).unwrap().get(&key),
            Some(&serde_json::Value::Object(record()))
        );
        assert_eq!(d.own_steps().last().unwrap().label, ADD_TAB_COMMIT_LABEL);
        let back = unpack(2, &pack(&d, "tabs").unwrap()).unwrap();
        assert_eq!(back.operation_tabs(op).unwrap().len(), 1);
        undo(&d);
        assert!(d.operation_tabs(op).unwrap().is_empty());
        assert_eq!(
            d.write_operation_tabs(op, "9:9", Some(&record()), ADD_TAB_COMMIT_LABEL),
            Err(OperationError::NoSuchTarget)
        );
    }

    #[test]
    fn an_unassign_and_a_delete_take_the_record_in_their_step() {
        let (d, op, rect) = page_with_cut();
        let key = member_key_text(&[rect]);
        d.write_operation_tabs(op, &key, Some(&record()), ADD_TAB_COMMIT_LABEL)
            .unwrap();
        d.set_operation_members(op, &[vec![rect]], false).unwrap();
        assert!(d.operation_tabs(op).unwrap().is_empty());
        undo(&d);
        assert_eq!(
            d.operation_tabs(op).unwrap().len(),
            1,
            "undo brings it back"
        );
        d.delete_objects(&[rect]).unwrap();
        assert!(d.operation_tabs(op).unwrap().is_empty());
        undo(&d);
        assert_eq!(d.operation_tabs(op).unwrap().len(), 1);
    }
}
