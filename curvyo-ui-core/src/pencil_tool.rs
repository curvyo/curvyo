//! The Pencil's gesture, from pen samples to a finished stroke or an erase (`specs/0158-pen-display`
//! slice 1 criteria 1, 11, 12, 15; ADR 0095 decision 3).
//!
//! The session converts each sample's position to document space and hands the samples over in
//! order, one at a time; the tool answers what the sample did. Samples split over batches give
//! the same answers as one batch, because the tool keeps no batch state.

mod stroke;

use curvyo_document_core::{Length, Point};

pub use stroke::{MIN_STROKE_LENGTH_MM, MIN_STROKE_SAMPLES, PencilStroke, WidthSource};

use crate::pen_sample::{PenEnd, PenPhase, PenSample};
use crate::pencil_pressure::{PencilWidths, PressureCurve};

/// The Smoothing value at start: 40 %, read as 0.4 mm as the Freehand slider does (criterion 9).
pub const DEFAULT_PENCIL_SMOOTHING_MM: f64 = 0.4;

/// The Pencil's settings for the session, never stored in the document (criteria 9, 17).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PencilSettings {
    /// The fit accuracy (Smoothing percent / 100 in mm).
    pub smoothing: Length,
    /// "Simulate pressure": widths from speed for a stroke without pressure.
    pub simulate: bool,
    /// Min and Max.
    pub widths: PencilWidths,
    /// The pressure curve.
    pub curve: PressureCurve,
}

impl Default for PencilSettings {
    fn default() -> Self {
        Self {
            smoothing: Length::from_mm(DEFAULT_PENCIL_SMOOTHING_MM),
            simulate: false,
            widths: PencilWidths::default(),
            curve: PressureCurve::default(),
        }
    }
}

/// What one sample did.
#[derive(Debug, Clone)]
pub enum PencilOutcome {
    /// A hovering or out-of-range sample with nothing in progress: the pointer moved.
    Hover(PenEnd),
    /// The tip touched: a stroke started (the session may refuse it with [`PencilTool::escape`]).
    Started,
    /// The eraser end touched: an erase started.
    EraseStarted,
    /// A contact sample of the gesture in progress.
    Moved,
    /// The tip lifted from a stroke long enough to keep.
    Finished(PencilStroke),
    /// The tip lifted from a stroke too short to keep (criterion 15).
    TooShort,
    /// The eraser end lifted: the document points of its path.
    Erased(Vec<Point>),
    /// A sample of a contact Escape cancelled.
    Ignored,
}

#[derive(Debug, Clone, Default)]
enum State {
    #[default]
    Idle,
    Drawing(PencilStroke),
    Erasing(Vec<Point>),
    /// Escape cancelled the gesture; the rest of the contact draws nothing.
    Ignoring,
}

/// The Pencil's state machine.
#[derive(Debug, Clone, Default)]
pub struct PencilTool {
    state: State,
}

impl PencilTool {
    /// A tool with nothing in progress.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Takes one sample at document `point`; `settings` are read when a stroke starts.
    pub fn sample(
        &mut self,
        point: Point,
        sample: &PenSample,
        settings: &PencilSettings,
    ) -> PencilOutcome {
        let contact = sample.phase == PenPhase::Contact;
        match (&mut self.state, contact) {
            (State::Idle, true) => {
                if sample.end == PenEnd::Eraser {
                    self.state = State::Erasing(vec![point]);
                    PencilOutcome::EraseStarted
                } else {
                    let mut stroke = PencilStroke::new(*settings);
                    stroke.push(point, sample.pressure, sample.time_ms);
                    self.state = State::Drawing(stroke);
                    PencilOutcome::Started
                }
            }
            (State::Idle, false) => PencilOutcome::Hover(sample.end),
            (State::Drawing(stroke), true) => {
                stroke.push(point, sample.pressure, sample.time_ms);
                PencilOutcome::Moved
            }
            (State::Erasing(path), true) => {
                path.push(point);
                PencilOutcome::Moved
            }
            (State::Ignoring, true) => PencilOutcome::Ignored,
            (_, false) => match std::mem::take(&mut self.state) {
                State::Drawing(stroke) if stroke.is_too_short() => PencilOutcome::TooShort,
                State::Drawing(stroke) => PencilOutcome::Finished(stroke),
                State::Erasing(path) => PencilOutcome::Erased(path),
                State::Idle | State::Ignoring => PencilOutcome::Ignored,
            },
        }
    }

    /// Escape: drops the stroke or erase in progress, and the rest of its contact draws nothing
    /// (criterion 15). Returns whether anything was in progress.
    pub fn escape(&mut self) -> bool {
        if matches!(self.state, State::Drawing(_) | State::Erasing(_)) {
            self.state = State::Ignoring;
            true
        } else {
            false
        }
    }

    /// Forgets everything (a tool switch, a cancelled pointer).
    pub fn reset(&mut self) {
        self.state = State::Idle;
    }

    /// The stroke in progress, for the preview.
    #[must_use]
    pub const fn stroke(&self) -> Option<&PencilStroke> {
        match &self.state {
            State::Drawing(stroke) => Some(stroke),
            _ => None,
        }
    }

    /// The eraser's path in progress.
    #[must_use]
    pub fn eraser_path(&self) -> Option<&[Point]> {
        match &self.state {
            State::Erasing(path) => Some(path),
            _ => None,
        }
    }

    /// Whether a contact is in progress (drawing, erasing or ignored).
    #[must_use]
    pub const fn in_contact(&self) -> bool {
        !matches!(self.state, State::Idle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ScreenPx;
    use crate::pen_sample::Pressure;

    fn sample(x: f64, phase: PenPhase, end: PenEnd, pressure: f64, time_ms: f64) -> PenSample {
        PenSample {
            x: ScreenPx(x),
            y: ScreenPx(0.0),
            pressure: Some(Pressure::new(pressure)),
            tilt: None,
            end,
            phase,
            barrel: 0,
            time_ms,
        }
    }

    /// A stroke of `n` contact samples from x = 0 to x = n - 1 mm, then a hover.
    fn stroke_samples(n: usize) -> Vec<PenSample> {
        #[allow(clippy::cast_precision_loss)]
        let mut out: Vec<PenSample> = (0..n)
            .map(|i| {
                sample(
                    i as f64,
                    PenPhase::Contact,
                    PenEnd::Tip,
                    0.1 + 0.8 * i as f64 / n as f64,
                    i as f64 * 8.0,
                )
            })
            .collect();
        #[allow(clippy::cast_precision_loss)]
        out.push(sample(
            n as f64,
            PenPhase::Hover,
            PenEnd::Tip,
            0.0,
            n as f64 * 8.0,
        ));
        out
    }

    fn feed(tool: &mut PencilTool, samples: &[PenSample]) -> Vec<PencilOutcome> {
        let settings = PencilSettings::default();
        samples
            .iter()
            .map(|s| tool.sample(Point::new(s.x.0, s.y.0), s, &settings))
            .collect()
    }

    fn finished(outcomes: &[PencilOutcome]) -> Vec<PencilStroke> {
        outcomes
            .iter()
            .filter_map(|o| match o {
                PencilOutcome::Finished(s) => Some(s.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_stroke_over_several_batches_equals_one_batch() {
        let samples = stroke_samples(50);
        let mut one = PencilTool::new();
        let whole = finished(&feed(&mut one, &samples));
        let mut split = PencilTool::new();
        let mut parts = Vec::new();
        for chunk in samples.chunks(7) {
            parts.extend(feed(&mut split, chunk));
        }
        let parts = finished(&parts);
        assert_eq!(whole.len(), 1);
        assert_eq!(parts.len(), 1);
        assert_eq!(whole[0].points(), parts[0].points());
        assert_eq!(whole[0].widths(), parts[0].widths());
    }

    #[test]
    fn hover_samples_start_nothing() {
        let mut tool = PencilTool::new();
        let outcomes = feed(
            &mut tool,
            &[
                sample(1.0, PenPhase::Hover, PenEnd::Tip, 0.0, 0.0),
                sample(2.0, PenPhase::Out, PenEnd::Tip, 0.0, 1.0),
            ],
        );
        assert!(
            outcomes
                .iter()
                .all(|o| matches!(o, PencilOutcome::Hover(PenEnd::Tip)))
        );
        assert!(!tool.in_contact());
    }

    #[test]
    fn two_strokes_in_one_batch_are_two_strokes() {
        let mut samples = stroke_samples(10);
        samples.extend(stroke_samples(10));
        let mut tool = PencilTool::new();
        assert_eq!(finished(&feed(&mut tool, &samples)).len(), 2);
    }

    #[test]
    fn escape_drops_the_stroke_and_the_rest_of_its_contact() {
        let samples = stroke_samples(20);
        let mut tool = PencilTool::new();
        feed(&mut tool, &samples[..5]);
        assert!(tool.escape());
        let rest = feed(&mut tool, &samples[5..]);
        assert!(rest.iter().all(|o| matches!(o, PencilOutcome::Ignored)));
        assert!(!tool.in_contact());
        assert!(!tool.escape());
    }

    #[test]
    fn a_tap_is_too_short() {
        let mut tool = PencilTool::new();
        let outcomes = feed(
            &mut tool,
            &[
                sample(1.0, PenPhase::Contact, PenEnd::Tip, 0.4, 0.0),
                sample(1.0, PenPhase::Hover, PenEnd::Tip, 0.0, 9.0),
            ],
        );
        assert!(matches!(outcomes[1], PencilOutcome::TooShort));
    }

    #[test]
    fn the_eraser_end_erases_along_its_path() {
        let mut tool = PencilTool::new();
        let outcomes = feed(
            &mut tool,
            &[
                sample(1.0, PenPhase::Hover, PenEnd::Eraser, 0.0, 0.0),
                sample(1.0, PenPhase::Contact, PenEnd::Eraser, 0.5, 1.0),
                sample(2.0, PenPhase::Contact, PenEnd::Eraser, 0.5, 2.0),
                sample(3.0, PenPhase::Hover, PenEnd::Eraser, 0.0, 3.0),
            ],
        );
        assert!(matches!(outcomes[0], PencilOutcome::Hover(PenEnd::Eraser)));
        assert!(matches!(outcomes[1], PencilOutcome::EraseStarted));
        let PencilOutcome::Erased(path) = &outcomes[3] else {
            panic!("{outcomes:?}");
        };
        assert_eq!(path.len(), 2);
    }

    #[test]
    fn tilt_and_barrel_change_nothing() {
        let plain = stroke_samples(12);
        let mut tilted = plain.clone();
        for s in &mut tilted {
            s.tilt = Some(crate::pen_sample::Tilt::new(0.4, -0.3));
            s.barrel = 3;
        }
        let a = finished(&feed(&mut PencilTool::new(), &plain));
        let b = finished(&feed(&mut PencilTool::new(), &tilted));
        assert_eq!(a[0].points(), b[0].points());
        assert_eq!(a[0].widths(), b[0].widths());
    }
}
