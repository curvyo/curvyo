//! Samples a centre line adaptively for the variable-width outline (ADR 0053, amendment of
//! 2026-11-10, item 2): de Casteljau subdivision until each piece is flat and turns little, then
//! forced samples at every width point and refinement until the linear width is close enough.

use kurbo::{CubicBez, ParamCurve, ParamCurveDeriv, Point, Vec2};

/// The deepest subdivision of one segment; it stops a cusp from splitting forever.
const MAX_DEPTH: u32 = 24;

/// Distances below this, in millimetres, count as one point.
const COINCIDENT_MM: f64 = 1e-9;

/// The construction targets in millimetres and radians, derived from the tolerance.
#[derive(Debug, Clone, Copy)]
pub(super) struct Targets {
    /// The largest distance of a control point from its piece's chord.
    pub chord: f64,
    /// The largest turn of a piece's control polygon.
    pub turn: f64,
    /// The largest error of a linearly interpolated half width.
    pub width: f64,
}

impl Targets {
    /// The targets for `tolerance` millimetres and a largest stroke width of `max_width`:
    /// 0.2, 0.4 and 0.2 of the tolerance (0.002, 0.004 and 0.002 mm at 0.01 mm).
    pub fn new(tolerance: f64, max_width: f64) -> Self {
        let turn = if max_width > 0.0 {
            (8.0 * 0.4 * tolerance / max_width).sqrt().min(0.5)
        } else {
            0.5
        };
        Self {
            chord: 0.2 * tolerance,
            turn,
            width: 0.2 * tolerance,
        }
    }
}

/// One sample of the centre line.
#[derive(Debug, Clone, Copy)]
pub(super) struct Sample {
    /// The point on the centre line.
    pub point: Point,
    /// The unit direction of travel (the chord direction where the derivative vanishes).
    pub tangent: Vec2,
    /// The cumulative chord length from the start of the path.
    pub s: f64,
    /// The segment the sample lies on.
    pub segment: usize,
    /// The parameter on that segment.
    pub t: f64,
}

/// The samples of `segments` in path order: every segment from `t = 0` to `t = 1`, so the
/// first sample of a segment repeats the last point of the one before. `half_at` gives the half
/// width at a normalised position; `forced` are the normalised positions that must be samples.
/// `None` when the centre line has no length.
pub(super) fn sample_path(
    segments: &[CubicBez],
    targets: Targets,
    forced: &[f64],
    half_at: &dyn Fn(f64) -> f64,
) -> Option<(Vec<Sample>, f64)> {
    let mut samples = Vec::new();
    for (index, cubic) in segments.iter().enumerate() {
        samples.push(raw(cubic, index, 0.0));
        subdivide(cubic, index, 0.0, 1.0, targets, 0, &mut samples);
    }
    let total = accumulate(&mut samples);
    if !(total > COINCIDENT_MM && total.is_finite()) {
        return None;
    }
    for &position in forced {
        insert_at(&mut samples, segments, position * total);
    }
    refine_widths(&mut samples, segments, targets, total, half_at);
    let total = accumulate(&mut samples);
    set_tangents(&mut samples, segments);
    Some((samples, total))
}

/// A sample at `t` of segment `index`, its length and tangent still unset.
fn raw(cubic: &CubicBez, index: usize, t: f64) -> Sample {
    Sample {
        point: cubic.eval(t),
        tangent: Vec2::ZERO,
        s: 0.0,
        segment: index,
        t,
    }
}

/// Pushes the samples of `cubic` on `(t0, t1]`, splitting until flat and turning little.
fn subdivide(
    cubic: &CubicBez,
    index: usize,
    t0: f64,
    t1: f64,
    targets: Targets,
    depth: u32,
    out: &mut Vec<Sample>,
) {
    let piece = cubic.subsegment(t0..t1);
    if depth >= MAX_DEPTH || (flat(&piece, targets.chord) && turn(&piece) <= targets.turn) {
        out.push(raw(cubic, index, t1));
        return;
    }
    let middle = f64::midpoint(t0, t1);
    subdivide(cubic, index, t0, middle, targets, depth + 1, out);
    subdivide(cubic, index, middle, t1, targets, depth + 1, out);
}

/// Whether both inner control points of `piece` lie within `chord` of its chord.
fn flat(piece: &CubicBez, chord: f64) -> bool {
    let base = piece.p3 - piece.p0;
    let length = base.hypot();
    let distance = |p: Point| {
        if length <= COINCIDENT_MM {
            (p - piece.p0).hypot()
        } else {
            (p - piece.p0).cross(base).abs() / length
        }
    };
    distance(piece.p1) <= chord && distance(piece.p2) <= chord
}

/// The total turn of `piece`'s control polygon in radians, legs of no length skipped.
fn turn(piece: &CubicBez) -> f64 {
    let legs = [
        piece.p1 - piece.p0,
        piece.p2 - piece.p1,
        piece.p3 - piece.p2,
    ];
    let mut previous: Option<Vec2> = None;
    let mut total = 0.0;
    for leg in legs {
        if leg.hypot() <= COINCIDENT_MM {
            continue;
        }
        if let Some(before) = previous {
            total += before.cross(leg).atan2(before.dot(leg)).abs();
        }
        previous = Some(leg);
    }
    total
}

/// Sets each sample's cumulative chord length and returns the total.
fn accumulate(samples: &mut [Sample]) -> f64 {
    let mut s = 0.0;
    let mut previous: Option<Point> = None;
    for sample in samples.iter_mut() {
        if let Some(point) = previous {
            s += (sample.point - point).hypot();
        }
        sample.s = s;
        previous = Some(sample.point);
    }
    s
}

/// Inserts a sample at chord length `s` unless one is already there.
fn insert_at(samples: &mut Vec<Sample>, segments: &[CubicBez], s: f64) {
    let Some(index) = samples
        .windows(2)
        .position(|pair| pair[0].s <= s && s <= pair[1].s && pair[1].s > pair[0].s)
    else {
        return;
    };
    let (a, b) = (samples[index], samples[index + 1]);
    if (s - a.s).abs() <= COINCIDENT_MM || (b.s - s).abs() <= COINCIDENT_MM {
        return;
    }
    let fraction = (s - a.s) / (b.s - a.s);
    let t = a.t + (b.t - a.t) * fraction;
    let mut sample = raw(&segments[a.segment], a.segment, t);
    sample.s = s;
    samples.insert(index + 1, sample);
}

/// Splits every interval whose linear half width strays more than the target from the profile.
fn refine_widths(
    samples: &mut Vec<Sample>,
    segments: &[CubicBez],
    targets: Targets,
    total: f64,
    half_at: &dyn Fn(f64) -> f64,
) {
    let mut refined = Vec::with_capacity(samples.len());
    let mut stack: Vec<(Sample, u32)> = Vec::new();
    let mut iter = samples.iter().copied();
    let Some(first) = iter.next() else {
        return;
    };
    refined.push(first);
    for next in iter {
        stack.push((next, 0));
        while let Some((b, depth)) = stack.pop() {
            // invariant: `refined` starts with `first` and only grows.
            let Some(&a) = refined.last() else { break };
            let same = a.segment == b.segment && b.s > a.s;
            if same && depth < MAX_DEPTH {
                let (ha, hb) = (half_at(a.s / total), half_at(b.s / total));
                // Smoothstep strays most from its chord near the quarters, not at the middle.
                let strays = [0.25, 0.5, 0.75].into_iter().any(|f: f64| {
                    let s = a.s + (b.s - a.s) * f;
                    (half_at(s / total) - (ha + (hb - ha) * f)).abs() > targets.width
                });
                if strays {
                    let mut middle = raw(&segments[a.segment], a.segment, f64::midpoint(a.t, b.t));
                    middle.s = a.s + (middle.point - a.point).hypot();
                    stack.push((b, depth + 1));
                    stack.push((middle, depth + 1));
                    continue;
                }
            }
            refined.push(b);
        }
    }
    *samples = refined;
}

/// Sets each sample's unit tangent: the derivative, or the chord to a neighbour on the same
/// segment where the derivative vanishes.
fn set_tangents(samples: &mut [Sample], segments: &[CubicBez]) {
    for index in 0..samples.len() {
        let sample = samples[index];
        let derivative = segments[sample.segment].deriv().eval(sample.t).to_vec2();
        let tangent = if derivative.hypot() > COINCIDENT_MM {
            derivative / derivative.hypot()
        } else {
            neighbour_chord(samples, index)
        };
        samples[index].tangent = tangent;
    }
}

/// The unit chord from the neighbouring samples on the same segment.
fn neighbour_chord(samples: &[Sample], index: usize) -> Vec2 {
    let here = samples[index];
    let after = samples
        .get(index + 1)
        .filter(|next| next.segment == here.segment);
    let before = index
        .checked_sub(1)
        .map(|i| samples[i])
        .filter(|previous| previous.segment == here.segment);
    let chord = match (before, after) {
        (Some(a), Some(b)) => b.point - a.point,
        (None, Some(b)) => b.point - here.point,
        (Some(a), None) => here.point - a.point,
        (None, None) => Vec2::ZERO,
    };
    if chord.hypot() > COINCIDENT_MM {
        chord / chord.hypot()
    } else {
        Vec2::ZERO
    }
}

/// Cuts every interval on one segment into `density` equal parameter steps and returns the new
/// total: the dense reference of the same construction.
pub(super) fn densify(samples: &mut Vec<Sample>, segments: &[CubicBez], density: u32) -> f64 {
    let mut dense = Vec::with_capacity(samples.len() * density as usize);
    for (index, sample) in samples.iter().enumerate() {
        if let Some(previous) = index.checked_sub(1).map(|i| samples[i])
            && previous.segment == sample.segment
        {
            for step in 1..density {
                let t = previous.t + (sample.t - previous.t) * f64::from(step) / f64::from(density);
                dense.push(raw(&segments[sample.segment], sample.segment, t));
            }
        }
        dense.push(*sample);
    }
    *samples = dense;
    let total = accumulate(samples);
    set_tangents(samples, segments);
    total
}
