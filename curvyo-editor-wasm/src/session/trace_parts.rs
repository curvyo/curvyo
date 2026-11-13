//! The parts of a composite trace in the Trace tool (`specs/0094-masked-composite-trace` slice 2;
//! `specs/0114-image-trace-mask`): the parts as the image stores them (ADR 0061), each part's
//! clip of its stored regions, the active part, and the join of the parts' results into the
//! preview and the result Create paths writes.

use std::cell::{Cell, RefCell};

use curvyo_document_core::{NodeId, Point, TracePartSettings};
use curvyo_ui_core::{
    RegionOperand, TracePartArea, TracePartClip, join_trace_parts, read_trace_wire, region_operand,
    trace_part_clips,
};
use vectrace::Output;

use super::trace_mask_tool::MaskPart;
use super::trace_region::RegionCache;
use super::{Session, Tool, TraceSource};

/// At most this many parts (spec, Proposals).
pub const TRACE_PARTS_MAX: usize = 8;

/// How many working-image pixels a part's crop reaches past its clip's box at least (criterion
/// 31): enough that Denoise, the smoothing and the fit near the crop border stay outside the clip
/// (Proposal).
const CROP_MARGIN_PX: f64 = 16.0;

/// The tool state of the parts: the active one, and which image the parts are read for. The
/// parts themselves are the document's.
#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct TraceParts {
    active: usize,
    /// While a multi-image trace reads one image's parts (Frage 11 A: each image with its own
    /// mask): that image, else the one traced image.
    pub(super) for_image: Cell<Option<NodeId>>,
    /// The regions of recent operand sets over the image, so a slider move unites nothing.
    regions: RefCell<Vec<RegionCache>>,
    /// The clips of the last parts' areas over the image.
    clips: RefCell<Option<ClipCache>>,
}

/// The parts' areas over the image's corners and the clips made of them.
type ClipCache = (Vec<TracePartArea>, [Point; 4], Vec<TracePartClip>);

/// The added and the subtracted operands of a part's mask.
pub(super) type SignedOperands = (Vec<RegionOperand>, Vec<RegionOperand>);

/// One chip of the Parts row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)] // the chip's independent states, as the host reads them
pub struct TracePartView {
    /// Stable across renumbering: the host keeps the part's results under it.
    pub id: u32,
    /// Stored regions.
    pub regions: usize,
    /// Outside is chosen.
    pub outside: bool,
    /// The first part without regions: the whole image.
    pub whole: bool,
    /// A later part without regions: it adds nothing.
    pub muted: bool,
    pub active: bool,
    /// Its clip keeps some of the image.
    pub contributes: bool,
}

impl Session {
    /// How many parts there are.
    #[must_use]
    pub fn trace_part_count(&self) -> usize {
        self.trace_mask_parts().len()
    }

    /// The index of the active part.
    #[must_use]
    pub fn trace_active_part(&self) -> usize {
        self.trace
            .parts
            .active
            .min(self.trace_mask_parts().len().saturating_sub(1))
    }

    /// The added and the subtracted operands of `part`'s regions over the traced image.
    pub(super) fn trace_part_operands(&self, part: &MaskPart) -> SignedOperands {
        let Some(map) = self.trace_mask_image().map(|image| image.placement) else {
            return (Vec::new(), Vec::new());
        };
        let (mut added, mut subtracted) = (Vec::new(), Vec::new());
        for region in &part.regions {
            let operand = region_operand(region, map);
            if region.subtract {
                subtracted.push(operand);
            } else {
                added.push(operand);
            }
        }
        (added, subtracted)
    }

    /// What part `index` asks to keep over the image with the corners `quad`.
    fn trace_part_area(&self, index: usize, quad: [Point; 4]) -> TracePartArea {
        let Some(part) = self.trace_mask_parts().into_iter().nth(index) else {
            return TracePartArea::Nothing;
        };
        if part.regions.is_empty() {
            return if index == 0 {
                TracePartArea::Whole
            } else {
                TracePartArea::Nothing
            };
        }
        let cache = self.trace_part_region(self.trace_part_operands(&part), quad);
        match cache.region() {
            Ok(region) if cache.meets() => {
                TracePartArea::Mask(region.clone().with_outside(part.outside))
            }
            _ => TracePartArea::Nothing,
        }
    }

    /// The region of `operands` over `quad`, from the recent ones when it was made before.
    fn trace_part_region(&self, operands: SignedOperands, quad: [Point; 4]) -> RegionCache {
        let mut regions = self.trace.parts.regions.borrow_mut();
        if let Some(found) = regions.iter().find(|c| c.made_of(&operands, quad)) {
            return found.clone();
        }
        let made = RegionCache::new(operands, quad);
        if regions.len() > TRACE_PARTS_MAX {
            regions.remove(0);
        }
        regions.push(made.clone());
        made
    }

    /// Each part's clip once the later parts are taken out (`curvyo_ui_core::trace_part_clips`);
    /// all nothing without one traced image.
    #[must_use]
    pub fn trace_part_clip_list(&self) -> Vec<TracePartClip> {
        let count = self.trace_part_count();
        let Some(image) = self.trace_mask_image() else {
            return vec![TracePartClip::Nothing; count];
        };
        let quad = image.corners();
        let areas: Vec<TracePartArea> = (0..count)
            .map(|index| self.trace_part_area(index, quad))
            .collect();
        let mut clips = self.trace.parts.clips.borrow_mut();
        match clips.as_ref() {
            Some((a, q, made)) if *a == areas && *q == quad => made.clone(),
            _ => {
                let made = trace_part_clips(&areas, &quad);
                *clips = Some((areas, quad, made.clone()));
                made
            }
        }
    }

    /// Part `index`'s clip for the worker's settings (`"mask"`, as `trace_mask_json`): empty for
    /// the whole image and for a part that adds nothing.
    #[must_use]
    pub fn trace_part_mask_json(&self, index: usize) -> String {
        self.trace_part_clip_list()
            .get(index)
            .and_then(TracePartClip::region)
            .map_or_else(String::new, |region| {
                serde_json::json!({
                    "outside": region.outside(),
                    "region": region.to_numbers(),
                })
                .to_string()
            })
    }

    /// The chips of the Parts row (criteria 18, 19, 27, 32).
    #[must_use]
    pub fn trace_parts(&self) -> Vec<TracePartView> {
        let clips = self.trace_part_clip_list();
        let active = self.trace_active_part();
        self.trace_mask_parts()
            .iter()
            .enumerate()
            .map(|(index, part)| {
                let empty = part.regions.is_empty();
                TracePartView {
                    id: part.chip,
                    regions: part.regions.len(),
                    outside: part.outside,
                    whole: index == 0 && empty,
                    muted: index > 0 && empty,
                    active: index == active,
                    contributes: clips.get(index).is_some_and(TracePartClip::contributes),
                }
            })
            .collect()
    }

    /// Makes part `index` active; no region is selected.
    fn load_part(&mut self, index: usize) {
        self.trace.parts.active = index;
        self.trace.mask.selected = None;
    }

    /// Add part (criterion 19, `0114` ADR 0061 §3): a new last part, active, Inside, with the
    /// bar's settings without the picked colours, in one step "Add part"; an untouched image
    /// stores its first part in the same step. A lasso in progress is closed first. The new
    /// chip's id, or `None` at 8 parts (`TRACE_PARTS_MAX`), on a locked image or without one
    /// traced image.
    pub fn add_trace_part(&mut self) -> Option<u32> {
        if self.tool != Tool::Trace
            || self.trace_part_count() >= TRACE_PARTS_MAX
            || self.trace_mask_locked()
        {
            return None;
        }
        self.close_trace_lasso();
        let image = self.trace_mask_owner_id()?;
        let implicit = self.trace_implicit_part();
        let settings: TracePartSettings =
            curvyo_ui_core::without_picked_colours(&self.trace.mask.settings);
        let id = self.recording(|this| {
            this.document
                .add_trace_part(image, &implicit, &settings)
                .ok()
        })?;
        let parts = self.trace_mask_parts();
        let index = parts
            .iter()
            .position(|p| p.stored.as_deref() == Some(&id))?;
        let chip = parts[index].chip;
        self.load_part(index);
        Some(chip)
    }

    /// A chip pick (criterion 19): part `index` becomes active, a lasso in progress is closed
    /// into the part before; `false` when there is no such part or no traced image.
    pub fn set_trace_active_part(&mut self, index: usize) -> bool {
        if self.tool != Tool::Trace
            || index >= self.trace_part_count()
            || self.trace_mask_image().is_none()
        {
            return false;
        }
        if index != self.trace_active_part() {
            self.close_trace_lasso();
            self.load_part(index);
        }
        true
    }

    /// Remove part (criterion 28, `0114` criterion 20): part `index` with its regions and settings
    /// in one step "Remove part"; the part before it (or the new first) becomes active; a lasso
    /// in progress is closed first. `false` with one part or on a locked image.
    pub fn remove_trace_part(&mut self, index: usize) -> bool {
        if self.tool != Tool::Trace || self.trace_part_count() < 2 || self.trace_mask_locked() {
            return false;
        }
        self.close_trace_lasso();
        let Some(image) = self.trace_mask_owner_id() else {
            return false;
        };
        let parts = self.trace_mask_parts();
        let Some(MaskPart {
            stored: Some(id), ..
        }) = parts.get(index).cloned()
        else {
            return false;
        };
        let active = self.trace_active_part();
        let removed = self.recording(|this| this.document.remove_trace_part(image, &id).is_ok());
        if !removed {
            return false;
        }
        let next = match index.cmp(&active) {
            std::cmp::Ordering::Equal => index.saturating_sub(1),
            std::cmp::Ordering::Less => active - 1,
            std::cmp::Ordering::Greater => active,
        };
        self.load_part(next);
        true
    }

    /// The parts' results `wires` (one per part, in order; `None` for a part that adds nothing)
    /// joined and shown as the preview of `image`, with `full` also kept for Create paths; the
    /// joined result's header numbers, or `None` when a wire is not a result.
    pub fn set_trace_joined(
        &mut self,
        image: NodeId,
        wires: &[Option<&[f64]>],
        full: bool,
    ) -> Option<Vec<f64>> {
        let clips = self.for_trace_image(image, Self::trace_part_clip_list);
        let empty = Output {
            status: vectrace::wire::Status::Traced,
            paths: 0,
            nodes: 0,
            specks_removed: 0,
            foreground_pixels: 0,
            covers_whole_image: false,
            outlines: Vec::new(),
            palette: Vec::new(),
            layers: Vec::new(),
            joined: 0,
            texts: vectrace::wire::Texts::default(),
            shapes: Vec::new(),
            unclipped: 0,
        };
        let mut read: Vec<(Output, TracePartClip)> = Vec::with_capacity(wires.len());
        for (index, wire) in wires.iter().enumerate() {
            let clip = clips.get(index).cloned().unwrap_or(TracePartClip::Nothing);
            match wire {
                Some(numbers) if clip.contributes() => read.push((read_trace_wire(numbers)?, clip)),
                _ => read.push((empty.clone(), TracePartClip::Nothing)),
            }
        }
        let pairs: Vec<(&Output, &TracePartClip)> = read.iter().map(|(w, c)| (w, c)).collect();
        let active = if self.trace_mask_image().is_some_and(|i| i.id == image) {
            self.trace_active_part()
        } else {
            0
        };
        let joined = join_trace_parts(&pairs, active);
        #[allow(clippy::cast_precision_loss)] // counts far below 2^52
        let header = vec![
            match joined.status {
                vectrace::wire::Status::Traced => 0.0,
                vectrace::wire::Status::TooDetailed => 1.0,
                vectrace::wire::Status::Invalid => 2.0,
            },
            joined.paths as f64,
            joined.nodes as f64,
            joined.specks_removed as f64,
            joined.foreground_pixels as f64,
            f64::from(u8::from(joined.covers_whole_image)),
            joined.outlines.len() as f64,
        ];
        // As a single trace: the preview first, then the result, whose fit may show the texts.
        self.show_trace_read(image, joined.clone());
        if full {
            self.keep_trace_result(image, &joined);
        }
        Some(header)
    }

    /// The pixels of part `index` for the worker (criterion 31): as [`Self::trace_source`], cut to
    /// its clip's box plus a margin, the map moved with the cut. The margin is 16 px, or the pixel
    /// count of an Ignore below of `specks_mm` when that is more, so a shape that reaches into the
    /// clip is not taken for a speck. The whole picture for a part whose clip is the whole image,
    /// an Outside region or reaches the picture's edges, and with Upscale Auto (`upscale_auto`)
    /// when the cut-out would be upscaled and the picture not, or the other way round.
    pub fn trace_part_source(
        &mut self,
        image: NodeId,
        index: usize,
        reduced: bool,
        specks_mm: f64,
        upscale_auto: bool,
    ) -> Option<TraceSource> {
        let (clip, count) = self.for_trace_image(image, |s| {
            (
                s.trace_part_clip_list().get(index).cloned(),
                s.trace_part_count(),
            )
        });
        let source = self.trace_source(image, reduced)?;
        let Some(TracePartClip::Region(region)) = clip else {
            return Some(source);
        };
        if region.outside() || count < 2 {
            return Some(source);
        }
        let pixel_mm2 = curvyo_document_core::Affine::from_array(source.map)
            .determinant()
            .abs();
        let speck_px = if pixel_mm2 > 0.0 && specks_mm.is_finite() {
            (specks_mm * specks_mm / pixel_mm2).ceil()
        } else {
            0.0
        };
        let (width, height) = (source.width, source.height);
        let cut = crop_to(
            source.clone(),
            region.polygons(),
            CROP_MARGIN_PX.max(speck_px),
        );
        // Upscale Auto must decide alike for the cut-out and the picture.
        let upscaled = |w: u32, h: u32| vectrace::Upscale::Auto.applies(w, h);
        if upscale_auto && upscaled(width, height) != upscaled(cut.width, cut.height) {
            return Some(source);
        }
        Some(cut)
    }
}

impl Session {
    /// `read` with the parts of the traced picture `image` (one of several selected, Frage 11 A:
    /// each image with its own stored mask) standing for the traced image's.
    pub fn for_trace_image<R>(&self, image: NodeId, read: impl FnOnce(&Self) -> R) -> R {
        let before = self.trace.parts.for_image.replace(Some(image));
        let out = read(self);
        self.trace.parts.for_image.set(before);
        out
    }
}

/// `source` cut to the pixel box of `polygons` (document mm) plus `margin` pixels; unchanged when
/// the box is the picture or the map cannot be inverted.
pub(super) fn crop_to(source: TraceSource, polygons: &[Vec<Point>], margin: f64) -> TraceSource {
    let map = curvyo_document_core::Affine::from_array(source.map);
    let Some(inverse) = map.inverse() else {
        return source;
    };
    let (mut x0, mut y0, mut x1, mut y1) = (
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    );
    for p in polygons.iter().flatten() {
        let q = inverse.apply_point(*p);
        x0 = x0.min(q.x);
        y0 = y0.min(q.y);
        x1 = x1.max(q.x);
        y1 = y1.max(q.y);
    }
    let (w, h) = (f64::from(source.width), f64::from(source.height));
    let clamp = |v: f64, hi: f64| v.clamp(0.0, hi);
    let (left, top) = (
        clamp((x0 - margin).floor(), w),
        clamp((y0 - margin).floor(), h),
    );
    let (right, bottom) = (
        clamp((x1 + margin).ceil(), w),
        clamp((y1 + margin).ceil(), h),
    );
    if !(left.is_finite() && right > left && bottom > top)
        || (left, top, right, bottom) == (0.0, 0.0, w, h)
    {
        return source;
    }
    // invariant: clamped to 0..=width/height, whole numbers.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let (left, top, right, bottom) = (left as u32, top as u32, right as u32, bottom as u32);
    let (width, height) = (right - left, bottom - top);
    let row = source.width as usize * 4;
    let mut rgba = Vec::with_capacity(width as usize * height as usize * 4);
    for y in top..bottom {
        let start = y as usize * row + left as usize * 4;
        rgba.extend_from_slice(&source.rgba[start..start + width as usize * 4]);
    }
    let moved = map.compose(curvyo_document_core::Affine::translation(
        curvyo_document_core::Vec2::new(f64::from(left), f64::from(top)),
    ));
    TraceSource {
        width,
        height,
        rgba,
        map: moved.to_array(),
    }
}
