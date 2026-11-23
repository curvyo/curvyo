//! A model file checked against its pin and parsed (ADR 0075 §6): only a file whose size and
//! SHA-256 match the compiled-in list is ever parsed.

use sha2::{Digest, Sha256};

use crate::error::AiError;
use crate::models::{ModelFile, ModelKind};

/// Whether `bytes` are the pinned file `file`: its size and SHA-256 (criterion 64).
///
/// # Errors
/// [`AiError::WrongModel`] for any other bytes; a placeholder never matches.
pub fn verify(file: &ModelFile, bytes: &[u8]) -> Result<(), AiError> {
    if file.placeholder || bytes.len() as u64 != file.bytes {
        return Err(AiError::WrongModel);
    }
    let digest: [u8; 32] = Sha256::digest(bytes).into();
    if digest == file.sha256 {
        Ok(())
    } else {
        Err(AiError::WrongModel)
    }
}

/// A parsed model of one kind.
#[derive(Debug)]
pub struct Model {
    pub(crate) kind: ModelKind,
    pub(crate) graph: rten::Model,
}

impl Model {
    /// Verifies `bytes` against `file`, then parses them. External tensor data has no file to
    /// come from here and is refused by the loader; an operator outside the ONNX domains is
    /// unknown to the runtime and refused.
    ///
    /// # Errors
    /// [`AiError::WrongModel`] when the bytes are not the pinned file; [`AiError::BadModel`] when
    /// they do not parse.
    pub fn load(file: &ModelFile, bytes: &[u8]) -> Result<Self, AiError> {
        verify(file, bytes)?;
        let graph =
            rten::Model::load(bytes.to_vec()).map_err(|e| AiError::BadModel(e.to_string()))?;
        Ok(Self {
            kind: file.kind,
            graph,
        })
    }

    /// The kind of model.
    #[must_use]
    pub const fn kind(&self) -> ModelKind {
        self.kind
    }
}
