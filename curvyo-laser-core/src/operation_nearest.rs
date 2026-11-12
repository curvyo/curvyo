//! The nearest rule of criterion 12: which operations a leaf runs with (ADR 0046 §5).

use std::collections::HashMap;

use curvyo_document_core::{NodeId, OperationSnapshot};

use crate::operation_kind::KindRead;
use crate::operation_sequence::run_sequence;

/// One operation as the rules read it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OperationRule {
    /// Its node.
    pub id: NodeId,
    /// Its kind.
    pub kind: KindRead,
    /// Show (view only).
    pub show: bool,
    /// Output (what runs).
    pub output: bool,
}

impl OperationRule {
    /// The rule of a stored operation.
    #[must_use]
    pub fn of(snapshot: &OperationSnapshot) -> Self {
        Self {
            id: snapshot.id,
            kind: KindRead::of(snapshot.kind.as_deref()),
            show: snapshot.show,
            output: snapshot.output,
        }
    }

    /// Whether a leaf assigned to it runs in it: its kind is known and Output is on.
    #[must_use]
    pub const fn runs(&self) -> bool {
        self.output && self.kind.known().is_some()
    }
}

/// Which chain entries may decide a leaf.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChainScope {
    /// The whole chain, nearest first.
    Whole,
    /// Only the leaf's own entry: a source leaf hidden on its drawing page inside a placed page
    /// (criterion 13, Frage 149 A).
    OwnOnly,
}

/// The node that decided a leaf's operations and those operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Assignment {
    /// The nearest key of the chain with any membership: the target (Cut-out's item, Frage 168
    /// A), a path of node ids (ADR 0046 §2).
    pub target: Vec<NodeId>,
    /// The target's operations as positions in the stored list, in run order; Unknown and Output
    /// off ones included.
    pub operations: Vec<usize>,
}

/// One laser page's operations with their run order and an index from target to operations.
#[derive(Debug, Clone)]
pub struct OperationList {
    rules: Vec<OperationRule>,
    sequence: Vec<usize>,
    by_target: HashMap<Vec<NodeId>, Vec<usize>>,
}

impl OperationList {
    /// The list of `operations` (stored order) under the Cuts last switch. Missing items are not
    /// targets: a gone node is in no chain.
    #[must_use]
    pub fn new(operations: &[OperationSnapshot], cuts_last: bool) -> Self {
        let rules: Vec<OperationRule> = operations.iter().map(OperationRule::of).collect();
        let kinds: Vec<KindRead> = rules.iter().map(|rule| rule.kind).collect();
        let sequence = run_sequence(&kinds, cuts_last);
        let mut by_target: HashMap<Vec<NodeId>, Vec<usize>> = HashMap::new();
        for &at in &sequence {
            for member in &operations[at].members {
                by_target.entry(member.clone()).or_default().push(at);
            }
        }
        Self {
            rules,
            sequence,
            by_target,
        }
    }

    /// The operations in stored order.
    #[must_use]
    pub fn rules(&self) -> &[OperationRule] {
        &self.rules
    }

    /// The run order, as positions in the stored list.
    #[must_use]
    pub fn sequence(&self) -> &[usize] {
        &self.sequence
    }

    /// The operations `target` itself is in, in run order.
    #[must_use]
    pub fn operations_of(&self, target: &[NodeId]) -> &[usize] {
        self.by_target.get(target).map_or(&[], Vec::as_slice)
    }

    /// The assignment of a leaf with the chain `chain` (the leaf first, then its holders, the
    /// clone or placement last): the first entry with any membership decides, whatever its
    /// operations' kinds or switches, and nothing further up counts. `None` is Not assigned.
    #[must_use]
    pub fn nearest(&self, chain: &[Vec<NodeId>], scope: ChainScope) -> Option<Assignment> {
        let entries = match scope {
            ChainScope::Whole => chain,
            ChainScope::OwnOnly => &chain[..chain.len().min(1)],
        };
        entries.iter().find_map(|key| {
            let operations = self.by_target.get(key)?;
            Some(Assignment {
                target: key.clone(),
                operations: operations.clone(),
            })
        })
    }

    /// The operations of `assignment` the leaf runs in, in run order.
    pub fn running<'a>(&'a self, assignment: &'a Assignment) -> impl Iterator<Item = usize> + 'a {
        assignment
            .operations
            .iter()
            .copied()
            .filter(|&at| self.rules[at].runs())
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use curvyo_document_core::{Length, NodeId};

    use super::*;

    pub(crate) fn id(n: u64) -> NodeId {
        serde_json::from_value(serde_json::json!({ "peer": "1", "counter": n })).unwrap()
    }

    pub(crate) fn op(
        n: u64,
        kind: Option<&str>,
        output: bool,
        members: &[NodeId],
    ) -> OperationSnapshot {
        OperationSnapshot {
            id: id(1000 + n),
            name: format!("op {n}"),
            kind: kind.map(str::to_owned),
            show: true,
            output,
            margin: Length::from_mm(0.0),
            members: members.iter().map(|&member| vec![member]).collect(),
            settings: None,
            grid: false,
        }
    }

    #[test]
    fn the_nearest_assigned_node_decides_and_nothing_further_up_counts() {
        // Group G in Cut-out, text T inside G in Engrave (criterion 12 example).
        let (g, t, r) = (id(1), id(2), id(3));
        let list = OperationList::new(
            &[
                op(0, Some("cut_out"), true, &[g]),
                op(1, Some("engrave"), true, &[t]),
            ],
            true,
        );
        let at_t = list
            .nearest(&[vec![t], vec![g]], ChainScope::Whole)
            .unwrap();
        assert_eq!((at_t.target, at_t.operations.clone()), (vec![t], vec![1]));
        let at_r = list
            .nearest(&[vec![r], vec![g]], ChainScope::Whole)
            .unwrap();
        assert_eq!((at_r.target, at_r.operations.clone()), (vec![g], vec![0]));
        assert_eq!(
            list.nearest(&[vec![id(9)]], ChainScope::Whole),
            None,
            "Not assigned"
        );
    }

    #[test]
    fn an_item_in_several_operations_runs_in_each_in_run_order() {
        let x = id(1);
        let ops = [
            op(0, Some("cut"), true, &[x]),
            op(1, Some("score"), true, &[x]),
            op(2, Some("engrave"), false, &[x]),
        ];
        let on = OperationList::new(&ops, true);
        let assignment = on.nearest(&[vec![x]], ChainScope::Whole).unwrap();
        assert_eq!(assignment.operations, [1, 2, 0]);
        assert_eq!(
            on.running(&assignment).collect::<Vec<_>>(),
            [1, 0],
            "Output off runs nothing"
        );
        let off = OperationList::new(&ops, false);
        let assignment = off.nearest(&[vec![x]], ChainScope::Whole).unwrap();
        assert_eq!(off.running(&assignment).collect::<Vec<_>>(), [0, 1]);
    }

    #[test]
    fn an_unknown_operation_stops_the_rule_and_runs_nothing() {
        let (leaf, group) = (id(1), id(2));
        let list = OperationList::new(
            &[
                op(0, Some("cut"), true, &[group]),
                op(1, Some("dot_mode"), true, &[leaf]),
                op(2, None, true, &[leaf]),
            ],
            true,
        );
        let assignment = list
            .nearest(&[vec![leaf], vec![group]], ChainScope::Whole)
            .unwrap();
        assert_eq!(
            assignment.target,
            vec![leaf],
            "the leaf never falls back to its group's Cut"
        );
        assert_eq!(list.running(&assignment).count(), 0);
    }

    #[test]
    fn an_output_off_operation_still_decides_the_leaf() {
        let (leaf, group) = (id(1), id(2));
        let list = OperationList::new(
            &[
                op(0, Some("cut"), true, &[group]),
                op(1, Some("engrave"), false, &[leaf]),
            ],
            true,
        );
        let assignment = list
            .nearest(&[vec![leaf], vec![group]], ChainScope::Whole)
            .unwrap();
        assert_eq!(assignment.target, vec![leaf]);
        assert_eq!(list.running(&assignment).count(), 0);
    }

    #[test]
    fn a_hidden_source_leaf_in_a_placed_page_uses_only_its_own_key() {
        let (leaf, layer, placement) = (id(1), id(2), id(3));
        let list = OperationList::new(
            &[
                op(0, Some("cut_out"), true, &[placement]),
                op(1, Some("engrave"), true, &[]),
            ],
            true,
        );
        let chain = [vec![leaf], vec![layer], vec![placement]];
        assert_eq!(list.nearest(&chain, ChainScope::OwnOnly), None);
        assert_eq!(
            list.nearest(&chain, ChainScope::Whole).unwrap().target,
            vec![placement]
        );
        let own = OperationList::new(
            &[
                op(0, Some("cut_out"), true, &[placement]),
                op(1, Some("engrave"), true, &[leaf]),
            ],
            true,
        );
        let assignment = own.nearest(&chain, ChainScope::OwnOnly).unwrap();
        assert_eq!(
            (assignment.target, assignment.operations),
            (vec![leaf], vec![1])
        );
        assert_eq!(own.nearest(&[], ChainScope::OwnOnly), None);
    }
}
