//! The triangles of the laser fixtures on the canvas (`specs/0102-laser-fixtures` UX notes §5 to
//! §7): the caller passes geometry in document mm, this module turns it into ink-only triangles.
//!
//! The fixtures share one artwork layer with the page grid, and a layer keeps the first fragment
//! of a pixel. So each builder pushes the topmost part first (an outline before its casing,
//! before its fill), and the kinds come in reverse draw order (stops, clamps, boards, tapes,
//! major lines, minor lines).

use curvyo_document_core::{Point, ViewTransform};

use crate::color::RgbaColor;
use crate::glyphs::{DrawList, Vertex, dashed_line, thick_line};
use crate::theme;

/// Fixture ink, `--toolbar-icon`.
const INK: RgbaColor = theme::GRID_INK;
/// Minor bed grid lines, `--grid-minor` (ink at 20 %).
const MINOR: RgbaColor = INK.with_alpha(51);
/// Major bed grid lines, `--lane-line`.
const MAJOR: RgbaColor = RgbaColor::opaque(0x7B, 0x7B, 0x7F);
/// The tape ground, `--ruler-bg`.
const RULER_BG: RgbaColor = RgbaColor::opaque(0xDC, 0xDC, 0xE0);
/// A short tape tick, `--ruler-tick-minor`.
const RULER_TICK_MINOR: RgbaColor = RgbaColor::opaque(0x7B, 0x7B, 0x7F);
/// The tape's inner edge, `--ruler-edge`.
const RULER_EDGE: RgbaColor = RgbaColor::opaque(0xB4, 0xB4, 0xB8);
/// A holder's board, `--toolbar-bg`.
const BOARD: RgbaColor = RgbaColor::opaque(0xDC, 0xDC, 0xE0);
/// Outlines of boards and clamps, in screen px.
const OUTLINE_PX: f64 = 1.5;
/// Slot outlines, hatch lines and tape ticks, in screen px.
const THIN_PX: f64 = 1.0;
/// The gap between two clamp hatch lines, in screen px.
const HATCH_GAP_PX: f64 = 6.0;
/// At most this many hatch lines per clamp; a denser hatch is spread out.
const MAX_HATCH_LINES: f64 = 2_000.0;
/// The slot cue's outline and a selected slot's outline, in screen px.
const CUE_PX: f64 = 2.0;
/// A stop bar is never thinner than this, in screen px.
const MIN_STOP_PX: f64 = 2.0;
/// Segments of a circle slot.
const CIRCLE_SEGMENTS: usize = 48;
/// The dash of an outside slot and of the selection box, in screen px.
const DASH_PX: (f64, f64) = (4.0, 3.0);

/// One tick of a tape: from the edge into the band; long every 10 mm.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TickLook {
    /// The end on the edge.
    pub start: Point,
    /// The end inside the band.
    pub end: Point,
    /// A long tick, drawn in the stronger tick ink.
    pub long: bool,
}

/// One tape along an edge of the work area.
#[derive(Debug, Clone, PartialEq)]
pub struct TapeLook {
    /// The band's four corners.
    pub band: [Point; 4],
    /// The band's inner edge.
    pub inner_edge: (Point, Point),
    /// The ticks to draw (the caller drops those too dense to see).
    pub ticks: Vec<TickLook>,
}

/// The engraved grid: its minor and major lines and its tapes.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct GridLook {
    /// Lines 1 device pixel wide.
    pub minor: Vec<(Point, Point)>,
    /// Lines 2 device pixels wide.
    pub major: Vec<(Point, Point)>,
    /// The minor lines' fade, `0..=1` (UX notes §5.1).
    pub minor_alpha: f32,
    /// The tapes that are on.
    pub tapes: Vec<TapeLook>,
}

/// The outline of a slot.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SlotOutline {
    /// A rectangle by its four corners.
    Rect([Point; 4]),
    /// A circle by its centre and diameter in mm.
    Circle {
        /// The centre.
        centre: Point,
        /// The diameter.
        diameter_mm: f64,
    },
}

impl SlotOutline {
    /// The outline as a closed polygon.
    fn polygon(&self) -> Vec<Point> {
        match *self {
            Self::Rect(corners) => corners.to_vec(),
            Self::Circle {
                centre,
                diameter_mm,
            } => {
                let r = diameter_mm / 2.0;
                (0..CIRCLE_SEGMENTS)
                    .map(|k| {
                        #[allow(clippy::cast_precision_loss)] // k < 48
                        let a = std::f64::consts::TAU * k as f64 / CIRCLE_SEGMENTS as f64;
                        Point::new(centre.x + r * a.cos(), centre.y + r * a.sin())
                    })
                    .collect()
            }
        }
    }
}

/// A slot of a board: its outline and whether it lies outside the board (drawn dashed).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SlotLook {
    /// The outline.
    pub outline: SlotOutline,
    /// Whether it lies past the board's edge.
    pub outside: bool,
}

/// A holder's board and its slots.
#[derive(Debug, Clone, PartialEq)]
pub struct BoardLook {
    /// The board's corners, in order around it.
    pub corners: [Point; 4],
    /// Its readable slots.
    pub slots: Vec<SlotLook>,
    /// Whether a running fixture job engraves the board: its ground is the canvas ground, so the
    /// hatch over it keeps its contrast (`specs/0103-fixture-jobs` UX notes J6).
    pub engraved: bool,
}

/// An L-stop as two filled legs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StopLook {
    /// The two legs, each by its corners in order.
    pub legs: [[Point; 4]; 2],
}

/// A screw: two concentric rings, no fill (`specs/0102-laser-fixtures` criterion 37, UX notes
/// §5.5).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScrewLook {
    /// The common centre.
    pub centre: Point,
    /// The head's diameter in mm.
    pub head_mm: f64,
    /// The thread's diameter in mm.
    pub thread_mm: f64,
}

/// Everything the drawn fixtures of one page show.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FixtureLook {
    /// The grid, at most one.
    pub grid: Option<GridLook>,
    /// The holders.
    pub boards: Vec<BoardLook>,
    /// The clamps by their corners (corner 0 to 1 is the width, 0 to 3 the height).
    pub clamps: Vec<[Point; 4]>,
    /// The stops.
    pub stops: Vec<StopLook>,
    /// The screws.
    pub screws: Vec<ScrewLook>,
}

/// The triangles of `look` in the reverse draw order of UX notes §5.5.
#[must_use]
pub fn build_fixture_look(look: &FixtureLook, view: ViewTransform, dpr: f64) -> DrawList {
    let px = 1.0 / view.scale();
    let mut list = DrawList::default();
    for screw in &look.screws {
        screw_triangles(&mut list, screw, px, dpr);
    }
    for stop in &look.stops {
        stop_triangles(&mut list, stop, px);
    }
    for clamp in &look.clamps {
        clamp_triangles(&mut list, *clamp, px);
    }
    for board in &look.boards {
        board_triangles(&mut list, board, px);
    }
    if let Some(grid) = &look.grid {
        grid_triangles(&mut list, grid, px, dpr);
    }
    list
}

/// The selected fixture's box (the accent, dashed); its handles are the rectangle's
/// (`build_transform_handles`).
#[must_use]
pub fn build_fixture_box(corners: [Point; 4], view: ViewTransform) -> DrawList {
    let px = 1.0 / view.scale();
    let mut list = DrawList::default();
    let dash = (DASH_PX.0 * px, DASH_PX.1 * px);
    for i in 0..4 {
        let (a, b) = (corners[i], corners[(i + 1) % 4]);
        list.extend(dashed_line(a, b, 3.0 * px, dash, theme::SELECTION_CASING));
        list.extend(dashed_line(a, b, px, dash, theme::ACCENT));
    }
    list
}

/// The outline a fixture had when its drag began, as the rectangle's drag ghost: a 1.5 px line in
/// `--drag-ghost` over its casing (`specs/0102-laser-fixtures` UX notes §7.1).
#[must_use]
pub fn build_fixture_ghost(segments: &[(Point, Point)], view: ViewTransform) -> DrawList {
    let px = 1.0 / view.scale();
    let width = theme::LIVE_PREVIEW_STROKE_PX * px;
    let mut list = DrawList::default();
    for &(a, b) in segments {
        list.triangles
            .extend(thick_line(a, b, width, theme::DRAG_GHOST).triangles);
    }
    for &(a, b) in segments {
        list.triangles
            .extend(thick_line(a, b, width + 2.0 * px, theme::SELECTION_CASING).triangles);
    }
    list
}

/// A slot lit as the fit target (`--accent-hover` fill, a 2 px accent outline over its casing),
/// or with `fill` false a selected slot's outline only.
#[must_use]
pub fn build_slot_cue(outline: &SlotOutline, fill: bool, view: ViewTransform) -> DrawList {
    let px = 1.0 / view.scale();
    let polygon = outline.polygon();
    let mut list = DrawList::default();
    if fill {
        fan(&mut list, &polygon, theme::ACCENT_HOVER);
    }
    ring_of(
        &mut list,
        &polygon,
        CUE_PX * 3.0 * px,
        theme::SELECTION_CASING,
    );
    ring_of(&mut list, &polygon, CUE_PX * px, theme::ACCENT);
    list
}

fn grid_triangles(list: &mut DrawList, grid: &GridLook, px: f64, dpr: f64) {
    let device = px / if dpr > 0.0 { dpr } else { 1.0 };
    for tape in &grid.tapes {
        let edge = thick_line(tape.inner_edge.0, tape.inner_edge.1, px, RULER_EDGE);
        list.triangles.extend(edge.triangles);
        for tick in &tape.ticks {
            let colour = if tick.long { INK } else { RULER_TICK_MINOR };
            list.triangles
                .extend(thick_line(tick.start, tick.end, THIN_PX * px, colour).triangles);
        }
        fan(list, &tape.band, RULER_BG);
    }
    for &(a, b) in &grid.major {
        list.triangles
            .extend(thick_line(a, b, 2.0 * device, MAJOR).triangles);
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // 0..=51 by the clamp.
    let alpha = (f32::from(MINOR.a) * grid.minor_alpha.clamp(0.0, 1.0)).round() as u8;
    if alpha == 0 {
        return;
    }
    for &(a, b) in &grid.minor {
        list.triangles
            .extend(thick_line(a, b, device, MINOR.with_alpha(alpha)).triangles);
    }
}

fn board_triangles(list: &mut DrawList, board: &BoardLook, px: f64) {
    for slot in &board.slots {
        let polygon = slot.outline.polygon();
        if slot.outside {
            let dash = (DASH_PX.0 * px, DASH_PX.1 * px);
            for i in 0..polygon.len() {
                let (a, b) = (polygon[i], polygon[(i + 1) % polygon.len()]);
                list.triangles
                    .extend(dashed_line(a, b, THIN_PX * px, dash, INK).triangles);
            }
        } else {
            ring_of(list, &polygon, THIN_PX * px, INK);
        }
        fan(list, &polygon, theme::CANVAS_BG);
    }
    ring_of(list, &board.corners, OUTLINE_PX * px, INK);
    ring_of(
        list,
        &board.corners,
        OUTLINE_PX * 3.0 * px,
        theme::SELECTION_CASING,
    );
    let ground = if board.engraved {
        theme::CANVAS_BG
    } else {
        BOARD
    };
    fan(list, &board.corners, ground);
}

fn clamp_triangles(list: &mut DrawList, corners: [Point; 4], px: f64) {
    ring_of(list, &corners, OUTLINE_PX * px, INK);
    ring_of(
        list,
        &corners,
        OUTLINE_PX * 3.0 * px,
        theme::SELECTION_CASING,
    );
    for (a, b) in hatch(corners, HATCH_GAP_PX * px) {
        list.triangles
            .extend(thick_line(a, b, THIN_PX * px, INK).triangles);
    }
}

/// The 45-degree hatch of a rectangle, clipped to it: lines `s + t = c` in its own frame.
fn hatch(corners: [Point; 4], gap_mm: f64) -> Vec<(Point, Point)> {
    let along = corners[0].vector_to(corners[1]);
    let across = corners[0].vector_to(corners[3]);
    let (width, height) = (along.length(), across.length());
    if !(width > 0.0 && height > 0.0 && gap_mm > 0.0) {
        return Vec::new();
    }
    let step = (gap_mm * std::f64::consts::SQRT_2).max((width + height) / MAX_HATCH_LINES);
    let (unit_along, unit_across) = (along.scaled(1.0 / width), across.scaled(1.0 / height));
    let at = |s: f64, t: f64| {
        corners[0]
            .translated(unit_along.scaled(s))
            .translated(unit_across.scaled(t))
    };
    let mut lines = Vec::new();
    let mut sum = step;
    while sum < width + height {
        let (first, last) = ((sum - height).max(0.0), sum.min(width));
        lines.push((at(first, sum - first), at(last, sum - last)));
        sum += step;
    }
    lines
}

/// A screw's thread is drawn from this many device px across (UX notes §5.5).
const MIN_THREAD_DEVICE_PX: f64 = 4.0;

fn screw_triangles(list: &mut DrawList, screw: &ScrewLook, px: f64, dpr: f64) {
    let circle = |diameter_mm: f64| {
        SlotOutline::Circle {
            centre: screw.centre,
            diameter_mm,
        }
        .polygon()
    };
    let dpr = if dpr.is_finite() && dpr > 0.0 {
        dpr
    } else {
        1.0
    };
    let thread = circle(screw.thread_mm);
    let thread_drawn = screw.thread_mm / px * dpr >= MIN_THREAD_DEVICE_PX;
    if thread_drawn {
        ring_of(list, &thread, THIN_PX * px, INK);
    }
    let head = circle(screw.head_mm);
    ring_of(list, &head, OUTLINE_PX * px, INK);
    if thread_drawn {
        ring_of(list, &thread, THIN_PX * 3.0 * px, theme::SELECTION_CASING);
    }
    ring_of(list, &head, OUTLINE_PX * 3.0 * px, theme::SELECTION_CASING);
}

fn stop_triangles(list: &mut DrawList, stop: &StopLook, px: f64) {
    for leg in &stop.legs {
        let leg = widened(*leg, MIN_STOP_PX * px);
        fan(list, &leg, INK);
    }
    for leg in &stop.legs {
        ring_of(list, leg, 2.0 * px, theme::SELECTION_CASING);
    }
}

/// A leg whose short side is below `min_mm` widened to it, about its long axis.
fn widened(leg: [Point; 4], min_mm: f64) -> [Point; 4] {
    let across = leg[1].vector_to(leg[2]);
    let thickness = across.length();
    if thickness >= min_mm || thickness <= 0.0 {
        return leg;
    }
    let grow = across.scaled((min_mm - thickness) / (2.0 * thickness));
    [
        leg[0].translated(grow.negated()),
        leg[1].translated(grow.negated()),
        leg[2].translated(grow),
        leg[3].translated(grow),
    ]
}

/// A convex polygon filled as a triangle fan.
fn fan(list: &mut DrawList, polygon: &[Point], color: RgbaColor) {
    for i in 1..polygon.len().saturating_sub(1) {
        for p in [polygon[0], polygon[i], polygon[i + 1]] {
            list.triangles.push(Vertex { position: p, color });
        }
    }
}

/// A closed outline `width_mm` wide.
fn ring_of(list: &mut DrawList, polygon: &[Point], width_mm: f64, color: RgbaColor) {
    for i in 0..polygon.len() {
        let (a, b) = (polygon[i], polygon[(i + 1) % polygon.len()]);
        list.triangles
            .extend(thick_line(a, b, width_mm, color).triangles);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view() -> ViewTransform {
        ViewTransform::identity()
    }

    fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> [Point; 4] {
        [
            Point::new(x0, y0),
            Point::new(x1, y0),
            Point::new(x1, y1),
            Point::new(x0, y1),
        ]
    }

    #[test]
    fn a_clamp_hatch_stays_inside_its_rectangle() {
        let lines = hatch(rect(10.0, 10.0, 50.0, 30.0), 3.0);
        assert_eq!(lines.len(), 14);
        for (a, b) in lines {
            for p in [a, b] {
                assert!((10.0 - 1e-9..=50.0 + 1e-9).contains(&p.x), "{p:?}");
                assert!((10.0 - 1e-9..=30.0 + 1e-9).contains(&p.y), "{p:?}");
            }
        }
    }

    #[test]
    fn faded_minor_lines_draw_nothing_and_major_lines_stay() {
        let grid = GridLook {
            minor: vec![(Point::new(0.0, 0.0), Point::new(0.0, 10.0))],
            major: vec![(Point::new(5.0, 0.0), Point::new(5.0, 10.0))],
            minor_alpha: 0.0,
            tapes: Vec::new(),
        };
        let look = FixtureLook {
            grid: Some(grid),
            ..FixtureLook::default()
        };
        assert_eq!(build_fixture_look(&look, view(), 1.0).triangles.len(), 6);
    }

    #[test]
    fn a_board_draws_its_outline_before_its_casing_and_fill() {
        let look = FixtureLook {
            boards: vec![BoardLook {
                corners: rect(0.0, 0.0, 100.0, 60.0),
                slots: Vec::new(),
                engraved: false,
            }],
            ..FixtureLook::default()
        };
        let list = build_fixture_look(&look, view(), 1.0);
        assert_eq!(list.triangles.first().unwrap().color, INK);
        assert_eq!(list.triangles.last().unwrap().color, BOARD);
    }

    #[test]
    fn an_engraved_board_takes_the_canvas_ground() {
        let look = FixtureLook {
            boards: vec![BoardLook {
                corners: rect(0.0, 0.0, 100.0, 60.0),
                slots: Vec::new(),
                engraved: true,
            }],
            ..FixtureLook::default()
        };
        let list = build_fixture_look(&look, view(), 1.0);
        assert_eq!(list.triangles.first().unwrap().color, INK);
        assert_eq!(list.triangles.last().unwrap().color, theme::CANVAS_BG);
    }

    #[test]
    fn a_screw_draws_two_ink_rings_with_no_fill_and_drops_a_tiny_thread() {
        let screw = ScrewLook {
            centre: Point::new(50.0, 50.0),
            head_mm: 10.0,
            thread_mm: 6.0,
        };
        let look = FixtureLook {
            screws: vec![screw],
            ..FixtureLook::default()
        };
        let list = build_fixture_look(&look, view(), 1.0);
        assert_eq!(list.triangles.first().unwrap().color, INK);
        assert!(
            list.triangles
                .iter()
                .all(|v| v.color == INK || v.color == theme::SELECTION_CASING)
        );
        let tiny = FixtureLook {
            screws: vec![ScrewLook {
                thread_mm: 2.0,
                ..screw
            }],
            ..FixtureLook::default()
        };
        let both = list.triangles.len();
        assert_eq!(
            build_fixture_look(&tiny, view(), 1.0).triangles.len(),
            both / 2
        );
    }

    #[test]
    fn a_thin_stop_leg_is_widened_to_two_pixels() {
        let leg = widened(rect(0.0, 0.0, 60.0, 0.5), 2.0);
        assert!((leg[1].vector_to(leg[2]).length() - 2.0).abs() < 1e-9);
    }
}
