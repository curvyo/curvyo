//! What one kind runs of one leaf, by the leaf's outline topology (criteria 19 to 24); the
//! geometry itself (fill, union, outset) is computed elsewhere from this answer.

use crate::operation_kind::OperationKind;

/// The topology of one drawn leaf as the run rules need it. Style (fill switch, colour, stroke,
/// dash) is not part of it, with one exception: a gradient fill makes a leaf a raster item of
/// the Image kind (`specs/0123` criterion 3; every other kind reads it as its outlines).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeafShape {
    /// A path, text or primitive with this many closed and open outlines.
    Outlines {
        /// Closed outlines.
        closed: usize,
        /// Open outlines.
        open: usize,
    },
    /// A leaf of this many closed and open outlines whose fill paints a gradient.
    Gradient {
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
}

/// What one kind runs of one leaf.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KindRun {
    /// The lines that run.
    pub lines: Lines,
    /// The leaf runs as one filled area by its own fill rule, its open outlines closed by a
    /// straight line.
    pub area: bool,
    /// The leaf is raster engraved (its pixels or its gradient as dots, `specs/0123`).
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

/// The frame of an image as every kind but Image sees it: one closed outline (Frage 271 A).
const FRAME: LeafShape = LeafShape::Outlines { closed: 1, open: 0 };

/// What `kind` runs of a leaf of shape `shape`.
#[must_use]
pub fn kind_run(kind: OperationKind, shape: LeafShape) -> KindRun {
    let nothing = KindRun::NOTHING;
    let (closed, open) = match (kind, shape) {
        (OperationKind::Image, LeafShape::Image) => {
            return KindRun {
                raster: true,
                ..nothing
            };
        }
        (OperationKind::Image, LeafShape::Gradient { closed, open }) => {
            return KindRun {
                raster: closed + open > 0,
                ..nothing
            };
        }
        (OperationKind::Image, LeafShape::Outlines { .. }) => return nothing,
        (_, LeafShape::Image) => return kind_run(kind, FRAME),
        (_, LeafShape::Outlines { closed, open } | LeafShape::Gradient { closed, open }) => {
            (closed, open)
        }
    };
    match kind {
        OperationKind::Image => nothing,
        OperationKind::Cut | OperationKind::Score => KindRun {
            lines: if closed + open == 0 {
                Lines::None
            } else {
                Lines::All
            },
            ..nothing
        },
        OperationKind::Engrave | OperationKind::EngraveOutline => match (closed, open) {
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
        OperationKind::CutOut if closed + open == 0 => nothing,
        OperationKind::CutOut => KindRun {
            contour: true,
            ..nothing
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use OperationKind::{Cut, CutOut, Engrave, EngraveOutline, Image, Score};

    const CLOSED: LeafShape = LeafShape::Outlines { closed: 1, open: 0 };
    const OPEN: LeafShape = LeafShape::Outlines { closed: 0, open: 2 };
    const MIXED: LeafShape = LeafShape::Outlines { closed: 1, open: 1 };
    const VECTOR: [OperationKind; 5] = [Cut, Score, Engrave, EngraveOutline, CutOut];

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
    }

    #[test]
    fn every_vector_kind_runs_an_image_as_one_closed_outline() {
        for kind in VECTOR {
            assert_eq!(
                kind_run(kind, LeafShape::Image),
                kind_run(kind, CLOSED),
                "{kind:?}: Frage 271 A, the frame rectangle"
            );
            assert!(!kind_run(kind, LeafShape::Image).raster, "{kind:?}");
        }
        assert_eq!(kind_run(Score, LeafShape::Image).lines, Lines::All);
        assert!(kind_run(Engrave, LeafShape::Image).area, "hatched");
        let traced = kind_run(EngraveOutline, LeafShape::Image);
        assert!(traced.area && traced.lines == Lines::Closed);
    }

    #[test]
    fn every_vector_kind_runs_a_gradient_shape_as_its_outlines() {
        for kind in VECTOR {
            for (closed, open) in [(1, 0), (0, 2), (1, 1), (0, 0)] {
                assert_eq!(
                    kind_run(kind, LeafShape::Gradient { closed, open }),
                    kind_run(kind, LeafShape::Outlines { closed, open }),
                    "{kind:?} {closed} {open}"
                );
            }
        }
    }

    #[test]
    fn image_rasters_images_and_gradient_shapes_and_runs_nothing_else() {
        assert_eq!(
            kind_run(Image, LeafShape::Image),
            KindRun {
                lines: Lines::None,
                area: false,
                raster: true,
                contour: false
            }
        );
        for (closed, open) in [(1, 0), (0, 1), (2, 1)] {
            let run = kind_run(Image, LeafShape::Gradient { closed, open });
            assert!(run.raster && run.lines == Lines::None && !run.area);
        }
        assert!(kind_run(Image, LeafShape::Gradient { closed: 0, open: 0 }).is_empty());
        for shape in [CLOSED, OPEN, MIXED] {
            assert!(kind_run(Image, shape).is_empty(), "criterion 48: {shape:?}");
        }
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
    }

    #[test]
    fn engrave_outline_adds_the_closed_outlines() {
        let run = kind_run(EngraveOutline, MIXED);
        assert!(run.area);
        assert_eq!(run.lines, Lines::Closed, "criterion 22");
        assert_eq!(kind_run(EngraveOutline, OPEN).lines, Lines::All);
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
