//! Shared helpers of the tracer's integration tests: the fixture files (binary PGM and PAM, read
//! and written here so the frontend's `npm test` can read the same bytes), the anti-aliased
//! renderer that made them, and the golden files.

#![allow(dead_code)]

pub mod quality;

use std::ops::ControlFlow;
use std::path::PathBuf;

use curvyo_document_core::{Affine, Length};
use curvyo_vectorize_core::{Raster, TraceParams, TraceResult, trace};

/// Millimetres per pixel at 96 dpi.
pub const MM_PER_PX_96: f64 = 25.4 / 96.0;

/// A picture as RGBA8 with straight alpha.
#[derive(Debug, Clone, PartialEq)]
pub struct Picture {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl Picture {
    /// The picture of grey `levels`, opaque.
    pub fn grey(width: u32, height: u32, levels: &[u8]) -> Self {
        Self {
            width,
            height,
            rgba: levels.iter().flat_map(|&v| [v, v, v, 255]).collect(),
        }
    }

    pub fn raster(&self) -> Raster<'_> {
        Raster {
            width: self.width,
            height: self.height,
            rgba: &self.rgba,
        }
    }

    /// The grey levels (the red channel; fixtures in PGM are grey).
    pub fn levels(&self) -> Vec<u8> {
        self.rgba.as_chunks::<4>().0.iter().map(|p| p[0]).collect()
    }
}

/// Black shapes on white: each pixel's grey is the share of `samples` x `samples` points inside.
pub fn render(width: u32, height: u32, samples: u32, inside: impl Fn(f64, f64) -> bool) -> Picture {
    render_with_truth(width, height, samples, inside).0
}

/// [`render`] and the drawn shape at 4 x 4 samples per pixel, row by row of the 4x grid (the
/// samples themselves for 4, each pixel's centre four times over for 1; empty otherwise).
pub fn render_with_truth(
    width: u32,
    height: u32,
    samples: u32,
    inside: impl Fn(f64, f64) -> bool,
) -> (Picture, Vec<bool>) {
    let mut levels = Vec::with_capacity((width * height) as usize);
    let keep = matches!(samples, 1 | 4);
    let w4 = width as usize * 4;
    let mut truth = if keep {
        vec![false; w4 * height as usize * 4]
    } else {
        Vec::new()
    };
    let n = f64::from(samples);
    for y in 0..height {
        for x in 0..width {
            let mut hits = 0_u32;
            for sy in 0..samples {
                for sx in 0..samples {
                    let px = f64::from(x) + (f64::from(sx) + 0.5) / n;
                    let py = f64::from(y) + (f64::from(sy) + 0.5) / n;
                    if inside(px, py) {
                        hits += 1;
                        if keep {
                            let r = 4 / samples;
                            for dy in 0..r {
                                for dx in 0..r {
                                    let row = (y * 4 + sy * r + dy) as usize;
                                    truth[row * w4 + (x * 4 + sx * r + dx) as usize] = true;
                                }
                            }
                        }
                    }
                }
            }
            let cover = f64::from(hits) / (n * n);
            levels.push((255.0 * (1.0 - cover)).round() as u8);
        }
    }
    (Picture::grey(width, height, &levels), truth)
}

pub fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/trace")
}

/// Writes `picture` as binary PGM (grey) or, with `alpha`, PAM (`RGB_ALPHA`).
pub fn write_picture(name: &str, picture: &Picture, alpha: bool) {
    let mut bytes = if alpha {
        format!(
            "P7\nWIDTH {}\nHEIGHT {}\nDEPTH 4\nMAXVAL 255\nTUPLTYPE RGB_ALPHA\nENDHDR\n",
            picture.width, picture.height
        )
        .into_bytes()
    } else {
        format!("P5\n{} {}\n255\n", picture.width, picture.height).into_bytes()
    };
    if alpha {
        bytes.extend_from_slice(&picture.rgba);
    } else {
        bytes.extend(picture.levels());
    }
    std::fs::write(fixture_dir().join(name), bytes).unwrap();
}

/// Reads a fixture written by [`write_picture`].
pub fn read_picture(name: &str) -> Picture {
    let bytes = std::fs::read(fixture_dir().join(name)).unwrap();
    if bytes.starts_with(b"P7\n") {
        let end = bytes.windows(7).position(|w| w == b"ENDHDR\n").unwrap() + 7;
        let header = std::str::from_utf8(&bytes[..end]).unwrap();
        let field = |key: &str| -> u32 {
            header
                .lines()
                .find_map(|l| l.strip_prefix(key))
                .unwrap()
                .trim()
                .parse()
                .unwrap()
        };
        let (width, height) = (field("WIDTH "), field("HEIGHT "));
        return Picture {
            width,
            height,
            rgba: bytes[end..].to_vec(),
        };
    }
    let text = String::from_utf8_lossy(&bytes[..bytes.len().min(20)]);
    let mut fields = text.split_ascii_whitespace();
    assert_eq!(fields.next(), Some("P5"));
    let width: u32 = fields.next().unwrap().parse().unwrap();
    let height: u32 = fields.next().unwrap().parse().unwrap();
    let start = bytes.len() - (width * height) as usize;
    Picture::grey(width, height, &bytes[start..])
}

/// The settings of a test: the given Threshold, Smooth and Specks.
pub fn params(threshold: u8, smooth: u8, specks_mm: f64) -> TraceParams {
    TraceParams {
        threshold,
        smooth,
        specks: Length::from_mm(specks_mm),
        ..TraceParams::with_threshold(threshold)
    }
}

/// Traces without stopping.
pub fn run(picture: &Picture, map: Affine, params: &TraceParams) -> TraceResult {
    let mut go = |_| ControlFlow::Continue(());
    trace(picture.raster(), map, params, &mut go).unwrap()
}

/// The map of a picture placed at 96 dpi at the origin.
pub fn at_96_dpi() -> Affine {
    Affine::scale(MM_PER_PX_96, MM_PER_PX_96)
}

/// The signed area of a closed path (kurbo's sign: positive is counter-clockwise in y-up, which
/// is clockwise on the y-down canvas).
pub fn area(path: &kurbo::BezPath) -> f64 {
    kurbo::Shape::area(path)
}

/// Thread CPU time in seconds (`CLAUDE.md` §5: timing tests never use wall time).
#[cfg(unix)]
pub fn thread_cpu_seconds() -> f64 {
    let t = rustix::time::clock_gettime(rustix::time::ClockId::ThreadCPUTime);
    t.tv_sec as f64 + t.tv_nsec as f64 * 1e-9
}
