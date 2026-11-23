//! The leaf index of the output page (ADR 0046 §7, §8; `specs/0101` `adrs.md` decision 7): every
//! output-capable leaf with its chain of keys and hidden state, built from one drawing read and
//! kept until that read changes, so an assignment recomputes only the membership and the summary.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use curvyo_document_core::{ManufacturingSnapshot, NodeId, ObjectSnapshot, PathSnapshot};
use curvyo_laser_core::operations::{Hidden, LeafShape};
use curvyo_ui_core::{PageMembership, Reach, object_bounds, placement_members};

use super::super::Session;
use super::super::object_cache::DocumentRead;

/// One output-capable leaf of the output page, as the nearest rule and the summary read it.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::session) struct IndexedLeaf {
    /// The leaf's own node (a placed page is its placement node).
    pub(in crate::session) chain: Vec<Vec<NodeId>>,
    /// Its hidden state (criteria 13, 16).
    pub(in crate::session) hidden: Hidden,
    /// Its topology, which decides what each kind runs.
    pub(in crate::session) shape: LeafShape,
    /// Where its geometry is read (`0029`'s job input).
    pub(in crate::session) source: LeafSource,
}

/// Where an indexed leaf's geometry is read: indexes into the drawing read's `leaves`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::session) enum LeafSource {
    /// An item of the output page, in document space.
    Own(usize),
    /// The leaf at `member` in the picture of the clone at `clone` (a clone of a group, text or
    /// image, or of such a clone), by its place: the ids of a picture can repeat (a clone inside
    /// a cloned group draws its original's ids).
    CloneMember { clone: usize, member: usize },
    /// A member of a cloned group that its clone does not draw (hidden inside the group): it
    /// counts, but has no geometry.
    NotDrawn,
    /// A placed page's frame, its placement node.
    PlacedFrame(NodeId),
    /// The member at `index` of the placed page `placement`.
    PlacedMember { index: usize, placement: NodeId },
    /// The leaf at `member` in the picture of the clone at `clone`, a member of the placed page
    /// `placement`.
    PlacedCloneMember {
        clone: usize,
        member: usize,
        placement: NodeId,
    },
}

/// The index of one output page, for the drawing read it was built from.
#[derive(Debug)]
struct Entry {
    read: Arc<DocumentRead>,
    manufacturing: NodeId,
    leaves: Arc<[IndexedLeaf]>,
}

/// The leaf index cache of the session: valid while the drawing read and the laser page are the
/// ones it was built for, so a change of assignment or order reads it again without a drawing read.
#[derive(Debug, Default)]
pub(in crate::session) struct LeafCache(RefCell<Option<Entry>>);

impl Session {
    /// The leaves of the active laser page's output page, from the cache when the drawing read
    /// and the page are unchanged.
    pub(in crate::session) fn output_leaves(
        &self,
        manufacturing: &ManufacturingSnapshot,
    ) -> Arc<[IndexedLeaf]> {
        let read = self.document_read();
        let cache = &self.operations.leaves.0;
        if let Some(entry) = cache.borrow().as_ref()
            && Arc::ptr_eq(&entry.read, &read)
            && entry.manufacturing == manufacturing.id
        {
            return Arc::clone(&entry.leaves);
        }
        let leaves: Arc<[IndexedLeaf]> = build_leaves(self, &read, manufacturing).into();
        *cache.borrow_mut() = Some(Entry {
            read,
            manufacturing: manufacturing.id,
            leaves: Arc::clone(&leaves),
        });
        leaves
    }
}

/// Whether the object is an output-capable leaf kind at all: sketches and dimensions are not,
/// nor are guides (criterion 10).
fn output_capable(object: &ObjectSnapshot, read: &DocumentRead) -> bool {
    !matches!(
        object,
        ObjectSnapshot::Sketch(_)
            | ObjectSnapshot::Dimension(_)
            | ObjectSnapshot::RegistrationMark(_)
    ) && !read.guides.contains(&object.id())
}

/// Every leaf the output page runs: its own items (placed pages included, by page membership) and
/// the members of each placed page (inner rows, keyed by their source object and placed on the
/// placement, ADR 0046 §2 and §7).
fn build_leaves(
    session: &Session,
    read: &DocumentRead,
    manufacturing: &ManufacturingSnapshot,
) -> Vec<IndexedLeaf> {
    let Some(page) = manufacturing.page.as_ref() else {
        return Vec::new();
    };
    let membership = session.page_membership();
    let visible: HashSet<NodeId> = read.visible.iter().map(ObjectSnapshot::id).collect();
    let objects: HashMap<NodeId, &ObjectSnapshot> =
        read.leaves.iter().map(|o| (o.id(), o)).collect();
    let links = session.document.clone_links();
    let mut out = Vec::new();
    for (at, object) in read.leaves.iter().enumerate() {
        if !output_capable(object, read) {
            continue;
        }
        if !membership.is_member(page, object.id(), object_bounds(object), Reach::Touches) {
            continue;
        }
        // A clone of a group draws its members, which are the leaves (criteria 12, 17); a clone
        // of one path draws that path, so the clone itself is the leaf. Any other picture clone
        // (of a text, an image or another picture clone) draws its picture's leaves, each keyed
        // by the clone (`specs/0123` criterion 46).
        if let Some((origin, _)) = links.get(&object.id())
            && session.document.is_group(*origin)
        {
            out.extend(clone_member_leaves(
                session,
                read,
                &CloneOf {
                    at,
                    clone: object.id(),
                    origin: *origin,
                    clone_visible: visible.contains(&object.id()),
                },
                &objects,
                &visible,
            ));
            continue;
        }
        if object.picture().is_some() {
            out.extend(own_picture_leaves(session, object, at, &visible));
            continue;
        }
        out.push(own_leaf(session, object, at, &visible));
    }
    out.extend(placed_leaves(
        session,
        read,
        manufacturing,
        &membership,
        &visible,
    ));
    out
}

/// The leaves of the page's own picture clone `object` (of a text, an image or another picture
/// clone), at `at` in the drawing read: its picture's leaves, each keyed by the clone.
fn own_picture_leaves(
    session: &Session,
    object: &ObjectSnapshot,
    at: usize,
    visible: &HashSet<NodeId>,
) -> Vec<IndexedLeaf> {
    let chain = session.document.key_chain(&[object.id()]);
    let hidden = if visible.contains(&object.id()) {
        Hidden::No
    } else {
        Hidden::OnOutputPage
    };
    picture_leaves(
        object,
        |_| Some((chain.clone(), hidden)),
        |member| LeafSource::CloneMember { clone: at, member },
    )
}

/// The leaf of the page's own item `object`, at `at` in the drawing read.
fn own_leaf(
    session: &Session,
    object: &ObjectSnapshot,
    at: usize,
    visible: &HashSet<NodeId>,
) -> IndexedLeaf {
    IndexedLeaf {
        chain: session.document.key_chain(&[object.id()]),
        hidden: if visible.contains(&object.id()) {
            Hidden::No
        } else {
            Hidden::OnOutputPage
        },
        shape: shape_of(object),
        source: LeafSource::Own(at),
    }
}

/// The leaves of the placed pages: each placement's frame and the members of its source page.
fn placed_leaves(
    session: &Session,
    read: &DocumentRead,
    manufacturing: &ManufacturingSnapshot,
    membership: &PageMembership,
    visible: &HashSet<NodeId>,
) -> Vec<IndexedLeaf> {
    let mut out = Vec::new();
    let pages = session.document.pages();
    for placement in &manufacturing.placements {
        let Some(source) = pages.iter().find(|p| p.id == placement.page) else {
            continue;
        };
        // The placed page's own item: its frame, keyed by its placement node (ADR 0046 §2).
        out.push(IndexedLeaf {
            chain: vec![vec![placement.id]],
            hidden: Hidden::No,
            shape: LeafShape::Outlines { closed: 1, open: 0 },
            source: LeafSource::PlacedFrame(placement.id),
        });
        for index in placement_members(&read.leaves, source, &read.guides, membership) {
            let member = &read.leaves[index];
            if !output_capable(member, read) {
                continue;
            }
            let mut chain = session.document.key_chain(&[member.id()]);
            chain.push(vec![placement.id]);
            let hidden = if visible.contains(&member.id()) {
                Hidden::No
            } else {
                Hidden::InSource
            };
            // A picture clone on the source page draws its picture's leaves (criterion 46).
            if member.picture().is_some() {
                out.extend(picture_leaves(
                    member,
                    |_| Some((chain.clone(), hidden)),
                    |at| LeafSource::PlacedCloneMember {
                        clone: index,
                        member: at,
                        placement: placement.id,
                    },
                ));
                continue;
            }
            out.push(IndexedLeaf {
                chain,
                hidden,
                shape: shape_of(member),
                source: LeafSource::PlacedMember {
                    index,
                    placement: placement.id,
                },
            });
        }
    }
    out
}

/// A clone of a group, as the members' leaves need it.
struct CloneOf {
    at: usize,
    clone: NodeId,
    origin: NodeId,
    clone_visible: bool,
}

/// The leaves of a clone's members: each leaf the clone draws, keyed by the path through the
/// clone to the member of the origin group it comes from (ADR 0046 §2, Frage 167 B), so a
/// clone's own assignment reaches them through the nearest rule; then each member it does not
/// draw. A member is hidden when the clone or the member is not visible.
fn clone_member_leaves(
    session: &Session,
    read: &DocumentRead,
    clone: &CloneOf,
    objects: &HashMap<NodeId, &ObjectSnapshot>,
    visible: &HashSet<NodeId>,
) -> Vec<IndexedLeaf> {
    let hidden = |member: NodeId| {
        if clone.clone_visible && visible.contains(&member) {
            Hidden::No
        } else {
            Hidden::OnOutputPage
        }
    };
    let key = |member: NodeId| {
        let object = objects.get(&member)?;
        if !output_capable(object, read) {
            return None;
        }
        let path = session.document.clone_member_path(clone.clone, member)?;
        Some((session.document.key_chain(&path), hidden(member)))
    };
    let Some(object) = read.leaves.get(clone.at) else {
        return Vec::new();
    };
    let mut out = picture_leaves(object, key, |member| LeafSource::CloneMember {
        clone: clone.at,
        member,
    });
    let drawn: HashSet<NodeId> = object
        .picture_members()
        .map(|members| members.iter().copied().collect())
        .unwrap_or_default();
    for member in session.document.leaves_of(clone.origin) {
        if drawn.contains(&member) {
            continue;
        }
        let Some((chain, hidden)) = key(member) else {
            continue;
        };
        out.push(IndexedLeaf {
            chain,
            hidden,
            shape: objects
                .get(&member)
                .map_or(LeafShape::Outlines { closed: 1, open: 0 }, |o| shape_of(o)),
            source: LeafSource::NotDrawn,
        });
    }
    out
}

/// The leaves of the picture of the clone `object`, one per picture leaf by its place, each with
/// the chain and hidden state `key` gives for the member it comes from (`None` leaves it out)
/// and the source `source` gives for its place (`specs/0123` criterion 46).
fn picture_leaves(
    object: &ObjectSnapshot,
    mut key: impl FnMut(NodeId) -> Option<(Vec<Vec<NodeId>>, Hidden)>,
    source: impl Fn(usize) -> LeafSource,
) -> Vec<IndexedLeaf> {
    let (Some(picture), Some(members)) = (object.picture(), object.picture_members()) else {
        return Vec::new();
    };
    picture
        .iter()
        .zip(members)
        .enumerate()
        .filter_map(|(at, (leaf, &member))| {
            let (chain, hidden) = key(member)?;
            Some(IndexedLeaf {
                chain,
                hidden,
                shape: shape_of(leaf),
                source: source(at),
            })
        })
        .collect()
}

/// The topology of a leaf for the run rules. Text and primitives count as one closed outline
/// (Proposal: their outline count is not read here; the run rules differ only for open outlines).
/// A leaf whose fill paints a gradient is a `Gradient` shape, the raster item of an Image
/// operation (`specs/0123` criterion 3; ADR 0069 §13).
fn shape_of(object: &ObjectSnapshot) -> LeafShape {
    let (closed, open) = match object {
        ObjectSnapshot::Image(_) => return LeafShape::Image,
        ObjectSnapshot::Path(path) => path_counts(path),
        _ => (1, 0),
    };
    if object.style().fill.painted_gradient().is_some() {
        LeafShape::Gradient { closed, open }
    } else {
        LeafShape::Outlines { closed, open }
    }
}

fn path_counts(path: &PathSnapshot) -> (usize, usize) {
    path.subpaths().fold(
        (0, 0),
        |(c, o), sub| {
            if sub.closed { (c + 1, o) } else { (c, o + 1) }
        },
    )
}
