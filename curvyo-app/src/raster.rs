//! An anti-aliased fill of `kurbo` paths into a small opaque RGBA picture, for the import
//! dialog's SVG preview (`specs/0087-image-clipboard-and-drop` criterion 26).
//!
//! Scanline fill with four sample rows per pixel and exact horizontal span coverage, nonzero or
//! even-odd; every fill is blended over what is there. The work is bounded: a path is flattened
//! only where it can touch the picture, and a budget of work units (an edge costs one per sample
//! row it spans in the picture) stops the drawing once spent.

use kurbo::{BezPath, ParamCurve, PathEl, Point, Rect, Shape};

/// Sample rows per pixel row.
const SAMPLES: u32 = 4;
/// The flattening tolerance in pixels.
const TOLERANCE: f64 = 0.1;
/// The work units of one picture: an edge costs 1 plus the sample rows it spans inside the
/// picture, a fill the sample rows it sweeps, a stroke its input segments. Once spent, nothing
/// more is drawn (some 20 M units are well under a second).
pub(crate) const MAX_WORK: usize = 20_000_000;

/// A picture of `width` x `height` pixels, opaque, row-major RGBA8, top row first.
#[derive(Debug)]
pub(crate) struct Canvas {
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) pixels: Vec<u8>,
    /// Work units still allowed by [`MAX_WORK`].
    budget: usize,
}

/// One straight edge, top to bottom, with the direction it was drawn in.
#[derive(Debug, Clone, Copy)]
struct Edge {
    top: Point,
    bottom: Point,
    winding: i32,
}

impl Canvas {
    /// A picture of `width` x `height` pixels filled with `rgb`.
    pub(crate) fn new(width: u32, height: u32, rgb: [u8; 3]) -> Self {
        let count = width as usize * height as usize;
        let mut pixels = Vec::with_capacity(count * 4);
        for _ in 0..count {
            pixels.extend_from_slice(&[rgb[0], rgb[1], rgb[2], 255]);
        }
        Self {
            width,
            height,
            pixels,
            budget: MAX_WORK,
        }
    }

    /// The same picture with a budget of `budget` work units instead of [`MAX_WORK`].
    #[cfg(test)]
    pub(crate) fn with_budget(mut self, budget: usize) -> Self {
        self.budget = budget;
        self
    }

    /// Takes `units` from the budget; `false` (and the budget spent) when it does not hold them.
    pub(crate) fn charge(&mut self, units: usize) -> bool {
        if units > self.budget {
            self.budget = 0;
            return false;
        }
        self.budget -= units;
        true
    }

    /// Whether the edge budget is spent, so nothing more is drawn.
    pub(crate) fn exhausted(&self) -> bool {
        self.budget == 0
    }

    /// Fills `path` (pixel coordinates) with `rgb` at `alpha` (0 to 1), nonzero or `even_odd`.
    pub(crate) fn fill(&mut self, path: &BezPath, even_odd: bool, rgb: [u8; 3], alpha: f64) {
        if alpha <= 0.0 || self.exhausted() {
            return;
        }
        let Some(mut edges) = self.edges_of(path) else {
            self.budget = 0;
            return;
        };
        if edges.is_empty() {
            return;
        }
        edges.sort_by(|a, b| a.top.y.total_cmp(&b.top.y));
        let first = edges[0].top.y.max(0.0).floor();
        let last = edges
            .iter()
            .map(|e| e.bottom.y)
            .fold(f64::MIN, f64::max)
            .min(f64::from(self.height))
            .ceil();
        // invariant: both are clamped to 0..=height, which is a u32.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let (first, last) = (first as u32, last.max(first) as u32);
        if !self.charge((last - first) as usize * SAMPLES as usize) {
            return;
        }
        let mut coverage = vec![0.0_f64; self.width as usize];
        let mut active: Vec<Edge> = Vec::new();
        let mut next = 0;
        let mut crossings: Vec<(f64, i32)> = Vec::new();
        let weight = 1.0 / f64::from(SAMPLES);
        for row in first..last {
            coverage.fill(0.0);
            for sample in 0..SAMPLES {
                let y = f64::from(row) + (f64::from(sample) + 0.5) * weight;
                while next < edges.len() && edges[next].top.y <= y {
                    active.push(edges[next]);
                    next += 1;
                }
                active.retain(|e| e.bottom.y > y);
                crossings.clear();
                crossings.extend(active.iter().filter(|e| e.top.y <= y).map(|e| {
                    let t = (y - e.top.y) / (e.bottom.y - e.top.y);
                    (e.top.x + t * (e.bottom.x - e.top.x), e.winding)
                }));
                crossings.sort_by(|a, b| a.0.total_cmp(&b.0));
                let mut winding = 0;
                let mut from = 0.0;
                for &(x, direction) in &crossings {
                    let inside = if even_odd {
                        winding % 2 != 0
                    } else {
                        winding != 0
                    };
                    if inside {
                        add_span(&mut coverage, from, x, weight);
                    }
                    winding += direction;
                    from = x;
                }
            }
            self.blend_row(row, &coverage, rgb, alpha);
        }
    }

    /// The edges of `path` that can touch the picture: segments wholly above, below or right of
    /// it are dropped, those wholly left of it become their chord (same winding inside), the rest
    /// is flattened. Every edge is charged its cost (1 plus the sample rows it spans inside the
    /// picture); `None` when the budget runs out.
    fn edges_of(&mut self, path: &BezPath) -> Option<Vec<Edge>> {
        let (width, height) = (f64::from(self.width), f64::from(self.height));
        let mut edges = Vec::new();
        let mut left = self.budget;
        let mut push = |a: Point, b: Point, edges: &mut Vec<Edge>| -> bool {
            // Exact on purpose: only an edge of no height never crosses a sample row.
            #[allow(clippy::float_cmp)]
            if a.y == b.y {
                return true;
            }
            let (top, bottom, winding) = if a.y < b.y { (a, b, 1) } else { (b, a, -1) };
            let rows = (bottom.y.min(height) - top.y.max(0.0)).max(0.0) * f64::from(SAMPLES);
            // invariant: at most height x SAMPLES, a few thousand.
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let cost = 1 + rows.ceil() as usize;
            if cost > left {
                return false;
            }
            left -= cost;
            edges.push(Edge {
                top,
                bottom,
                winding,
            });
            true
        };
        for segment in path.segments() {
            let bounds: Rect = segment.bounding_box();
            if !bounds.is_finite() || bounds.y1 < 0.0 || bounds.y0 > height || bounds.x0 > width {
                continue;
            }
            let (start, end) = (segment.start(), segment.end());
            if bounds.x1 < 0.0 {
                if !push(start, end, &mut edges) {
                    return None;
                }
                continue;
            }
            let mut last = start;
            let mut ok = true;
            kurbo::flatten(
                [PathEl::MoveTo(start), segment.as_path_el()],
                TOLERANCE,
                |el| {
                    if let PathEl::LineTo(p) = el {
                        ok = ok && push(last, p, &mut edges);
                        last = p;
                    }
                },
            );
            if !ok {
                return None;
            }
        }
        self.budget = left;
        Some(edges)
    }

    fn blend_row(&mut self, row: u32, coverage: &[f64], rgb: [u8; 3], alpha: f64) {
        let start = row as usize * self.width as usize * 4;
        for (x, &cover) in coverage.iter().enumerate() {
            let a = cover.clamp(0.0, 1.0) * alpha;
            if a <= 0.0 {
                continue;
            }
            let at = start + x * 4;
            for (channel, &value) in self.pixels[at..at + 3].iter_mut().zip(&rgb) {
                let mixed = f64::from(*channel) + (f64::from(value) - f64::from(*channel)) * a;
                // invariant: a mix of two bytes with 0 <= a <= 1 is within 0..=255.
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                {
                    *channel = mixed.round() as u8;
                }
            }
        }
    }
}

/// Adds `weight` times the covered part of every pixel between `from` and `to` to `coverage`.
fn add_span(coverage: &mut [f64], from: f64, to: f64, weight: f64) {
    #[allow(clippy::cast_precision_loss)] // a preview is at most a few hundred pixels wide
    let width = coverage.len() as f64;
    let (from, to) = (from.clamp(0.0, width), to.clamp(0.0, width));
    if to <= from {
        return;
    }
    // invariant: both are clamped to 0..=width.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let (first, last) = (from.floor() as usize, to.floor() as usize);
    if first == last {
        coverage[first] += (to - from) * weight;
        return;
    }
    #[allow(clippy::cast_precision_loss)]
    let first_end = (first + 1) as f64;
    coverage[first] += (first_end - from) * weight;
    for cell in &mut coverage[first + 1..last] {
        *cell += weight;
    }
    if let Some(cell) = coverage.get_mut(last) {
        #[allow(clippy::cast_precision_loss)]
        let last_start = last as f64;
        *cell += (to - last_start) * weight;
    }
}

#[cfg(test)]
mod tests {
    use kurbo::Circle;

    use super::*;

    fn pixel(canvas: &Canvas, x: u32, y: u32) -> [u8; 4] {
        let at = (y * canvas.width + x) as usize * 4;
        canvas.pixels[at..at + 4].try_into().unwrap()
    }

    #[test]
    fn a_rectangle_fills_its_pixels_and_antialiases_its_edges() {
        let mut canvas = Canvas::new(10, 10, [255, 255, 255]);
        canvas.fill(
            &Rect::new(2.0, 2.0, 6.5, 8.0).to_path(0.1),
            false,
            [0, 0, 0],
            1.0,
        );
        assert_eq!(pixel(&canvas, 3, 3), [0, 0, 0, 255]);
        assert_eq!(pixel(&canvas, 8, 3), [255, 255, 255, 255]);
        assert_eq!(pixel(&canvas, 3, 9), [255, 255, 255, 255]);
        let half = pixel(&canvas, 6, 4)[0];
        assert!((126..=129).contains(&half), "half covered: {half}");
    }

    #[test]
    fn even_odd_leaves_the_hole_of_two_nested_circles() {
        let mut path = Circle::new((10.0, 10.0), 8.0).to_path(0.1);
        path.extend(Circle::new((10.0, 10.0), 4.0).to_path(0.1));
        let mut nonzero = Canvas::new(20, 20, [255, 255, 255]);
        nonzero.fill(&path, false, [0, 0, 0], 1.0);
        let mut even_odd = Canvas::new(20, 20, [255, 255, 255]);
        even_odd.fill(&path, true, [0, 0, 0], 1.0);
        assert_eq!(pixel(&nonzero, 10, 10)[0], 0);
        assert_eq!(pixel(&even_odd, 10, 10)[0], 255);
        assert_eq!(pixel(&even_odd, 10, 4)[0], 0, "the ring is filled");
    }

    #[test]
    fn a_shape_far_outside_costs_no_flattening_and_one_left_of_it_still_counts() {
        let mut canvas = Canvas::new(10, 10, [255, 255, 255]);
        canvas.fill(
            &Circle::new((1e9, 1e9), 1e8).to_path(0.1),
            false,
            [0, 0, 0],
            1.0,
        );
        assert_eq!(canvas.budget, MAX_WORK, "nothing was flattened");
        // A huge circle whose left half lies far left: the picture is inside it.
        canvas.fill(
            &Circle::new((-1e6, 5.0), 1e6 + 20.0).to_path(0.1),
            false,
            [0, 0, 0],
            1.0,
        );
        assert_eq!(pixel(&canvas, 5, 5), [0, 0, 0, 255]);
    }

    #[test]
    fn full_height_edges_are_charged_per_row_so_a_zigzag_stops_early() {
        // The architect's zigzag: thin full-height teeth, each edge spanning every row.
        let mut zigzag = BezPath::new();
        zigzag.move_to((0.0, 0.0));
        for tooth in 0..100_000 {
            let x = f64::from(tooth) * 0.001;
            zigzag.line_to((x, 256.0));
            zigzag.line_to((x + 0.0005, 0.0));
        }
        let mut canvas = Canvas::new(256, 256, [255, 255, 255]).with_budget(100_000);
        canvas.fill(&zigzag, false, [0, 0, 0], 1.0);
        assert!(
            canvas.exhausted(),
            "200,000 edges x 1024 rows are over 100,000 units"
        );
        // A small fill costs its rows: 1 + 4 x 4 per side edge and 4 x 4 sample rows swept.
        let mut canvas = Canvas::new(10, 10, [255, 255, 255]).with_budget(1_000);
        canvas.fill(
            &Rect::new(2.0, 2.0, 6.0, 6.0).to_path(0.1),
            false,
            [0, 0, 0],
            1.0,
        );
        assert_eq!(canvas.budget, 1_000 - 2 * 17 - 16);
    }

    #[test]
    fn a_spent_budget_stops_the_drawing() {
        let mut canvas = Canvas::new(10, 10, [255, 255, 255]);
        canvas.budget = 3;
        canvas.fill(
            &Circle::new((5.0, 5.0), 4.0).to_path(0.1),
            false,
            [0, 0, 0],
            1.0,
        );
        assert!(canvas.exhausted());
        assert_eq!(pixel(&canvas, 5, 5), [255, 255, 255, 255]);
    }
}
