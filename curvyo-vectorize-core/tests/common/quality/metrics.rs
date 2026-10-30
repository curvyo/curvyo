//! The 0025 quality metrics of one trace against its thresholded picture (`specs/0025-raster-trace`
//! criteria 27, 28; `research.md` section 5), whose geometry helpers the slice A metrics share.

use kurbo::{BezPath, ParamCurve, ParamCurveArclen, PathEl, PathSeg, Point};

/// The distance from `p` to the segment `a`–`b`.
pub fn to_segment(p: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len = dx * dx + dy * dy;
    let t = if len == 0.0 {
        0.0
    } else {
        (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / len).clamp(0.0, 1.0)
    };
    ((p.0 - a.0 - t * dx).powi(2) + (p.1 - a.1 - t * dy).powi(2)).sqrt()
}

/// The metrics of one trace against its thresholded picture (`research.md` section 5).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Metrics {
    /// Nodes per 100 mm of outline.
    pub node_density: f64,
    /// Outlines below 0.25 mm² (a preset's run: below 0.95 x its Specks², criterion 61).
    pub micro_paths: f64,
    /// Hausdorff distance in pixels between the outlines and the pixel border, both ways.
    pub max_deviation_px: f64,
    /// Mean distance in pixels from the outlines to the pixel border.
    pub mean_deviation_px: f64,
    /// XOR of the outlines rasterised at 4x and the mask, as % of the foreground.
    pub area_error_pct: f64,
    /// Curvature sign flips per 100 mm.
    pub wobble: f64,
    /// Share of closed outlines, %.
    pub closed_pct: f64,
}

impl Metrics {
    pub const NAMES: [&'static str; 7] = [
        "node_density",
        "micro_paths",
        "max_deviation_px",
        "mean_deviation_px",
        "area_error_pct",
        "wobble",
        "closed_pct",
    ];

    pub fn values(&self) -> [f64; 7] {
        [
            self.node_density,
            self.micro_paths,
            self.max_deviation_px,
            self.mean_deviation_px,
            self.area_error_pct,
            self.wobble,
            self.closed_pct,
        ]
    }
}

/// The segments of `paths` flattened to `tolerance`, as polylines.
pub fn polylines(paths: &[BezPath], tolerance: f64) -> Vec<Vec<Point>> {
    let mut lines = Vec::new();
    for path in paths {
        let mut current: Vec<Point> = Vec::new();
        let mut start = Point::ZERO;
        kurbo::flatten(path.iter(), tolerance, |el| match el {
            PathEl::MoveTo(p) => {
                if current.len() > 1 {
                    lines.push(std::mem::take(&mut current));
                }
                current = vec![p];
                start = p;
            }
            PathEl::LineTo(p) => current.push(p),
            PathEl::ClosePath => {
                current.push(start);
            }
            _ => {}
        });
        if current.len() > 1 {
            lines.push(current);
        }
    }
    lines
}

/// The 4x raster of `paths` (pixel coordinates), nonzero, one bool per subpixel.
pub fn rasterise_4x(paths: &[BezPath], width: usize, height: usize) -> Vec<bool> {
    let (w, h) = (width * 4, height * 4);
    let lines = polylines(paths, 0.01);
    let mut edges: Vec<(Point, Point)> = Vec::new();
    for line in &lines {
        for pair in line.windows(2) {
            edges.push((pair[0], pair[1]));
        }
    }
    let mut out = vec![false; w * h];
    for row in 0..h {
        let y = (row as f64 + 0.5) / 4.0;
        let mut crossings: Vec<(f64, i32)> = edges
            .iter()
            .filter(|(a, b)| (a.y <= y) != (b.y <= y))
            .map(|(a, b)| {
                let t = (y - a.y) / (b.y - a.y);
                (a.x + t * (b.x - a.x), if b.y > a.y { 1 } else { -1 })
            })
            .collect();
        crossings.sort_by(|p, q| p.0.total_cmp(&q.0));
        let mut winding = 0;
        let mut from = 0.0;
        for (x, dir) in crossings {
            if winding != 0 {
                fill_span(&mut out[row * w..(row + 1) * w], from, x);
            }
            winding += dir;
            from = x;
        }
    }
    out
}

fn fill_span(row: &mut [bool], from: f64, to: f64) {
    for (i, cell) in row.iter_mut().enumerate() {
        let x = (i as f64 + 0.5) / 4.0;
        if x >= from && x < to {
            *cell = true;
        }
    }
}

/// The unit edges between foreground and background pixels (the picture's outside is
/// background), as segments in pixel coordinates.
pub fn pixel_border(mask: &[bool], width: usize, height: usize) -> Vec<(Point, Point)> {
    let at = |x: isize, y: isize| {
        x >= 0
            && y >= 0
            && (x as usize) < width
            && (y as usize) < height
            && mask[y as usize * width + x as usize]
    };
    let mut edges = Vec::new();
    for y in 0..=height as isize {
        for x in 0..=width as isize {
            if at(x, y) != at(x, y - 1) && x < width as isize {
                edges.push((
                    Point::new(x as f64, y as f64),
                    Point::new(x as f64 + 1.0, y as f64),
                ));
            }
            if at(x, y) != at(x - 1, y) && y < height as isize {
                edges.push((
                    Point::new(x as f64, y as f64),
                    Point::new(x as f64, y as f64 + 1.0),
                ));
            }
        }
    }
    edges
}

/// A bucket grid of segments for nearest-distance queries.
pub struct Grid {
    cell: f64,
    cols: usize,
    rows: usize,
    buckets: Vec<Vec<(Point, Point)>>,
}

impl Grid {
    pub fn new(segments: &[(Point, Point)], width: usize, height: usize, cell: f64) -> Self {
        let cols = (width as f64 / cell).ceil() as usize + 2;
        let rows = (height as f64 / cell).ceil() as usize + 2;
        let mut buckets = vec![Vec::new(); cols * rows];
        for &(a, b) in segments {
            let (x0, x1) = (a.x.min(b.x), a.x.max(b.x));
            let (y0, y1) = (a.y.min(b.y), a.y.max(b.y));
            let c0 = ((x0 / cell).floor().max(-1.0) + 1.0) as usize;
            let c1 = (((x1 / cell).floor() + 1.0) as usize).min(cols - 1);
            let r0 = ((y0 / cell).floor().max(-1.0) + 1.0) as usize;
            let r1 = (((y1 / cell).floor() + 1.0) as usize).min(rows - 1);
            for r in r0..=r1 {
                for c in c0..=c1 {
                    buckets[r * cols + c].push((a, b));
                }
            }
        }
        Self {
            cell,
            cols,
            rows,
            buckets,
        }
    }

    /// The distance from `p` to the nearest segment, searching rings of cells outward.
    pub fn distance(&self, p: Point) -> f64 {
        let c = ((p.x / self.cell).floor() + 1.0).clamp(0.0, (self.cols - 1) as f64) as isize;
        let r = ((p.y / self.cell).floor() + 1.0).clamp(0.0, (self.rows - 1) as f64) as isize;
        let mut best = f64::INFINITY;
        for ring in 0..(self.cols.max(self.rows) as isize) {
            for rr in (r - ring)..=(r + ring) {
                for cc in (c - ring)..=(c + ring) {
                    if (rr - r).abs() != ring && (cc - c).abs() != ring {
                        continue;
                    }
                    if rr < 0 || cc < 0 || rr as usize >= self.rows || cc as usize >= self.cols {
                        continue;
                    }
                    for &(a, b) in &self.buckets[rr as usize * self.cols + cc as usize] {
                        best = best.min(to_segment((p.x, p.y), (a.x, a.y), (b.x, b.y)));
                    }
                }
            }
            // Every cell not yet searched is at least `ring` cells away.
            if best <= ring as f64 * self.cell {
                break;
            }
        }
        best
    }
}

/// The metrics of `outlines` (in pixel coordinates) against `mask`, with `mm_per_px`.
pub fn measure(
    outlines: &[BezPath],
    mask: &[bool],
    width: usize,
    height: usize,
    mm_per_px: f64,
) -> Metrics {
    measure_micro(outlines, mask, width, height, mm_per_px, 0.25)
}

/// [`measure`] with micro paths counted below `micro_mm2`.
pub fn measure_micro(
    outlines: &[BezPath],
    mask: &[bool],
    width: usize,
    height: usize,
    mm_per_px: f64,
    micro_mm2: f64,
) -> Metrics {
    let length_px: f64 = outlines
        .iter()
        .flat_map(BezPath::segments)
        .map(|s| s.arclen(1e-3))
        .sum();
    let length_mm = (length_px * mm_per_px).max(f64::MIN_POSITIVE);
    let nodes: usize = outlines.iter().map(|p| p.segments().count()).sum();
    let micro = outlines
        .iter()
        .filter(|p| kurbo::Shape::area(*p).abs() * mm_per_px * mm_per_px < micro_mm2)
        .count();
    // Deviation: samples every 0.1 px along the outlines to the pixel border, and the border's
    // edge midpoints to the outlines.
    let border = pixel_border(mask, width, height);
    let border_grid = Grid::new(&border, width, height, 4.0);
    let mut sum = 0.0;
    let mut count = 0_usize;
    let mut max = 0.0_f64;
    for path in outlines {
        for seg in path.segments() {
            let steps = (seg.arclen(1e-3) / 0.1).ceil().max(1.0) as usize;
            for i in 0..steps {
                let d = border_grid.distance(seg.eval(i as f64 / steps as f64));
                sum += d;
                count += 1;
                max = max.max(d);
            }
        }
    }
    let lines = polylines(outlines, 0.01);
    let line_segments: Vec<(Point, Point)> = lines
        .iter()
        .flat_map(|l| l.windows(2).map(|w| (w[0], w[1])))
        .collect();
    if !line_segments.is_empty() {
        let line_grid = Grid::new(&line_segments, width, height, 4.0);
        for &(a, b) in &border {
            max = max.max(line_grid.distance(a.midpoint(b)));
        }
    }
    // Area error at 4x.
    let raster = rasterise_4x(outlines, width, height);
    let (w4, mut xor, mut fg) = (width * 4, 0_usize, 0_usize);
    for (i, &covered) in raster.iter().enumerate() {
        let (x, y) = ((i % w4) / 4, (i / w4) / 4);
        let reference = mask[y * width + x];
        fg += usize::from(reference);
        xor += usize::from(covered != reference);
    }
    // Wobble: sign changes of the turn between chords of the flattened outline.
    let mut flips = 0_usize;
    for line in polylines(outlines, 0.02) {
        let mut last_sign = 0.0_f64;
        for t in line.windows(3) {
            let turn = (t[1] - t[0]).cross(t[2] - t[1]);
            let len = (t[1] - t[0]).hypot() * (t[2] - t[1]).hypot();
            if len == 0.0 || (turn / len).abs() < 1e-3 {
                continue;
            }
            let sign = turn.signum();
            if last_sign != 0.0 && sign != last_sign {
                flips += 1;
            }
            last_sign = sign;
        }
    }
    let closed = outlines
        .iter()
        .filter(|p| matches!(p.elements().last(), Some(PathEl::ClosePath)))
        .count();
    let per_100mm = |n: usize| n as f64 / length_mm * 100.0;
    Metrics {
        node_density: per_100mm(nodes),
        micro_paths: micro as f64,
        max_deviation_px: max,
        mean_deviation_px: if count == 0 { 0.0 } else { sum / count as f64 },
        area_error_pct: if fg == 0 {
            0.0
        } else {
            xor as f64 / fg as f64 * 100.0
        },
        wobble: per_100mm(flips),
        closed_pct: if outlines.is_empty() {
            100.0
        } else {
            closed as f64 / outlines.len() as f64 * 100.0
        },
    }
}

/// The area error against the drawn shape (`adrs.md` 2026-10-28, second note, point 2): XOR of
/// `outlines` rasterised at 4x with `truth_4x`, as % of the true foreground.
pub fn area_error_true_pct(
    outlines: &[BezPath],
    truth_4x: &[bool],
    width: usize,
    height: usize,
) -> f64 {
    let raster = rasterise_4x(outlines, width, height);
    let fg = truth_4x.iter().filter(|&&t| t).count();
    let xor = raster.iter().zip(truth_4x).filter(|(a, b)| a != b).count();
    if fg == 0 {
        0.0
    } else {
        xor as f64 / fg as f64 * 100.0
    }
}

/// The largest distance from samples every 0.1 px along `a` to the flattened `b`, in pixels
/// (the directed half of a Hausdorff distance).
pub fn directed_distance(a: &[BezPath], b: &[BezPath], width: usize, height: usize) -> f64 {
    let lines = polylines(b, 0.001);
    let segments: Vec<(Point, Point)> = lines
        .iter()
        .flat_map(|l| l.windows(2).map(|w| (w[0], w[1])))
        .collect();
    if segments.is_empty() {
        return f64::INFINITY;
    }
    let grid = Grid::new(&segments, width, height, 4.0);
    let mut max = 0.0_f64;
    for path in a {
        for seg in path.segments() {
            let steps = (seg.arclen(1e-3) / 0.1).ceil().max(1.0) as usize;
            for i in 0..=steps {
                max = max.max(grid.distance(seg.eval(i as f64 / steps as f64)));
            }
        }
    }
    max
}

/// The segments of a path, for callers outside this module.
pub fn segments(path: &BezPath) -> impl Iterator<Item = PathSeg> + '_ {
    path.segments()
}
