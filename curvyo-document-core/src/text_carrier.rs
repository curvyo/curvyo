//! The outline a text on path sits on, read from its carrier or from its kept copy, and the copy
//! written when a carrier is removed (ADR 0022 §3, `specs/0080-text-on-path/adrs.md` decisions 3
//! and 7).

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use loro::{LoroTree, TreeID};

use crate::document::Document;
use crate::path_model::{NodeId, PathSnapshot};
use crate::primitive_model::{ObjectSnapshot, Shape};
use crate::primitive_outline::{OutlineAnchor, drawing_outline_of_rotated};
use crate::text_path_model::{CarrierCurve, EllipseCarrier};

/// Outline `outline` of `path` (an index past the last reads as the last).
fn path_curve(path: &PathSnapshot, outline: u32) -> Option<CarrierCurve> {
    let count = path.subpaths().count();
    let index = usize::try_from(outline)
        .unwrap_or(usize::MAX)
        .min(count.checked_sub(1)?);
    let subpath = path.subpaths().nth(index)?;
    (!subpath.anchors.is_empty()).then(|| CarrierCurve {
        closed: subpath.closed,
        anchors: subpath
            .anchors
            .iter()
            .map(|a| OutlineAnchor {
                point: a.point,
                handle_in: a.handle_in,
                handle_out: a.handle_out,
                kind: a.kind,
            })
            .collect(),
        ellipse: None,
    })
}

/// The outline a text put on `object` sits on: a path's or a resolved clone's outline `outline`,
/// a primitive's drawn outline (`drawing_outline_of_rotated`: the curve the renderer and the
/// exporters draw, ADR 0022 §3). `None` for a text: it is not a carrier.
#[must_use]
pub fn carrier_curve(object: &ObjectSnapshot, outline: u32) -> Option<CarrierCurve> {
    match object {
        ObjectSnapshot::Path(path) => path_curve(path, outline),
        ObjectSnapshot::Sketch(sketch) => path_curve(&sketch.resolved, outline),
        ObjectSnapshot::Clone(clone) => path_curve(&clone.resolved, outline),
        ObjectSnapshot::Primitive(primitive) => {
            let drawn = drawing_outline_of_rotated(&primitive.shape, primitive.rotation);
            let ellipse = match primitive.shape {
                Shape::Ellipse { frame, .. } => Some(EllipseCarrier {
                    centre: frame.center,
                    rx: frame.rx.as_mm(),
                    ry: frame.ry.as_mm(),
                    rotation: primitive.rotation.as_radians(),
                }),
                _ => None,
            };
            if drawn.anchors.is_empty() {
                return None;
            }
            Some(CarrierCurve {
                closed: drawn.closed,
                anchors: drawn.anchors,
                ellipse,
            })
        }
        // Neither a text nor an image is a carrier (ADR 0022, `specs/0058-bitmap-import/adrs.md`).
        ObjectSnapshot::Text(_) | ObjectSnapshot::Image(_) => None,
    }
}

/// Fills the outline of every text on path among `leaves` (stacking order, `None` for an
/// orphan) from its live carrier, else from its kept copy (ADR 0022 §3); returns the texts on
/// each live carrier (carrier → texts, stacking order).
pub(crate) fn resolve_carriers(
    leaves: &mut [Option<ObjectSnapshot>],
) -> HashMap<NodeId, Vec<NodeId>> {
    let mut texts_on: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
    let wanted: Vec<usize> = leaves
        .iter()
        .enumerate()
        .filter_map(|(i, leaf)| match leaf {
            Some(ObjectSnapshot::Text(text)) if text.on_path.is_some() => Some(i),
            _ => None,
        })
        .collect();
    if wanted.is_empty() {
        return texts_on;
    }
    let index: HashMap<NodeId, usize> = leaves
        .iter()
        .enumerate()
        .filter_map(|(i, leaf)| leaf.as_ref().map(|object| (object.id(), i)))
        .collect();
    let mut curves: HashMap<(NodeId, u32), Option<Arc<CarrierCurve>>> = HashMap::new();
    for i in wanted {
        let Some((carrier, text_id)) = (match &leaves[i] {
            Some(ObjectSnapshot::Text(text)) => text.on_path.as_ref().map(|p| (p.carrier, text.id)),
            _ => None,
        }) else {
            continue;
        };
        let live = curves
            .entry((carrier.node, carrier.outline))
            .or_insert_with(|| {
                index
                    .get(&carrier.node)
                    .and_then(|&at| leaves[at].as_ref())
                    .and_then(|object| carrier_curve(object, carrier.outline))
                    .map(Arc::new)
            })
            .clone();
        if live.is_some() {
            texts_on.entry(carrier.node).or_default().push(text_id);
        }
        if let Some(ObjectSnapshot::Text(text)) = &mut leaves[i]
            && let Some(on_path) = &mut text.on_path
        {
            on_path.kept = live.is_none() && on_path.kept_curve.is_some();
            on_path.curve = live.or_else(|| on_path.kept_curve.clone());
        }
    }
    texts_on
}

/// Fills the outline of the one text `text` from its carrier read alone, else from its kept copy
/// (a read of one node, `Document::text` and `Document::object`).
pub(crate) fn resolve_one(tree: &LoroTree, text: &mut crate::text_model::TextSnapshot) {
    let Some(on_path) = &mut text.on_path else {
        return;
    };
    let live = crate::shapes::object_in(tree, on_path.carrier.node)
        .filter(|object| !matches!(object, ObjectSnapshot::Text(_)))
        .and_then(|object| carrier_curve(&object, on_path.carrier.outline))
        .map(Arc::new);
    on_path.kept = live.is_none() && on_path.kept_curve.is_some();
    on_path.curve = live.or_else(|| on_path.kept_curve.clone());
}

/// What the texts on the carriers among `changed` would sit on if `changed` replaced those
/// objects of `objects` (a read): the live preview of a drag that edits a carrier
/// (`specs/0080-text-on-path` criterion 24, `adrs.md` decision 7), the analogue of
/// `clone_previews`. Each text keeps its laid-out outline of the read; the caller lays out only
/// these again. `None` when more than `cap` texts follow.
#[must_use]
pub fn text_path_previews(
    objects: &[ObjectSnapshot],
    changed: &[ObjectSnapshot],
    cap: usize,
) -> Option<Vec<crate::text_model::TextSnapshot>> {
    let replaced: HashMap<NodeId, &ObjectSnapshot> =
        changed.iter().map(|object| (object.id(), object)).collect();
    let mut previews = Vec::new();
    for object in objects {
        let ObjectSnapshot::Text(text) = object else {
            continue;
        };
        let Some(on_path) = &text.on_path else {
            continue;
        };
        let Some(carrier) = replaced.get(&on_path.carrier.node) else {
            continue;
        };
        if previews.len() == cap {
            return None;
        }
        let mut moved = (**text).clone();
        if let Some(on_path) = &mut moved.on_path {
            on_path.curve = carrier_curve(carrier, on_path.carrier.outline).map(Arc::new);
            on_path.kept = false;
        }
        previews.push(moved);
    }
    Some(previews)
}

/// How far Duplicate moves a copied text along its carrier when the carrier is not copied
/// (`specs/0080-text-on-path` criterion 31, Proposal).
pub const DUPLICATE_TEXT_SHIFT: crate::units::Length = crate::units::Length::from_mm(5.0);

/// Puts every copied text on path on its carrier's copy when the carrier was copied too, else
/// moves it along the same carrier by its shift in `shifts` (by its source's id), else by
/// [`DUPLICATE_TEXT_SHIFT`] (criterion 31). The copies are uncommitted.
pub(crate) fn relink_copied_texts(
    tree: &LoroTree,
    made: &[(TreeID, TreeID)],
    shifts: &HashMap<NodeId, crate::units::Length>,
) {
    let node = |id: TreeID| NodeId::from_parts(id.peer, id.counter);
    let copy_of: HashMap<NodeId, NodeId> = made
        .iter()
        .map(|(source, copy)| (node(*source), node(*copy)))
        .collect();
    for &(source, copy) in made {
        let Ok(meta) = tree.get_meta(copy) else {
            continue;
        };
        let (Some(mut carrier), Some(mut values)) = (
            crate::text_path_codec::read_carrier(&meta),
            crate::text_path_codec::read_values(&meta),
        ) else {
            continue;
        };
        if let Some(&carrier_copy) = copy_of.get(&carrier.node) {
            carrier.node = carrier_copy;
            crate::text_path_codec::write_carrier(&meta, carrier);
        } else {
            let shift = shifts
                .get(&node(source))
                .copied()
                .unwrap_or(DUPLICATE_TEXT_SHIFT);
            values.position =
                crate::units::Length::from_mm(values.position.as_mm() + shift.as_mm());
            crate::text_path_codec::write_values(&meta, &values);
        }
    }
}

/// Writes, without committing, a new outline index on every text on an outline of `path`
/// after a node edit removed, inserted or merged outlines of it
/// (`specs/0055-compound-path-node-editing` criteria 4 to 6): `renumber` maps the old index
/// to the new one, so the text stays on the outline it was put on. A text whose outline was
/// removed gets the index of the outline that took its place (read as the last outline when
/// none did, `specs/0080-text-on-path/adrs.md` decision 1).
pub(crate) fn renumber_carried_outlines(
    tree: &LoroTree,
    path: NodeId,
    renumber: &dyn Fn(u32) -> u32,
) {
    for leaf in crate::group_tree::leaves_under(tree, None) {
        let Ok(meta) = tree.get_meta(leaf) else {
            continue;
        };
        let Some(mut carrier) = crate::text_path_codec::read_carrier(&meta) else {
            continue;
        };
        let outline = renumber(carrier.outline);
        if carrier.node == path && outline != carrier.outline {
            carrier.outline = outline;
            crate::text_path_codec::write_carrier(&meta, carrier);
        }
    }
}

impl Document {
    /// Writes, without committing, the kept copy of its carrier's outline on every live text
    /// outside `removed` whose carrier is in it (`specs/0080-text-on-path` criterion 25, Frage 36
    /// A): the text keeps the curve it sits on. `text_carrier` stays, so Undo of the removal
    /// links it again at once. Call it before the nodes are deleted, in the commit that deletes
    /// them. Returns how many texts kept their path.
    pub(crate) fn keep_carriers_of_removed(&self, tree: &LoroTree, removed: &[TreeID]) -> usize {
        let mut gone: HashSet<NodeId> = HashSet::new();
        for &node in removed {
            gone.insert(NodeId::from_parts(node.peer, node.counter));
            for leaf in crate::group_tree::leaves_under(tree, Some(node)) {
                gone.insert(NodeId::from_parts(leaf.peer, leaf.counter));
            }
        }
        let read = self.read_objects();
        let mut kept = 0;
        for object in read.objects.iter() {
            let ObjectSnapshot::Text(text) = object else {
                continue;
            };
            let Some(on_path) = &text.on_path else {
                continue;
            };
            if gone.contains(&text.id) || !gone.contains(&on_path.carrier.node) || on_path.kept {
                continue;
            }
            let Some(curve) = &on_path.curve else {
                continue;
            };
            if let Ok(meta) = tree.get_meta(TreeID::new(text.id.peer, text.id.counter)) {
                crate::text_path_codec::write_kept(&meta, curve);
                kept += 1;
            }
        }
        self.kept_carriers.set(self.kept_carriers.get() + kept);
        kept
    }

    /// How many texts kept the outline of their removed carrier since the last call, for the
    /// notice of the removing command ("1 text kept its path.", criterion 25); resets the count.
    #[must_use]
    pub fn take_kept_carriers(&self) -> usize {
        self.kept_carriers.replace(0)
    }
}
