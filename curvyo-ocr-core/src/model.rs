//! Loads and checks the detector, the recogniser and the alphabet.

use rten::Model;
use sha2::{Digest, Sha256};

use crate::OcrError;

/// The SHA-256 of the shipped detector (`frontend/public/models/paddleocr-latin/det.onnx`).
pub const DETECTOR_SHA256: &str =
    "ee40e80071ba3a320d4efda75f3e22047a7d049e9bf7bcaaf9daea23fc21b935";
/// The SHA-256 of the shipped recogniser (`rec.onnx`).
pub const RECOGNISER_SHA256: &str =
    "614ffc2d6d3902d360fad7f1b0dd455ee45e877069d14c4e51a99dc4ef144409";
/// The SHA-256 of the shipped alphabet (`dict.txt`).
pub const ALPHABET_SHA256: &str =
    "3c0a8a79b612653c25f765271714f71281e4e955962c153e272b7b8c1d2b13ff";

/// The parsed detector, recogniser and alphabet.
pub struct OcrModel {
    detector: Model,
    recogniser: Model,
    /// The symbol of each class after the CTC blank (class 0), the space last.
    alphabet: Vec<String>,
}

impl std::fmt::Debug for OcrModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OcrModel")
            .field("alphabet", &self.alphabet.len())
            .finish_non_exhaustive()
    }
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    Sha256::digest(bytes)
        .iter()
        .fold(String::with_capacity(64), |mut out, b| {
            let _ = write!(out, "{b:02x}");
            out
        })
}

impl OcrModel {
    /// Checks each file's SHA-256 against the constants of this crate, then parses them. The
    /// alphabet is one symbol per line; the space is appended as the last class (`PaddleOCR`'s
    /// `use_space_char`).
    ///
    /// # Errors
    /// [`OcrError::WrongModel`] when a hash differs, [`OcrError::BadModel`] when a file with the
    /// right hash cannot be parsed.
    pub fn load(detector: &[u8], recogniser: &[u8], alphabet: &str) -> Result<Self, OcrError> {
        if hex(detector) != DETECTOR_SHA256
            || hex(recogniser) != RECOGNISER_SHA256
            || hex(alphabet.as_bytes()) != ALPHABET_SHA256
        {
            return Err(OcrError::WrongModel);
        }
        let parse = |bytes: &[u8]| {
            Model::load(bytes.to_vec()).map_err(|e| OcrError::BadModel(e.to_string()))
        };
        let mut symbols: Vec<String> = alphabet.lines().map(str::to_owned).collect();
        symbols.push(" ".to_owned());
        Ok(Self {
            detector: parse(detector)?,
            recogniser: parse(recogniser)?,
            alphabet: symbols,
        })
    }

    pub(crate) fn detector(&self) -> &Model {
        &self.detector
    }

    pub(crate) fn recogniser(&self) -> &Model {
        &self.recogniser
    }

    /// The symbol of class `class` (1-based; 0 is the CTC blank).
    pub(crate) fn symbol(&self, class: usize) -> Option<&str> {
        self.alphabet.get(class.checked_sub(1)?).map(String::as_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn other_bytes_are_the_wrong_model() {
        assert_eq!(
            OcrModel::load(b"det", b"rec", "a\nb").map(|_| ()),
            Err(OcrError::WrongModel)
        );
    }

    #[test]
    fn the_hash_is_lower_case_hex() {
        assert_eq!(
            hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }
}
