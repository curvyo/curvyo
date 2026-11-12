//! The mask region of the Trace tool (`specs/0094-masked-composite-trace` slice 1, `adrs.md`
//! decision 1; `specs/0114-image-trace-mask` slice 1): the active part's stored regions and the
//! paths selected with the image, their mask made once per change of them, the test that it meets
//! the image, its numbers for the worker and the Overlay's extra wash.

use curvyo_document_core::{
    AnchorId, AnchorKind, AnchorSnapshot, Angle, Color, Fill, ObjectSnapshot, Opacity,
    PathSnapshot, Point, Stroke, Style, SubpathSnapshot, Vec2,
};
use curvyo_ui_core::{TraceRegion, TraceRegionError, mask_operand};

use super::trace::TraceSelection;
use super::trace_parts::SignedOperands;
use super::{Session, Tool};

/// What is left of the image outside the region in Overlay (UX notes §2): 12 %, on top of the
/// 35 % of the ordinary wash.
const REGION_WASH_LEFT: f64 = 0.12;
/// What the ordinary wash leaves of the image (`trace_draw.rs`).
const WASH_LEFT: f64 = 0.35;

/// The region of one set of mask operands over one image.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RegionCache {
    /// The added operands (mask paths in document order, then added regions) and the
    /// subtracted ones it was made from.
    operands: SignedOperands,
    /// The image's corners it was tested against.
    quad: [Point; 4],
    /// The union (Inside); `None` when no mask path encloses an area.
    region: Result<TraceRegion, TraceRegionError>,
    /// The union covers some of the image.
    meets: bool,
    /// The image minus the region (the Inside wash) and the image within it (the Outside wash).
    beyond: Vec<Vec<Point>>,
    within: Vec<Vec<Point>>,
}

/// Whether `object` can be a mask path: a path, a compound path, a primitive, or a linked clone
/// that resolves to path outlines (a group arrives as its leaves). Texts, images, sketches and
/// dimensions are not (criterion 14).
pub(super) fn is_mask_path(object: &ObjectSnapshot) -> bool {
    match object {
        ObjectSnapshot::Path(_) | ObjectSnapshot::Primitive(_) => true,
        ObjectSnapshot::Clone(_) => {
            mask_operand(object).map_or(true, |operand| !operand.is_empty())
        }
        ObjectSnapshot::Text(_)
        | ObjectSnapshot::Image(_)
        | ObjectSnapshot::Sketch(_)
        | ObjectSnapshot::Dimension(_) => false,
    }
}

impl RegionCache {
    pub(super) fn new(operands: SignedOperands, quad: [Point; 4]) -> Self {
        let region = operands
            .0
            .as_ref()
            .map_err(Clone::clone)
            .and_then(|added| TraceRegion::of_signed_operands(added, &operands.1, false));
        let [beyond, within] = region.as_ref().ok().map_or_else(
            || [vec![quad.to_vec()], Vec::new()],
            |r| r.quad_parts(&quad),
        );
        Self {
            operands,
            quad,
            meets: !within.is_empty(),
            region,
            beyond,
            within,
        }
    }

    /// Whether it was made of `operands` over `quad`.
    pub(super) fn made_of(&self, operands: &SignedOperands, quad: [Point; 4]) -> bool {
        &self.operands == operands && self.quad == quad
    }

    /// The union (Inside), or why there is none.
    pub(super) const fn region(&self) -> &Result<TraceRegion, TraceRegionError> {
        &self.region
    }

    /// Whether the union covers some of the image.
    pub(super) const fn meets(&self) -> bool {
        self.meets
    }
}

impl Session {
    /// The mask paths of the selection (one image and paths, `specs/0094`), in document order;
    /// empty for any other selection.
    pub(super) fn trace_mask_objects(&self) -> Vec<ObjectSnapshot> {
        let (selection, images) = self.trace_images();
        if selection != TraceSelection::Images || images.len() != 1 {
            return Vec::new();
        }
        // Selected groups count as their leaves.
        let selected: Vec<curvyo_document_core::NodeId> = self
            .selection
            .ids()
            .iter()
            .flat_map(|&id| self.document.leaves_of(id))
            .collect();
        self.objects()
            .iter()
            .filter(|object| selected.contains(&object.id()) && is_mask_path(object))
            .cloned()
            .collect()
    }

    /// The mask of the active part (its stored regions and the selected mask paths) over the
    /// selected image, made again only when they or the image changed; `None` without either.
    fn trace_region_cache(&self) -> Option<RegionCache> {
        let masks = self.trace_mask_objects();
        let part = self.trace_active_mask_part()?;
        if masks.is_empty() && part.regions.is_empty() {
            return None;
        }
        let quad = self.trace_mask_image()?.corners();
        let operands = self.trace_part_operands(&part, &masks);
        let mut cache = self.trace.region.borrow_mut();
        if cache.as_ref().is_none_or(|c| !c.made_of(&operands, quad)) {
            *cache = Some(RegionCache::new(operands, quad));
        }
        cache.clone()
    }

    /// Whether the mask paths cover some of the image (criterion 7).
    pub(super) fn trace_mask_meets_image(&self) -> bool {
        self.trace_region_cache().is_some_and(|c| c.meets)
    }

    /// Why the mask paths make no region, when it is not that none of them encloses an area
    /// (the kernel refused the union; "0 paths: the mask could not be combined.").
    pub(super) fn trace_mask_failure(&self) -> Option<TraceRegionError> {
        self.trace_region_cache()?
            .region
            .err()
            .filter(|e| *e != TraceRegionError::NoArea)
    }

    /// The region the trace keeps with the active part's mode, when it has regions or mask
    /// paths and they cover some of the image.
    #[must_use]
    pub fn trace_region(&self) -> Option<TraceRegion> {
        let cache = self.trace_region_cache()?;
        let region = cache.region.ok().filter(|_| cache.meets)?;
        Some(region.with_outside(self.trace_mask_outside()))
    }

    /// The region for the worker's settings (`"mask"`, `trace_glue::trace_mask_region`): `{"outside":
    /// bool, "region": [numbers]}`, or empty without a region.
    #[must_use]
    pub fn trace_mask_json(&self) -> String {
        self.trace_region().map_or_else(String::new, |region| {
            serde_json::json!({
                "outside": region.outside(),
                "region": region.to_numbers(),
            })
            .to_string()
        })
    }

    /// Chooses Inside (`false`) or Outside in the Mask item: for a stored part one step "Mask:
    /// Inside" or "Mask: Outside" (`0114` criterion 17), for the implicit part tool state
    /// (ADR 0061 §3).
    pub fn set_trace_mask_outside(&mut self, outside: bool) {
        if self.tool != Tool::Trace {
            self.trace.mask_outside = false;
            return;
        }
        let stored = self.trace_active_mask_part().and_then(|part| part.stored);
        match (stored, self.trace_mask_image()) {
            (Some(part), Some(image)) => {
                self.recording(|this| {
                    this.document
                        .set_trace_part_outside(image.id, &part, outside)
                        .unwrap_or(false)
                });
            }
            _ => self.trace.mask_outside = outside,
        }
    }

    /// Whether Outside is chosen for the active part.
    #[must_use]
    pub fn trace_mask_outside(&self) -> bool {
        self.trace_active_mask_part()
            .map_or(self.trace.mask_outside, |part| part.outside)
    }

    /// Overlay with a mask (UX notes §2): the part of the image the trace does not keep washed
    /// further to 12 %, a white compound path directly above the ordinary wash, with the image's
    /// id and never a document node.
    pub(super) fn trace_region_wash(&self, objects: &mut Vec<ObjectSnapshot>) {
        let Some(cache) = self.trace_region_cache() else {
            return;
        };
        let polygons = if self.trace_mask_outside() {
            &cache.within
        } else {
            &cache.beyond
        };
        let (_, images) = self.trace_images();
        let Some(image) = images.first() else {
            return;
        };
        let Some(at) = objects
            .iter()
            .position(|object| matches!(object, ObjectSnapshot::Image(i) if i.id == image.id))
        else {
            return;
        };
        if polygons.is_empty() {
            return;
        }
        let strength = image.shown_opacity.get() * (1.0 - REGION_WASH_LEFT / WASH_LEFT);
        let wash = white_polygons(image.id, polygons, strength);
        let place = if objects.get(at + 1).is_some_and(|o| o.id() == image.id) {
            at + 2
        } else {
            at + 1
        };
        objects.insert(place, ObjectSnapshot::Path(wash));
    }
}

/// `polygons` as one white compound path at `strength`, carrying `id`.
fn white_polygons(
    id: curvyo_document_core::NodeId,
    polygons: &[Vec<Point>],
    strength: f64,
) -> PathSnapshot {
    let mut n = 0_u64;
    let mut subpaths = polygons.iter().map(|polygon| {
        polygon
            .iter()
            .map(|&point| {
                n += 1;
                AnchorSnapshot {
                    id: AnchorId::new(0, n),
                    point,
                    handle_in: Vec2::ZERO,
                    handle_out: Vec2::ZERO,
                    kind: AnchorKind::Corner,
                }
            })
            .collect::<Vec<_>>()
    });
    let anchors = subpaths.next().unwrap_or_default();
    let extra_subpaths = subpaths
        .map(|anchors| SubpathSnapshot {
            closed: true,
            anchors,
        })
        .collect();
    PathSnapshot {
        id,
        closed: true,
        style: Style {
            stroke: Stroke {
                enabled: false,
                ..Stroke::default()
            },
            fill: Fill {
                enabled: true,
                color: Color {
                    r: 255,
                    g: 255,
                    b: 255,
                },
                opacity: Opacity::new(strength).unwrap_or(Opacity::OPAQUE),
                ..Fill::default()
            },
        },
        anchors,
        extra_subpaths,
        curve_kind: curvyo_document_core::CurveKind::Bezier,
        spiro_transform: None,
        width_points: Vec::new(),
        rotation: Angle::from_radians(0.0),
    }
}
