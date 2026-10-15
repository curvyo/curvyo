//! The preview of the clones downstream of an edit (`specs/0049-linked-clone` criterion 11): what
//! they would show if a drag's results replaced the objects it edits, through the one resolver.

use std::collections::HashMap;

use crate::clone_read::{RawLeaf, resolve};
use crate::path_model::{NodeId, PathSnapshot};
use crate::primitive_model::ObjectSnapshot;

/// What the clones downstream of `changed` would show if `changed` replaced those objects of
/// `objects` (a read): the live preview of a drag that edits an original (`0049` criterion 11).
/// Only the clones that follow a changed object, directly or through a chain, are resolved;
/// `None` when there are more than `cap` of them (the preview is then left out). With
/// `transform` (the drag maps its objects as a whole, a Select drag), a clone with own placement
/// is held back: neither it nor the clones behind it are previewed (`specs/0057-clone-overrides`
/// criterion 9); a node edit previews every clone behind it.
#[must_use]
pub fn clone_previews(
    objects: &[ObjectSnapshot],
    changed: &[ObjectSnapshot],
    cap: usize,
    transform: bool,
) -> Option<Vec<PathSnapshot>> {
    let mut followers: HashMap<NodeId, Vec<usize>> = HashMap::new();
    let mut at: HashMap<NodeId, usize> = HashMap::new();
    for (i, object) in objects.iter().enumerate() {
        at.insert(object.id(), i);
        if let ObjectSnapshot::Clone(clone) = object {
            followers.entry(clone.chain.origin).or_default().push(i);
        }
    }
    let replaced: HashMap<NodeId, &ObjectSnapshot> =
        changed.iter().map(|object| (object.id(), object)).collect();
    let mut downstream: Vec<usize> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut frontier: Vec<NodeId> = replaced.keys().copied().collect();
    while let Some(id) = frontier.pop() {
        for &i in followers.get(&id).into_iter().flatten() {
            let clone = objects[i].id();
            let held = transform
                && matches!(&objects[i], ObjectSnapshot::Clone(c) if c.chain.own_placement);
            if !held && !replaced.contains_key(&clone) && seen.insert(i) {
                if downstream.len() == cap {
                    return None;
                }
                downstream.push(i);
                frontier.push(clone);
            }
        }
    }
    if downstream.is_empty() {
        return Some(Vec::new());
    }
    // The leaves the downstream clones need: themselves and every object up their chains.
    let mut needed: Vec<usize> = Vec::new();
    let mut taken = std::collections::HashSet::new();
    for &i in &downstream {
        let mut current = Some(i);
        while let Some(j) = current.filter(|j| taken.insert(*j)) {
            needed.push(j);
            current = match objects[j] {
                ObjectSnapshot::Clone(ref clone) => at.get(&clone.chain.origin).copied(),
                _ => None,
            };
        }
    }
    needed.sort_unstable();
    let leaves: Vec<RawLeaf> = needed
        .iter()
        .map(|&j| {
            let object = replaced
                .get(&objects[j].id())
                .copied()
                .unwrap_or(&objects[j]);
            match object {
                ObjectSnapshot::Path(path) => RawLeaf::Path(path.clone()),
                ObjectSnapshot::Primitive(primitive) => RawLeaf::Primitive(primitive.clone()),
                ObjectSnapshot::Clone(clone) => RawLeaf::of_clone(clone),
            }
        })
        .collect();
    let wanted: std::collections::HashSet<NodeId> =
        downstream.iter().map(|&i| objects[i].id()).collect();
    Some(
        resolve(leaves)
            .leaves
            .into_iter()
            .flatten()
            .filter_map(|object| match object {
                ObjectSnapshot::Clone(clone) if wanted.contains(&clone.id()) => {
                    Some(clone.resolved)
                }
                _ => None,
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::affine::Affine;
    use crate::clone_read::tests_support::{clone, resolve_read, square};
    use crate::units::{Point, Vec2};

    #[test]
    fn criterion_11_the_preview_resolves_the_clones_behind_an_edited_original() {
        let leaves = vec![
            square(1),
            clone(2, 1, Affine::translation(Vec2::new(100.0, 0.0))),
            clone(3, 2, Affine::IDENTITY),
            square(4),
            clone(5, 4, Affine::IDENTITY),
        ];
        let objects = resolve_read(leaves).objects;
        let ObjectSnapshot::Path(root) = &objects[0] else {
            panic!("a path")
        };
        let moved = ObjectSnapshot::Path(root.mapped(Affine::translation(Vec2::new(5.0, 5.0))));
        let previews = clone_previews(&objects, std::slice::from_ref(&moved), 500, false).unwrap();
        assert_eq!(
            previews.len(),
            2,
            "A and B, not the clone of the other square"
        );
        assert!(
            previews
                .iter()
                .all(|p| p.anchors[0].point == Point::new(105.0, 5.0))
        );
        assert_eq!(
            clone_previews(&objects, &[moved], 1, false),
            None,
            "over the cap"
        );
    }

    #[test]
    fn criterion_9_a_transform_drag_leaves_held_clones_and_their_followers_alone() {
        use crate::clone_read::tests_support::styled_clone;
        let leaves = vec![
            square(1),
            styled_clone(
                2,
                1,
                Affine::IDENTITY,
                crate::style_model::Style::default(),
                true,
            ),
            clone(3, 2, Affine::IDENTITY),
            clone(4, 1, Affine::IDENTITY),
        ];
        let objects = resolve_read(leaves).objects;
        let ObjectSnapshot::Path(root) = &objects[0] else {
            panic!("a path")
        };
        let moved = ObjectSnapshot::Path(root.mapped(Affine::translation(Vec2::new(5.0, 0.0))));
        let changed = std::slice::from_ref(&moved);
        assert_eq!(
            clone_previews(&objects, changed, 500, true).unwrap().len(),
            1
        );
        assert_eq!(
            clone_previews(&objects, changed, 500, false).unwrap().len(),
            3
        );
    }
}
