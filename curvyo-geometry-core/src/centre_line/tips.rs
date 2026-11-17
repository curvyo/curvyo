//! How a chain runs into a tip of its region, a sharp convex corner of the outline (Join tips,
//! ADR 0063 amendment CENTRE-TIPS).

use kurbo::Point;

use super::chains::Chain;
use crate::medial_axis::AxisSample;
use crate::medial_axis::boundary::Boundary;

/// The outline is walked this far either side of a vertex to measure its turn, in millimetres.
const TURN_REACH_MM: f64 = 0.25;
/// A tip's outline turns by more than this sine-free measure: the cosine of 30 degrees (its
/// inner angle is below 150 degrees).
const TIP_TURN_COS: f64 = 0.866;
/// A chain sample takes a tip when the tip lies at most this many of its radii away.
const TIP_REACH_RADII: f64 = 3.0;
/// Convex corners closer together than this many radii of the disc nearest them are a flat
/// end's corners, no tip.
const FLAT_END_RADII: f64 = 2.5;
/// A tip lies beside its path: the cosine of its direction from the sample against the path's
/// is at most this (45 degrees off the path's normal).
const TIP_BESIDE_COS: f64 = 0.707;
/// A tip already painted lies within this share of the radius outside the nearest disc.
const TIP_PAINTED_SHARE: f64 = 0.1;

/// A chain sample whose path runs into a tip of the outline.
#[derive(Debug, Clone, Copy)]
struct CornerTip {
    /// The sample's index in its chain.
    index: usize,
    /// The sample.
    at: AxisSample,
    /// The tip, on the outline.
    tip: Point,
}

/// The sharp convex corners of `rings` (outer ring positive, holes negative: the region lies
/// to the left of every ring), each the sharpest vertex of its neighbourhood.
fn outline_tips(rings: &[Vec<Point>]) -> Vec<Point> {
    let mut found = Vec::new();
    for ring in rings {
        let n = ring.len();
        if n < 3 {
            continue;
        }
        let mut along = vec![0.0; n + 1];
        for i in 0..n {
            along[i + 1] = along[i] + (ring[(i + 1) % n] - ring[i]).hypot();
        }
        let total = along[n];
        if total <= 4.0 * TURN_REACH_MM {
            continue;
        }
        let at = |s: f64| -> Point {
            let s = s.rem_euclid(total);
            let i = along.partition_point(|&a| a <= s).clamp(1, n) - 1;
            let span = along[i + 1] - along[i];
            let t = if span > 0.0 {
                (s - along[i]) / span
            } else {
                0.0
            };
            ring[i].lerp(ring[(i + 1) % n], t)
        };
        // The cosine of the turn at each vertex, over the reach either side; convex turns only.
        let turns: Vec<f64> = (0..n)
            .map(|i| {
                let back = ring[i] - at(along[i] - TURN_REACH_MM);
                let ahead = at(along[i] + TURN_REACH_MM) - ring[i];
                let (lb, la) = (back.hypot(), ahead.hypot());
                if lb == 0.0 || la == 0.0 || back.cross(ahead) <= 0.0 {
                    return 1.0;
                }
                back.dot(ahead) / (lb * la)
            })
            .collect();
        for i in 0..n {
            if turns[i] >= TIP_TURN_COS {
                continue;
            }
            // The sharpest of its neighbourhood (the first of equals).
            // Walked out either way only as far as the reach (the ring is longer than four).
            let beaten = |k: usize| turns[k] < turns[i] || (turns[k] <= turns[i] && k < i);
            let mut sharpest = true;
            for forward in [true, false] {
                let mut gap = 0.0;
                let mut k = i;
                while sharpest {
                    let next = if forward {
                        (k + 1) % n
                    } else {
                        (k + n - 1) % n
                    };
                    gap += (ring[next] - ring[k]).hypot();
                    if gap > TURN_REACH_MM || next == i {
                        break;
                    }
                    sharpest = !beaten(next);
                    k = next;
                }
            }
            if sharpest {
                found.push(ring[i]);
            }
        }
    }
    found
}

/// Runs each chain into the tips of `rings` it passes without reaching (a crest too wide for
/// a spur, a drop's short tail): the chain's sample whose disc comes nearest a tip takes it
/// when [`takes_tip`] says so. Whether any did.
pub(super) fn join_outline_tips(
    chains: &mut [Chain],
    rings: &[Vec<Point>],
    boundary: &Boundary,
) -> bool {
    let mut joined = false;
    let tips = outline_tips(rings);
    for &tip in &tips {
        // The nearest disc over every chain: a tip already reached, or another chain's end,
        // stops the search.
        let mut best: Option<(f64, usize, usize)> = None;
        for (chain, path) in chains.iter().enumerate() {
            for (index, sample) in path.samples.iter().enumerate() {
                let gap = (sample.point - tip).hypot() - sample.radius;
                if best.is_none_or(|(least, _, _)| gap < least) {
                    best = Some((gap, chain, index));
                }
            }
        }
        let Some((_, chain, index)) = best else {
            continue;
        };
        if !takes_tip(&chains[chain], index, tip, &tips) {
            continue;
        }
        let corner = CornerTip {
            index,
            at: chains[chain].samples[index],
            tip,
        };
        joined |= corner_tip(&mut chains[chain], &corner, boundary);
    }
    joined
}

/// Whether the sample `index` of `chain`, the nearest disc to the outline's tip `tip`, runs
/// into it: the tip lies within [`TIP_REACH_RADII`] of its radius and is not yet painted, the
/// chain runs on past the sample by its radius both ways (a tip at a free end is that end's
/// business), the tip lies beside the path (the outer corner of a bend), not ahead of it, and
/// no other tip lies close (two corners of a flat end).
fn takes_tip(chain: &Chain, index: usize, tip: Point, tips: &[Point]) -> bool {
    let samples = &chain.samples;
    let sample = samples[index];
    let reach = (sample.point - tip).hypot();
    if reach - sample.radius <= TIP_PAINTED_SHARE * sample.radius
        || reach > TIP_REACH_RADII * sample.radius
    {
        return false;
    }
    let flat = tips.iter().any(|&other| {
        let gap = (other - tip).hypot();
        gap > 0.0 && gap < FLAT_END_RADII * sample.radius
    });
    // The first samples a radius away either way, wrapping round a ring.
    let away = |step: isize| {
        let mut length = 0.0;
        let mut k = index;
        for _ in 1..samples.len() {
            let next = if chain.closed {
                (k.cast_signed() + step)
                    .rem_euclid(samples.len().cast_signed())
                    .cast_unsigned()
            } else {
                k.checked_add_signed(step).filter(|&k| k < samples.len())?
            };
            length += (samples[next].point - samples[k].point).hypot();
            k = next;
            if length >= sample.radius && (samples[k].point - sample.point).hypot() >= sample.radius
            {
                return Some(k);
            }
        }
        None
    };
    let beside = match (away(-1), away(1)) {
        (Some(behind), Some(ahead)) => {
            let along = samples[ahead].point - samples[behind].point;
            let towards = tip - sample.point;
            along.dot(towards).abs() <= TIP_BESIDE_COS * along.hypot() * towards.hypot()
        }
        _ => false,
    };
    beside && !flat
}

/// Replaces `samples[at]`, a tip (radius 0), by the legs into it and out of it along `spur`
/// (from the junction's centre to the tip): each leg runs midway between the spur and the
/// outline on its arm's side, half as wide as the shape there, so the two legs tile the tip
/// and each one's width follows the local width into the point (Join tips, CENTRE-TIPS).
pub(super) fn legs_to_tip(
    samples: &mut Vec<AxisSample>,
    at: usize,
    closed: bool,
    spur: &[AxisSample],
    boundary: &Boundary,
) {
    let n = samples.len();
    let (Some(first), Some(tip)) = (spur.first(), spur.last()) else {
        return;
    };
    let axis = tip.point - first.point;
    let neighbour = |k: Option<usize>| k.map(|k| samples[k].point);
    let before = neighbour(at.checked_sub(1).or(closed.then(|| n - 1)));
    let after = neighbour((at + 1 < n).then_some(at + 1).or(closed.then_some(0)));
    let side = |p: Option<Point>| p.map_or(0.0, |p| axis.cross(p - first.point));
    // The legs run on opposite sides of the spur; the arm farther off its line chooses.
    let (sb, sa) = (side(before), side(after));
    let before_side = if sb.abs() >= sa.abs() {
        sb.signum()
    } else {
        -sa.signum()
    };
    let spur = resampled(spur);
    let leg = |sign: f64| -> Vec<AxisSample> {
        spur[..spur.len() - 1]
            .iter()
            .filter_map(|s| {
                let foot = boundary
                    .nearest_where(s.point, |_, f| {
                        let d = f - s.point;
                        axis.cross(d) * sign > 0.0 && axis.dot(d) >= 0.0
                    })
                    .map(|(_, f, _)| f)?;
                // The disc on the foot's side reaches a little past the spur, so the two legs
                // overlap there.
                let across = (foot - s.point).hypot().min(2.0 * s.radius.max(0.0));
                let reach = across * (0.5 + LEG_OVERLAP);
                let inward = (s.point - foot) / (foot - s.point).hypot().max(1e-12);
                let centre = foot + inward * reach;
                // Never past the outline elsewhere (a notch beside the spur).
                let room = boundary
                    .nearest_where(centre, |_, _| true)
                    .map_or(reach, |(d, _, _)| d);
                Some(AxisSample {
                    point: centre,
                    radius: reach.min(room),
                })
            })
            .scan(f64::INFINITY, |narrowest, mut sample| {
                // The width only falls towards the tip: the traced outline's stairs go.
                *narrowest = sample.radius.min(*narrowest);
                sample.radius = *narrowest;
                Some(sample)
            })
            .collect()
    };
    if after.is_some() {
        let mut out = leg(-before_side);
        out.reverse();
        let tail = samples.split_off(at + 1);
        samples.extend(out);
        samples.extend(tail);
    }
    if before.is_some() {
        samples.splice(at..at, leg(before_side));
    }
}

/// Each leg's disc reaches this share of the spur's local width past its middle, across the
/// spur.
const LEG_OVERLAP: f64 = 0.25;
/// A spur's samples are at most this share of its first radius apart on the legs.
const LEG_STEP_SHARE: f64 = 0.5;
/// The most samples on one leg.
const LEG_STEPS_MAX: usize = 64;

/// `spur` resampled evenly along its polyline, positions and radii interpolated, its first and
/// last samples kept.
fn resampled(spur: &[AxisSample]) -> Vec<AxisSample> {
    let mut along = vec![0.0];
    for w in spur.windows(2) {
        along.push(along[along.len() - 1] + (w[1].point - w[0].point).hypot());
    }
    let total = along[along.len() - 1];
    let step = (LEG_STEP_SHARE * spur[0].radius).max(1e-3);
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let count = ((total / step).ceil() as usize).clamp(1, LEG_STEPS_MAX);
    (0..=count)
        .map(|k| {
            #[allow(clippy::cast_precision_loss)]
            let s = total * k as f64 / count as f64;
            let i = along.partition_point(|&a| a <= s).clamp(1, spur.len() - 1) - 1;
            let span = along[i + 1] - along[i];
            let t = if span > 0.0 {
                ((s - along[i]) / span).clamp(0.0, 1.0)
            } else {
                0.0
            };
            AxisSample {
                point: spur[i].point.lerp(spur[i + 1].point, t),
                radius: spur[i].radius + (spur[i + 1].radius - spur[i].radius) * t,
            }
        })
        .collect()
}

/// Samples of a corner tip's straight spur, from the corner to the tip.
const CORNER_SPUR_STEPS: usize = 8;

/// Runs `chain` into the outline's tip `corner.tip` from its sample `corner.at` (Join tips):
/// the samples within that sample's radius go, and the legs run into the tip as at a
/// junction's tip. Whether it did.
fn corner_tip(chain: &mut Chain, corner: &CornerTip, boundary: &Boundary) -> bool {
    let at = corner.index;
    let n = chain.samples.len();
    if !chain.closed && (at == 0 || at + 1 == n) {
        return false;
    }
    let near = |s: &AxisSample| (s.point - corner.at.point).hypot() < corner.at.radius;
    // The samples within the corner's radius either side of it, wrapping on a ring.
    let mut keep = vec![true; n];
    keep[at] = false;
    for step in [1_isize, -1] {
        let mut k = at;
        for _ in 1..n {
            k = if chain.closed {
                (k.cast_signed() + step)
                    .rem_euclid(n.cast_signed())
                    .cast_unsigned()
            } else {
                match k.checked_add_signed(step).filter(|&k| k < n) {
                    Some(k) => k,
                    None => break,
                }
            };
            if !near(&chain.samples[k]) || (!chain.closed && (k == 0 || k + 1 == n)) {
                break;
            }
            keep[k] = false;
        }
    }
    let tip = AxisSample {
        point: corner.tip,
        radius: 0.0,
    };
    let mut samples = Vec::with_capacity(n);
    let mut tip_index = 0;
    for (k, s) in chain.samples.iter().enumerate() {
        if k == at {
            tip_index = samples.len();
            samples.push(tip);
        } else if keep[k] {
            samples.push(*s);
        }
    }
    if samples.len() < 3 {
        return false;
    }
    let spur: Vec<AxisSample> = (0..=CORNER_SPUR_STEPS)
        .map(|k| {
            #[allow(clippy::cast_precision_loss)]
            let t = k as f64 / CORNER_SPUR_STEPS as f64;
            let point = corner.at.point.lerp(corner.tip, t);
            AxisSample {
                point,
                radius: if k == CORNER_SPUR_STEPS {
                    0.0
                } else {
                    corner.at.radius * (1.0 - t)
                },
            }
        })
        .collect();
    legs_to_tip(&mut samples, tip_index, chain.closed, &spur, boundary);
    chain.samples = samples;
    true
}
