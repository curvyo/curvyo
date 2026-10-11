//! Turns the raw cuts of one contour into its final cut parameters: snapped to nodes, ordered and
//! merged (`specs/0036-cut-at-crossings` criteria 3 and 4).

use curvyo_document_core::Point;

use crate::Outline;
use crate::curve_chords::{CUT_MERGE_MM, dist};
use crate::curve_intersections::{CutParameter, Seg};
use crate::outline_touch::TOUCH_DISTANCE_MM;

/// A cut this close to a node is made at the node (millimetres, `0036` criterion 4).
const SNAP_MM: f64 = TOUCH_DISTANCE_MM;

/// A raw cut made exact: snapped to a node, ordered, merged.
struct Placed {
    segment: usize,
    t: f64,
    at: Point,
    /// How far along the contour it is, by chord lengths (a lower bound of the arc length).
    along: f64,
}

/// Whether two cuts are one: close in space and close along the contour. A contour that passes
/// one point twice (a figure eight) has two cuts there, far apart along it.
fn same_cut(a: &Placed, b: &Placed) -> bool {
    dist(a.at, b.at) <= CUT_MERGE_MM && (b.along - a.along).abs() <= CUT_MERGE_MM
}

/// Snaps, sorts and merges the raw cuts of one contour.
pub(crate) fn settle(
    outline: &Outline<'_>,
    segs: &[Seg],
    along: &[f64],
    cuts: Vec<(usize, f64)>,
) -> Vec<CutParameter> {
    let anchors = outline.anchors;
    let count = anchors.len();
    let n_seg = segs.len();
    let mut placed: Vec<Placed> = Vec::with_capacity(cuts.len());
    for (segment, t) in cuts {
        let t = t.clamp(0.0, 1.0);
        let at = segs[segment].point_at(t);
        let start = anchors[segment].0;
        let end = anchors[(segment + 1) % count].0;
        let cut = if t <= 0.0 || dist(at, start) <= SNAP_MM {
            Placed {
                segment,
                t: 0.0,
                at: start,
                along: along[segment],
            }
        } else if t >= 1.0 || dist(at, end) <= SNAP_MM {
            // The next node; the end node of an open contour is `n_seg` and is dropped below.
            let next = segment + 1;
            let wraps = outline.closed && next == n_seg;
            Placed {
                segment: if wraps { 0 } else { next },
                t: 0.0,
                at: end,
                along: if wraps { 0.0 } else { along[next] },
            }
        } else {
            let along = along[segment] + t * (along[segment + 1] - along[segment]);
            Placed {
                segment,
                t,
                at,
                along,
            }
        };
        // The end nodes of an open contour are no cuts.
        if !outline.closed && cut.t == 0.0 && (cut.segment == 0 || cut.segment == n_seg) {
            continue;
        }
        placed.push(cut);
    }
    placed.sort_by(|l, r| l.segment.cmp(&r.segment).then(l.t.total_cmp(&r.t)));
    let mut kept: Vec<Placed> = Vec::with_capacity(placed.len());
    for cut in placed {
        match kept.last_mut() {
            Some(last) if same_cut(last, &cut) => {
                if cut.t == 0.0 && last.t != 0.0 {
                    *last = cut;
                }
            }
            _ => kept.push(cut),
        }
    }
    if outline.closed && kept.len() > 1 {
        let (first, last) = (&kept[0], &kept[kept.len() - 1]);
        // The last cut seen from before the first one, round the closing segment.
        let wrapped = Placed {
            segment: last.segment,
            t: last.t,
            at: last.at,
            along: last.along - along[n_seg],
        };
        if same_cut(&wrapped, first) {
            if last.t == 0.0 && first.t != 0.0 {
                kept.remove(0);
            } else {
                kept.pop();
            }
        }
    }
    kept.into_iter()
        .map(|cut| CutParameter {
            segment: cut.segment,
            t: cut.t,
        })
        .collect()
}
