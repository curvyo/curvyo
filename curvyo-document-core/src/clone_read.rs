//! The one read of every object with its linked clones resolved (`specs/0049-linked-clone/adrs.md`
//! decision 3, ADR 0015 §6): chains are followed with an explicit stack, each composed matrix and
//! style source (ADR 0016 §3) is computed once from its original's, and the root's outline is
//! transformed once per clone.

use std::collections::{HashMap, HashSet};

use crate::affine::Affine;
use crate::clone_codec::CloneLink;
use crate::clone_model::{CloneChain, CloneSnapshot};
use crate::path_model::{NodeId, PathSnapshot};
use crate::primitive_model::{ObjectSnapshot, PrimitiveSnapshot};
use crate::style_model::Style;

/// One leaf as stored, before any clone is resolved.
#[derive(Debug, Clone)]
pub(crate) enum RawLeaf {
    Sketch(Box<crate::SketchSnapshot>),
    /// A path or compound path: a possible root.
    Path(PathSnapshot),
    /// A primitive: never an original.
    Primitive(PrimitiveSnapshot),
    /// A clone node, its link and its overrides.
    Clone(NodeId, CloneLink, RawOverrides),
    /// A clone node whose link cannot be read (merge residue): always an orphan.
    BrokenClone(NodeId),
    /// A text object: never an original.
    Text(Box<crate::text_model::TextSnapshot>),
    /// A text clone as stored, filled from its source text after the walk (ADR 0022 §4).
    TextClone(Box<crate::text_clone_codec::TextCloneParts>),
    /// An image: never an original.
    Image(crate::image_model::ImageSnapshot),
    /// A leaf whose registers cannot be read (merge residue of a text): left out.
    Unreadable(NodeId),
}

/// What a clone node stores besides its link (ADR 0016 §1).
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct RawOverrides {
    /// The own-placement flag.
    pub(crate) own_placement: bool,
    /// Its own style when it has the `own_style` marker.
    pub(crate) own_style: Option<Style>,
}

impl RawLeaf {
    fn id(&self) -> NodeId {
        match self {
            Self::Path(path) => path.id,
            Self::Sketch(snapshot) => snapshot.id,
            Self::Primitive(primitive) => primitive.id,
            Self::Text(text) => text.id,
            Self::Image(image) => image.id,
            Self::TextClone(parts) => parts.id,
            Self::Clone(id, ..) | Self::BrokenClone(id) | Self::Unreadable(id) => *id,
        }
    }

    /// The leaf a resolved clone was read from: its link and overrides, as stored.
    pub(crate) fn of_clone(clone: &CloneSnapshot) -> Self {
        Self::Clone(
            clone.id(),
            CloneLink {
                origin: clone.chain.origin,
                matrix: clone.chain.matrix,
            },
            RawOverrides {
                own_placement: clone.chain.own_placement,
                own_style: clone.chain.own_style.then(|| clone.resolved.style.clone()),
            },
        )
    }
}

/// Every object of a document version, read once.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ObjectRead {
    /// Every object in stacking order, clones resolved; orphans are left out (not drawn, not
    /// hit, not selected, not output; `0049` criterion 35). Shared, so a cache hands it out
    /// without a copy.
    pub objects: std::sync::Arc<[ObjectSnapshot]>,
    /// The position of each object in `objects`, by id.
    pub index: HashMap<NodeId, usize>,
    /// The clones that follow each object directly, in stacking order (orphans left out).
    pub clones_of: HashMap<NodeId, Vec<NodeId>>,
    /// The clones whose chain does not end in a live path or compound path, or whose outline is
    /// not finite, in stacking order.
    pub orphans: Vec<NodeId>,
    /// The texts on each live carrier, in stacking order (`specs/0080-text-on-path`).
    pub texts_on: HashMap<NodeId, Vec<NodeId>>,
}

impl ObjectRead {
    /// The read of objects already resolved (the leaves of `Document::read_nodes`, which ran the
    /// resolver): the index and the direct clones of each, no orphans.
    #[must_use]
    pub fn from_objects(objects: std::sync::Arc<[ObjectSnapshot]>) -> Self {
        let mut clones_of: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
        let mut index = HashMap::with_capacity(objects.len());
        let mut texts_on: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
        for (i, object) in objects.iter().enumerate() {
            index.insert(object.id(), i);
            if let ObjectSnapshot::Clone(clone) = object {
                clones_of
                    .entry(clone.chain.origin)
                    .or_default()
                    .push(clone.id());
            }
            if let ObjectSnapshot::Text(text) = object
                && let Some(source) = text.source
            {
                clones_of.entry(source.node).or_default().push(text.id);
            }
            if let ObjectSnapshot::Text(text) = object
                && let Some(on_path) = &text.on_path
                && !on_path.kept
                && on_path.curve.is_some()
            {
                texts_on
                    .entry(on_path.carrier.node)
                    .or_default()
                    .push(text.id);
            }
        }
        Self {
            objects,
            index,
            clones_of,
            orphans: Vec::new(),
            texts_on,
        }
    }

    /// The object with this id, if the read holds it.
    #[must_use]
    pub fn get(&self, id: NodeId) -> Option<&ObjectSnapshot> {
        self.index.get(&id).map(|&i| &self.objects[i])
    }
}

/// What [`resolve`] gives: one entry per input leaf, in input order (`None` for an orphan), so a
/// caller that walks the tree itself can put each result back where its leaf was.
#[derive(Debug)]
pub(crate) struct Resolution {
    /// The object each leaf reads as, or `None` for an orphan.
    pub(crate) leaves: Vec<Option<ObjectSnapshot>>,
    /// The direct clones of each object, in stacking order.
    pub(crate) clones_of: HashMap<NodeId, Vec<NodeId>>,
    /// The orphans, in stacking order.
    pub(crate) orphans: Vec<NodeId>,
    /// The texts on each live carrier (`crate::text_carrier`).
    pub(crate) texts_on: HashMap<NodeId, Vec<NodeId>>,
}

impl Resolution {
    /// The read with the orphans left out of the object list.
    pub(crate) fn into_read(self) -> ObjectRead {
        let objects: std::sync::Arc<[ObjectSnapshot]> = self.leaves.into_iter().flatten().collect();
        let index = objects
            .iter()
            .enumerate()
            .map(|(i, object)| (object.id(), i))
            .collect();
        ObjectRead {
            objects,
            index,
            clones_of: self.clones_of,
            orphans: self.orphans,
            texts_on: self.texts_on,
        }
    }
}

/// Where the resolution of one leaf stands.
#[derive(Debug, Clone, Copy)]
enum State {
    /// A path: the root of its own chain.
    Root,
    /// A primitive or a broken clone: not an original.
    NotOrigin,
    /// A clone not visited yet.
    Pending,
    /// A clone on the stack of the walk in progress.
    OnStack,
    /// A resolved clone: the index of its root, its depth, its composed matrix and the index of
    /// its style source (`None`: the root).
    Resolved(usize, usize, Affine, Option<usize>),
    /// A clone that cannot be resolved.
    Orphan,
}

/// Resolves every clone of `leaves` (stacking order). No recursion: a chain or ring of any length
/// costs one stack entry per clone. Whether a clone is an orphan depends only on the leaves, not
/// on the order of the walk.
pub(crate) fn resolve(leaves: Vec<RawLeaf>) -> Resolution {
    let index: HashMap<NodeId, usize> = leaves
        .iter()
        .enumerate()
        .map(|(i, leaf)| (leaf.id(), i))
        .collect();
    let mut states: Vec<State> = leaves
        .iter()
        .map(|leaf| match leaf {
            RawLeaf::Path(_) | RawLeaf::Sketch(_) => State::Root,
            RawLeaf::Primitive(_)
            | RawLeaf::BrokenClone(_)
            | RawLeaf::Text(_)
            | RawLeaf::Image(_)
            | RawLeaf::TextClone(_)
            | RawLeaf::Unreadable(_) => State::NotOrigin,
            RawLeaf::Clone(..) => State::Pending,
        })
        .collect();
    for start in 0..leaves.len() {
        if matches!(states[start], State::Pending) {
            walk(start, &leaves, &index, &mut states);
        }
    }
    assemble(leaves, &states, &index)
}

/// Follows the chain from the clone at `start` until it meets a root, a resolved clone or a dead
/// end, then unwinds the stack, composing each matrix once.
fn walk(start: usize, leaves: &[RawLeaf], index: &HashMap<NodeId, usize>, states: &mut [State]) {
    let mut stack = vec![start];
    states[start] = State::OnStack;
    let base = loop {
        // invariant: only clones are pushed, and the top of the stack is always a clone.
        let RawLeaf::Clone(_, link, _) = &leaves[stack[stack.len() - 1]] else {
            break None;
        };
        let Some(&next) = index.get(&link.origin) else {
            break None;
        };
        match states[next] {
            State::Root => break Some((next, 0, Affine::IDENTITY, None)),
            State::Resolved(root, depth, composed, source) => {
                break Some((root, depth, composed, source));
            }
            State::Pending => {
                states[next] = State::OnStack;
                stack.push(next);
            }
            State::OnStack | State::Orphan | State::NotOrigin => break None,
        }
    };
    let mut base = base;
    while let Some(clone) = stack.pop() {
        base = base.and_then(|(root, depth, composed, source)| {
            let RawLeaf::Clone(_, link, overrides) = &leaves[clone] else {
                return None;
            };
            let own = link.matrix.compose(composed);
            let source = if overrides.own_style.is_some() {
                Some(clone)
            } else {
                source
            };
            own.is_finite().then_some((root, depth + 1, own, source))
        });
        states[clone] = match base {
            Some((root, depth, composed, source)) => State::Resolved(root, depth, composed, source),
            None => State::Orphan,
        };
    }
}

/// Builds the result from the resolved states, in input order. Each resolved clone's outline is
/// mapped once; a clone whose mapped outline is not finite is an orphan, and so is every clone
/// behind it (decided in depth order, so each original is decided before its clones).
fn assemble(leaves: Vec<RawLeaf>, states: &[State], index: &HashMap<NodeId, usize>) -> Resolution {
    let mut clones: Vec<Option<CloneSnapshot>> = vec![None; leaves.len()];
    let mut by_depth: Vec<(usize, usize)> = Vec::new();
    let mut shared: HashMap<usize, std::sync::Arc<PathSnapshot>> = HashMap::new();
    let failed = refine_sketch_roots(&leaves, states, &mut shared);
    for (i, (leaf, state)) in leaves.iter().zip(states).enumerate() {
        if let (RawLeaf::Clone(id, link, overrides), State::Resolved(root, depth, composed, source)) =
            (leaf, *state)
            && !failed.contains(&root)
            && let Some(root_path) = root_path(&leaves[root])
        {
            let root_outline = shared_outline(&mut shared, root, root_path);
            let (style_source, style) = match source.map(|s| &leaves[s]) {
                Some(RawLeaf::Clone(
                    source_id,
                    _,
                    RawOverrides {
                        own_style: Some(style),
                        ..
                    },
                )) => (*source_id, Some(style)),
                _ => (root_path.id, None),
            };
            let mut clone = CloneSnapshot::new(
                *id,
                CloneChain {
                    origin: link.origin,
                    root: root_path.id,
                    depth,
                    matrix: link.matrix,
                    composed,
                    own_placement: overrides.own_placement,
                    own_style: overrides.own_style.is_some(),
                    style_source,
                    root_outline,
                },
            );
            if let Some(style) = style {
                clone.resolved.style = style.clone();
            }
            if outline_is_finite(&clone.resolved) {
                clones[i] = Some(clone);
                by_depth.push((depth, i));
            }
        }
    }
    by_depth.sort_unstable();
    for (_, i) in by_depth {
        let RawLeaf::Clone(_, link, _) = &leaves[i] else {
            continue;
        };
        let origin_lost = index
            .get(&link.origin)
            .is_some_and(|&o| matches!(leaves[o], RawLeaf::Clone(..)) && clones[o].is_none());
        if origin_lost {
            clones[i] = None;
        }
    }
    let mut result = Resolution {
        leaves: Vec::with_capacity(leaves.len()),
        clones_of: HashMap::new(),
        orphans: Vec::new(),
        texts_on: HashMap::new(),
    };
    let mut text_clones = Vec::new();
    for (i, leaf) in leaves.into_iter().enumerate() {
        let object = match leaf {
            RawLeaf::Path(path) => Some(ObjectSnapshot::Path(path)),
            RawLeaf::Sketch(snapshot) => Some(ObjectSnapshot::Sketch(snapshot)),
            RawLeaf::Primitive(primitive) => Some(ObjectSnapshot::Primitive(primitive)),
            RawLeaf::Text(text) => Some(ObjectSnapshot::Text(text)),
            RawLeaf::Image(image) => Some(ObjectSnapshot::Image(image)),
            RawLeaf::TextClone(parts) => {
                text_clones.push((i, *parts));
                None
            }
            RawLeaf::Unreadable(_) => None,
            RawLeaf::Clone(id, ..) | RawLeaf::BrokenClone(id) => {
                if let Some(clone) = clones[i].take() {
                    result
                        .clones_of
                        .entry(clone.chain.origin)
                        .or_default()
                        .push(id);
                    Some(ObjectSnapshot::Clone(clone))
                } else {
                    result.orphans.push(id);
                    None
                }
            }
        };
        result.leaves.push(object);
    }
    // Text clones read their sources (ADR 0022 §4), then every text on path its carrier (§3).
    let filled = crate::text_clone_read::fill_clones(&mut result.leaves, text_clones);
    for (source, clone) in filled.links {
        result.clones_of.entry(source).or_default().push(clone);
    }
    result.orphans.extend(filled.orphans);
    result.texts_on = crate::text_carrier::resolve_carriers(&mut result.leaves);
    result
}

/// Whether every point and handle of `path` is a finite number.
fn outline_is_finite(path: &PathSnapshot) -> bool {
    path.all_anchors().all(|a| {
        a.point.x.is_finite()
            && a.point.y.is_finite()
            && a.handle_in.x.is_finite()
            && a.handle_in.y.is_finite()
            && a.handle_out.x.is_finite()
            && a.handle_out.y.is_finite()
    })
}

/// Leaves for the unit tests of the resolver and of the preview.
#[cfg(test)]
pub(crate) mod tests_support {
    use super::*;
    use crate::path_model::{AnchorId, NewAnchor};
    use crate::style_model::Style;
    use crate::units::{Angle, Point};

    pub(crate) fn resolve_read(leaves: Vec<RawLeaf>) -> ObjectRead {
        resolve(leaves).into_read()
    }

    pub(crate) fn id(n: i32) -> NodeId {
        NodeId::from_parts(1, n)
    }

    pub(crate) fn square(n: i32) -> RawLeaf {
        let corner =
            |k: u64, x: f64, y: f64| NewAnchor::corner(AnchorId::new(9, k), Point::new(x, y));
        RawLeaf::Path(PathSnapshot {
            id: id(n),
            closed: true,
            style: Style::default(),
            anchors: vec![
                corner(1, 0.0, 0.0),
                corner(2, 10.0, 0.0),
                corner(3, 10.0, 10.0),
                corner(4, 0.0, 10.0),
            ],
            extra_subpaths: Vec::new(),
            rotation: Angle::from_radians(0.3),
        })
    }

    pub(crate) fn clone(n: i32, origin: i32, matrix: Affine) -> RawLeaf {
        RawLeaf::Clone(
            id(n),
            CloneLink {
                origin: id(origin),
                matrix,
            },
            RawOverrides::default(),
        )
    }

    /// A clone with its own style (and optionally own placement).
    pub(crate) fn styled_clone(
        n: i32,
        origin: i32,
        matrix: Affine,
        style: Style,
        own_placement: bool,
    ) -> RawLeaf {
        RawLeaf::Clone(
            id(n),
            CloneLink {
                origin: id(origin),
                matrix,
            },
            RawOverrides {
                own_placement,
                own_style: Some(style),
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::path_model::{AnchorId, NewAnchor};
    use crate::style_model::Style;
    use crate::units::{Angle, Point, Vec2};

    use super::tests_support::{clone, id, resolve_read, square};

    fn clone_of(read: &ObjectRead, n: i32) -> &CloneSnapshot {
        read.objects
            .iter()
            .find_map(|object| match object {
                ObjectSnapshot::Clone(clone) if clone.id() == id(n) => Some(clone),
                _ => None,
            })
            .unwrap()
    }

    fn xs_ys(path: &PathSnapshot) -> (f64, f64, f64, f64) {
        let xs = path.anchors.iter().map(|a| a.point.x);
        let ys = path.anchors.iter().map(|a| a.point.y);
        (
            xs.clone().fold(f64::INFINITY, f64::min),
            xs.fold(f64::NEG_INFINITY, f64::max),
            ys.clone().fold(f64::INFINITY, f64::min),
            ys.fold(f64::NEG_INFINITY, f64::max),
        )
    }

    #[test]
    fn a_chain_composes_and_both_ways_agree() {
        // Criterion 50: O the square, M_A = translate (30, 0), M_B = scale 2.
        let m_a = Affine::translation(Vec2::new(30.0, 0.0));
        let m_b = Affine::scale(2.0, 2.0);
        let read = resolve_read(vec![square(1), clone(3, 2, m_b), clone(2, 1, m_a)]);
        let a = clone_of(&read, 2);
        let b = clone_of(&read, 3);
        assert_eq!(xs_ys(&a.resolved), (30.0, 40.0, 0.0, 10.0));
        assert_eq!(xs_ys(&b.resolved), (60.0, 80.0, 0.0, 20.0));
        assert_eq!(b.chain.composed, Affine::new(2.0, 0.0, 0.0, 2.0, 60.0, 0.0));
        assert_eq!((a.chain.depth, b.chain.depth), (1, 2));
        assert_eq!((a.chain.root, b.chain.root), (id(1), id(1)));
        assert_eq!(b.chain.origin, id(2));
        let by_link = a.resolved.mapped(m_b);
        for (x, y) in by_link.anchors.iter().zip(&b.resolved.anchors) {
            assert!((x.point.x - y.point.x).abs() < 1e-9);
            assert!((x.point.y - y.point.y).abs() < 1e-9);
        }
        assert_eq!(b.resolved.id, id(3));
        assert_eq!(b.resolved.rotation, Angle::from_radians(0.0));
        assert_eq!(read.clones_of[&id(1)], vec![id(2)]);
        assert_eq!(read.clones_of[&id(2)], vec![id(3)]);
        assert_eq!(read.orphans, Vec::<NodeId>::new());
    }

    fn blue() -> Style {
        let mut style = Style::default();
        style.fill.enabled = true;
        style.fill.color = crate::path_model::Color { r: 0, g: 0, b: 255 };
        style
    }

    #[test]
    fn a_clone_reads_the_style_of_the_nearest_own_style_clone_on_its_chain() {
        // `0057` criterion 15: B → A → O, A own style blue, B Original's; C a clone of O.
        let read = resolve_read(vec![
            square(1),
            super::tests_support::styled_clone(2, 1, Affine::IDENTITY, blue(), true),
            clone(3, 2, Affine::IDENTITY),
            clone(4, 1, Affine::IDENTITY),
        ]);
        let (a, b, c) = (clone_of(&read, 2), clone_of(&read, 3), clone_of(&read, 4));
        assert_eq!(a.resolved.style, blue());
        assert_eq!(b.resolved.style, blue());
        assert_eq!(c.resolved.style, Style::default());
        assert_eq!(
            (
                a.chain.style_source,
                b.chain.style_source,
                c.chain.style_source
            ),
            (id(2), id(2), id(1))
        );
        assert!(a.chain.own_style && !b.chain.own_style && !c.chain.own_style);
        assert!(a.chain.own_placement && !b.chain.own_placement);
        assert_eq!(
            b.chain.root_outline.style,
            Style::default(),
            "the outline stays shared"
        );
    }

    #[test]
    fn the_nearest_own_style_wins_over_one_further_down() {
        let mut red = Style::default();
        red.stroke.color = crate::path_model::Color { r: 255, g: 0, b: 0 };
        let objects = resolve_read(vec![
            super::tests_support::styled_clone(3, 2, Affine::IDENTITY, red.clone(), false),
            clone(4, 3, Affine::IDENTITY),
            super::tests_support::styled_clone(2, 1, Affine::IDENTITY, blue(), false),
            square(1),
        ]);
        assert_eq!(clone_of(&objects, 4).resolved.style, red);
        assert_eq!(clone_of(&objects, 4).chain.style_source, id(3));
        assert_eq!(clone_of(&objects, 2).resolved.style, blue());
    }

    #[test]
    fn a_resolved_clone_converts_back_to_the_leaf_it_was_read_from() {
        let leaf = super::tests_support::styled_clone(2, 1, Affine::scale(2.0, 1.0), blue(), true);
        let read = resolve_read(vec![square(1), leaf.clone()]);
        let back = RawLeaf::of_clone(clone_of(&read, 2));
        let (RawLeaf::Clone(a, la, oa), RawLeaf::Clone(b, lb, ob)) = (&leaf, &back) else {
            panic!("clones")
        };
        assert_eq!((a, la, oa), (b, lb, ob));
    }

    #[test]
    fn the_read_maps_one_outline_per_clone_never_one_per_link() {
        // Criterion 63: a chain of eight and seven direct clones, fifteen clones in all.
        let mut leaves = vec![square(1)];
        leaves.extend((2..=9).map(|k| clone(k, k - 1, Affine::scale(1.1, 1.0))));
        leaves.extend((10..=16).map(|k| clone(k, 1, Affine::IDENTITY)));
        let before = crate::clone_model::outline_transforms();
        let read = resolve_read(leaves);
        assert_eq!(crate::clone_model::outline_transforms() - before, 15);
        assert_eq!(read.objects.len(), 16);
    }

    #[test]
    fn a_finite_matrix_that_maps_to_infinity_orphans_the_clone_and_its_followers() {
        // ADR 0015 §3 as amended: 43 links of scale 1e7 compose to 1e301, finite, but an
        // anchor at 1e8 mm maps to infinity. The clone behind it scales back down and is an orphan too.
        let big = |n: i32| {
            let corner =
                |k: u64, x: f64, y: f64| NewAnchor::corner(AnchorId::new(9, k), Point::new(x, y));
            RawLeaf::Path(PathSnapshot {
                id: id(n),
                closed: true,
                style: Style::default(),
                anchors: vec![corner(1, 0.0, 0.0), corner(2, 1e8, 1e8)],
                extra_subpaths: Vec::new(),
                rotation: Angle::from_radians(0.0),
            })
        };
        let mut leaves = vec![big(0)];
        leaves.extend((1..=43).map(|k| clone(k, k - 1, Affine::scale(1e7, 1e7))));
        leaves.push(clone(44, 43, Affine::scale(1e-300, 1e-300)));
        leaves.push(clone(45, 0, Affine::IDENTITY));
        let resolution = resolve(leaves);
        assert!(
            resolution.leaves[42].is_some(),
            "42 links still map finitely"
        );
        assert!(resolution.leaves[43].is_none(), "the 43rd maps to infinity");
        assert!(
            resolution.leaves[44].is_none(),
            "and the clone behind it goes with it"
        );
        assert!(resolution.leaves[45].is_some());
        assert_eq!(resolution.orphans, vec![id(43), id(44)]);
    }

    #[test]
    fn the_result_has_one_entry_per_leaf_in_input_order() {
        let resolution = resolve(vec![
            clone(3, 99, Affine::IDENTITY),
            square(1),
            clone(2, 1, Affine::IDENTITY),
        ]);
        let ids: Vec<Option<NodeId>> = resolution
            .leaves
            .iter()
            .map(|leaf| leaf.as_ref().map(ObjectSnapshot::id))
            .collect();
        assert_eq!(ids, vec![None, Some(id(1)), Some(id(2))]);
    }

    #[test]
    fn the_read_keeps_the_stacking_order() {
        let read = resolve_read(vec![clone(2, 1, Affine::IDENTITY), square(1)]);
        let ids: Vec<NodeId> = read.objects.iter().map(ObjectSnapshot::id).collect();
        assert_eq!(ids, vec![id(2), id(1)]);
    }

    #[test]
    fn missing_wrong_kind_rings_and_their_followers_are_orphans() {
        let primitive = RawLeaf::BrokenClone(id(50));
        let read = resolve_read(vec![
            square(1),
            clone(2, 99, Affine::IDENTITY), // names nothing
            clone(3, 50, Affine::IDENTITY), // names a broken clone
            clone(4, 4, Affine::IDENTITY),  // names itself
            clone(5, 6, Affine::IDENTITY),  // a ring of two
            clone(6, 5, Affine::IDENTITY),
            clone(7, 5, Affine::IDENTITY), // follows the ring
            clone(8, 2, Affine::IDENTITY), // follows an orphan
            clone(9, 1, Affine::IDENTITY), // fine
            primitive,
        ]);
        assert_eq!(
            read.orphans,
            [2, 3, 4, 5, 6, 7, 8, 50].map(id).to_vec(),
            "every broken chain, in stacking order"
        );
        assert_eq!(read.objects.len(), 2);
        assert_eq!(read.clones_of.len(), 1);
    }

    #[test]
    fn a_non_finite_composed_matrix_orphans_the_clone_and_its_followers() {
        let huge = Affine::scale(1e200, 1e200);
        let read = resolve_read(vec![
            square(1),
            clone(2, 1, huge),
            clone(3, 2, huge),
            clone(4, 3, Affine::IDENTITY),
        ]);
        assert_eq!(read.orphans, vec![id(3), id(4)]);
        assert_eq!(
            clone_of(&read, 2).chain.composed,
            huge,
            "beyond ±1e7 still draws"
        );
    }

    #[test]
    fn the_walk_order_does_not_change_the_orphans() {
        let leaves = vec![
            square(1),
            clone(2, 3, Affine::IDENTITY),
            clone(3, 4, Affine::IDENTITY),
            clone(4, 2, Affine::IDENTITY),
            clone(5, 1, Affine::IDENTITY),
            clone(6, 5, Affine::IDENTITY),
        ];
        let mut reversed = leaves.clone();
        reversed.reverse();
        let mut forward = resolve_read(leaves).orphans;
        let mut backward = resolve_read(reversed).orphans;
        forward.sort_by_key(|n| n.counter);
        backward.sort_by_key(|n| n.counter);
        assert_eq!(forward, backward);
        assert_eq!(forward, vec![id(2), id(3), id(4)]);
    }

    #[test]
    fn a_chain_and_a_ring_of_ten_thousand_resolve_without_recursion() {
        // Criterion 39: the chain resolves in full, the ring is all orphans.
        let n = 10_000;
        let mut chain = vec![square(0)];
        // Front to back the deepest first, so each walk runs the whole remaining chain.
        chain.extend((1..=n).rev().map(|k| clone(k, k - 1, Affine::IDENTITY)));
        let read = resolve_read(chain);
        assert_eq!(read.orphans, Vec::<NodeId>::new());
        assert_eq!(clone_of(&read, n).chain.depth, 10_000);

        let ring: Vec<RawLeaf> = (1..=n)
            .map(|k| clone(k, if k == n { 1 } else { k + 1 }, Affine::IDENTITY))
            .collect();
        let read = resolve_read(ring);
        assert_eq!(read.orphans.len(), 10_000);
        assert_eq!(read.objects.len(), 0);
    }
}

fn root_path(leaf: &RawLeaf) -> Option<&PathSnapshot> {
    match leaf {
        RawLeaf::Path(path) => Some(path),
        RawLeaf::Sketch(sketch) => Some(&sketch.resolved),
        _ => None,
    }
}

/// Refines one analytic root for the most magnified chain, independently of visibility.
fn refine_sketch_roots(
    leaves: &[RawLeaf],
    states: &[State],
    shared: &mut HashMap<usize, std::sync::Arc<PathSnapshot>>,
) -> HashSet<usize> {
    let mut stretches: HashMap<usize, f64> = HashMap::new();
    for state in states {
        if let State::Resolved(root, _, map, _) = state
            && matches!(leaves[*root], RawLeaf::Sketch(_))
        {
            // Frobenius norm conservatively bounds linear stretch. Placement's norm is
            // applied separately in outline_scaled, so no transform is baked twice.
            let stretch = (map.a * map.a + map.b * map.b + map.c * map.c + map.d * map.d).sqrt();
            stretches
                .entry(*root)
                .and_modify(|value| *value = value.max(stretch))
                .or_insert(stretch);
        }
    }
    let mut failed = HashSet::new();
    for (root, stretch) in stretches {
        let RawLeaf::Sketch(sketch) = &leaves[root] else {
            continue;
        };
        match crate::sketch_outline::outline_scaled(
            sketch.id,
            &sketch.solution,
            sketch.placement,
            &sketch.resolved.style,
            stretch,
        ) {
            Ok(outline) => {
                shared.insert(root, std::sync::Arc::new(outline));
            }
            Err(_) => {
                failed.insert(root);
            }
        }
    }
    failed
}

fn shared_outline(
    shared: &mut HashMap<usize, std::sync::Arc<PathSnapshot>>,
    root: usize,
    path: &PathSnapshot,
) -> std::sync::Arc<PathSnapshot> {
    shared
        .entry(root)
        .or_insert_with(|| std::sync::Arc::new(path.clone()))
        .clone()
}
