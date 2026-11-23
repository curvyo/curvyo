//! The canvas look of adjusted pictures (`specs/0135` criterion 15): an image whose adjustments
//! are not empty draws its adjusted picture, made at the display level in the host's ticks (never
//! in the frame of an edit) and kept by content; until it is made the image shows its last look,
//! or the picture as imported. Only the looks drawn in the last frame wait to be made, the newest
//! first, and a look keeps the picture's transparency.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use curvyo_document_core::{AssetHash, DocumentVersion, ImageSnapshot, NodeId, ObjectSnapshot};
use curvyo_image_core::WorkingImage;
use curvyo_laser_core::adjust::{
    ADJUST_WORK, AdjustKey, AdjustRegisters, AdjustedPicture, LOOK_BYTES, PyramidCache,
};

use super::picture::UNASSIGNED_INTERVAL_UM;

/// The most pixels of a picture's canvas look (Proposal): the first halving of the working image
/// with at most 1 Mpx, so a look costs a quarter of the exact picture's adjustment.
const LOOK_PIXELS: u64 = 1 << 20;

/// The content keys kept at most before the memo is cleared (a Strength drag adds one per value).
const KEYS_KEPT: usize = 4_096;

/// What one image's look is made from: its asset, its adjustments as read (with a held
/// Strength), its frame and the Interval of its first Image operation.
type ContentKey = (AssetHash, String, u32);

/// The Intervals of the pictures in Image operations, with what they were read for.
type Intervals = (
    (DocumentVersion, Option<NodeId>, u64),
    Arc<HashMap<NodeId, u32>>,
);

/// The canvas looks of the session, never saved.
#[derive(Debug)]
pub(in crate::session) struct LookCache {
    /// Its own adjusted pictures within [`LOOK_BYTES`], so a laser page's plans never drop a look
    /// half made.
    pyramids: PyramidCache,
    keys: RefCell<HashMap<ContentKey, Option<AdjustKey>>>,
    intervals: RefCell<Option<Intervals>>,
    /// The made looks by name, and the name of each made look.
    names: RefCell<HashMap<AssetHash, Arc<WorkingImage>>>,
    named: RefCell<HashMap<AdjustKey, AssetHash>>,
    /// The look each image drew last, while its next one is made.
    shown: RefCell<HashMap<NodeId, AssetHash>>,
    /// The looks drawn in the last frame and not made yet, newest first, with their working
    /// images.
    pending: RefCell<Vec<(AdjustKey, Arc<WorkingImage>)>>,
}

impl Default for LookCache {
    fn default() -> Self {
        Self {
            pyramids: PyramidCache::with_limit(LOOK_BYTES),
            keys: RefCell::default(),
            intervals: RefCell::default(),
            names: RefCell::default(),
            named: RefCell::default(),
            shown: RefCell::default(),
            pending: RefCell::default(),
        }
    }
}

/// What one frame drew: the made looks it shows and the keys of the looks it asked for.
#[derive(Default)]
struct Drawn {
    names: HashSet<AssetHash>,
    keys: HashSet<AdjustKey>,
}

/// The halvings of a `width` x `height` picture down to at most [`LOOK_PIXELS`].
fn look_level(width: u32, height: u32) -> u32 {
    let (mut w, mut h, mut level) = (u64::from(width), u64::from(height), 0);
    while w * h > LOOK_PIXELS && (w > 1 || h > 1) {
        w = (w / 2).max(1);
        h = (h / 2).max(1);
        level += 1;
    }
    level
}

/// The look of an adjusted picture made from `source` at reduced level `level`: its colour, or
/// its grey, with the source's alpha at that level (the dots composite over white; the canvas
/// shows the picture's transparency, ADR 0073 §8).
fn look_image(picture: &AdjustedPicture, source: &WorkingImage, level: u32) -> WorkingImage {
    let mut look = if let Some(colour) = &picture.colour {
        (**colour).clone()
    } else {
        let base = picture.pyramid.base();
        WorkingImage {
            width: base.width,
            height: base.height,
            pixels: base.grey.iter().flat_map(|&g| [g, g, g, 255]).collect(),
            original_size: (base.width, base.height),
            truncated: false,
            animated: false,
        }
    };
    if let Some(alpha) = source.reduced_alpha(level)
        && alpha.len() * 4 == look.pixels.len()
    {
        for (pixel, a) in look.pixels.as_chunks_mut::<4>().0.iter_mut().zip(alpha) {
            pixel[3] = a;
        }
    }
    look
}

impl crate::session::Session {
    /// `objects` with each adjusted image drawing its look (criterion 15); `None` when no image
    /// changes, so the frame keeps its list.
    pub(in crate::session) fn adjusted_artwork(
        &self,
        objects: &[ObjectSnapshot],
    ) -> Option<Vec<ObjectSnapshot>> {
        let adjusted = |image: &ImageSnapshot| image.adjustments.is_adjusted();
        let any = objects.iter().any(|object| match object {
            ObjectSnapshot::Image(image) => adjusted(image),
            _ => object.picture().is_some_and(|picture| {
                picture
                    .iter()
                    .any(|o| matches!(o, ObjectSnapshot::Image(i) if adjusted(i)))
            }),
        });
        if !any {
            self.image_adjust.look.shown.borrow_mut().clear();
            self.image_adjust.look.pending.borrow_mut().clear();
            return None;
        }
        let mut used = Drawn::default();
        let mut out = objects.to_vec();
        for object in &mut out {
            let owner = object.id();
            match object {
                ObjectSnapshot::Image(image) => self.look_image_into(owner, image, &mut used),
                ObjectSnapshot::Clone(clone) => {
                    let Some(picture) = clone.chain.picture.as_ref() else {
                        continue;
                    };
                    let mut leaves = picture.to_vec();
                    for leaf in &mut leaves {
                        if let ObjectSnapshot::Image(image) = leaf {
                            self.look_image_into(owner, image, &mut used);
                        }
                    }
                    clone.chain.picture = Some(Arc::from(leaves));
                }
                _ => {}
            }
        }
        let look = &self.image_adjust.look;
        let Drawn { names, keys } = used;
        look.names
            .borrow_mut()
            .retain(|name, _| names.contains(name));
        look.named
            .borrow_mut()
            .retain(|_, name| names.contains(name));
        look.shown
            .borrow_mut()
            .retain(|_, name| names.contains(name));
        // A look no image draws any more (a Strength drag's passed values) is not made.
        look.pending
            .borrow_mut()
            .retain(|(key, _)| keys.contains(key));
        Some(out)
    }

    /// Points `image` (drawn for object `owner`) at its look when it is made, else at its last
    /// look; queues the look when it is not made yet.
    fn look_image_into(&self, owner: NodeId, image: &mut ImageSnapshot, used: &mut Drawn) {
        if !image.adjustments.is_adjusted() {
            return;
        }
        let look = &self.image_adjust.look;
        let Some(key) = self.look_key(owner, image) else {
            look.shown.borrow_mut().remove(&image.id);
            return;
        };
        let made = look.named.borrow().get(&key).cloned();
        let shown = if let Some(name) = made {
            look.shown.borrow_mut().insert(image.id, name.clone());
            Some(name)
        } else {
            let mut pending = look.pending.borrow_mut();
            if !pending.iter().any(|(k, _)| *k == key)
                && let Some(pixels) = self.job_pixels(&image.asset)
            {
                pending.insert(0, (key.clone(), pixels));
            }
            used.keys.insert(key);
            look.shown.borrow().get(&image.id).cloned()
        };
        if let Some(name) = shown {
            used.names.insert(name.clone());
            image.asset = name;
            image.shows_look = true;
        }
    }

    /// The content key of `image`'s look, drawn for `owner`; `None` when no step runs.
    fn look_key(&self, owner: NodeId, image: &ImageSnapshot) -> Option<AdjustKey> {
        let mut adjustments = image.adjustments.clone();
        if let Some((held, strength)) = self.held_adjust_strength()
            && (held == owner || held == image.id)
        {
            adjustments.strength = strength;
        }
        let interval_um = self
            .look_intervals()
            .get(&owner)
            .copied()
            .unwrap_or(UNASSIGNED_INTERVAL_UM);
        let corners = image.corners();
        let content = (
            image.asset.clone(),
            format!("{adjustments:?} {corners:?}"),
            interval_um,
        );
        let look = &self.image_adjust.look;
        if let Some(key) = look.keys.borrow().get(&content) {
            return key.clone();
        }
        let key = self.job_pixels(&image.asset).and_then(|pixels| {
            let registers = AdjustRegisters::read(&adjustments);
            self.hold_stored_results(&look.pyramids, &registers);
            let level = look_level(pixels.width, pixels.height);
            let resolved = look.pyramids.resolve(
                (&image.asset, &pixels, &corners),
                &registers,
                interval_um,
                level,
            );
            (!resolved.key.steps.is_empty()).then_some(resolved.key)
        });
        let mut keys = look.keys.borrow_mut();
        if keys.len() >= KEYS_KEPT {
            keys.clear();
        }
        keys.insert(content, key.clone());
        key
    }

    /// The Interval of each picture's first Image operation on the active laser page (criterion
    /// 60), by the node its leaf is drawn for; read once per document version.
    fn look_intervals(&self) -> Arc<HashMap<NodeId, u32>> {
        let at = (
            self.document.version(),
            self.active_manufacturing().map(|m| m.id),
            self.materials.library_generation(),
        );
        let look = &self.image_adjust.look;
        if let Some((held, intervals)) = look.intervals.borrow().as_ref()
            && *held == at
        {
            return Arc::clone(intervals);
        }
        let intervals = self
            .with_page_job_input(|input| self.first_intervals(input))
            .unwrap_or_default();
        let intervals = Arc::new(intervals);
        *look.intervals.borrow_mut() = Some((at, Arc::clone(&intervals)));
        intervals
    }

    /// Whether a canvas look waits to be made in the host's ticks.
    #[must_use]
    pub fn canvas_look_pending(&self) -> bool {
        !self.image_adjust.look.pending.borrow().is_empty()
    }

    /// One tick's share of the newest look waiting; whether it was finished, so the host draws
    /// again.
    pub fn advance_canvas_look(&self) -> bool {
        let look = &self.image_adjust.look;
        let Some((key, pixels)) = look.pending.borrow().first().cloned() else {
            return false;
        };
        let Some(picture) = look.pyramids.advance(&key, &pixels, ADJUST_WORK) else {
            return false;
        };
        let name = AssetHash::of(format!("curvyo adjusted look {key:?}").as_bytes());
        look.names.borrow_mut().insert(
            name.clone(),
            Arc::new(look_image(&picture, &pixels, key.level)),
        );
        look.named.borrow_mut().insert(key.clone(), name);
        look.pending.borrow_mut().retain(|(k, _)| *k != key);
        true
    }

    /// The made look named `asset`, for the frame's textures.
    pub(in crate::session) fn look_picture(&self, asset: &AssetHash) -> Option<Arc<WorkingImage>> {
        self.image_adjust.look.names.borrow().get(asset).cloned()
    }

    /// Whether the host's ticks have work: the pending job (`specs/0123` criterion 37 d) or a
    /// canvas look (criterion 15).
    #[must_use]
    pub fn ticks_pending(&self) -> bool {
        self.job_pending() || self.canvas_look_pending()
    }

    /// One tick of the host: the pending job's share first, else a look's; whether the views or
    /// the canvas should be read again.
    pub fn advance_tick(&self) -> bool {
        if self.job_pending() {
            self.advance_job()
        } else {
            self.advance_canvas_look()
        }
    }
}

#[cfg(test)]
#[path = "look_tests.rs"]
mod tests;
