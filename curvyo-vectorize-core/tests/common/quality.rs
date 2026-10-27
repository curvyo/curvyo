//! The black and white quality set and its metrics (`specs/0025-raster-trace` criteria 27, 28;
//! `research.md` section 5). The pictures are synthetic, drawn here from fixed shapes and a fixed
//! noise sequence, so they need no copyright note and no binary in the repository; the
//! `#[ignore]` writer puts them in `target/trace-quality/` for the offline baselines.

use kurbo::{BezPath, ParamCurve, ParamCurveArclen, PathEl, PathSeg, Point};

use super::{Picture, render};

/// What kind of picture an image of the set is (the area error target depends on it).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A logo or text: area error under 1 %.
    LogoOrText,
    /// Everything else: area error under 3 %.
    Other,
}

/// One image of the set.
pub struct QualityImage {
    /// Its name, with the number of `research.md` section 5.
    pub name: &'static str,
    pub picture: Picture,
    /// Its resolution: the pixel size in mm is `25.4 / dpi`.
    pub dpi: f64,
    pub kind: Kind,
}

/// A fixed pseudo-random sequence (64-bit LCG), the same on every machine.
pub struct Noise(u64);

impl Noise {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    /// The next value in `-1..1`.
    pub fn next(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((self.0 >> 11) as f64 / (1_u64 << 53) as f64) * 2.0 - 1.0
    }
}

/// The distance from `p` to the segment `a`–`b`.
fn to_segment(p: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len = dx * dx + dy * dy;
    let t = if len == 0.0 {
        0.0
    } else {
        (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / len).clamp(0.0, 1.0)
    };
    ((p.0 - a.0 - t * dx).powi(2) + (p.1 - a.1 - t * dy).powi(2)).sqrt()
}

/// Whether `p` is inside the polygon `points` (even-odd).
fn in_polygon(p: (f64, f64), points: &[(f64, f64)]) -> bool {
    let mut inside = false;
    let mut j = points.len() - 1;
    for i in 0..points.len() {
        let (a, b) = (points[i], points[j]);
        if (a.1 > p.1) != (b.1 > p.1) && p.0 < (b.0 - a.0) * (p.1 - a.1) / (b.1 - a.1) + a.0 {
            inside = !inside;
        }
        j = i;
    }
    inside
}

fn star(cx: f64, cy: f64, outer: f64, inner: f64, points: usize) -> Vec<(f64, f64)> {
    (0..points * 2)
        .map(|i| {
            let r = if i % 2 == 0 { outer } else { inner };
            let a = std::f64::consts::PI * i as f64 / points as f64 - std::f64::consts::FRAC_PI_2;
            (cx + r * a.cos(), cy + r * a.sin())
        })
        .collect()
}

/// Stick letters on a grid of 4 x 6 units: the strokes of "CURVYO TRACE 2026".
fn letter(c: char) -> &'static [((f64, f64), (f64, f64))] {
    match c {
        'C' => &[
            ((4., 0.), (0., 0.)),
            ((0., 0.), (0., 6.)),
            ((0., 6.), (4., 6.)),
        ],
        'U' => &[
            ((0., 0.), (0., 6.)),
            ((0., 6.), (4., 6.)),
            ((4., 6.), (4., 0.)),
        ],
        'R' => &[
            ((0., 6.), (0., 0.)),
            ((0., 0.), (4., 0.)),
            ((4., 0.), (4., 3.)),
            ((4., 3.), (0., 3.)),
            ((1., 3.), (4., 6.)),
        ],
        'V' => &[((0., 0.), (2., 6.)), ((2., 6.), (4., 0.))],
        'Y' => &[
            ((0., 0.), (2., 3.)),
            ((4., 0.), (2., 3.)),
            ((2., 3.), (2., 6.)),
        ],
        'O' | '0' => &[
            ((0., 0.), (4., 0.)),
            ((4., 0.), (4., 6.)),
            ((4., 6.), (0., 6.)),
            ((0., 6.), (0., 0.)),
        ],
        'T' => &[((0., 0.), (4., 0.)), ((2., 0.), (2., 6.))],
        'A' => &[
            ((0., 6.), (2., 0.)),
            ((2., 0.), (4., 6.)),
            ((1., 3.), (3., 3.)),
        ],
        'E' => &[
            ((4., 0.), (0., 0.)),
            ((0., 0.), (0., 6.)),
            ((0., 6.), (4., 6.)),
            ((0., 3.), (3., 3.)),
        ],
        '2' => &[
            ((0., 0.), (4., 0.)),
            ((4., 0.), (4., 3.)),
            ((4., 3.), (0., 6.)),
            ((0., 6.), (4., 6.)),
        ],
        '6' => &[
            ((4., 0.), (0., 0.)),
            ((0., 0.), (0., 6.)),
            ((0., 6.), (4., 6.)),
            ((4., 6.), (4., 3.)),
            ((4., 3.), (0., 3.)),
        ],
        _ => &[],
    }
}

/// The strokes of `text` at `(x, y)` with `unit` px per grid unit.
fn text_strokes(text: &str, x: f64, y: f64, unit: f64) -> Vec<((f64, f64), (f64, f64))> {
    let mut strokes = Vec::new();
    for (i, c) in text.chars().enumerate() {
        let left = x + i as f64 * 6.0 * unit;
        for &((ax, ay), (bx, by)) in letter(c) {
            strokes.push((
                (left + ax * unit, y + ay * unit),
                (left + bx * unit, y + by * unit),
            ));
        }
    }
    strokes
}

fn near_strokes(p: (f64, f64), strokes: &[((f64, f64), (f64, f64))], radius: f64) -> bool {
    strokes.iter().any(|&(a, b)| to_segment(p, a, b) <= radius)
}

/// Adds `amplitude` grey levels of fixed noise to every pixel.
fn with_noise(picture: Picture, amplitude: f64, seed: u64) -> Picture {
    let mut noise = Noise::new(seed);
    let levels: Vec<u8> = picture
        .levels()
        .into_iter()
        .map(|v| {
            (f64::from(v) + amplitude * noise.next())
                .round()
                .clamp(0.0, 255.0) as u8
        })
        .collect();
    Picture::grey(picture.width, picture.height, &levels)
}

/// The black and white set: images 1, 3, 4, 5, 7, 10, 11 and 12 of `research.md` section 5.
pub fn quality_set() -> Vec<QualityImage> {
    let logo = {
        let star = star(420.0, 190.0, 140.0, 60.0, 5);
        render(600, 400, 4, move |x, y| {
            let d = ((x - 150.0).powi(2) + (y - 190.0).powi(2)).sqrt();
            (50.0..=110.0).contains(&d)
                || in_polygon((x, y), &star)
                || ((60.0..540.0).contains(&x) && (345.0..372.0).contains(&y))
        })
    };
    let text = {
        let mut strokes = text_strokes("CURVYO TRACE", 30.0, 30.0, 10.0);
        strokes.extend(text_strokes("TRACE 2026 CURVY", 30.0, 160.0, 5.0));
        strokes.extend(text_strokes("CURVE TO TRACE", 30.0, 230.0, 3.0));
        render(900, 300, 4, move |x, y| near_strokes((x, y), &strokes, 4.0))
    };
    let small = {
        let strokes = text_strokes("TRACE", 4.0, 6.0, 1.6);
        let strokes2 = text_strokes("CURVY", 4.0, 30.0, 1.6);
        render(120, 60, 4, move |x, y| {
            near_strokes((x, y), &strokes, 0.9) || near_strokes((x, y), &strokes2, 0.9)
        })
    };
    let line_art = {
        let skew = 1.5_f64.to_radians();
        let mut strokes = Vec::new();
        let spiral: Vec<(f64, f64)> = (0..400)
            .map(|i| {
                let t = f64::from(i) * 0.05;
                (200.0 + 6.0 * t * t.cos(), 200.0 + 6.0 * t * t.sin())
            })
            .collect();
        strokes.extend(spiral.windows(2).map(|w| (w[0], w[1])));
        for k in 0..4 {
            let wave: Vec<(f64, f64)> = (0..=120)
                .map(|i| {
                    let x = 40.0 + f64::from(i) * 4.4;
                    (x, 430.0 + f64::from(k) * 40.0 + 15.0 * (x / 37.0).sin())
                })
                .collect();
            strokes.extend(wave.windows(2).map(|w| (w[0], w[1])));
        }
        let drawn = render(600, 600, 4, move |x, y| {
            let (sx, sy) = (
                x * skew.cos() + y * skew.sin(),
                -x * skew.sin() + y * skew.cos(),
            );
            near_strokes((sx, sy), &strokes, 2.5)
        });
        with_noise(drawn, 14.0, 5)
    };
    let silhouette = {
        let blobs = [
            (300.0, 300.0, 170.0),
            (180.0, 200.0, 90.0),
            (420.0, 190.0, 80.0),
            (300.0, 470.0, 70.0),
        ];
        let mut noise = Noise::new(7);
        let holes: Vec<(f64, f64, f64)> = (0..14)
            .map(|i| {
                let a = f64::from(i) * 0.9;
                (
                    300.0 + 120.0 * a.cos(),
                    300.0 + 120.0 * a.sin(),
                    7.0 + 3.0 * noise.next(),
                )
            })
            .collect();
        let drawn = render(600, 600, 4, move |x, y| {
            let inside = blobs
                .iter()
                .any(|&(cx, cy, r)| (x - cx).powi(2) + (y - cy).powi(2) <= r * r);
            inside
                && !holes
                    .iter()
                    .any(|&(cx, cy, r)| (x - cx).powi(2) + (y - cy).powi(2) <= r * r)
        });
        with_noise(drawn, 18.0, 11)
    };
    let pixel_art = {
        let mut noise = Noise::new(3);
        let cells: Vec<bool> = (0..16 * 8).map(|_| noise.next() > 0.1).collect();
        render(256, 256, 1, move |x, y| {
            let (cx, cy) = ((x / 16.0) as usize, (y / 16.0) as usize);
            // Mirrored like an icon.
            let col = if cx < 8 { cx } else { 15 - cx };
            cells[cy * 8 + col]
        })
    };
    let stencil = {
        let mut strokes = Vec::new();
        for i in 0..10 {
            let o = 40.0 + f64::from(i) * 56.0;
            strokes.push(((o, 20.0), (o, 580.0)));
            strokes.push((
                (20.0, 20.0 + f64::from(i) * 60.0),
                (580.0, 60.0 + f64::from(i) * 50.0),
            ));
        }
        render(600, 600, 4, move |x, y| {
            near_strokes((x, y), &strokes, 1.75)
        })
    };
    let technical = {
        let rects = [
            (40.0, 40.0, 760.0, 560.0),
            (100.0, 100.0, 380.0, 300.0),
            (450.0, 350.0, 700.0, 500.0),
        ];
        let circles = [
            (240.0, 200.0, 60.0),
            (575.0, 200.0, 110.0),
            (575.0, 200.0, 40.0),
            (240.0, 430.0, 80.0),
        ];
        render(800, 600, 4, move |x, y| {
            let on_rect = rects.iter().any(|&(x0, y0, x1, y1)| {
                let near_x = (x0 - 1.5..=x1 + 1.5).contains(&x);
                let near_y = (y0 - 1.5..=y1 + 1.5).contains(&y);
                (near_x && ((y - y0).abs() <= 1.5 || (y - y1).abs() <= 1.5))
                    || (near_y && ((x - x0).abs() <= 1.5 || (x - x1).abs() <= 1.5))
            });
            let on_circle = circles.iter().any(|&(cx, cy, r)| {
                (((x - cx).powi(2) + (y - cy).powi(2)).sqrt() - r).abs() <= 1.5
            });
            // An arc: the upper half of a circle.
            let arc = y <= 430.0
                && ((((x - 600.0).powi(2) + (y - 430.0).powi(2)).sqrt() - 100.0).abs() <= 1.5);
            on_rect || on_circle || arc
        })
    };
    vec![
        QualityImage {
            name: "01-flat-logo",
            picture: logo,
            dpi: 96.0,
            kind: Kind::LogoOrText,
        },
        QualityImage {
            name: "03-text-300dpi",
            picture: text,
            dpi: 300.0,
            kind: Kind::LogoOrText,
        },
        QualityImage {
            name: "04-small-text",
            picture: small,
            dpi: 72.0,
            kind: Kind::LogoOrText,
        },
        QualityImage {
            name: "05-line-art-scan",
            picture: line_art,
            dpi: 150.0,
            kind: Kind::Other,
        },
        QualityImage {
            name: "07-silhouette",
            picture: silhouette,
            dpi: 150.0,
            kind: Kind::Other,
        },
        QualityImage {
            name: "10-pixel-art",
            picture: pixel_art,
            dpi: 96.0,
            kind: Kind::Other,
        },
        QualityImage {
            name: "11-stencil-lines",
            picture: stencil,
            dpi: 300.0,
            kind: Kind::Other,
        },
        QualityImage {
            name: "12-technical-drawing",
            picture: technical,
            dpi: 150.0,
            kind: Kind::Other,
        },
    ]
}

/// The metrics of one trace against its thresholded picture (`research.md` section 5).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Metrics {
    /// Nodes per 100 mm of outline.
    pub node_density: f64,
    /// Outlines below 0.25 mm².
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
fn polylines(paths: &[BezPath], tolerance: f64) -> Vec<Vec<Point>> {
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
fn rasterise_4x(paths: &[BezPath], width: usize, height: usize) -> Vec<bool> {
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
fn pixel_border(mask: &[bool], width: usize, height: usize) -> Vec<(Point, Point)> {
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
struct Grid {
    cell: f64,
    cols: usize,
    rows: usize,
    buckets: Vec<Vec<(Point, Point)>>,
}

impl Grid {
    fn new(segments: &[(Point, Point)], width: usize, height: usize, cell: f64) -> Self {
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
    fn distance(&self, p: Point) -> f64 {
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
    let length_px: f64 = outlines
        .iter()
        .flat_map(BezPath::segments)
        .map(|s| s.arclen(1e-3))
        .sum();
    let length_mm = (length_px * mm_per_px).max(f64::MIN_POSITIVE);
    let nodes: usize = outlines.iter().map(|p| p.segments().count()).sum();
    let micro = outlines
        .iter()
        .filter(|p| kurbo::Shape::area(*p).abs() * mm_per_px * mm_per_px < 0.25)
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

/// The segments of a path, for callers outside this module.
pub fn segments(path: &BezPath) -> impl Iterator<Item = PathSeg> + '_ {
    path.segments()
}
