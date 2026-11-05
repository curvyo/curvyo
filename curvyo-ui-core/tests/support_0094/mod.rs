//! The tester's synthetic fixtures of `specs/0094-masked-composite-trace` (M1, M2 and their
//! colour and text variants), drawn at any resolution so that tests stay fast. Each pixel is
//! sampled at its centre in M1's own 1000 x 1000 px frame.

#![allow(
    dead_code,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation
)]

/// What a pixel of M1 shows, at M1 coordinates `u`, `v` (0..1000).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ink {
    Paper,
    Disc,
    Square,
    Bar,
    Grey,
}

/// M1's content at `u`, `v`.
#[must_use]
pub fn m1_ink(u: f64, v: f64) -> Ink {
    if (u - 900.0).abs() <= 50.0 && (v - 100.0).abs() <= 50.0 {
        Ink::Grey
    } else if (u - 250.0).powi(2) + (v - 500.0).powi(2) <= 150.0 * 150.0 {
        Ink::Disc
    } else if (u - 750.0).abs() <= 150.0 && (v - 500.0).abs() <= 150.0 {
        Ink::Square
    } else if (480.0..=520.0).contains(&v) {
        Ink::Bar
    } else {
        Ink::Paper
    }
}

/// The colours of one variant.
#[derive(Debug, Clone, Copy)]
pub struct Palette {
    pub paper_left: [u8; 3],
    pub paper_right: [u8; 3],
    pub disc: [u8; 3],
    pub square: [u8; 3],
    pub bar: [u8; 3],
    pub grey: [u8; 3],
}

/// M1: black disc, square and bar, a #808080 square, white paper.
pub const M1: Palette = Palette {
    paper_left: [255, 255, 255],
    paper_right: [255, 255, 255],
    disc: [0, 0, 0],
    square: [0, 0, 0],
    bar: [0, 0, 0],
    grey: [128, 128, 128],
};

/// M2: M1 with the right half's paper #D8D8D8.
pub const M2: Palette = Palette {
    paper_right: [0xD8, 0xD8, 0xD8],
    ..M1
};

/// A colour variant for Multicolour: red disc, blue square, green bar, grey square.
pub const COLOUR: Palette = Palette {
    paper_left: [255, 255, 255],
    paper_right: [255, 255, 255],
    disc: [200, 30, 30],
    square: [30, 60, 200],
    bar: [30, 160, 60],
    grey: [128, 128, 128],
};

/// `palette` drawn `n` x `n` px, straight-alpha RGBA.
#[must_use]
pub fn draw(palette: Palette, n: u32) -> Vec<u8> {
    let scale = 1000.0 / f64::from(n);
    let mut rgba = Vec::with_capacity((n * n * 4) as usize);
    for y in 0..n {
        for x in 0..n {
            let u = (f64::from(x) + 0.5) * scale;
            let v = (f64::from(y) + 0.5) * scale;
            let c = match m1_ink(u, v) {
                Ink::Paper if u >= 500.0 => palette.paper_right,
                Ink::Paper => palette.paper_left,
                Ink::Disc => palette.disc,
                Ink::Square => palette.square,
                Ink::Bar => palette.bar,
                Ink::Grey => palette.grey,
            };
            rgba.extend([c[0], c[1], c[2], 255]);
        }
    }
    rgba
}

/// The pixel map that places an `n` px picture as 100 x 100 mm at the origin (10 px per mm at
/// 1000 px).
#[must_use]
pub fn map_100mm(n: u32) -> [f64; 6] {
    let s = 100.0 / f64::from(n);
    [s, 0.0, 0.0, s, 0.0, 0.0]
}

/// A text sheet: `n` x `n` px white with black "glyph" blocks in two lines, one in the top
/// quarter (y 15..25 % of the height) and one in the bottom quarter (75..85 %), and a third line
/// that straddles the middle (45..55 %). Each line is 5 blocks from x 20 % to 80 %.
#[must_use]
pub fn text_sheet(n: u32) -> Vec<u8> {
    let nf = f64::from(n);
    let mut rgba = Vec::with_capacity((n * n * 4) as usize);
    for y in 0..n {
        for x in 0..n {
            let fx = (f64::from(x) + 0.5) / nf;
            let fy = (f64::from(y) + 0.5) / nf;
            let in_line = [0.15, 0.45, 0.75]
                .iter()
                .any(|&top| fy >= top && fy < top + 0.10);
            let block = (0.2..0.8).contains(&fx) && ((fx - 0.2) * 100.0) as i64 % 12 < 9;
            let c = if in_line && block { 0 } else { 255 };
            rgba.extend([c, c, c, 255]);
        }
    }
    rgba
}

/// The quads (picture pixels) of the three lines of [`text_sheet`]: top, middle, bottom.
#[must_use]
pub fn text_quads(n: u32) -> [[[f64; 2]; 4]; 3] {
    let nf = f64::from(n);
    let quad = |top: f64| {
        let (x0, x1) = (0.18 * nf, 0.82 * nf);
        let (y0, y1) = ((top - 0.02) * nf, (top + 0.12) * nf);
        [[x0, y0], [x1, y0], [x1, y1], [x0, y1]]
    };
    [quad(0.15), quad(0.45), quad(0.75)]
}

/// Base64 of `bytes` (standard alphabet, padded), for SVG data URIs.
#[must_use]
pub fn base64(bytes: &[u8]) -> String {
    const ABC: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(ABC[((n >> (18 - 6 * i)) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}
