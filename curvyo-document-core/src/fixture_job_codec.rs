//! The stored shape of a fixture job node (ADR 0052 §1 to §3): its keys and how one job is read
//! back field by field, with a count of the values that did not read.

use loro::{LoroMap, LoroValue, TreeID, ValueOrContainer};
use serde::Serialize;

use crate::group_tree::node_id;
use crate::history::lineage::{decode_node, encode_node};
use crate::manufacturing_codec::string;
use crate::manufacturing_fixtures::SlotSnapshot;
use crate::node_registers::{KEY_NAME, read_registers};
use crate::operation_codec::{KEY_OP_KIND, KEY_OP_OUTPUT, KEY_OP_SHOW};
use crate::path_model::NodeId;
use crate::paths::tree_id_of;
use crate::shape_codec::KEY_SHAPE;

/// The `shape` tag of a fixture job node.
pub(crate) const SHAPE_FIXTURE_JOB: &str = "fixture_job";
/// The prefix of a tick, `target.<part key>` = `true`.
pub(crate) const TARGET_PREFIX: &str = "target.";
/// The separator between a node id and its part suffix; it never occurs in an encoded id.
const SUFFIX_SEPARATOR: char = '.';
/// The part suffixes a key may carry (ADR 0052 §2): the grid's lines and tapes, the screw's head
/// and thread. Which of them a fixture has is the extension's rule.
pub const PART_SUFFIXES: [&str; 4] = ["lines", "tapes", "head", "thread"];

/// A fixture part by place, without node ids: what a profile set stores and a copy maps back.
/// The derived order is the Parts-list order: the fixture's own parts (no suffix, then the
/// suffixes `head`, `lines`, `tapes`, `thread`), then the slots by index.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct JobTarget {
    /// The 0-based index among the fixture's slots; `None` for the fixture's own node.
    pub slot: Option<usize>,
    /// The part suffix (`lines`, `tapes`, `head`, `thread`), `None` for a whole node.
    pub suffix: Option<String>,
}

/// A fixture part by node: the key a tick is stored under, `target.<node>[.<suffix>]`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct FixturePartKey {
    /// The fixture node or one of its slot nodes.
    pub node: NodeId,
    /// The part suffix, `None` for a whole node.
    pub suffix: Option<String>,
}

/// One stored tick that names a live part of the job's fixture.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TargetRead {
    /// The part by node.
    pub key: FixturePartKey,
    /// The part by place.
    pub target: JobTarget,
    /// Whether the stored value is `true`; any other value is damage and no target.
    pub ticked: bool,
}

/// A fixture job as read (ADR 0052 §3). The kind is opaque here and typed by the extension.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FixtureJobSnapshot {
    /// Its node.
    pub id: NodeId,
    /// Its name.
    pub name: String,
    /// The stored kind text; `None` when absent or not a text (an Unknown job).
    pub kind: Option<String>,
    /// The Show switch: absent or damaged reads on.
    pub show: bool,
    /// The Output switch: absent or damaged reads off, so a job never runs by accident.
    pub output: bool,
    /// The keys that name the fixture or one of its readable slots, in Parts-list order. Keys of
    /// other nodes are left out and are not damage.
    pub targets: Vec<TargetRead>,
    /// How many stored values did not read.
    pub damaged: usize,
}

/// A job's values without a node: what a profile set stores and a copy writes (ADR 0052 §7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobRecord {
    /// The kind text.
    pub kind: String,
    /// The name.
    pub name: String,
    /// The Show switch; Output is never part of a record.
    pub show: bool,
    /// The ticked parts, sorted, without duplicates.
    pub targets: Vec<JobTarget>,
}

impl FixtureJobSnapshot {
    /// The job as a record; `None` for a job without a kind text.
    #[must_use]
    pub fn record(&self) -> Option<JobRecord> {
        let mut targets: Vec<JobTarget> = self
            .targets
            .iter()
            .filter(|target| target.ticked)
            .map(|target| target.target.clone())
            .collect();
        targets.sort_unstable();
        targets.dedup();
        Some(JobRecord {
            kind: self.kind.clone()?,
            name: self.name.clone(),
            show: self.show,
            targets,
        })
    }
}

/// The stored key of the part `key`.
pub(crate) fn target_key(key: &FixturePartKey) -> String {
    let node = encode_node(tree_id_of(key.node));
    match &key.suffix {
        Some(suffix) => format!("{TARGET_PREFIX}{node}{SUFFIX_SEPARATOR}{suffix}"),
        None => format!("{TARGET_PREFIX}{node}"),
    }
}

/// The node and the suffix a stored key names; `None` for a key that is not a target key or
/// whose id does not read.
pub(crate) fn parse_target_key(key: &str) -> Option<(TreeID, Option<&str>)> {
    let rest = key.strip_prefix(TARGET_PREFIX)?;
    let (node, suffix) = match rest.split_once(SUFFIX_SEPARATOR) {
        Some((node, suffix)) => (node, Some(suffix)),
        None => (rest, None),
    };
    Some((decode_node(node)?, suffix))
}

/// Whether `meta` is a fixture job node.
pub(crate) fn is_job_meta(meta: &LoroMap) -> bool {
    string(meta, KEY_SHAPE).as_deref() == Some(SHAPE_FIXTURE_JOB)
}

/// A switch read: its value and whether it was damaged; `absent` when there is none.
fn switch(meta: &LoroMap, key: &str, absent: bool) -> (bool, usize) {
    match meta.get(key) {
        None => (absent, 0),
        Some(ValueOrContainer::Value(LoroValue::Bool(flag))) => (flag, 0),
        Some(_) => (absent, 1),
    }
}

/// Reads the job node `node` of the fixture `fixture`, whose readable slots are `slots`.
pub(crate) fn read_job(
    node: TreeID,
    meta: &LoroMap,
    fixture: TreeID,
    slots: &[SlotSnapshot],
) -> FixtureJobSnapshot {
    let kind = string(meta, KEY_OP_KIND);
    let (show, show_damage) = switch(meta, KEY_OP_SHOW, true);
    let (output, output_damage) = switch(meta, KEY_OP_OUTPUT, false);
    let mut damaged = show_damage + output_damage + usize::from(kind.is_none());
    let mut targets = Vec::new();
    meta.for_each(|key, value| {
        let Some((part, suffix)) = parse_target_key(key) else {
            return;
        };
        let slot = if part == fixture {
            None
        } else {
            match slots.iter().position(|s| tree_id_of(s.id) == part) {
                Some(index) => Some(index),
                None => return,
            }
        };
        let ticked = matches!(value, ValueOrContainer::Value(LoroValue::Bool(true)));
        damaged += usize::from(!ticked);
        let suffix = suffix.map(str::to_owned);
        targets.push(TargetRead {
            key: FixturePartKey {
                node: node_id(part),
                suffix: suffix.clone(),
            },
            target: JobTarget { slot, suffix },
            ticked,
        });
    });
    targets.sort_unstable_by(|a, b| a.target.cmp(&b.target));
    FixtureJobSnapshot {
        id: node_id(node),
        name: read_registers(meta).name.unwrap_or_default(),
        kind,
        show,
        output,
        targets,
        damaged,
    }
}

/// Writes a new job node's registers: its kind, name and Show, Output off, and one key per tick.
pub(crate) fn write_job_registers(meta: &LoroMap, record: &JobRecord, ticks: &[FixturePartKey]) {
    // invariant: plain values under known keys on an attached map cannot fail.
    #[allow(clippy::unwrap_used)]
    {
        meta.insert(KEY_SHAPE, SHAPE_FIXTURE_JOB).unwrap();
        meta.insert(KEY_OP_KIND, record.kind.as_str()).unwrap();
        meta.insert(KEY_NAME, record.name.as_str()).unwrap();
        meta.insert(KEY_OP_SHOW, record.show).unwrap();
        meta.insert(KEY_OP_OUTPUT, false).unwrap();
        for tick in ticks {
            meta.insert(&target_key(tick), true).unwrap();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_target_key_reads_back_and_the_separator_never_occurs_in_an_id() {
        let key = FixturePartKey {
            node: node_id(TreeID::new(7, 3)),
            suffix: Some("lines".to_owned()),
        };
        assert_eq!(target_key(&key), "target.7:3.lines");
        assert_eq!(
            parse_target_key("target.7:3.lines"),
            Some((TreeID::new(7, 3), Some("lines")))
        );
        assert_eq!(
            parse_target_key("target.9:12"),
            Some((TreeID::new(9, 12), None))
        );
        assert_eq!(parse_target_key("target.x"), None);
        assert_eq!(parse_target_key("member.7:3"), None);
        assert!(!encode_node(TreeID::new(u64::MAX, i32::MAX)).contains(SUFFIX_SEPARATOR));
    }

    #[test]
    fn the_target_order_is_the_parts_list_order() {
        let whole = |suffix: Option<&str>| JobTarget {
            slot: None,
            suffix: suffix.map(str::to_owned),
        };
        let slot = |n| JobTarget {
            slot: Some(n),
            suffix: None,
        };
        let mut targets = vec![
            slot(1),
            whole(Some("thread")),
            slot(0),
            whole(None),
            whole(Some("head")),
        ];
        targets.sort_unstable();
        assert_eq!(
            targets,
            [
                whole(None),
                whole(Some("head")),
                whole(Some("thread")),
                slot(0),
                slot(1)
            ]
        );
        assert!(whole(Some("lines")) < whole(Some("tapes")));
    }
}
