//! Why a model did not load or run (ADR 0075 §2).

/// Why a model did not load or run.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AiError {
    /// The bytes are not the pinned file: another size or SHA-256 (also every placeholder).
    #[error("the model file does not match its pinned size and checksum")]
    WrongModel,
    /// The pinned file did not parse or run as the model it should be.
    #[error("the model could not be run: {0}")]
    BadModel(String),
    /// The result would pass the working limit (16 megapixels, 8192 px on a side).
    #[error("{width} x {height} px is over the working limit of 16 megapixels")]
    TooLarge {
        /// The result's width.
        width: u64,
        /// The result's height.
        height: u64,
    },
    /// The caller's poll asked to stop.
    #[error("cancelled")]
    Cancelled,
    /// The input picture is not usable (empty, a pixel buffer of another size).
    #[error("{0}")]
    Invalid(String),
}
