//! Turning a reorder gesture into moves: the canvas keys Home, End, Page Up and Page Down, the
//! Alt+arrow keys of the Layers list, and a drop on a row (`specs/0050-layers-and-objects-panel`
//! criteria 29 to 33). The rules of criterion 30 live once in `Document::check_move`
//! (`adrs.md`, the note of 2026-10-12 on decisions 8 and 14); this module only says where the
//! nodes go and names a refusal in the maker's words.

use std::collections::HashSet;

use curvyo_document_core::{Document, MovePlan, MoveRefusal, MoveTarget, NodeId};

/// A reorder among siblings, or out of and into a container.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reorder {
    /// To the front of the siblings (Home, Alt+Home).
    Front,
    /// To the back of the siblings (End, Alt+End).
    Back,
    /// One place toward the front (Page Up, Alt+Up).
    Forward,
    /// One place toward the back (Page Down, Alt+Down).
    Backward,
    /// Out of the container, to the place directly in front of it (Alt+Left).
    Out,
    /// Into the container directly in front, as its front-most children (Alt+Right).
    In,
}

/// The nodes of `nodes` grouped by their parent, each group in stacking order (back to front).
fn by_parent(document: &Document, nodes: &[NodeId]) -> Vec<(Option<NodeId>, Vec<NodeId>)> {
    let wanted: HashSet<NodeId> = nodes.iter().copied().collect();
    let mut parents: Vec<Option<NodeId>> = Vec::new();
    for &node in nodes {
        let parent = document.parent_node(node);
        if !parents.contains(&parent) {
            parents.push(parent);
        }
    }
    parents
        .into_iter()
        .map(|parent| {
            let members = document
                .children(parent)
                .into_iter()
                .filter(|id| wanted.contains(id))
                .collect();
            (parent, members)
        })
        .collect()
}

/// The target of `how` for `members` under `parent`, or `None` when they are already where
/// `how` would put them.
fn target_of(
    document: &Document,
    parent: Option<NodeId>,
    members: &[NodeId],
    how: Reorder,
) -> Option<MoveTarget> {
    let siblings = document.children(parent);
    let moved: HashSet<NodeId> = members.iter().copied().collect();
    let positions: Vec<usize> = siblings
        .iter()
        .enumerate()
        .filter(|(_, id)| moved.contains(id))
        .map(|(at, _)| at)
        .collect();
    let (&back, &front) = (positions.first()?, positions.last()?);
    let contiguous = front - back + 1 == positions.len();
    match how {
        Reorder::Front => {
            (!(contiguous && front + 1 == siblings.len())).then_some(MoveTarget::FrontOf(parent))
        }
        Reorder::Back => (!(contiguous && back == 0)).then_some(MoveTarget::BackOf(parent)),
        // Steps are `steps`'.
        Reorder::Forward | Reorder::Backward => None,
        Reorder::Out => parent.map(MoveTarget::Above),
        Reorder::In => siblings[front + 1..]
            .first()
            .filter(|&&next| document.is_group(next))
            .map(|&next| MoveTarget::FrontOf(Some(next))),
    }
}

/// The one-place steps of `members` under `parent` toward the front (`forward`) or the back: each
/// selected node passes the unselected sibling next to it, a node at the edge or behind another
/// selected node that cannot move stays (criteria 32 and 33, amended 2026-10-12). One plan per
/// node that moves; they are independent, because each names its own node and the unselected
/// sibling it passes, which no other plan moves.
fn steps(
    document: &Document,
    parent: Option<NodeId>,
    members: &[NodeId],
    forward: bool,
) -> Vec<MovePlan> {
    let mut order = document.children(parent);
    let moved: HashSet<NodeId> = members.iter().copied().collect();
    let mut plans = Vec::new();
    let len = order.len();
    // Front-most first when stepping forward, back-most first when stepping back, so a node
    // that cannot move blocks the selected node behind it.
    let indices: Vec<usize> = if forward {
        (0..len).rev().collect()
    } else {
        (0..len).collect()
    };
    for at in indices {
        let node = order[at];
        if !moved.contains(&node) {
            continue;
        }
        let next = if forward {
            at + 1
        } else {
            match at.checked_sub(1) {
                Some(before) => before,
                None => continue,
            }
        };
        let Some(&passed) = order.get(next) else {
            continue;
        };
        if moved.contains(&passed) {
            continue;
        }
        plans.push(MovePlan {
            nodes: vec![node],
            target: if forward {
                MoveTarget::Above(passed)
            } else {
                MoveTarget::Below(passed)
            },
        });
        order.swap(at, next);
    }
    plans
}

/// The moves `how` makes of `nodes`: per parent the nodes have, one move that keeps their
/// relative order, or for a one-place step one move per node that steps (criterion 32). Empty
/// when nothing would move. The caller runs them in one step and checks each with
/// `Document::check_move` first; the plans are independent of each other.
#[must_use]
pub fn reorder_plans(document: &Document, nodes: &[NodeId], how: Reorder) -> Vec<MovePlan> {
    by_parent(document, nodes)
        .into_iter()
        .flat_map(|(parent, members)| match how {
            Reorder::Forward | Reorder::Backward => {
                steps(document, parent, &members, how == Reorder::Forward)
            }
            _ => target_of(document, parent, &members, how)
                .map(|target| MovePlan {
                    nodes: members,
                    target,
                })
                .into_iter()
                .collect(),
        })
        .collect()
}

/// "Already at the front" or "Already at the back": a step or a move to the edge that changes
/// nothing is said, never silent (criterion 32); `None` for the other reorders.
#[must_use]
pub const fn unchanged_text(how: Reorder) -> Option<&'static str> {
    match how {
        Reorder::Front | Reorder::Forward => Some("Already at the front"),
        Reorder::Back | Reorder::Backward => Some("Already at the back"),
        Reorder::Out | Reorder::In => None,
    }
}

/// Why a move is refused, in the maker's words (criterion 30): the tooltip of a refused drop
/// and the notice of a refused key.
#[must_use]
pub const fn move_refusal_text(refusal: MoveRefusal) -> &'static str {
    match refusal {
        MoveRefusal::Locked | MoveRefusal::LockedTarget => "Locked",
        MoveRefusal::LayerNotTopLevel => "Layers stay at the top level",
        MoveRefusal::TooDeep => "Groups can be nested 32 levels deep",
        MoveRefusal::IntoItself => "Cannot go into itself",
        MoveRefusal::NotAContainer => "Only layers and groups hold objects",
        MoveRefusal::Nothing | MoveRefusal::NoSuchNode => "Nothing to move",
    }
}

#[cfg(test)]
mod tests {
    use curvyo_document_core::{Point, RectBounds};

    use super::*;

    fn rect(document: &Document, x: f64) -> NodeId {
        document.create_rect(RectBounds::from_corners(
            Point::new(x, 0.0),
            Point::new(x + 10.0, 10.0),
        ))
    }

    fn run(document: &Document, nodes: &[NodeId], how: Reorder) {
        for plan in reorder_plans(document, nodes, how) {
            document.move_nodes(&plan).unwrap();
        }
    }

    #[test]
    #[allow(clippy::many_single_char_names)]
    fn front_back_forward_and_backward_move_among_siblings() {
        let document = Document::new(1);
        let ids: Vec<NodeId> = (0..5).map(|i| rect(&document, f64::from(i))).collect();
        let [a, b, c, d, e] = ids[..] else { panic!() };
        run(&document, &[b], Reorder::Forward);
        assert_eq!(document.children(None), vec![a, c, b, d, e]);
        run(&document, &[b], Reorder::Backward);
        assert_eq!(document.children(None), vec![a, b, c, d, e]);
        run(&document, &[d, b], Reorder::Front);
        assert_eq!(document.children(None), vec![a, c, e, b, d]);
        run(&document, &[d, b], Reorder::Back);
        assert_eq!(document.children(None), vec![b, d, a, c, e]);
    }

    /// Criteria 32 and 33 (amended 2026-10-12): a selection with gaps steps node by node, the
    /// gaps are kept, a node at the edge stays, and no node passes another selected node.
    #[test]
    #[allow(clippy::many_single_char_names)]
    fn a_selection_with_gaps_steps_node_by_node() {
        let document = Document::new(1);
        let ids: Vec<NodeId> = (0..6).map(|i| rect(&document, f64::from(i))).collect();
        let [a, b, c, d, e, f] = ids[..] else {
            panic!()
        };
        run(&document, &[a, c], Reorder::Forward);
        assert_eq!(document.children(None), vec![b, a, d, c, e, f]);
        run(&document, &[e, f], Reorder::Forward);
        assert_eq!(
            document.children(None),
            vec![b, a, d, c, e, f],
            "the front-most stays, and e does not pass f"
        );
        assert_eq!(reorder_plans(&document, &[e, f], Reorder::Forward), vec![]);
        run(&document, &[b, d, f], Reorder::Backward);
        assert_eq!(document.children(None), vec![b, d, a, c, f, e]);
    }

    #[test]
    fn nothing_moves_where_the_nodes_already_are() {
        let document = Document::new(1);
        let ids: Vec<NodeId> = (0..3).map(|i| rect(&document, f64::from(i))).collect();
        assert_eq!(reorder_plans(&document, &[ids[2]], Reorder::Front), vec![]);
        assert_eq!(
            reorder_plans(&document, &[ids[2]], Reorder::Forward),
            vec![]
        );
        assert_eq!(reorder_plans(&document, &[ids[0]], Reorder::Back), vec![]);
        assert_eq!(
            reorder_plans(&document, &[ids[0]], Reorder::Backward),
            vec![]
        );
        assert_eq!(reorder_plans(&document, &[ids[0]], Reorder::Out), vec![]);
    }

    #[test]
    fn out_and_in_cross_one_container_boundary() {
        let document = Document::new(1);
        let a = rect(&document, 0.0);
        let layer = document.new_layer(None);
        let b = rect(&document, 20.0);
        document
            .move_nodes(&MovePlan {
                nodes: vec![b],
                target: MoveTarget::FrontOf(Some(layer)),
            })
            .unwrap();
        run(&document, &[a], Reorder::In);
        assert_eq!(document.children(Some(layer)), vec![b, a]);
        run(&document, &[a], Reorder::Out);
        assert_eq!(document.children(None), vec![layer, a]);
        assert_eq!(document.children(Some(layer)), vec![b]);
    }

    #[test]
    fn nodes_of_several_layers_move_within_each_layer() {
        let document = Document::new(1);
        let cut = document.new_layer(None);
        let engrave = document.new_layer(None);
        let ids: Vec<NodeId> = (0..4).map(|i| rect(&document, f64::from(i))).collect();
        for (pair, layer) in [(&ids[0..2], cut), (&ids[2..4], engrave)] {
            document
                .move_nodes(&MovePlan {
                    nodes: pair.to_vec(),
                    target: MoveTarget::FrontOf(Some(layer)),
                })
                .unwrap();
        }
        let plans = reorder_plans(&document, &[ids[0], ids[2]], Reorder::Front);
        assert_eq!(plans.len(), 2);
        run(&document, &[ids[0], ids[2]], Reorder::Front);
        assert_eq!(document.children(Some(cut)), vec![ids[1], ids[0]]);
        assert_eq!(document.children(Some(engrave)), vec![ids[3], ids[2]]);
    }

    #[test]
    fn every_refusal_has_words() {
        assert_eq!(
            move_refusal_text(MoveRefusal::LayerNotTopLevel),
            "Layers stay at the top level"
        );
        assert_eq!(move_refusal_text(MoveRefusal::LockedTarget), "Locked");
        assert_eq!(
            move_refusal_text(MoveRefusal::TooDeep),
            "Groups can be nested 32 levels deep"
        );
    }
}
