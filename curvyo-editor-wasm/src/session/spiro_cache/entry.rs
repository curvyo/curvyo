//! One solve of a Spiro curve's controls, kept while they stay.

use curvyo_document_core::{NewAnchor, Tolerance};
use curvyo_geometry_core::SpiroShape;

/// One solve: the controls it was made for, their shape and anchors; none for controls without
/// a curve (too few or not finite).
#[derive(Debug, Clone)]
pub(super) struct Entry {
    pub(super) controls: Vec<NewAnchor>,
    pub(super) closed: bool,
    tolerance: f64,
    pub(super) shape: Option<SpiroShape>,
    pub(super) output: Option<Vec<NewAnchor>>,
}

impl Entry {
    fn of_shape(controls: &[NewAnchor], closed: bool, tolerance: f64, shape: SpiroShape) -> Self {
        let output = curvyo_ui_core::spiro_shape_nodes(
            controls,
            closed,
            &shape,
            Tolerance::from_mm(tolerance),
        )
        .ok();
        Self {
            controls: controls.to_vec(),
            closed,
            tolerance,
            shape: output.is_some().then_some(shape),
            output,
        }
    }

    /// `controls` solved: the same curve wherever they come from (`specs/0114` criterion 2).
    pub(super) fn fresh(controls: &[NewAnchor], closed: bool, tolerance: f64) -> Self {
        match curvyo_ui_core::solve_spiro_nodes(controls, closed) {
            Ok(shape) => Self::of_shape(controls, closed, tolerance, shape),
            Err(_) => Self {
                controls: controls.to_vec(),
                closed,
                tolerance,
                shape: None,
                output: None,
            },
        }
    }

    /// The same shape drawn to `tolerance`.
    pub(super) fn at_tolerance(&self, tolerance: f64) -> Self {
        match &self.shape {
            Some(shape) => Self::of_shape(&self.controls, self.closed, tolerance, shape.clone()),
            None => self.clone(),
        }
    }

    pub(super) fn same_controls(&self, controls: &[NewAnchor], closed: bool) -> bool {
        self.controls == controls && self.closed == closed
    }

    pub(super) fn serves(&self, controls: &[NewAnchor], closed: bool, tolerance: f64) -> bool {
        self.same_controls(controls, closed) && self.tolerance <= tolerance
    }
}
