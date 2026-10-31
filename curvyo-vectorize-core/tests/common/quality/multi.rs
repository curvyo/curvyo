//! The multicolour fixtures of `specs/0089-trace-quality-and-colour` slice M (criteria 26 to
//! 30): coloured layers known by their inside tests, drawn with 4 x 4 samples per pixel (an
//! exact area average, no seams of their own), the label of every sample kept as the truth; and
//! the metrics of a colour trace against it: colour assignment, gaps and overlaps on the 4x
//! grid, shared-boundary identity, triple junctions, nodes.

use std::sync::Arc;

use curvyo_vectorize_core::{ColourTrace, Rgb8};
use kurbo::{BezPath, PathEl, Point};

use super::super::Picture;
use super::degrade::jpeg;
use super::render::fill_even_odd;

/// The inside test of a layer, in 1x pixels.
pub type Inside = Arc<dyn Fn(f64, f64) -> bool + Send + Sync>;

/// A multicolour picture: its size, its background (alpha 0 for a transparent one) and its
/// layers bottom to top (a later layer covers an earlier one).
#[derive(Clone)]
pub struct MultiScene {
    pub name: &'static str,
    pub width: usize,
    pub height: usize,
    pub background: [u8; 4],
    pub layers: Vec<(Inside, [u8; 4])>,
    /// JPEG quality applied after drawing, if any.
    pub jpeg: Option<u32>,
    /// The colours the trace should find (the layers' distinct colours unless merged on
    /// purpose).
    pub colours: usize,
    /// A fixed count of colours for the trace (a gradient in steps), else automatic.
    pub count: Option<u8>,
}

/// A drawn multicolour picture and its truth: per 4x sample `0` background or `k + 1` for layer
/// `k`.
pub struct MultiDrawn {
    pub picture: Picture,
    pub labels_4x: Vec<u8>,
}

impl MultiScene {
    /// Draws the scene: each pixel is the mean of its 16 samples' colours (straight alpha
    /// averaged premultiplied).
    pub fn draw(&self) -> MultiDrawn {
        let (w, h) = (self.width, self.height);
        let mut labels = vec![0_u8; w * h * 16];
        let mut rgba = Vec::with_capacity(w * h * 4);
        for y in 0..h {
            for x in 0..w {
                let mut sum = [0.0_f64; 4];
                for j in 0..4 {
                    for i in 0..4 {
                        let (sx, sy) = (
                            x as f64 + (i as f64 + 0.5) / 4.0,
                            y as f64 + (j as f64 + 0.5) / 4.0,
                        );
                        let mut label = 0;
                        let mut c = self.background;
                        for (k, (inside, colour)) in self.layers.iter().enumerate() {
                            if inside(sx, sy) {
                                label = k + 1;
                                c = *colour;
                            }
                        }
                        labels[(y * 4 + j) * w * 4 + x * 4 + i] = label as u8;
                        let a = f64::from(c[3]) / 255.0;
                        for k in 0..3 {
                            sum[k] += f64::from(c[k]) * a;
                        }
                        sum[3] += a;
                    }
                }
                let a = sum[3] / 16.0;
                for k in 0..3 {
                    rgba.push(if a > 0.0 {
                        (sum[k] / 16.0 / a).round().clamp(0.0, 255.0) as u8
                    } else {
                        0
                    });
                }
                rgba.push((a * 255.0).round() as u8);
            }
        }
        let mut picture = Picture {
            width: w as u32,
            height: h as u32,
            rgba,
        };
        if let Some(q) = self.jpeg {
            jpeg(&mut picture, q);
        }
        MultiDrawn {
            picture,
            labels_4x: labels,
        }
    }
}

fn disc(cx: f64, cy: f64, r: f64) -> Inside {
    Arc::new(move |x, y| (x - cx).powi(2) + (y - cy).powi(2) <= r * r)
}

fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Inside {
    Arc::new(move |x, y| x >= x0 && x < x1 && y >= y0 && y < y1)
}

fn ring(cx: f64, cy: f64, outer: f64, inner: f64) -> Inside {
    Arc::new(move |x, y| {
        let d = (x - cx).powi(2) + (y - cy).powi(2);
        d <= outer * outer && d > inner * inner
    })
}

/// A pie sector around (`cx`, `cy`) between the directions `from` and `to` (radians, from +x
/// clockwise on screen), within radius `r`.
fn sector(cx: f64, cy: f64, r: f64, from: f64, to: f64) -> Inside {
    Arc::new(move |x, y| {
        let (dx, dy) = (x - cx, y - cy);
        if dx * dx + dy * dy > r * r {
            return false;
        }
        let mut a = dy.atan2(dx);
        if a < 0.0 {
            a += std::f64::consts::TAU;
        }
        a >= from && a < to
    })
}

pub const WHITE: [u8; 4] = [255, 255, 255, 255];
pub const CLEAR: [u8; 4] = [0, 0, 0, 0];
pub const RED: [u8; 4] = [0xD0, 0x22, 0x2A, 255];
pub const BLUE: [u8; 4] = [0x23, 0x4E, 0xB8, 255];
pub const YELLOW: [u8; 4] = [0xF2, 0xC2, 0x0E, 255];
pub const GREEN: [u8; 4] = [0x2A, 0x9A, 0x4B, 255];
pub const PALE: [u8; 4] = [0xFF, 0xF2, 0xA8, 255];

/// The slice M fixtures.
pub fn multi_set() -> Vec<MultiScene> {
    let three = |name, background, jpeg| MultiScene {
        name,
        width: 600,
        height: 400,
        background,
        layers: vec![
            (rect(40.3, 40.6, 300.4, 360.2), RED),
            (rect(300.4, 40.6, 560.7, 200.5), BLUE),
            (rect(300.4, 200.5, 560.7, 360.2), YELLOW),
        ],
        jpeg,
        colours: 3,
        count: None,
    };
    let mut pale = three("pale-accent", WHITE, None);
    // 0.2 % of 600 x 400 px: 480 px, a 21.9 px square inside the blue.
    pale.layers.push((rect(420.2, 100.3, 442.1, 122.2), PALE));
    pale.colours = 4;
    let wedge = std::f64::consts::TAU / 8.0;
    let eight: [[u8; 4]; 8] = [
        RED,
        BLUE,
        YELLOW,
        GREEN,
        [0x7A, 0x3C, 0x9E, 255],
        [0xF0, 0x7C, 0x22, 255],
        [0x1B, 0xA8, 0xB0, 255],
        [0x55, 0x55, 0x55, 255],
    ];
    vec![
        three("adjacent-straight", WHITE, None),
        MultiScene {
            name: "adjacent-curved",
            width: 500,
            height: 400,
            background: WHITE,
            layers: vec![
                (rect(30.2, 120.4, 470.6, 370.3), YELLOW),
                (disc(250.3, 140.7, 110.2), RED),
                (disc(250.3, 140.7, 45.6), BLUE),
            ],
            jpeg: None,
            colours: 3,
            count: None,
        },
        MultiScene {
            name: "enclosed",
            width: 400,
            height: 400,
            background: WHITE,
            layers: vec![
                (rect(40.5, 40.5, 360.5, 360.5), RED),
                (disc(200.2, 200.4, 120.3), GREEN),
                (ring(200.2, 200.4, 70.1, 40.3), RED),
                (disc(200.2, 200.4, 20.2), BLUE),
            ],
            jpeg: None,
            colours: 3,
            count: None,
        },
        pale,
        MultiScene {
            name: "near-equal",
            width: 500,
            height: 300,
            background: WHITE,
            layers: vec![
                (rect(30.3, 30.4, 250.2, 270.6), [0xD9, 0xD9, 0xD9, 255]),
                (rect(250.2, 30.4, 470.7, 270.6), [0xDC, 0xD6, 0xCF, 255]),
                // 0.02 apart: traced as one colour.
                (rect(60.1, 60.2, 200.4, 240.3), [0x60, 0x60, 0x60, 255]),
                (rect(100.6, 60.2, 200.4, 240.3), [0x64, 0x62, 0x60, 255]),
            ],
            jpeg: None,
            colours: 3,
            count: None,
        },
        three("transparent", CLEAR, None),
        three("logo3-aa-jpeg75", WHITE, Some(75)),
        MultiScene {
            name: "gradient-steps",
            width: 512,
            height: 200,
            background: WHITE,
            layers: (0..64)
                .map(|k: i32| {
                    let v = (k * 3) as u8;
                    (
                        rect(
                            64.0 + 6.0 * f64::from(k),
                            40.0,
                            70.0 + 6.0 * f64::from(k),
                            160.0,
                        ),
                        [v, v, v, 255],
                    )
                })
                .collect(),
            jpeg: None,
            colours: 4,
            count: Some(4),
        },
        MultiScene {
            name: "junctions-8",
            width: 400,
            height: 400,
            background: WHITE,
            layers: (0..8)
                .map(|k| {
                    (
                        sector(
                            200.3,
                            200.6,
                            170.0,
                            k as f64 * wedge,
                            (k + 1) as f64 * wedge,
                        ),
                        eight[k],
                    )
                })
                .collect(),
            jpeg: None,
            colours: 8,
            count: None,
        },
    ]
}

/// The metrics of a colour trace against its scene's truth.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MultiMetrics {
    pub colours: usize,
    pub layers: usize,
    /// Share of the interior truth samples covered by the layer of their own colour (nearest
    /// layer colour to the truth colour).
    pub assigned: f64,
    /// Share of all samples covered by two layers or more.
    pub overlap: f64,
    /// Share of all samples inside the content (every sample of the pixel and of its eight
    /// neighbours is a layer's) covered by no layer.
    pub gap: f64,
    /// Segments found reversed in another layer, and groups of equal segments that are not one
    /// pair of opposite directions in two layers.
    pub shared: usize,
    pub bad_shared: usize,
    pub paths: usize,
    pub nodes: usize,
}

fn dist(a: [u8; 4], b: Rgb8) -> f64 {
    let d = [
        f64::from(a[0]) - f64::from(b.r),
        f64::from(a[1]) - f64::from(b.g),
        f64::from(a[2]) - f64::from(b.b),
    ];
    (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()
}

/// The segments of a path as point tuples: (start, c1, c2, end), lines with their ends as
/// handles.
fn segments(path: &BezPath) -> Vec<[Point; 4]> {
    let mut out = Vec::new();
    let (mut start, mut last) = (Point::ZERO, Point::ZERO);
    for el in path.elements() {
        match *el {
            PathEl::MoveTo(p) => {
                start = p;
                last = p;
            }
            PathEl::LineTo(p) => {
                out.push([last, last, p, p]);
                last = p;
            }
            PathEl::CurveTo(a, b, p) => {
                out.push([last, a, b, p]);
                last = p;
            }
            PathEl::QuadTo(_, p) => last = p,
            PathEl::ClosePath => {
                if last != start {
                    out.push([last, last, start, start]);
                }
                last = start;
            }
        }
    }
    out
}

fn key(s: &[Point; 4]) -> [u64; 8] {
    let mut k = [0; 8];
    for (i, p) in s.iter().enumerate() {
        k[2 * i] = p.x.to_bits();
        k[2 * i + 1] = p.y.to_bits();
    }
    k
}

/// Measures `result` (traced with the identity map, mm = px, scaled by `mm_per_px`) against
/// `scene` and its drawing.
pub fn measure(
    scene: &MultiScene,
    drawn: &MultiDrawn,
    result: &ColourTrace,
    mm_per_px: f64,
) -> MultiMetrics {
    let (w4, h4) = (scene.width * 4, scene.height * 4);
    let n = w4 * h4;
    let covers: Vec<Vec<bool>> = result
        .layers
        .iter()
        .map(|l| fill_even_odd(&l.outlines, 1.0 / mm_per_px, w4, h4))
        .collect();
    // The layer each truth layer should be: the one of the nearest colour.
    let truth_layer: Vec<Option<usize>> = scene
        .layers
        .iter()
        .map(|(_, c)| {
            (0..result.layers.len()).min_by(|&a, &b| {
                dist(*c, result.layers[a].sample).total_cmp(&dist(*c, result.layers[b].sample))
            })
        })
        .collect();
    let labels = &drawn.labels_4x;
    // A pixel is pure when its 16 samples share one label.
    let (w, h) = (scene.width, scene.height);
    let pixel_label = |x: usize, y: usize| -> Option<u8> {
        let first = labels[(y * 4) * w4 + x * 4];
        for j in 0..4 {
            for i in 0..4 {
                if labels[(y * 4 + j) * w4 + x * 4 + i] != first {
                    return None;
                }
            }
        }
        Some(first)
    };
    let pure: Vec<Option<u8>> = (0..w * h).map(|i| pixel_label(i % w, i / w)).collect();
    let interior = |x: usize, y: usize, want_any_layer: bool| -> Option<u8> {
        let centre = pure[y * w + x]?;
        for yy in y.saturating_sub(1)..(y + 2).min(h) {
            for xx in x.saturating_sub(1)..(x + 2).min(w) {
                let p = pure[yy * w + xx]?;
                if want_any_layer {
                    if p == 0 {
                        return None;
                    }
                } else if p != centre {
                    return None;
                }
            }
        }
        if want_any_layer && centre == 0 {
            None
        } else {
            Some(centre)
        }
    };
    let (mut inside, mut right, mut overlap, mut gap) = (0_u64, 0_u64, 0_u64, 0_u64);
    for s in 0..n {
        let count = covers.iter().filter(|c| c[s]).count();
        if count >= 2 {
            overlap += 1;
        }
        let (x, y) = ((s % w4) / 4, (s / w4) / 4);
        if count == 0 && interior(x, y, true).is_some() {
            gap += 1;
        }
        if let Some(label) = interior(x, y, false)
            && label != 0
        {
            inside += 1;
            if let Some(l) = truth_layer[label as usize - 1]
                && covers[l][s]
            {
                right += 1;
            }
        }
    }
    // Shared boundaries: every segment, grouped by its geometry in either direction.
    let mut all: Vec<([u64; 8], bool, usize)> = Vec::new();
    for (k, layer) in result.layers.iter().enumerate() {
        for path in &layer.outlines {
            for s in segments(path) {
                let fwd = key(&s);
                let rev = key(&[s[3], s[2], s[1], s[0]]);
                if fwd <= rev {
                    all.push((fwd, true, k));
                } else {
                    all.push((rev, false, k));
                }
            }
        }
    }
    all.sort_unstable();
    let (mut shared, mut bad) = (0, 0);
    let mut i = 0;
    while i < all.len() {
        let mut j = i;
        while j < all.len() && all[j].0 == all[i].0 {
            j += 1;
        }
        match &all[i..j] {
            [_] => {}
            [a, b] if a.1 != b.1 && a.2 != b.2 => shared += 2,
            _ => bad += j - i,
        }
        i = j;
    }
    MultiMetrics {
        colours: result.palette.len(),
        layers: result.layers.len(),
        assigned: right as f64 / inside.max(1) as f64,
        overlap: overlap as f64 / n as f64,
        gap: gap as f64 / n as f64,
        shared,
        bad_shared: bad,
        paths: result.counts.paths,
        nodes: result.counts.nodes,
    }
}

/// The nodes (segment ends) that every one of `layers` has, exactly.
pub fn common_nodes(result: &ColourTrace, layers: &[usize]) -> Vec<Point> {
    let nodes = |k: usize| -> Vec<(u64, u64)> {
        let mut v: Vec<(u64, u64)> = result.layers[k]
            .outlines
            .iter()
            .flat_map(segments)
            .map(|s| (s[0].x.to_bits(), s[0].y.to_bits()))
            .collect();
        v.sort_unstable();
        v.dedup();
        v
    };
    let mut common = nodes(layers[0]);
    for &k in &layers[1..] {
        let other = nodes(k);
        common.retain(|p| other.binary_search(p).is_ok());
    }
    common
        .into_iter()
        .map(|(x, y)| Point::new(f64::from_bits(x), f64::from_bits(y)))
        .collect()
}
