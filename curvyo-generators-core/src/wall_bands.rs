//! The slit bands of a developed wall: the stretches that carry columns of slits, and where the
//! columns of a run of bends stand (ADR 0082 §6, ADR 0083 §4, §13.6).

use curvyo_document_core::Length;

use crate::footprint::{Shape, Span};
use crate::slit_bend::{Band, SlitPattern, Slot, Window, slots, slots_at};
use crate::wall::{LocalSpacing, Wall, column_pitch};

/// One band and the bends it lies on.
#[derive(Debug, Clone)]
pub(crate) struct BandSpec {
    pub(crate) band: Band,
    /// `(start, end, inner radius)` of every bend the band lies on, along the strip.
    pub(crate) arcs: Vec<(f64, f64, f64)>,
    /// The shape of each of those bends, for the stretch of the inner face under the band.
    pub(crate) shapes: Vec<Shape>,
}

/// The columns of a run: where they stand along the strip and the period each takes. N =
/// max(2, floor(∫ ds / g)) columns at equal steps of the integral of 1 / g, the outer two half a
/// step from the ends (ADR 0083 §13.6).
pub(crate) fn run_columns(spec: &BandSpec, local: LocalSpacing) -> (Vec<f64>, Vec<f64>) {
    let (from, to) = (spec.band.from.as_mm(), spec.band.to.as_mm());
    let pieces: Vec<(f64, f64, f64)> = spec
        .arcs
        .iter()
        .filter_map(|&(s0, s1, radius)| {
            let (a, b) = (s0.max(from), s1.min(to));
            (b > a).then(|| (a, b, (local.factor * radius).max(local.least)))
        })
        .collect();
    let total: f64 = pieces.iter().map(|&(a, b, g)| (b - a) / g).sum();
    // Two columns at least (no uncut band), but never closer than the least gap: a stretch too
    // short for that gets fewer columns, one or none (ADR 0083 §13.6, lead decision).
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let fits = ((to - from) / local.least + 1e-9).floor() as usize;
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let columns = (total.floor() as usize).max(2).min(fits);
    if columns == 0 {
        return (Vec::new(), Vec::new());
    }
    #[allow(clippy::cast_precision_loss)]
    let step = (to - from) / columns as f64;
    #[allow(clippy::cast_precision_loss)]
    let step_of_integral = total / columns as f64;
    let mut xs = Vec::with_capacity(columns);
    for column in 0..columns {
        #[allow(clippy::cast_precision_loss)]
        let target = step_of_integral * (column as f64 + 0.5);
        let mut done = 0.0;
        let mut x = to;
        for &(a, b, g) in &pieces {
            let part = (b - a) / g;
            if target <= done + part || (a, b, g) == pieces[pieces.len() - 1] {
                x = a + (target - done).clamp(0.0, part) * g;
                break;
            }
            done += part;
        }
        xs.push(x);
    }
    let periods = (0..columns)
        .map(|k| {
            let before = k.checked_sub(1).map(|i| xs[k] - xs[i]);
            let after = xs.get(k + 1).map(|next| next - xs[k]);
            match (before, after) {
                (Some(a), Some(b)) => a.min(b),
                (Some(a), None) => a,
                (None, Some(b)) => b,
                (None, None) => step,
            }
        })
        .collect();
    (xs, periods)
}

/// The bends of `spans` as `(start, end, inner radius)` along the strip, and their shapes.
fn arcs_of(spans: &[Span]) -> (Vec<(f64, f64, f64)>, Vec<Shape>) {
    let arcs = spans
        .iter()
        .map(|s| {
            let radius = match s.shape {
                Shape::Bend { radius, .. } => radius,
                Shape::Straight { .. } => 0.0,
            };
            (s.s0, s.s0 + s.length, radius)
        })
        .collect();
    (arcs, spans.iter().map(|s| s.shape).collect())
}

impl Wall<'_> {
    /// The last span of the run that starts at the bend `first`: with `runs` the bends that
    /// follow it without a piece start, else `first` alone.
    fn run_end(&self, first: usize) -> usize {
        let spans = &self.footprint.spans;
        let mut last = first;
        while self.runs
            && last + 1 < spans.len()
            && matches!(spans[last + 1].shape, Shape::Bend { .. })
            && !spans[last + 1].piece_start
        {
            last += 1;
        }
        last
    }

    /// The stretches that carry slit columns, with the bends they lie on: one band per bend, or
    /// (`runs`) one per run of consecutive bends; or the whole strip.
    pub(crate) fn bands(&self, length: f64, height: f64) -> Vec<BandSpec> {
        let bridge = self.pattern.bridge.as_mm();
        let (lead_min, trail_min) = self.ends.as_ref().map_or_else(
            || (bridge.max(self.seam_margin), bridge.max(self.seam_margin)),
            |e| (e.lead.max(bridge), e.trail.max(bridge)),
        );
        let height = Length::from_mm(height);
        let band = |from: f64, to: f64| Band {
            from: Length::from_mm(from.max(lead_min)),
            to: Length::from_mm(to.min(length - trail_min)),
            height,
        };
        if self.slits_everywhere {
            return vec![BandSpec {
                band: band(0.0, length),
                arcs: vec![(0.0, length, 0.0)],
                shapes: Vec::new(),
            }];
        }
        // A bend ends one bridge from a window; at a straight it runs on to the junction.
        let spans = &self.footprint.spans;
        let is_bend = |i: usize| matches!(spans[i].shape, Shape::Bend { .. });
        let mut out = Vec::new();
        let mut first = 0;
        while first < spans.len() {
            if !is_bend(first) {
                first += 1;
                continue;
            }
            let last = self.run_end(first);
            let before = first.checked_sub(1).and_then(|i| spans.get(i));
            let after = spans.get(last + 1);
            let lead = if before.is_some_and(|s| s.window) {
                bridge
            } else {
                0.0
            };
            let trail = if after.is_some_and(|s| s.window) {
                bridge
            } else {
                0.0
            };
            let (start, finish) = (spans[first].s0, spans[last].s0 + spans[last].length);
            let (arcs, shapes) = arcs_of(&spans[first..=last]);
            let spec = BandSpec {
                band: band(start + lead, finish - trail),
                arcs,
                shapes,
            };
            if spec.band.to.as_mm() > spec.band.from.as_mm() {
                out.push(spec);
            }
            first = last + 1;
        }
        out
    }

    /// The pattern of a single bend's band: with local spacing, the columns are fitted by floor
    /// so no gap is below the spacing the radius asks for (the two-column minimum aside).
    fn pattern_for(&self, band: &Band, radius: f64) -> SlitPattern {
        match self.local_spacing {
            Some(local) if radius > 0.0 => SlitPattern {
                spacing: Length::from_mm(column_pitch(
                    band.from.as_mm(),
                    band.to.as_mm(),
                    (local.factor * radius).max(local.least),
                )),
                ..self.pattern
            },
            _ => self.pattern,
        }
    }

    fn run_spacing(&self) -> Option<LocalSpacing> {
        self.local_spacing.filter(|_| self.runs)
    }

    /// The slots of one band.
    pub(crate) fn band_slots(&self, spec: &BandSpec, windows: &[Window]) -> Vec<Slot> {
        if let Some(local) = self.run_spacing() {
            let (xs, periods) = run_columns(spec, local);
            return slots_at(&spec.band, (&xs, &periods), &self.pattern, windows);
        }
        let pattern = self.pattern_for(&spec.band, spec.arcs[0].2);
        slots(&spec.band, &pattern, windows)
    }

    /// The gap between two columns of every band that has two (the smallest of them is the
    /// reported automatic spacing).
    pub(crate) fn pitches(&self) -> Vec<f64> {
        let length = self.footprint.total();
        self.bands(length, self.height.as_mm())
            .iter()
            .filter_map(|spec| match self.run_spacing() {
                Some(local) => {
                    let (xs, periods) = run_columns(spec, local);
                    periods
                        .into_iter()
                        .reduce(f64::min)
                        .filter(|_| xs.len() >= 2)
                }
                None => Some(self.pattern_for(&spec.band, spec.arcs[0].2).spacing.as_mm()),
            })
            .collect()
    }

    /// The length of every band of a run that cannot hold two columns at the least
    /// gap (it gets one or none, ADR 0083 §13.6); empty for the rounded box.
    pub(crate) fn short_bands(&self) -> Vec<f64> {
        let Some(local) = self.run_spacing() else {
            return Vec::new();
        };
        let length = self.footprint.total();
        self.bands(length, self.height.as_mm())
            .iter()
            .filter(|spec| run_columns(spec, local).0.len() < 2)
            .map(|spec| spec.band.to.as_mm() - spec.band.from.as_mm())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(arcs: &[(f64, f64, f64)]) -> BandSpec {
        BandSpec {
            band: Band {
                from: Length::from_mm(arcs[0].0),
                to: Length::from_mm(arcs[arcs.len() - 1].1),
                height: Length::from_mm(50.0),
            },
            arcs: arcs.to_vec(),
            shapes: Vec::new(),
        }
    }

    const LOCAL: LocalSpacing = LocalSpacing {
        factor: 0.22,
        least: 1.3,
    };

    #[test]
    fn one_radius_gives_the_even_pitch_of_a_single_bend() {
        let band = spec(&[(10.0, 70.0, 30.0)]);
        let (xs, _) = run_columns(&band, LOCAL);
        let pitch = column_pitch(10.0, 70.0, 0.22 * 30.0);
        assert_eq!(xs.len(), 9, "60 / 6.6 floors to 9");
        for (k, x) in xs.iter().enumerate() {
            #[allow(clippy::cast_precision_loss)]
            let want = 10.0 + pitch * (k as f64 + 0.5);
            assert!((x - want).abs() < 1e-9);
        }
    }

    #[test]
    fn a_tight_arc_gets_denser_columns_than_a_wide_one_and_no_gap_is_below_the_least() {
        let band = spec(&[(0.0, 20.0, 10.0), (20.0, 80.0, 60.0)]);
        let (xs, periods) = run_columns(&band, LOCAL);
        let integral: f64 = 20.0 / 2.2 + 60.0 / 13.2;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let columns = integral.floor() as usize;
        assert_eq!(xs.len(), columns);
        assert!(xs.len() >= 2);
        let gaps: Vec<f64> = xs.windows(2).map(|w| w[1] - w[0]).collect();
        assert!(gaps.iter().all(|g| *g >= 1.3));
        assert!(gaps[0] < gaps[gaps.len() - 1], "denser on the tight arc");
        assert_eq!(periods.len(), xs.len());
        assert!(
            (periods[0] - gaps[0]).abs() < 1e-9,
            "a wave takes the smaller gap"
        );
        // The outer two columns stand half a step from the band's ends.
        assert!((xs[0] - gaps[0] / 2.0).abs() < gaps[0] * 0.6);
    }

    #[test]
    fn a_band_too_short_for_two_columns_still_gets_two() {
        let band = spec(&[(0.0, 3.0, 5.0)]);
        let (xs, _) = run_columns(&band, LOCAL);
        assert_eq!(xs.len(), 2);
    }

    /// Lead decision on criterion 6: the least gap holds everywhere. A stretch that cannot hold two
    /// columns that far apart gets one, or none.
    #[test]
    fn a_stretch_too_short_for_two_columns_at_the_least_gap_gets_one_or_none() {
        let one = run_columns(&spec(&[(0.0, 2.0, 5.0)]), LOCAL).0;
        assert_eq!(one.len(), 1, "2 mm holds one column at 1.3 mm");
        let none = run_columns(&spec(&[(0.0, 1.0, 5.0)]), LOCAL).0;
        assert!(none.is_empty(), "1 mm holds none");
        let two = run_columns(&spec(&[(0.0, 2.7, 5.0)]), LOCAL).0;
        assert_eq!(two.len(), 2);
        assert!(two[1] - two[0] >= 1.3 - 1e-9);
    }
}
