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
        Reorder::Forward => siblings[front + 1..]
            .first()
            .map(|&next| MoveTarget::Above(next)),
        Reorder::Backward => back
            .checked_sub(1)
            .map(|before| MoveTarget::Below(siblings[before])),
        Reorder::Out => parent.map(MoveTarget::Above),
        Reorder::In => siblings[front + 1..]
            .first()
            .filter(|&&next| document.is_group(next))
            .map(|&next| MoveTarget::FrontOf(Some(next))),
    }
}

/// The moves `how` makes of `nodes`, one per parent the nodes have, each keeping the relative
/// order of its nodes. Empty when nothing would move. The caller runs them in one step and checks
/// each with `Document::check_move` first.
#[must_use]
pub fn reorder_plans(document: &Document, nodes: &[NodeId], how: Reorder) -> Vec<MovePlan> {
    by_parent(document, nodes)
        .into_iter()
        .filter_map(|(parent, members)| {
            target_of(document, parent, &members, how).map(|target| MovePlan {
                nodes: members,
                target,
            })
        })
        .collect()
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
