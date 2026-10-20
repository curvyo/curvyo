//! The faces of a font file Add font accepts in the browser, with their names and class bytes
//! (`specs/0075-font-library` criteria 28 and 32, `adrs.md` decisions 3 and 7).

use curvyo_text_core::{listable_faces, os2_class_bytes};
use serde_json::{Value, json};

/// The faces Add font accepts in the browser (criteria 28 and 32), as JSON
/// `[{"index", "family", "style", "os2": hex|null}]`; empty for a file that is not a font.
#[must_use]
pub fn font_file_faces(bytes: &[u8]) -> String {
    let faces: Vec<Value> = listable_faces(bytes)
        .into_iter()
        .map(|face| {
            let os2 = os2_class_bytes(bytes, face.index).map(|os2| {
                os2.iter().fold(String::new(), |mut hex, b| {
                    use std::fmt::Write as _;
                    let _ = write!(hex, "{b:02x}");
                    hex
                })
            });
            json!({ "index": face.index, "family": face.family, "style": face.style, "os2": os2 })
        })
        .collect();
    Value::Array(faces).to_string()
}
