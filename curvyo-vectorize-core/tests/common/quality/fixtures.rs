//! The black and white quality set of `specs/0025-raster-trace` (criteria 27, 28; `research.md`
//! section 5). The pictures are synthetic, drawn here from fixed shapes and a fixed
//! noise sequence, so they need no copyright note and no binary in the repository; the
//! `#[ignore]` writer puts them in `target/trace-quality/` for the offline baselines.

use super::super::{Picture, render_with_truth};
use super::metrics::to_segment;

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
    /// The drawn shape at 4 x 4 samples per pixel, before any noise (`adrs.md` 2026-10-28,
    /// second note, point 2); `None` for the photograph.
    pub truth_4x: Option<Vec<bool>>,
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

/// Image 4 of the set, small low-resolution text (120 x 60 px), drawn `scale` times larger (the
/// 4x reference of criterion 42).
pub fn small_text(scale: u32) -> Picture {
    small_text_with_truth(scale).0
}

/// [`small_text`] and its drawn shape at 4 x 4 samples per pixel.
fn small_text_with_truth(scale: u32) -> (Picture, Vec<bool>) {
    let s = f64::from(scale);
    let strokes = text_strokes("TRACE", 4.0, 6.0, 1.6);
    let strokes2 = text_strokes("CURVY", 4.0, 30.0, 1.6);
    render_with_truth(120 * scale, 60 * scale, 4, move |x, y| {
        let p = (x / s, y / s);
        near_strokes(p, &strokes, 0.9) || near_strokes(p, &strokes2, 0.9)
    })
}

/// The black and white set: images 1, 3, 4, 5, 7, 10, 11 and 12 of `research.md` section 5.
pub fn quality_set() -> Vec<QualityImage> {
    let logo = {
        let star = star(420.0, 190.0, 140.0, 60.0, 5);
        render_with_truth(600, 400, 4, move |x, y| {
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
        render_with_truth(900, 300, 4, move |x, y| near_strokes((x, y), &strokes, 4.0))
    };
    let small = small_text_with_truth(1);
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
        let drawn = render_with_truth(600, 600, 4, move |x, y| {
            let (sx, sy) = (
                x * skew.cos() + y * skew.sin(),
                -x * skew.sin() + y * skew.cos(),
            );
            near_strokes((sx, sy), &strokes, 2.5)
        });
        (with_noise(drawn.0, 14.0, 5), drawn.1)
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
        let drawn = render_with_truth(600, 600, 4, move |x, y| {
            let inside = blobs
                .iter()
                .any(|&(cx, cy, r)| (x - cx).powi(2) + (y - cy).powi(2) <= r * r);
            inside
                && !holes
                    .iter()
                    .any(|&(cx, cy, r)| (x - cx).powi(2) + (y - cy).powi(2) <= r * r)
        });
        (with_noise(drawn.0, 18.0, 11), drawn.1)
    };
    let pixel_art = {
        let mut noise = Noise::new(3);
        let cells: Vec<bool> = (0..16 * 8).map(|_| noise.next() > 0.1).collect();
        render_with_truth(256, 256, 1, move |x, y| {
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
        render_with_truth(600, 600, 4, move |x, y| {
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
        render_with_truth(800, 600, 4, move |x, y| {
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
            picture: logo.0,
            truth_4x: Some(logo.1),
            dpi: 96.0,
            kind: Kind::LogoOrText,
        },
        QualityImage {
            name: "03-text-300dpi",
            picture: text.0,
            truth_4x: Some(text.1),
            dpi: 300.0,
            kind: Kind::LogoOrText,
        },
        QualityImage {
            name: "04-small-text",
            picture: small.0,
            truth_4x: Some(small.1),
            dpi: 72.0,
            kind: Kind::LogoOrText,
        },
        QualityImage {
            name: "05-line-art-scan",
            picture: line_art.0,
            truth_4x: Some(line_art.1),
            dpi: 150.0,
            kind: Kind::Other,
        },
        QualityImage {
            name: "07-silhouette",
            picture: silhouette.0,
            truth_4x: Some(silhouette.1),
            dpi: 150.0,
            kind: Kind::Other,
        },
        QualityImage {
            name: "10-pixel-art",
            picture: pixel_art.0,
            truth_4x: Some(pixel_art.1),
            dpi: 96.0,
            kind: Kind::Other,
        },
        QualityImage {
            name: "11-stencil-lines",
            picture: stencil.0,
            truth_4x: Some(stencil.1),
            dpi: 300.0,
            kind: Kind::Other,
        },
        QualityImage {
            name: "12-technical-drawing",
            picture: technical.0,
            truth_4x: Some(technical.1),
            dpi: 150.0,
            kind: Kind::Other,
        },
    ]
}

/// The photograph fixture of criterion 61: a synthetic street-scene-like photo (400 x 300 px at
/// 96 dpi): a sky gradient, soft-edged buildings and trees, a figure, fine sensor noise of 12
/// grey levels and grain. Generated here from fixed shapes and a fixed noise sequence, so it has
/// no origin to credit and no copyright.
pub fn photograph() -> Picture {
    let (width, height) = (400_u32, 300_u32);
    let mut noise = Noise::new(2026);
    let soft = |d: f64| (0.5 - d / 3.0).clamp(0.0, 1.0);
    let blobs = [
        (90.0, 120.0, 45.0),
        (130.0, 150.0, 38.0),
        (320.0, 130.0, 55.0),
        (290.0, 170.0, 35.0),
    ];
    let mut levels = Vec::with_capacity((width * height) as usize);
    for y in 0..height {
        for x in 0..width {
            let (fx, fy) = (f64::from(x) + 0.5, f64::from(y) + 0.5);
            // Sky to ground: light at the top, mid grey at the bottom, with clouds.
            let mut v = 225.0 - 90.0 * fy / f64::from(height)
                + 18.0 * (fx / 23.0).sin() * (fy / 31.0).cos();
            // Buildings: dark blocks with lit windows.
            for &(x0, x1, top, grey) in &[
                (150.0, 210.0, 90.0, 70.0),
                (215.0, 260.0, 60.0, 95.0),
                (20.0, 60.0, 140.0, 85.0),
            ] {
                let inside = soft(x0 - fx).min(soft(fx - x1)).min(soft(top - fy));
                let window = (fx - x0) % 12.0 < 5.0 && (fy - top) % 14.0 < 6.0;
                let tone = if window { grey + 90.0 } else { grey };
                v = v * (1.0 - inside) + tone * inside;
            }
            // Trees: dark round crowns with leafy texture.
            for &(cx, cy, r) in &blobs {
                let d = ((fx - cx).powi(2) + (fy - cy).powi(2)).sqrt();
                let inside = soft(d - r);
                let leaves = 60.0 + 35.0 * ((fx * 0.9).sin() * (fy * 1.1).cos());
                v = v * (1.0 - inside) + leaves * inside;
            }
            // A figure: head and body.
            let head = ((fx - 280.0).powi(2) + (fy - 205.0).powi(2)).sqrt();
            let body = soft((fx - 280.0).abs() - 12.0)
                .min(soft(215.0 - fy))
                .min(soft(fy - 275.0));
            let person = soft(head - 9.0).max(body);
            v = v * (1.0 - person) + 40.0 * person;
            // Sensor noise and grain.
            v += 12.0 * noise.next();
            if noise.next() > 0.98 {
                v -= 60.0 * noise.next().abs();
            }
            levels.push(v.round().clamp(0.0, 255.0) as u8);
        }
    }
    Picture::grey(width, height, &levels)
}
