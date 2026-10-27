//! The texts and images of an imported drawing (`specs/0024-svg-import-export` criteria 78, 93,
//! 100 to 104; `adrs.md` dated note 2026-10-27): their values, their checks before the one
//! commit, and their writes (texts as `0074` text nodes, texts on path linked to their carrier
//! once every leaf exists, images with their bytes in the asset store of `0058`).

use std::sync::Arc;

use loro::{LoroTree, TreeID, TreeParentId};
use serde::Serialize;

use crate::affine::Affine;
use crate::assets::{AssetHash, MAX_PROJECT_ASSET_BYTES};
use crate::document::Document;
use crate::objects::ObjectEditError;
use crate::path_model::NodeId;
use crate::style_model::Style;
use crate::text_chars::TextChars;
use crate::text_model::{
    FontRef, MAX_TEXT_CHARS, MAX_TEXT_SIZE, MIN_TEXT_SIZE, TextLayoutValues, TextSnapshot,
    is_face_hash,
};
use crate::text_path_model::{CarrierRef, OnPathValues};
use crate::units::Length;

/// One text of a drawing.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DrawingText {
    /// The string; lines separated by `\n`; 1 to [`MAX_TEXT_CHARS`] characters.
    pub string: String,
    /// The face (an import's missing face carries [`crate::MISSING_FACE_HASH`]).
    pub font: FontRef,
    /// The height of the capital H, within the size range.
    pub size: Length,
    /// Alignment, spacing, writing mode.
    #[serde(skip_serializing_if = "TextLayoutValues::is_default")]
    pub layout: TextLayoutValues,
    /// The per-character kerns and turns (ADR 0021).
    #[serde(skip_serializing_if = "TextChars::is_empty")]
    pub chars: TextChars,
    /// Text space to the document.
    pub placement: Affine,
    /// Fill and stroke.
    pub style: Style,
}

impl DrawingText {
    /// Whether the values can be written as a text object (criterion 31, the size range, a
    /// stored face hash, a usable placement).
    pub(crate) fn is_valid(&self) -> bool {
        let chars = self.string.chars().count();
        (1..=MAX_TEXT_CHARS).contains(&chars)
            && (MIN_TEXT_SIZE.as_mm()..=MAX_TEXT_SIZE.as_mm()).contains(&self.size.as_mm())
            && is_face_hash(&self.font.face_hash)
            && self.placement.is_within_limit()
            && self.placement.determinant().abs() > crate::affine::SINGULAR_DETERMINANT
    }

    /// The snapshot written for this text.
    pub(crate) fn snapshot(&self) -> TextSnapshot {
        TextSnapshot {
            id: NodeId::from_parts(0, 0),
            string: self.string.clone(),
            font: self.font.clone(),
            size: self.size,
            layout: self.layout.held(),
            chars: self.chars.clone(),
            placement: self.placement,
            style: self.style.clone(),
            outline: None,
            on_path: None,
            source: None,
        }
    }
}

/// The checks of the assets a drawing brings: every image names one, and the new bytes keep the
/// project within [`MAX_PROJECT_ASSET_BYTES`].
pub(crate) fn check_assets(
    document: &Document,
    assets: &[Arc<[u8]>],
    named: &[usize],
) -> Result<(), ObjectEditError> {
    if named.iter().any(|at| *at >= assets.len()) {
        return Err(ObjectEditError::NoSuchObject);
    }
    let files: Vec<(AssetHash, &[u8])> = assets
        .iter()
        .map(|bytes| (AssetHash::of(bytes), &**bytes))
        .collect();
    let store = document.assets.borrow();
    if store.total_bytes() + store.added_bytes(&files) > MAX_PROJECT_ASSET_BYTES {
        return Err(ObjectEditError::ProjectFull);
    }
    Ok(())
}

impl Document {
    /// Writes `text` as a new text node under `parent`, uncommitted.
    pub(crate) fn write_drawing_text(&self, parent: TreeParentId, text: &DrawingText) -> TreeID {
        let previous = self.draw_parent.replace(match parent {
            TreeParentId::Node(group) => Some(group),
            _ => None,
        });
        let id = self.create_text_uncommitted(&text.snapshot());
        self.draw_parent.set(previous);
        TreeID::new(id.peer, id.counter)
    }

    /// Writes an image of `bytes` framed by `placement` under `parent`, uncommitted; the bytes
    /// go into the asset store once.
    pub(crate) fn write_drawing_image(
        &self,
        tree: &LoroTree,
        parent: TreeParentId,
        bytes: &Arc<[u8]>,
        placement: Affine,
    ) -> TreeID {
        let hash = AssetHash::of(bytes);
        self.assets.borrow_mut().insert(hash.clone(), bytes.clone());
        // invariant: the parent is the root or a group this writer created.
        #[allow(clippy::unwrap_used)]
        let node = tree.create(parent).unwrap();
        // invariant: the meta map of a node just created can be read.
        #[allow(clippy::unwrap_used)]
        let meta = tree.get_meta(node).unwrap();
        crate::image_codec::write_image(&meta, &hash, placement);
        node
    }
}

/// Links the text `text` to its carrier `carrier` with `values`, uncommitted.
pub(crate) fn link_text(tree: &LoroTree, text: TreeID, carrier: TreeID, values: &OnPathValues) {
    // invariant: both nodes were created by the drawing's writer.
    #[allow(clippy::unwrap_used)]
    let meta = tree.get_meta(text).unwrap();
    crate::text_path_codec::write_carrier(
        &meta,
        CarrierRef {
            node: NodeId::from_parts(carrier.peer, carrier.counter),
            outline: 0,
        },
    );
    crate::text_path_codec::write_values(&meta, values);
}
