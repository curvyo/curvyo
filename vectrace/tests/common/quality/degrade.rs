//! Capture artefacts applied to a drawn reference picture (`specs/0089-trace-quality-and-colour`
//! criterion 2), each a fixed computation with a fixed noise sequence, the same on every machine.

use super::super::Picture;
use super::fixtures::Noise;

/// One artefact.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Artefact {
    /// Baseline JPEG at quality 50: 4:2:0 chroma and the standard quantisation tables (the
    /// entropy coding is lossless and left out).
    Jpeg50,
    /// Grey noise of `amplitude` levels on every pixel and dark dust specks.
    ScanNoise { amplitude: f64, seed: u64 },
    /// Cream paper with a fibre texture.
    PaperTexture { seed: u64 },
    /// A soft cast shadow over the right part.
    Shadow,
    /// Light falling off from the top left to the bottom right.
    Lighting,
}

impl Artefact {
    /// Applies the artefact to `picture` drawn at `scale` (texture sizes follow the scale).
    pub fn apply(self, picture: &mut Picture, scale: f64) {
        match self {
            Self::Jpeg50 => jpeg(picture, 50),
            Self::ScanNoise { amplitude, seed } => scan_noise(picture, amplitude, seed),
            Self::PaperTexture { seed } => paper_texture(picture, scale, seed),
            Self::Shadow => shade(picture, |x, y| {
                // Distance past the line from (0.55, 0) to (1.0, 0.75), soft over 0.08.
                let d = ((x - 0.55) * 0.75 - y * 0.45) / (0.75_f64.hypot(0.45));
                1.0 - 0.5 * smoothstep(-0.04, 0.04, d)
            }),
            Self::Lighting => shade(picture, |x, y| 1.0 - 0.45 * (0.6 * x + 0.4 * y)),
        }
    }
}

fn smoothstep(a: f64, b: f64, x: f64) -> f64 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn byte(v: f64) -> u8 {
    v.round().clamp(0.0, 255.0) as u8
}

/// Multiplies every pixel's colour by `factor(x / width, y / height)`.
fn shade(picture: &mut Picture, factor: impl Fn(f64, f64) -> f64) {
    let (w, h) = (picture.width as usize, picture.height as usize);
    for (i, px) in picture.rgba.as_chunks_mut::<4>().0.iter_mut().enumerate() {
        let f = factor(
            ((i % w) as f64 + 0.5) / w as f64,
            ((i / w) as f64 + 0.5) / h as f64,
        );
        for c in &mut px[..3] {
            *c = byte(f64::from(*c) * f);
        }
    }
}

fn scan_noise(picture: &mut Picture, amplitude: f64, seed: u64) {
    let mut noise = Noise::new(seed);
    for px in picture.rgba.as_chunks_mut::<4>().0 {
        let n = amplitude * noise.next();
        for c in &mut px[..3] {
            *c = byte(f64::from(*c) + n);
        }
    }
    let (w, h) = (picture.width as usize, picture.height as usize);
    let specks = (w * h) / 4000;
    for _ in 0..specks {
        let cx = f64::midpoint(noise.next(), 1.0) * w as f64;
        let cy = f64::midpoint(noise.next(), 1.0) * h as f64;
        let r = 0.5 + noise.next().abs();
        for y in (cy - r).floor().max(0.0) as usize..((cy + r).ceil() as usize).min(h) {
            for x in (cx - r).floor().max(0.0) as usize..((cx + r).ceil() as usize).min(w) {
                if (x as f64 + 0.5 - cx).hypot(y as f64 + 0.5 - cy) <= r {
                    let i = (y * w + x) * 4;
                    for c in &mut picture.rgba[i..i + 3] {
                        *c = (*c).min(60);
                    }
                }
            }
        }
    }
}

fn paper_texture(picture: &mut Picture, scale: f64, seed: u64) {
    let (w, h) = (picture.width as usize, picture.height as usize);
    let cell = (6.0 * scale).max(1.0);
    let (gw, gh) = (
        (w as f64 / cell) as usize + 2,
        (h as f64 / cell) as usize + 2,
    );
    let mut noise = Noise::new(seed);
    let grid: Vec<f64> = (0..gw * gh).map(|_| noise.next()).collect();
    let fibres: Vec<(f64, f64, f64)> = (0..(w * h) / 3000)
        .map(|_| {
            (
                f64::midpoint(noise.next(), 1.0) * w as f64,
                f64::midpoint(noise.next(), 1.0) * h as f64,
                noise.next() * std::f64::consts::PI,
            )
        })
        .collect();
    let tint = [0.97, 0.95, 0.90];
    for y in 0..h {
        for x in 0..w {
            let (gx, gy) = (x as f64 / cell, y as f64 / cell);
            let (ix, iy) = (gx as usize, gy as usize);
            let (fx, fy) = (gx - ix as f64, gy - iy as f64);
            let at = |i: usize, j: usize| grid[j * gw + i];
            let v = at(ix, iy) * (1.0 - fx) * (1.0 - fy)
                + at(ix + 1, iy) * fx * (1.0 - fy)
                + at(ix, iy + 1) * (1.0 - fx) * fy
                + at(ix + 1, iy + 1) * fx * fy;
            let mut f = 1.0 - 0.04 * (v + 1.0) - 0.015 * noise.next().abs();
            for &(cx, cy, a) in &fibres {
                let (dx, dy) = (x as f64 - cx, y as f64 - cy);
                let along = dx * a.cos() + dy * a.sin();
                let across = -dx * a.sin() + dy * a.cos();
                if along.abs() < 12.0 * scale && across.abs() < 0.6 {
                    f -= 0.06;
                }
            }
            let i = (y * w + x) * 4;
            for k in 0..3 {
                picture.rgba[i + k] = byte(f64::from(picture.rgba[i + k]) * f * tint[k]);
            }
        }
    }
}

/// The standard luminance quantisation table (ITU T.81 Annex K), the quality 50 table.
const LUMA_Q: [f64; 64] = [
    16., 11., 10., 16., 24., 40., 51., 61., 12., 12., 14., 19., 26., 58., 60., 55., 14., 13., 16.,
    24., 40., 57., 69., 56., 14., 17., 22., 29., 51., 87., 80., 62., 18., 22., 37., 56., 68., 109.,
    103., 77., 24., 35., 55., 64., 81., 104., 113., 92., 49., 64., 78., 87., 103., 121., 120.,
    101., 72., 92., 95., 98., 112., 100., 103., 99.,
];

/// The standard chrominance quantisation table (ITU T.81 Annex K).
const CHROMA_Q: [f64; 64] = [
    17., 18., 24., 47., 99., 99., 99., 99., 18., 21., 26., 66., 99., 99., 99., 99., 24., 26., 56.,
    99., 99., 99., 99., 99., 47., 66., 99., 99., 99., 99., 99., 99., 99., 99., 99., 99., 99., 99.,
    99., 99., 99., 99., 99., 99., 99., 99., 99., 99., 99., 99., 99., 99., 99., 99., 99., 99., 99.,
    99., 99., 99., 99., 99., 99., 99.,
];

/// `table` (the quality 50 table) scaled to JPEG `quality` (the IJG rule).
fn scaled(table: &[f64; 64], quality: u32) -> [f64; 64] {
    let q = quality.clamp(1, 100);
    let s = if q < 50 { 5000 / q } else { 200 - 2 * q };
    table.map(|v| ((v * f64::from(s) + 50.0) / 100.0).floor().max(1.0))
}

/// Quantises every 8 x 8 block of `plane` (`w` x `h`, edge pixels repeated) with `table`.
fn quantise_plane(plane: &mut [f64], w: usize, h: usize, table: &[f64; 64]) {
    let basis: Vec<f64> = (0..64)
        .map(|i: i32| {
            let (u, x) = (i / 8, i % 8);
            let c = if u == 0 { (0.125_f64).sqrt() } else { 0.5 };
            c * ((2.0 * f64::from(x) + 1.0) * f64::from(u) * std::f64::consts::PI / 16.0).cos()
        })
        .collect();
    for by in (0..h).step_by(8) {
        for bx in (0..w).step_by(8) {
            let mut block = [0.0; 64];
            for y in 0..8 {
                for x in 0..8 {
                    let (sx, sy) = ((bx + x).min(w - 1), (by + y).min(h - 1));
                    block[y * 8 + x] = plane[sy * w + sx] - 128.0;
                }
            }
            let mut coeff = [0.0; 64];
            for v in 0..8 {
                for u in 0..8 {
                    let mut s = 0.0;
                    for y in 0..8 {
                        for x in 0..8 {
                            s += basis[v * 8 + y] * basis[u * 8 + x] * block[y * 8 + x];
                        }
                    }
                    let q = table[v * 8 + u];
                    coeff[v * 8 + u] = (s / q).round() * q;
                }
            }
            for y in 0..8 {
                for x in 0..8 {
                    if bx + x >= w || by + y >= h {
                        continue;
                    }
                    let mut s = 0.0;
                    for v in 0..8 {
                        for u in 0..8 {
                            s += basis[v * 8 + y] * basis[u * 8 + x] * coeff[v * 8 + u];
                        }
                    }
                    plane[(by + y) * w + bx + x] = s + 128.0;
                }
            }
        }
    }
}

/// Baseline JPEG at `quality` (4:2:0 chroma, the standard tables scaled; the entropy coding is
/// lossless and left out).
pub fn jpeg(picture: &mut Picture, quality: u32) {
    let (w, h) = (picture.width as usize, picture.height as usize);
    let (cw, ch) = (w.div_ceil(2), h.div_ceil(2));
    let mut luma = vec![0.0; w * h];
    let mut cb = vec![0.0; cw * ch];
    let mut cr = vec![0.0; cw * ch];
    for (i, px) in picture.rgba.as_chunks::<4>().0.iter().enumerate() {
        let (r, g, b) = (f64::from(px[0]), f64::from(px[1]), f64::from(px[2]));
        luma[i] = 0.299 * r + 0.587 * g + 0.114 * b;
        let c = ((i / w) / 2) * cw + (i % w) / 2;
        cb[c] += (128.0 - 0.168_736 * r - 0.331_264 * g + 0.5 * b) / 4.0;
        cr[c] += (128.0 + 0.5 * r - 0.418_688 * g - 0.081_312 * b) / 4.0;
    }
    // Odd edges: the 2 x 2 average saw fewer pixels; scale back up.
    for cy in 0..ch {
        for cx in 0..cw {
            let n = ((2 * cx + 2).min(w) - 2 * cx) * ((2 * cy + 2).min(h) - 2 * cy);
            cb[cy * cw + cx] *= 4.0 / n as f64;
            cr[cy * cw + cx] *= 4.0 / n as f64;
        }
    }
    let (luma_q, chroma_q) = (scaled(&LUMA_Q, quality), scaled(&CHROMA_Q, quality));
    quantise_plane(&mut luma, w, h, &luma_q);
    quantise_plane(&mut cb, cw, ch, &chroma_q);
    quantise_plane(&mut cr, cw, ch, &chroma_q);
    for (i, px) in picture.rgba.as_chunks_mut::<4>().0.iter_mut().enumerate() {
        let c = ((i / w) / 2) * cw + (i % w) / 2;
        let (y, b, r) = (luma[i], cb[c] - 128.0, cr[c] - 128.0);
        px[0] = byte(y + 1.402 * r);
        px[1] = byte(y - 0.344_136 * b - 0.714_136 * r);
        px[2] = byte(y + 1.772 * b);
    }
}
