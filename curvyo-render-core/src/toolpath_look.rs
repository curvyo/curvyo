//! Draws a cached toolpath up to a head position (`specs/0029-laser-job-preview-and-output`
//! criteria 8, 9, 11, 37; ADR 0064 §10; UX notes §4, §5): burn moves in their kind's look, travel
//! dashed, a ring per run, an arrowhead per burn path, the part after the head pale.

use curvyo_document_core::{Point, Vec2, ViewTransform};

use crate::color::RgbaColor;
use crate::glyphs::{DrawList, circle, push_thick_line, ring};
use crate::theme::{OP_CUT, OP_ENGRAVE_HATCH, OP_INK, OP_LANE_LINE, SELECTION_CASING};

/// How a move is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolpathStroke {
    /// A Cut burn: 1 px solid red.
    Cut,
    /// A Cut-out burn: 2 px red, dashed 6 on 3.
    CutOut,
    /// A Score burn (and an Engrave + Outline's outline): 1.5 px ink, dotted.
    Score,
    /// An Engrave scan line: 1 px pale orange, thinned to 3 px apart.
    Engrave,
    /// A travel: 1 px lane line, dashed 4 on 3.
    Travel,
}

/// One drawn move in document space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ToolpathSegment {
    /// The move's index in the job (ascending over the look).
    pub index: usize,
    /// Where it starts.
    pub from: Point,
    /// Where it ends.
    pub to: Point,
    /// Its look.
    pub stroke: ToolpathStroke,
    /// An Engrave scan line's number in its run (thinning, UX P4); 0 for every other stroke.
    pub line: u32,
    /// An Engrave run's interval in mm; 0 for every other stroke.
    pub spacing_mm: f64,
}

/// The arrowhead at the start of a burn path.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ToolpathArrow {
    /// The move it starts on.
    pub index: usize,
    /// The path's first point.
    pub at: Point,
    /// The direction of its first move, unit length.
    pub direction: Vec2,
    /// The path's length in mm (no arrowhead below 12 screen px, UX P6).
    pub length_mm: f64,
    /// Its look.
    pub stroke: ToolpathStroke,
}

/// A whole job's toolpath in document space, built once per job.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ToolpathLook {
    /// The drawn moves in job order.
    pub segments: Vec<ToolpathSegment>,
    /// One arrowhead per burn path (one per Engrave area).
    pub arrows: Vec<ToolpathArrow>,
    /// The start of each run's first burn path, in run order.
    pub rings: Vec<Point>,
}

/// Where the head is: the moves before `index` are done, the move `index` is done up to `at`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ToolpathHead {
    /// The first move not done in full.
    pub index: usize,
    /// The head's point.
    pub at: Point,
}

/// The screen offset of a run's ring from its point, up and left, in px (UX notes §4).
pub const RING_OFFSET_PX: f64 = 12.0;
/// The ring's diameter in px.
pub const RING_DIAMETER_PX: f64 = 17.0;
/// At most this many arrowheads per frame (UX notes §13).
pub const MAX_ARROWS: usize = 2_000;
/// The alpha after the head: 35 % (UX P7).
const PALE_ALPHA: u8 = 89;
/// The width after the head: 60 % (UX P7).
const PALE_WIDTH: f64 = 0.6;

/// The width in px, the colour and the dash (on, off in px; `None` solid) of a stroke.
const fn stroke_style(stroke: ToolpathStroke) -> (f64, RgbaColor, Option<(f64, f64)>) {
    match stroke {
        ToolpathStroke::Cut => (1.0, OP_CUT, None),
        ToolpathStroke::CutOut => (2.0, OP_CUT, Some((6.0, 3.0))),
        ToolpathStroke::Score => (1.5, OP_INK, Some((1.5, 2.0))),
        ToolpathStroke::Engrave => (1.0, OP_ENGRAVE_HATCH, None),
        ToolpathStroke::Travel => (1.0, OP_LANE_LINE, Some((4.0, 3.0))),
    }
}

/// A dash walker that keeps its phase along a run of joined segments.
struct Dasher {
    phase: f64,
    last: Option<Point>,
}

impl Dasher {
    /// Draws `a` to `b` dashed `(on, off)` in mm, continuing the phase when `a` is where the
    /// last segment ended.
    fn draw(
        &mut self,
        list: &mut DrawList,
        (a, b): (Point, Point),
        width: f64,
        (on, off): (f64, f64),
        color: RgbaColor,
    ) {
        if self.last != Some(a) {
            self.phase = 0.0;
        }
        self.last = Some(b);
        let length = a.vector_to(b).length();
        let period = on + off;
        if !length.is_finite() || length <= 0.0 || period <= 0.0 {
            return;
        }
        let step = a.vector_to(b).scaled(1.0 / length);
        let mut at = -self.phase;
        while at < length {
            let start = at.max(0.0);
            let end = (at + on).min(length);
            if end > start {
                push_thick_line(
                    list,
                    a.translated(step.scaled(start)),
                    a.translated(step.scaled(end)),
                    width,
                    color,
                );
            }
            at += period;
        }
        self.phase = (self.phase + length) % period;
    }
}

/// Draws `segments` with full weight or pale.
fn draw_segments(
    list: &mut DrawList,
    segments: &[ToolpathSegment],
    px: f64,
    pale: bool,
    head: Option<(usize, Point, bool)>,
) {
    let mut dasher = Dasher {
        phase: 0.0,
        last: None,
    };
    for segment in segments {
        let (from, to) = match head {
            Some((index, at, true)) if segment.index == index => (segment.from, at),
            Some((index, at, false)) if segment.index == index => (at, segment.to),
            _ => (segment.from, segment.to),
        };
        if segment.stroke == ToolpathStroke::Engrave && segment.spacing_mm > 0.0 {
            let spacing_px = segment.spacing_mm / px;
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            // invariant: a ratio of positive finite values, at least 1, far below u32::MAX
            let every = (3.0 / spacing_px).ceil().clamp(1.0, f64::from(u32::MAX)) as u32;
            if segment.line % every != 0 {
                continue;
            }
        }
        let (width_px, color, dash) = stroke_style(segment.stroke);
        let (width, color) = if pale {
            (width_px * PALE_WIDTH * px, color.with_alpha(PALE_ALPHA))
        } else {
            (width_px * px, color)
        };
        match dash {
            None => {
                dasher.last = None;
                push_thick_line(list, from, to, width, color);
            }
            Some((on, off)) => dasher.draw(list, (from, to), width, (on * px, off * px), color),
        }
    }
}

/// An open chevron 5 px long at `at` pointing along `direction`, with a 3 px white casing.
fn arrowhead(list: &mut DrawList, arrow: &ToolpathArrow, px: f64) {
    let (width_px, color, _) = stroke_style(arrow.stroke);
    let back = arrow.direction.scaled(-5.0 * px);
    let side = Vec2::new(-arrow.direction.y, arrow.direction.x).scaled(3.0 * px);
    let tip = arrow.at.translated(arrow.direction.scaled(5.0 * px));
    let wings = [
        tip.translated(back).translated(side),
        tip.translated(back).translated(side.negated()),
    ];
    for wing in wings {
        push_thick_line(list, wing, tip, (width_px + 3.0) * px, SELECTION_CASING);
    }
    for wing in wings {
        push_thick_line(list, wing, tip, width_px.max(1.5) * px, color);
    }
}

/// The draw list of `look` in `view`, the moves after `head` pale with the head's ring; `None`
/// draws the whole job at full weight.
#[must_use]
pub fn build_toolpath(
    look: &ToolpathLook,
    head: Option<ToolpathHead>,
    view: ViewTransform,
) -> DrawList {
    let px = 1.0 / view.scale();
    let mut list = DrawList::default();
    let split = head.map_or(look.segments.len(), |h| {
        look.segments.partition_point(|s| s.index < h.index)
    });
    let (done, rest) = look.segments.split_at(split);
    let pending_head = head.map(|h| (h.index, h.at, false));
    draw_segments(&mut list, rest, px, true, pending_head);
    draw_segments(&mut list, done, px, false, None);
    if let (Some(h), Some(first)) = (head, rest.first())
        && first.index == h.index
    {
        draw_segments(
            &mut list,
            std::slice::from_ref(first),
            px,
            false,
            Some((h.index, h.at, true)),
        );
    }
    let mut arrows = 0;
    for arrow in &look.arrows {
        if arrows >= MAX_ARROWS {
            break;
        }
        if arrow.length_mm / px < 12.0 {
            continue;
        }
        arrowhead(&mut list, arrow, px);
        arrows += 1;
    }
    let offset = Vec2::new(-RING_OFFSET_PX * px, -RING_OFFSET_PX * px);
    for &at in &look.rings {
        let centre = at.translated(offset);
        let edge = at
            .translated(offset.scaled(
                1.0 - RING_DIAMETER_PX / 2.0 / (RING_OFFSET_PX * std::f64::consts::SQRT_2),
            ));
        push_thick_line(&mut list, at, edge, px, OP_INK);
        list.extend(circle(centre, RING_DIAMETER_PX * px, SELECTION_CASING));
        list.extend(ring(centre, RING_DIAMETER_PX * px, 1.5 * px, OP_INK));
    }
    if let Some(h) = head {
        list.extend(head_ring(h.at, view));
    }
    list
}

/// The head of the play bar and of a running frame: a ring with its casing and a dot, constant
/// in screen size (`specs/0029` UX notes 5, `specs/0118` W33).
#[must_use]
pub fn head_ring(at: Point, view: ViewTransform) -> DrawList {
    let px = 1.0 / view.scale();
    let mut list = ring(at, 20.0 * px, 4.0 * px, SELECTION_CASING);
    list.extend(ring(at, 16.0 * px, 2.0 * px, OP_INK));
    list.extend(circle(at, 2.0 * px, OP_INK));
    list
}

#[cfg(test)]
mod tests {
    use super::*;

    fn segment(index: usize, x0: f64, x1: f64, stroke: ToolpathStroke) -> ToolpathSegment {
        ToolpathSegment {
            index,
            from: Point::new(x0, 0.0),
            to: Point::new(x1, 0.0),
            stroke,
            line: 0,
            spacing_mm: 0.0,
        }
    }

    fn look() -> ToolpathLook {
        ToolpathLook {
            segments: vec![
                segment(0, 0.0, 10.0, ToolpathStroke::Travel),
                segment(1, 10.0, 20.0, ToolpathStroke::Cut),
                segment(2, 20.0, 30.0, ToolpathStroke::Cut),
            ],
            arrows: Vec::new(),
            rings: vec![Point::new(10.0, 0.0)],
        }
    }

    fn alphas(list: &DrawList) -> Vec<u8> {
        list.triangles.iter().map(|v| v.color.a).collect()
    }

    #[test]
    fn the_whole_job_draws_at_full_weight_without_a_head() {
        let list = build_toolpath(&look(), None, ViewTransform::new(1.0, Point::new(0.0, 0.0)));
        assert!(!list.triangles.is_empty(), "drawn");
        assert!(alphas(&list).iter().all(|&a| a == 255), "no pale vertex");
    }

    #[test]
    fn the_moves_after_the_head_are_pale_and_the_split_move_is_cut_at_it() {
        let head = ToolpathHead {
            index: 1,
            at: Point::new(15.0, 0.0),
        };
        let list = build_toolpath(
            &look(),
            Some(head),
            ViewTransform::new(1.0, Point::new(0.0, 0.0)),
        );
        let pale: Vec<_> = list
            .triangles
            .iter()
            .filter(|v| v.color.a == PALE_ALPHA)
            .collect();
        assert!(!pale.is_empty(), "a pale part");
        assert!(
            pale.iter().all(|v| v.position.x >= 15.0 - 1.0),
            "only after the head is pale"
        );
        let red_full = list
            .triangles
            .iter()
            .filter(|v| v.color == OP_CUT)
            .map(|v| v.position.x)
            .fold(f64::NEG_INFINITY, f64::max);
        assert!(
            (red_full - 15.0).abs() < 1e-9,
            "full red ends at the head: {red_full}"
        );
    }

    #[test]
    fn engrave_lines_closer_than_3_px_are_thinned() {
        let lines = |scale: f64| {
            let look = ToolpathLook {
                segments: (0..10)
                    .map(|i| ToolpathSegment {
                        line: i,
                        spacing_mm: 0.1,
                        ..segment(i as usize, 0.0, 10.0, ToolpathStroke::Engrave)
                    })
                    .collect(),
                ..ToolpathLook::default()
            };
            build_toolpath(&look, None, ViewTransform::new(scale, Point::new(0.0, 0.0)))
                .triangles
                .len()
                / 6
        };
        assert_eq!(lines(100.0), 10, "10 px apart: every line");
        assert_eq!(lines(10.0), 4, "1 px apart: every third line");
    }

    #[test]
    fn short_paths_get_no_arrowhead() {
        let mut look = look();
        look.rings.clear();
        look.arrows.push(ToolpathArrow {
            index: 1,
            at: Point::new(10.0, 0.0),
            direction: Vec2::new(1.0, 0.0),
            length_mm: 20.0,
            stroke: ToolpathStroke::Cut,
        });
        let view = |scale| ViewTransform::new(scale, Point::new(0.0, 0.0));
        let plain = build_toolpath(
            &ToolpathLook {
                arrows: Vec::new(),
                ..look.clone()
            },
            None,
            view(1.0),
        );
        assert!(build_toolpath(&look, None, view(1.0)).triangles.len() > plain.triangles.len());
        let small = build_toolpath(&look, None, view(0.5)).triangles.len();
        let plain_small = build_toolpath(
            &ToolpathLook {
                arrows: Vec::new(),
                ..look
            },
            None,
            view(0.5),
        );
        assert_eq!(
            small,
            plain_small.triangles.len(),
            "20 mm at 0.5 px/mm is 10 px"
        );
    }
}
