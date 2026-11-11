//! The preview of the clones downstream of an edit (`specs/0049-linked-clone` criterion 11): what
//! they would show if a drag's results replaced the objects it edits, through the one resolver.

use std::collections::{HashMap, HashSet};

use crate::affine::Affine;
use crate::clone_read::{RawLeaf, resolve};
use crate::clone_transform::{CloneLinks, HeldClones, Hold, clone_matrices_after_maps};
use crate::outline_map::object_map;
use crate::path_model::{NodeId, PathSnapshot};
use crate::primitive_model::ObjectSnapshot;

/// What the clones downstream of `changed` would show if `changed` replaced those objects of
/// `objects` (a read): the live preview of a drag that edits an original (`0049` criterion 11).
/// Only the clones that follow a changed object, directly or through a chain, are resolved;
/// `None` when more than `cap` of them are not tiling instances (the preview is then left out;
/// a tiling's instances always follow live, `specs/0069-tiling` criterion 29). `held` is
/// [`Document::own_placement_clones`](crate::Document::own_placement_clones); a clone with own
/// placement missing from it is held whole. With `transform` (the drag maps its objects as a
/// whole, a Select drag), each clone gets the matrix the commit's held-back rule gives it
/// ([`crate::clone_matrices_after`] on the change of each outline): a clone that is held back
/// is not previewed, nor are the clones behind it (`specs/0057-clone-overrides` criterion 9),
/// and a tiling instance follows a scale of its source in its own cell (`0069` criterion 29). A
/// node edit previews every clone behind it.
#[must_use]
#[allow(clippy::implicit_hasher)] // the one map type every caller has
pub fn clone_previews(
    objects: &[ObjectSnapshot],
    changed: &[ObjectSnapshot],
    cap: usize,
    held: &HeldClones,
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
    let rule = if transform {
        TransformHold::of(objects, &at, changed, held)
    } else {
        TransformHold::default()
    };
    let replaced: HashMap<NodeId, &ObjectSnapshot> =
        changed.iter().map(|object| (object.id(), object)).collect();
    let mut downstream: Vec<usize> = Vec::new();
    let mut counted = 0;
    let mut seen = HashSet::new();
    let mut frontier: Vec<NodeId> = replaced.keys().copied().collect();
    while let Some(id) = frontier.pop() {
        for &i in followers.get(&id).into_iter().flatten() {
            let clone = objects[i].id();
            if !rule.stays.contains(&clone) && !replaced.contains_key(&clone) && seen.insert(i) {
                if !matches!(held.get(&clone), Some(Hold::Translation { .. })) {
                    if counted == cap {
                        return None;
                    }
                    counted += 1;
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
    let mut taken = HashSet::new();
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
                ObjectSnapshot::Sketch(sketch) => RawLeaf::Sketch(sketch.clone()),
                ObjectSnapshot::Primitive(primitive) => RawLeaf::Primitive(primitive.clone()),
                ObjectSnapshot::Clone(clone) => {
                    let mut leaf = RawLeaf::of_clone(clone);
                    if let (RawLeaf::Clone(_, link, _), Some(&matrix)) =
                        (&mut leaf, rule.followed.get(&clone.id()))
                    {
                        link.matrix = matrix;
                    }
                    leaf
                }
                ObjectSnapshot::Text(text) => RawLeaf::Text(text.clone()),
                ObjectSnapshot::Image(image) => RawLeaf::Image(image.clone()),
                ObjectSnapshot::Dimension(dimension) => RawLeaf::Dimension(dimension.clone()),
            }
        })
        .collect();
    let wanted: HashSet<NodeId> = downstream.iter().map(|&i| objects[i].id()).collect();
    Some(
        resolve(leaves)
            .leaves
            .into_iter()
            .flatten()
            .filter_map(|object| match object {
                ObjectSnapshot::Clone(clone) if wanted.contains(&clone.id()) => {
                    Some(*clone.resolved)
                }
                _ => None,
            })
            .collect(),
    )
}

/// What the held-back rule of a commit decides for the clones with own placement, read from the
/// change of each changed outline as the transform writers read it.
#[derive(Default)]
struct TransformHold {
    /// The clones whose drawn outline stays: held back, or unlinked with the outline they had.
    stays: HashSet<NodeId>,
    /// The new matrix of every tiling instance that follows the linear part of the change.
    followed: HashMap<NodeId, Affine>,
}

impl TransformHold {
    fn of(
        objects: &[ObjectSnapshot],
        at: &HashMap<NodeId, usize>,
        changed: &[ObjectSnapshot],
        held: &HeldClones,
    ) -> Self {
        let mut held = std::borrow::Cow::Borrowed(held);
        let mut links = CloneLinks::new();
        for object in objects {
            if let ObjectSnapshot::Clone(clone) = object {
                links.insert(clone.id(), (clone.chain.origin, clone.chain.matrix));
                if clone.chain.own_placement && !held.contains_key(&clone.id()) {
                    held.to_mut().insert(clone.id(), Hold::Whole);
                }
            }
        }
        if held.is_empty() {
            return Self::default();
        }
        let moved: Vec<(NodeId, Affine)> = changed
            .iter()
            .filter_map(|after| {
                let before = &objects[*at.get(&after.id())?];
                Some((after.id(), object_map(before, after)?))
            })
            .collect();
        let decided = clone_matrices_after_maps(&links, &held, &moved);
        Self {
            stays: decided
                .held_back
                .iter()
                .map(|&(id, _)| id)
                .chain(decided.held_unlinked.iter().copied())
                .collect(),
            followed: decided.followed.into_iter().collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
        let previews = clone_previews(
            &objects,
            std::slice::from_ref(&moved),
            500,
            &HeldClones::new(),
            false,
        )
        .unwrap();
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
            clone_previews(&objects, &[moved], 1, &HeldClones::new(), false),
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
            clone_previews(&objects, changed, 500, &HeldClones::new(), true)
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            clone_previews(&objects, changed, 500, &HeldClones::new(), false)
                .unwrap()
                .len(),
            3
        );
    }

    /// `0069` criterion 29, live: a tiling instance follows a scale of its source about its
    /// tiling's pivot and holds back a move; a plain clone with own placement stays.
    #[test]
    fn a_transform_drag_scales_a_tiling_instance_in_its_cell() {
        use crate::clone_read::tests_support::styled_clone;
        use crate::clone_transform::Hold;
        let own = |id: i32, at: f64| {
            styled_clone(
                id,
                1,
                Affine::translation(Vec2::new(at, 0.0)),
                crate::style_model::Style::default(),
                true,
            )
        };
        let objects = resolve_read(vec![square(1), own(2, 100.0), own(3, 200.0)]).objects;
        let ObjectSnapshot::Path(root) = &objects[0] else {
            panic!("a path")
        };
        let (min, max) = root.all_anchors().fold(
            (
                Point::new(f64::MAX, f64::MAX),
                Point::new(f64::MIN, f64::MIN),
            ),
            |(a, b), anchor| {
                let p = anchor.point;
                (
                    Point::new(a.x.min(p.x), a.y.min(p.y)),
                    Point::new(b.x.max(p.x), b.y.max(p.y)),
                )
            },
        );
        let pivot = Point::new(f64::midpoint(min.x, max.x), f64::midpoint(min.y, max.y));
        let id = |n: usize| objects[n].id();
        let mut held = HeldClones::new();
        held.insert(id(1), Hold::Translation { pivot });
        held.insert(id(2), Hold::Whole);
        let scale = Affine::translation(Vec2::new(pivot.x, pivot.y))
            .compose(Affine::scale(2.0, 1.0))
            .compose(Affine::translation(Vec2::new(-pivot.x, -pivot.y)));
        let scaled = ObjectSnapshot::Path(root.mapped(scale));
        let previews = clone_previews(&objects, std::slice::from_ref(&scaled), 0, &held, true)
            .expect("a tiling instance is not counted against the cap");
        assert_eq!(previews.len(), 1, "only the tiling instance follows");
        assert_eq!(previews[0].id, id(1));
        let want = root.mapped(Affine::translation(Vec2::new(100.0, 0.0)).compose(scale));
        for (a, b) in previews[0].all_anchors().zip(want.all_anchors()) {
            assert!((a.point.x - b.point.x).abs() < 1e-9 && (a.point.y - b.point.y).abs() < 1e-9);
        }
        let moved = ObjectSnapshot::Path(root.mapped(Affine::translation(Vec2::new(5.0, 0.0))));
        assert!(
            clone_previews(&objects, &[moved], 500, &held, true)
                .unwrap()
                .is_empty(),
            "a move is held back"
        );
    }
}
