//! The samples of one Pencil stroke and their widths, computed as each sample arrives (ADR 0095
//! decision 4; `specs/0158-pen-display` slice 1 criteria 3, 7, 8, 15).

use curvyo_document_core::{Length, Point};
use curvyo_geometry_core::StrokeSample;

use super::PencilSettings;
use crate::pen_sample::Pressure;
use crate::pencil_pressure::{is_usable_pressure, smoothed_at};
use crate::speed_pressure::speed_pressure_at;

/// The shortest stroke that becomes a path, in millimetres (`0032` criterion 12).
pub const MIN_STROKE_LENGTH_MM: f64 = 0.1;
/// The fewest samples that become a path.
pub const MIN_STROKE_SAMPLES: usize = 2;
/// How many trailing widths a new sample can change: its own and, through the 3-sample mean and
/// the first sample's speed, the two before it.
const AFFECTED_WIDTHS: usize = 3;

/// Where a stroke's widths come from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WidthSource {
    /// Pen pressure: some sample is neither 0 nor 0.5 (criteria 3, 7).
    Pressure,
    /// Simulated from speed: no pressure and "Simulate pressure" on (criterion 8).
    Speed,
    /// No profile: the stroke takes the style's width (criterion 7).
    Uniform,
}

/// One stroke in progress: document points, source-clock times, raw pressures and widths.
#[derive(Debug, Clone)]
pub struct PencilStroke {
    settings: PencilSettings,
    points: Vec<Point>,
    times: Vec<f64>,
    raw: Vec<f64>,
    speed: Vec<f64>,
    widths: Vec<Length>,
    usable: bool,
    length: f64,
}

impl PencilStroke {
    /// An empty stroke drawn with `settings` (taken at its start, criterion 9).
    #[must_use]
    pub const fn new(settings: PencilSettings) -> Self {
        Self {
            settings,
            points: Vec::new(),
            times: Vec::new(),
            raw: Vec::new(),
            speed: Vec::new(),
            widths: Vec::new(),
            usable: false,
            length: 0.0,
        }
    }

    /// Appends a sample; only the trailing widths it affects are recomputed, unless it changes
    /// the width source (the first usable pressure), which recomputes all once.
    pub fn push(&mut self, point: Point, pressure: Option<Pressure>, time_ms: f64) {
        let before = self.source();
        if let Some(&last) = self.points.last() {
            self.length += (point.x - last.x).hypot(point.y - last.y);
        }
        let p = pressure.map_or(0.0, Pressure::value);
        self.usable |= pressure.is_some() && is_usable_pressure(p);
        self.points.push(point);
        self.times.push(time_ms);
        self.raw.push(p);
        let n = self.points.len() - 1;
        self.speed
            .push(speed_pressure_at(&self.points, &self.times, n));
        if n == 1 {
            self.speed[0] = speed_pressure_at(&self.points, &self.times, 0);
        }
        self.widths.push(self.settings.widths.max());
        let from = if self.source() == before {
            (n + 1).saturating_sub(AFFECTED_WIDTHS)
        } else {
            0
        };
        for i in from..=n {
            self.widths[i] = self.width_at(i);
        }
    }

    fn width_at(&self, i: usize) -> Length {
        let (widths, curve) = (self.settings.widths, self.settings.curve);
        match self.source() {
            WidthSource::Pressure => widths.width_for(curve, smoothed_at(&self.raw, i)),
            WidthSource::Speed => widths.width_for(curve, smoothed_at(&self.speed, i)),
            WidthSource::Uniform => widths.max(),
        }
    }

    /// Where the widths come from so far.
    #[must_use]
    pub const fn source(&self) -> WidthSource {
        if self.usable {
            WidthSource::Pressure
        } else if self.settings.simulate {
            WidthSource::Speed
        } else {
            WidthSource::Uniform
        }
    }

    /// Whether some sample had a usable pressure (criterion 7's has-pressure rule).
    #[must_use]
    pub const fn has_pressure(&self) -> bool {
        self.usable
    }

    /// The settings the stroke is drawn with.
    #[must_use]
    pub const fn settings(&self) -> &PencilSettings {
        &self.settings
    }

    /// The document points in order.
    #[must_use]
    pub fn points(&self) -> &[Point] {
        &self.points
    }

    /// The width at each point.
    #[must_use]
    pub fn widths(&self) -> &[Length] {
        &self.widths
    }

    /// Fewer than two samples or shorter than 0.1 mm: nothing is created (criterion 15).
    #[must_use]
    pub fn is_too_short(&self) -> bool {
        self.points.len() < MIN_STROKE_SAMPLES || self.length < MIN_STROKE_LENGTH_MM
    }

    /// The samples the fit takes.
    #[must_use]
    pub fn fit_samples(&self) -> Vec<StrokeSample> {
        self.points
            .iter()
            .zip(&self.widths)
            .map(|(&point, &width)| StrokeSample { point, width })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pencil_pressure::{PencilWidths, PressureCurve};

    fn settings(simulate: bool) -> PencilSettings {
        PencilSettings {
            simulate,
            ..PencilSettings::default()
        }
    }

    #[test]
    fn widths_follow_the_smoothed_pressure_as_samples_arrive() {
        let mut stroke = PencilStroke::new(settings(false));
        for (i, p) in [0.2, 0.2, 1.0, 0.2, 0.2].into_iter().enumerate() {
            #[allow(clippy::cast_precision_loss)]
            stroke.push(
                Point::new(i as f64, 0.0),
                Some(Pressure::new(p)),
                i as f64 * 8.0,
            );
        }
        assert_eq!(stroke.source(), WidthSource::Pressure);
        let w = PencilWidths::default();
        for (got, smoothed) in
            stroke
                .widths()
                .iter()
                .zip([0.2, 0.2, 1.4 / 3.0, 1.4 / 3.0, 1.4 / 3.0])
        {
            let want = w.width_for(PressureCurve::Linear, smoothed).as_mm();
            assert!((got.as_mm() - want).abs() < 1e-9);
        }
    }

    #[test]
    fn the_first_usable_pressure_recomputes_every_width() {
        let mut stroke = PencilStroke::new(settings(false));
        stroke.push(Point::new(0.0, 0.0), Some(Pressure::new(0.0)), 0.0);
        stroke.push(Point::new(1.0, 0.0), Some(Pressure::new(0.0)), 8.0);
        assert_eq!(stroke.source(), WidthSource::Uniform);
        assert!((stroke.widths()[0].as_mm() - 2.0).abs() < 1e-12);
        stroke.push(Point::new(2.0, 0.0), Some(Pressure::new(0.9)), 16.0);
        assert_eq!(stroke.source(), WidthSource::Pressure);
        assert!(
            (stroke.widths()[0].as_mm() - 0.2).abs() < 1e-12,
            "re-read as pressure 0"
        );
    }

    #[test]
    fn all_half_or_none_pressures_are_uniform_unless_simulated() {
        for pressure in [None, Some(Pressure::new(0.5)), Some(Pressure::new(0.0))] {
            let mut stroke = PencilStroke::new(settings(false));
            for i in 0..5 {
                stroke.push(Point::new(f64::from(i), 0.0), pressure, f64::from(i) * 8.0);
            }
            assert_eq!(stroke.source(), WidthSource::Uniform);
            assert!(!stroke.has_pressure());
        }
        let mut stroke = PencilStroke::new(settings(true));
        for i in 0..5 {
            stroke.push(Point::new(f64::from(i), 0.0), None, f64::from(i) * 8.0);
        }
        assert_eq!(stroke.source(), WidthSource::Speed);
        // 125 mm/s everywhere: one width, the first sample included.
        let first = stroke.widths()[0].as_mm();
        assert!(
            stroke
                .widths()
                .iter()
                .all(|w| (w.as_mm() - first).abs() < 1e-9)
        );
        assert!(first > 0.2 && first < 2.0);
    }

    #[test]
    fn simulation_is_ignored_when_the_pen_has_pressure() {
        let mut stroke = PencilStroke::new(settings(true));
        for i in 0..4 {
            stroke.push(
                Point::new(f64::from(i), 0.0),
                Some(Pressure::new(0.8)),
                f64::from(i),
            );
        }
        assert_eq!(stroke.source(), WidthSource::Pressure);
    }

    #[test]
    fn too_short_strokes() {
        let mut stroke = PencilStroke::new(settings(false));
        stroke.push(Point::new(0.0, 0.0), None, 0.0);
        assert!(stroke.is_too_short());
        stroke.push(Point::new(0.05, 0.0), None, 1.0);
        assert!(stroke.is_too_short());
        stroke.push(Point::new(0.11, 0.0), None, 2.0);
        assert!(!stroke.is_too_short());
    }
}
