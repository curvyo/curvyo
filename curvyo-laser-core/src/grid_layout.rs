//! The engraved grid's lines, major lines and tapes over the work area, counted from the origin
//! corner of the page's profile copy plus the grid's offset (`specs/0102-laser-fixtures` criteria
//! 15 to 17 and 27 a; Frage 174 A). Page coordinates have y growing downwards, as on the canvas.

use curvyo_document_core::{DocumentSize, Length, Point};
use curvyo_extension_core::LineSegment;

use crate::machine_profile::OriginCorner;

/// The width of a tape's band along its edge, in mm (criterion 17).
pub const TAPE_BAND_MM: f64 = 5.0;
/// The distance between two tape ticks, in mm (criterion 17).
pub const TICK_STEP_MM: i64 = 5;
/// [`TICK_STEP_MM`] as a length in the layout's f64 arithmetic.
const TICK_STEP_F: f64 = 5.0;
/// The length of a short tick, in mm (Proposal).
pub const SHORT_TICK_MM: f64 = 2.0;
/// The length of a long tick, in mm, every second tick (Proposal).
pub const LONG_TICK_MM: f64 = 4.0;
/// The distance between two numbers of a tape, in mm (criterion 17).
pub const NUMBER_STEP_MM: i64 = 50;
/// The smallest spacing a layout is drawn at; a stored spacing below it reads as this.
pub const MIN_SPACING_MM: f64 = 0.5;

/// An edge of the work area a tape runs along.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TapeEdge {
    /// The bottom edge (page y at the height).
    Bottom,
    /// The top edge.
    Top,
    /// The left edge.
    Left,
    /// The right edge.
    Right,
}

/// The tape switches of the four edges (criterion 15).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[allow(clippy::struct_excessive_bools)] // one switch per edge, as the spec names them
pub struct TapeEdges {
    /// The bottom edge has a tape.
    pub bottom: bool,
    /// The top edge has a tape.
    pub top: bool,
    /// The left edge has a tape.
    pub left: bool,
    /// The right edge has a tape.
    pub right: bool,
}

impl TapeEdges {
    /// All four tapes on: the default switches (Frage 174 C, answered 2026-11-10).
    #[must_use]
    pub const fn all() -> Self {
        Self {
            bottom: true,
            top: true,
            left: true,
            right: true,
        }
    }

    /// Whether `edge` has a tape.
    #[must_use]
    pub fn has(self, edge: TapeEdge) -> bool {
        match edge {
            TapeEdge::Bottom => self.bottom,
            TapeEdge::Top => self.top,
            TapeEdge::Left => self.left,
            TapeEdge::Right => self.right,
        }
    }
}

/// The parameters of an engraved grid that the layout needs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GridSpec {
    /// The distance between two lines.
    pub spacing: Length,
    /// Every n-th line from the origin is major (2 to 20 in the spec; a smaller value reads as 1).
    pub major_every: u32,
    /// The tapes that are on.
    pub tapes: TapeEdges,
    /// The origin corner of the page's profile copy.
    pub corner: OriginCorner,
    /// The grid's origin offset from the corner, along X.
    pub origin_x: Length,
    /// The grid's origin offset from the corner, along Y.
    pub origin_y: Length,
}

/// A tick of a tape: a short or long mark from the edge into the work area.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tick {
    /// The end on the edge.
    pub start: Point,
    /// The end inside the work area.
    pub end: Point,
    /// True for every second tick (every 10 mm).
    pub long: bool,
}

/// A number of a tape: millimetres counted from the grid origin along the edge.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Number {
    /// Where the number sits, at the inner side of the band.
    pub at: Point,
    /// The value in millimetres (negative before the origin).
    pub mm: i64,
}

/// One tape: the ticks and the numbers along one edge.
#[derive(Debug, Clone, PartialEq)]
pub struct Tape {
    /// The edge the tape runs along.
    pub edge: TapeEdge,
    /// The ticks, one every 5 mm.
    pub ticks: Vec<Tick>,
    /// The numbers, one every 50 mm.
    pub numbers: Vec<Number>,
}

/// Everything the grid draws: minor lines, major lines (drawn thicker) and the tapes.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct GridLayout {
    /// The lines that are not major, edges included.
    pub minor: Vec<LineSegment>,
    /// Every n-th line from the origin.
    pub major: Vec<LineSegment>,
    /// The tapes that are on.
    pub tapes: Vec<Tape>,
}

/// One axis of the work area, counted from the origin corner: `far` when the corner lies at the
/// large end of the axis, `origin` the grid origin's offset from the corner in mm.
#[derive(Debug, Clone, Copy)]
struct Axis {
    extent: f64,
    far: bool,
    origin: f64,
}

impl Axis {
    /// The page coordinate of the corner-relative distance `u`.
    fn page(self, u: f64) -> f64 {
        if self.far { self.extent - u } else { u }
    }

    /// The corner-relative distance of the grid line `k` at `spacing`.
    fn distance(self, k: i64, spacing: f64) -> f64 {
        #[allow(clippy::cast_precision_loss)] // k is bounded by the work area over the spacing
        let steps = k as f64;
        self.origin + steps * spacing
    }

    /// The indices `k` whose lines `origin + k · spacing` lie within `0..=extent`.
    fn indices(self, spacing: f64) -> std::ops::RangeInclusive<i64> {
        const EDGE: f64 = 1e-9;
        #[allow(clippy::cast_possible_truncation)] // bounded: the work area over at least 0.5 mm
        let first = ((0.0 - self.origin) / spacing - EDGE).ceil() as i64;
        #[allow(clippy::cast_possible_truncation)]
        let last = ((self.extent - self.origin) / spacing + EDGE).floor() as i64;
        first..=last
    }
}

/// The layout of the grid over `work_area` (criteria 15 to 17). Zero or negative sizes are read as
/// the smallest spacing and a major every 1 line, so the layout never loops without end.
#[must_use]
pub fn layout(spec: GridSpec, work_area: DocumentSize) -> GridLayout {
    let spacing = spec.spacing.as_mm().max(MIN_SPACING_MM);
    let major = i64::from(spec.major_every.max(1));
    let (width, height) = (work_area.width.as_mm(), work_area.height.as_mm());
    let x = axis(spec.corner, true, width, spec.origin_x);
    let y = axis(spec.corner, false, height, spec.origin_y);
    let mut out = GridLayout::default();
    for k in x.indices(spacing) {
        let at = x.page(x.distance(k, spacing));
        let line = LineSegment {
            start: Point::new(at, 0.0),
            end: Point::new(at, height),
        };
        push_line(&mut out, line, k, major);
    }
    for k in y.indices(spacing) {
        let at = y.page(y.distance(k, spacing));
        let line = LineSegment {
            start: Point::new(0.0, at),
            end: Point::new(width, at),
        };
        push_line(&mut out, line, k, major);
    }
    for edge in [
        TapeEdge::Bottom,
        TapeEdge::Top,
        TapeEdge::Left,
        TapeEdge::Right,
    ] {
        if spec.tapes.has(edge) {
            out.tapes.push(tape(edge, x, y, width, height));
        }
    }
    out
}

/// The axis of the work area along X (`horizontal`) or Y, from the origin corner.
fn axis(corner: OriginCorner, horizontal: bool, extent: f64, origin: Length) -> Axis {
    let far = if horizontal {
        is_right(corner)
    } else {
        !is_top(corner)
    };
    Axis {
        extent,
        far,
        origin: origin.as_mm(),
    }
}

fn push_line(out: &mut GridLayout, line: LineSegment, k: i64, major: i64) {
    if k.rem_euclid(major) == 0 {
        out.major.push(line);
    } else {
        out.minor.push(line);
    }
}

/// The ticks and numbers of the tape along `edge`: the ticks sit every 5 mm of the edge's axis
/// from the grid origin, every second one long, every tenth one carries its number.
fn tape(edge: TapeEdge, x: Axis, y: Axis, width: f64, height: f64) -> Tape {
    let along = match edge {
        TapeEdge::Bottom | TapeEdge::Top => x,
        TapeEdge::Left | TapeEdge::Right => y,
    };
    let mut tape = Tape {
        edge,
        ticks: Vec::new(),
        numbers: Vec::new(),
    };
    for j in along.indices(TICK_STEP_F) {
        let at = along.page(along.distance(j, TICK_STEP_F));
        let mm = j * TICK_STEP_MM;
        let long = j.rem_euclid(2) == 0;
        let len = if long { LONG_TICK_MM } else { SHORT_TICK_MM };
        tape.ticks.push(Tick {
            start: edge_point(edge, at, 0.0, width, height),
            end: edge_point(edge, at, len, width, height),
            long,
        });
        if mm.rem_euclid(NUMBER_STEP_MM) == 0 {
            tape.numbers.push(Number {
                at: edge_point(edge, at, TAPE_BAND_MM, width, height),
                mm,
            });
        }
    }
    tape
}

/// The point at `along` on the edge's line, moved `inward` mm into the work area.
fn edge_point(edge: TapeEdge, along: f64, inward: f64, width: f64, height: f64) -> Point {
    match edge {
        TapeEdge::Bottom => Point::new(along, height - inward),
        TapeEdge::Top => Point::new(along, inward),
        TapeEdge::Left => Point::new(inward, along),
        TapeEdge::Right => Point::new(width - inward, along),
    }
}

/// The grid lattice of the snapping: its intersections in page coordinates (criterion 27 a).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Lattice {
    origin: Point,
    spacing: f64,
    width: f64,
    height: f64,
}

impl Lattice {
    /// The lattice of `spec` over `work_area`, in page coordinates.
    #[must_use]
    pub fn new(spec: GridSpec, work_area: DocumentSize) -> Self {
        let (width, height) = (work_area.width.as_mm(), work_area.height.as_mm());
        let x = axis(spec.corner, true, width, spec.origin_x);
        let y = axis(spec.corner, false, height, spec.origin_y);
        Self {
            origin: Point::new(x.page(x.origin), y.page(y.origin)),
            spacing: spec.spacing.as_mm().max(MIN_SPACING_MM),
            width,
            height,
        }
    }

    /// The intersection nearest to `p`, rounded per axis and kept inside the work area. A lattice
    /// with no line inside the area returns `p` unchanged.
    #[must_use]
    pub fn nearest(&self, p: Point) -> Point {
        Point::new(
            snap_axis(p.x, self.origin.x, self.spacing, self.width),
            snap_axis(p.y, self.origin.y, self.spacing, self.height),
        )
    }
}

fn snap_axis(value: f64, origin: f64, spacing: f64, extent: f64) -> f64 {
    const EDGE: f64 = 1e-9;
    let first = ((0.0 - origin) / spacing - EDGE).ceil();
    let last = ((extent - origin) / spacing + EDGE).floor();
    if first > last {
        return value;
    }
    let k = ((value - origin) / spacing).round().clamp(first, last);
    origin + k * spacing
}

const fn is_top(corner: OriginCorner) -> bool {
    matches!(corner, OriginCorner::TopLeft | OriginCorner::TopRight)
}

const fn is_right(corner: OriginCorner) -> bool {
    matches!(corner, OriginCorner::TopRight | OriginCorner::BottomRight)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(spacing: f64, major: u32, corner: OriginCorner) -> GridSpec {
        GridSpec {
            spacing: Length::from_mm(spacing),
            major_every: major,
            tapes: TapeEdges::all(),
            corner,
            origin_x: Length::from_mm(0.0),
            origin_y: Length::from_mm(0.0),
        }
    }

    fn area(w: f64, h: f64) -> DocumentSize {
        DocumentSize::from_mm(w, h)
    }

    fn vertical(lines: &[LineSegment]) -> Vec<f64> {
        lines
            .iter()
            .filter(|l| l.start.x == l.end.x)
            .map(|l| l.start.x)
            .collect()
    }

    #[test]
    fn a_600_by_400_area_has_61_and_41_lines_of_which_13_and_9_are_major() {
        let lay = layout(spec(10.0, 5, OriginCorner::BottomLeft), area(600.0, 400.0));
        assert_eq!(lay.minor.len() + lay.major.len(), 61 + 41);
        assert_eq!(vertical(&lay.major).len(), 13);
        let horizontal_major = lay.major.iter().filter(|l| l.start.y == l.end.y).count();
        assert_eq!(horizontal_major, 9);
    }

    #[test]
    fn a_600_mm_bottom_tape_has_121_ticks_61_long_and_13_numbers() {
        let lay = layout(spec(10.0, 5, OriginCorner::BottomLeft), area(600.0, 400.0));
        let bottom = lay
            .tapes
            .iter()
            .find(|t| t.edge == TapeEdge::Bottom)
            .unwrap();
        assert_eq!(bottom.ticks.len(), 121);
        assert_eq!(bottom.ticks.iter().filter(|t| t.long).count(), 61);
        assert_eq!(bottom.numbers.len(), 13);
        assert_eq!(bottom.numbers.first().map(|n| n.mm), Some(0));
        assert_eq!(bottom.numbers.last().map(|n| n.mm), Some(600));
    }

    #[test]
    fn all_four_tapes_are_on_by_default_and_each_can_be_switched_off() {
        let lay = layout(spec(10.0, 5, OriginCorner::TopRight), area(100.0, 100.0));
        let edges: Vec<TapeEdge> = lay.tapes.iter().map(|t| t.edge).collect();
        assert_eq!(
            edges,
            vec![
                TapeEdge::Bottom,
                TapeEdge::Top,
                TapeEdge::Left,
                TapeEdge::Right
            ]
        );
        let mut only_top = spec(10.0, 5, OriginCorner::TopRight);
        only_top.tapes = TapeEdges {
            top: true,
            ..TapeEdges::default()
        };
        let lay = layout(only_top, area(100.0, 100.0));
        assert_eq!(lay.tapes.len(), 1);
        assert_eq!(lay.tapes[0].edge, TapeEdge::Top);
    }

    #[test]
    fn a_right_corner_mirrors_the_lines_and_the_offset_counts_from_it() {
        let mut grid = spec(25.0, 5, OriginCorner::BottomRight);
        grid.origin_x = Length::from_mm(5.0);
        let lay = layout(grid, area(100.0, 50.0));
        let xs = vertical(
            &lay.minor
                .iter()
                .chain(&lay.major)
                .copied()
                .collect::<Vec<_>>(),
        );
        assert_eq!(xs.len(), 4);
        for expected in [20.0, 45.0, 70.0, 95.0] {
            assert!(xs.iter().any(|x| (x - expected).abs() < 1e-9), "{expected}");
        }
    }

    #[test]
    fn a_major_of_one_or_less_reads_as_every_line_major() {
        let lay = layout(spec(50.0, 0, OriginCorner::BottomLeft), area(100.0, 100.0));
        assert_eq!(lay.minor.len(), 0);
        assert_eq!(lay.major.len(), 3 + 3);
    }

    #[test]
    fn a_1000_mm_square_at_one_millimetre_has_2002_lines() {
        let lay = layout(spec(1.0, 5, OriginCorner::BottomLeft), area(1000.0, 1000.0));
        assert_eq!(lay.minor.len() + lay.major.len(), 2002);
    }

    #[test]
    fn the_lattice_rounds_each_axis_and_stays_in_the_area() {
        let lattice = Lattice::new(spec(10.0, 5, OriginCorner::TopLeft), area(100.0, 50.0));
        assert_eq!(
            lattice.nearest(Point::new(12.0, 27.0)),
            Point::new(10.0, 30.0)
        );
        assert_eq!(
            lattice.nearest(Point::new(-4.0, 60.0)),
            Point::new(0.0, 50.0)
        );
    }
}
