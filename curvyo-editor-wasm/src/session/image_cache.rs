//! The working images of the canvas (ADR 0017 §5, `specs/0058-bitmap-import` criteria 30 and
//! 46): decoded from the document's assets on first draw (or handed over by the import), keyed by
//! hash, dropped when no drawn image names them any more; an asset that cannot be decoded, or
//! whose bytes do not hash to its name, is remembered as unreadable and announced once.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use curvyo_document_core::{AssetHash, Document};
use curvyo_image_core::WorkingImage;
use curvyo_render_core::DrawList;

use super::Session;

/// One cached picture.
#[derive(Debug, Clone)]
enum Entry {
    Ready(Arc<WorkingImage>),
    Unreadable,
}

/// The working images of one session.
#[derive(Debug, Clone, Default)]
pub(super) struct ImageCache {
    entries: HashMap<AssetHash, Entry>,
    /// An unreadable asset was met and its notice is waiting for the host.
    bad_asset_pending: bool,
    /// The notice was given in this session (once per open).
    bad_asset_told: bool,
}

impl ImageCache {
    /// Keeps the picture an import just decoded.
    pub(super) fn insert(&mut self, hash: AssetHash, working: Arc<WorkingImage>) {
        self.entries.insert(hash, Entry::Ready(working));
    }

    /// The working image of `hash`, decoded from `document`'s asset when it is not cached.
    fn working(&mut self, document: &Document, hash: &AssetHash) -> Option<Arc<WorkingImage>> {
        let entry = self.entries.entry(hash.clone()).or_insert_with(|| {
            document
                .asset(hash)
                .filter(|bytes| AssetHash::of(bytes) == *hash)
                .and_then(|bytes| curvyo_image_core::decode_working(&bytes).ok())
                .map_or(Entry::Unreadable, |working| Entry::Ready(Arc::new(working)))
        });
        match entry {
            Entry::Ready(working) => Some(working.clone()),
            Entry::Unreadable => {
                if !self.bad_asset_told {
                    self.bad_asset_pending = true;
                }
                None
            }
        }
    }

    /// Drops every picture no hash of `alive` names.
    fn retain(&mut self, alive: &HashSet<&AssetHash>) {
        self.entries.retain(|hash, _| alive.contains(hash));
    }
}

/// The pictures one frame draws, by the image layers of its draw list.
pub type FrameImages = Vec<(AssetHash, Arc<WorkingImage>)>;

impl Session {
    /// The working image of every image `list` draws, decoded where needed; an unreadable one is
    /// drawn as its frame with a cross over the artwork instead (criterion 30). Pictures no image
    /// of the frame draws are released: a hidden or deleted image decodes again when it is drawn.
    pub fn frame_images(&mut self, list: &mut DrawList) -> FrameImages {
        let quads = list.images.clone();
        let mut ready = Vec::new();
        for quad in &quads {
            match self.images.working(&self.document, &quad.asset) {
                Some(working) => ready.push((quad.asset.clone(), working)),
                None => list.extend(curvyo_render_core::build_unreadable_frame(
                    quad.corners,
                    self.view(),
                )),
            }
        }
        let alive: HashSet<&AssetHash> = quads.iter().map(|quad| &quad.asset).collect();
        self.images.retain(&alive);
        ready
    }

    /// "An image in this project could not be read." once per open, when a drawn image's asset
    /// could not be read; empty otherwise.
    pub fn take_bad_asset_notice(&mut self) -> String {
        if !self.images.bad_asset_pending {
            return String::new();
        }
        self.images.bad_asset_pending = false;
        self.images.bad_asset_told = true;
        curvyo_ui_core::BAD_ASSET_NOTICE.to_owned()
    }
}
