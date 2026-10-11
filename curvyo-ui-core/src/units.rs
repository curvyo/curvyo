//! The objects the Select tool sees at one level of the document: the direct children of the
//! context, each a leaf or a group (`specs/0023-groups` criteria 8 to 10).
//!
//! The selection stays a set of leaf ids (the Select tool's box, handles, move, nudge, typed
//! values, style and the Node tool all work on leaves); a group is selected when all of its leaves
//! are. A [`Units`] says which leaves belong together at the current level, so a click, a
//! Shift-click, a marquee, a lasso and the Alt-click cycle select and toggle a group as one object.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use curvyo_document_core::{Document, NodeId};

/// One object of the context that has more than one way to be reached: a group with its leaves.
#[derive(Debug, Clone, PartialEq, Eq)]
struct GroupUnit {
    id: NodeId,
    leaves: Arc<[NodeId]>,
}

/// The grouping of the leaves under one context. A leaf that is not under a group of the context
/// is its own unit and is not listed, so a document without groups costs nothing.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Units {
    /// For each leaf inside a group of the context, that group.
    group_of: HashMap<NodeId, GroupUnit>,
}

/// A selected object of the context, as the commands and the panel see it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectedUnit {
    /// The object's id: the group's id for a group, the leaf's own id for a leaf.
    pub id: NodeId,
    /// Whether it is a group.
    pub is_group: bool,
    /// Its leaves in paint order: just itself for a leaf.
    pub leaves: Vec<NodeId>,
}

impl Units {
    /// The units of `context` (`None` is the document root) as `document` holds them now.
    #[must_use]
    pub fn build(document: &Document, context: Option<NodeId>) -> Self {
        let mut group_of = HashMap::new();
        for (child, leaves) in document.child_leaves(context) {
            if leaves.len() == 1 && leaves[0] == child {
                continue;
            }
            let unit = GroupUnit {
                id: child,
                leaves: leaves.iter().copied().collect(),
            };
            for leaf in leaves {
                group_of.insert(leaf, unit.clone());
            }
        }
        Self { group_of }
    }

    /// Whether no leaf is inside a group of the context.
    #[must_use]
    pub fn is_flat(&self) -> bool {
        self.group_of.is_empty()
    }

    /// The object of the context that `leaf` belongs to: its group, else itself.
    #[must_use]
    pub fn unit_of(&self, leaf: NodeId) -> NodeId {
        self.group_of.get(&leaf).map_or(leaf, |unit| unit.id)
    }

    /// The group of the context that holds `leaf`, if there is one.
    #[must_use]
    pub fn group_of(&self, leaf: NodeId) -> Option<NodeId> {
        self.group_of.get(&leaf).map(|unit| unit.id)
    }

    /// Every leaf that is selected, selected, or deselected together with `leaf`: the leaves of
    /// its group, else just `leaf`.
    #[must_use]
    pub fn leaves_with<'a>(&'a self, leaf: &'a NodeId) -> &'a [NodeId] {
        self.group_of
            .get(leaf)
            .map_or(std::slice::from_ref(leaf), |unit| &unit.leaves)
    }

    /// The leaves of the object of the context `unit`: those of the group of that id, else the
    /// leaf `unit` itself.
    #[must_use]
    pub fn leaves_of_unit(&self, unit: NodeId) -> Vec<NodeId> {
        self.group_of
            .values()
            .find(|group| group.id == unit)
            .map_or_else(|| vec![unit], |group| group.leaves.to_vec())
    }

    /// The objects of the context that hold at least one of the selected `leaves`, in the order
    /// their first selected leaf appears in `leaves`. A group counts once however many of its
    /// leaves are selected.
    #[must_use]
    pub fn selected(&self, leaves: &[NodeId]) -> Vec<SelectedUnit> {
        let mut found: Vec<SelectedUnit> = Vec::new();
        let mut seen: HashSet<NodeId> = HashSet::new();
        for &leaf in leaves {
            let id = self.unit_of(leaf);
            if !seen.insert(id) {
                continue;
            }
            found.push(match self.group_of.get(&leaf) {
                Some(unit) => SelectedUnit {
                    id,
                    is_group: true,
                    leaves: unit.leaves.to_vec(),
                },
                None => SelectedUnit {
                    id,
                    is_group: false,
                    leaves: vec![leaf],
                },
            });
        }
        found
    }

    /// The distinct leaves of everything in `leaves`' units: the selection a click on a leaf
    /// makes, widened to whole groups, in paint order of the first group member.
    #[must_use]
    pub fn widen(&self, leaves: &[NodeId]) -> Vec<NodeId> {
        let mut widened = Vec::with_capacity(leaves.len());
        for &leaf in leaves {
            for &member in self.leaves_with(&leaf) {
                if !widened.contains(&member) {
                    widened.push(member);
                }
            }
        }
        widened
    }
}

#[cfg(test)]
mod tests {
    use curvyo_document_core::{Document, Length, Point, RectBounds};

    use super::*;

    fn rect(document: &Document, x: f64) -> NodeId {
        document.create_rect(RectBounds {
            origin: Point::new(x, 0.0),
            width: Length::from_mm(10.0),
            height: Length::from_mm(10.0),
        })
    }

    #[test]
    fn a_document_without_groups_is_flat() {
        let document = Document::new(1);
        let a = rect(&document, 0.0);
        let units = Units::build(&document, None);
        assert!(units.is_flat());
        assert_eq!(units.unit_of(a), a);
        assert_eq!(units.leaves_with(&a), &[a]);
        assert_eq!(
            units.selected(&[a]),
            vec![SelectedUnit {
                id: a,
                is_group: false,
                leaves: vec![a]
            }]
        );
    }

    #[test]
    fn at_the_root_a_group_is_one_unit_however_deep_its_leaves_lie() {
        let document = Document::new(1);
        let (a, b, c, d) = (
            rect(&document, 0.0),
            rect(&document, 20.0),
            rect(&document, 40.0),
            rect(&document, 60.0),
        );
        let inner = document.group_objects(&[b, c]).unwrap();
        let outer = document.group_objects(&[a, inner]).unwrap();
        let units = Units::build(&document, None);
        assert_eq!(units.unit_of(c), outer);
        assert_eq!(units.unit_of(d), d);
        assert_eq!(units.leaves_with(&b), &[a, b, c]);
        assert_eq!(units.widen(&[c]), vec![a, b, c]);
        assert_eq!(units.leaves_of_unit(outer), vec![a, b, c]);
        let selected = units.selected(&[c, d, a]);
        assert_eq!(selected.len(), 2, "the group counts once");
        assert!(selected[0].is_group && selected[0].id == outer);
        assert!(!selected[1].is_group);
    }

    #[test]
    fn inside_a_group_its_children_are_the_units() {
        let document = Document::new(1);
        let (a, b, c) = (
            rect(&document, 0.0),
            rect(&document, 20.0),
            rect(&document, 40.0),
        );
        let inner = document.group_objects(&[b, c]).unwrap();
        let outer = document.group_objects(&[a, inner]).unwrap();
        let units = Units::build(&document, Some(outer));
        assert_eq!(units.unit_of(a), a, "a leaf child is its own unit");
        assert_eq!(units.unit_of(b), inner);
        assert_eq!(units.group_of(c), Some(inner));
        assert_eq!(units.group_of(a), None);
    }
}
