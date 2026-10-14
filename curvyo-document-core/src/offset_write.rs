//! The one write of an offset (`specs/0038-path-offset` criteria 16, 20, 21, 22, 42): new objects
//! placed directly below or above their source with its complete style, and originals changed in
//! place keeping their id, name, style and stacking place, all in one commit. It computes no
//! geometry; the caller passes the result.

use std::collections::HashSet;

use loro::{LoroTree, TreeID, TreeParentId};

use crate::corner_radii_codec::write_corner_radii_if_changed;
use crate::document::{Document, OBJECTS_TREE};
use crate::objects::ObjectEditError;
use crate::path_codec::{self, insert_anchors_container, leaf_exists, push_anchor};
use crate::path_model::{AnchorId, NodeId, SubpathSnapshot};
use crate::paths::tree_id_of;
use crate::primitive_model::ObjectSnapshot;
use crate::primitive_model::Shape;
use crate::replace::{PieceOutlines, check_new_anchor_ids};
use crate::shape_codec::{self, SHAPE_ELLIPSE, SHAPE_POLYGON, SHAPE_RECT, SHAPE_STAR};
use crate::style_codec;
use crate::subpath_codec::replace_extra_subpaths;
use crate::units::Angle;

/// The geometry an offset writes for one object.
#[derive(Debug, Clone, PartialEq)]
pub enum OffsetGeometry {
    /// A path of these outlines, the first in the ordinary keys and the rest as extra subpaths,
    /// with rotation 0.
    Path(PieceOutlines),
    /// A primitive of this kind and these fields, with this rotation.
    Primitive {
        /// The primitive's kind and fields, in its own frame.
        shape: Shape,
        /// Its rotation.
        rotation: Angle,
    },
}

/// One part of an offset's write.
#[derive(Debug, Clone, PartialEq)]
pub enum OffsetWrite {
    /// A new object with a copy of `source`'s complete style, directly below `source` in the
    /// stacking order (`below`) or directly above it.
    Add {
        /// The object the copy comes from.
        source: NodeId,
        /// Below (an outset) or above (an inset).
        below: bool,
        /// The copy's geometry.
        geometry: OffsetGeometry,
    },
    /// `target` keeps its id, name, style and place; only its geometry changes. A primitive given
    /// a path becomes a path in place.
    Change {
        /// The object changed.
        target: NodeId,
        /// Its new geometry.
        geometry: OffsetGeometry,
    },
}

impl Document {
    /// Writes an offset in **one commit** labelled `label` and returns, per write in order, the
    /// new object's id or the changed object's id.
    ///
    /// Several copies of one source sit next to it in the order given: copies below it in the
    /// order given from the top down, copies above it from the bottom up.
    ///
    /// Every id is resolved and every anchor id checked before the first write, so a refusal
    /// changes nothing.
    ///
    /// # Errors
    /// [`ObjectEditError::NoSuchObject`] if a source or target no longer exists;
    /// [`ObjectEditError::CloneOutOfRange`] if a changed object would push a clone that follows it
    /// past the limits of `0049` criterion 17;
    /// [`ObjectEditError::NoOutlines`] if there is no write, or a path has no outline or an
    /// outline no anchor; [`ObjectEditError::AnchorIds`] if two anchors share an id or an id
    /// belongs to an object that keeps its anchors; [`ObjectEditError::InvalidConversion`] if a
    /// path would be changed into a primitive or a primitive into another kind.
    ///
    /// # Panics
    /// Does not panic in practice: every write goes to a tree node and meta map this call just
    /// created or resolved.
    pub fn write_offset(
        &self,
        writes: &[OffsetWrite],
        label: &str,
    ) -> Result<Vec<NodeId>, ObjectEditError> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        validate(&tree, writes)?;
        self.offset_clone_limits(writes)?;

        tree.enable_fractional_index(0);
        let mut ids = Vec::with_capacity(writes.len());
        // The last copy placed below and above each source, so the next one goes next to it.
        let mut lowest: Vec<(TreeID, TreeID)> = Vec::new();
        let mut highest: Vec<(TreeID, TreeID)> = Vec::new();
        for write in writes {
            match write {
                OffsetWrite::Add {
                    source,
                    below,
                    geometry,
                } => {
                    let source_id = tree_id_of(*source);
                    // invariant: resolved above.
                    #[allow(clippy::unwrap_used)]
                    let style = crate::clone_write::leaf_style(
                        &tree,
                        *source,
                        &tree.get_meta(source_id).unwrap(),
                    );
                    let created = create_object(&tree, geometry, &style);
                    let placed = if *below { &mut lowest } else { &mut highest };
                    let next_to = placed
                        .iter()
                        .find(|(s, _)| *s == source_id)
                        .map_or(source_id, |(_, last)| *last);
                    // invariant: both nodes are live; the fractional index is on.
                    #[allow(clippy::unwrap_used)]
                    if *below {
                        tree.mov_before(created, next_to).unwrap();
                    } else {
                        tree.mov_after(created, next_to).unwrap();
                    }
                    placed.retain(|(s, _)| *s != source_id);
                    placed.push((source_id, created));
                    ids.push(NodeId::from_parts(created.peer, created.counter));
                }
                OffsetWrite::Change { target, geometry } => {
                    // invariant: resolved above.
                    #[allow(clippy::unwrap_used)]
                    let meta = tree.get_meta(tree_id_of(*target)).unwrap();
                    // A clone is consumed like any operand (`0049` criterion 33): it becomes a
                    // path in place with its root's style, and the clones behind it follow it.
                    if crate::clone_codec::is_clone_meta(&meta) {
                        let style = crate::clone_write::leaf_style(&tree, *target, &meta);
                        crate::clone_write::clear_clone_keys(&meta);
                        style_codec::write_style(&meta, &style);
                    }
                    change_geometry(&meta, geometry);
                    ids.push(*target);
                }
            }
        }
        self.commit_with_label(label);
        Ok(ids)
    }
}

impl Document {
    /// Criterion 17 of `0049` for an offset: a changed object that clones follow, directly or
    /// through a chain, must not push one of them past the limits. Reads nothing when no clone
    /// follows a changed object.
    fn offset_clone_limits(&self, writes: &[OffsetWrite]) -> Result<(), ObjectEditError> {
        let links = self.clone_links();
        let origins: HashSet<NodeId> = links.values().map(|&(origin, _)| origin).collect();
        let changed: Vec<ObjectSnapshot> = writes
            .iter()
            .filter_map(|write| match write {
                OffsetWrite::Change {
                    target,
                    geometry: OffsetGeometry::Path(outlines),
                } if origins.contains(target) => {
                    let mut path = match self.object(*target)? {
                        ObjectSnapshot::Path(path) => path,
                        ObjectSnapshot::Clone(clone) => clone.resolved,
                        ObjectSnapshot::Primitive(_) => return None,
                    };
                    path.id = *target;
                    let (first, rest) = outlines.split_first()?;
                    (path.anchors, path.closed) = (first.0.clone(), first.1);
                    path.extra_subpaths = rest
                        .iter()
                        .map(|(anchors, closed)| SubpathSnapshot {
                            closed: *closed,
                            anchors: anchors.clone(),
                        })
                        .collect();
                    Some(ObjectSnapshot::Path(path))
                }
                _ => None,
            })
            .collect();
        if changed.is_empty() {
            return Ok(());
        }
        self.clone_limits_hold(&links, &changed)
            .map_err(ObjectEditError::CloneOutOfRange)
    }
}

/// Every check of [`Document::write_offset`] before its first write: the objects exist, every
/// path has outlines with anchors, the anchor ids are new, and no change turns a path into a
/// primitive or a primitive into another kind.
fn validate(tree: &LoroTree, writes: &[OffsetWrite]) -> Result<(), ObjectEditError> {
    if writes.is_empty() {
        return Err(ObjectEditError::NoOutlines);
    }
    let mut seen: HashSet<AnchorId> = HashSet::new();
    let mut replaced: Vec<TreeID> = Vec::new();
    for write in writes {
        let (object, geometry) = match write {
            OffsetWrite::Add {
                source, geometry, ..
            } => (*source, geometry),
            OffsetWrite::Change { target, geometry } => (*target, geometry),
        };
        let tree_id = tree_id_of(object);
        if !leaf_exists(tree, tree_id) {
            return Err(ObjectEditError::NoSuchObject);
        }
        if let OffsetGeometry::Path(outlines) = geometry {
            if outlines.is_empty() || outlines.iter().any(|(anchors, _)| anchors.is_empty()) {
                return Err(ObjectEditError::NoOutlines);
            }
            for anchor in outlines.iter().flat_map(|(anchors, _)| anchors) {
                if !seen.insert(anchor.id) {
                    return Err(ObjectEditError::AnchorIds);
                }
            }
        }
        if let OffsetWrite::Change { geometry, .. } = write {
            let meta = tree
                .get_meta(tree_id)
                .map_err(|_| ObjectEditError::NoSuchObject)?;
            let tag = shape_codec::read_shape_tag(&meta);
            match (geometry, tag.as_deref()) {
                (OffsetGeometry::Path(_), _) => replaced.push(tree_id),
                (OffsetGeometry::Primitive { shape, .. }, Some(tag)) if tag == shape_tag(shape) => {
                }
                (OffsetGeometry::Primitive { .. }, _) => {
                    return Err(ObjectEditError::InvalidConversion);
                }
            }
        }
    }
    check_new_anchor_ids(tree, &replaced, &seen)
}

/// The stored tag of a primitive kind.
const fn shape_tag(shape: &Shape) -> &'static str {
    match shape {
        Shape::Rect { .. } => SHAPE_RECT,
        Shape::Ellipse { .. } => SHAPE_ELLIPSE,
        Shape::Polygon { .. } => SHAPE_POLYGON,
        Shape::Star { .. } => SHAPE_STAR,
    }
}

/// Creates a root-level object with `geometry` and `style`; the caller places it.
fn create_object(
    tree: &LoroTree,
    geometry: &OffsetGeometry,
    style: &crate::style_model::Style,
) -> TreeID {
    // invariant: a root-level create on a live tree and reading the new node's meta map cannot
    // fail.
    #[allow(clippy::unwrap_used)]
    let created = tree.create(TreeParentId::Root).unwrap();
    #[allow(clippy::unwrap_used)]
    let meta = tree.get_meta(created).unwrap();
    match geometry {
        OffsetGeometry::Path(outlines) => {
            path_codec::write_path_style(&meta, true, style);
            write_outlines(&meta, outlines);
        }
        OffsetGeometry::Primitive { shape, rotation } => {
            shape_codec::write_shape_tag(&meta, shape_tag(shape));
            style_codec::write_style(&meta, style);
            write_primitive_fields(&meta, shape);
            if rotation.normalized() != Angle::from_radians(0.0) {
                path_codec::write_rotation(&meta, *rotation);
            }
        }
    }
    created
}

/// Writes a path's outlines: the first in the ordinary keys, the rest as extra subpaths. Every
/// outline is closed.
fn write_outlines(meta: &loro::LoroMap, outlines: &PieceOutlines) {
    // invariant: the caller checked `outlines` to be non-empty.
    #[allow(clippy::unwrap_used)]
    let (first, rest) = outlines.split_first().unwrap();
    // invariant: a plain bool insert on an attached map cannot fail.
    #[allow(clippy::unwrap_used)]
    meta.insert(path_codec::KEY_CLOSED, first.1).unwrap();
    let anchors = insert_anchors_container(meta);
    for anchor in &first.0 {
        push_anchor(&anchors, anchor);
    }
    replace_extra_subpaths(meta, rest);
}

/// Writes every field of a primitive's kind.
fn write_primitive_fields(meta: &loro::LoroMap, shape: &Shape) {
    shape_codec::write_shape_frame(meta, shape);
    match *shape {
        Shape::Rect { corner_radii, .. } => {
            write_corner_radii_if_changed(meta, corner_radii);
        }
        Shape::Ellipse { .. } => {}
        Shape::Polygon { point_count, .. } => shape_codec::write_point_count(meta, point_count),
        Shape::Star {
            point_count,
            inner_ratio,
            ..
        } => {
            shape_codec::write_point_count(meta, point_count);
            shape_codec::write_inner_ratio(meta, inner_ratio);
        }
    }
}

/// Replaces an object's geometry in place, keeping every other key (name, style, links).
fn change_geometry(meta: &loro::LoroMap, geometry: &OffsetGeometry) {
    match geometry {
        OffsetGeometry::Path(outlines) => {
            shape_codec::strip_primitive_keys(meta);
            write_outlines(meta, outlines);
            if path_codec::read_rotation(meta) != Angle::from_radians(0.0) {
                path_codec::write_rotation(meta, Angle::from_radians(0.0));
            }
        }
        OffsetGeometry::Primitive { shape, .. } => write_primitive_fields(meta, shape),
    }
}
