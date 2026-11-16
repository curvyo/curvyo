//! The lines of the centre marks at the centres of a pin or screw grid
//! (`specs/0113-fixture-sets-and-pins` criterion 25; ADR 0059 §5).

use curvyo_document_core::{Length, Point};

/// A circle on the page.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Circle {
    /// Its centre.
    pub centre: Point,
    /// Its diameter.
    pub diameter: Length,
}

/// A straight line on the page.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Line {
    /// Where it starts.
    pub from: Point,
    /// Where it ends.
    pub to: Point,
}

/// The lines of one part of a pin or screw grid, every centre in one batch.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MarkLines {
    /// The circles.
    pub circles: Vec<Circle>,
    /// The straight lines.
    pub lines: Vec<Line>,
}

/// The Centre mark at every centre: a circle of `size` as its diameter and two cross lines, one
/// along x and one along y, spanning that circle.
#[must_use]
pub fn centre_marks(centres: &[Point], size: Length) -> MarkLines {
    let half = size.as_mm() / 2.0;
    MarkLines {
        circles: centres
            .iter()
            .map(|&centre| Circle {
                centre,
                diameter: size,
            })
            .collect(),
        lines: centres
            .iter()
            .flat_map(|c| {
                [
                    Line {
                        from: Point::new(c.x - half, c.y),
                        to: Point::new(c.x + half, c.y),
                    },
                    Line {
                        from: Point::new(c.x, c.y - half),
                        to: Point::new(c.x, c.y + half),
                    },
                ]
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture_grid::{GridCounts, centres};

    #[test]
    fn a_centre_mark_is_a_circle_of_its_size_and_a_cross_spanning_it() {
        let marks = centre_marks(&[Point::new(10.0, 20.0)], Length::from_mm(4.0));
        assert_eq!(
            marks.circles,
            [Circle {
                centre: Point::new(10.0, 20.0),
                diameter: Length::from_mm(4.0)
            }]
        );
        assert_eq!(
            marks.lines,
            [
                Line {
                    from: Point::new(8.0, 20.0),
                    to: Point::new(12.0, 20.0)
                },
                Line {
                    from: Point::new(10.0, 18.0),
                    to: Point::new(10.0, 22.0)
                },
            ]
        );
    }

    #[test]
    fn a_grid_of_four_hundred_centres_yields_four_hundred_of_each_in_one_batch() {
        let mm = Length::from_mm;
        let grid = centres(
            Point::new(0.0, 0.0),
            GridCounts::new(50, 8).unwrap(),
            mm(20.0),
            mm(20.0),
        );
        let marks = centre_marks(&grid, mm(4.0));
        assert_eq!((marks.circles.len(), marks.lines.len()), (400, 800));
    }
}
