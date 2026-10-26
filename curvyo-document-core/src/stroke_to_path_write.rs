//! The one write of Stroke to path (`specs/0085-stroke-to-path` criteria 14 to 17, 19 and 21;
//! `adrs.md` decision 3): in **one commit** the clones that must not follow a change are
//! unlinked, the texts are converted to glyph paths, every subject loses its stroke and gets its
//! stroke outline directly above it, or, with Fill off, becomes that outline under its own id. It
//! computes no geometry; the caller passes the outlines.

use std::collections::{HashMap, HashSet};

use loro::{LoroMap, LoroTree, TreeID, TreeParentId};

use crate::document::{Document, OBJECTS_TREE};
use crate::objects::{CopySource, ObjectEditError};
use crate::offset_write::{OffsetGeometry, change_geometry, write_outlines};
use crate::path_codec::{self, leaf_exists};
use crate::path_model::{AnchorId, NodeId};
use crate::paths::tree_id_of;
use crate::replace::{PieceOutlines, check_new_anchor_ids};
use crate::style_codec;
use crate::style_model::{Fill, Stroke, Style};
use crate::text_paths::TextConversion;

/// The commit label of Stroke to path (ADR 0002 §9).
pub const STROKE_TO_PATH_COMMIT_LABEL: &str = "stroke_to_path";

/// The object whose stroke one outline replaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StrokeSubject {
    /// A path, compound path, primitive or linked clone (a clone is unlinked first, so it must be
    /// among [`StrokeToPath::unlinks`]).
    Object(NodeId),
    /// The glyph path number `glyph` (in the order of [`TextConversion::glyphs`]) of a text of
    /// [`StrokeToPath::texts`].
    Glyph {
        /// The text.
        text: NodeId,
        /// The glyph's index.
        glyph: usize,
    },
}

/// One subject and its stroke outline.
#[derive(Debug, Clone, PartialEq)]
pub struct StrokeOutlineWrite {
    /// The subject.
    pub subject: StrokeSubject,
    /// Its stroke outline: closed outlines, the first in the ordinary keys, the rest as extra
    /// subpaths.
    pub outline: PieceOutlines,
}

/// Everything one Stroke to path writes.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct StrokeToPath {
    /// Linked clones made paths first, with their fresh anchor ids: the selected clones that are
    /// subjects, and every clone that follows a subject, so that nothing on the canvas jumps
    /// (criterion 17).
    pub unlinks: Vec<CopySource>,
    /// The texts converted to glyph paths, as Object to path converts them (criterion 15).
    pub texts: Vec<TextConversion>,
    /// The subjects with their outlines, in stacking order.
    pub outlines: Vec<StrokeOutlineWrite>,
}

/// The style of a stroke outline (Frage 47 A): Fill on in the stroke's colour and opacity, Stroke
/// off with the stroke's width and colour kept; no dash, join, cap or markers.
#[must_use]
pub fn stroke_outline_style(subject: &Style) -> Style {
    Style {
        stroke: Stroke {
            enabled: false,
            width: subject.stroke.width,
            color: subject.stroke.color,
            ..Stroke::default()
        },
        fill: Fill {
            enabled: true,
            color: subject.stroke.color,
            opacity: subject.stroke.opacity,
        },
    }
}

impl Document {
    /// Writes Stroke to path in **one commit** labelled [`STROKE_TO_PATH_COMMIT_LABEL`] and
    /// returns what to select: per outline in order its fill part (when the subject keeps its
    /// fill) and its outline. Each outline lies directly above its subject, in the same layer or
    /// group, with the subject's own opacity; a subject with Fill off is replaced by its outline
    /// under its own id, name and place (criterion 19). A primitive fill part stays the same
    /// primitive (criterion 14).
    ///
    /// Every id and anchor id is checked before the first write.
    ///
    /// # Errors
    /// [`ObjectEditError::NoOutlines`] when there is no outline or an outline without anchors;
    /// [`ObjectEditError::NoSuchObject`] for a subject, clone or text that does not exist or a
    /// glyph the text does not have; [`ObjectEditError::AnchorIds`] when anchor ids repeat or are
    /// in use.
    pub fn stroke_to_path(&self, write: &StrokeToPath) -> Result<Vec<NodeId>, ObjectEditError> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        validate(&tree, write)?;
        self.batch(STROKE_TO_PATH_COMMIT_LABEL, |document| {
            if !write.unlinks.is_empty() {
                document
                    .unlink_clones_and_texts(&write.unlinks, &[])
                    .map_err(|_| ObjectEditError::NoSuchObject)?;
            }
            let mut glyphs: HashMap<(NodeId, usize), NodeId> = HashMap::new();
            if !write.texts.is_empty() {
                let groups = document.convert_texts_uncommitted(&write.texts)?;
                for (text, group) in write.texts.iter().zip(groups) {
                    let leaves = group.map(|g| document.leaves_of(g)).unwrap_or_default();
                    for (index, leaf) in leaves.into_iter().enumerate() {
                        glyphs.insert((text.id, index), leaf);
                    }
                }
            }
            tree.enable_fractional_index(0);
            let mut selected = Vec::with_capacity(2 * write.outlines.len());
            for item in &write.outlines {
                let target = match item.subject {
                    StrokeSubject::Object(id) => id,
                    StrokeSubject::Glyph { text, glyph } => *glyphs
                        .get(&(text, glyph))
                        .ok_or(ObjectEditError::NoSuchObject)?,
                };
                selected.extend(write_one(&tree, target, &item.outline)?);
            }
            Ok(selected)
        })
    }
}

/// The subject `target` loses its stroke and gets `outline` above it, or becomes `outline`.
/// Returns the objects to select.
fn write_one(
    tree: &LoroTree,
    target: NodeId,
    outline: &PieceOutlines,
) -> Result<Vec<NodeId>, ObjectEditError> {
    let tree_id = tree_id_of(target);
    let meta = tree
        .get_meta(tree_id)
        .map_err(|_| ObjectEditError::NoSuchObject)?;
    let style = style_codec::read_style(&meta);
    let painted = stroke_outline_style(&style);
    if !style.fill.enabled {
        change_geometry(&meta, &OffsetGeometry::Path(outline.clone()));
        style_codec::write_changes(&meta, &style, &painted);
        return Ok(vec![target]);
    }
    let mut fill_part = style.clone();
    fill_part.stroke.enabled = false;
    style_codec::write_changes(&meta, &style, &fill_part);
    let opacity = crate::node_registers::read_registers(&meta).opacity.get();
    let created = create_outline(tree, outline, &painted, opacity);
    // invariant: both nodes are live and the fractional index is on.
    #[allow(clippy::unwrap_used)]
    tree.mov_after(created, tree_id).unwrap();
    Ok(vec![
        target,
        NodeId::from_parts(created.peer, created.counter),
    ])
}

/// A new path node holding `outline` with `style` and the `opacity` register; the caller places
/// it.
fn create_outline(tree: &LoroTree, outline: &PieceOutlines, style: &Style, opacity: f64) -> TreeID {
    // invariant: a root-level create on a live tree and reading the new node's meta map cannot
    // fail.
    #[allow(clippy::unwrap_used)]
    let created = tree.create(TreeParentId::Root).unwrap();
    #[allow(clippy::unwrap_used)]
    let meta: LoroMap = tree.get_meta(created).unwrap();
    path_codec::write_path_style(&meta, true, style);
    write_outlines(&meta, outline);
    crate::clone_write::write_opacity_register(&meta, opacity);
    created
}

/// Every check before the first write: something to write, every outline with anchors, every
/// subject, clone and text live, every glyph known, every anchor id new and used once.
fn validate(tree: &LoroTree, write: &StrokeToPath) -> Result<(), ObjectEditError> {
    if write.outlines.is_empty()
        || write
            .outlines
            .iter()
            .any(|item| item.outline.is_empty() || item.outline.iter().any(|(a, _)| a.is_empty()))
    {
        return Err(ObjectEditError::NoOutlines);
    }
    let live = |id: NodeId| leaf_exists(tree, tree_id_of(id));
    let glyph_counts: HashMap<NodeId, usize> = write
        .texts
        .iter()
        .map(|text| (text.id, text.glyphs.len()))
        .collect();
    for item in &write.outlines {
        let known = match item.subject {
            StrokeSubject::Object(id) => live(id),
            StrokeSubject::Glyph { text, glyph } => {
                glyph_counts.get(&text).is_some_and(|&count| glyph < count)
            }
        };
        if !known {
            return Err(ObjectEditError::NoSuchObject);
        }
    }
    if !write.unlinks.iter().all(|unlink| live(unlink.id))
        || !write.texts.iter().all(|text| live(text.id))
    {
        return Err(ObjectEditError::NoSuchObject);
    }
    let mut seen: HashSet<AnchorId> = HashSet::new();
    let anchors = write
        .outlines
        .iter()
        .flat_map(|item| &item.outline)
        .chain(
            write
                .texts
                .iter()
                .flat_map(|text| text.glyphs.iter().flatten()),
        )
        .flat_map(|(anchors, _)| anchors.iter().map(|anchor| anchor.id))
        .chain(
            write
                .unlinks
                .iter()
                .flat_map(|u| u.anchor_ids.iter().copied()),
        );
    for id in anchors {
        if !seen.insert(id) {
            return Err(ObjectEditError::AnchorIds);
        }
    }
    check_new_anchor_ids(tree, &[], &seen)
}
