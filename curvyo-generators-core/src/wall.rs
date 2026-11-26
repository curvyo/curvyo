//! The developed wall: the strip outline with its slit slots, edge notches and the slots for the
//! tabs of the plates that sit inside it (ADR 0082 §4, §5).
//!
//! The strip is flat: `x` along the neutral line from the seam, `y` from the top edge down.

use curvyo_document_core::{Length, Point};

use crate::contour::{Contour, Seg};
use crate::footprint::{Footprint, Tab};
use crate::slit_bend::{SlitPattern, Slot, Window};
use crate::wave_slot::wave;

/// The slot a plate's tab needs: `thickness` across the strip, twice that along it, with its
/// outer face `inset` from the top or the bottom edge.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Joint {
    pub(crate) thickness: Length,
    /// Half the slot's length along the strip: the tab's half width (the thickness while automatic).
    pub(crate) half_width: Length,
    pub(crate) inset: Length,
    pub(crate) from_top: bool,
}

/// One step of the end edge of a strip piece: the heights `y0..y1` reach out to `x`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Step {
    pub(crate) y0: f64,
    pub(crate) y1: f64,
    pub(crate) x: f64,
}

/// A strip piece that ends at corners (ADR 0083 §5): the steps of its two end edges, top to
/// bottom, and how far the first and last slit column keep from the ends.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Ends {
    pub(crate) start: Vec<Step>,
    pub(crate) end: Vec<Step>,
    pub(crate) lead: f64,
    pub(crate) trail: f64,
}

/// The spacing of an automatic pattern from the local radius (ADR 0083 §4): `factor` times the
/// bend's radius, at least `least`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct LocalSpacing {
    pub(crate) factor: f64,
    pub(crate) least: f64,
}

/// What the strip is made of.
#[derive(Debug, Clone)]
pub(crate) struct Wall<'a> {
    pub(crate) footprint: &'a Footprint,
    pub(crate) height: Length,
    pub(crate) pattern: SlitPattern,
    /// Slits over the whole strip, not only on the bends.
    pub(crate) slits_everywhere: bool,
    pub(crate) joints: Vec<Joint>,
    /// Only the plain outline, for the ghost (`Edit::Draft`): no slits and no tab slots.
    pub(crate) draft: bool,
    /// Finger ends of a piece between corners; `None` for the rounded box's closed strip.
    pub(crate) ends: Option<Ends>,
    /// The spacing from each bend's radius; `None` uses `pattern.spacing` everywhere.
    pub(crate) local_spacing: Option<LocalSpacing>,
    /// Consecutive bends are one band whose columns follow the integral of the wanted gap (the
    /// path box, ADR 0083 §13.4); `false` keeps one band per bend (the rounded box).
    pub(crate) runs: bool,
    /// How far the first and last slit column keep from the cut ends of a strip without finger
    /// ends (the seam of a path box without a corner, ADR 0083 §6); 0 for the rounded box.
    pub(crate) seam_margin: f64,
}

/// An axis-aligned rectangle in strip coordinates.
#[derive(Debug, Clone, Copy)]
struct Rect {
    x0: f64,
    x1: f64,
    y0: f64,
    y1: f64,
}

impl Wall<'_> {
    /// The strip: its outline first (clockwise), then the closed slots (counter-clockwise).
    pub(crate) fn contours(&self, tabs: &[Tab]) -> Vec<Contour> {
        let length = self.footprint.total();
        let height = self.height.as_mm();
        if self.draft {
            return vec![outline(
                length,
                height,
                Vec::new(),
                Vec::new(),
                self.ends.as_ref(),
            )];
        }
        let mut rects = self.tab_rects(tabs, height);
        let windows: Vec<Window> = rects
            .iter()
            .map(|r| Window {
                from: Length::from_mm(r.x0),
                to: Length::from_mm(r.x1),
                rows: self.on_arc(r.x0, r.x1).then_some((r.y0, r.y1)),
            })
            .collect();
        let mut waves: Vec<(Slot, bool)> = Vec::new();
        for spec in self.bands(length, height) {
            for slot in self.band_slots(&spec, &windows) {
                match slot.wave {
                    Some(_) => waves.push((slot, slot.y0 <= 1e-9)),
                    None => rects.push(slot_rect(&slot)),
                }
            }
        }
        let mut top: Vec<Notch> = Vec::new();
        let mut bottom: Vec<Notch> = Vec::new();
        for r in rects.iter().filter(|r| r.y0 <= 1e-9) {
            top.push(rect_notch_top(*r));
        }
        for r in rects
            .iter()
            .filter(|r| r.y1 >= height - 1e-9 && r.y0 > 1e-9)
        {
            bottom.push(rect_notch_bottom(*r, height));
        }
        for (slot, from_top) in &waves {
            let notch = wave_notch(slot, *from_top, height);
            if *from_top {
                top.push(notch);
            } else {
                bottom.push(notch);
            }
        }
        let holes = rects
            .iter()
            .copied()
            .filter(|r| r.y0 > 1e-9 && r.y1 < height - 1e-9);
        let mut contours = vec![outline(length, height, top, bottom, self.ends.as_ref())];
        contours.extend(holes.map(hole));
        contours
    }

    /// A tab slot over `x0..x1` that lies on a bend of the true arc (no window chord): the
    /// columns beside it run on, clipped (ADR 0089 decision 2).
    fn on_arc(&self, x0: f64, x1: f64) -> bool {
        let middle = f64::midpoint(x0, x1);
        self.footprint.spans.iter().any(|span| {
            matches!(span.shape, crate::footprint::Shape::Bend { .. })
                && middle >= span.s0 - 1e-9
                && middle <= span.s0 + span.length + 1e-9
        })
    }

    /// The slots of the plates' tabs, one per tab and joint.
    fn tab_rects(&self, tabs: &[Tab], height: f64) -> Vec<Rect> {
        let mut rects = Vec::new();
        for tab in tabs {
            for joint in &self.joints {
                let (t, e) = (joint.thickness.as_mm(), joint.inset.as_mm());
                let half = joint.half_width.as_mm();
                let (y0, y1) = if joint.from_top {
                    (e, e + t)
                } else {
                    (height - e - t, height - e)
                };
                rects.push(Rect {
                    x0: tab.s - half,
                    x1: tab.s + half,
                    y0,
                    y1,
                });
            }
        }
        rects
    }
}

/// The distance between two columns of a band `from..to` that asks for `spacing`: as many columns
/// as fit (at least two, or a bend would keep an uncut band), spread evenly.
pub(crate) fn column_pitch(from: f64, to: f64, spacing: f64) -> f64 {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let columns = (((to - from) / spacing).floor() as usize).max(2);
    #[allow(clippy::cast_precision_loss)]
    let pitch = (to - from) / columns as f64;
    pitch
}

fn slot_rect(slot: &Slot) -> Rect {
    Rect {
        x0: slot.x - slot.width / 2.0,
        x1: slot.x + slot.width / 2.0,
        y0: slot.y0,
        y1: slot.y1,
    }
}

fn p(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

/// A cut in the top or bottom edge: the walk enters the strip at `enter`, follows `segs` and
/// leaves it at `exit`.
struct Notch {
    enter: Point,
    exit: Point,
    segs: Vec<Seg>,
}

fn lines(points: &[Point]) -> Vec<Seg> {
    points
        .windows(2)
        .map(|pair| Seg::Line {
            from: pair[0],
            to: pair[1],
        })
        .collect()
}

fn rect_notch_top(r: Rect) -> Notch {
    let (enter, exit) = (p(r.x0, 0.0), p(r.x1, 0.0));
    Notch {
        enter,
        exit,
        segs: lines(&[enter, p(r.x0, r.y1), p(r.x1, r.y1), exit]),
    }
}

/// The bottom edge is walked westwards: in at the right, out at the left.
fn rect_notch_bottom(r: Rect, height: f64) -> Notch {
    let (enter, exit) = (p(r.x1, height), p(r.x0, height));
    Notch {
        enter,
        exit,
        segs: lines(&[enter, p(r.x1, r.y0), p(r.x0, r.y0), exit]),
    }
}

/// A wave slot that runs out through the top edge (`from_top`) or the bottom edge: its spine
/// starts or ends at the edge-side inflection beyond the strip, the closed end is exact.
fn wave_notch(slot: &Slot, from_top: bool, height: f64) -> Notch {
    let period = slot.wave.unwrap_or(1.0);
    let visible = if from_top { slot.y1 } else { height - slot.y0 };
    // Half waves enough to reach one millimetre beyond the edge.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let count = ((visible + 1.0) / period).ceil() as u32;
    let reach = f64::from(count) * period;
    if from_top {
        let w = wave(
            (slot.x, slot.y1 - reach),
            period,
            slot.width,
            count,
            (Some(0.0), None),
        );
        let (l_end, r_end) = (
            w.left[w.left.len() - 1].end(),
            w.right[w.right.len() - 1].end(),
        );
        let mut segs = w.left.clone();
        segs.push(Seg::Line {
            from: l_end,
            to: r_end,
        });
        segs.extend(w.right.iter().rev().map(Seg::reversed));
        Notch {
            enter: w.left[0].start(),
            exit: w.right[0].start(),
            segs,
        }
    } else {
        let w = wave(
            (slot.x, slot.y0),
            period,
            slot.width,
            count,
            (None, Some(height)),
        );
        let (l_start, r_start) = (w.left[0].start(), w.right[0].start());
        let mut segs: Vec<Seg> = w.right.iter().rev().map(Seg::reversed).collect();
        segs.push(Seg::Line {
            from: r_start,
            to: l_start,
        });
        segs.extend(w.left.iter().copied());
        Notch {
            enter: w.right[w.right.len() - 1].end(),
            exit: w.left[w.left.len() - 1].end(),
            segs,
        }
    }
}

fn go(contour: &mut Contour, here: &mut Point, to: Point) {
    contour.line_to(*here, to);
    *here = to;
}

/// The rectangle `length` x `height` with notches in its top and bottom edges; the end edges
/// follow `ends` when a piece has them.
fn outline(
    length: f64,
    height: f64,
    mut top: Vec<Notch>,
    mut bottom: Vec<Notch>,
    ends: Option<&Ends>,
) -> Contour {
    top.sort_by(|a, b| a.enter.x.total_cmp(&b.enter.x));
    bottom.sort_by(|a, b| b.enter.x.total_cmp(&a.enter.x));
    let mut contour = Contour::new();
    let left = ends
        .and_then(|e| e.start.first())
        .map_or(0.0, |step| step.x);
    let mut here = p(left, 0.0);
    for notch in top {
        go(&mut contour, &mut here, notch.enter);
        for seg in notch.segs {
            contour.push(seg);
        }
        here = notch.exit;
    }
    if let Some(ends) = ends {
        for step in &ends.end {
            go(&mut contour, &mut here, p(step.x, step.y0));
            go(&mut contour, &mut here, p(step.x, step.y1));
        }
    } else {
        go(&mut contour, &mut here, p(length, 0.0));
        go(&mut contour, &mut here, p(length, height));
    }
    for notch in bottom {
        go(&mut contour, &mut here, notch.enter);
        for seg in notch.segs {
            contour.push(seg);
        }
        here = notch.exit;
    }
    if let Some(ends) = ends {
        for step in ends.start.iter().rev() {
            go(&mut contour, &mut here, p(step.x, step.y1));
            go(&mut contour, &mut here, p(step.x, step.y0));
        }
    } else {
        go(&mut contour, &mut here, p(0.0, height));
        go(&mut contour, &mut here, p(0.0, 0.0));
    }
    contour
}

fn hole(r: Rect) -> Contour {
    polyline(&[p(r.x0, r.y0), p(r.x0, r.y1), p(r.x1, r.y1), p(r.x1, r.y0)])
}

fn polyline(points: &[Point]) -> Contour {
    let mut contour = Contour::new();
    for (index, from) in points.iter().enumerate() {
        contour.line_to(*from, points[(index + 1) % points.len()]);
    }
    contour
}
