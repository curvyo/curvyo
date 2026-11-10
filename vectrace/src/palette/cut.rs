//! The median cut of the flat colours: boxes of distinct colours split at the largest gap or
//! the best Otsu cut along their widest channel, and the test for whether they form groups.

use super::MERGE_DISTANCE;

/// An automatic split needs at least this share of the box's variance between its two groups (a
/// normal spread gives about 0.64, an even one 0.75, two colours close to 1).
pub(super) const MODES: f64 = 0.8;

/// An automatic split of a box of three groups needs at least this share of the variance
/// between the three (an even spread gives about 0.89, a normal one 0.81).
pub(super) const MODES3: f64 = 0.95;

/// A distinct colour and how many flat pixels have it.
pub(super) type Distinct = ([u8; 3], u64);

/// One box of the median cut: a slice of the distinct colours with its pixel sums.
#[derive(Debug, Clone, Copy)]
pub(super) struct Box3 {
    pub(super) from: usize,
    pub(super) to: usize,
    pub(super) count: u64,
}

/// The fewest pixels a half of an automatic split holds: 0.05 % of the votes, at least 4.
pub(super) fn min_count(distinct: &[Distinct]) -> u64 {
    (distinct.iter().map(|d| d.1).sum::<u64>() / 2000).max(4)
}

/// The distinct colours of `samples` with their counts, in packed order.
pub(super) fn distinct(samples: &[[u8; 3]]) -> Vec<Distinct> {
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
pub(super) fn mean(colours: &[Distinct]) -> [f64; 3] {
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
pub(super) fn cut(
    distinct: &mut [Distinct],
    boxes: usize,
    auto: bool,
    min_count: u64,
) -> Vec<Box3> {
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
