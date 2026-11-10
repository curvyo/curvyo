//! The score of every pixel for Pick colours: the two-colour mixing model, the picture's noise
//! taken out, and the fringe of cheap resampling (`specs/0089` criteria 17 and 18).

use crate::params::Rgb8;
use crate::raster::Raster;
use crate::select::{colour, gate};

/// How far past the gate a pixel's own colour may be and still be tried through its
/// neighbourhood: the noise it can carry (+-6 per channel is at most 10.4 bytes off).
const NOISE_REACH: f64 = 14.0;

/// The most a channel may vary within a 5 x 5 window that still counts as flat: noise of +-8.
const FLAT_RANGE: u8 = 16;

/// Below this many pixels the lookup tables cost more than they save.
const TABLE_FROM_PIXELS: usize = 200_000;

/// The score (0 to 255) of every pixel of `raster` for the picked `swatches`: the best of the
/// pixel's own colour, the median of its 3 x 3 window and, where the 5 x 5 window is flat, its
/// mean, so noise is taken out without widening the tolerance and corners are not rounded; then
/// the fringe pass.
pub(crate) fn scores(
    raster: Raster<'_>,
    swatches: &[Rgb8],
    background: Option<Rgb8>,
    tolerance: u8,
) -> Vec<u8> {
    let (width, height) = (raster.width as usize, raster.height as usize);
    let pixels = raster.rgba.as_chunks::<4>().0;
    let picker = Picker::new(
        swatches,
        background,
        tolerance,
        pixels.len() >= TABLE_FROM_PIXELS,
    );
    let mut cache = Cache::new();
    let mut score = vec![0_u8; pixels.len()];
    for y in 0..height {
        for x in 0..width {
            let [r, g, b, _] = pixels[y * width + x];
            let mut best = picker.cached(&mut cache, [r, g, b]);
            if best < u8::MAX && picker.is_near([r, g, b]) {
                let window = Window::read(pixels, width, height, x, y);
                best = best.max(picker.cached(&mut cache, window.median));
                if let Some(mean) = window.flat_mean {
                    best = best.max(picker.cached(&mut cache, mean));
                }
            }
            score[y * width + x] = best;
        }
    }
    picker.fringe(&mut score, pixels, width);
    score
}

/// What a pixel's neighbourhood says about its colour.
struct Window {
    /// The per-channel median of the 3 x 3 window.
    median: [u8; 3],
    /// The per-channel mean of the 5 x 5 window when it is flat.
    flat_mean: Option<[u8; 3]>,
}

impl Window {
    fn read(pixels: &[[u8; 4]], width: usize, height: usize, x: usize, y: usize) -> Self {
        let mut median = [0_u8; 3];
        let mut mean = [0_u8; 3];
        let mut flat = true;
        for k in 0..3 {
            let mut inner = [0_u8; 9];
            let (mut low, mut high, mut sum) = (u8::MAX, 0_u8, 0_u32);
            for dy in 0..5 {
                let sy = (y + dy).saturating_sub(2).min(height - 1);
                for dx in 0..5 {
                    let sx = (x + dx).saturating_sub(2).min(width - 1);
                    let v = pixels[sy * width + sx][k];
                    low = low.min(v);
                    high = high.max(v);
                    sum += u32::from(v);
                    if (1..4).contains(&dx) && (1..4).contains(&dy) {
                        inner[(dy - 1) * 3 + dx - 1] = v;
                    }
                }
            }
            median[k] = median9(inner);
            // invariant: the mean of bytes is a byte.
            mean[k] = u8::try_from((sum + 12) / 25).unwrap_or(u8::MAX);
            flat &= high - low <= FLAT_RANGE;
        }
        Self {
            median,
            flat_mean: flat.then_some(mean),
        }
    }
}

/// The median of nine values by a sorting network.
fn median9(mut p: [u8; 9]) -> u8 {
    let mut order = |a: usize, b: usize| {
        if p[a] > p[b] {
            p.swap(a, b);
        }
    };
    for (a, b) in [
        (1, 2),
        (4, 5),
        (7, 8),
        (0, 1),
        (3, 4),
        (6, 7),
        (1, 2),
        (4, 5),
        (7, 8),
        (0, 3),
        (5, 8),
        (4, 7),
        (3, 6),
        (1, 4),
        (2, 5),
        (4, 7),
        (4, 2),
        (6, 4),
        (4, 2),
    ] {
        order(a, b);
    }
    p[4]
}

/// The two-colour mixing model of Pick colours (trial C1): a pixel on the segment from the
/// background to a picked colour (within the gate) belongs to it by its mixing fraction, the
/// anti-aliasing blend at its coverage; one on the segment between two picked colours belongs
/// fully (touching picks are one region, criterion 19); the score is the largest over all.
/// Without a background colour (a transparent border) a pixel belongs fully within the gate of
/// a picked colour, and the alpha carries the edges.
struct Picker {
    /// From the background to each picked colour (a point segment without a background).
    toward: Vec<Segment>,
    /// Between each pair of picked colours.
    between: Vec<Segment>,
    /// The picked colours.
    colours: Vec<[f64; 3]>,
    gate: f64,
    /// Which 8-bit colour cells can hold a colour within the gate (and within the gate plus
    /// the noise reach) of some segment; `None` for a small picture.
    tables: Option<Tables>,
}

struct Tables {
    hit: Vec<bool>,
    near: Vec<bool>,
}

/// The cell of a colour in the tables: 5 bits per channel.
fn cell(c: [u8; 3]) -> usize {
    usize::from(c[0] >> 3) << 10 | usize::from(c[1] >> 3) << 5 | usize::from(c[2] >> 3)
}

/// A segment of colours with its box widened by the gate, so most pixels are dismissed by six
/// comparisons.
struct Segment {
    from: [f64; 3],
    d: [f64; 3],
    len2: f64,
    lo: [f64; 3],
    hi: [f64; 3],
    gate2: f64,
}

impl Segment {
    fn new(from: [f64; 3], to: [f64; 3], gate: f64) -> Self {
        let d = [to[0] - from[0], to[1] - from[1], to[2] - from[2]];
        Self {
            from,
            d,
            len2: d[0] * d[0] + d[1] * d[1] + d[2] * d[2],
            lo: [0, 1, 2].map(|k| from[k].min(to[k]) - gate),
            hi: [0, 1, 2].map(|k| from[k].max(to[k]) + gate),
            gate2: gate * gate,
        }
    }

    /// The distance from `p` to the segment.
    fn distance(&self, p: [f64; 3]) -> f64 {
        let q = [
            p[0] - self.from[0],
            p[1] - self.from[1],
            p[2] - self.from[2],
        ];
        let f = if self.len2 == 0.0 {
            0.0
        } else {
            ((q[0] * self.d[0] + q[1] * self.d[1] + q[2] * self.d[2]) / self.len2).clamp(0.0, 1.0)
        };
        let off = [
            q[0] - f * self.d[0],
            q[1] - f * self.d[1],
            q[2] - f * self.d[2],
        ];
        (off[0] * off[0] + off[1] * off[1] + off[2] * off[2]).sqrt()
    }

    /// The fraction toward the segment's end of `p`, when `p` lies within the gate of it.
    fn hit(&self, p: [f64; 3]) -> Option<f64> {
        if (0..3).any(|k| p[k] < self.lo[k] || p[k] > self.hi[k]) {
            return None;
        }
        let q = [
            p[0] - self.from[0],
            p[1] - self.from[1],
            p[2] - self.from[2],
        ];
        let f = if self.len2 == 0.0 {
            1.0
        } else {
            ((q[0] * self.d[0] + q[1] * self.d[1] + q[2] * self.d[2]) / self.len2).clamp(0.0, 1.0)
        };
        let off = [
            q[0] - f * self.d[0],
            q[1] - f * self.d[1],
            q[2] - f * self.d[2],
        ];
        (off[0] * off[0] + off[1] * off[1] + off[2] * off[2] <= self.gate2).then_some(f)
    }
}

impl Picker {
    fn new(swatches: &[Rgb8], background: Option<Rgb8>, tolerance: u8, tabled: bool) -> Self {
        let gate = gate(tolerance);
        let colours: Vec<[f64; 3]> = swatches.iter().map(|c| colour(c.r, c.g, c.b)).collect();
        let bg = background.map(|c| colour(c.r, c.g, c.b));
        let toward = colours
            .iter()
            .map(|&c| Segment::new(bg.unwrap_or(c), c, gate))
            .collect();
        let mut between = Vec::new();
        for (i, &a) in colours.iter().enumerate() {
            for &b in &colours[i + 1..] {
                between.push(Segment::new(a, b, gate));
            }
        }
        let mut picker = Self {
            toward,
            between,
            colours,
            gate,
            tables: None,
        };
        if tabled {
            picker.tables = Some(picker.tables());
        }
        picker
    }

    /// The tables: a cell is marked when its centre is within the radius plus the cell's
    /// half diagonal of a segment, so no colour that can score is dismissed.
    fn tables(&self) -> Tables {
        let half_diagonal = 4.0 * 3.0_f64.sqrt();
        // Only a membership of at least one half can make a pixel content, so the noise
        // reach is measured to the half of each mixing segment that lies at the picked colour.
        let halves: Vec<Segment> = self
            .toward
            .iter()
            .map(|s| {
                let mid = [0, 1, 2].map(|k| s.from[k] + 0.5 * s.d[k]);
                let end = [0, 1, 2].map(|k| s.from[k] + s.d[k]);
                Segment::new(mid, end, self.gate)
            })
            .collect();
        let segments: Vec<&Segment> = self.toward.iter().chain(&self.between).collect();
        let strong: Vec<&Segment> = halves.iter().chain(&self.between).collect();
        let mut hit = vec![false; 1 << 15];
        let mut near = vec![false; 1 << 15];
        for (index, (h, n)) in hit.iter_mut().zip(near.iter_mut()).enumerate() {
            let i = u32::try_from(index).unwrap_or(0);
            let centre = [
                f64::from((i >> 10) * 8) + 3.5,
                f64::from(((i >> 5) & 31) * 8) + 3.5,
                f64::from((i & 31) * 8) + 3.5,
            ];
            let closest = |list: &[&Segment]| {
                list.iter()
                    .map(|s| s.distance(centre))
                    .fold(f64::INFINITY, f64::min)
            };
            *h = closest(&segments) <= self.gate + half_diagonal;
            *n = closest(&strong) <= self.gate + NOISE_REACH + half_diagonal;
        }
        Tables { hit, near }
    }

    /// The cached score of `c`, without a lookup when no segment is near its cell.
    fn cached(&self, cache: &mut Cache, c: [u8; 3]) -> u8 {
        if self.tables.as_ref().is_some_and(|t| !t.hit[cell(c)]) {
            return 0;
        }
        cache.get(c[0], c[1], c[2], |p| self.score(p))
    }

    /// Whether `c` can be a noisy colour of one that scores.
    fn is_near(&self, c: [u8; 3]) -> bool {
        self.tables.as_ref().is_none_or(|t| t.near[cell(c)])
    }

    /// Lets a pixel next to a selected one also be an anti-aliasing blend of a picked colour and
    /// the colour of a neighbour that is not selected (a fringe of cheap resampling, not only the
    /// background), at its mixing fraction; raises `score` in place.
    fn fringe(&self, score: &mut [u8], pixels: &[[u8; 4]], width: usize) {
        let first = score.to_vec();
        let height = pixels.len() / width;
        let selected = |at: usize| first[at] >= 128;
        for y in 0..height {
            for x in 0..width {
                let at = y * width + x;
                if selected(at) {
                    continue;
                }
                let around = |f: &mut dyn FnMut(usize)| {
                    for ny in y.saturating_sub(1)..=(y + 1).min(height - 1) {
                        for nx in x.saturating_sub(1)..=(x + 1).min(width - 1) {
                            if (nx, ny) != (x, y) {
                                f(ny * width + nx);
                            }
                        }
                    }
                };
                let mut touches = false;
                around(&mut |near| touches |= selected(near));
                if !touches {
                    continue;
                }
                let [red, green, blue, _] = pixels[at];
                let own = colour(red, green, blue);
                let mut best = 0.0_f64;
                around(&mut |near| {
                    if selected(near) {
                        return;
                    }
                    let [nr, ng, nb, _] = pixels[near];
                    for &picked in &self.colours {
                        if let Some(f) =
                            Segment::new(colour(nr, ng, nb), picked, self.gate).hit(own)
                        {
                            best = best.max(f);
                        }
                    }
                });
                // invariant: `best` is within 0 to 1, so the product is within 0 to 255.
                let level = u8::try_from(round_half_up(best * 255.0)).unwrap_or(u8::MAX);
                score[at] = score[at].max(level);
            }
        }
    }

    /// The membership of `p` times 255, rounded.
    fn score(&self, p: [f64; 3]) -> u8 {
        let mut best = 0.0_f64;
        for segment in &self.toward {
            if let Some(f) = segment.hit(p) {
                best = best.max(f);
            }
        }
        if best < 1.0 && self.between.iter().any(|s| s.hit(p).is_some()) {
            best = 1.0;
        }
        // invariant: `best` is within 0 to 1, so the product is within 0 to 255.
        u8::try_from(round_half_up(best * 255.0)).unwrap_or(u8::MAX)
    }
}

/// `v` (0 to 255) rounded half up, as an integer.
fn round_half_up(v: f64) -> u32 {
    let whole = v.floor();
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // 0 to 255
    let n = whole as u32;
    if v - whole >= 0.5 { n + 1 } else { n }
}

/// A direct-mapped cache of the score per colour: pictures repeat their colours, and the score
/// is a pure function of the colour, so the cache changes the speed only.
struct Cache {
    /// `rgb + 1` in the low 32 bits (0 is empty), the score above.
    slots: Vec<u64>,
}

impl Cache {
    const BITS: u32 = 16;

    fn new() -> Self {
        Self {
            slots: vec![0; 1 << Self::BITS],
        }
    }

    fn get(&mut self, r: u8, g: u8, b: u8, score: impl Fn([f64; 3]) -> u8) -> u8 {
        let rgb = u32::from(r) << 16 | u32::from(g) << 8 | u32::from(b);
        let index = (rgb.wrapping_mul(2_654_435_761) >> (32 - Self::BITS)) as usize;
        let slot = self.slots[index];
        let key = u64::from(rgb + 1);
        if slot & 0xFFFF_FFFF == key {
            // invariant: the score was stored as a byte.
            return u8::try_from(slot >> 32).unwrap_or(u8::MAX);
        }
        let value = score(colour(r, g, b));
        self.slots[index] = key | u64::from(value) << 32;
        value
    }
}
