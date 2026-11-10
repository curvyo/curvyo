//! The leaf index of the output page (ADR 0046 §7, §8; `specs/0101` `adrs.md` decision 7): every
//! output-capable leaf with its chain of keys and hidden state, built from one drawing read and
//! kept until that read changes, so an assignment recomputes only the membership and the summary.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use curvyo_document_core::{ManufacturingSnapshot, NodeId, ObjectSnapshot, PathSnapshot};
use curvyo_laser_core::operations::{Hidden, LeafShape};
use curvyo_ui_core::{Reach, object_bounds, placement_members};

use super::super::Session;
use super::super::object_cache::DocumentRead;

/// One output-capable leaf of the output page, as the nearest rule and the summary read it.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct IndexedLeaf {
    /// The leaf's own node (a placed page is its placement node).
    pub(super) chain: Vec<Vec<NodeId>>,
    /// Its hidden state (criteria 13, 16).
    pub(super) hidden: Hidden,
    /// Its topology, which decides what each kind runs.
    pub(super) shape: LeafShape,
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
    pub(super) fn output_leaves(
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
        ObjectSnapshot::Sketch(_) | ObjectSnapshot::Dimension(_)
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
    for object in read.leaves.iter().filter(|o| output_capable(o, read)) {
        if !membership.is_member(page, object.id(), object_bounds(object), Reach::Touches) {
            continue;
        }
        // A clone of a group draws its members, which are the leaves (criteria 12, 17); a clone
        // of one path draws that path, so the clone itself is the leaf.
        if let Some((origin, _)) = links.get(&object.id())
            && session.document.is_group(*origin)
        {
            out.extend(clone_member_leaves(
                session,
                read,
                &CloneOf {
                    clone: object.id(),
                    origin: *origin,
                    clone_visible: visible.contains(&object.id()),
                },
                &objects,
                &visible,
            ));
            continue;
        }
        out.push(IndexedLeaf {
            chain: session.document.key_chain(&[object.id()]),
            hidden: if visible.contains(&object.id()) {
                Hidden::No
            } else {
                Hidden::OnOutputPage
            },
            shape: shape_of(object),
        });
    }
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
        });
        for index in placement_members(&read.leaves, source, &read.guides, &membership) {
            let member = &read.leaves[index];
            if !output_capable(member, read) {
                continue;
            }
            let mut chain = session.document.key_chain(&[member.id()]);
            chain.push(vec![placement.id]);
            out.push(IndexedLeaf {
                chain,
                hidden: if visible.contains(&member.id()) {
                    Hidden::No
                } else {
                    Hidden::InSource
                },
                shape: shape_of(member),
            });
        }
    }
    out
}

/// A clone of a group, as the members' leaves need it.
struct CloneOf {
    clone: NodeId,
    origin: NodeId,
    clone_visible: bool,
}

/// The leaves of a clone's members: each member of the origin group, keyed by its path through
/// the clone (ADR 0046 §2, Frage 167 B), so a clone's own assignment reaches them through the
/// nearest rule. A member is hidden when the clone or the member is not visible.
fn clone_member_leaves(
    session: &Session,
    read: &DocumentRead,
    clone: &CloneOf,
    objects: &HashMap<NodeId, &ObjectSnapshot>,
    visible: &HashSet<NodeId>,
) -> Vec<IndexedLeaf> {
    session
        .document
        .leaves_of(clone.origin)
        .into_iter()
        .filter_map(|member| {
            let object = objects.get(&member)?;
            if !output_capable(object, read) {
                return None;
            }
            let path = session.document.clone_member_path(clone.clone, member)?;
            Some(IndexedLeaf {
                chain: session.document.key_chain(&path),
                hidden: if clone.clone_visible && visible.contains(&member) {
                    Hidden::No
                } else {
                    Hidden::OnOutputPage
                },
                shape: shape_of(object),
            })
        })
        .collect()
}

/// The topology of a leaf for the run rules. Text and primitives count as one closed outline
/// (Proposal: their outline count is not read here; the run rules differ only for open outlines).
fn shape_of(object: &ObjectSnapshot) -> LeafShape {
    match object {
        ObjectSnapshot::Path(path) => path_shape(path),
        ObjectSnapshot::Image(_) => LeafShape::Image,
        _ => LeafShape::Outlines { closed: 1, open: 0 },
    }
}

fn path_shape(path: &PathSnapshot) -> LeafShape {
    let (closed, open) =
        path.subpaths().fold(
            (0, 0),
            |(c, o), sub| {
                if sub.closed { (c + 1, o) } else { (c, o + 1) }
            },
        );
    LeafShape::Outlines { closed, open }
}
