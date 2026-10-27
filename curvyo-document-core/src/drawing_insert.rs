//! Writing a whole drawing (`specs/0024-svg-import-export` criteria 2, 3 and 77; ADR 0018 §2):
//! a tree of groups, layers, paths, primitives and linked clones, built by an importer, inserted
//! as **one** new layer in one commit, or made the content of a new document whose setup change
//! holds it, so the new document has no step to undo. Existing node kinds and registers only; no
//! format change.

use std::sync::Arc;

use loro::{LoroTree, TreeID, TreeParentId};
use serde::Serialize;

use crate::affine::Affine;
use crate::clone_codec::{CloneLink, write_clone};
use crate::display_unit::DisplayUnit;
use crate::document::{Document, OBJECTS_TREE};
use crate::document_background::DocumentBackground;
use crate::drawing_content::DrawingText;
use crate::group_tree::{KEY_GROUP, node_id};
use crate::node_registers::{KEY_LAYER, KEY_LOCKED, KEY_NAME, KEY_VISIBLE};
use crate::objects::ObjectEditError;
use crate::path_model::{AnchorId, AnchorKind, NewAnchor, NodeId};
use crate::paths::tree_id_of;
use crate::primitive_model::Shape;
use crate::style_model::{Opacity, Style};
use crate::text_path_model::OnPathValues;
use crate::units::{Angle, DocumentSize, Point, Vec2};

/// The label of Import SVG's commit; its history name is "Import SVG".
pub const IMPORT_SVG_COMMIT_LABEL: &str = "import_svg";

/// The registers of one node of a drawing.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DrawingRegisters {
    /// The name, already cleaned (`clean_name`); `None` for the default.
    pub name: Option<String>,
    /// Whether the node is shown.
    pub visible: bool,
    /// Whether the node is locked.
    pub locked: bool,
    /// The node's opacity.
    pub opacity: Opacity,
    /// Whether the node is a guide (`0046`): an import's carrier that SVG does not draw.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub guide: bool,
}

impl Default for DrawingRegisters {
    fn default() -> Self {
        Self {
            name: None,
            visible: true,
            locked: false,
            opacity: Opacity::OPAQUE,
            guide: false,
        }
    }
}

/// One anchor of a drawing's path, without its id (the caller mints the ids).
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct DrawingAnchor {
    /// Its position in document millimetres.
    pub point: Point,
    /// Its handle toward the previous anchor, relative to `point`.
    pub handle_in: Vec2,
    /// Its handle toward the next anchor, relative to `point`.
    pub handle_out: Vec2,
    /// Its kind.
    pub kind: AnchorKind,
}

/// One outline of a drawing's path.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DrawingOutline {
    /// Whether it is closed.
    pub closed: bool,
    /// Its anchors in order; at least one.
    pub anchors: Vec<DrawingAnchor>,
}

/// One node of a drawing, in z-order among its siblings.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum DrawingNode {
    /// A group, or with `layer` a layer (only at the top level of [`Document::from_drawing`]).
    Group {
        /// Its registers.
        registers: DrawingRegisters,
        /// Whether it is a layer.
        layer: bool,
        /// Its children.
        children: Vec<DrawingNode>,
    },
    /// A path or compound path.
    Path {
        /// Its registers.
        registers: DrawingRegisters,
        /// Its outlines in order; at least one.
        outlines: Vec<DrawingOutline>,
        /// Its style.
        style: Style,
    },
    /// A rectangle, ellipse, polygon or star.
    Primitive {
        /// Its registers.
        registers: DrawingRegisters,
        /// Its kind and frame.
        shape: Shape,
        /// Its rotation about the frame's centre.
        rotation: Angle,
        /// Its style.
        style: Style,
    },
    /// A linked clone of the leaf number `original` (document order of all leaves of the
    /// drawing, clones included), which is a path or another clone.
    Clone {
        /// Its registers.
        registers: DrawingRegisters,
        /// The original's leaf number.
        original: usize,
        /// The clone matrix.
        matrix: Affine,
    },
    /// A straight text (`0074`); it takes no leaf number.
    Text {
        /// Its registers.
        registers: DrawingRegisters,
        /// Its values.
        text: DrawingText,
    },
    /// A text on the path or primitive leaf number `carrier` (`0080`); it takes no leaf number.
    TextOnPath {
        /// Its registers.
        registers: DrawingRegisters,
        /// Its values; `placement` is the straight placement kept for Release.
        text: DrawingText,
        /// The carrier's leaf number.
        carrier: usize,
        /// Its values on the carrier.
        values: OnPathValues,
    },
    /// An image (`0058`) of the asset number `asset`; it takes no leaf number.
    Image {
        /// Its registers.
        registers: DrawingRegisters,
        /// The index of its bytes in the drawing's assets.
        asset: usize,
        /// The unit square to document millimetres.
        placement: Affine,
    },
}

/// The page settings of a drawing that becomes a new document.
#[derive(Debug, Clone, PartialEq)]
pub struct DrawingPage {
    /// The page size, within the document limits.
    pub size: DocumentSize,
    /// The display unit, when the drawing names one.
    pub display_unit: Option<DisplayUnit>,
    /// The background, when the drawing gives one.
    pub background: Option<DocumentBackground>,
}

impl DrawingNode {
    /// How many anchor ids the drawing `nodes` needs: one per anchor of every path.
    #[must_use]
    pub fn anchor_count(nodes: &[Self]) -> usize {
        nodes
            .iter()
            .map(|node| match node {
                Self::Group { children, .. } => Self::anchor_count(children),
                Self::Path { outlines, .. } => outlines.iter().map(|o| o.anchors.len()).sum(),
                _ => 0,
            })
            .sum()
    }
}

/// The writer of one drawing: the anchor ids in order, the created leaves by number and the
/// clones to link once every leaf exists.
struct Writer<'a, I: Iterator<Item = AnchorId>> {
    document: &'a Document,
    tree: LoroTree,
    ids: I,
    leaves: Vec<TreeID>,
    clones: Vec<(TreeID, usize, Affine)>,
    /// Texts on path to link once every leaf exists.
    links: Vec<(TreeID, usize, OnPathValues)>,
    assets: &'a [Arc<[u8]>],
}

impl<I: Iterator<Item = AnchorId>> Writer<'_, I> {
    /// Writes `nodes` as the last children of `parent`, in order; returns the created nodes.
    fn write(&mut self, parent: TreeParentId, nodes: &[DrawingNode]) -> Vec<TreeID> {
        nodes
            .iter()
            .map(|node| self.write_one(parent, node))
            .collect()
    }

    fn write_one(&mut self, parent: TreeParentId, node: &DrawingNode) -> TreeID {
        let (created, registers) = match node {
            DrawingNode::Group {
                registers,
                layer,
                children,
            } => {
                let group = self.create(parent);
                let meta = self.meta(group);
                // invariant: inserting plain values into a node just created cannot fail.
                #[allow(clippy::unwrap_used)]
                {
                    meta.insert(KEY_GROUP, true).unwrap();
                    if *layer {
                        meta.insert(KEY_LAYER, true).unwrap();
                    }
                }
                self.write(TreeParentId::Node(group), children);
                (group, registers)
            }
            DrawingNode::Path {
                registers,
                outlines,
                style,
            } => (self.write_path(parent, outlines, style), registers),
            DrawingNode::Primitive {
                registers,
                shape,
                rotation,
                style,
            } => {
                let previous = self.document.draw_parent.replace(match parent {
                    TreeParentId::Node(group) => Some(group),
                    _ => None,
                });
                let id = self
                    .document
                    .create_primitive_uncommitted(*shape, *rotation);
                self.document.draw_parent.set(previous);
                let leaf = tree_id_of(id);
                let meta = self.meta(leaf);
                let created = crate::style_codec::read_style(&meta);
                crate::style_codec::write_changes(&meta, &created, style);
                self.leaves.push(leaf);
                (leaf, registers)
            }
            DrawingNode::Clone {
                registers,
                original,
                matrix,
            } => {
                let leaf = self.create(parent);
                self.leaves.push(leaf);
                self.clones.push((leaf, *original, *matrix));
                (leaf, registers)
            }
            DrawingNode::Text { registers, text } => {
                (self.document.write_drawing_text(parent, text), registers)
            }
            DrawingNode::TextOnPath {
                registers,
                text,
                carrier,
                values,
            } => (self.text_on_path(parent, text, *carrier, values), registers),
            DrawingNode::Image {
                registers,
                asset,
                placement,
            } => (self.image(parent, *asset, *placement), registers),
        };
        self.write_registers(created, registers);
        created
    }

    /// Writes a path of `outlines` (at least one) with `style`.
    fn write_path(
        &mut self,
        parent: TreeParentId,
        outlines: &[DrawingOutline],
        style: &Style,
    ) -> TreeID {
        let mut renumbered = outlines.iter().map(|outline| {
            let anchors = outline
                .anchors
                .iter()
                .map(|a| NewAnchor {
                    // invariant: `validate` checked one id per anchor.
                    #[allow(clippy::unwrap_used)]
                    id: self.ids.next().unwrap(),
                    point: a.point,
                    handle_in: a.handle_in,
                    handle_out: a.handle_out,
                    kind: a.kind,
                })
                .collect::<Vec<_>>();
            (anchors, outline.closed)
        });
        // invariant: `validate` checked at least one outline.
        #[allow(clippy::unwrap_used)]
        let (first, closed) = renumbered.next().unwrap();
        let rest: Vec<_> = renumbered.collect();
        let id = self.document.create_path_uncommitted(
            parent,
            &first,
            closed,
            style,
            Angle::from_radians(0.0),
        );
        let leaf = tree_id_of(id);
        crate::subpath_codec::append_outlines(&self.meta(leaf), &rest);
        self.leaves.push(leaf);
        leaf
    }

    /// Writes a text on path; it is linked to its carrier once every leaf exists.
    fn text_on_path(
        &mut self,
        parent: TreeParentId,
        text: &DrawingText,
        carrier: usize,
        values: &OnPathValues,
    ) -> TreeID {
        let created = self.document.write_drawing_text(parent, text);
        self.links.push((created, carrier, *values));
        created
    }

    /// Writes an image of the asset number `asset`.
    fn image(&self, parent: TreeParentId, asset: usize, placement: Affine) -> TreeID {
        self.document
            .write_drawing_image(&self.tree, parent, &self.assets[asset], placement)
    }

    fn create(&self, parent: TreeParentId) -> TreeID {
        // invariant: the parent is the root or a group this writer created.
        #[allow(clippy::unwrap_used)]
        self.tree.create(parent).unwrap()
    }

    fn meta(&self, node: TreeID) -> loro::LoroMap {
        // invariant: every node passed here was created by this writer.
        #[allow(clippy::unwrap_used)]
        self.tree.get_meta(node).unwrap()
    }

    fn write_registers(&self, node: TreeID, registers: &DrawingRegisters) {
        let meta = self.meta(node);
        // invariant: inserting plain values into a node just created cannot fail.
        #[allow(clippy::unwrap_used)]
        {
            if let Some(name) = &registers.name {
                meta.insert(KEY_NAME, name.as_str()).unwrap();
            }
            if !registers.visible {
                meta.insert(KEY_VISIBLE, false).unwrap();
            }
            if registers.locked {
                meta.insert(KEY_LOCKED, true).unwrap();
            }
        }
        if registers.guide {
            crate::guides::write_guide(&meta, true);
        }
        crate::clone_write::write_opacity_register(&meta, registers.opacity.get());
    }

    /// Writes every clone's link now that all leaves exist.
    fn link_clones(&self) {
        for &(clone, original, matrix) in &self.clones {
            let origin = node_id(self.leaves[original]);
            write_clone(&self.meta(clone), CloneLink { origin, matrix });
        }
        for (text, carrier, values) in &self.links {
            crate::drawing_content::link_text(&self.tree, *text, self.leaves[*carrier], values);
        }
    }
}

impl Document {
    /// Adds `nodes` as the children of **one new layer** named `layer_name` on top of the top
    /// level, in one commit labelled [`IMPORT_SVG_COMMIT_LABEL`] (`0024` criterion 3). Layers of
    /// the drawing must already be groups (layers do not nest). `anchor_ids` are the paths'
    /// anchor ids, [`DrawingNode::anchor_count`] in document order; `assets` the bytes the
    /// images name, written to the asset store in the same commit (criterion 104). Returns the
    /// layer and its top-level children.
    ///
    /// # Errors
    /// [`ObjectEditError::AnchorIds`] when the ids are not as many as needed, repeat or are in
    /// use; [`ObjectEditError::NoOutlines`] for a path without an outline or an outline without
    /// an anchor; [`ObjectEditError::NoSuchObject`] for a clone whose chain does not end on a
    /// path within [`MAX_CLONE_DEPTH`](crate::MAX_CLONE_DEPTH); [`ObjectEditError::TooDeep`] for groups nested deeper
    /// than [`MAX_GROUP_DEPTH`](crate::MAX_GROUP_DEPTH) under the layer; [`ObjectEditError::InvalidText`] for a text
    /// that cannot be written; [`ObjectEditError::ProjectFull`] when the assets would pass the
    /// project's limit. Nothing is written.
    pub fn insert_drawing(
        &self,
        layer_name: &str,
        nodes: &[DrawingNode],
        anchor_ids: &[AnchorId],
        assets: &[Arc<[u8]>],
    ) -> Result<(NodeId, Vec<NodeId>), ObjectEditError> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        crate::drawing_check::validate(self, &tree, nodes, anchor_ids, 1, assets)?;
        Ok(self.batch(IMPORT_SVG_COMMIT_LABEL, |document| {
            let layer = crate::layers::create_layer_node(&tree, layer_name);
            let mut writer = Writer {
                document,
                tree: tree.clone(),
                ids: anchor_ids.iter().copied(),
                leaves: Vec::new(),
                clones: Vec::new(),
                links: Vec::new(),
                assets,
            };
            let children = writer.write(TreeParentId::Node(layer), nodes);
            writer.link_clones();
            (node_id(layer), children.into_iter().map(node_id).collect())
        }))
    }

    /// A new document bound to `peer` whose page is `page` and whose content is `nodes` at the
    /// top level, all in its setup change: like [`Document::new`] it has no step, so nothing
    /// can be undone (`0024` criterion 2). Arguments and errors as [`Document::insert_drawing`].
    ///
    /// # Errors
    /// See [`Document::insert_drawing`].
    pub fn from_drawing(
        peer: u64,
        page: &DrawingPage,
        nodes: &[DrawingNode],
        anchor_ids: &[AnchorId],
        assets: &[Arc<[u8]>],
    ) -> Result<Self, ObjectEditError> {
        let document = Self::blank(peer, page.size);
        let tree = document.loro().get_tree(OBJECTS_TREE);
        crate::drawing_check::validate(&document, &tree, nodes, anchor_ids, 0, assets)?;
        document.uncommitted(|document| {
            if let Some(unit) = page.display_unit {
                let _ = document.set_display_unit(unit);
            }
            if let Some(background) = page.background {
                let _ = document.set_background(background);
            }
            tree.enable_fractional_index(0);
            let mut writer = Writer {
                document,
                tree: tree.clone(),
                ids: anchor_ids.iter().copied(),
                leaves: Vec::new(),
                clones: Vec::new(),
                links: Vec::new(),
                assets,
            };
            writer.write(TreeParentId::Root, nodes);
            writer.link_clones();
        });
        document.commit_new_document();
        document.commit_state.set_warm();
        Ok(document)
    }
}
