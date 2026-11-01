//! The pure translations of an align or distribute press.
//!
//! `specs/0061-align-and-distribute/` criteria 2 to 9: boxes and a reference
//! in, one translation per box out. No document, no selection, no UI.

use curvyo_document_core::{Length, Point, Tolerance, Vec2};

/// The tolerance a translation must exceed to count as a move: the geometric
/// tolerance of the earlier specs (1e-6 mm for drawn geometry, so a
/// sub-micrometre difference is "already there", criterion 3).
const MOVE_TOLERANCE: Tolerance = Tolerance::from_mm(1e-6);

/// An axis-aligned rectangle in document millimetres: a unit's true bounding
/// box, the selection's union box or a page's rectangle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AlignBox {
    /// Top-left corner: the smaller x and the smaller y (y grows downward).
    pub min: Point,
    /// Bottom-right corner: the larger x and the larger y.
    pub max: Point,
}

impl AlignBox {
    /// The box of the two extreme corners.
    #[must_use]
    pub fn new(min: Point, max: Point) -> Self {
        Self {
            min: Point::new(min.x.min(max.x), min.y.min(max.y)),
            max: Point::new(max.x.max(min.x), max.y.max(min.y)),
        }
    }

    /// The union of `boxes`: the box around all of them; `None` when empty.
    #[must_use]
    pub fn union(boxes: &[Self]) -> Option<Self> {
        let mut iter = boxes.iter();
        let first = *iter.next()?;
        Some(iter.fold(first, |a, b| {
            Self::new(
                Point::new(a.min.x.min(b.min.x), a.min.y.min(b.min.y)),
                Point::new(a.max.x.max(b.max.x), a.max.y.max(b.max.y)),
            )
        }))
    }

    /// The centre of the axis: the middle of `min` and `max` on `axis`.
    #[must_use]
    pub fn centre(&self, axis: AlignAxis) -> f64 {
        match axis {
            AlignAxis::Horizontal => f64::midpoint(self.min.x, self.max.x),
            AlignAxis::Vertical => f64::midpoint(self.min.y, self.max.y),
        }
    }

    /// The smaller coordinate on `axis` (the left or top edge).
    #[must_use]
    pub fn start(&self, axis: AlignAxis) -> f64 {
        match axis {
            AlignAxis::Horizontal => self.min.x,
            AlignAxis::Vertical => self.min.y,
        }
    }

    /// The larger coordinate on `axis` (the right or bottom edge).
    #[must_use]
    pub fn end(&self, axis: AlignAxis) -> f64 {
        match axis {
            AlignAxis::Horizontal => self.max.x,
            AlignAxis::Vertical => self.max.y,
        }
    }

    /// The extent on `axis`.
    #[must_use]
    pub fn extent(&self, axis: AlignAxis) -> f64 {
        self.end(axis) - self.start(axis)
    }
}

/// The two axes of an align or distribute command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlignAxis {
    /// Left, right and horizontal centres; a horizontal distribute.
    Horizontal,
    /// Top, bottom and vertical centres; a vertical distribute.
    Vertical,
}

/// Which edge or centre of every box an align moves onto the reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlignCommand {
    /// Left edges onto the reference's left edge.
    Left,
    /// Horizontal centres onto the reference's horizontal centre.
    CentreH,
    /// Right edges onto the reference's right edge.
    Right,
    /// Top edges onto the reference's top edge.
    Top,
    /// Vertical centres onto the reference's vertical centre.
    CentreV,
    /// Bottom edges onto the reference's bottom edge.
    Bottom,
}

impl AlignCommand {
    /// The axis the command works on.
    #[must_use]
    pub const fn axis(self) -> AlignAxis {
        match self {
            Self::Left | Self::CentreH | Self::Right => AlignAxis::Horizontal,
            Self::Top | Self::CentreV | Self::Bottom => AlignAxis::Vertical,
        }
    }

    /// The coordinate of `reference` this command moves every box onto.
    #[must_use]
    pub fn target(self, reference: &AlignBox) -> f64 {
        match self {
            Self::Left => reference.start(AlignAxis::Horizontal),
            Self::CentreH => reference.centre(AlignAxis::Horizontal),
            Self::Right => reference.end(AlignAxis::Horizontal),
            Self::Top => reference.start(AlignAxis::Vertical),
            Self::CentreV => reference.centre(AlignAxis::Vertical),
            Self::Bottom => reference.end(AlignAxis::Vertical),
        }
    }

    /// The coordinate of `box` this command moves.
    #[must_use]
    pub fn source(self, box_: &AlignBox) -> f64 {
        match self {
            Self::Left => box_.start(AlignAxis::Horizontal),
            Self::CentreH => box_.centre(AlignAxis::Horizontal),
            Self::Right => box_.end(AlignAxis::Horizontal),
            Self::Top => box_.start(AlignAxis::Vertical),
            Self::CentreV => box_.centre(AlignAxis::Vertical),
            Self::Bottom => box_.end(AlignAxis::Vertical),
        }
    }
}

/// How a distribute spaces the objects: equal gaps between the boxes, or
/// equal distances between the centres (criterion 6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DistributeMode {
    /// Equal gaps between neighbouring boxes (criterion 6), or the typed gap
    /// of criterion 8 when one is given.
    Gaps,
    /// Equal distances between the centres (criterion 6); the outermost
    /// always stay.
    Centres,
}

/// One translation per input box, in the input's order: the align of every
/// box onto `reference` by `command` (criterion 2). A box that already lies
/// on the reference gets the zero translation, and the other axis is never
/// touched (criterion 3).
#[must_use]
pub fn align_translations(
    boxes: &[AlignBox],
    reference: &AlignBox,
    command: AlignCommand,
) -> Vec<Vec2> {
    let axis = command.axis();
    let target = command.target(reference);
    boxes
        .iter()
        .map(|box_| {
            let delta = target - command.source(box_);
            translation(axis, delta)
        })
        .collect()
}

/// One translation per input box, in the input's order: the distribute of
/// `axis` by `mode` (criterion 6). The boxes are ordered by their position on
/// the axis; with [`DistributeMode::Gaps`] and no `gap` the first and last
/// stay and the others get equal gaps between them; with a typed `gap` the
/// first stays and the others are spaced from it (criterion 8) — from two
/// boxes, since the typed gap does not need the ends to define the spacing
/// (the customer's amendment of 2026-10-30). With [`DistributeMode::Centres`]
/// the first and last stay and the centres are evenly spaced between them.
/// Below that (fewer than three boxes, or two without a typed gap) nothing
/// changes (criterion 7): every translation is zero.
#[must_use]
pub fn distribute_translations(
    boxes: &[AlignBox],
    axis: AlignAxis,
    mode: DistributeMode,
    gap: Option<Length>,
) -> Vec<Vec2> {
    let zero = Vec2::ZERO;
    let floor_of = if gap.is_some() && mode == DistributeMode::Gaps {
        2
    } else {
        3
    };
    if boxes.len() < floor_of {
        return vec![zero; boxes.len()];
    }
    let order = order_on(boxes, axis);
    let first = boxes[order[0]];
    let last = boxes[order[order.len() - 1]];
    let mut placements = vec![0.0; boxes.len()];
    match mode {
        DistributeMode::Centres => {
            let start = first.centre(axis);
            let step = (last.centre(axis) - start)
                / f64::from(u32::try_from(boxes.len() - 1).unwrap_or(1));
            for (rank, &index) in order.iter().enumerate() {
                placements[index] = start + step * f64::from(u32::try_from(rank).unwrap_or(0))
                    - boxes[index].centre(axis);
            }
        }
        DistributeMode::Gaps => {
            // The typed gap of criterion 8 replaces the ends: the first
            // stays, the rest are spaced from their neighbour. Without one
            // the ends stay and the free space between them is shared evenly
            // over the gaps between the boxes (criterion 6).
            let gap = gap.map_or_else(
                || {
                    let extents: f64 = boxes.iter().map(|b| b.extent(axis)).sum();
                    (last.end(axis) - first.start(axis) - extents)
                        / f64::from(u32::try_from(boxes.len() - 1).unwrap_or(1))
                },
                Length::as_mm,
            );
            let mut cursor = first.start(axis);
            for &index in &order {
                placements[index] = cursor - boxes[index].start(axis);
                cursor += boxes[index].extent(axis) + gap;
            }
        }
    }
    placements
        .into_iter()
        .map(|placement| translation(axis, placement))
        .collect()
}

/// One translation per input box, in the input's order: the distribute of
/// `axis` across the `frame` (the active page) instead of between the
/// outermost boxes (criterion 16). Equal gaps: the n boxes, ordered by
/// position, leave n + 1 equal gaps — page edge to first, between them, last
/// to page edge; a typed `gap` puts the first box that gap from the
/// frame's start edge and spaces the rest by it (they may reach past the end
/// edge). Equal centres: the n centres cut the frame into n + 1 equal parts.
/// One box is therefore centred across the frame.
#[must_use]
pub fn distribute_on_frame_translations(
    boxes: &[AlignBox],
    frame: &AlignBox,
    axis: AlignAxis,
    mode: DistributeMode,
    gap: Option<Length>,
) -> Vec<Vec2> {
    let order = order_on(boxes, axis);
    let parts = f64::from(u32::try_from(boxes.len() + 1).unwrap_or(u32::MAX));
    let mut placements = vec![0.0; boxes.len()];
    match mode {
        DistributeMode::Centres => {
            let step = frame.extent(axis) / parts;
            let mut centre = frame.start(axis);
            for &index in &order {
                centre += step;
                placements[index] = centre - boxes[index].centre(axis);
            }
        }
        DistributeMode::Gaps => {
            let gap = gap.map_or_else(
                || {
                    let extents: f64 = boxes.iter().map(|b| b.extent(axis)).sum();
                    (frame.extent(axis) - extents) / parts
                },
                Length::as_mm,
            );
            let mut cursor = frame.start(axis) + gap;
            for &index in &order {
                placements[index] = cursor - boxes[index].start(axis);
                cursor += boxes[index].extent(axis) + gap;
            }
        }
    }
    placements
        .into_iter()
        .map(|placement| translation(axis, placement))
        .collect()
}

/// The indices of `boxes` ordered by their start on `axis`, stable, so ties
/// keep the input (paint) order and the result never depends on the
/// selection order.
fn order_on(boxes: &[AlignBox], axis: AlignAxis) -> Vec<usize> {
    let mut order: Vec<usize> = (0..boxes.len()).collect();
    order.sort_by(|&a, &b| {
        boxes[a]
            .start(axis)
            .total_cmp(&boxes[b].start(axis))
            .then(a.cmp(&b))
    });
    order
}

/// The translation `delta` on `axis`, zero when it does not exceed the move
/// tolerance (criterion 3: an object that already lies on the reference does
/// not move).
pub(crate) fn translation(axis: AlignAxis, delta: f64) -> Vec2 {
    if delta.abs() <= MOVE_TOLERANCE.as_mm() || !delta.is_finite() {
        return Vec2::ZERO;
    }
    match axis {
        AlignAxis::Horizontal => Vec2::new(delta, 0.0),
        AlignAxis::Vertical => Vec2::new(0.0, delta),
    }
}
