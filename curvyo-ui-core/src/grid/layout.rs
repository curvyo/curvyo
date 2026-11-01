//! The drawn lines of one page grid for one view, clipped to the page and the visible canvas.

use curvyo_document_core::{Page, Point, ViewTransform};

use super::{GridFrame, axis_lines};

/// One grid line, already clipped to the page rectangle and the visible rectangle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GridLine {
    /// One end, document mm.
    pub start: Point,
    /// The other end, document mm.
    pub end: Point,
    /// `0` minor to `1` major: the renderer blends the minor and major colours by it.
    pub emphasis: f32,
    /// Opacity, `0..=1`: the renderer multiplies it into the colour.
    pub alpha: f32,
}

/// What [`grid_layout`] returns: the lines to draw, bounded by the screen size.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct GridLayout {
    /// Every line with a non-zero opacity.
    pub lines: Vec<GridLine>,
}

/// The grid of `page` as seen in `view`, where `visible` is the canvas rectangle in document mm
/// (criteria 4 to 8). A hidden grid, or a page outside the canvas, gives an empty layout. The
/// cost depends only on the visible area: only the index range that intersects page ∩ visible is
/// walked, and every emitted line is at least 4 px from its neighbours.
#[must_use]
pub fn grid_layout(page: &Page, view: ViewTransform, visible: (Point, Point)) -> GridLayout {
    if !page.grid.shown {
        return GridLayout::default();
    }
    let page_max = Point::new(
        page.origin.x + page.size.width.as_mm(),
        page.origin.y + page.size.height.as_mm(),
    );
    let min = Point::new(
        page.origin.x.max(visible.0.x),
        page.origin.y.max(visible.0.y),
    );
    let max = Point::new(page_max.x.min(visible.1.x), page_max.y.min(visible.1.y));
    if min.x >= max.x || min.y >= max.y {
        return GridLayout::default();
    }
    let frame = GridFrame::of(page);
    let corners = [
        frame.to_grid(min),
        frame.to_grid(Point::new(max.x, min.y)),
        frame.to_grid(max),
        frame.to_grid(Point::new(min.x, max.y)),
    ];
    let (u_lo, u_hi) = extent(corners.iter().map(|c| c.0));
    let (v_lo, v_hi) = extent(corners.iter().map(|c| c.1));
    let scale = view.scale();
    let mut lines = Vec::new();
    for line in axis_lines(&page.grid, scale, u_lo, u_hi) {
        let a = frame.to_document(line.pos, v_lo);
        let b = frame.to_document(line.pos, v_hi);
        push_clipped(&mut lines, a, b, (min, max), line.emphasis, line.alpha);
    }
    for line in axis_lines(&page.grid, scale, v_lo, v_hi) {
        let a = frame.to_document(u_lo, line.pos);
        let b = frame.to_document(u_hi, line.pos);
        push_clipped(&mut lines, a, b, (min, max), line.emphasis, line.alpha);
    }
    GridLayout { lines }
}

fn extent(values: impl Iterator<Item = f64>) -> (f64, f64) {
    values.fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), v| {
        (lo.min(v), hi.max(v))
    })
}

fn push_clipped(
    lines: &mut Vec<GridLine>,
    a: Point,
    b: Point,
    rect: (Point, Point),
    emphasis: f32,
    alpha: f32,
) {
    if let Some((start, end)) = clip(a, b, rect) {
        lines.push(GridLine {
            start,
            end,
            emphasis,
            alpha,
        });
    }
}

/// Liang-Barsky: the part of segment `a..b` inside the axis-aligned `rect`, if any.
pub(super) fn clip(a: Point, b: Point, rect: (Point, Point)) -> Option<(Point, Point)> {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let mut t0: f64 = 0.0;
    let mut t1: f64 = 1.0;
    for (p, q) in [
        (-dx, a.x - rect.0.x),
        (dx, rect.1.x - a.x),
        (-dy, a.y - rect.0.y),
        (dy, rect.1.y - a.y),
    ] {
        if p == 0.0 {
            if q < 0.0 {
                return None;
            }
        } else {
            let t = q / p;
            if p < 0.0 {
                t0 = t0.max(t);
            } else {
                t1 = t1.min(t);
            }
        }
    }
    if t0 > t1 {
        return None;
    }
    let at = |t: f64| Point::new(a.x + dx * t, a.y + dy * t);
    // Keep an axis-aligned line's own coordinate exact: the interpolation would round it.
    let (mut start, mut end) = (at(t0), at(t1));
    if dx == 0.0 {
        start.x = a.x;
        end.x = a.x;
    }
    if dy == 0.0 {
        start.y = a.y;
        end.y = a.y;
    }
    Some((start, end))
}
