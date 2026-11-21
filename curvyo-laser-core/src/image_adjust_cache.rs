//! The grey pyramids of the pictures in use, keyed by their adjusted content (ADR 0073 §7, ADR
//! 0068 §3; `specs/0135` criteria 18 to 21, 60, 61).
//!
//! One entry per asset, reduced level and list of pixel steps: two clones, two operations at one
//! Interval and an undo share one adjusted picture, and an empty list is the `0123` pyramid of the
//! asset itself. The picture analysis a preset reads is kept per asset. The made pictures and the
//! half-done runs share one byte limit ([`ADJUSTED_BYTES`] for the job, [`LOOK_BYTES`] for the
//! canvas look): past it the least recently used pictures, then the least recently worked runs,
//! are dropped. Lookups only: nothing iterates the maps into output.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use curvyo_document_core::{AssetHash, Length, Point};
use curvyo_image_core::{AdjustRun, Adjusted, Analysis, PixelStep, WorkingImage, peak_bytes};

use crate::image_adjust_fields::ManualStep;
use crate::image_adjust_resolve::{AdjustRegisters, analysis_after, pixel_steps};
use crate::image_preset::Preset;
use crate::job_raster_sample::GreyPyramid;

/// The bytes the job's cache keeps at most, its made pictures and its half-done runs together,
/// before the least recently used are dropped (ADR 0073 §7, Proposal). With [`LOOK_BYTES`] the
/// adjustments of a session hold at most 320 MiB in wasm.
pub const ADJUSTED_BYTES: usize = 288 << 20;

/// The bytes the canvas look's own cache keeps at most, made pictures and runs together (ADR 0073
/// §7, Proposal): looks are at most 1 Mpx, so a few of them.
pub const LOOK_BYTES: usize = 32 << 20;

/// The work one budgeted step of the host gives an adjustment (`specs/0135` criterion 13): about
/// 15 to 30 ms of a 4 Mpx stack natively, so no tick runs a whole adjustment (units of
/// [`AdjustRun::advance`]).
pub const ADJUST_WORK: u64 = 12_000_000;

/// The bytes one adjustment may hold while it runs (`specs/0135` criterion 56, ADR 0073 §9,
/// Proposal): a 16 Mpx colour chain needs 2 x its 64 MB; a step past it is not run, with the
/// group's memory line.
pub const RUN_BYTES: u64 = 160 << 20;

/// The adjustments kept half done at most: a Strength drag leaves a run per value it passed, so
/// the least recently worked are dropped.
const RUNS_KEPT: usize = 2;

/// What one adjusted picture is made of: its asset, the reduced level it ran on (0 for the job)
/// and its pixel steps.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AdjustKey {
    /// The asset of the working image.
    pub asset: AssetHash,
    /// The halvings of the working image the steps ran on; 0 is the exact picture.
    pub level: u32,
    /// The steps in pixel units of that level.
    pub steps: Vec<PixelStep>,
}

/// The steps of one picture at one Interval, as resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolved {
    /// The steps that run, in stored units (what Make editable writes, criterion 29).
    pub steps: Vec<ManualStep>,
    /// The content key of the adjusted picture.
    pub key: AdjustKey,
}

/// One adjusted picture: its grey pyramid and, when the steps left it in colour, the colour
/// picture (Denoise alone; the Before and after pane and the canvas look read it).
#[derive(Debug, Clone)]
pub struct AdjustedPicture {
    /// The grey pyramid the dots read.
    pub pyramid: Arc<GreyPyramid>,
    /// The adjusted colour picture, `None` when unchanged or grey.
    pub colour: Option<Arc<WorkingImage>>,
    /// The first of the key's steps that did not fit [`RUN_BYTES`]; it and the steps after it
    /// were not run (criterion 56). `None` when every step ran.
    pub not_run: Option<usize>,
}

impl AdjustedPicture {
    fn bytes(&self) -> usize {
        let base = self.pyramid.base();
        let grey = base.width as usize * base.height as usize;
        let colour = self
            .colour
            .as_ref()
            .map_or(0, |c| c.width as usize * c.height as usize * 4);
        grey + grey / 3 + colour
    }
}

#[derive(Debug)]
struct Held {
    picture: AdjustedPicture,
    used: u64,
    bytes: usize,
}

#[derive(Debug, Default)]
struct Entries {
    /// By asset and the geometry steps the picture is analysed after (criterion 39).
    analyses: HashMap<(AssetHash, Vec<ManualStep>), Arc<Analysis>>,
    pictures: HashMap<AdjustKey, Held>,
    /// The adjustments being worked, least recently worked first, with the bytes each holds.
    running: Vec<(AdjustKey, AdjustRun, usize)>,
    clock: u64,
    bytes: usize,
    /// The bytes a run may hold; [`RUN_BYTES`] unless set.
    run_bytes: Option<u64>,
}

impl Entries {
    fn touch(&mut self, key: &AdjustKey) -> Option<AdjustedPicture> {
        self.clock += 1;
        let clock = self.clock;
        self.pictures.get_mut(key).map(|held| {
            held.used = clock;
            held.picture.clone()
        })
    }

    fn insert(&mut self, key: &AdjustKey, picture: AdjustedPicture, limit: usize) {
        self.clock += 1;
        let bytes = picture.bytes();
        self.bytes += bytes;
        let used = self.clock;
        if let Some(old) = self.pictures.insert(
            key.clone(),
            Held {
                picture,
                used,
                bytes,
            },
        ) {
            self.bytes -= old.bytes;
        }
        self.fit(key, limit);
    }

    /// Drops the least recently used pictures, then the least recently worked runs, other than
    /// `key`'s, until the pictures and runs together hold at most `limit` bytes.
    fn fit(&mut self, key: &AdjustKey, limit: usize) {
        loop {
            let running: usize = self.running.iter().map(|(_, _, bytes)| bytes).sum();
            if self.bytes + running <= limit {
                return;
            }
            let oldest = self
                .pictures
                .iter()
                .filter(|(k, _)| *k != key)
                .min_by_key(|(_, held)| held.used)
                .map(|(k, _)| k.clone());
            if let Some(oldest) = oldest {
                if let Some(dropped) = self.pictures.remove(&oldest) {
                    self.bytes -= dropped.bytes;
                }
            } else if let Some(at) = self.running.iter().position(|(k, _, _)| k != key) {
                self.running.remove(at);
            } else {
                return;
            }
        }
    }
}

/// The grey pyramids and analyses of the pictures in use, shared by the job and the Operations
/// view's preview (a clone of the cache shares its maps).
#[derive(Debug, Clone)]
pub struct PyramidCache {
    entries: Arc<Mutex<Entries>>,
    limit: usize,
}

impl Default for PyramidCache {
    fn default() -> Self {
        Self::with_limit(ADJUSTED_BYTES)
    }
}

impl PyramidCache {
    /// An empty cache that keeps at most `limit` bytes of adjusted pixels.
    #[must_use]
    pub fn with_limit(limit: usize) -> Self {
        Self {
            entries: Arc::new(Mutex::new(Entries::default())),
            limit,
        }
    }

    /// The shared maps; a panic while they were held leaves whole entries only, so they are read
    /// on.
    fn entries(&self) -> MutexGuard<'_, Entries> {
        self.entries.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Sets the bytes a run may hold for every clone of this cache (a test's small budget); the
    /// pictures made already stay.
    #[doc(hidden)]
    pub fn set_run_bytes(&self, bytes: u64) {
        self.entries().run_bytes = Some(bytes);
    }

    /// Whether resolving `registers` needs no work: Expert, or the analysis of `asset` is held.
    #[must_use]
    pub fn resolvable(&self, asset: &AssetHash, registers: &AdjustRegisters) -> bool {
        registers.preset == Preset::Expert
            || self
                .entries()
                .analyses
                .contains_key(&(asset.clone(), registers.geometry_steps()))
    }

    /// The picture analysis of `asset` with the working image `image` after the geometry steps
    /// `geometry` (criterion 39), computed on first use.
    #[must_use]
    pub fn analysis(
        &self,
        asset: &AssetHash,
        image: &WorkingImage,
        geometry: &[ManualStep],
    ) -> Arc<Analysis> {
        let key = (asset.clone(), geometry.to_vec());
        if let Some(held) = self.entries().analyses.get(&key) {
            return Arc::clone(held);
        }
        let analysis = Arc::new(analysis_after(image, geometry));
        self.entries().analyses.insert(key, Arc::clone(&analysis));
        analysis
    }

    /// The steps of the picture `asset` (working image `image`, placed in `frame`) with
    /// `registers` at `interval_um`, run on reduced level `level` (ADR 0073 §6, §8): the mm
    /// fields become pixels of that level, so a reduced picture runs the same mm.
    #[must_use]
    pub fn resolve(
        &self,
        (asset, image, frame): (&AssetHash, &WorkingImage, &[Point; 4]),
        registers: &AdjustRegisters,
        interval_um: u32,
        level: u32,
    ) -> Resolved {
        let steps = registers.effective_steps(interval_um, |geometry| {
            (*self.analysis(asset, image, geometry)).clone()
        });
        let input = reduced_size(image, if steps.is_empty() { 0 } else { level });
        let pixel = pixel_steps(&steps, frame, (image.width, image.height), input);
        let level = if pixel.is_empty() { 0 } else { level };
        Resolved {
            steps,
            key: AdjustKey {
                asset: asset.clone(),
                level,
                steps: pixel,
            },
        }
    }

    /// The adjusted picture of `key` when it is held; no work.
    #[must_use]
    pub fn held(&self, key: &AdjustKey) -> Option<AdjustedPicture> {
        self.entries().touch(key)
    }

    /// The adjusted picture of `key` from the working image `image`, made on first use.
    #[allow(clippy::must_use_candidate)] // also fills the cache; callers may only warm it
    pub fn picture(&self, key: &AdjustKey, image: &WorkingImage) -> AdjustedPicture {
        loop {
            if let Some(picture) = self.advance(key, image, u64::MAX) {
                return picture;
            }
        }
    }

    /// Works the adjusted picture of `key` from the working image `image` for about `work` units
    /// of [`AdjustRun::advance`] (`specs/0135` criterion 13): the picture once it is finished,
    /// `None` while it is half done (kept for the next call). The pyramid of an empty list is
    /// made at once.
    #[allow(clippy::must_use_candidate)] // a tick of the job works it without reading it
    pub fn advance(
        &self,
        key: &AdjustKey,
        image: &WorkingImage,
        work: u64,
    ) -> Option<AdjustedPicture> {
        if let Some(held) = self.held(key) {
            return Some(held);
        }
        // The reduced input is read when a run starts only, not on every step of it.
        let input = || {
            if key.level == 0 {
                std::borrow::Cow::Borrowed(image)
            } else {
                std::borrow::Cow::Owned(image.reduced_over_white(key.level))
            }
        };
        let picture = if key.steps.is_empty() {
            AdjustedPicture {
                pyramid: Arc::new(GreyPyramid::of(image)),
                colour: None,
                not_run: None,
            }
        } else {
            let started = {
                let mut entries = self.entries();
                let at = entries.running.iter().position(|(k, _, _)| k == key);
                at.map(|at| {
                    let (_, run, bytes) = entries.running.remove(at);
                    (run, bytes)
                })
            };
            let budget = self.entries().run_bytes.unwrap_or(RUN_BYTES);
            let (mut run, bytes) = started.unwrap_or_else(|| {
                let input = input();
                let pixels = u64::from(input.width) * u64::from(input.height);
                let run = AdjustRun::within(&input, &key.steps, budget);
                let ran = &key.steps[..run.not_run().unwrap_or(key.steps.len())];
                let bytes = usize::try_from(peak_bytes(pixels, ran)).unwrap_or(usize::MAX);
                (run, bytes)
            });
            if !run.advance(work) {
                let mut entries = self.entries();
                entries.running.push((key.clone(), run, bytes));
                if entries.running.len() > RUNS_KEPT {
                    entries.running.remove(0);
                }
                entries.fit(key, self.limit);
                return None;
            }
            let not_run = run.not_run();
            match run.finish() {
                Adjusted::Unchanged => AdjustedPicture {
                    pyramid: Arc::new(GreyPyramid::of(&input())),
                    colour: None,
                    not_run,
                },
                Adjusted::Rgba(rgba) => AdjustedPicture {
                    pyramid: Arc::new(GreyPyramid::of(&rgba)),
                    colour: Some(Arc::new(rgba)),
                    not_run,
                },
                Adjusted::Grey(plane) => AdjustedPicture {
                    pyramid: Arc::new(GreyPyramid::of_plane(plane)),
                    colour: None,
                    not_run,
                },
            }
        };
        self.entries().insert(key, picture.clone(), self.limit);
        Some(picture)
    }

    /// The pyramid of `asset` with the working image `image` and no step (the `0123` pyramid).
    pub fn pyramid(&mut self, asset: &AssetHash, image: &WorkingImage) -> Arc<GreyPyramid> {
        let key = AdjustKey {
            asset: asset.clone(),
            level: 0,
            steps: Vec::new(),
        };
        self.picture(&key, image).pyramid
    }

    /// Drops the pictures and analyses of assets not in `assets`.
    pub fn retain(&mut self, assets: &[&AssetHash]) {
        let assets: HashSet<&AssetHash> = assets.iter().copied().collect();
        let mut entries = self.entries();
        entries
            .analyses
            .retain(|(asset, _), _| assets.contains(asset));
        entries
            .pictures
            .retain(|key, _| assets.contains(&key.asset));
        entries
            .running
            .retain(|(key, _, _)| assets.contains(&key.asset));
        entries.bytes = entries.pictures.values().map(|held| held.bytes).sum();
    }

    /// How many pictures are kept.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries().pictures.len()
    }

    /// Whether no picture is kept.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// The size of `image` after `level` halvings (each side halved rounding down, at least 1).
fn reduced_size(image: &WorkingImage, level: u32) -> (u32, u32) {
    (0..level).fold((image.width, image.height), |(w, h), _| {
        ((w / 2).max(1), (h / 2).max(1))
    })
}

/// The reduced level whose pixel pitch is at most half of `dot` for a picture whose full pixel
/// pitch is `pitch` (ADR 0073 §8: the mip that fits a coarse preview); 0 when the full picture is
/// needed.
#[must_use]
pub fn reduced_level(pitch: Length, dot: Length) -> u32 {
    let dot_mm = dot.as_mm();
    let mut level = 0;
    let mut pitch = pitch.as_mm();
    while pitch > 0.0 && pitch * 2.0 <= dot_mm / 2.0 && level < 16 {
        pitch *= 2.0;
        level += 1;
    }
    level
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image_adjust_fields::StepKind;
    use crate::image_adjust_resolve::StoredStep;

    fn image(side: u32) -> WorkingImage {
        WorkingImage {
            width: side,
            height: side,
            pixels: (0..side * side)
                .flat_map(|i| {
                    let g = u8::try_from(i % side * 255 / side.max(2)).unwrap_or(0);
                    [g, g, g, 255]
                })
                .collect(),
            original_size: (side, side),
            truncated: false,
            animated: false,
        }
    }

    fn frame(mm: f64) -> [Point; 4] {
        [
            Point::new(0.0, 0.0),
            Point::new(mm, 0.0),
            Point::new(mm, mm),
            Point::new(0.0, mm),
        ]
    }

    fn levels() -> AdjustRegisters {
        AdjustRegisters {
            steps: vec![StoredStep {
                enabled: true,
                step: Some(StepKind::Levels.default_step(100)),
            }],
            ..AdjustRegisters::default()
        }
    }

    /// Criterion 18: one adjusted picture per content; the empty list is the asset's pyramid.
    #[test]
    fn equal_content_shares_one_picture() {
        let cache = PyramidCache::default();
        let img = image(16);
        let asset = AssetHash::of(&img.pixels);
        let a = cache.resolve((&asset, &img, &frame(1.6)), &levels(), 100, 0);
        let b = cache.resolve((&asset, &img, &frame(1.6)), &levels(), 200, 0);
        assert_eq!(a.key, b.key, "a manual stack is the same at every Interval");
        let first = cache.picture(&a.key, &img);
        let second = cache.picture(&b.key, &img);
        assert!(Arc::ptr_eq(&first.pyramid, &second.pyramid));
        let empty = cache.resolve(
            (&asset, &img, &frame(1.6)),
            &AdjustRegisters::default(),
            100,
            3,
        );
        assert_eq!((empty.key.level, empty.key.steps.len()), (0, 0));
        let plain = cache.clone().pyramid(&asset, &img);
        assert_eq!(*plain, GreyPyramid::of(&img), "the 0123 pyramid");
        assert_eq!(cache.len(), 2);
    }

    /// Criterion 56: a step past the run budget is not run, and the picture says from which.
    #[test]
    fn a_step_past_the_run_budget_is_not_run_and_named() {
        let cache = PyramidCache::default();
        let img = image(16);
        let asset = AssetHash::of(&img.pixels);
        let key = cache
            .resolve((&asset, &img, &frame(1.6)), &levels(), 100, 0)
            .key;
        assert_eq!(cache.picture(&key, &img).not_run, None);
        let small = PyramidCache::default();
        small.clone().set_run_bytes(5 * 256);
        let picture = small.picture(&key, &img);
        assert_eq!(picture.not_run, Some(0), "Levels needs 6 x 256 bytes");
        assert_eq!(
            *picture.pyramid,
            GreyPyramid::of(&img),
            "the picture as imported"
        );
    }

    #[test]
    fn a_preset_needs_the_analysis_once() {
        let cache = PyramidCache::default();
        let img = image(16);
        let asset = AssetHash::of(&img.pixels);
        let photo = AdjustRegisters {
            preset: Preset::Photo,
            ..AdjustRegisters::default()
        };
        assert!(cache.resolvable(&asset, &levels()));
        assert!(!cache.resolvable(&asset, &photo));
        let at_1 = cache.resolve((&asset, &img, &frame(1.6)), &photo, 100, 0);
        assert!(cache.resolvable(&asset, &photo));
        let at_2 = cache.resolve((&asset, &img, &frame(1.6)), &photo, 200, 0);
        assert_ne!(
            at_1.key, at_2.key,
            "criterion 60: the preset follows the Interval"
        );
    }

    #[test]
    fn the_least_recently_used_picture_is_dropped_past_the_limit() {
        let img = image(64);
        let asset = AssetHash::of(&img.pixels);
        let one = AdjustedPicture {
            pyramid: Arc::new(GreyPyramid::of(&img)),
            colour: None,
            not_run: None,
        }
        .bytes();
        let cache = PyramidCache::with_limit(one * 2);
        let keys: Vec<AdjustKey> = [10_u8, 20, 30]
            .iter()
            .map(|&black| AdjustKey {
                asset: asset.clone(),
                level: 0,
                steps: vec![PixelStep::Levels {
                    black_pct: black,
                    white_pct: 100,
                    gamma_centi: 100,
                }],
            })
            .collect();
        cache.picture(&keys[0], &img);
        cache.picture(&keys[1], &img);
        assert!(
            cache.held(&keys[0]).is_some(),
            "touched, so the second is older"
        );
        cache.picture(&keys[2], &img);
        assert!(cache.held(&keys[1]).is_none());
        assert!(cache.held(&keys[0]).is_some() && cache.held(&keys[2]).is_some());
        let mut kept = cache.clone();
        kept.retain(&[]);
        assert!(cache.is_empty());
    }

    #[test]
    fn a_reduced_level_runs_the_same_mm_on_fewer_pixels() {
        let mm = Length::from_mm;
        assert_eq!(reduced_level(mm(0.025), mm(0.1)), 1);
        assert_eq!(reduced_level(mm(0.025), mm(0.4)), 3);
        assert_eq!(reduced_level(mm(0.1), mm(0.1)), 0);
        let cache = PyramidCache::default();
        let img = image(64);
        let asset = AssetHash::of(&img.pixels);
        let sharpen = AdjustRegisters {
            steps: vec![StoredStep {
                enabled: true,
                step: Some(StepKind::Sharpen.default_step(200)),
            }],
            ..AdjustRegisters::default()
        };
        let full = cache.resolve((&asset, &img, &frame(6.4)), &sharpen, 100, 0);
        let half = cache.resolve((&asset, &img, &frame(6.4)), &sharpen, 100, 1);
        let radius = |k: &AdjustKey| match k.steps[0] {
            PixelStep::Sharpen { radius, .. } => radius,
            _ => unreachable!(),
        };
        assert_eq!(radius(&full.key).0, 2 * radius(&half.key).0);
        let picture = cache.picture(&half.key, &img);
        assert_eq!(picture.pyramid.base().width, 32);
    }

    /// Criterion 13: a picture worked in bands is the one-shot picture, and a drag's runs are
    /// bounded.
    #[test]
    fn a_picture_worked_in_bands_is_the_one_shot_picture() {
        let img = image(64);
        let asset = AssetHash::of(&img.pixels);
        let sharpen = AdjustRegisters {
            steps: vec![StoredStep {
                enabled: true,
                step: Some(StepKind::Sharpen.default_step(100)),
            }],
            ..AdjustRegisters::default()
        };
        let stepped = PyramidCache::default();
        let key = stepped
            .resolve((&asset, &img, &frame(6.4)), &sharpen, 100, 1)
            .key;
        let mut calls = 0;
        let picture = loop {
            calls += 1;
            if let Some(picture) = stepped.advance(&key, &img, 64) {
                break picture;
            }
        };
        assert!(calls > 10, "{calls}");
        let once = PyramidCache::default().picture(&key, &img);
        assert_eq!(*picture.pyramid, *once.pyramid);
        let other = |black: u8| AdjustKey {
            asset: asset.clone(),
            level: 0,
            steps: vec![PixelStep::Levels {
                black_pct: black,
                white_pct: 100,
                gamma_centi: 100,
            }],
        };
        for black in 1..6 {
            assert!(stepped.advance(&other(black), &img, 1).is_none());
        }
        assert_eq!(stepped.entries().running.len(), RUNS_KEPT);
    }

    /// ADR 0073 §7: the half-done runs count in the cache's one byte limit with its pictures.
    #[test]
    fn half_done_runs_share_the_byte_limit_with_the_pictures() {
        let img = image(64);
        let asset = AssetHash::of(&img.pixels);
        let key = |black: u8| AdjustKey {
            asset: asset.clone(),
            level: 0,
            steps: vec![
                PixelStep::Levels {
                    black_pct: black,
                    white_pct: 100,
                    gamma_centi: 100,
                },
                PixelStep::Sharpen {
                    radius: curvyo_image_core::SubPx(16),
                    amount_pct: 100,
                    threshold_pct: 0,
                },
            ],
        };
        let run = usize::try_from(peak_bytes(64 * 64, &key(1).steps)).unwrap();
        let made = PyramidCache::default().picture(&key(9), &img).bytes();
        // Room for a picture and one run, or two runs, not for a picture and two runs.
        assert!(made / 2 < run);
        let cache = PyramidCache::with_limit(2 * run + made / 2);
        cache.picture(&key(9), &img);
        assert!(cache.advance(&key(1), &img, 1).is_none());
        assert!(cache.held(&key(9)).is_some(), "a picture and a run fit");
        assert!(cache.advance(&key(2), &img, 1).is_none());
        assert!(cache.held(&key(9)).is_none(), "the picture goes first");
        assert_eq!(cache.entries().running.len(), 2);
        let small = PyramidCache::with_limit(run + run / 2);
        assert!(small.advance(&key(1), &img, 1).is_none());
        assert!(small.advance(&key(2), &img, 1).is_none());
        let running: Vec<AdjustKey> = small
            .entries()
            .running
            .iter()
            .map(|(k, _, _)| k.clone())
            .collect();
        assert_eq!(running, [key(2)], "then the older run");
        let worst = ADJUSTED_BYTES + LOOK_BYTES;
        assert!(worst <= 320 << 20, "{worst}");
    }
}
