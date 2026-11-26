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
    /// A wall piece of a path box starts at this span, at a corner: runs of bends and windows
    /// stop here (ADR 0083 §13.4).
    pub(crate) piece_start: bool,
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
    /// How far a tab keeps from either end of a straight (the wall thickness for the rounded
    /// box; the finger zone and the wall thickness beside a corner of a path box).
    pub(crate) margin: f64,
    /// Tabs that keep a flat window on a bend although the tab could stand on the arc
    /// (`bend_tabs` `curve`); the generator tells the maker.
    pub(crate) windowed: u32,
}

/// A piece of the cycle before the seam is chosen.
pub(crate) struct Piece {
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
        Self::start_at(pieces, wall, (index, at), seam_in_straight.is_some())
    }

    /// The footprint of the closed loop `shapes` with its seam at distance `at` along shape
    /// `index` (a loop without a corner, ADR 0083 §6).
    pub(crate) fn from_loop(
        shapes: &[Shape],
        wall: f64,
        (index, at, seam): (usize, f64, bool),
    ) -> Self {
        let pieces: Vec<Piece> = shapes
            .iter()
            .enumerate()
            .map(|(rank, shape)| Piece {
                shape: *shape,
                rank,
            })
            .collect();
        Self::start_at(&pieces, wall, (index, at), seam)
    }

    /// The spans of the cycle `pieces` started at distance `at` along piece `index`: that piece's
    /// second half starts the strip, its first half ends it. `seam` says the cut is the middle
    /// of a straight, whose two halves then count as one whole straight.
    pub(crate) fn start_at(
        pieces: &[Piece],
        wall: f64,
        (index, at): (usize, f64),
        seam: bool,
    ) -> Self {
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
                piece_start: false,
            });
            s0 += length;
        }
        Self {
            spans,
            wall,
            margin: wall,
            windowed: 0,
        }
    }

    /// The footprint of `shapes` in walk order from the first one's start, which is a corner
    /// (a piece starts there); every straight is a whole straight, there is no seam.
    pub(crate) fn from_shapes(shapes: &[Shape], wall: f64, margin: f64) -> Self {
        let mut s0 = 0.0;
        let spans = shapes
            .iter()
            .enumerate()
            .map(|(id, shape)| {
                let length = neutral_length(shape, wall);
                let span = Span {
                    shape: *shape,
                    s0,
                    length,
                    whole: matches!(shape, Shape::Straight { .. }).then(|| Whole {
                        id,
                        offset: 0.0,
                        length,
                        seam: false,
                    }),
                    window: false,
                    piece_start: false,
                };
                s0 += length;
                span
            })
            .collect();
        Self {
            spans,
            wall,
            margin,
            windowed: 0,
        }
    }

    /// Marks the spans a wall piece starts at: `piece_of[i]` is the piece of span `i` (a footprint
    /// from `from_shapes`, one span per shape).
    pub(crate) fn mark_pieces(&mut self, piece_of: &[usize]) {
        for (index, span) in self.spans.iter_mut().enumerate() {
            span.piece_start = index == 0 || piece_of.get(index) != piece_of.get(index - 1);
        }
    }

    /// Makes the spans with a non-zero entry of `turns` flats: a flat is a chord that replaces a
    /// stretch turning by that much (signed), so its neutral length holds half a wall thickness
    /// times the turn and it carries no tab (ADR 0083 §13.5, §13.7).
    pub(crate) fn with_flat_turns(&mut self, turns: &[f64]) {
        let mut s0 = 0.0;
        for (span, &turn) in self.spans.iter_mut().zip(turns) {
            if turn != 0.0 {
                span.whole = None;
                span.length += self.wall / 2.0 * turn;
            }
            span.s0 = s0;
            s0 += span.length;
        }
    }

    /// The strip's length on the neutral line.
    pub(crate) fn total(&self) -> f64 {
        self.spans.iter().map(|span| span.length).sum()
    }
}

pub(crate) fn neutral_length(shape: &Shape, wall: f64) -> f64 {
    match *shape {
        Shape::Straight { .. } => straight_length(shape),
        Shape::Bend { radius, sweep, .. } => {
            sweep.abs() * (radius + sweep.signum() * wall / 2.0).max(0.0)
        }
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
pub(crate) fn split(shape: &Shape, at: f64) -> (Shape, Option<Shape>) {
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
#[path = "footprint_tests.rs"]
mod tests;
