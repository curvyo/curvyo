//! The keys a group assignment writes and the group's tick state (ADR 0056 §1, `specs/0111`
//! criteria 9 and 10), over the operation list the session already holds.

use curvyo_document_core::NodeId;

use crate::operation_nearest::{ChainScope, OperationList};

/// The tick of a group in one operation row, derived from the group's leaves by the nearest rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupTick {
    /// Every leaf runs in the operation.
    Checked,
    /// Some leaves do: the row reads "Group `<n>` in part".
    Mixed,
    /// None does (also a group without leaves).
    Unchecked,
}

/// The keys to add to the operation at position `op` of `list` so every leaf of `group` runs in
/// it: the group's own key and the key of every node of `inner` (the group's nodes at any depth,
/// placements included) that already holds a key on any operation, so the nearest rule does not
/// shadow it. Keys the operation holds already are left out; empty means nothing changes.
#[must_use]
pub fn group_assign_keys(
    group: NodeId,
    inner: &[NodeId],
    list: &OperationList,
    op: usize,
) -> Vec<NodeId> {
    std::iter::once(group)
        .chain(
            inner
                .iter()
                .copied()
                .filter(|&node| !list.operations_of(&[node]).is_empty()),
        )
        .filter(|&node| !list.operations_of(&[node]).contains(&op))
        .collect()
}

/// The keys to remove from the operation at position `op` of `list` when `group` is unticked:
/// the group's own key and every key of `inner` on that operation. Memberships in other
/// operations stay.
#[must_use]
pub fn group_unassign_keys(
    group: NodeId,
    inner: &[NodeId],
    list: &OperationList,
    op: usize,
) -> Vec<NodeId> {
    std::iter::once(group)
        .chain(inner.iter().copied())
        .filter(|&node| list.operations_of(&[node]).contains(&op))
        .collect()
}

/// The tick of a group in the operation at position `op`, from the chains of its leaves (each
/// leaf first, the group and its holders after, as the nearest rule reads them).
#[must_use]
pub fn group_tick(leaf_chains: &[Vec<Vec<NodeId>>], list: &OperationList, op: usize) -> GroupTick {
    let assigned = leaf_chains
        .iter()
        .filter(|chain| {
            list.nearest(chain, ChainScope::Whole)
                .is_some_and(|assignment| assignment.operations.contains(&op))
        })
        .count();
    match assigned {
        0 => GroupTick::Unchecked,
        n if n == leaf_chains.len() => GroupTick::Checked,
        _ => GroupTick::Mixed,
    }
}

#[cfg(test)]
mod tests {
    use curvyo_document_core::OperationSnapshot;

    use super::*;
    use crate::operation_nearest::tests::{id, op};

    /// Applies `keys` (added, or removed with `add` false) to operation `at`.
    fn apply(ops: &mut [OperationSnapshot], at: usize, keys: &[NodeId], add: bool) {
        for &key in keys {
            let members = &mut ops[at].members;
            members.retain(|path| path != &vec![key]);
            if add {
                members.push(vec![key]);
            }
        }
    }

    #[test]
    fn a_group_press_reaches_members_with_their_own_key_and_keeps_their_other_memberships() {
        // Spec criterion 9: group G with letters A, B; B in Engrave 1; select G, press Cut.
        let (g, a, b) = (id(1), id(2), id(3));
        let mut ops = vec![
            op(0, Some("engrave"), true, &[b]),
            op(1, Some("cut"), true, &[]),
        ];
        let chains = [vec![vec![a], vec![g]], vec![vec![b], vec![g]]];
        let list = OperationList::new(&ops, false);
        assert_eq!(group_tick(&chains, &list, 0), GroupTick::Mixed);
        assert_eq!(group_tick(&chains, &list, 1), GroupTick::Unchecked);
        let keys = group_assign_keys(g, &[a, b], &list, 1);
        assert_eq!(keys, [g, b]);
        apply(&mut ops, 1, &keys, true);
        let list = OperationList::new(&ops, false);
        assert_eq!(group_tick(&chains, &list, 1), GroupTick::Checked);
        assert_eq!(
            list.operations_of(&[b]),
            [0, 1],
            "B still runs in Engrave 1"
        );
        assert_eq!(group_assign_keys(g, &[a, b], &list, 1), [] as [NodeId; 0]);
        // The mixed Engrave tick adds the rest by the same rule.
        let rest = group_assign_keys(g, &[a, b], &list, 0);
        assert_eq!(rest, [g]);
        apply(&mut ops, 0, &rest, true);
        let list = OperationList::new(&ops, false);
        assert_eq!(group_tick(&chains, &list, 0), GroupTick::Checked);
        // Unticking Cut removes the group key and B's Cut key, not B's Engrave key.
        let gone = group_unassign_keys(g, &[a, b], &list, 1);
        assert_eq!(gone, [g, b]);
        apply(&mut ops, 1, &gone, false);
        let list = OperationList::new(&ops, false);
        assert_eq!(group_tick(&chains, &list, 1), GroupTick::Unchecked);
        assert_eq!(list.operations_of(&[b]), [0]);
    }

    #[test]
    fn a_group_of_unassigned_leaves_writes_one_key_and_an_empty_group_reads_unchecked() {
        let (g, a, inner) = (id(1), id(2), id(3));
        let ops = [op(0, Some("score"), true, &[])];
        let list = OperationList::new(&ops, false);
        assert_eq!(group_assign_keys(g, &[inner, a], &list, 0), [g]);
        assert_eq!(group_tick(&[], &list, 0), GroupTick::Unchecked);
        assert_eq!(
            group_unassign_keys(g, &[inner, a], &list, 0),
            [] as [NodeId; 0]
        );
    }

    #[test]
    fn a_nested_member_with_its_own_key_is_reached_at_any_depth() {
        let (g, h, deep) = (id(1), id(2), id(3));
        let ops = [
            op(0, Some("engrave"), true, &[deep]),
            op(1, Some("cut"), true, &[]),
        ];
        let list = OperationList::new(&ops, false);
        assert_eq!(group_assign_keys(g, &[h, deep], &list, 1), [g, deep]);
        let chains = [vec![vec![deep], vec![h], vec![g]]];
        assert_eq!(group_tick(&chains, &list, 0), GroupTick::Checked);
    }
}
