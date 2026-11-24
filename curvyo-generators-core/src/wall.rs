//! The developed wall: the strip outline with its slit slots, edge notches and the slots for the
//! tabs of the plates that sit inside it (ADR 0082 §4, §5).
//!
//! The strip is flat: `x` along the neutral line from the seam, `y` from the top edge down.

use curvyo_document_core::{Length, Point};

use crate::contour::{Contour, Seg};
use crate::footprint::{Footprint, Shape, Tab};
use crate::slit_bend::{Band, SlitPattern, Slot, Window, slots};
use crate::wave_slot::wave;

/// The slot a plate's tab needs: `thickness` across the strip, twice that along it, with its
/// outer face `inset` from the top or the bottom edge.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Joint {
    pub(crate) thickness: Length,
    pub(crate) inset: Length,
    pub(crate) from_top: bool,
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
            return vec![outline(length, height, Vec::new(), Vec::new())];
        }
        let mut rects = self.tab_rects(tabs, height);
        let windows: Vec<Window> = rects
            .iter()
            .map(|r| Window {
                from: Length::from_mm(r.x0),
                to: Length::from_mm(r.x1),
            })
            .collect();
        let mut waves: Vec<(Slot, bool)> = Vec::new();
        for band in self.bands(length, height) {
            for slot in slots(&band, &self.pattern, &windows) {
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
        let mut contours = vec![outline(length, height, top, bottom)];
        contours.extend(holes.map(hole));
        contours
    }

    /// The slots of the plates' tabs, one per tab and joint.
    fn tab_rects(&self, tabs: &[Tab], height: f64) -> Vec<Rect> {
        let mut rects = Vec::new();
        for tab in tabs {
            for joint in &self.joints {
                let (t, e) = (joint.thickness.as_mm(), joint.inset.as_mm());
                let (y0, y1) = if joint.from_top {
                    (e, e + t)
                } else {
                    (height - e - t, height - e)
                };
                rects.push(Rect {
                    x0: tab.s - t,
                    x1: tab.s + t,
                    y0,
                    y1,
                });
            }
        }
        rects
    }

    /// The stretches that carry slit columns: the bent spans, or the whole strip.
    fn bands(&self, length: f64, height: f64) -> Vec<Band> {
        let bridge = self.pattern.bridge.as_mm();
        let height = Length::from_mm(height);
        let band = |from: f64, to: f64| Band {
            from: Length::from_mm(from.max(bridge)),
            to: Length::from_mm(to.min(length - bridge)),
            height,
        };
        if self.slits_everywhere {
            return vec![band(0.0, length)];
        }
        // A bend ends one bridge from a window; at a straight it runs on to the junction.
        let spans = &self.footprint.spans;
        spans
            .iter()
            .enumerate()
            .filter(|(_, span)| matches!(span.shape, Shape::Bend { .. }))
            .map(|(index, span)| {
                let before = index.checked_sub(1).and_then(|i| spans.get(i));
                let after = spans.get(index + 1);
                let lead = if before.is_some_and(|s| s.window) {
                    bridge
                } else {
                    0.0
                };
                let trail = if after.is_some_and(|s| s.window) {
                    bridge
                } else {
                    0.0
                };
                band(span.s0 + lead, span.s0 + span.length - trail)
            })
            .filter(|band| band.to.as_mm() > band.from.as_mm())
            .collect()
    }
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

/// The rectangle `length` x `height` with notches in its top and bottom edges.
fn outline(length: f64, height: f64, mut top: Vec<Notch>, mut bottom: Vec<Notch>) -> Contour {
    top.sort_by(|a, b| a.enter.x.total_cmp(&b.enter.x));
    bottom.sort_by(|a, b| b.enter.x.total_cmp(&a.enter.x));
    let mut contour = Contour::new();
    let mut here = p(0.0, 0.0);
    for notch in top {
        go(&mut contour, &mut here, notch.enter);
        for seg in notch.segs {
            contour.push(seg);
        }
        here = notch.exit;
    }
    go(&mut contour, &mut here, p(length, 0.0));
    go(&mut contour, &mut here, p(length, height));
    for notch in bottom {
        go(&mut contour, &mut here, notch.enter);
        for seg in notch.segs {
            contour.push(seg);
        }
        here = notch.exit;
    }
    go(&mut contour, &mut here, p(0.0, height));
    go(&mut contour, &mut here, p(0.0, 0.0));
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
