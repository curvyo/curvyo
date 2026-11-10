//! Merges found colours that lie closer than the merge distance into each other and into the
//! maker's fixed colours, and orders the result.

use super::{Entry, Found, MAX_COLOURS, MERGE_DISTANCE, bytes, distance};
use crate::params::Rgb8;

/// A palette colour while merging: its value, its pixels, whether it is fixed.
pub(super) type Merging = ([f64; 3], u64, bool);

/// The `fixed` colours, then the `found` ones merged into them or each other while two lie
/// closer than [`MERGE_DISTANCE`] (the closest pair first; a fixed colour keeps its value), most
/// pixels first.
pub(super) fn merged(found: Vec<Found>, fixed: &[Rgb8]) -> Vec<Entry> {
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
pub(super) fn closest_pair(all: &[Merging]) -> Option<(usize, usize)> {
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
pub(super) fn combined(one: Merging, other: Merging) -> Merging {
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
pub(super) fn ordered(all: Vec<Merging>) -> Vec<Entry> {
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

/// `c` rounded to bytes.
fn rounded(c: [f64; 3]) -> Rgb8 {
    // invariant: means of bytes lie in 0 to 255.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let byte = |v: f64| (v + 0.5).floor().clamp(0.0, 255.0) as u8;
    Rgb8::new(byte(c[0]), byte(c[1]), byte(c[2]))
}
