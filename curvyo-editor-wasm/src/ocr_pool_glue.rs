//! Carries a picture's line crops between the trace worker that detects them and the pool workers
//! that read them (`specs/0091-trace-text-recognition` round 2): numbers and JSON across the wasm
//! boundary, host-independent so ordinary `cargo test` covers it.
//!
//! The crops: `{lines: [{quad, width, pad, samples}]}` (`samples`: how many numbers of the array
//! are the line's, its padding included) and one `f32` array of their samples, line after line.
//! A read: a JSON array with one entry per crop, the line as `ocr_lines_json` writes it or `null`.

use serde_json::{Value, json};
use vectrace::ocr::{LINE_HEIGHT as OCR_LINE_HEIGHT, LineCrop, OcrModel, read_crop_text};

/// The crops as the JSON of their quads and widths and their samples in one array.
#[must_use]
pub fn crops_wire(crops: &[LineCrop]) -> (String, Vec<f32>) {
    let lines: Vec<Value> = crops
        .iter()
        .map(|c| json!({ "quad": c.quad, "width": c.width, "pad": c.pad, "samples": c.rgb.len() }))
        .collect();
    let samples = crops.iter().flat_map(|c| c.rgb.iter().copied()).collect();
    (json!({ "lines": lines }).to_string(), samples)
}

/// The crops of [`crops_wire`]'s `json` and `samples`; `None` unless they match.
#[must_use]
pub fn read_crops_wire(json: &str, samples: &[f32]) -> Option<Vec<LineCrop>> {
    let value: Value = serde_json::from_str(json).ok()?;
    let mut at: usize = 0;
    let mut out = Vec::new();
    for line in value.get("lines")?.as_array()? {
        let width = usize::try_from(line.get("width")?.as_u64()?).ok()?;
        let pad = line.get("pad")?.as_array()?;
        let pad = [
            usize::try_from(pad.first()?.as_u64()?).ok()?,
            usize::try_from(pad.get(1)?.as_u64()?).ok()?,
        ];
        let mut quad = [[0.0_f32; 2]; 4];
        let corners = line.get("quad")?.as_array()?;
        if corners.len() != 4 {
            return None;
        }
        for (corner, out) in corners.iter().zip(quad.iter_mut()) {
            let xy = corner.as_array()?;
            // invariant: the JSON holds `f32` values written by `crops_wire`.
            #[allow(clippy::cast_possible_truncation)]
            {
                *out = [xy.first()?.as_f64()? as f32, xy.get(1)?.as_f64()? as f32];
            }
        }
        let count = (pad[0] + width + pad[1]).checked_mul(OCR_LINE_HEIGHT * 3)?;
        let rgb = samples.get(at..at.checked_add(count)?)?.to_vec();
        at += count;
        out.push(LineCrop {
            quad,
            width,
            pad,
            rgb,
        });
    }
    (at == samples.len()).then_some(out)
}

/// Reads `crops` with `model`: the JSON array of their lines (`null` where nothing was read), or
/// `None` when a crop cannot be read. `done` hears the crops read so far.
#[must_use]
pub fn read_crops(
    model: &OcrModel,
    crops: &[LineCrop],
    done: &mut dyn FnMut(usize),
) -> Option<String> {
    let mut out = Vec::with_capacity(crops.len());
    for (i, crop) in crops.iter().enumerate() {
        let line = read_crop_text(model, crop).ok()?;
        out.push(line.map_or(
            Value::Null,
            |l| json!({ "quad": l.quad, "text": l.text, "confidence": l.confidence, "positions": l.positions }),
        ));
        done(i + 1);
    }
    Some(Value::Array(out).to_string())
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use vectrace::ocr::{crop_text, read_text};

    use super::*;

    fn model() -> OcrModel {
        let dir =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../frontend/public/models/paddleocr-latin");
        let read = |name: &str| std::fs::read(dir.join(name)).unwrap();
        let alphabet = String::from_utf8(read("dict.txt")).unwrap();
        OcrModel::load(&read("det.onnx"), &read("rec.onnx"), &alphabet).unwrap()
    }

    /// The crops travel through the wire unchanged, read in two shares they give the lines of
    /// the read in one worker, in the same order.
    #[test]
    fn crops_read_in_shares_give_the_one_worker_read() {
        let model = model();
        let bytes = std::fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../tests/assets/vectrace-fixtures/poster.png"),
        )
        .unwrap();
        let image = curvyo_image_core::decode_working(&bytes).unwrap();
        let whole = read_text(
            &model,
            &image.pixels,
            image.width,
            image.height,
            &mut |_| std::ops::ControlFlow::Continue(()),
        )
        .unwrap();
        let crops = crop_text(&model, &image.pixels, image.width, image.height).unwrap();
        let (json, samples) = crops_wire(&crops);
        let back = read_crops_wire(&json, &samples).unwrap();
        assert_eq!(back, crops);
        let (first, second) = back.split_at(back.len() / 2);
        let mut lines = Vec::new();
        for share in [first, second] {
            let read: Value =
                serde_json::from_str(&read_crops(&model, share, &mut |_| {}).unwrap()).unwrap();
            lines.extend(
                read.as_array()
                    .unwrap()
                    .iter()
                    .filter(|l| !l.is_null())
                    .cloned(),
            );
        }
        let expected: Vec<Value> = whole
            .iter()
            .map(|l| {
                json!({ "quad": l.quad, "text": l.text, "confidence": l.confidence, "positions": l.positions })
            })
            .collect();
        assert_eq!(lines, expected);
    }

    #[test]
    fn mismatched_samples_are_no_crops() {
        let (json, mut samples) = crops_wire(&[LineCrop {
            quad: [[0.0, 0.0], [8.0, 0.0], [8.0, 48.0], [0.0, 48.0]],
            width: 8,
            pad: [1, 2],
            rgb: vec![0.5; 11 * OCR_LINE_HEIGHT * 3],
        }]);
        assert!(read_crops_wire(&json, &samples).is_some());
        samples.pop();
        assert!(read_crops_wire(&json, &samples).is_none());
    }
}
