//! The branches of one scope's list, derived from the branch marks its stack machines left
//! (`specs/0042-history-branches` criteria 1 to 3, 7 and 28). Nothing here is stored.

use std::collections::HashMap;

use super::stack_machine::BranchMark;
use super::step_id::StepRef;
use super::step_index::Position;

/// A branch of the Document scope's or an Object scope's list: steps that were taken back and then
/// left by a newer step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryBranch {
    /// Its identity: the newest of its steps in this list.
    pub id: StepRef,
    /// The step of this list it forked from; `None` when it forked before the first listed step.
    pub fork: Option<StepRef>,
    /// Its rows as indices into the list (0 is the newest row), newest first.
    pub rows: Vec<usize>,
    /// The row of the fork point.
    pub fork_row: Option<usize>,
}

/// One row of a scope's list as the derivation reads it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Listed {
    /// The step.
    pub step: StepRef,
    /// Where it sits in the log.
    pub position: Position,
    /// The branch it is on, if it is on one.
    pub mark: Option<BranchMark>,
}

/// The branches of a list given oldest first, newest branch first. The fork point of a branch is
/// the newest listed step, outside the branch, that is not newer than the step its machine forked
/// from: in the Document scope that step itself, in an Object scope the object's last step before
/// it.
pub(crate) fn derive(
    list: &[Listed],
    position_of: impl Fn(StepRef) -> Option<Position>,
) -> Vec<HistoryBranch> {
    let end = list.len().saturating_sub(1);
    let mut groups: Vec<(BranchMark, Vec<usize>)> = Vec::new();
    let mut by_key: HashMap<StepRef, usize> = HashMap::new();
    for (at, listed) in list.iter().enumerate() {
        if let Some(mark) = listed.mark {
            let group = *by_key.entry(mark.key).or_insert_with(|| {
                groups.push((mark, Vec::new()));
                groups.len() - 1
            });
            groups[group].1.push(at);
        }
    }
    let mut branches: Vec<HistoryBranch> = groups
        .into_iter()
        .filter_map(|(mark, members)| {
            let oldest = *members.first()?;
            let newest = *members.last()?;
            let fork_at = mark.fork.and_then(&position_of).and_then(|fork| {
                (0..oldest).rev().find(|at| {
                    list[*at].position <= fork && list[*at].mark.is_none_or(|m| m.key != mark.key)
                })
            });
            Some(HistoryBranch {
                id: list[newest].step,
                fork: fork_at.map(|at| list[at].step),
                rows: members.iter().rev().map(|at| end - at).collect(),
                fork_row: fork_at.map(|at| end - at),
            })
        })
        .collect();
    branches.sort_unstable_by_key(|branch| branch.rows.first().copied());
    branches
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step(n: i32) -> StepRef {
        StepRef::new(1, n)
    }

    fn position(n: i32) -> Position {
        (u32::try_from(n).unwrap_or_default(), 1, n)
    }

    fn listed(n: i32, mark: Option<(i32, Option<i32>)>) -> Listed {
        Listed {
            step: step(n),
            position: position(n),
            mark: mark.map(|(key, fork)| BranchMark {
                key: step(key),
                fork: fork.map(step),
            }),
        }
    }

    #[allow(clippy::unnecessary_wraps)]
    fn by_position(n: StepRef) -> Option<Position> {
        Some(position(n.counter()))
    }

    #[test]
    fn a_branch_is_named_by_its_newest_step_and_forks_from_its_machines_fork_point() {
        // s1, s2 and s3 on a branch from s1, s4.
        let list = [
            listed(1, None),
            listed(2, Some((3, Some(1)))),
            listed(3, Some((3, Some(1)))),
            listed(4, None),
        ];
        let branches = derive(&list, by_position);
        assert_eq!(
            branches,
            [HistoryBranch {
                id: step(3),
                fork: Some(step(1)),
                rows: vec![1, 2],
                fork_row: Some(3),
            }]
        );
    }

    #[test]
    fn in_an_object_scope_the_fork_is_the_objects_last_step_before_the_machines_fork_point() {
        // The machine forked at s5, a step on another object; this list has s2, then the branch.
        let list = [
            listed(2, None),
            listed(7, Some((8, Some(5)))),
            listed(8, Some((8, Some(5)))),
            listed(9, None),
        ];
        let branches = derive(&list, by_position);
        assert_eq!(branches[0].fork, Some(step(2)));
        assert_eq!(branches[0].fork_row, Some(3));
    }

    #[test]
    fn a_branch_without_a_listed_fork_point_forks_from_the_start() {
        let list = [listed(1, Some((1, None))), listed(2, None)];
        let branches = derive(&list, by_position);
        assert_eq!(branches[0].fork, None);
        assert_eq!(branches[0].fork_row, None);
        assert_eq!(branches[0].rows, vec![1]);
    }

    #[test]
    fn branches_come_newest_first_and_a_fork_inside_a_branch_points_into_it() {
        // s1 s2 s3; s3 left from s2 by s4; s2 and s4 left from s1 by s5.
        let list = [
            listed(1, None),
            listed(2, Some((4, Some(1)))),
            listed(3, Some((3, Some(2)))),
            listed(4, Some((4, Some(1)))),
            listed(5, None),
        ];
        let branches = derive(&list, by_position);
        assert_eq!(branches.len(), 2);
        assert_eq!(branches[0].id, step(4));
        assert_eq!(branches[0].rows, vec![1, 3]);
        assert_eq!(branches[0].fork_row, Some(4));
        assert_eq!(branches[1].id, step(3));
        assert_eq!(branches[1].fork, Some(step(2)));
        assert_eq!(branches[1].fork_row, Some(3));
    }
}
