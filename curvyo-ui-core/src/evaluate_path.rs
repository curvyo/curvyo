//! Derives ordinary Bézier geometry from editable path controls.

use curvyo_document_core::{
    Affine, AnchorId, AnchorKind, CurveKind, NewAnchor, ObjectSnapshot, PathSnapshot, Point,
    Tolerance, Vec2,
};
use curvyo_geometry_core::{
    SpiroControl, SpiroError, SpiroShape, SplineControl, SplineError, evaluate_bspline,
    solve_spiro, spiro_cubics,
};

/// How far a drawn Spiro may stray from its exact curve, in document mm.
pub const SPIRO_TOLERANCE: Tolerance = Tolerance::from_mm(0.01);

/// An editable curve could not be evaluated.
#[derive(Debug, thiserror::Error)]
pub enum EvaluatePathError {
    /// B-spline evaluation refused invalid or unrepresentable input.
    #[error(transparent)]
    BSpline(#[from] SplineError),
    /// The Spiro solver found no curve through the controls.
    #[error(transparent)]
    Spiro(#[from] SpiroError),
    /// This curve type does not yet have a validated evaluator.
    #[error("this curve cannot be evaluated")]
    Unsupported,
}

/// Produces artwork geometry while retaining clone identity and its shared root.
///
/// # Errors
/// Returns an error if an editable curve cannot be evaluated.
pub fn evaluate_object(object: &ObjectSnapshot) -> Result<ObjectSnapshot, EvaluatePathError> {
    Ok(match object {
        ObjectSnapshot::Path(path) => ObjectSnapshot::Path(evaluate_path(path)?),
        ObjectSnapshot::Clone(clone) => {
            let mut clone = clone.clone();
            clone.resolved = Box::new(evaluate_path(&clone.resolved)?);
            clone.chain.root_outline =
                std::sync::Arc::new(evaluate_path(&clone.chain.root_outline)?);
            ObjectSnapshot::Clone(clone)
        }
        other => other.clone(),
    })
}

/// Derives geometry for artwork without modifying the editable controls.
/// The resulting anchor ids are local to this geometry and must not be used for node edits.
///
/// # Errors
/// Returns an error if the curve cannot be evaluated.
pub fn evaluate_path(path: &PathSnapshot) -> Result<PathSnapshot, EvaluatePathError> {
    match path.curve_kind {
        CurveKind::Bezier => Ok(path.clone()),
        CurveKind::Spiro => {
            if path.is_compound() {
                return Err(EvaluatePathError::Unsupported);
            }
            // The controls are local to the path's placement: the curve is solved there, to a
            // tolerance tightened by the placement's stretch, and placed afterwards.
            let placement = path
                .spiro_transform
                .as_deref()
                .copied()
                .unwrap_or(Affine::IDENTITY);
            let tolerance = Tolerance::from_mm(SPIRO_TOLERANCE.as_mm() / stretch(placement));
            let mut result = path.clone();
            result.anchors = evaluate_spiro_nodes(&path.anchors, path.closed, tolerance)?;
            result.curve_kind = CurveKind::Bezier;
            result.spiro_transform = None;
            Ok(result.mapped(placement))
        }
        CurveKind::BSpline => {
            if path.is_compound() {
                return Err(EvaluatePathError::Unsupported);
            }
            let anchors = evaluate_bspline_nodes(&path.anchors, path.closed)?;
            let mut result = path.clone();
            result.anchors = anchors;
            result.curve_kind = CurveKind::Bezier;
            Ok(result)
        }
    }
}

/// The largest factor by which `matrix` lengthens a vector, at least 1.
#[must_use]
pub fn stretch(matrix: Affine) -> f64 {
    // The largest singular value of the linear part: sqrt((s + sqrt(s^2 - 4 det^2)) / 2) with s
    // the sum of the squared entries.
    let sum = matrix.a.mul_add(
        matrix.a,
        matrix
            .b
            .mul_add(matrix.b, matrix.c.mul_add(matrix.c, matrix.d * matrix.d)),
    );
    let det = matrix.a.mul_add(matrix.d, -(matrix.b * matrix.c));
    let root = (4.0 * det).mul_add(-det, sum * sum).max(0.0).sqrt();
    f64::midpoint(sum, root).sqrt().max(1.0)
}

/// Evaluates Spiro controls (a corner node breaks the curve) into Bézier anchors.
///
/// # Errors
/// Returns an error for too few or non-finite controls or an invalid tolerance.
pub fn evaluate_spiro_nodes(
    nodes: &[NewAnchor],
    closed: bool,
    tolerance: Tolerance,
) -> Result<Vec<NewAnchor>, SpiroError> {
    let shape = solve_spiro(&spiro_controls(nodes), closed)?;
    spiro_shape_nodes(nodes, closed, &shape, tolerance)
}

/// The Spiro controls of `nodes`: a corner node breaks the curve's smoothness.
fn spiro_controls(nodes: &[NewAnchor]) -> Vec<SpiroControl> {
    nodes
        .iter()
        .map(|anchor| SpiroControl {
            point: anchor.point,
            corner: anchor.kind == AnchorKind::Corner,
        })
        .collect()
}

/// Solves Spiro controls: a function of the controls alone (`specs/0115`).
///
/// # Errors
/// Returns an error for too few or non-finite controls.
pub fn solve_spiro_nodes(nodes: &[NewAnchor], closed: bool) -> Result<SpiroShape, SpiroError> {
    solve_spiro(&spiro_controls(nodes), closed)
}

/// The Bézier anchors of a solved Spiro shape through `nodes`.
///
/// # Errors
/// Returns an error when `shape` does not fit `nodes` or cannot be approximated.
pub fn spiro_shape_nodes(
    nodes: &[NewAnchor],
    closed: bool,
    shape: &SpiroShape,
    tolerance: Tolerance,
) -> Result<Vec<NewAnchor>, SpiroError> {
    let spans = spiro_cubics(&spiro_controls(nodes), closed, shape, tolerance)?;
    let cubics: Vec<[Point; 4]> = spans
        .iter()
        .map(|span| [span.start, span.control1, span.control2, span.end])
        .collect();
    Ok(anchors_of(&cubics, closed))
}

/// Evaluates B-spline controls for a transient preview.
///
/// # Errors
/// Returns an error for invalid or unrepresentable controls.
pub fn evaluate_bspline_nodes(
    nodes: &[NewAnchor],
    closed: bool,
) -> Result<Vec<NewAnchor>, SplineError> {
    let controls: Vec<SplineControl> = nodes
        .iter()
        .map(|anchor| SplineControl {
            point: anchor.point,
            corner: anchor.kind == curvyo_document_core::AnchorKind::Corner,
        })
        .collect();
    let spans = evaluate_bspline(&controls, closed)?;
    let cubics: Vec<[Point; 4]> = spans
        .iter()
        .map(|span| [span.start, span.control1, span.control2, span.end])
        .collect();
    Ok(anchors_of(&cubics, closed))
}

/// The Bézier anchors of a chain of cubic spans (start, control 1, control 2, end), closed back
/// to the first span's start when `closed`. The anchor ids are positions, local to this geometry.
fn anchors_of(spans: &[[Point; 4]], closed: bool) -> Vec<NewAnchor> {
    let mut anchors = Vec::with_capacity(spans.len() + usize::from(!closed));
    for (i, span) in spans.iter().enumerate() {
        let id = AnchorId::new(0, u64::try_from(i).unwrap_or(u64::MAX));
        let mut anchor = NewAnchor::corner(id, span[0]);
        anchor.handle_out = span[0].vector_to(span[1]);
        if i > 0 {
            anchor.handle_in = span[0].vector_to(spans[i - 1][2]);
        } else if closed && let Some(last) = spans.last() {
            anchor.handle_in = span[0].vector_to(last[2]);
        }
        anchors.push(anchor);
    }
    if !closed && let Some(last) = spans.last() {
        let id = AnchorId::new(0, u64::try_from(spans.len()).unwrap_or(u64::MAX));
        let mut anchor = NewAnchor::corner(id, last[3]);
        anchor.handle_in = last[3].vector_to(last[2]);
        anchor.handle_out = Vec2::ZERO;
        anchors.push(anchor);
    }
    anchors
}

#[cfg(test)]
mod stretch_tests {
    use super::stretch;
    use curvyo_document_core::Affine;

    /// The largest lengthening, not the Frobenius norm: the identity and a rotation lengthen
    /// nothing, a scale by its larger factor, a shear by its largest singular value.
    #[test]
    fn stretch_is_the_largest_lengthening() {
        assert!((stretch(Affine::IDENTITY) - 1.0).abs() < 1e-12);
        let (sin, cos) = 0.6_f64.sin_cos();
        assert!((stretch(Affine::new(cos, sin, -sin, cos, 5.0, 7.0)) - 1.0).abs() < 1e-12);
        assert!((stretch(Affine::scale(3.0, 0.5)) - 3.0).abs() < 1e-12);
        assert!(
            (stretch(Affine::scale(0.2, 0.5)) - 1.0).abs() < 1e-12,
            "at least 1"
        );
        // [[1, 1], [0, 1]]: singular values (sqrt(5) +- 1) / 2.
        let shear = Affine::new(1.0, 0.0, 1.0, 1.0, 0.0, 0.0);
        assert!((stretch(shear) - f64::midpoint(5.0_f64.sqrt(), 1.0)).abs() < 1e-12);
    }
}
