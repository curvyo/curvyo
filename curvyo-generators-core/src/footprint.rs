//! The plan outline of a wall as spans: straight runs and bent runs of the inner face, from the
//! strip's seam round the box, with their lengths on the neutral line and the places of the tabs.
//!
//! Screen coordinates, the walk is clockwise. Lengths are millimetres.

use curvyo_document_core::{Length, Point};

use crate::outline::polar;

/// A straight shorter than this is no straight (ADR 0082 §5).
const ZERO_STRAIGHT: f64 = 0.01;
/// The seam goes into a straight only when it holds this many wall thicknesses.
const MIN_SEAM_STRAIGHT: f64 = 8.0;

/// The shape of one span, on the inner face.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Shape {
    Straight {
        from: Point,
        to: Point,
    },
    Bend {
        centre: Point,
        radius: f64,
        from: f64,
        sweep: f64,
    },
}

/// One run of the wall.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Span {
    pub(crate) shape: Shape,
    /// Where the span starts on the strip, along the neutral line.
    pub(crate) s0: f64,
    /// Its length on the neutral line.
    pub(crate) length: f64,
    /// For a straight: which whole straight it belongs to, where it starts on it and how long that
    /// whole straight is (the seam splits one straight into two spans).
    pub(crate) whole: Option<Whole>,
    /// A flat window on a bend: the chord a tab stands on, with no slit column (ADR 0082 §5).
    pub(crate) window: bool,
}

/// A straight as the maker sees it, possibly split by the seam.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Whole {
    pub(crate) id: usize,
    pub(crate) offset: f64,
    pub(crate) length: f64,
    pub(crate) seam: bool,
}

/// A tab on a straight: where it sits on the strip and in the plan.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Tab {
    /// The centre on the strip, along the neutral line.
    pub(crate) s: f64,
    /// The centre on the inner face.
    pub(crate) centre: Point,
    /// The unit direction of the walk there.
    pub(crate) along: (f64, f64),
}

/// The wall's footprint: spans from the seam, clockwise.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Footprint {
    pub(crate) spans: Vec<Span>,
    /// The wall thickness the neutral line is laid out for.
    pub(crate) wall: f64,
}

/// A piece of the cycle before the seam is chosen.
struct Piece {
    shape: Shape,
    rank: usize,
}

pub(crate) fn straight_length(shape: &Shape) -> f64 {
    match *shape {
        Shape::Straight { from, to } => libm::hypot(to.x - from.x, to.y - from.y),
        Shape::Bend { .. } => 0.0,
    }
}

impl Footprint {
    /// The rounded rectangle of inner width `width`, depth `depth` and corner radius `radius`
    /// (snapped to the half of the shorter side when within 0.01 mm of it) for a wall of
    /// thickness `wall`.
    pub(crate) fn rounded_rect(width: Length, depth: Length, radius: Length, wall: Length) -> Self {
        let (w, d, wall) = (width.as_mm(), depth.as_mm(), wall.as_mm());
        let limit = w.min(d) / 2.0;
        let r = if radius.as_mm() > limit - ZERO_STRAIGHT {
            limit
        } else {
            radius.as_mm()
        };
        let (hw, hd) = (w / 2.0, d / 2.0);
        let q = std::f64::consts::FRAC_PI_2;
        let line = |from: (f64, f64), to: (f64, f64)| Shape::Straight {
            from: Point::new(from.0, from.1),
            to: Point::new(to.0, to.1),
        };
        let bend = |cx: f64, cy: f64, from: f64| Shape::Bend {
            centre: Point::new(cx, cy),
            radius: r,
            from,
            sweep: q,
        };
        let cycle = [
            (line((-hw + r, -hd), (hw - r, -hd)), 0),
            (bend(hw - r, -hd + r, -q), 4),
            (line((hw, -hd + r), (hw, hd - r)), 1),
            (bend(hw - r, hd - r, 0.0), 4),
            (line((hw - r, hd), (-hw + r, hd)), 2),
            (bend(-hw + r, hd - r, q), 4),
            (line((-hw, hd - r), (-hw, -hd + r)), 3),
            (bend(-hw + r, -hd + r, 2.0 * q), 4),
        ];
        let mut pieces: Vec<Piece> = cycle
            .into_iter()
            .filter(|(shape, _)| match shape {
                Shape::Bend { radius, .. } => *radius >= ZERO_STRAIGHT,
                Shape::Straight { .. } => straight_length(shape) >= ZERO_STRAIGHT,
            })
            .map(|(shape, rank)| Piece { shape, rank })
            .collect();
        merge_bends(&mut pieces);
        Self::from_seam(&pieces, wall, (w, d))
    }

    /// The spans from the seam: in the middle of the longest straight when it is long enough,
    /// else at the middle of the back (top) side.
    fn from_seam(pieces: &[Piece], wall: f64, size: (f64, f64)) -> Self {
        let longest = pieces
            .iter()
            .enumerate()
            .filter(|(_, piece)| matches!(piece.shape, Shape::Straight { .. }))
            .max_by(|(_, a), (_, b)| {
                straight_length(&a.shape)
                    .total_cmp(&straight_length(&b.shape))
                    .then(b.rank.cmp(&a.rank))
            })
            .map(|(index, _)| index);
        let seam_in_straight = longest
            .filter(|&index| straight_length(&pieces[index].shape) >= MIN_SEAM_STRAIGHT * wall);
        let (index, at) = match seam_in_straight {
            Some(index) => (index, straight_length(&pieces[index].shape) / 2.0),
            None => back_middle(pieces, size),
        };
        let seam = seam_in_straight.is_some();
        let whole_of =
            |piece_index: usize, offset: f64, is_seam: bool| match pieces[piece_index].shape {
                Shape::Straight { .. } => Some(Whole {
                    id: piece_index,
                    offset,
                    length: straight_length(&pieces[piece_index].shape),
                    seam: is_seam,
                }),
                Shape::Bend { .. } => None,
            };
        // Split the piece at the seam; the second half starts the strip, the first half ends it.
        let (first, second) = split(&pieces[index].shape, at);
        let count = pieces.len();
        let mut ordered: Vec<(Shape, Option<Whole>)> = Vec::new();
        match second {
            Some(second) => ordered.push((second, whole_of(index, at, seam))),
            None => ordered.push((first, whole_of(index, 0.0, false))),
        }
        for step in 1..count {
            let at_index = (index + step) % count;
            ordered.push((pieces[at_index].shape, whole_of(at_index, 0.0, false)));
        }
        if second.is_some() {
            ordered.push((first, whole_of(index, 0.0, seam)));
        }
        let mut spans = Vec::with_capacity(ordered.len());
        let mut s0 = 0.0;
        for (shape, whole) in ordered {
            let length = neutral_length(&shape, wall);
            spans.push(Span {
                shape,
                s0,
                length,
                whole,
                window: false,
            });
            s0 += length;
        }
        Self { spans, wall }
    }

    /// The strip's length on the neutral line.
    pub(crate) fn total(&self) -> f64 {
        self.spans.iter().map(|span| span.length).sum()
    }
}

fn neutral_length(shape: &Shape, wall: f64) -> f64 {
    match *shape {
        Shape::Straight { .. } => straight_length(shape),
        Shape::Bend { radius, sweep, .. } => sweep.abs() * (radius + wall / 2.0),
    }
}

/// Neighbouring bends on one centre become one bend (a zero straight between them was dropped).
fn merge_bends(pieces: &mut Vec<Piece>) {
    let same = |a: &Shape, b: &Shape| match (a, b) {
        (Shape::Bend { centre: c1, .. }, Shape::Bend { centre: c2, .. }) => {
            libm::hypot(c1.x - c2.x, c1.y - c2.y) < 1e-9
        }
        _ => false,
    };
    let mut index = 0;
    while pieces.len() > 1 && index + 1 < pieces.len() {
        if same(&pieces[index].shape, &pieces[index + 1].shape) {
            let next = pieces.remove(index + 1);
            if let (Shape::Bend { sweep, .. }, Shape::Bend { sweep: extra, .. }) =
                (&mut pieces[index].shape, next.shape)
            {
                *sweep += extra;
            }
        } else {
            index += 1;
        }
    }
    // The last bend may continue into the first (the cycle starts mid-bend).
    while pieces.len() > 1 {
        let last = pieces.len() - 1;
        if !same(&pieces[last].shape, &pieces[0].shape) {
            break;
        }
        let tail = pieces.remove(last);
        if let (
            Shape::Bend { sweep, from, .. },
            Shape::Bend {
                sweep: extra,
                from: tail_from,
                ..
            },
        ) = (&mut pieces[0].shape, tail.shape)
        {
            *sweep += extra;
            *from = tail_from;
        }
    }
}

/// The piece and the distance along it of the back (top) side's middle point.
fn back_middle(pieces: &[Piece], size: (f64, f64)) -> (usize, f64) {
    let top = (0.0, -size.1 / 2.0);
    let mut best = (0, 0.0, f64::MAX);
    for (index, piece) in pieces.iter().enumerate() {
        let (distance, at) = match piece.shape {
            Shape::Straight { from, to } => {
                let length = straight_length(&piece.shape);
                let t = (((top.0 - from.x) * (to.x - from.x) + (top.1 - from.y) * (to.y - from.y))
                    / (length * length))
                    .clamp(0.0, 1.0);
                let point = (from.x + t * (to.x - from.x), from.y + t * (to.y - from.y));
                (libm::hypot(point.0 - top.0, point.1 - top.1), t * length)
            }
            Shape::Bend {
                centre,
                radius,
                from,
                sweep,
            } => {
                let target = -std::f64::consts::FRAC_PI_2;
                let mut turn = (target - from).rem_euclid(std::f64::consts::TAU);
                if turn > sweep.abs() {
                    turn = if turn - sweep.abs() < std::f64::consts::TAU - turn {
                        sweep.abs()
                    } else {
                        0.0
                    };
                }
                let point = polar(centre, radius, from + turn);
                (libm::hypot(point.x - top.0, point.y - top.1), turn * radius)
            }
        };
        if distance < best.2 - 1e-9 {
            best = (index, at, distance);
        }
    }
    (best.0, best.1)
}

/// The shape cut `at` along it into the part before and the part after (`None` at either end).
fn split(shape: &Shape, at: f64) -> (Shape, Option<Shape>) {
    match *shape {
        Shape::Straight { from, to } => {
            let length = straight_length(shape);
            let t = at / length;
            let mid = Point::new(from.x + t * (to.x - from.x), from.y + t * (to.y - from.y));
            (
                Shape::Straight { from, to: mid },
                Some(Shape::Straight { from: mid, to }),
            )
        }
        Shape::Bend {
            centre,
            radius,
            from,
            sweep,
        } => {
            let turn = at / radius * sweep.signum();
            if turn.abs() < 1e-9 {
                (*shape, None)
            } else {
                (
                    Shape::Bend {
                        centre,
                        radius,
                        from,
                        sweep: turn,
                    },
                    Some(Shape::Bend {
                        centre,
                        radius,
                        from: from + turn,
                        sweep: sweep - turn,
                    }),
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mm(value: f64) -> Length {
        Length::from_mm(value)
    }

    fn footprint(w: f64, d: f64, r: f64, t: f64) -> Footprint {
        Footprint::rounded_rect(mm(w), mm(d), mm(r), mm(t))
    }

    #[test]
    fn the_strip_is_as_long_as_the_perimeter_on_the_neutral_line() {
        // An independent sum, not the spans' own lengths (ADR 0082 §9).
        for (w, d, r, t) in [
            (120.0, 80.0, 15.0, 3.0),
            (100.0, 40.0, 20.0, 3.0),
            (40.0, 100.0, 20.0, 3.0),
            (60.0, 60.0, 30.0, 3.0),
            (120.0, 80.0, 6.0, 3.0),
        ] {
            let expected =
                2.0 * (w - 2.0 * r) + 2.0 * (d - 2.0 * r) + std::f64::consts::PI * (2.0 * r + t);
            let total = footprint(w, d, r, t).total();
            assert!(
                (total - expected).abs() < 1e-9,
                "{w}x{d} r{r}: {total} vs {expected}"
            );
        }
    }

    #[test]
    fn the_seam_is_the_middle_of_the_back_straight() {
        let fp = footprint(120.0, 80.0, 15.0, 3.0);
        assert_eq!(
            fp.spans.len(),
            9,
            "two seam halves, four bends, three straights"
        );
        let Shape::Straight { from, .. } = fp.spans[0].shape else {
            panic!("the strip starts on a straight");
        };
        assert!(
            from.x.abs() < 1e-9 && (from.y + 40.0).abs() < 1e-9,
            "{from:?}"
        );
        let tall = footprint(40.0, 100.0, 20.0, 3.0);
        let Shape::Straight { from, .. } = tall.spans[0].shape else {
            panic!("a right straight");
        };
        assert!(
            (from.x - 20.0).abs() < 1e-9 && from.y.abs() < 1e-9,
            "the longer side: {from:?}"
        );
    }

    #[test]
    fn a_stadium_has_two_straights_and_two_half_circles() {
        let fp = footprint(100.0, 40.0, 20.0, 3.0);
        let bends: Vec<f64> = fp
            .spans
            .iter()
            .filter_map(|span| match span.shape {
                Shape::Bend { sweep, .. } => Some(sweep),
                Shape::Straight { .. } => None,
            })
            .collect();
        assert_eq!(bends.len(), 2);
        for sweep in bends {
            assert!((sweep - std::f64::consts::PI).abs() < 1e-9, "{sweep}");
        }
    }

    #[test]
    fn a_circle_is_one_closed_bend_from_the_back_point() {
        let fp = footprint(60.0, 60.0, 29.995, 3.0);
        assert_eq!(fp.spans.len(), 1, "no straight, no zero-length piece");
        let Shape::Bend {
            centre,
            radius,
            from,
            sweep,
        } = fp.spans[0].shape
        else {
            panic!("a bend");
        };
        assert!(centre.x.abs() < 1e-9 && centre.y.abs() < 1e-9);
        assert!((radius - 30.0).abs() < 1e-9, "snapped to half the side");
        assert!((from + std::f64::consts::FRAC_PI_2).abs() < 1e-9);
        assert!((sweep - std::f64::consts::TAU).abs() < 1e-9);
    }

    #[test]
    fn tabs_sit_in_equal_sub_spans_and_the_seam_falls_between_two_tabs() {
        let fp = footprint(120.0, 80.0, 15.0, 3.0);
        let tabs = fp.straight_tabs(3.0);
        // Top 90 mm: 3 rounds up to the even 4; right 50: 2; bottom 90: 3; left 50: 2.
        assert_eq!(tabs.len(), 4 + 2 + 3 + 2);
        let total = fp.total();
        let seam_gap = tabs
            .iter()
            .map(|tab| tab.s.min(total - tab.s))
            .fold(f64::MAX, f64::min);
        assert!(
            (seam_gap - 11.25).abs() < 1e-9,
            "half a pitch from the seam: {seam_gap}"
        );
        for tab in &tabs {
            assert!(tab.s > 0.0 && tab.s < total);
        }
    }

    #[test]
    fn a_short_straight_has_no_tab() {
        // 40 x 28 with r 10: straights of 20 and 8 mm; wall 3 -> 8 < 4 * 3 has none.
        let fp = footprint(40.0, 28.0, 10.0, 3.0);
        let tabs = fp.straight_tabs(3.0);
        assert!(
            tabs.iter()
                .all(|tab| (tab.centre.y.abs() - 14.0).abs() < 1e-9)
        );
    }
}
