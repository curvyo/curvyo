//! The input hash of an AI step (ADR 0077 §2; `specs/0135` criterion 48): which picture and model
//! fields a stored result was made from, so a changed upstream step marks it Out of date.

use sha2::{Digest, Sha256};

use crate::working::WorkingImage;

/// The version tag the hash starts with; a change of what is hashed is a new tag.
const TAG: &[u8] = b"curvyo-ai-input/1\0";

/// The SHA-256 over the version tag, the step `kind`, the `model` name and `version`, the model
/// fields `fields` (key and value, in the given order) and the step's input pixels with their
/// width, height and channel count.
#[must_use]
pub fn input_hash(
    kind: &str,
    (model, version): (&str, &str),
    fields: &[(&str, i64)],
    input: &WorkingImage,
) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update(TAG);
    for text in [kind, model, version] {
        hash.update(text.as_bytes());
        hash.update([0]);
    }
    for (key, value) in fields {
        hash.update(key.as_bytes());
        hash.update(b"=");
        hash.update(value.to_le_bytes());
    }
    hash.update(input.width.to_le_bytes());
    hash.update(input.height.to_le_bytes());
    hash.update(4_u32.to_le_bytes());
    hash.update(&input.pixels);
    hash.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn image(g: u8) -> WorkingImage {
        WorkingImage {
            width: 2,
            height: 1,
            pixels: vec![g, g, g, 255, 0, 0, 0, 255],
            original_size: (2, 1),
            truncated: false,
            animated: false,
        }
    }

    #[test]
    fn the_hash_follows_the_pixels_the_model_and_the_fields() {
        let base = input_hash("upscale", ("esrgan", "1"), &[("factor", 2)], &image(9));
        assert_eq!(
            base,
            input_hash("upscale", ("esrgan", "1"), &[("factor", 2)], &image(9))
        );
        for other in [
            input_hash("upscale", ("esrgan", "1"), &[("factor", 4)], &image(9)),
            input_hash("upscale", ("esrgan", "2"), &[("factor", 2)], &image(9)),
            input_hash("upscale", ("esrgan", "1"), &[("factor", 2)], &image(8)),
            input_hash("ai_denoise", ("esrgan", "1"), &[("factor", 2)], &image(9)),
        ] {
            assert_ne!(base, other);
        }
    }
}
