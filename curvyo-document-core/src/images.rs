//! Adding images to the document (`specs/0058-bitmap-import` criteria 2, 5, 13 to 15, 23, 45;
//! ADR 0017 §2): the bytes into the asset store (once per distinct file, within the project cap),
//! one leaf per picture on top of the active layer, all in one commit.

use std::sync::Arc;

use crate::affine::Affine;
use crate::assets::{AssetError, AssetHash, MAX_PROJECT_ASSET_BYTES};
use crate::document::{Document, OBJECTS_TREE};
use crate::image_model::{IMPORT_IMAGE_COMMIT_LABEL, IMPORT_IMAGES_COMMIT_LABEL};
use crate::node_registers::{KEY_NAME, clean_name};
use crate::path_model::NodeId;

/// One picture to add.
#[derive(Debug, Clone)]
pub struct NewImage {
    /// The file's bytes, verbatim.
    pub bytes: Arc<[u8]>,
    /// The frame: the unit square to document millimetres.
    pub placement: Affine,
    /// The display name (`specs/0058-bitmap-import` criterion 22).
    pub name: String,
}

/// Why images cannot be added; nothing was written.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ImageInsertError {
    /// The project would hold more than 512 MiB of pictures.
    #[error(transparent)]
    Assets(#[from] AssetError),
    /// A placement is not finite or beyond the matrix limit.
    #[error("the image frame is not a valid map")]
    InvalidPlacement,
}

impl Document {
    /// Adds `images` on top of the active layer (or the entered group; the Initial Layer is made
    /// when the document has none), in one commit "Import image" or "Import images", and returns
    /// their ids in order. The bytes go into the asset store once per distinct file.
    ///
    /// # Errors
    /// [`ImageInsertError::Assets`] when the new bytes would pass [`MAX_PROJECT_ASSET_BYTES`];
    /// [`ImageInsertError::InvalidPlacement`] for a frame that is not a valid map. Nothing is
    /// written then.
    pub fn add_images(&self, images: &[NewImage]) -> Result<Vec<NodeId>, ImageInsertError> {
        let label = if images.len() == 1 {
            IMPORT_IMAGE_COMMIT_LABEL
        } else {
            IMPORT_IMAGES_COMMIT_LABEL
        };
        self.check_images(images)?;
        Ok(self.batch(label, |document| document.write_images(images)))
    }

    /// Checks `images` as [`Document::add_images`] does, without writing: for a caller that adds
    /// them inside its own batch (a mixed import of SVG drawings and images, criterion 5).
    ///
    /// # Errors
    /// As [`Document::add_images`].
    pub fn check_images(&self, images: &[NewImage]) -> Result<(), ImageInsertError> {
        if images
            .iter()
            .any(|image| !image.placement.is_within_limit())
        {
            return Err(ImageInsertError::InvalidPlacement);
        }
        let files: Vec<(AssetHash, &[u8])> = images
            .iter()
            .map(|image| (AssetHash::of(&image.bytes), &*image.bytes))
            .collect();
        let store = self.assets.borrow();
        let adds = store.added_bytes(&files);
        if store.total_bytes() + adds > MAX_PROJECT_ASSET_BYTES {
            return Err(AssetError::ProjectFull { adds }.into());
        }
        Ok(())
    }

    /// Writes `images` (checked by [`Document::check_images`]) without committing.
    ///
    /// # Panics
    /// Does not panic in practice: every write goes to a node this call just created.
    pub fn write_images(&self, images: &[NewImage]) -> Vec<NodeId> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        images
            .iter()
            .map(|image| {
                let hash = AssetHash::of(&image.bytes);
                self.assets
                    .borrow_mut()
                    .insert(hash.clone(), image.bytes.clone());
                // invariant: creating a node under the root or a live group cannot fail.
                #[allow(clippy::unwrap_used)]
                let node = tree.create(self.drawing_parent(&tree)).unwrap();
                // invariant: the meta map of a node just created can be read.
                #[allow(clippy::unwrap_used)]
                let meta = tree.get_meta(node).unwrap();
                crate::image_codec::write_image(&meta, &hash, image.placement);
                if let Some(name) = clean_name(&image.name) {
                    // invariant: inserting a plain string into a new node's map cannot fail.
                    #[allow(clippy::unwrap_used)]
                    meta.insert(KEY_NAME, name).unwrap();
                }
                NodeId::from_parts(node.peer, node.counter)
            })
            .collect()
    }

    /// The bytes of the asset `hash`, when this project holds them.
    #[must_use]
    pub fn asset(&self, hash: &AssetHash) -> Option<Arc<[u8]>> {
        self.assets.borrow().get(hash)
    }

    /// The bytes of pictures this project holds, in total.
    #[must_use]
    pub fn asset_bytes(&self) -> u64 {
        self.assets.borrow().total_bytes()
    }

    /// Keeps only the assets that an image in the tree names (the document wipe, ADR 0017 §2).
    pub(crate) fn prune_assets(&self) {
        let alive = self
            .read_objects()
            .objects
            .iter()
            .filter_map(|object| match object {
                crate::primitive_model::ObjectSnapshot::Image(image) => Some(image.asset.clone()),
                _ => None,
            })
            .collect();
        self.assets.borrow_mut().retain(&alive);
    }
}
