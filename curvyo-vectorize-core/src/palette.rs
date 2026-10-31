//! Builds the palette of a multicolour trace (`specs/0089` adrs.md §10, criterion 26): a
//! deterministic median cut of the flat pixels in the sRGB unit cube (trial C1), colours closer
//! than [`MERGE_DISTANCE`] merged, the maker's fixed colours kept as they are.
//!
//! Only flat pixels vote (every neighbour of the 3 x 3 window within [`FLAT_BYTES`] per channel),
//! so the blends of anti-aliased edges and the fringes of JPEG never become colours of their
//! own. An automatic count splits a box only where its colours fall into groups: the two means
//! of the split lie at least the merge distance apart, and the between-group share of the
//! variance along its widest channel is at least [`MODES`] for two groups or [`MODES3`] for
//! three (a box that still holds three colours), so noise stays one colour; colours that form
//! no groups at all (a photo, a gradient) get [`DEFAULT_COLOURS`] steps. Content too thin for a
//! flat pixel (line art) votes with its ridge pixels ([`Lines`]): the inks the flat colours do
//! not explain take the free slots, each as its strongest blend. Basic operations and `sqrt`
//! only, no hash maps.

use crate::params::Rgb8;

/// Colours closer than this merge, in bytes (0.03 of the sRGB unit cube, frozen 2026-10-30).
pub(crate) const MERGE_DISTANCE: f64 = 0.03 * 255.0;

/// The most colours a palette holds (criterion 26).
pub const MAX_COLOURS: usize = 16;

/// The fewest colours a typed count asks for (criterion 26).
pub const MIN_COLOURS: usize = 2;

/// A pixel is flat when every opaque neighbour of its 3 x 3 window is within this many bytes per
/// channel of it.
pub(crate) const FLAT_BYTES: u8 = 12;

/// An automatic split needs at least this share of the box's variance between its two groups (a
/// normal spread gives about 0.64, an even one 0.75, two colours close to 1).
const MODES: f64 = 0.8;

/// An automatic split of a box of three groups needs at least this share of the variance
/// between the three (an even spread gives about 0.89, a normal one 0.81).
const MODES3: f64 = 0.95;

/// A line's colour is the blend this share of its ridge pixels lies closer to the background
/// than (its strongest blends, a stray pixel aside).
const LINE_STRENGTH: f64 = 0.8;

/// The fewest ridge pixels that make a line colour.
const LINE_PIXELS: usize = 4;

/// The count an automatic palette falls back to when the flat colours form no distinct groups
/// (a photo, a gradient): criterion 26's default (**Proposal**).
pub const DEFAULT_COLOURS: usize = 6;

/// A palette entry before its pixels are counted: its colour and whether the maker fixed it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Entry {
    pub colour: Rgb8,
    pub fixed: bool,
}

/// The ridge pixels of content that has no flat pixel (lines narrower than the flat window)
/// and the background their blends run toward (`None`: none known, a line is its mean).
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Lines {
    pub samples: Vec<[u8; 3]>,
    pub toward: Option<[f64; 3]>,
}

/// One box of the median cut: a slice of the distinct colours with its pixel sums.
#[derive(Debug, Clone, Copy)]
struct Box3 {
    from: usize,
    to: usize,
    count: u64,
}

/// A distinct colour and how many flat pixels have it.
type Distinct = ([u8; 3], u64);

/// A found colour: its value in bytes and the pixels behind it.
type Found = ([f64; 3], u64);

/// The palette of the flat colours `samples` and the ridge pixels of `lines` (`count`: `None`
/// for the automatic count, else the number of colours wanted, both within [`MIN_COLOURS`] and
/// [`MAX_COLOURS`] with `fixed` included), the `fixed` colours first and kept exactly, then the
/// found colours by their pixels, most first (ties by value). Lines only take the slots the
/// flat colours leave.
pub(crate) fn palette(
    samples: &[[u8; 3]],
    lines: &Lines,
    count: Option<usize>,
    fixed: &[Rgb8],
) -> Vec<Entry> {
    let wanted = count.map(|n| n.clamp(MIN_COLOURS, MAX_COLOURS));
    let fixed_n = fixed.len().min(MAX_COLOURS);
    let free = wanted.unwrap_or(MAX_COLOURS).saturating_sub(fixed_n);
    let mut found = flat_colours(samples, wanted.is_none(), free, fixed_n);
    let slots = free.saturating_sub(found.len());
    let known: Vec<[f64; 3]> = found
        .iter()
        .map(|f| f.0)
        .chain(fixed.iter().map(|&f| bytes(f)))
        .collect();
    found.extend(line_colours(lines, &known, slots));
    merged(found, fixed)
}

/// The colours of the flat `samples` in at most `free` boxes; `auto` finds the count, falling
/// back to [`DEFAULT_COLOURS`] (less the `fixed_n` fixed ones) when they form no groups.
fn flat_colours(samples: &[[u8; 3]], auto: bool, free: usize, fixed_n: usize) -> Vec<Found> {
    let mut distinct = distinct(samples);
    let min_count = min_count(&distinct);
    let mut boxes = cut(&mut distinct, free, auto, min_count);
    if auto && boxes.len() == 1 {
        // No distinct groups: a spread of colours gets the default count of steps.
        let steps = DEFAULT_COLOURS.saturating_sub(fixed_n);
        boxes = cut(&mut distinct, steps, false, min_count);
    }
    boxes
        .iter()
        .map(|b| (mean(&distinct[b.from..b.to]), b.count))
        .collect()
}

/// The inks of the ridge pixels of `lines` that none of the `known` colours explains, at most
/// `slots`: the strongest remaining pixel ([`LINE_STRENGTH`] of the rest weaker) seeds an ink,
/// the pixels on its blends ([`off_line`]) are its line, the ink their [`LINE_STRENGTH`]
/// blend; a line needs [`LINE_PIXELS`] pixels.
fn line_colours(lines: &Lines, known: &[[f64; 3]], slots: usize) -> Vec<Found> {
    let explained = |p: [f64; 3]| {
        known
            .iter()
            .any(|&c| off_line(p, lines.toward, c) < MERGE_DISTANCE)
    };
    let strength = |p: [f64; 3]| lines.toward.map_or(0.0, |bg| distance(p, bg));
    let mut left: Vec<[f64; 3]> = lines
        .samples
        .iter()
        .map(|p| [f64::from(p[0]), f64::from(p[1]), f64::from(p[2])])
        .filter(|&p| !explained(p))
        .collect();
    left.sort_by(|a, b| {
        strength(*a)
            .total_cmp(&strength(*b))
            .then(a[0].total_cmp(&b[0]))
            .then(a[1].total_cmp(&b[1]))
            .then(a[2].total_cmp(&b[2]))
    });
    let mut out = Vec::new();
    while out.len() < slots && left.len() >= LINE_PIXELS {
        let seed = left[quantile(left.len())];
        let (line, rest): (Vec<[f64; 3]>, Vec<[f64; 3]>) = left
            .into_iter()
            .partition(|&p| off_line(p, lines.toward, seed) < MERGE_DISTANCE);
        if line.len() >= LINE_PIXELS {
            out.push((
                line[quantile(line.len())],
                u64::try_from(line.len()).unwrap_or(u64::MAX),
            ));
        }
        left = rest;
    }
    out
}

/// The index below which [`LINE_STRENGTH`] of `n` sorted values lie.
fn quantile(n: usize) -> usize {
    // invariant: a share of a pixel count fits `usize`.
    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )]
    let at = ((n.saturating_sub(1)) as f64 * LINE_STRENGTH) as usize;
    at
}

/// How far `p` lies from the blends of `colour` toward the background `toward` and beyond
/// (the ray from `toward` through `colour`: a line drawn in `colour`, its ink overshooting), or
/// from `colour` itself without a background.
fn off_line(p: [f64; 3], toward: Option<[f64; 3]>, colour: [f64; 3]) -> f64 {
    let Some(bg) = toward else {
        return distance(p, colour);
    };
    let ray = [colour[0] - bg[0], colour[1] - bg[1], colour[2] - bg[2]];
    let to = [p[0] - bg[0], p[1] - bg[1], p[2] - bg[2]];
    let len = ray[0] * ray[0] + ray[1] * ray[1] + ray[2] * ray[2];
    if len == 0.0 {
        return distance(p, colour);
    }
    let t = ((to[0] * ray[0] + to[1] * ray[1] + to[2] * ray[2]) / len).max(0.0);
    distance(
        p,
        [bg[0] + ray[0] * t, bg[1] + ray[1] * t, bg[2] + ray[2] * t],
    )
}

/// The fewest pixels a half of an automatic split holds: 0.05 % of the votes, at least 4.
fn min_count(distinct: &[Distinct]) -> u64 {
    (distinct.iter().map(|d| d.1).sum::<u64>() / 2000).max(4)
}

/// The distinct colours of `samples` with their counts, in packed order.
fn distinct(samples: &[[u8; 3]]) -> Vec<Distinct> {
    let mut packed: Vec<u32> = samples
        .iter()
        .map(|c| u32::from(c[0]) << 16 | u32::from(c[1]) << 8 | u32::from(c[2]))
        .collect();
    packed.sort_unstable();
    let mut out: Vec<Distinct> = Vec::new();
    for p in packed {
        // invariant: each channel is the byte it was packed from.
        #[allow(clippy::cast_possible_truncation)]
        let colour = [(p >> 16) as u8, (p >> 8) as u8, p as u8];
        match out.last_mut() {
            Some(last) if last.0 == colour => last.1 += 1,
            _ => out.push((colour, 1)),
        }
    }
    out
}

/// The weighted mean of `colours`, in bytes.
fn mean(colours: &[Distinct]) -> [f64; 3] {
    let mut sum = [0.0; 3];
    let mut n = 0.0;
    for &(c, k) in colours {
        #[allow(clippy::cast_precision_loss)] // at most 64 Mpx
        let k = k as f64;
        for i in 0..3 {
            sum[i] += f64::from(c[i]) * k;
        }
        n += k;
    }
    if n == 0.0 {
        return sum;
    }
    [sum[0] / n, sum[1] / n, sum[2] / n]
}

/// Per channel the sum of squared deviations of `colours` from their mean.
fn spread(colours: &[Distinct]) -> [f64; 3] {
    let m = mean(colours);
    let mut out = [0.0; 3];
    for &(c, k) in colours {
        #[allow(clippy::cast_precision_loss)] // at most 64 Mpx
        let k = k as f64;
        for i in 0..3 {
            let d = f64::from(c[i]) - m[i];
            out[i] += d * d * k;
        }
    }
    out
}

/// The median cut: up to `boxes` boxes over `distinct` (reordered inside each box); `auto` splits
/// only boxes of groups, each half with at least `min_count` pixels; otherwise the box with the
/// largest spread is split until there are `boxes` or none has two colours.
fn cut(distinct: &mut [Distinct], boxes: usize, auto: bool, min_count: u64) -> Vec<Box3> {
    if distinct.is_empty() || boxes == 0 {
        return Vec::new();
    }
    let mut out = vec![Box3 {
        from: 0,
        to: distinct.len(),
        count: distinct.iter().map(|d| d.1).sum(),
    }];
    // Boxes no split helps.
    let mut done = vec![false];
    while out.len() < boxes {
        let mut best: Option<(f64, usize)> = None;
        for (i, b) in out.iter().enumerate() {
            if done[i] || b.to - b.from < 2 {
                continue;
            }
            let s = spread(&distinct[b.from..b.to]);
            let total = s[0] + s[1] + s[2];
            if best.is_none_or(|(t, _)| total > t) {
                best = Some((total, i));
            }
        }
        let Some((_, i)) = best else {
            break;
        };
        let b = out[i];
        match split(&mut distinct[b.from..b.to], auto, min_count) {
            Some(at) => {
                let left: u64 = distinct[b.from..b.from + at].iter().map(|d| d.1).sum();
                out[i] = Box3 {
                    from: b.from,
                    to: b.from + at,
                    count: left,
                };
                out.push(Box3 {
                    from: b.from + at,
                    to: b.to,
                    count: b.count - left,
                });
                done.push(false);
            }
            None => done[i] = true,
        }
    }
    out
}

/// Where the box `colours` (reordered) is split, or `None` when `auto` and it is not groups
/// (see the module comment).
fn split(colours: &mut [Distinct], auto: bool, min_count: u64) -> Option<usize> {
    let (share, at, gap) = otsu(colours, auto, min_count)?;
    let groups = !auto
        || (gap >= MERGE_DISTANCE && (share >= MODES || three_share(colours, min_count) >= MODES3));
    groups.then_some(at)
}

/// The between-group share of the variance along the channel `colours` are sorted by (their
/// widest, see [`otsu`]) for the best split into three groups of at least `min_count` pixels
/// whose neighbouring means lie the merge distance apart (multi-level Otsu over the distinct
/// channel values); 0 without one.
fn three_share(colours: &[Distinct], min_count: u64) -> f64 {
    let s = spread(colours);
    let channel = widest(s);
    // Per distinct channel value: (pixels, sum of values).
    let mut values: Vec<(u64, f64)> = Vec::new();
    let mut last: Option<u8> = None;
    for &(c, k) in colours {
        #[allow(clippy::cast_precision_loss)] // at most 64 Mpx
        let sum = f64::from(c[channel]) * k as f64;
        match values.last_mut() {
            Some(v) if last == Some(c[channel]) => {
                v.0 += k;
                v.1 += sum;
            }
            _ => values.push((k, sum)),
        }
        last = Some(c[channel]);
    }
    let mut prefix: Vec<(u64, f64)> = vec![(0, 0.0)];
    for &(k, sum) in &values {
        let (pk, ps) = prefix[prefix.len() - 1];
        prefix.push((pk + k, ps + sum));
    }
    let (total_k, total_sum) = prefix[values.len()];
    #[allow(clippy::cast_precision_loss)] // at most 64 Mpx
    let group = |from: usize, to: usize| {
        let k = prefix[to].0 - prefix[from].0;
        (k, (prefix[to].1 - prefix[from].1) / k.max(1) as f64)
    };
    #[allow(clippy::cast_precision_loss)] // at most 64 Mpx
    let mean = total_sum / total_k.max(1) as f64;
    let mut best: f64 = 0.0;
    for i in 1..values.len() {
        for j in i + 1..values.len() {
            let parts = [group(0, i), group(i, j), group(j, values.len())];
            if parts.iter().any(|p| p.0 < min_count)
                || parts[1].1 - parts[0].1 < MERGE_DISTANCE
                || parts[2].1 - parts[1].1 < MERGE_DISTANCE
            {
                continue;
            }
            #[allow(clippy::cast_precision_loss)] // at most 64 Mpx
            let between: f64 = parts
                .iter()
                .map(|&(k, m)| k as f64 * (m - mean) * (m - mean))
                .sum();
            best = best.max(between);
        }
    }
    if s[channel] == 0.0 {
        return 0.0;
    }
    best / s[channel]
}

/// The channel with the largest `spread` (ties to the first).
fn widest(s: [f64; 3]) -> usize {
    if s[0] >= s[1] && s[0] >= s[2] {
        0
    } else if s[1] >= s[2] {
        1
    } else {
        2
    }
}

/// Sorts `colours` along their widest channel and returns the best two-group split of that
/// channel (Otsu) as the between-group share of its variance, the split position and the gap
/// between the two means; `None` without a split (with `balanced`, one whose halves both hold
/// `min_count` pixels).
fn otsu(colours: &mut [Distinct], balanced: bool, min_count: u64) -> Option<(f64, usize, f64)> {
    let s = spread(colours);
    let channel = widest(s);
    colours.sort_unstable_by_key(|&(c, _)| (c[channel], c));
    let total_k: u64 = colours.iter().map(|d| d.1).sum();
    #[allow(clippy::cast_precision_loss)] // at most 64 Mpx
    let (total_n, total_sum) = (
        total_k as f64,
        colours
            .iter()
            .map(|&(c, k)| f64::from(c[channel]) * k as f64)
            .sum::<f64>(),
    );
    let mut best: Option<(f64, usize, f64)> = None;
    let (mut n0, mut sum0, mut k0) = (0.0, 0.0, 0_u64);
    for at in 1..colours.len() {
        let (c, k) = colours[at - 1];
        #[allow(clippy::cast_precision_loss)] // at most 64 Mpx
        let kf = k as f64;
        n0 += kf;
        sum0 += f64::from(c[channel]) * kf;
        k0 += k;
        if colours[at].0[channel] == c[channel] {
            continue;
        }
        let n1 = total_n - n0;
        let (m0, m1) = (sum0 / n0, (total_sum - sum0) / n1);
        let between = n0 * n1 * (m0 - m1) * (m0 - m1);
        let fits = !balanced || (k0 >= min_count && total_k - k0 >= min_count);
        if fits && best.is_none_or(|(b, _, _)| between > b) {
            best = Some((between, at, (m1 - m0).abs()));
        }
    }
    let (between, at, gap) = best?;
    // Between-group variance as a share of the whole along the channel.
    let share = between / (total_n * total_n) / (s[channel] / total_n);
    Some((share, at, gap))
}

/// The colour distance in bytes (Euclidean in the sRGB cube).
pub(crate) fn distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    let d = [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()
}

/// A colour's bytes as `f64`.
pub(crate) fn bytes(c: Rgb8) -> [f64; 3] {
    [f64::from(c.r), f64::from(c.g), f64::from(c.b)]
}

/// `c` rounded to bytes.
fn rounded(c: [f64; 3]) -> Rgb8 {
    // invariant: means of bytes lie in 0 to 255.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let byte = |v: f64| (v + 0.5).floor().clamp(0.0, 255.0) as u8;
    Rgb8::new(byte(c[0]), byte(c[1]), byte(c[2]))
}

/// A palette colour while merging: its value, its pixels, whether it is fixed.
type Merging = ([f64; 3], u64, bool);

/// The `fixed` colours, then the `found` ones merged into them or each other while two lie
/// closer than [`MERGE_DISTANCE`] (the closest pair first; a fixed colour keeps its value), most
/// pixels first.
fn merged(found: Vec<Found>, fixed: &[Rgb8]) -> Vec<Entry> {
    let mut all: Vec<Merging> = Vec::new();
    for &f in fixed.iter().take(MAX_COLOURS) {
        if all
            .iter()
            .all(|e| distance(e.0, bytes(f)) >= MERGE_DISTANCE)
        {
            all.push((bytes(f), 0, true));
        }
    }
    all.extend(found.into_iter().map(|(c, k)| (c, k, false)));
    while let Some((i, j)) = closest_pair(&all) {
        all[i] = combined(all[i], all[j]);
        all.remove(j);
    }
    ordered(all)
}

/// The closest pair of `all` nearer than [`MERGE_DISTANCE`] that is not two fixed colours.
fn closest_pair(all: &[Merging]) -> Option<(usize, usize)> {
    let mut closest: Option<(f64, usize, usize)> = None;
    for i in 0..all.len() {
        for j in i + 1..all.len() {
            if all[i].2 && all[j].2 {
                continue;
            }
            let d = distance(all[i].0, all[j].0);
            if d < MERGE_DISTANCE && closest.is_none_or(|(c, _, _)| d < c) {
                closest = Some((d, i, j));
            }
        }
    }
    closest.map(|(_, i, j)| (i, j))
}

/// Two merging colours as one: a fixed one's value, else the pixel-weighted mean.
fn combined(one: Merging, other: Merging) -> Merging {
    let colour = if one.2 {
        one.0
    } else if other.2 {
        other.0
    } else {
        #[allow(clippy::cast_precision_loss)] // at most 64 Mpx
        let (k1, k2) = (one.1 as f64, other.1 as f64);
        let total = (k1 + k2).max(1.0);
        [
            (one.0[0] * k1 + other.0[0] * k2) / total,
            (one.0[1] * k1 + other.0[1] * k2) / total,
            (one.0[2] * k1 + other.0[2] * k2) / total,
        ]
    };
    (colour, one.1 + other.1, one.2 || other.2)
}

/// The merged colours as entries: the fixed ones first, then the rest by pixels (ties by value),
/// a rest colour that rounds within the merge distance of an earlier one dropped, at most
/// [`MAX_COLOURS`].
fn ordered(all: Vec<Merging>) -> Vec<Entry> {
    let (fixed_part, mut found_part): (Vec<_>, Vec<_>) = all.into_iter().partition(|e| e.2);
    found_part.sort_by(|a, b| {
        b.1.cmp(&a.1).then_with(|| {
            let (ca, cb) = (rounded(a.0), rounded(b.0));
            (ca.r, ca.g, ca.b).cmp(&(cb.r, cb.g, cb.b))
        })
    });
    let mut out: Vec<Entry> = fixed_part
        .iter()
        .map(|e| Entry {
            colour: rounded(e.0),
            fixed: true,
        })
        .collect();
    for e in found_part {
        let colour = rounded(e.0);
        if out
            .iter()
            .all(|o| distance(bytes(o.colour), bytes(colour)) >= MERGE_DISTANCE)
            && out.len() < MAX_COLOURS
        {
            out.push(Entry {
                colour,
                fixed: false,
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn many(colour: [u8; 3], n: usize) -> Vec<[u8; 3]> {
        vec![colour; n]
    }

    /// A deterministic noise of +-`amp` per channel.
    fn noisy(colour: [u8; 3], n: usize, amp: i32) -> Vec<[u8; 3]> {
        let mut seed: u32 = 12345;
        (0..n)
            .map(|_| {
                let mut c = colour;
                for v in &mut c {
                    seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12345);
                    let d = i32::try_from((seed >> 16) % 65_536).unwrap() % (2 * amp + 1) - amp;
                    *v = u8::try_from((i32::from(*v) + d).clamp(0, 255)).unwrap();
                }
                c
            })
            .collect()
    }

    fn colours(p: &[Entry]) -> Vec<[u8; 3]> {
        p.iter()
            .map(|e| [e.colour.r, e.colour.g, e.colour.b])
            .collect()
    }

    #[test]
    fn auto_finds_the_colours_and_ignores_noise() {
        let mut s = noisy([200, 30, 30], 4000, 4);
        s.extend(noisy([30, 60, 200], 3000, 4));
        s.extend(noisy([240, 200, 40], 2000, 4));
        let p = palette(&s, &Lines::default(), None, &[]);
        assert_eq!(p.len(), 3, "{:?}", colours(&p));
        let c = colours(&p);
        assert!(
            distance(
                bytes(Rgb8::new(c[0][0], c[0][1], c[0][2])),
                [200.0, 30.0, 30.0]
            ) < 2.0
        );
    }

    #[test]
    fn a_pale_accent_of_a_fifth_of_a_percent_is_kept() {
        let mut s = many([200, 30, 30], 30_000);
        s.extend(many([30, 60, 200], 19_900));
        s.extend(many([0xFF, 0xF2, 0xA8], 100));
        let p = palette(&s, &Lines::default(), None, &[]);
        assert_eq!(p.len(), 3, "{:?}", colours(&p));
        assert!(colours(&p).contains(&[0xFF, 0xF2, 0xA8]));
    }

    #[test]
    fn near_greys_stay_apart_and_closer_colours_merge() {
        let mut s = many([0xD9, 0xD9, 0xD9], 5000);
        s.extend(many([0xDC, 0xD6, 0xCF], 5000));
        assert_eq!(palette(&s, &Lines::default(), None, &[]).len(), 2);
        // 0.02 apart: one colour.
        let mut s = many([100, 100, 100], 5000);
        s.extend(many([105, 102, 100], 5000));
        assert_eq!(palette(&s, &Lines::default(), None, &[]).len(), 1);
        assert_eq!(palette(&s, &Lines::default(), Some(2), &[]).len(), 1);
    }

    #[test]
    fn three_groups_in_one_box_are_all_found() {
        // Two near-greys 0.043 apart and a small pale accent: no single split of their box
        // explains 0.8 of its variance, its halves do.
        let mut s = many([200, 30, 30], 7000);
        s.extend(many([0xD9, 0xD9, 0xD9], 4500));
        s.extend(many([0xDC, 0xD6, 0xCF], 4500));
        s.extend(many([0xFF, 0xF2, 0xA8], 100));
        let p = palette(&s, &Lines::default(), None, &[]);
        assert_eq!(p.len(), 4, "{:?}", colours(&p));
        for c in [[0xD9, 0xD9, 0xD9], [0xDC, 0xD6, 0xCF], [0xFF, 0xF2, 0xA8]] {
            assert!(colours(&p).contains(&c), "{c:?} in {:?}", colours(&p));
        }
    }

    #[test]
    fn line_samples_give_their_strongest_colour() {
        // Ridge pixels of an anti-aliased black line on white: coverage 0.6 to 1.
        let ridge: Vec<[u8; 3]> = (0..200)
            .map(|i| {
                let v = u8::try_from(i % 100).unwrap();
                [v, v, v]
            })
            .collect();
        let lines = Lines {
            samples: ridge,
            toward: Some([255.0, 255.0, 255.0]),
        };
        let p = palette(&[], &lines, None, &[]);
        assert_eq!(p.len(), 1, "{:?}", colours(&p));
        assert!(p[0].colour.r < 25, "{:?}", colours(&p));
        // A typed count does not cut a line's blends into steps.
        assert_eq!(palette(&[], &lines, Some(6), &[]).len(), 1);
        // A line that is a flat colour's blend toward the background (or its ink beyond it) is
        // that colour.
        let grey = many([24, 24, 24], 1000);
        assert_eq!(palette(&grey, &lines, None, &[]).len(), 1);
        // Flat colours take the slots first.
        let s = many([200, 30, 30], 1000);
        let p = palette(&s, &lines, Some(2), &[]);
        assert_eq!(p.len(), 2, "{:?}", colours(&p));
    }

    #[test]
    fn a_gradient_gets_the_default_steps_automatically_and_steps_with_a_count() {
        let s: Vec<[u8; 3]> = (0..=255).flat_map(|v| many([v, v, v], 20)).collect();
        assert_eq!(
            palette(&s, &Lines::default(), None, &[]).len(),
            DEFAULT_COLOURS
        );
        // One noisy colour stays one.
        assert_eq!(
            palette(&noisy([90, 120, 30], 5000, 3), &Lines::default(), None, &[]).len(),
            1
        );
        let p = palette(&s, &Lines::default(), Some(5), &[]);
        assert_eq!(p.len(), 5, "{:?}", colours(&p));
    }

    #[test]
    fn fixed_colours_come_first_and_keep_their_value() {
        let mut s = many([200, 30, 30], 4000);
        s.extend(many([30, 60, 200], 3000));
        let fixed = [Rgb8::new(202, 31, 29), Rgb8::new(0, 200, 0)];
        let p = palette(&s, &Lines::default(), None, &fixed);
        assert_eq!(
            colours(&p),
            [[202, 31, 29], [0, 200, 0], [30, 60, 200]],
            "{p:?}"
        );
        assert!(p[0].fixed && p[1].fixed && !p[2].fixed);
        // A count of 2 with two fixed colours: only the fixed ones.
        assert_eq!(palette(&s, &Lines::default(), Some(2), &fixed).len(), 2);
    }

    #[test]
    fn the_same_samples_in_another_order_give_the_same_palette() {
        let mut s = noisy([200, 30, 30], 3000, 6);
        s.extend(noisy([30, 60, 200], 3000, 6));
        let a = palette(&s, &Lines::default(), Some(6), &[]);
        s.reverse();
        assert_eq!(a, palette(&s, &Lines::default(), Some(6), &[]));
    }
}
