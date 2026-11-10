//! What one kind runs of one leaf, by the leaf's outline topology (criteria 19 to 24); the
//! geometry itself (fill, union, outset) is computed elsewhere from this answer.

use crate::operation_kind::OperationKind;

/// The topology of one drawn leaf as the run rules need it. Style (fill switch, colour, stroke,
/// dash) is not part of it: style never changes what runs (criterion 24).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeafShape {
    /// A path, text or primitive with this many closed and open outlines.
    Outlines {
        /// Closed outlines.
        closed: usize,
        /// Open outlines.
        open: usize,
    },
    /// An image: its frame and its pixels.
    Image,
}

/// Which outlines of a leaf run as lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lines {
    /// None.
    None,
    /// Every outline as drawn, an open one stays open, in the leaf's own path order.
    All,
    /// Every closed outline, traced once.
    Closed,
    /// The image's frame.
    Frame,
}

/// What one kind runs of one leaf.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KindRun {
    /// The lines that run.
    pub lines: Lines,
    /// The leaf runs as one filled area by its own fill rule, its open outlines closed by a
    /// straight line.
    pub area: bool,
    /// The image is raster engraved (its pixels are output by `0029`).
    pub raster: bool,
    /// The leaf joins its item's outer contour (Cut-out, one contour per item).
    pub contour: bool,
}

impl KindRun {
    const NOTHING: Self = Self {
        lines: Lines::None,
        area: false,
        raster: false,
        contour: false,
    };

    /// Whether nothing runs.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        *self == Self::NOTHING
    }
}

/// What `kind` runs of a leaf of shape `shape`.
#[must_use]
pub fn kind_run(kind: OperationKind, shape: LeafShape) -> KindRun {
    let nothing = KindRun::NOTHING;
    match (kind, shape) {
        (OperationKind::Cut, LeafShape::Image) => KindRun {
            lines: Lines::Frame,
            ..nothing
        },
        (OperationKind::Score, LeafShape::Image)
        | (OperationKind::CutOut, LeafShape::Outlines { closed: 0, open: 0 }) => nothing,
        (OperationKind::Cut | OperationKind::Score, LeafShape::Outlines { closed, open }) => {
            KindRun {
                lines: if closed + open == 0 {
                    Lines::None
                } else {
                    Lines::All
                },
                ..nothing
            }
        }
        (OperationKind::Engrave, LeafShape::Image) => KindRun {
            raster: true,
            ..nothing
        },
        (OperationKind::EngraveOutline, LeafShape::Image) => KindRun {
            lines: Lines::Frame,
            raster: true,
            ..nothing
        },
        (
            OperationKind::Engrave | OperationKind::EngraveOutline,
            LeafShape::Outlines { closed, open },
        ) => match (closed, open) {
            (0, 0) => nothing,
            (0, _) => KindRun {
                lines: Lines::All,
                ..nothing
            },
            _ => KindRun {
                area: true,
                lines: if kind == OperationKind::EngraveOutline {
                    Lines::Closed
                } else {
                    Lines::None
                },
                ..nothing
            },
        },
        (OperationKind::CutOut, _) => KindRun {
            contour: true,
            ..nothing
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use OperationKind::{Cut, CutOut, Engrave, EngraveOutline, Score};

    const CLOSED: LeafShape = LeafShape::Outlines { closed: 1, open: 0 };
    const OPEN: LeafShape = LeafShape::Outlines { closed: 0, open: 2 };
    const MIXED: LeafShape = LeafShape::Outlines { closed: 1, open: 1 };

    #[test]
    fn cut_and_score_run_every_outline_as_a_line() {
        for kind in [Cut, Score] {
            for shape in [CLOSED, OPEN, MIXED] {
                assert_eq!(
                    kind_run(kind, shape).lines,
                    Lines::All,
                    "{kind:?} {shape:?}"
                );
                assert!(!kind_run(kind, shape).area);
            }
        }
        assert_eq!(
            kind_run(Cut, LeafShape::Image).lines,
            Lines::Frame,
            "criterion 19"
        );
        assert!(
            !kind_run(Cut, LeafShape::Image).raster,
            "its pixels do not run"
        );
        assert!(kind_run(Score, LeafShape::Image).is_empty(), "criterion 20");
    }

    #[test]
    fn engrave_fills_a_leaf_with_a_closed_outline_and_lines_an_all_open_one() {
        assert_eq!(
            kind_run(Engrave, MIXED),
            KindRun {
                lines: Lines::None,
                area: true,
                raster: false,
                contour: false
            },
            "criterion 21: one closed and one open outline run as one area"
        );
        assert!(kind_run(Engrave, CLOSED).area);
        assert_eq!(kind_run(Engrave, OPEN).lines, Lines::All);
        assert!(!kind_run(Engrave, OPEN).area);
        assert!(kind_run(Engrave, LeafShape::Image).raster);
    }

    #[test]
    fn engrave_outline_adds_the_closed_outlines_and_an_image_frame() {
        let run = kind_run(EngraveOutline, MIXED);
        assert!(run.area);
        assert_eq!(run.lines, Lines::Closed, "criterion 22");
        assert_eq!(kind_run(EngraveOutline, OPEN).lines, Lines::All);
        let image = kind_run(EngraveOutline, LeafShape::Image);
        assert!(image.raster);
        assert_eq!(image.lines, Lines::Frame);
    }

    #[test]
    fn cut_out_joins_every_leaf_to_its_items_contour() {
        for shape in [CLOSED, OPEN, MIXED, LeafShape::Image] {
            let run = kind_run(CutOut, shape);
            assert!(
                run.contour && run.lines == Lines::None && !run.area,
                "{shape:?}"
            );
        }
        let empty = LeafShape::Outlines { closed: 0, open: 0 };
        for kind in OperationKind::ALL {
            assert!(kind_run(kind, empty).is_empty(), "{kind:?}");
        }
    }
}
