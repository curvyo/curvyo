//! The Pencil tool's stroke state machine (`specs/0032-pen-tablet-input` criteria 3 to 5, 7, 8
//! and 12): a press with the tip in contact starts a stroke, moves add samples, the release
//! hands the finished stroke over, Escape drops it.
//!
//! The samples are ephemeral (ADR 0009 §2): nothing here touches the document. The caller turns
//! a finished stroke into one commit.

use curvyo_document_core::{Point, Tolerance};
use curvyo_geometry_core::{OutlineTriple, WidthProfile, fit_freehand, variable_outline};

use crate::pencil_pressure::PencilSettings;
use crate::pointer_sample::{PointerKind, PointerSample};

/// The `buttons` bit of the primary contact: a pen tip touching, the left mouse button.
const PRIMARY_BUTTON: u32 = 1;

/// The fit's accuracy, a margin under criterion 7's 0.1 mm (`adrs.md` "the fit").
const FIT_ACCURACY: Tolerance = Tolerance::from_mm(0.08);

/// A stroke shorter than this creates nothing (criterion 12).
const MINIMUM_LENGTH_MM: f64 = 0.1;

/// How far a pressure must be from 0 and from 0.5 to count as real pressure (criterion 3). A
/// device without pressure reports exactly 0.5 or 0; this only absorbs float noise.
const PRESSURE_EPSILON: f64 = 1.0e-6;

/// Whether a stroke has pressure: it is drawn with a pen and at least one sample has a pressure
/// other than 0 and other than 0.5 (criterion 3).
#[must_use]
pub fn stroke_has_pressure(samples: &[PointerSample]) -> bool {
    samples.iter().any(|sample| {
        sample.kind == PointerKind::Pen
            && sample.pressure.is_finite()
            && sample.pressure.abs() > PRESSURE_EPSILON
            && (sample.pressure - 0.5).abs() > PRESSURE_EPSILON
    })
}

/// What a stroke becomes: an open centre line without pressure (criterion 7), the closed outline
/// of the centre line with its widths with pressure (criterion 8).
#[derive(Debug, Clone, PartialEq)]
pub enum StrokeShape {
    /// The fitted open centre line, anchors with relative handles.
    Centre(Vec<OutlineTriple>),
    /// The outlines of the variable-width stroke: outer outlines positive, holes negative.
    Outlines(Vec<Vec<Point>>),
}

/// A stroke that was released with at least two samples and 0.1 mm of length.
#[derive(Debug, Clone, PartialEq)]
pub struct FinishedStroke {
    samples: Vec<PointerSample>,
}

impl FinishedStroke {
    /// The sample positions in document millimetres.
    #[must_use]
    pub fn points(&self) -> Vec<Point> {
        self.samples.iter().map(|sample| sample.position).collect()
    }

    /// Whether the stroke has pressure (criterion 3).
    #[must_use]
    pub fn has_pressure(&self) -> bool {
        stroke_has_pressure(&self.samples)
    }

    /// The shape the stroke becomes with `settings`, `None` if the geometry gives nothing.
    #[must_use]
    pub fn shape(&self, settings: &PencilSettings) -> Option<StrokeShape> {
        shape_of(&self.samples, settings)
    }
}

/// The shape of the samples of a stroke (finished or in progress).
fn shape_of(samples: &[PointerSample], settings: &PencilSettings) -> Option<StrokeShape> {
    let points: Vec<Point> = samples.iter().map(|sample| sample.position).collect();
    let centre = fit_freehand(&points, FIT_ACCURACY);
    if centre.len() < 2 {
        return None;
    }
    if !stroke_has_pressure(samples) {
        return Some(StrokeShape::Centre(centre));
    }
    let pressures: Vec<f64> = samples.iter().map(|sample| sample.pressure).collect();
    let profile = WidthProfile::from_samples(&points, &settings.widths(&pressures));
    variable_outline(&centre, &profile)
        .ok()
        .map(StrokeShape::Outlines)
}

/// The length of the polyline through the samples, in millimetres.
fn polyline_length(samples: &[PointerSample]) -> f64 {
    samples
        .windows(2)
        .map(|pair| pair[0].position.vector_to(pair[1].position).length())
        .sum()
}

/// The Pencil tool: idle, or drawing a stroke.
#[derive(Debug, Default)]
pub struct PencilTool {
    /// The samples of the stroke in progress, `None` while idle.
    stroke: Option<Vec<PointerSample>>,
}

impl PencilTool {
    /// A press: starts a stroke with `sample` when the primary contact is down and the pointer is
    /// not a finger (criteria 4 and 5). Returns whether a stroke started.
    pub fn press(&mut self, sample: PointerSample, buttons: u32) -> bool {
        if sample.kind == PointerKind::Touch || buttons & PRIMARY_BUTTON == 0 {
            return false;
        }
        self.stroke = Some(vec![sample]);
        true
    }

    /// Adds the samples of a move to the stroke in progress; touch samples are ignored, and so is
    /// everything while idle.
    pub fn extend(&mut self, samples: &[PointerSample]) {
        if let Some(stroke) = &mut self.stroke {
            stroke.extend(
                samples
                    .iter()
                    .filter(|sample| sample.kind != PointerKind::Touch),
            );
        }
    }

    /// The release: adds its samples and hands the stroke over, or `None` when idle, with fewer
    /// than two samples, or under 0.1 mm long (criterion 12). The tool is idle afterwards.
    pub fn release(&mut self, samples: &[PointerSample]) -> Option<FinishedStroke> {
        self.extend(samples);
        let stroke = self.stroke.take()?;
        if stroke.len() < 2 || polyline_length(&stroke) < MINIMUM_LENGTH_MM {
            return None;
        }
        Some(FinishedStroke { samples: stroke })
    }

    /// Escape: drops the stroke in progress. Returns whether there was one (criterion 12).
    pub fn cancel(&mut self) -> bool {
        self.stroke.take().is_some()
    }

    /// Whether a stroke is in progress.
    #[must_use]
    pub const fn is_drawing(&self) -> bool {
        self.stroke.is_some()
    }

    /// The shape the stroke in progress would become now (criterion 11: the preview), `None`
    /// while idle or before it has a shape.
    #[must_use]
    pub fn preview(&self, settings: &PencilSettings) -> Option<StrokeShape> {
        shape_of(self.stroke.as_ref()?, settings)
    }
}
