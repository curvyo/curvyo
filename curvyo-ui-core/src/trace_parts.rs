//! The parts of a composite trace (`specs/0094-masked-composite-trace` slice 2, `adrs.md`
//! decision 5 and the slice 2 note): what each part keeps once the later parts are taken out,
//! and the join of the parts' traced results into one.
//!
//! Stacking is creation order, bottom first; a later part wins inside its region (Frage 132 A).
//! Each part's kept area goes to the kernel's `visible_regions`; a part nothing later overlaps
//! keeps its own region, so its curves and its slice 1 clip stay as they are. The join keeps every
//! outline whose box meets no edge of its part's clip; the pieces that end on a clip edge are
//! united per colour, one operand per part, so the seams close on the 0.001 mm grid.

use curvyo_document_core::{Point, Vec2};
use curvyo_geometry_core::{BooleanOp, Outline, OutlineTriple, Visible, boolean, visible_regions};
use kurbo::{BezPath, Shape};
use vectrace::{Rgb8 as TraceRgb, node_count};

use crate::trace_region::{TRACE_REGION_TOLERANCE, TraceRegion, outer_sign, wound_polylines};
use vectrace::Output;
use vectrace::shapes::ShapeRegion;
use vectrace::wire::Texts;
use vectrace::wire::{Layer, Status};

/// How far the box that stands for "everything" reaches past the picture, mm: far enough that
/// no traced outline (which never leaves the picture) touches its edge (`adrs.md` decision 1).
const BEYOND_MM: f64 = 1.0;

/// What one part asks to keep.
#[derive(Debug, Clone, PartialEq)]
pub enum TracePartArea {
    /// The whole image (the first part without mask paths).
    Whole,
    /// Its mask region, Inside or Outside.
    Mask(TraceRegion),
    /// Nothing: a part without mask paths that is not the first, or whose mask misses the image.
    Nothing,
}

/// What one part's trace is clipped to once the later parts are taken out.
#[derive(Debug, Clone, PartialEq)]
pub enum TracePartClip {
    /// The part adds nothing: it has no area, or later parts cover all of it.
    Nothing,
    /// The whole image, unclipped.
    Whole,
    /// Its clip region.
    Region(TraceRegion),
}

impl TracePartClip {
    /// The region its trace is clipped to; `None` for the whole image and for nothing.
    #[must_use]
    pub const fn region(&self) -> Option<&TraceRegion> {
        match self {
            Self::Region(region) => Some(region),
            Self::Whole | Self::Nothing => None,
        }
    }

    /// Whether the part adds anything to the join.
    #[must_use]
    pub const fn contributes(&self) -> bool {
        !matches!(self, Self::Nothing)
    }
}

/// The clip of each part of `areas` (stacking order, bottom first) over the image with the
/// corners `quad`: its own area where no later part overlaps it, else its visible region.
#[must_use]
pub fn trace_part_clips(areas: &[TracePartArea], quad: &[Point; 4]) -> Vec<TracePartClip> {
    let own = |area: &TracePartArea| match area {
        TracePartArea::Whole => TracePartClip::Whole,
        TracePartArea::Mask(region) => TracePartClip::Region(region.clone()),
        TracePartArea::Nothing => TracePartClip::Nothing,
    };
    let everything = grown_box(quad);
    // The kept area of every part that has one, as kernel polygons.
    let kept: Vec<(usize, Vec<Vec<Point>>)> = areas
        .iter()
        .enumerate()
        .filter_map(|(i, area)| {
            let polygons = match area {
                TracePartArea::Whole => vec![everything.clone()],
                TracePartArea::Mask(region) if !region.outside() => region.polygons().to_vec(),
                TracePartArea::Mask(region) => without(&everything, region.polygons()),
                TracePartArea::Nothing => Vec::new(),
            };
            (!polygons.is_empty()).then_some((i, polygons))
        })
        .collect();
    let mut clips: Vec<TracePartClip> = areas.iter().map(|_| TracePartClip::Nothing).collect();
    if kept.len() <= 1 {
        for (i, _) in &kept {
            clips[*i] = own(&areas[*i]);
        }
        return clips;
    }
    let corners: Vec<Vec<Vec<OutlineTriple>>> = kept
        .iter()
        .map(|(_, polygons)| polygons.iter().map(|p| corner_outline(p)).collect())
        .collect();
    let operands: Vec<Vec<Outline<'_>>> = corners
        .iter()
        .map(|polygons| polygons.iter().map(|p| Outline::new(p, true)).collect())
        .collect();
    let refs: Vec<&[Outline<'_>]> = operands.iter().map(Vec::as_slice).collect();
    let Ok(visible) = visible_regions(&refs, TRACE_REGION_TOLERANCE) else {
        // invariant: the polygons are kernel output inside a finite box; should the kernel refuse
        // anyway, every part keeps its own area (overlaps then are not resolved).
        for (i, _) in &kept {
            clips[*i] = own(&areas[*i]);
        }
        return clips;
    };
    for ((i, _), seen) in kept.iter().zip(visible) {
        clips[*i] = match seen {
            Visible::Untouched => own(&areas[*i]),
            Visible::Trimmed(polygons) => TraceRegion::from_polygons(polygons, false)
                .map_or(TracePartClip::Nothing, TracePartClip::Region),
            Visible::Hidden => TracePartClip::Nothing,
        };
    }
    clips
}

/// The parts' traced results joined into one (`parts`: each part's result and its clip, in
/// stacking order); the palette is that of part `palette_of` (the active part, whose palette strip
/// the bar shows). One part that adds anything is returned as it is; a part that did not trace
/// (too detailed, invalid) makes the join say so.
#[must_use]
pub fn join_trace_parts(parts: &[(&Output, &TracePartClip)], palette_of: usize) -> Output {
    let live: Vec<(usize, &Output, &TracePartClip)> = parts
        .iter()
        .enumerate()
        .filter(|(_, (_, clip))| clip.contributes())
        .map(|(k, (wire, clip))| (k, *wire, *clip))
        .collect();
    if let [(_, wire, _)] = live.as_slice() {
        return (*wire).clone();
    }
    if let Some((_, failed, _)) = live.iter().find(|(_, w, _)| w.status != Status::Traced) {
        return empty(failed.status);
    }
    let multicolour = live.iter().any(|(_, w, _)| !w.layers.is_empty());
    let sign = outer_sign(
        &live
            .iter()
            .flat_map(|(_, w, _)| w.outlines.iter().cloned())
            .collect::<Vec<_>>(),
    );
    let buckets = sort_into_colours(&live, multicolour);
    let mut joined = empty(Status::Traced);
    // Where each part's kept outline went: (part, old index) to new index.
    let mut moved: Vec<((usize, usize), usize)> = Vec::new();
    for bucket in buckets {
        let start = joined.outlines.len();
        for (key, outline) in bucket.kept {
            moved.push((key, joined.outlines.len()));
            joined.outlines.push(outline);
        }
        for (key, outline) in bucket.seam.unite(sign) {
            if let Some(key) = key {
                moved.push((key, joined.outlines.len()));
            }
            joined.outlines.push(outline);
        }
        if multicolour {
            joined.layers.push(Layer {
                colour: bucket.colour,
                outlines: joined.outlines.len() - start,
            });
        }
    }
    for (k, wire, _) in &live {
        joined.specks_removed += wire.specks_removed;
        joined.foreground_pixels += wire.foreground_pixels;
        joined.joined += wire.joined;
        joined.unclipped += wire.unclipped;
        joined.texts.found = joined.texts.found.max(wire.texts.found);
        joined.texts.lines.extend(wire.texts.lines.iter().cloned());
        joined.shapes.extend(
            wire.shapes
                .iter()
                .filter_map(|region| moved_region(region, *k, &moved)),
        );
    }
    if let Some((wire, _)) = parts.get(palette_of) {
        joined.palette.clone_from(&wire.palette);
    }
    joined.paths = joined.outlines.len();
    joined.nodes = joined.outlines.iter().map(node_count).sum();
    joined
}

/// The outlines of one fill colour over all parts.
struct Bucket {
    colour: TraceRgb,
    /// The outlines that stay as they are, with their part and index.
    kept: Vec<((usize, usize), BezPath)>,
    seam: Seam,
}

/// The pieces of one colour that end on a clip edge, per part.
#[derive(Default)]
struct Seam {
    parts: Vec<(usize, Vec<(usize, BezPath)>)>,
}

impl Seam {
    fn push(&mut self, part: usize, index: usize, outline: BezPath) {
        match self.parts.last_mut() {
            Some((k, pieces)) if *k == part => pieces.push((index, outline)),
            _ => self.parts.push((part, vec![(index, outline)])),
        }
    }

    /// The pieces united when they come from two parts or more (one operand per part), as
    /// polylines wound with `sign`; pieces of a single part stay as they are, with their part and
    /// index.
    fn unite(self, sign: f64) -> Vec<(Option<(usize, usize)>, BezPath)> {
        let keep_all = |parts: Vec<(usize, Vec<(usize, BezPath)>)>| {
            parts
                .into_iter()
                .flat_map(|(k, pieces)| {
                    pieces
                        .into_iter()
                        .map(move |(index, outline)| (Some((k, index)), outline))
                })
                .collect()
        };
        if self.parts.len() < 2 {
            return keep_all(self.parts);
        }
        let triples: Vec<Vec<Vec<OutlineTriple>>> = self
            .parts
            .iter()
            .map(|(_, pieces)| {
                pieces
                    .iter()
                    .flat_map(|(_, path)| crate::trace_region::triples_of(path))
                    .collect()
            })
            .collect();
        let operands: Vec<Vec<Outline<'_>>> = triples
            .iter()
            .map(|part| part.iter().map(|a| Outline::new(a, true)).collect())
            .collect();
        let refs: Vec<&[Outline<'_>]> = operands.iter().map(Vec::as_slice).collect();
        match boolean(BooleanOp::Union, &refs, TRACE_REGION_TOLERANCE) {
            Ok(result) => wound_polylines(result.outlines(), sign)
                .into_iter()
                .map(|outline| (None, outline))
                .collect(),
            // invariant: the pieces are clipped traced outlines inside the picture; should the
            // kernel refuse anyway, they stay as they are (the seam stays a line).
            Err(_) => keep_all(self.parts),
        }
    }
}

/// The live parts' outlines sorted into one bucket per fill colour (black for a single-colour
/// part), largest painted area first; ties keep the order in which the colours appear.
fn sort_into_colours(live: &[(usize, &Output, &TracePartClip)], multicolour: bool) -> Vec<Bucket> {
    let black = TraceRgb::new(0, 0, 0);
    let mut buckets: Vec<Bucket> = Vec::new();
    for (k, wire, clip) in live {
        let single = [Layer {
            colour: black,
            outlines: wire.outlines.len(),
        }];
        let layers: &[Layer] = if wire.layers.is_empty() {
            &single
        } else {
            &wire.layers
        };
        let mut index = 0;
        for layer in layers {
            let colour = if multicolour { layer.colour } else { black };
            let at = buckets
                .iter()
                .position(|b| b.colour == colour)
                .unwrap_or_else(|| {
                    buckets.push(Bucket {
                        colour,
                        kept: Vec::new(),
                        seam: Seam::default(),
                    });
                    buckets.len() - 1
                });
            for outline in &wire.outlines[index..index + layer.outlines] {
                let b = outline.control_box();
                let on_edge = clip.region().is_some_and(|region| {
                    region.edge_meets(Point::new(b.x0, b.y0), Point::new(b.x1, b.y1))
                });
                if on_edge {
                    buckets[at].seam.push(*k, index, outline.clone());
                } else {
                    buckets[at].kept.push(((*k, index), outline.clone()));
                }
                index += 1;
            }
        }
    }
    let painted = |b: &Bucket| -> f64 {
        b.kept
            .iter()
            .map(|(_, o)| o.area())
            .chain(
                b.seam
                    .parts
                    .iter()
                    .flat_map(|(_, p)| p.iter().map(|(_, o)| o.area())),
            )
            .sum::<f64>()
            .abs()
    };
    let mut keyed: Vec<(f64, Bucket)> = buckets.into_iter().map(|b| (painted(&b), b)).collect();
    keyed.sort_by(|a, b| b.0.total_cmp(&a.0));
    keyed.into_iter().map(|(_, b)| b).collect()
}

/// A shape region of part `part` with its outlines at their joined places; `None` when one of
/// them went into a seam union.
fn moved_region(
    region: &ShapeRegion,
    part: usize,
    moved: &[((usize, usize), usize)],
) -> Option<ShapeRegion> {
    let outlines = region
        .outlines
        .iter()
        .map(|&i| {
            moved
                .iter()
                .find(|(key, _)| *key == (part, i))
                .map(|(_, new)| *new)
        })
        .collect::<Option<Vec<usize>>>()?;
    Some(ShapeRegion {
        outlines,
        ..region.clone()
    })
}

/// A result with `status` and nothing in it.
fn empty(status: Status) -> Output {
    Output {
        status,
        paths: 0,
        nodes: 0,
        specks_removed: 0,
        foreground_pixels: 0,
        covers_whole_image: false,
        outlines: Vec::new(),
        palette: Vec::new(),
        layers: Vec::new(),
        joined: 0,
        texts: Texts::default(),
        shapes: Vec::new(),
        unclipped: 0,
    }
}

/// The box of `quad` grown by [`BEYOND_MM`], counter-clockwise on screen as the kernel's outer
/// outlines are not required to be.
fn grown_box(quad: &[Point; 4]) -> Vec<Point> {
    let (mut min, mut max) = (quad[0], quad[0]);
    for p in quad {
        min = Point::new(min.x.min(p.x), min.y.min(p.y));
        max = Point::new(max.x.max(p.x), max.y.max(p.y));
    }
    let (x0, y0, x1, y1) = (
        min.x - BEYOND_MM,
        min.y - BEYOND_MM,
        max.x + BEYOND_MM,
        max.y + BEYOND_MM,
    );
    vec![
        Point::new(x0, y0),
        Point::new(x1, y0),
        Point::new(x1, y1),
        Point::new(x0, y1),
    ]
}

/// The polygon `outer` minus `polygons` (kernel polygons, nonzero), as kernel polygons; empty
/// when nothing is left.
fn without(outer: &[Point], polygons: &[Vec<Point>]) -> Vec<Vec<Point>> {
    let outer = corner_outline(outer);
    let cut: Vec<Vec<OutlineTriple>> = polygons.iter().map(|p| corner_outline(p)).collect();
    let outer_operand = [Outline::new(&outer, true)];
    let cut_operand: Vec<Outline<'_>> = cut.iter().map(|p| Outline::new(p, true)).collect();
    boolean(
        BooleanOp::Difference,
        &[&outer_operand, &cut_operand],
        TRACE_REGION_TOLERANCE,
    )
    .map(curvyo_geometry_core::BooleanResult::into_outlines)
    .unwrap_or_default()
}

fn corner_outline(polygon: &[Point]) -> Vec<OutlineTriple> {
    polygon
        .iter()
        .map(|&p| (p, Vec2::ZERO, Vec2::ZERO))
        .collect()
}
