//! The counts of the Operations tab's summary line (criterion 17, ADR 0046 §5).

use std::collections::HashSet;

use curvyo_document_core::NodeId;

use crate::operation_kind_run::{LeafShape, kind_run};
use crate::operation_nearest::{Assignment, ChainScope, OperationList};

/// Whether and where a leaf is hidden.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hidden {
    /// Shown.
    No,
    /// Hidden by its own eye or a holder's on the output page: it still runs (142 B).
    OnOutputPage,
    /// A source leaf hidden on its drawing page inside a placed page: it is not drawn there and
    /// runs only with its own assignment (criterion 13).
    InSource,
}

/// One output-capable leaf of the output page (guides, dimensions and fixtures are never passed).
#[derive(Debug, Clone, Copy)]
pub struct Leaf<'a> {
    /// The leaf first, then its holders, the clone or placement last.
    pub chain: &'a [Vec<NodeId>],
    /// Its hidden state.
    pub hidden: Hidden,
    /// Its topology.
    pub shape: LeafShape,
}

/// How a leaf stands in the job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LeafStatus {
    /// Not on the output page's job at all: a hidden source leaf with no assignment of its own.
    Absent,
    /// No assignment at any level: drawn dimmed, never runs.
    NotAssigned,
    /// Assigned; `running` are the operations it runs in (positions in the stored list).
    Assigned {
        /// The nearest rule's answer.
        assignment: Assignment,
        /// The operations it runs in, in run order.
        running: Vec<usize>,
    },
}

/// The four counts and the number of operations that run.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Summary {
    /// `Runs <N> operations`: known operations with Output on that at least one leaf runs in.
    pub operations: usize,
    /// `<M> items`: leaves that hold an assignment and run, hidden or not, each once.
    pub running: usize,
    /// `<K> items not assigned.`
    pub not_assigned: usize,
    /// `<J> items run while hidden.` (a part of `running`).
    pub runs_while_hidden: usize,
    /// `<H> items sit in operations with Output off.` (may overlap `running`).
    pub output_off: usize,
}

/// The status of `leaf` under `list`.
#[must_use]
pub fn leaf_status(list: &OperationList, leaf: &Leaf<'_>) -> LeafStatus {
    let scope = match leaf.hidden {
        Hidden::InSource => ChainScope::OwnOnly,
        Hidden::No | Hidden::OnOutputPage => ChainScope::Whole,
    };
    let Some(assignment) = list.nearest(leaf.chain, scope) else {
        return match leaf.hidden {
            Hidden::InSource => LeafStatus::Absent,
            Hidden::No | Hidden::OnOutputPage => LeafStatus::NotAssigned,
        };
    };
    let running = list
        .running(&assignment)
        .filter(|&at| {
            list.rules()[at]
                .kind
                .known()
                .is_some_and(|kind| !kind_run(kind, leaf.shape).is_empty())
        })
        .collect();
    LeafStatus::Assigned {
        assignment,
        running,
    }
}

/// The summary over `leaves`.
#[must_use]
pub fn summarize<'a>(list: &OperationList, leaves: impl IntoIterator<Item = Leaf<'a>>) -> Summary {
    let mut summary = Summary::default();
    let mut running_operations = HashSet::new();
    for leaf in leaves {
        match leaf_status(list, &leaf) {
            LeafStatus::Absent => {}
            LeafStatus::NotAssigned => summary.not_assigned += 1,
            LeafStatus::Assigned {
                assignment,
                running,
            } => {
                if !running.is_empty() {
                    summary.running += 1;
                    if leaf.hidden != Hidden::No {
                        summary.runs_while_hidden += 1;
                    }
                }
                let rules = list.rules();
                if assignment
                    .operations
                    .iter()
                    .any(|&at| !rules[at].output && rules[at].kind.known().is_some())
                {
                    summary.output_off += 1;
                }
                running_operations.extend(running);
            }
        }
    }
    summary.operations = running_operations.len();
    summary
}

#[cfg(test)]
mod tests {
    use curvyo_document_core::OperationSnapshot;

    use super::*;
    use crate::operation_nearest::tests::{id, op};

    const SHAPE: LeafShape = LeafShape::Outlines { closed: 1, open: 0 };

    fn leaf(chain: &[Vec<NodeId>], hidden: Hidden) -> Leaf<'_> {
        Leaf {
            chain,
            hidden,
            shape: SHAPE,
        }
    }

    fn list(ops: &[OperationSnapshot]) -> OperationList {
        OperationList::new(ops, true)
    }

    #[test]
    fn no_operation_means_every_leaf_is_not_assigned() {
        let chains = [vec![vec![id(1)]], vec![vec![id(2)]]];
        let summary = summarize(&list(&[]), chains.iter().map(|c| leaf(c, Hidden::No)));
        assert_eq!(
            summary,
            Summary {
                not_assigned: 2,
                ..Summary::default()
            }
        );
    }

    #[test]
    #[allow(clippy::many_single_char_names)]
    fn running_hidden_output_off_and_not_assigned_are_counted() {
        let (g, a, b, c, d, e) = (id(1), id(2), id(3), id(4), id(5), id(6));
        let ops = [
            op(0, Some("cut"), true, &[g, d]),
            op(1, Some("engrave"), false, &[b, d]),
            op(2, Some("dot_mode"), true, &[e]),
            op(3, Some("score"), true, &[]),
        ];
        let chains: [Vec<Vec<NodeId>>; 6] = [
            vec![vec![a], vec![g]],
            vec![vec![b], vec![g]],
            vec![vec![c]],
            vec![vec![d]],
            vec![vec![e]],
            vec![vec![id(7)], vec![g]],
        ];
        let hidden = [
            Hidden::No,
            Hidden::No,
            Hidden::No,
            Hidden::No,
            Hidden::No,
            Hidden::OnOutputPage,
        ];
        let summary = summarize(
            &list(&ops),
            chains.iter().zip(hidden).map(|(c, h)| leaf(c, h)),
        );
        assert_eq!(
            summary,
            Summary {
                operations: 1,
                running: 3,
                not_assigned: 1,
                runs_while_hidden: 1,
                output_off: 2,
            },
            "a, d and the hidden one under g run; b sits in Output off only; e is Unknown"
        );
    }

    #[test]
    fn a_hidden_source_leaf_runs_only_with_its_own_key_and_is_counted_hidden() {
        let (leaf_a, leaf_b, placement) = (id(1), id(2), id(3));
        let ops = [
            op(0, Some("cut_out"), true, &[placement]),
            op(1, Some("engrave"), true, &[leaf_a]),
        ];
        let l = list(&ops);
        let a = [vec![leaf_a], vec![placement]];
        let b = [vec![leaf_b], vec![placement]];
        assert!(matches!(
            leaf_status(&l, &leaf(&b, Hidden::InSource)),
            LeafStatus::Absent
        ));
        let LeafStatus::Assigned { running, .. } = leaf_status(&l, &leaf(&a, Hidden::InSource))
        else {
            panic!("assigned");
        };
        assert_eq!(running, [1], "not through the placed page's Cut-out");
        let summary = summarize(
            &l,
            [
                leaf(&a, Hidden::InSource),
                leaf(&b, Hidden::InSource),
                leaf(&b, Hidden::No),
            ],
        );
        assert_eq!(
            summary,
            Summary {
                operations: 2,
                running: 2,
                not_assigned: 0,
                runs_while_hidden: 1,
                output_off: 0,
            }
        );
    }

    #[test]
    fn an_image_in_score_only_runs_nothing() {
        let x = id(1);
        let ops = [op(0, Some("score"), true, &[x])];
        let chain = [vec![x]];
        let image = Leaf {
            chain: &chain,
            hidden: Hidden::No,
            shape: LeafShape::Image,
        };
        let summary = summarize(&list(&ops), [image]);
        assert_eq!(summary.running, 0);
        assert_eq!(summary.operations, 0);
    }
}
