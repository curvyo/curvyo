//! Errors identify invalid sketch inputs before numerical mutation.

/// A rejected input or unsuccessful numerical operation.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum SketchError {
    /// Geometry or references violate the sketch contract.
    #[error("invalid sketch: {0}")]
    Invalid(String),
    /// A named expression cannot produce the declared dimension.
    #[error("invalid formula: {0}")]
    Formula(String),
    /// The numerical backend could not produce a usable candidate.
    #[error("solver failure: {0}")]
    Solver(String),
}
