//! The guard of a trace: fits each outline within the room its neighbours leave, one level
//! coarser per span that crosses or comes too close, down to the pixel border.

use kurbo::Shape;

use crate::fit::Seg;
use crate::guard::conflicts;
use crate::outline::{Bound, Chain, Contour, SPAN_LEVELS, to_path};
use crate::params::Geometry;
use crate::progress::{Phase, Poller};
use crate::trace::TraceError;

/// A shape whose pixels cover less than this many Specks areas is fitted with at most
/// [`TINY_TOLERANCE_PX`] and, in the band, is its pixel border.
pub(crate) const BAND_TINY: f64 = 1.5;

/// The tolerance of a shape of a few pixels, source px.
pub(crate) const TINY_TOLERANCE_PX: f64 = 0.25;

/// An outline keeps its fit down to this share of the Specks area (the harness and 0025
/// criterion 63 allow 0.95 for pixel rounding); below it the Specks band applies.
pub(crate) const BAND_SHARE: f64 = 0.95;

/// The Specks band: a fitted outline whose area falls below the Specks area `min_area_px`,
/// which its pixels reached (the Specks step kept them), keeps its pixel border (criteria 10
/// and 19).
pub(crate) struct Band {
    pub(crate) min_area_px: f64,
}

impl Band {
    /// Whether the outline `segs` falls into the band.
    fn holds(&self, segs: &[Seg]) -> bool {
        self.min_area_px > 0.0 && to_path(segs).area().abs() < BAND_SHARE * self.min_area_px
    }
}

/// One outline in the guard: its parts per span, each span's fall-back level, which corners
/// have gone back to their vertices, whether it has become its pixel border; the room of its
/// edges among the smoothed contours, its freely smoothed contour and, once a span needs one,
/// its contour smoothed within the walked room.
struct Guarded {
    spans: Vec<Vec<Seg>>,
    levels: Vec<u8>,
    at_vertex: Vec<bool>,
    border: bool,
    /// The geometry it is fitted with: the trace's, or with the tolerance of a shape of a few
    /// pixels ([`TINY_TOLERANCE_PX`]), so whether its fit keeps the Specks area does not depend
    /// on the Tolerance.
    geometry: Geometry,
    room: Vec<f64>,
    free: Option<Contour>,
    clamped: Option<Contour>,
}

impl Guarded {
    /// All parts in order, and for each the span it belongs to.
    fn parts(&self) -> (Vec<Seg>, Vec<usize>) {
        let mut segs = Vec::new();
        let mut owner = Vec::new();
        for (i, span) in self.spans.iter().enumerate() {
            segs.extend(span.iter().copied());
            owner.extend(std::iter::repeat_n(i, span.len()));
        }
        (segs, owner)
    }

    /// The outline fitted at its levels (the contours made when first needed).
    fn refit(&mut self, chain: &Chain, clearance: &[f64]) {
        let geometry = &self.geometry;
        if self.free.is_none() {
            self.free = Some(chain.contour(
                geometry,
                clearance,
                Bound::Smoothed(&self.room),
                &self.at_vertex,
            ));
        }
        if self.clamped.is_none() && self.levels.iter().any(|&l| l > 0) {
            self.clamped = Some(chain.contour(geometry, clearance, Bound::Walked, &self.at_vertex));
        }
        // invariant: `free` was just made.
        let free = self.free.as_ref().unwrap_or_else(|| unreachable!());
        let clamped = self.clamped.as_ref().unwrap_or(free);
        self.spans = chain.fitted(geometry, &self.levels, free, clamped);
    }

    /// Puts the corners of the spans `spans` at their vertices; false when they were already.
    fn corners_to_vertices(&mut self, spans: &[usize]) -> bool {
        let count = self.at_vertex.len();
        let mut changed = false;
        for &i in spans {
            for k in [i, (i + 1) % count.max(1)] {
                if let Some(flag) = self.at_vertex.get_mut(k)
                    && !*flag
                {
                    *flag = true;
                    changed = true;
                }
            }
        }
        if changed {
            self.free = None;
            self.clamped = None;
        }
        changed
    }
}

/// The fitted outlines of `chains` (each within the room its `clearances` and `rooms` leave)
/// and how many fell back: an outline in the Specks `band` becomes its pixel border; every
/// span with a segment that crosses its own outline or comes too close to another goes one
/// level coarser (`outline::SPAN_LEVELS`: the fit within the walked room, the walked contour
/// simplified within half its room, the contour as walked); where the walked contour is not
/// enough, the span's corners go back to their vertices, and then the outline becomes its
/// pixel border (criterion 10).
pub(crate) fn guarded(
    chains: &[&Chain],
    geometry: &Geometry,
    clearances: &[Vec<f64>],
    rooms: Vec<Vec<f64>>,
    band: &Band,
    poller: &mut Poller<'_>,
) -> Result<(Vec<Vec<Seg>>, usize), TraceError> {
    let total = chains.len();
    let mut state: Vec<Guarded> = Vec::with_capacity(total);
    for (k, (chain, edge_room)) in chains.iter().zip(rooms).enumerate() {
        if k % 64 == 0 {
            poller.check(Phase::Tracing, k, total)?;
        }
        let tiny = band.min_area_px > 0.0
            && to_path(&chain.pixel_border()).area().abs() < BAND_TINY * band.min_area_px;
        let mut outline = Guarded {
            spans: Vec::new(),
            levels: vec![0; chain.span_count()],
            at_vertex: vec![false; chain.span_count()],
            border: false,
            geometry: if tiny {
                Geometry {
                    tolerance_px: geometry.tolerance_px.min(TINY_TOLERANCE_PX),
                    ..*geometry
                }
            } else {
                *geometry
            },
            room: edge_room,
            free: None,
            clamped: None,
        };
        outline.refit(chain, &clearances[k]);
        state.push(outline);
    }
    let mut fell_back = vec![false; total];
    loop {
        for (k, outline) in state.iter_mut().enumerate() {
            if !outline.border && band.holds(&outline.spans.concat()) {
                // The walked contour simplified within half its room keeps nearly the pixel
                // area; a shape too small even for that is its pixel border.
                if outline.levels.iter().any(|&l| l < 2) {
                    for level in &mut outline.levels {
                        *level = (*level).max(2);
                    }
                    outline.refit(chains[k], &clearances[k]);
                }
                if band.holds(&outline.spans.concat()) {
                    // A shape of a few pixels is its pixel border; a larger one (a sliver or a
                    // ragged shape whose fit shrank) its pixel border simplified within its
                    // room, when that keeps the area.
                    let border = chains[k].pixel_border();
                    let simplified = chains[k].border_simplified(&clearances[k]);
                    let tiny = to_path(&border).area().abs() < BAND_TINY * band.min_area_px;
                    outline.spans = if tiny || band.holds(&simplified) {
                        vec![border]
                    } else {
                        vec![simplified]
                    };
                    outline.border = true;
                }
            }
        }
        let owners: Vec<(Vec<Seg>, Vec<usize>)> = state.iter().map(Guarded::parts).collect();
        let outlines: Vec<Vec<Seg>> = owners.iter().map(|(parts, _)| parts.clone()).collect();
        let conflicting = conflicts(&outlines);
        let mut changed = false;
        for (k, flagged) in conflicting.iter().enumerate() {
            let outline = &mut state[k];
            if flagged.is_empty() || outline.border {
                continue;
            }
            changed = true;
            fell_back[k] = true;
            let mut spans: Vec<usize> = flagged.iter().map(|&i| owners[k].1[i]).collect();
            spans.dedup();
            let coarser: Vec<usize> = spans
                .iter()
                .copied()
                .filter(|&i| outline.levels[i] + 1 < SPAN_LEVELS)
                .collect();
            if coarser.is_empty() {
                // The walked contour from corner to corner still conflicts: the corners
                // themselves (placed beyond the pixels) go back to their vertices, then the
                // outline becomes its pixel border.
                if outline.corners_to_vertices(&spans) {
                    outline.levels.fill(0);
                    outline.refit(chains[k], &clearances[k]);
                } else {
                    outline.spans = vec![chains[k].pixel_border()];
                    outline.border = true;
                }
                continue;
            }
            for &i in &coarser {
                outline.levels[i] += 1;
            }
            outline.refit(chains[k], &clearances[k]);
        }
        if !changed {
            break;
        }
        poller.check(Phase::Tracing, total, total)?;
    }
    let fallbacks = fell_back.iter().filter(|&&f| f).count();
    let fitted = state.into_iter().map(|g| g.spans.concat()).collect();
    Ok((fitted, fallbacks))
}
