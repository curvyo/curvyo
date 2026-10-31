//! Renders a reference scene (coloured layers of filled vector paths or implicit shapes over a
//! background) at any scale to straight-alpha RGBA with 4 x 4 samples per pixel, keeping the
//! samples as the 4x truth (`specs/0089-trace-quality-and-colour` criterion 2).

use std::sync::Arc;

use kurbo::{Affine, BezPath, PathEl, Point};

use super::super::Picture;

/// The inside test of an implicit shape, in the scene's 1x pixels.
pub type Inside = Arc<dyn Fn(f64, f64) -> bool + Send + Sync>;

/// What a layer covers.
#[derive(Clone)]
pub enum Shape {
    /// Closed paths filled even-odd, in 1x pixels; the scene's vector truth.
    Paths(Vec<BezPath>),
    /// A shape known only by its inside test (strokes, text); measured against the 4x truth.
    Implicit(Inside),
}

/// One coloured layer of a scene.
#[derive(Clone)]
pub struct Layer {
    pub shape: Shape,
    /// The ink, straight alpha.
    pub rgba: [u8; 4],
    /// A seam colour drawn under the ink over the ink's 3 x 3 dilation (grey seams of cheap
    /// resampling); not part of the truth.
    pub seam: Option<[u8; 4]>,
    /// Whether the layer is content the trace must find (part of the truth); a layer the
    /// selection must leave out (an unpicked colour, slice C) is not.
    pub content: bool,
}

impl Layer {
    pub fn ink(shape: Shape, rgba: [u8; 4]) -> Self {
        Self {
            shape,
            rgba,
            seam: None,
            content: true,
        }
    }

    /// A layer drawn but not content.
    pub fn other(shape: Shape, rgba: [u8; 4]) -> Self {
        Self {
            content: false,
            ..Self::ink(shape, rgba)
        }
    }
}

/// A picture to draw: its 1x size in pixels, its background and its layers (which must not
/// overlap, so their paths together are the truth boundary).
#[derive(Clone)]
pub struct Scene {
    pub width: f64,
    pub height: f64,
    pub background: [u8; 4],
    pub layers: Vec<Layer>,
}

/// A scene drawn at one scale.
pub struct Drawn {
    pub picture: Picture,
    /// Every layer's samples together, 4 x 4 per pixel, row by row of the 4x grid.
    pub truth_4x: Vec<bool>,
}

impl Scene {
    /// The pixel size at `scale`.
    pub fn size(&self, scale: f64) -> (usize, usize) {
        (
            (self.width * scale).round() as usize,
            (self.height * scale).round() as usize,
        )
    }

    /// The truth boundary at `scale` in pixels, or `None` when a layer is implicit.
    pub fn truth_paths(&self, scale: f64) -> Option<Vec<BezPath>> {
        let mut out = Vec::new();
        for layer in self.layers.iter().filter(|l| l.content) {
            match &layer.shape {
                Shape::Paths(paths) => {
                    out.extend(paths.iter().map(|p| Affine::scale(scale) * p.clone()));
                }
                Shape::Implicit(_) => return None,
            }
        }
        Some(out)
    }

    /// Draws the scene at `scale`.
    pub fn draw(&self, scale: f64) -> Drawn {
        let (w, h) = self.size(scale);
        let (w4, h4) = (w * 4, h * 4);
        let mut truth = vec![false; w4 * h4];
        let mut rgba: Vec<[f64; 4]> = vec![premultiply(self.background); w * h];
        for layer in &self.layers {
            let samples = match &layer.shape {
                Shape::Paths(paths) => fill_even_odd(paths, scale, w4, h4),
                Shape::Implicit(inside) => sample(inside, scale, w4, h4),
            };
            let cover = coverage(&samples, w, h);
            if let Some(seam) = layer.seam {
                let grown = dilate(&cover, w, h);
                composite(&mut rgba, &grown, seam);
            }
            composite(&mut rgba, &cover, layer.rgba);
            if layer.content {
                for (t, s) in truth.iter_mut().zip(&samples) {
                    *t |= *s;
                }
            }
        }
        let bytes = rgba.iter().flat_map(|p| unpremultiply(*p)).collect();
        Drawn {
            picture: Picture {
                width: w as u32,
                height: h as u32,
                rgba: bytes,
            },
            truth_4x: truth,
        }
    }
}

fn premultiply(c: [u8; 4]) -> [f64; 4] {
    let a = f64::from(c[3]) / 255.0;
    [
        f64::from(c[0]) * a,
        f64::from(c[1]) * a,
        f64::from(c[2]) * a,
        a,
    ]
}

fn unpremultiply(p: [f64; 4]) -> [u8; 4] {
    let a = p[3];
    let channel = |v: f64| {
        if a <= 0.0 {
            0
        } else {
            (v / a).round().clamp(0.0, 255.0) as u8
        }
    };
    [
        channel(p[0]),
        channel(p[1]),
        channel(p[2]),
        (a * 255.0).round().clamp(0.0, 255.0) as u8,
    ]
}

/// Draws `colour` with per-pixel `cover` over `rgba` (premultiplied "over").
fn composite(rgba: &mut [[f64; 4]], cover: &[f64], colour: [u8; 4]) {
    let ink = premultiply(colour);
    for (p, &c) in rgba.iter_mut().zip(cover) {
        if c > 0.0 {
            for k in 0..4 {
                p[k] = ink[k] * c + p[k] * (1.0 - ink[3] * c);
            }
        }
    }
}

/// The share of each pixel's 16 samples that are set.
fn coverage(samples: &[bool], w: usize, h: usize) -> Vec<f64> {
    let w4 = w * 4;
    let mut out = vec![0.0; w * h];
    for (i, &s) in samples.iter().enumerate() {
        if s {
            let (x, y) = ((i % w4) / 4, (i / w4) / 4);
            out[y * w + x] += 1.0 / 16.0;
        }
    }
    out
}

/// The largest coverage in each pixel's 3 x 3 neighbourhood.
fn dilate(cover: &[f64], w: usize, h: usize) -> Vec<f64> {
    let mut out = vec![0.0; w * h];
    for y in 0..h {
        for x in 0..w {
            let mut m = 0.0_f64;
            for yy in y.saturating_sub(1)..(y + 2).min(h) {
                for xx in x.saturating_sub(1)..(x + 2).min(w) {
                    m = m.max(cover[yy * w + xx]);
                }
            }
            out[y * w + x] = m;
        }
    }
    out
}

/// The samples of an implicit shape on the 4x grid.
fn sample(inside: &Inside, scale: f64, w4: usize, h4: usize) -> Vec<bool> {
    let mut out = vec![false; w4 * h4];
    for row in 0..h4 {
        let y = (row as f64 + 0.5) / 4.0 / scale;
        for col in 0..w4 {
            out[row * w4 + col] = inside((col as f64 + 0.5) / 4.0 / scale, y);
        }
    }
    out
}

/// The samples of `paths` (1x pixels, scaled by `scale`) filled even-odd on the 4x grid.
pub fn fill_even_odd(paths: &[BezPath], scale: f64, w4: usize, h4: usize) -> Vec<bool> {
    let mut edges: Vec<(Point, Point)> = Vec::new();
    for path in paths {
        let scaled = Affine::scale(scale * 4.0) * path.clone();
        let (mut start, mut last) = (Point::ZERO, Point::ZERO);
        kurbo::flatten(scaled.iter(), 0.02, |el| match el {
            PathEl::MoveTo(p) => {
                if last != start {
                    edges.push((last, start));
                }
                start = p;
                last = p;
            }
            PathEl::LineTo(p) => {
                edges.push((last, p));
                last = p;
            }
            PathEl::ClosePath => {
                edges.push((last, start));
                last = start;
            }
            _ => {}
        });
        if last != start {
            edges.push((last, start));
        }
    }
    let mut rows: Vec<Vec<usize>> = vec![Vec::new(); h4];
    for (i, (a, b)) in edges.iter().enumerate() {
        let (y0, y1) = (a.y.min(b.y), a.y.max(b.y));
        let r0 = (y0 - 0.5).ceil().max(0.0) as usize;
        let r1 = ((y1 - 0.5).ceil().max(0.0) as usize).min(h4);
        for row in rows.iter_mut().take(r1).skip(r0) {
            row.push(i);
        }
    }
    let mut out = vec![false; w4 * h4];
    let mut xs = Vec::new();
    for (r, row) in rows.iter().enumerate() {
        let y = r as f64 + 0.5;
        xs.clear();
        for &i in row {
            let (a, b) = edges[i];
            if (a.y <= y) != (b.y <= y) {
                xs.push(a.x + (y - a.y) / (b.y - a.y) * (b.x - a.x));
            }
        }
        xs.sort_by(f64::total_cmp);
        for pair in xs.as_chunks::<2>().0 {
            let c0 = (pair[0] - 0.5).ceil().max(0.0) as usize;
            let c1 = ((pair[1] - 0.5).ceil().max(0.0) as usize).min(w4);
            for cell in &mut out[r * w4 + c0.min(w4)..r * w4 + c1.max(c0.min(w4))] {
                *cell = true;
            }
        }
    }
    out
}

/// A closed polygon through `points`.
pub fn polygon(points: &[Point]) -> BezPath {
    let mut path = BezPath::new();
    path.move_to(points[0]);
    for &p in &points[1..] {
        path.line_to(p);
    }
    path.close_path();
    path
}

/// The corners of a `w` x `h` rectangle centred at `c`, turned by `degrees` (clockwise on the
/// y-down canvas), in drawing order.
pub fn rotated_rect(c: Point, w: f64, h: f64, degrees: f64) -> Vec<Point> {
    let (s, k) = degrees.to_radians().sin_cos();
    [(-w, -h), (w, -h), (w, h), (-w, h)]
        .iter()
        .map(|&(x, y)| {
            let (x, y) = (x / 2.0, y / 2.0);
            Point::new(c.x + x * k - y * s, c.y + x * s + y * k)
        })
        .collect()
}

/// `n` points on a circle (closed polygon of a circle, error below 1e-3 px for r < 500 px when
/// `n` is 720).
pub fn circle_points(c: Point, r: f64, n: usize) -> Vec<Point> {
    arc_points(c, r, 0.0, 360.0, n, false)
}

/// Points on the arc from `from` to `to` degrees (clockwise on the canvas), `n` steps; the end
/// point is included unless `closed_circle`.
pub fn arc_points(
    c: Point,
    r: f64,
    from: f64,
    to: f64,
    n: usize,
    closed_circle: bool,
) -> Vec<Point> {
    let steps = if closed_circle { n } else { n + 1 };
    (0..steps)
        .map(|i| {
            let a = (from + (to - from) * i as f64 / n as f64).to_radians();
            Point::new(c.x + r * a.cos(), c.y + r * a.sin())
        })
        .collect()
}

/// A disc as a polygon of 720 points.
pub fn disc(c: Point, r: f64) -> BezPath {
    polygon(&arc_points(c, r, 0.0, 360.0, 720, true))
}

/// A ring (even-odd: the outer circle and the inner one).
pub fn ring(c: Point, outer: f64, inner: f64) -> BezPath {
    let mut path = disc(c, outer);
    path.extend(disc(c, inner));
    path
}

/// The band of width `width` along the arc of radius `r` from `from` to `to` degrees, as one
/// closed outline (an open stroke with butt ends).
pub fn arc_band(c: Point, r: f64, width: f64, from: f64, to: f64) -> BezPath {
    let n = ((to - from).abs() * 2.0).ceil().max(8.0) as usize;
    let mut points = arc_points(c, r + width / 2.0, from, to, n, false);
    let mut inner = arc_points(c, r - width / 2.0, from, to, n, false);
    inner.reverse();
    points.extend(inner);
    polygon(&points)
}
