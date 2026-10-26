//! The Loro value shape of a compound path's further outlines
//! (`specs/0016-boolean-operations/adrs.md`, "compound path = option A").
//!
//! A path's first outline stays in the keys [`crate::path_codec`] owns
//! (`closed`, `anchors`); this module owns the one optional key that holds
//! the others:
//!
//! ```text
//! meta map of a path:
//!   extra_subpaths : movable list, optional       absent = one-outline path
//!     each element a map:
//!       closed  : bool                            LWW register
//!       anchors : movable list of anchor maps     same schema as `anchors`
//! ```
//!
//! A writer never writes an empty list; a reader treats an empty list like an
//! absent one. Every anchor of every outline has its own unique id, so a node
//! can be named by `(NodeId, AnchorId)` without an outline index.

use std::collections::HashMap;

use loro::{Container, LoroMap, LoroMovableList, LoroValue, ValueOrContainer};

use crate::path_codec::{
    KEY_ANCHORS, KEY_CLOSED, anchor_map_at, anchors_container, push_anchor, read_anchor_id,
    read_anchor_snapshot, read_closed, validate_anchor, write_closed,
};
use crate::path_model::{AnchorId, NewAnchor, SubpathSnapshot};

/// The key of the optional list of further outlines.
pub(crate) const KEY_EXTRA_SUBPATHS: &str = "extra_subpaths";

/// The movable list of further outlines, or `None` for an ordinary path.
fn extra_list(meta: &LoroMap) -> Option<LoroMovableList> {
    match meta.get(KEY_EXTRA_SUBPATHS) {
        Some(ValueOrContainer::Container(Container::MovableList(list))) => Some(list),
        _ => None,
    }
}

/// The map of the further outline at `index`.
///
/// # Panics
/// Does not panic in practice: every call site takes `index` from the same
/// list's length, and [`validate_extra_subpaths`] confirmed the element shape
/// for a document read from bytes.
fn subpath_map_at(list: &LoroMovableList, index: usize) -> LoroMap {
    match list.get(index) {
        Some(ValueOrContainer::Container(Container::Map(map))) => map,
        _ => panic!("invariant violated: extra_subpaths element is not a map"),
    }
}

fn subpath_anchors(map: &LoroMap) -> LoroMovableList {
    match map.get(KEY_ANCHORS) {
        Some(ValueOrContainer::Container(Container::MovableList(list))) => list,
        _ => panic!("invariant violated: subpath `anchors` is not a movable list"),
    }
}

/// The map of every outline, the first outline's first, in storage order: the meta map itself for
/// the first outline, the `extra_subpaths` element for each further one. Each holds the outline's
/// `closed` and `anchors` under the same two keys, so a command that edits an outline reads and
/// writes it the same way whichever outline it is.
pub(crate) fn outline_maps(meta: &LoroMap) -> Vec<LoroMap> {
    let mut maps = vec![meta.clone()];
    if let Some(extra) = extra_list(meta) {
        maps.extend((0..extra.len()).map(|i| subpath_map_at(&extra, i)));
    }
    maps
}

/// Every outline's anchor list, the first outline's first, in storage order.
pub(crate) fn all_anchor_lists(meta: &LoroMap) -> Vec<LoroMovableList> {
    outline_maps(meta).iter().map(anchors_container).collect()
}

/// One anchor's place: the outline that holds it (its number in storage order, `0` for the first),
/// that outline's map and anchor list, and the anchor's index in that list.
pub(crate) struct AnchorAt {
    /// The outline's number in storage order, `0` for the first outline.
    pub(crate) outline: usize,
    /// The map that holds the outline's `closed` and `anchors`.
    pub(crate) map: LoroMap,
    /// The outline's anchor list.
    pub(crate) list: LoroMovableList,
    /// The anchor's index in `list`.
    pub(crate) index: usize,
}

/// Where every anchor of a path is: the outline lists, and for each anchor id its outline and its
/// index in it. Built once per command with one pass over the anchors, so a command that resolves
/// thousands of anchor ids stays linear (scanning for each id would be quadratic, and a boolean
/// result has thousands of anchors). Lookup only: no iteration order is ever used.
pub(crate) struct AnchorPositions {
    maps: Vec<LoroMap>,
    lists: Vec<LoroMovableList>,
    positions: HashMap<AnchorId, (usize, usize)>,
}

impl AnchorPositions {
    /// Whether an anchor `id` is in any outline.
    pub(crate) fn contains(&self, id: AnchorId) -> bool {
        self.positions.contains_key(&id)
    }

    /// The list and index of the anchor `id`, in whichever outline it is.
    pub(crate) fn get(&self, id: AnchorId) -> Option<(LoroMovableList, usize)> {
        let &(outline, index) = self.positions.get(&id)?;
        Some((self.lists[outline].clone(), index))
    }

    /// The outline that holds the anchor `id`, and its index there.
    pub(crate) fn at(&self, id: AnchorId) -> Option<AnchorAt> {
        let &(outline, index) = self.positions.get(&id)?;
        Some(AnchorAt {
            outline,
            map: self.maps[outline].clone(),
            list: self.lists[outline].clone(),
            index,
        })
    }

    /// How many outlines the path has.
    pub(crate) fn outline_count(&self) -> usize {
        self.lists.len()
    }

    /// The anchor list of outline number `outline`.
    pub(crate) fn list(&self, outline: usize) -> &LoroMovableList {
        &self.lists[outline]
    }
}

/// Reads the id of every anchor of every outline of `meta` once.
pub(crate) fn anchor_positions(meta: &LoroMap) -> AnchorPositions {
    let maps = outline_maps(meta);
    let lists: Vec<LoroMovableList> = maps.iter().map(anchors_container).collect();
    let mut positions = HashMap::with_capacity(lists.iter().map(LoroMovableList::len).sum());
    for (outline, list) in lists.iter().enumerate() {
        for index in 0..list.len() {
            if let Some(id) = read_anchor_id(&anchor_map_at(list, index)) {
                positions.entry(id).or_insert((outline, index));
            }
        }
    }
    AnchorPositions {
        maps,
        lists,
        positions,
    }
}

/// Removes the outlines numbered `doomed` (storage order, `0` for the first) from a path that keeps
/// at least one (`specs/0055-compound-path-node-editing/adrs.md`, "outline removal and
/// promotion"): a further outline is deleted from `extra_subpaths`; when the first outline goes, the
/// first remaining further outline's `closed` and anchors (ids kept) are written into the first
/// outline's keys and its element deleted. When no further outline is left the key is deleted, which
/// is exactly an ordinary path. The one place Delete, Join and the Pen's connect remove an outline.
///
/// # Panics
/// Does not panic in practice: every number names an outline of `meta` (the caller took it from
/// [`anchor_positions`] of the same map), and at least one outline stays.
pub(crate) fn remove_outlines(meta: &LoroMap, doomed: &[usize]) {
    let mut doomed: Vec<usize> = doomed.to_vec();
    doomed.sort_unstable();
    doomed.dedup();
    if doomed.is_empty() {
        return;
    }
    // invariant: a path with an outline to remove besides the first has `extra_subpaths`, and the
    // caller keeps at least one outline, so a removed first outline has a further one to promote.
    #[allow(clippy::unwrap_used)]
    let extra = extra_list(meta).unwrap();
    for &outline in doomed.iter().rev().filter(|&&outline| outline > 0) {
        // invariant: `outline - 1` is an index of this list, removed from the highest down.
        #[allow(clippy::unwrap_used)]
        extra.delete(outline - 1, 1).unwrap();
    }
    if doomed[0] == 0 {
        let promoted = subpath_map_at(&extra, 0);
        let anchors = subpath_anchors(&promoted);
        let snapshots: Vec<NewAnchor> = (0..anchors.len())
            .map(|k| read_anchor_snapshot(&anchor_map_at(&anchors, k)))
            .collect();
        let closed = read_closed(&promoted);
        let first = crate::path_codec::insert_anchors_container(meta);
        for anchor in &snapshots {
            push_anchor(&first, anchor);
        }
        write_closed(meta, closed);
        // invariant: index 0 was just read from this list.
        #[allow(clippy::unwrap_used)]
        extra.delete(0, 1).unwrap();
    }
    if extra.is_empty() {
        // invariant: the key was just read from this attached map.
        #[allow(clippy::unwrap_used)]
        meta.delete(KEY_EXTRA_SUBPATHS).unwrap();
    }
}

/// Inserts a new open or closed outline so that it becomes outline number `outline` (storage order,
/// at least `1`: the first outline is never replaced this way), creating `extra_subpaths` when the
/// path has none. Split at a node puts the second part of an outline right after it.
///
/// # Panics
/// Does not panic in practice: `outline - 1` is at most the list's length (the caller passes the
/// number of an existing outline plus one).
pub(crate) fn insert_outline(meta: &LoroMap, outline: usize, anchors: &[NewAnchor], closed: bool) {
    let list = extra_list_or_new(meta);
    // invariant: see above; inserting a container into an attached list cannot fail otherwise.
    #[allow(clippy::unwrap_used)]
    let map = list.insert_container(outline - 1, LoroMap::new()).unwrap();
    write_outline(&map, anchors, closed);
}

/// Appends outlines after every outline of a path, creating `extra_subpaths` when the path has
/// none; writes nothing for an empty slice (a writer never writes an empty list). A new path gets
/// its further outlines this way, and Join across objects moves the other object's outlines in.
pub(crate) fn append_outlines(meta: &LoroMap, outlines: &[(Vec<NewAnchor>, bool)]) {
    if outlines.is_empty() {
        return;
    }
    let list = extra_list_or_new(meta);
    for (anchors, closed) in outlines {
        // invariant: pushing a container onto an attached list cannot fail.
        #[allow(clippy::unwrap_used)]
        let map = list.push_container(LoroMap::new()).unwrap();
        write_outline(&map, anchors, *closed);
    }
}

/// The `extra_subpaths` list, created empty when the path has none (the caller fills it at once,
/// so no empty list is ever committed).
fn extra_list_or_new(meta: &LoroMap) -> LoroMovableList {
    extra_list(meta).unwrap_or_else(|| {
        // invariant: inserting a container under a key just found absent cannot fail.
        #[allow(clippy::unwrap_used)]
        meta.insert_container(KEY_EXTRA_SUBPATHS, LoroMovableList::new())
            .unwrap()
    })
}

/// Writes `closed` and a fresh `anchors` list into a just created outline map.
fn write_outline(map: &LoroMap, anchors: &[NewAnchor], closed: bool) {
    write_closed(map, closed);
    // invariant: inserting a container under a fresh key of an attached map cannot fail.
    #[allow(clippy::unwrap_used)]
    let list = map
        .insert_container(KEY_ANCHORS, LoroMovableList::new())
        .unwrap();
    for anchor in anchors {
        push_anchor(&list, anchor);
    }
}

/// The number of anchors over all outlines.
pub(crate) fn total_anchor_count(meta: &LoroMap) -> usize {
    all_anchor_lists(meta)
        .iter()
        .map(LoroMovableList::len)
        .sum()
}

/// Replaces the further outlines of a path (or of a primitive becoming one) by `outlines`; an
/// empty slice leaves none.
pub(crate) fn replace_extra_subpaths(meta: &LoroMap, outlines: &[(Vec<NewAnchor>, bool)]) {
    if meta.get(KEY_EXTRA_SUBPATHS).is_some() {
        // invariant: deleting a key just confirmed present on an attached map cannot fail.
        #[allow(clippy::unwrap_used)]
        meta.delete(KEY_EXTRA_SUBPATHS).unwrap();
    }
    append_outlines(meta, outlines);
}

/// The further outlines as a read model; empty for an ordinary path.
pub(crate) fn read_extra_subpaths(meta: &LoroMap) -> Vec<SubpathSnapshot> {
    let Some(extra) = extra_list(meta) else {
        return Vec::new();
    };
    (0..extra.len())
        .map(|i| {
            let map = subpath_map_at(&extra, i);
            let anchors = subpath_anchors(&map);
            SubpathSnapshot {
                closed: read_closed(&map),
                anchors: (0..anchors.len())
                    .map(|k| read_anchor_snapshot(&anchor_map_at(&anchors, k)))
                    .collect(),
            }
        })
        .collect()
}

/// Whether a present `extra_subpaths` has the shape this module writes: a
/// movable list of maps, each with a boolean `closed` and an `anchors` movable
/// list whose elements are anchor maps with a valid id; an outline has at least
/// one anchor (a single anchor is valid: a foreign file may hold one,
/// `specs/0054-combine-open-paths` criterion 30). Absent is valid. Anything else is damage
/// (`OpenError::Damaged`), checked at open so the readers above can trust the
/// shape.
pub(crate) fn validate_extra_subpaths(meta: &LoroMap) -> bool {
    match meta.get(KEY_EXTRA_SUBPATHS) {
        None => true,
        Some(ValueOrContainer::Container(Container::MovableList(list))) => {
            (0..list.len()).all(|i| match list.get(i) {
                Some(ValueOrContainer::Container(Container::Map(map))) => {
                    // Every writer writes the state; open or closed is never guessed for an
                    // outline of a compound path (`0054` criterion 30).
                    let closed_is_bool = matches!(
                        map.get(KEY_CLOSED),
                        Some(ValueOrContainer::Value(LoroValue::Bool(_)))
                    );
                    closed_is_bool
                        && match map.get(KEY_ANCHORS) {
                            // An outline without an anchor is damage: a writer never writes one.
                            Some(ValueOrContainer::Container(Container::MovableList(anchors))) => {
                                !anchors.is_empty()
                                    && (0..anchors.len()).all(|k| validate_anchor(&anchors, k))
                            }
                            _ => false,
                        }
                }
                _ => false,
            })
        }
        Some(_) => false,
    }
}
