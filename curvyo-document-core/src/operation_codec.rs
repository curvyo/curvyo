//! The stored shape of a laser operation node (ADR 0046 §1, §2, §5; ADR 0054 §1, §4): its keys
//! and how one record is read back.

use loro::{LoroMap, LoroTree, LoroValue, TreeID, ValueOrContainer};

use crate::group_tree::node_id;
use crate::history::lineage::{decode_node, encode_node};
use crate::manufacturing_codec::{json_of, string};
use crate::node_registers::read_registers;
use crate::path_codec::{as_f64, node_exists};
use crate::path_model::NodeId;
use crate::units::Length;

/// The `shape` tag of an operation node.
pub(crate) const SHAPE_OPERATION: &str = "operation";
/// The kind text (`cut`, `score`, `engrave`, `engrave_outline`, `cut_out`), opaque here.
pub(crate) const KEY_OP_KIND: &str = "op_kind";

/// The Show switch; absent reads as on.
pub(crate) const KEY_OP_SHOW: &str = "op_show";
/// The Output switch; absent reads as on.
pub(crate) const KEY_OP_OUTPUT: &str = "op_output";
/// The Cut-out margin in mm, finite and at least 0; absent reads as 0.
pub(crate) const KEY_OP_MARGIN: &str = "op_margin";
/// The atomic settings map (ADR 0054 §1), plain values handed to the extension; absent is No
/// settings.
pub(crate) const KEY_OP_SETTINGS: &str = "op_settings";
/// The grid row switch (ADR 0054 §4); absent reads as off.
pub(crate) const KEY_OP_GRID: &str = "op_grid";
/// The prefix of a membership key, `member.<path>` = `true`.
pub(crate) const MEMBER_PREFIX: &str = "member.";
/// The separator of the node ids of a clone member's path; it never occurs in an encoded id.
pub(crate) const PATH_SEPARATOR: char = '/';
/// The register after `x.<process id>.` that holds the Cuts last switch (ADR 0046 §3).
pub(crate) const CUTS_LAST_FIELD: &str = "cuts_last";

/// One operation as read. Kind and colour are opaque texts; the laser extension interprets them.
#[derive(Debug, Clone, PartialEq)]
pub struct OperationSnapshot {
    /// Its node.
    pub id: NodeId,
    /// Its name.
    pub name: String,
    /// The stored kind text; `None` when the record cannot be read (ADR 0046 §1: an Unknown
    /// operation), so no register of it is trusted for a run.
    pub kind: Option<String>,
    /// The Show switch (view only).
    pub show: bool,
    /// The Output switch (what runs).
    pub output: bool,
    /// The Cut-out margin.
    pub margin: Length,
    /// The live targets, each a path: one node id for an own object, group, clone, placement or
    /// inner row; the outermost clone down to the member for a clone member (Frage 167 B). A key
    /// whose path is not live (a merge leftover) or does not read is left out (Frage 169 A).
    pub members: Vec<Vec<NodeId>>,
    /// The `op_settings` register as plain values, `None` when absent; a register that holds no
    /// plain value reads as `null` (Damaged settings, never an Unknown operation; ADR 0054 §1).
    pub settings: Option<serde_json::Value>,
    /// Whether it is the row of a test grid (`op_grid`, ADR 0054 §4).
    pub grid: bool,
}

/// The membership key of the target `path`.
pub(crate) fn member_key(path: &[TreeID]) -> String {
    let texts: Vec<String> = path.iter().map(|&node| encode_node(node)).collect();
    format!("{MEMBER_PREFIX}{}", texts.join(&PATH_SEPARATOR.to_string()))
}

/// The path of a membership key's text after the prefix; `None` when an id does not read.
pub(crate) fn decode_path(text: &str) -> Option<Vec<TreeID>> {
    text.split(PATH_SEPARATOR).map(decode_node).collect()
}

/// Whether every node of `path` is live.
pub(crate) fn path_is_live(tree: &LoroTree, path: &[TreeID]) -> bool {
    !path.is_empty() && path.iter().all(|&node| node_exists(tree, node))
}

/// The switch `key`: `Some(true)` when absent, `None` when present and not a boolean.
fn switch(meta: &LoroMap, key: &str) -> Option<bool> {
    match meta.get(key) {
        None => Some(true),
        Some(ValueOrContainer::Value(LoroValue::Bool(flag))) => Some(flag),
        Some(_) => None,
    }
}

/// The `op_settings` register: `None` when absent, `null` when it holds no plain value.
fn settings(meta: &LoroMap) -> Option<serde_json::Value> {
    match meta.get(KEY_OP_SETTINGS)? {
        ValueOrContainer::Value(value) => Some(json_of(&value).unwrap_or(serde_json::Value::Null)),
        ValueOrContainer::Container(_) => Some(serde_json::Value::Null),
    }
}

/// The grid switch: `Some(false)` when absent, `None` when present and not a boolean.
pub(crate) fn grid_switch(meta: &LoroMap) -> Option<bool> {
    match meta.get(KEY_OP_GRID) {
        None => Some(false),
        Some(ValueOrContainer::Value(LoroValue::Bool(flag))) => Some(flag),
        Some(_) => None,
    }
}

/// The margin: `Some(0)` when absent, `None` when present and not a finite number of at least 0.
fn margin(meta: &LoroMap) -> Option<f64> {
    match meta.get(KEY_OP_MARGIN) {
        None => Some(0.0),
        Some(ValueOrContainer::Value(value)) => {
            as_f64(&value).filter(|mm| mm.is_finite() && *mm >= 0.0)
        }
        Some(ValueOrContainer::Container(_)) => None,
    }
}

/// Reads the operation node `node`. A record with a register of the wrong type keeps its name
/// and memberships and reads with `kind` `None`.
pub(crate) fn read_operation(tree: &LoroTree, node: TreeID, meta: &LoroMap) -> OperationSnapshot {
    let mut members = Vec::new();
    let mut members_readable = true;
    meta.for_each(|key, value| {
        let Some(text) = key.strip_prefix(MEMBER_PREFIX) else {
            return;
        };
        if !matches!(value, ValueOrContainer::Value(LoroValue::Bool(true))) {
            members_readable = false;
        }
        if let Some(path) = decode_path(text).filter(|path| path_is_live(tree, path)) {
            members.push(path.into_iter().map(node_id).collect::<Vec<NodeId>>());
        }
    });
    members.sort_unstable_by_key(|path| {
        path.iter()
            .map(|id| (id.peer, id.counter))
            .collect::<Vec<_>>()
    });
    let show = switch(meta, KEY_OP_SHOW);
    let output = switch(meta, KEY_OP_OUTPUT);
    let margin_mm = margin(meta);
    let grid = grid_switch(meta);
    let readable = members_readable
        && show.is_some()
        && output.is_some()
        && margin_mm.is_some()
        && grid.is_some();
    OperationSnapshot {
        id: node_id(node),
        name: read_registers(meta).name.unwrap_or_default(),
        kind: string(meta, KEY_OP_KIND).filter(|_| readable),
        show: show.unwrap_or(true),
        output: output.unwrap_or(true),
        margin: Length::from_mm(margin_mm.unwrap_or(0.0)),
        members,
        settings: settings(meta),
        grid: grid.unwrap_or(false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_path_key_reads_back_and_the_separator_never_occurs_in_an_id() {
        let path = vec![TreeID::new(7, 3), TreeID::new(9, 12)];
        let key = member_key(&path);
        assert_eq!(key, "member.7:3/9:12");
        assert_eq!(
            key.strip_prefix(MEMBER_PREFIX).and_then(decode_path),
            Some(path)
        );
        assert!(!encode_node(TreeID::new(u64::MAX, i32::MAX)).contains(PATH_SEPARATOR));
    }
}
