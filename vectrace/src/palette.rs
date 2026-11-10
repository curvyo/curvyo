//! Builds the palette of a multicolour trace (`specs/0089` adrs.md §10, criterion 26): a
//! deterministic median cut of the flat pixels in the sRGB unit cube (trial C1), colours closer
//! than [`MERGE_DISTANCE`] merged, the maker's fixed colours kept as they are.
//!
//! Only flat pixels vote (every neighbour of the 3 x 3 window within [`FLAT_BYTES`] per channel),
//! so the blends of anti-aliased edges and the fringes of JPEG never become colours of their
//! own. An automatic count splits a box only where its colours fall into groups: the two means
//! of the split lie at least the merge distance apart, and the between-group share of the
//! variance along its widest channel is at least `MODES` for two groups or `MODES3` for
//! three (a box that still holds three colours), so noise stays one colour; colours that form
//! no groups at all (a photo, a gradient) get [`DEFAULT_COLOURS`] steps. Content too thin for a
//! flat pixel (line art) votes with its ridge pixels ([`Lines`]): the inks the flat colours do
//! not explain take the free slots, each as its strongest blend. Basic operations and `sqrt`
//! only, no hash maps.

use crate::params::Rgb8;

mod cut;
mod merge;

use cut::{cut, distinct, mean, min_count};
use merge::merged;

/// Colours closer than this merge, in bytes (0.03 of the sRGB unit cube, frozen 2026-10-30).
pub(crate) const MERGE_DISTANCE: f64 = 0.03 * 255.0;

/// The most colours a palette holds (criterion 26).
pub const MAX_COLOURS: usize = 16;

/// The fewest colours a typed count asks for (criterion 26).
pub const MIN_COLOURS: usize = 2;

/// A pixel is flat when every opaque neighbour of its 3 x 3 window is within this many bytes per
/// channel of it.
pub(crate) const FLAT_BYTES: u8 = 12;

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

/// A found colour: its value in bytes and the pixels behind it.
type Found = ([f64; 3], u64);

/// The palette of the flat colours `samples` and the ridge pixels of `lines` (`count`: `None`
/// for the automatic count, else the number of colours wanted, both within [`MIN_COLOURS`] and
/// [`MAX_COLOURS`] with `fixed` included; one fixed colour may be the whole palette), the
/// `fixed` colours first and kept exactly, then the found colours by their pixels, most first
/// (ties by value). Lines only take the slots the flat colours leave.
pub(crate) fn palette(
    samples: &[[u8; 3]],
    lines: &Lines,
    count: Option<usize>,
    fixed: &[Rgb8],
) -> Vec<Entry> {
    let fixed_n = fixed.len().min(MAX_COLOURS);
    // A palette deleted down to its last colour keeps it alone (customer 2026-11-09).
    let lowest = if fixed_n == 0 {
        MIN_COLOURS
    } else {
        fixed_n.min(MIN_COLOURS)
    };
    let wanted = count.map(|n| n.clamp(lowest, MAX_COLOURS));
    let free = wanted.unwrap_or(MAX_COLOURS).saturating_sub(fixed_n);
    // With a typed count a fixed colour explains the samples within the merge distance of it:
    // they would only find it again and merge into it, taking a slot the other colours need
    // (customer 2026-11-10). The automatic count has a slot for every group anyway.
    let unexplained: Vec<[u8; 3]> = samples
        .iter()
        .copied()
        .filter(|&s| {
            wanted.is_none()
                || fixed.iter().all(|&f| {
                    distance(
                        [f64::from(s[0]), f64::from(s[1]), f64::from(s[2])],
                        bytes(f),
                    ) >= MERGE_DISTANCE
                })
        })
        .collect();
    let mut found = flat_colours(&unexplained, wanted.is_none(), free, fixed_n);
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

/// The colour distance in bytes (Euclidean in the sRGB cube).
pub(crate) fn distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    let d = [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()
}

/// A colour's bytes as `f64`.
pub(crate) fn bytes(c: Rgb8) -> [f64; 3] {
    [f64::from(c.r), f64::from(c.g), f64::from(c.b)]
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

    /// Customer 2026-11-10: a colour picked from the image takes one of the Colours, and the
    /// others go to the other colours; the picked colour's own pixels do not take a slot again
    /// (Colours 3 with red picked gave red and one blend of blue and black).
    #[test]
    fn a_fixed_colour_of_the_image_leaves_the_other_slots_to_the_other_colours() {
        let mut s = many([208, 34, 42], 4000);
        s.extend(many([35, 78, 184], 3000));
        s.extend(many([20, 20, 20], 2000));
        let red = Rgb8::new(208, 34, 42);
        let p = palette(&s, &Lines::default(), Some(3), &[red]);
        assert_eq!(
            colours(&p),
            [[208, 34, 42], [35, 78, 184], [20, 20, 20]],
            "{p:?}"
        );
        // A picked shade within the merge distance of the image's colour explains it too.
        let p = palette(&s, &Lines::default(), Some(3), &[Rgb8::new(204, 37, 40)]);
        assert_eq!(colours(&p)[1..], [[35, 78, 184], [20, 20, 20]], "{p:?}");
        // The automatic count finds the others too.
        assert_eq!(palette(&s, &Lines::default(), None, &[red]).len(), 3);
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
